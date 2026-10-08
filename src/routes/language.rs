//! The language a page is served in, the addresses that used to say it, and the
//! one address that sets it.
//!
//! Since v4.1 **a page has exactly one URL**, and the language is not part of
//! it. The reader's language is resolved by [`crate::i18n::resolve`] — the `lang`
//! cookie, then `Accept-Language`, then French — and this module is what makes
//! that safe and what retires the two spellings that used to name a language:
//!
//! 0. **A retired spelling.** `/fr`, `/ty` and `/en` were language *prefixes*
//!    until v4.1 — `/fr/himene` was a page's canonical URL — and `?lang=` was a
//!    redirect to one. Both are gone: a prefixed path and any `?lang=` parameter
//!    answer **`301`** to the address the page has now (`/fr/himene` →
//!    `/himene`), because v4.1 published those URLs and a link already in the
//!    wild may not 404. The move is language-independent — there is no longer a
//!    language a URL can name — so it is one hop, and the query string comes
//!    along with its `lang` parameter dropped.
//! 1. **A retired *page*.** The addresses of step 33 — `/aepa`, `/recherche`,
//!    `/soutenir`, `/himene/pluriel`, `/artiste/{id}` — are answered `301` at
//!    their successors, from the one table in [`crate::pages::retired`]. Since
//!    prefixes are stripped first, `/fr/recherche` is one hop to `/paimi`
//!    rather than two.
//! 2. **`Vary: Cookie, Accept-Language`.** Every page response says it, because
//!    the same URL now answers two readers in two languages (see the module
//!    docs of [`crate::i18n`]). A shared cache that ignored this would hand one
//!    reader's language to everybody, which is the defect the v4 URL rules
//!    existed to prevent — and the review's "default is the system" is the
//!    decision that bought it back.
//! 3. **The switcher's address** — `/reo/{code}`, the one route here. It
//!    sets the cookie and `302`s back to the page the reader was on, which the
//!    switcher put in the link's own `next` parameter. It is not a page: no
//!    `<head>`, no layout, no Markdown form, and nothing links to it that a
//!    crawler should follow to content.
//!
//! **Only pages vary.** The API, both sitemaps, `robots.txt`, `llms.txt`, the
//! MCP endpoint and the well-known documents are language-neutral machine
//! surfaces: they are one document, they do not read a cookie, and they say no
//! `Vary` at all.
//!
//! **Why the retired spellings live here and not in `pages::retired`.** That
//! table holds the addresses *pages* published and moved away from; a prefix and
//! a query parameter are shapes of *every* address, so they are this layer's
//! business — and the layer is the only thing that runs before routing and could
//! never be a page.

use topcoat::{
    Result,
    context::Cx,
    router::{
        Body, HeaderValue, Layer, LayerFuture, Next, Path, StatusCode, header, path_param,
        request::uri, response::Response, route,
    },
};

use crate::i18n::{self, Lang};
use crate::pages::{artist, book, editor, home, retired, search, songs, support};
use crate::routes::negotiation;

/// The prefix the language switcher's own address lives under.
pub const PATH: &str = "/reo";

/// The query parameter a switcher link carries the page it came from in.
///
/// The switcher's address is not a page, so "the same page in the other
/// language" has to travel with the link: `next` holds the reader's own path
/// (and query), and the route sends them back to it.
pub const PARAM: &str = "next";

/// The header every page carries: the same URL answers per reader.
const VARY: &str = "Cookie, Accept-Language";

/// Adds `Cookie, Accept-Language` to a page response's `Vary`, **merged into a
/// single field**.
///
/// Topcoat's own view layer already appends `Vary: Accept` to a page — a view can
/// be served as HTML or as Markdown, by Topcoat's own negotiation — so this
/// cannot set the header without discarding that. It must not leave a second
/// field either: a cache is entitled to read the first `Vary` field alone, and
/// one that did would ignore the cookie and serve one reader's language to
/// everybody, which is exactly the defect the v4 URL rules existed to prevent.
fn merge_vary(response: &mut Response) {
    let existing = response
        .headers()
        .get_all(header::VARY)
        .iter()
        .filter_map(|value| value.to_str().ok())
        .filter(|value| !value.eq_ignore_ascii_case(VARY))
        .collect::<Vec<_>>()
        .join(", ");

    let merged = if existing.is_empty() {
        VARY.to_owned()
    } else {
        format!("{existing}, {VARY}")
    };

    // `insert` replaces every field of that name, which is the point: one
    // `Vary`, holding everything the response actually varies on.
    if let Ok(value) = HeaderValue::from_str(&merged) {
        response.headers_mut().insert(header::VARY, value);
    }
}

