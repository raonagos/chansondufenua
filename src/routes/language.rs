//! The language in the URL: `/fr/…`, `/ty/…`, `/en/…`, and the bare URL that
//! serves the default.
//!
//! This layer is what makes a language an *address* rather than a preference.
//! It sits outside the router's page handlers and does three things, in order:
//!
//! 1. **`?lang=xx`** — a reader asking for a language by name is redirected to
//!    that language's URL, so the query string never becomes a second indexed
//!    copy of a page. The choice is remembered in a cookie on the way.
//! 2. **A prefix** — `/ty/himene` is handled *internally* at `/himene`, with the
//!    language carried in the request context ([`crate::i18n::Language`]). The
//!    browser's URL never changes, the router keeps one route per page, and
//!    every page reaches its language through [`crate::i18n::resolve`]. This is
//!    Topcoat's own rewrite mechanism, and it is why there is no second copy of
//!    every route for every language.
//! 3. **A remembered choice** — a bare URL from a reader whose cookie names a
//!    language other than the default is redirected to that language's URL.
//!    This is the "cookie redirect" the scope asks for, and it is deliberately
//!    *not* a rendering rule: the bare URL itself answers in
//!    [`Lang::DEFAULT`] to everyone, which is what makes it safe to cache.
//!
//! **`Accept-Language` is never read.** Not here, not in
//! [`crate::i18n::resolve`]. A response that varies on a request header is a
//! response a shared cache is entitled to store once and hand to everybody, and
//! Cloudflare would hand out whichever language it saw first. The header is a
//! hint about a reader; a URL is a statement about a document.
//!
//! **Only pages are language-scoped.** The API, the two sitemaps, `robots.txt`,
//! `llms.txt`, the MCP endpoint and the well-known documents are
//! language-neutral machine surfaces: `/ty/api/songs` is not a thing, and it
//! 404s rather than serving a second address for one document.
//!
//! The two redirects are `302` and carry `Cache-Control: private, no-store`,
//! because what they answer depends on a cookie or a query the client just
//! sent. A permanently cached redirect to one language is exactly the mistake
//! this module exists to prevent.

use topcoat::{
    context::Cx,
    router::{
        Body, HeaderValue, Layer, LayerFuture, Next, Path, StatusCode,
        error::rewrite,
        header,
        request::{headers, uri},
        response::Response,
    },
};

use crate::i18n::{self, Lang, Language};
use crate::pages::{artist, book, editor, home, search, songs, support};
use crate::routes::negotiation;

/// The layer. A unit value: it holds no state, and the request context is where
/// everything it reads lives.
pub struct LanguageLayer;

impl Layer for LanguageLayer {
    /// `None`: this layer wraps every request, like the negotiation it runs
    /// beside.
    fn path(&self) -> Option<&Path> {
        None
    }

    fn handle<'a>(&'a self, cx: &'a Cx, body: Body, next: Next<'a>) -> LayerFuture<'a> {
        Box::pin(async move {
            let path = uri(cx).path();
            let query = uri(cx).query().unwrap_or("");

            // A path the site's pages do not own is not language-scoped at all:
            // it is answered as it is, whatever prefix it arrived under (and a
            // prefixed machine path matches no route, so it 404s, which is the
            // honest answer to an address that does not exist).
            let Some((page, named)) = page_of(path) else {
                return next.run(cx, body).await;
            };

            // 1. An explicit choice wins over the prefix. It is a redirect and
            //    not a rendering rule: `/himene?lang=ty` must not be a second
            //    indexable copy of `/ty/himene`.
            if let Some((wanted, rest)) = explicit(query) {
                let target = with_query(i18n::at(wanted, page), &rest);
                if target != path {
                    return Ok(redirect(&target, Some(wanted)));
                }
            }

            // 2. The prefix: handle the same page internally, in that language.
            //    The `lang` parameter is dropped — the prefix already said it,
            //    and leaving it in the query would make the rewritten request
            //    redirect itself.
            if let Some(lang) = named {
                let target = with_query(page.to_owned(), &without_lang(query));
                return Err(rewrite(target, body).with(Language(lang)).into());
            }

            // 3. A remembered choice, on a bare URL. Only for a language the
            //    bare page could not serve — redirecting `/himene` to
            //    `/fr/himene` would send a reader to the same French page by a
            //    longer road.
            //
            //    Only a *fresh* bare request is redirected. A request carrying
            //    [`crate::i18n::Language`] in its context is an internal rewrite
            //    this layer made a moment ago: its browser URL was already the
            //    prefixed one, and redirecting it again would answer a request
            //    for `/ty/himene` with a redirect to `/ty/himene` — a loop the
            //    client cannot break out of.
            if i18n::requested(cx).is_none()
                && let Some(chosen) = remembered(cx)
                && chosen != Lang::DEFAULT
            {
                return Ok(redirect(&i18n::at(chosen, page), None));
            }

            next.run(cx, body).await
        })
    }
}