/// The page a switcher link came from, where it came from nowhere: the root.
const HOME: &str = "/";

// The `{code}` of the one route below.
path_param!(code);

/// The switcher's link to the same page in `lang`: set the cookie, come back.
///
/// `next` is the page the reader is on, as [`crate::i18n::path_and_query`]
/// spells it, percent-encoded into the query because it is a path and may carry
/// its own query string.
pub fn switch(lang: Lang, next: &str) -> String {
    format!("{PATH}/{}?{PARAM}={}", lang.code(), encode(next))
}

/// `GET /reo/{code}` — the language switcher's target.
///
/// A `302` back to the page the reader came from, with the choice remembered in
/// a cookie. `302` and not `301`: what is being answered is a *preference*, and
/// a permanent redirect is cached by the browser forever — a reader who changes
/// their mind would be sent back to their old language by their own browser.
/// `no-store` for the same reason a shared cache must not keep it.
///
/// A code the site does not speak is a `404`: the address exists (it is a route)
/// but names no language, the same rule every other unknown segment follows.
#[route(GET "/reo/{code}")]
async fn choose(cx: &Cx) -> Result<Response> {
    let code: &str = path_param::<Code>(cx);

    let Some(lang) = Lang::from_tag(code) else {
        let mut response = Response::new(Body::empty());
        *response.status_mut() = StatusCode::NOT_FOUND;
        return Ok(response);
    };

    let mut response = Response::new(Body::empty());
    *response.status_mut() = StatusCode::FOUND;
    let headers = response.headers_mut();

    // `destination` returns a visible-ASCII local path, so neither of these can
    // fail — the `expect` records that rather than an assumption.
    headers.insert(
        header::LOCATION,
        HeaderValue::from_str(&destination(uri(cx).query().unwrap_or("")))
            .expect("a local path is a header value"),
    );
    headers.insert(
        header::CACHE_CONTROL,
        HeaderValue::from_static("private, no-store"),
    );
    headers.append(
        header::SET_COOKIE,
        HeaderValue::from_str(&remembered_cookie(lang)).expect("a cookie is a header value"),
    );

    Ok(response)
}

/// The `Set-Cookie` a language choice is kept in.
///
/// One year, the same as v4's jar wrote, and `Path=/` so the choice is the
/// site's rather than one page's. Not `Secure`: the site is served over TLS but
/// is also run on `http://127.0.0.1` in the tests, and a cookie a test cannot
/// read is a cookie this step cannot verify. Not `HttpOnly` either — a
/// progressive enhancement is allowed to read the reader's own preference.
fn remembered_cookie(lang: Lang) -> String {
    format!(
        "{}={}; Path=/; Max-Age={}; SameSite=Lax",
        i18n::COOKIE,
        lang.code(),
        60 * 60 * 24 * 365,
    )
}

/// The page a switcher link was on, read out of the link's own `next`.
///
/// The parameter is client-supplied, so it is validated rather than trusted: a
/// value that is not a local, visible-ASCII path — an absolute URL, a
/// protocol-relative `//host`, a backslash the browser might read as a slash, a
/// control byte, or a stray non-ASCII one — is refused and the reader is sent
/// home. A `next` that is a local path is a path the site itself wrote; there is
/// no second thing to check.
fn destination(query: &str) -> String {
    param(query, PARAM)
        .map(decode)
        .filter(|path| is_local(path))
        .unwrap_or_else(|| HOME.to_owned())
}

/// Whether `path` is a path on this site that a header can carry.
fn is_local(path: &str) -> bool {
    path.starts_with('/')
        && !path.starts_with("//")
        && !path.starts_with("/\\")
        && path.bytes().all(|byte| (0x20..=0x7e).contains(&byte))
}

/// The first value of `name` in a query string, if there is one.
///
/// A hand-rolled reader, like the other query readers in this crate, and for the
/// same reason: this runs before any extractor and wants one raw value, not a
/// typed field. A pair with no `=` is read as an empty value.
fn param<'q>(query: &'q str, name: &str) -> Option<&'q str> {
    query
        .split('&')
        .filter(|pair| !pair.is_empty())
        .find_map(|pair| {
            let (key, value) = pair.split_once('=').unwrap_or((pair, ""));
            (key == name).then_some(value)
        })
}

/// `value` percent-decoded, byte for byte. A malformed escape is left as it is
/// written rather than refused: [`is_local`] is what decides.
fn decode(value: &str) -> String {
    let bytes = value.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut index = 0;

    while index < bytes.len() {
        if bytes[index] == b'%'
            && index + 2 < bytes.len()
            && let (Some(high), Some(low)) = (hex(bytes[index + 1]), hex(bytes[index + 2]))
        {
            out.push(high * 16 + low);
            index += 3;
            continue;
        }

        out.push(bytes[index]);
        index += 1;
    }

    String::from_utf8_lossy(&out).into_owned()
}

/// One hexadecimal digit, or nothing.
fn hex(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        b'A'..=b'F' => Some(byte - b'A' + 10),
        _ => None,
    }
}

/// `value` percent-encoded: everything but the unreserved characters of RFC
/// 3986 §2.3, so `/`, `?`, `&`, `=` and `%` itself cannot end the parameter.
fn encode(value: &str) -> String {
    let mut out = String::with_capacity(value.len());

    for byte in value.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(byte as char);
            }
            _ => out.push_str(&format!("%{byte:02X}")),
        }
    }

    out
}

/// The layer. A unit value: it holds no state, and the request context is where
/// everything it reads lives.
pub struct LanguageLayer;

impl Layer for LanguageLayer {
    /// `None`: this layer wraps every request, retired spelling or not.
    fn path(&self) -> Option<&Path> {
        None
    }

    fn handle<'a>(&'a self, cx: &'a Cx, body: Body, next: Next<'a>) -> LayerFuture<'a> {
        Box::pin(async move {
            let path = uri(cx).path();
            let query = uri(cx).query().unwrap_or("");
            // The query with its `lang` parameter dropped, computed once: it is
            // both the target of the parameter's own `301` and what the
            // prefixed spelling redirects with.
            let rest = without_lang(query);

            // 0a. A prefix, or a retired page name — one hop to the address the
            //     page has now. Checked before routing, because a prefixed path
            //     matches no route and a retired one is deliberately not a page.
            if let Some(target) = canonical_address(path) {
                return Ok(negotiation::moved_permanently(&with_query(target, &rest)));
            }

            // 0b. `?lang=xx` is not an address any more. The parameter goes and
            //     the rest of the query stays, so a client that sent
            //     `?lang=ty&s=a` is not sent away from its selection.
            if rest != query {
                return Ok(negotiation::moved_permanently(&with_query(
                    path.to_owned(),
                    &rest,
                )));
            }

            let mut response = next.run(cx, body).await?;

            // Every page is written in a language the client chose, and the URL
            // no longer says which — so the response does.
            if is_page(path) {
                merge_vary(&mut response);
            }

            Ok(response)
        })
    }
}

/// The address `path` has now, when `path` is a spelling this step retired.
///
/// Two retirements, in one answer, so a client makes one hop rather than two:
/// the language prefix (`/ty/himene` → `/himene`) and the page names v4.1 moved
/// ([`crate::pages::retired`]). `None` for a path that is already canonical —
/// including one that merely resembles a retired address, which is nobody's page
/// and is left to the router.
fn canonical_address(path: &str) -> Option<String> {
    let (page, prefixed) = match split_prefix(path) {
        Some((_, page)) => (page, true),
        None => (path, false),
    };

    // Exactly one: `/ty/ty/himene` was never a page addressed at two languages,
    // and this step invents it no answer. It is not a spelling this site ever
    // published, so it 404s like any other path nobody owns.
    if prefixed && split_prefix(page).is_some() {
        return None;
    }

    if let Some(successor) = retired::successor(page) {
        return Some(successor);
    }

    prefixed.then(|| page.to_owned())
}