/// The page a request path names, and the language its URL named.
///
/// `None` for a path the pages do not own: the API, the sitemaps, `robots.txt`,
/// `llms.txt`, the MCP endpoint, the well-known documents, the card images —
/// and any song URL, which is a page.
///
/// Exactly one prefix: `/ty/ty/himene` is not a page in any language, and it is
/// not an address this layer invents a meaning for. It 404s.
///
/// **This is the list step 22 was told two files would need, and it is one.**
/// A new page (an artist's `/artiste/{id}`, step 29) is added here, next to the
/// route that declares it — `/himene/pluriel` is there for step 26, and
/// `negotiation::song_segment` has to know the same name for a different reason
/// (it must not mistake the page for a song).
///
/// Pagination's pages are here as a *shape* rather than as two paths, because
/// their number is not this layer's business: `/himene/page/2`,
/// `/himene/page/43` and `/himene/page/x` are all one page as far as language is
/// concerned — a prefixed one has to be rewritten to it — and which numbers the
/// catalogue has is decided by the page itself, in the language the URL named
/// (`/ty/himene/page/x` is the same 404 as the unprefixed one, in Tahitian).
fn page_of(path: &str) -> Option<(&str, Option<Lang>)> {
    match split_prefix(path) {
        Some((lang, page)) => {
            (split_prefix(page).is_none() && is_page(page)).then_some((page, Some(lang)))
        }
        None => is_page(path).then_some((path, None)),
    }
}

/// Whether `path` is one of the site's pages — the set the layout gives a
/// `<head>` and the negotiation layer gives a Markdown form.
fn is_page(path: &str) -> bool {
    path == home::PATH
        || path == home::AEPA_PATH
        || path == songs::PATH
        || path == editor::PATH
        || path == book::PATH
        || path == search::PATH
        || path == support::PATH
        || artist::segment(path).is_some()
        || songs::page_segment(path).is_some()
        || negotiation::song_segment(path).is_some()
}

/// Splits a leading language segment off a path.
///
/// `/ty/himene` → `(Ty, "/himene")`; `/ty` and `/ty/` → `(Ty, "/")`; `/fr/x` →
/// `(Fr, "/x")`; anything else → `None`.
fn split_prefix(path: &str) -> Option<(Lang, &str)> {
    let rest = path.strip_prefix('/')?;
    let (segment, _) = match rest.split_once('/') {
        Some((segment, tail)) => (segment, tail),
        None => (rest, ""),
    };
    let lang = Lang::from_code(segment)?;

    let page = &path[1 + segment.len()..];
    Some((lang, if page.is_empty() { "/" } else { page }))
}

/// The `?lang=` parameter, and the query string without it.
///
/// A hand-rolled reader rather than the query extractor, because this layer
/// runs before any handler and needs the *rest* of the query, not one field of
/// it. A malformed pair is skipped rather than fatal: a query string a client
/// mangled is not a reason to answer 500.
fn explicit(query: &str) -> Option<(Lang, String)> {
    let mut wanted = None;
    let mut rest = Vec::new();

    for pair in query.split('&').filter(|pair| !pair.is_empty()) {
        let (name, value) = pair.split_once('=').unwrap_or((pair, ""));
        if name == i18n::COOKIE {
            wanted = Lang::from_tag(value);
        } else {
            rest.push(pair);
        }
    }

    wanted.map(|lang| (lang, rest.join("&")))
}

/// A query string with its `lang` parameter removed.
fn without_lang(query: &str) -> String {
    query
        .split('&')
        .filter(|pair| !pair.is_empty() && !pair.starts_with("lang=") && *pair != i18n::COOKIE)
        .collect::<Vec<&str>>()
        .join("&")
}

/// `path` with `query` appended, if there is one.
fn with_query(path: String, query: &str) -> String {
    if query.is_empty() {
        path
    } else {
        format!("{path}?{query}")
    }
}

/// The language the request's cookie remembers, if it names one.
///
/// The cookie header is read directly rather than through Topcoat's jar: the
/// jar is installed *inside* the pathless layers (that is why
/// [`crate::i18n::resolve`] stopped reading it at all), and this layer runs
/// outside it.
fn remembered(cx: &Cx) -> Option<Lang> {
    let header = headers(cx).get(header::COOKIE)?.to_str().ok()?;

    header.split(';').find_map(|pair| {
        let (name, value) = pair.split_once('=')?;
        (name.trim() == i18n::COOKIE)
            .then(|| Lang::from_tag(value.trim()))
            .flatten()
    })
}

/// The redirect to a language's URL, with the cookie that remembers the choice.
///
/// `302` and not `301`: what is being answered is a *preference*, and a
/// permanent redirect is cached by the browser forever — a reader who changes
/// their mind would be sent back to their old language by their own browser.
/// `no-store` for the same reason a shared cache must not keep it: the answer
/// depends on a cookie or on a query string, not on the URL alone.
fn redirect(target: &str, remember: Option<Lang>) -> Response {
    let mut response = Response::new(Body::empty());
    *response.status_mut() = StatusCode::FOUND;
    let headers = response.headers_mut();

    if let Ok(location) = HeaderValue::from_str(target) {
        headers.insert(header::LOCATION, location);
    }

    headers.insert(
        header::CACHE_CONTROL,
        HeaderValue::from_static("private, no-store"),
    );
    headers.insert(header::VARY, HeaderValue::from_static("Cookie"));

    if let Some(lang) = remember {
        let cookie = format!(
            "{}={}; Path=/; Max-Age={}; SameSite=Lax",
            i18n::COOKIE,
            lang.code(),
            // One year, the same as v4's jar wrote.
            60 * 60 * 24 * 365,
        );
        if let Ok(value) = HeaderValue::from_str(&cookie) {
            headers.append(header::SET_COOKIE, value);
        }
    }

    response
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::i18n::Lang;
    use crate::pages::{book, editor, home, songs, support};

    /// The reader that decides whether a path is a page, and in what language.
    ///
    /// Exactly one prefix, and only ahead of a page: `/ty/ty/himene` is not a
    /// page in any language, and a machine surface (`/api/songs`, `robots.txt`,
    /// the sitemaps, the well-known documents) is addressable one way, which is
    /// not under a language.
    #[test]
    fn a_page_is_a_page_under_at_most_one_language() {
        assert_eq!(page_of("/himene"), Some(("/himene", None)));
        assert_eq!(page_of("/ty/himene"), Some(("/himene", Some(Lang::Ty))));
        assert_eq!(page_of("/en/"), Some(("/", Some(Lang::En))));
        assert_eq!(page_of("/fr"), Some(("/", Some(Lang::Fr))));
        assert_eq!(
            page_of("/en/himene/ahani-e"),
            Some(("/himene/ahani-e", Some(Lang::En)))
        );

        // Two prefixes is nobody's page, and neither is a prefix in front of
        // anything that is not one.
        assert_eq!(page_of("/ty/ty/himene"), None);
        assert_eq!(page_of("/ty/robots.txt"), None);
        assert_eq!(page_of("/ty/api/songs"), None);
        assert_eq!(page_of("/ty/.well-known/mcp/server-card.json"), None);
        assert_eq!(page_of("/xx/himene"), None);
        assert_eq!(page_of("/himene/a/b"), None);

        // The pages themselves, unprefixed — including the multi-lyric page,
        // whose selection lives in a query string and whose *path* is therefore
        // a page like any other: `/fr/himene/pluriel?s=…` has to strip to
        // `/himene/pluriel?s=…` or the page 404s in every language but the
        // default.
        for path in [
            home::PATH,
            home::AEPA_PATH,
            songs::PATH,
            editor::PATH,
            book::PATH,
            search::PATH,
            support::PATH,
        ] {
            assert!(page_of(path).is_some(), "{path}");
        }
        assert_eq!(
            page_of("/ty/himene/pluriel"),
            Some(("/himene/pluriel", Some(Lang::Ty)))
        );

        // An artist's page is a page like any other, and one id deep — an
        // artist URL below that is nobody's page, prefixed or not.
        assert_eq!(
            page_of("/artiste/1kvm9y2tcplm43wgeuni"),
            Some(("/artiste/1kvm9y2tcplm43wgeuni", None))
        );
        assert_eq!(
            page_of("/en/artiste/1kvm9y2tcplm43wgeuni"),
            Some(("/artiste/1kvm9y2tcplm43wgeuni", Some(Lang::En)))
        );
        assert_eq!(page_of("/artiste/a/b"), None);
        assert_eq!(page_of("/ty/artiste/a/b"), None);
        // The search page carries its needle in the query string, so its path is
        // a page like the multi-lyric page's is.
        assert_eq!(
            page_of("/fr/recherche"),
            Some(("/recherche", Some(Lang::Fr)))
        );

        // A page of the index: the two-segment shape is a page, whatever number
        // is in it and whether or not that number exists — which numbers exist is
        // the page's own question, answered in the language the URL named.
        assert_eq!(page_of("/himene/page/2"), Some(("/himene/page/2", None)));
        assert_eq!(
            page_of("/en/himene/page/2"),
            Some(("/himene/page/2", Some(Lang::En)))
        );
        assert_eq!(
            page_of("/ty/himene/page/x"),
            Some(("/himene/page/x", Some(Lang::Ty)))
        );
        // One segment is not the paginated shape: `/himene/page` is a *song* URL
        // as far as this layer is concerned — and a song called `page`, which no
        // song is, so it is the 404's. A third segment is nobody's page at all.
        assert_eq!(page_of("/himene/page"), Some(("/himene/page", None)));
        assert_eq!(page_of("/himene/page/2/3"), None);
        assert_eq!(page_of("/ty/himene/page/2/3"), None);
    }

    /// `?lang=` is read out of the query and the rest of the query is kept, so
    /// a redirect to a language does not throw away a client's other parameters.
    /// A malformed value is skipped rather than fatal.
    #[test]
    fn the_lang_parameter_is_read_and_the_rest_kept() {
        assert_eq!(explicit("lang=ty"), Some((Lang::Ty, String::new())));
        assert_eq!(
            explicit("page=2&lang=en&sort=old"),
            Some((Lang::En, "page=2&sort=old".to_owned()))
        );
        assert_eq!(explicit("lang=pt"), None);
        assert_eq!(explicit(""), None);
        assert_eq!(explicit("lang"), None);
    }

    /// The query string a prefixed request is rewritten with has its `lang`
    /// parameter dropped — otherwise the rewritten dispatch would read it again
    /// and redirect to itself.
    #[test]
    fn the_rewrite_drops_the_parameter_that_named_the_language() {
        assert_eq!(without_lang("lang=ty"), "");
        assert_eq!(without_lang("lang=ty&page=2"), "page=2");
        assert_eq!(without_lang("page=2&lang=ty"), "page=2");
        assert_eq!(without_lang(""), "");

        assert_eq!(with_query("/ty/himene".to_owned(), ""), "/ty/himene");
        assert_eq!(
            with_query("/ty/himene".to_owned(), "page=2"),
            "/ty/himene?page=2"
        );
    }
}