/// Splits a leading language segment off a path.
///
/// `/ty/himene` → `(Ty, "/himene")`; `/ty` and `/ty/` → `(Ty, "/")`; anything
/// else → `None`. The segment is read with [`Lang::from_tag`], which is exact
/// for a segment that is one of the three codes and nothing else here: a
/// segment with a region in it (`ty-PF`) is not a directory.
fn split_prefix(path: &str) -> Option<(Lang, &str)> {
    let rest = path.strip_prefix('/')?;
    let (segment, _) = rest.split_once('/').unwrap_or((rest, ""));
    let lang = Lang::from_tag(segment)?;

    let page = &path[1 + segment.len()..];
    Some((lang, if page.is_empty() { "/" } else { page }))
}

/// A query string with its `lang` parameter removed.
///
/// Any value, and not only a language the site speaks: the point is that `?lang=`
/// is not a parameter of this site at all, so `?lang=pt` is retired for the same
/// reason `?lang=fr` is. A malformed pair is left alone.
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

/// Whether `path` is one of the site's pages — the set that carries `Vary` and a
/// `<head>`.
///
/// A retired or prefixed address is never one: it is answered by a `301` before
/// this list is consulted. What is left is exactly the pages the router serves.
fn is_page(path: &str) -> bool {
    path == home::PATH
        || path == home::ROOT
        || path == songs::PATH
        || path == editor::PATH
        || path == book::PATH
        || path == search::PATH
        || path == support::PATH
        || artist::segment(path).is_some()
        || songs::page_segment(path).is_some()
        || negotiation::song_segment(path).is_some()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// **No page is addressed at a language code.** The prefix reader treats
    /// `/fr`, `/ty` and `/en` as retired spellings of whatever follows them, so a
    /// page that *began* with one of them would be impossible to reach. Every
    /// page's own path and every machine surface is checked, because the failure
    /// would be one page silently redirecting to another.
    #[test]
    fn no_live_address_starts_with_a_language_code() {
        let paths = [
            home::PATH,
            home::ROOT,
            songs::PATH,
            editor::PATH,
            book::PATH,
            search::PATH,
            support::PATH,
            artist::PREFIX,
            songs::PAGE_PREFIX,
            PATH,
            crate::routes::api::PATH,
            crate::routes::mcp::PATH,
            crate::routes::sitemap::PATH,
            crate::routes::sitemap::SONGS_PATH,
            crate::routes::llms::PATH,
            crate::routes::robots::PATH,
            crate::routes::catalog::PATH,
            crate::routes::card::PATH,
        ];

        for path in paths {
            assert_eq!(
                split_prefix(path),
                None,
                "{path} begins with a language code"
            );
        }

        // The shapes that are not constants: a song, a page of the index and an
        // artist's page.
        for path in [
            "/himene/ahani-e",
            "/himene/page/2",
            "/taata-himene/1kvm9y2tcplm43wgeuni",
        ] {
            assert_eq!(split_prefix(path), None, "{path}");
        }
    }

    /// A prefix is a retired spelling: the address the page has now, and the
    /// language the old URL named is *not* carried anywhere — there is no longer
    /// a language a URL can name.
    #[test]
    fn a_prefixed_address_moves_to_the_address_it_names() {
        assert_eq!(canonical_address("/fr/himene").as_deref(), Some("/himene"));
        assert_eq!(
            canonical_address("/ty/himene/ahani-e").as_deref(),
            Some("/himene/ahani-e")
        );
        assert_eq!(
            canonical_address("/en/puta-himene").as_deref(),
            Some("/puta-himene")
        );
        assert_eq!(canonical_address("/fr").as_deref(), Some("/"));
        assert_eq!(canonical_address("/ty/").as_deref(), Some("/"));
        assert_eq!(
            canonical_address("/en/taata-himene/1kvm9y2tcplm43wgeuni").as_deref(),
            Some("/taata-himene/1kvm9y2tcplm43wgeuni")
        );

        // A machine surface is not language-scoped, but its prefixed spelling is
        // still the prefixed spelling: one hop to the one address.
        assert_eq!(
            canonical_address("/fr/api/songs").as_deref(),
            Some("/api/songs")
        );
        assert_eq!(
            canonical_address("/ty/robots.txt").as_deref(),
            Some("/robots.txt")
        );

        // Two prefixes is nobody's address: `/ty/ty/himene` is not `/ty/himene`
        // twice, and this step invents it no answer.
        assert_eq!(canonical_address("/ty/ty/himene"), None);
        assert_eq!(canonical_address("/xx/himene"), None);
    }

    /// A retired *page* moves to its successor, and a prefixed one moves once —
    /// `/fr/recherche` is `/paimi`, not `/fr/paimi` and not two hops.
    #[test]
    fn a_retired_address_moves_once_to_the_address_that_answers_it() {
        assert_eq!(
            canonical_address("/recherche").as_deref(),
            Some(search::PATH)
        );
        assert_eq!(
            canonical_address("/fr/recherche").as_deref(),
            Some(search::PATH)
        );
        assert_eq!(
            canonical_address("/ty/recherche?q=ahani".split('?').next().unwrap()).as_deref(),
            Some(search::PATH)
        );
        assert_eq!(
            canonical_address("/en/soutenir").as_deref(),
            Some(support::PATH)
        );
        assert_eq!(canonical_address("/aepa").as_deref(), Some(home::PATH));
        assert_eq!(canonical_address("/fr/aepa").as_deref(), Some(home::PATH));
        assert_eq!(
            canonical_address("/himene/pluriel").as_deref(),
            Some(book::PATH)
        );
        assert_eq!(
            canonical_address("/fr/artiste/1kvm9y2tcplm43wgeuni").as_deref(),
            Some("/taata-himene/1kvm9y2tcplm43wgeuni")
        );

        // A page that is live now does not move, and a path that only resembles
        // a retired one is not invented an answer.
        for path in [
            "/",
            "/faariiraa",
            "/himene",
            "/paimi",
            "/tauturu",
            "/puta-himene",
            "/himene/ahani-e",
            "/himene/page/2",
            "/taata-himene/1kvm9y2tcplm43wgeuni",
            "/artiste/a/b",
            "/recherche/x",
            "/api/songs",
            "/reo/ty",
        ] {
            assert_eq!(canonical_address(path), None, "{path}");
        }
    }

    /// `?lang=` is retired whatever it holds, and the rest of the query is kept:
    /// a selection is not thrown away because a client also named a language.
    #[test]
    fn the_lang_parameter_is_dropped_and_the_rest_kept() {
        assert_eq!(without_lang("lang=ty"), "");
        assert_eq!(without_lang("lang=ty&page=2"), "page=2");
        assert_eq!(without_lang("page=2&lang=fr"), "page=2");
        assert_eq!(without_lang("lang=pt"), "");
        assert_eq!(without_lang(""), "");
        assert_eq!(without_lang("s=a&s=b"), "s=a&s=b");

        assert_eq!(with_query("/himene".to_owned(), ""), "/himene");
        assert_eq!(with_query("/himene".to_owned(), "page=2"), "/himene?page=2");
    }

    /// The switcher's link names the language and carries the page, encoded —
    /// a path cannot end the parameter it is written in.
    #[test]
    fn the_switcher_links_to_a_language_and_back_to_the_page() {
        assert_eq!(switch(Lang::Ty, "/himene"), "/reo/ty?next=%2Fhimene");
        assert_eq!(switch(Lang::Fr, "/"), "/reo/fr?next=%2F");
        assert_eq!(
            switch(Lang::En, "/puta-himene?s=a&s=b"),
            "/reo/en?next=%2Fputa-himene%3Fs%3Da%26s%3Db"
        );
        assert_eq!(
            switch(Lang::Ty, "/paimi?q=h%C4%ABmene"),
            "/reo/ty?next=%2Fpaimi%3Fq%3Dh%25C4%25ABmene"
        );
    }

    /// What the switcher wrote is what the route reads: the two halves are one
    /// encoding, and the whole of a path — query string and all — survives.
    #[test]
    fn the_switcher_and_the_route_agree_on_the_page() {
        for next in [
            "/himene",
            "/",
            "/puta-himene?s=a&s=b",
            "/paimi?q=h%C4%ABmene",
            "/himene/page/2",
            "/taata-himene/1kvm9y2tcplm43wgeuni",
        ] {
            let link = switch(Lang::Ty, next);
            let query = link.split_once('?').expect("a query").1;
            assert_eq!(destination(query), next, "{link}");
        }
    }

    /// A `next` that is not a path on this site is refused: the switcher's own
    /// address may not be turned into an open redirect.
    #[test]
    fn a_foreign_next_sends_the_reader_home() {
        for unsafe_next in [
            "https://example.com/",
            "//example.com/",
            "/\\example.com",
            "himene",
            "",
            "/himene\n",
            "/hīmene",
        ] {
            let query = format!("next={}", encode(unsafe_next));
            assert_eq!(destination(&query), "/", "{unsafe_next:?}");
        }

        // No `next` at all is the root, not a 500.
        assert_eq!(destination(""), "/");
        assert_eq!(destination("other=1"), "/");
        assert_eq!(destination("next=%2Fhimene&next=%2Fpaimi"), "/himene");
    }

    /// `Vary` is one field, and it keeps what Topcoat already said: a page's view
    /// layer appends `Accept` on its own, and a cache that read only the first
    /// field would never see the cookie.
    #[test]
    fn vary_is_merged_into_one_field() {
        let mut bare = Response::new(Body::empty());
        merge_vary(&mut bare);
        assert_eq!(bare.headers().get(header::VARY).unwrap(), VARY);
        assert_eq!(bare.headers().get_all(header::VARY).iter().count(), 1);

        let mut page = Response::new(Body::empty());
        page.headers_mut()
            .append(header::VARY, HeaderValue::from_static("Accept"));
        merge_vary(&mut page);
        assert_eq!(
            page.headers().get(header::VARY).unwrap(),
            "Accept, Cookie, Accept-Language"
        );
        assert_eq!(page.headers().get_all(header::VARY).iter().count(), 1);
    }

    /// The `Vary` set is exactly the pages: a machine surface does not read a
    /// cookie and must not tell a cache that it does.
    #[test]
    fn only_pages_vary() {
        for path in [
            home::PATH,
            home::ROOT,
            songs::PATH,
            editor::PATH,
            book::PATH,
            search::PATH,
            support::PATH,
            "/himene/ahani-e",
            "/himene/page/2",
            "/taata-himene/1kvm9y2tcplm43wgeuni",
        ] {
            assert!(is_page(path), "{path}");
        }

        for path in [
            "/api/songs",
            "/api/support",
            "/sitemap.xml",
            "/himene/sitemap.xml",
            "/robots.txt",
            "/llms.txt",
            "/mcp",
            "/.well-known/mcp/server-card.json",
            "/.well-known/api-catalog",
            "/api/openapi.json",
            PATH,
            "/reo/ty",
            "/drive/site",
            "/nobody",
        ] {
            assert!(!is_page(path), "{path}");
        }
    }

    /// The switcher's path is `/reo`, and every link it builds lives under
    /// it — the constant a link is built from and the literal the route is
    /// declared with are one string. `.run/step34.sh` asserts the literal too,
    /// against the source and against the running server.
    #[test]
    fn the_switcher_path_is_the_routes_own() {
        assert_eq!(PATH, "/reo");

        for lang in Lang::ALL {
            let link = switch(lang, "/himene");
            assert!(
                link.starts_with(&format!("{PATH}/{}", lang.code())),
                "{link}"
            );
            assert!(link.contains(PARAM), "{link}");
        }

        // It is not a page: nothing here renders a document or carries `Vary`.
        assert!(!is_page(PATH));
        assert!(!is_page(&switch(Lang::Ty, "/himene")));
    }
}
