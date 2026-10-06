//! The page shell: one layout, wrapping every page.
//!
//! Transcribed from v3's `app.rs` (`shell`) + `components/body/{mod,header}.rs`,
//! with seven differences, each deliberate and each explained where it happens:
//!
//! * the menu toggle is a checkbox instead of a scripted button,
//! * v3's Google Fonts request is replaced by three self-hosted families,
//! * the document is dark-only, and says so,
//! * the document head is decided from the request path by [`document_head`],
//!   because Topcoat has no per-page `<head>` API,
//! * the slot is wrapped in an [`error_boundary`], so a page that fails with a
//!   [`NotFoundError`] renders the site's own 404 instead of Topcoat's bare
//!   default. That covers *raised* errors only — a URL matching no route never
//!   reaches the layout at all, which is why `pages::not_found!("/")` also
//!   exists, and
//! * the document declares which language it is in, and names the other one,
//!   because the shell is where the chrome's words live. `crate::i18n` decides
//!   the language; this file only asks for the strings, and
//! * the three languages are also a visible switcher — three plain links in the
//!   header, in the served HTML of every page. A page's other addresses are part
//!   of the page, not something a menu draws after a click, and
//! * the three images the browser fetches are embedded in the binary and served
//!   from content-hashed URLs, rather than handed out of a static directory the
//!   way v3's web server did it. v3's own files, under a URL that cannot go
//!   stale — see [`crate::ui::assets`].
//!
//! The font change is the one visible redesign in step 4. v3 asked Google for
//! *Roboto Serif* and then wrote `font-family: Roboto, Arial, serif` — a
//! different family — so the file it downloaded was never applied to anything.
//! v4 names the families it actually ships and serves them from this binary; see
//! [`crate::ui::fonts`].

use topcoat::{
    Result,
    context::Cx,
    router::{Slot, StatusCode, error::NotFoundError, href, layout, request::uri},
    tailwind,
    view::{Unescaped, View, class, component, error_boundary, view},
};

use crate::db;
use crate::domain::Artist;
use crate::domain::song::{SITE_URL, Song};
use crate::i18n::{self, Key, Lang};
use crate::pages::{
    artiste, editor,
    home::{self, AEPA_PATH, PATH as HOME},
    pluriel, recherche, songs, support,
};
use crate::routes::{negotiation, og};
use crate::state;
use crate::ui::{assets, fonts, theme};

/// The site's name — v3's `<Title text="Chanson du Fenua"/>`.
///
/// The capital F is v3's, not a typo: a song page's title ends with the lowercase
/// one, and both are in the wild. It is the site's name, so it is not translated.
/// It is the whole title of the front door and of the pages that name nothing
/// else; every other page says what it is first — see [`site_head`].
const TITLE: &str = "Chanson du Fenua";

/// Everything the layout needs to write `<head>`.
///
/// Built by [`document_head`] from the request path. The optional fields are
/// per-route, not per-site: only the pages that carry prose have a description
/// and a card, only a song has structured data, and only a page reachable at
/// more than one URL has somewhere else to point as canonical.
struct DocumentHead {
    title: String,
    description: Option<String>,
    /// The preferred URL of a page reachable at more than one.
    canonical: Option<String>,
    /// Whether a crawler may index this page.
    ///
    /// False for the create-song form, which is the one page here that is not
    /// written to be found, and for the 404. The directive and `robots.txt` have
    /// to agree for it to mean anything: a path that its own `robots.txt`
    /// disallows is never fetched, so its `<meta name="robots">` is never read.
    noindex: bool,
    social: Option<SocialCards>,
    /// A schema.org `application/ld+json` payload, rendered verbatim.
    jsonld: Option<String>,
}

/// The Open Graph and Twitter tags, which v3 emitted per song.
struct SocialCards {
    /// `og:type`. A song page is `music.song`, which is what the type is for;
    /// any card a later step gives a non-song page declares its own.
    og_type: &'static str,
    og_title: String,
    og_description: String,
    og_url: String,
    og_image: String,
    og_image_alt: String,
    twitter_title: String,
    twitter_description: String,
    twitter_image: String,
    /// `og:locale` and its alternates, for the language the page is served in.
    ///
    /// v3 wrote `ty_PF` and `fr_FR` as constants, Tahitian first. Step 8 makes
    /// them follow the resolved language instead: a card for a page served in
    /// French should say so, which is what `og:locale` is for. The default page
    /// therefore carries `fr_FR` where v3 carried `ty_PF`; `?lang=ty` restores
    /// v3's pair exactly.
    ///
    /// Three languages make it an array: every language that is not the one
    /// served gets its own `og:locale:alternate` tag.
    locale: &'static str,
    locale_alternates: [&'static str; 2],
}

impl SocialCards {
    /// Reads the cards off a song's metadata.
    ///
    /// v3 built the same set from the same `MetaSongData`; the two `twitter_`
    /// fields that differ from their Open Graph twins are kept distinct rather
    /// than aliased, because the image URLs genuinely differ (`/drive/gentw/`
    /// against `/drive/genog/`) and the card is a wider crop.
    fn for_song(meta: &crate::domain::song::MetaSongData, lang: Lang) -> Self {
        Self {
            // `music.song`, not v3's `website`: this is a song, and Open Graph
            // has a type for one.
            og_type: "music.song",
            og_title: meta.page_title.clone(),
            og_description: meta.meta_og_description.clone(),
            og_url: meta.meta_og_url.clone(),
            og_image: meta.meta_img_url_og.clone(),
            og_image_alt: meta.meta_og_img_alt.clone(),
            twitter_title: meta.page_title.clone(),
            twitter_description: meta.meta_og_description.clone(),
            twitter_image: meta.meta_img_url_tw.clone(),
            locale: lang.og_locale(),
            locale_alternates: lang.others().map(Lang::og_locale),
        }
    }

    /// Reads the cards off a page that is not a song.
    ///
    /// The page's own words, and one card image for the whole site: a page that
    /// is not a song has no song to draw, and the alternative — a card per page,
    /// rendered per request — buys nothing a title and a description do not
    /// already say. The image is absolute because a card is read out of context,
    /// and it is the same URL for every language, because it says the site's name
    /// and nothing that is translated.
    ///
    /// `og:url` is the canonical URL, not the requested one: `/aepa` and `/` are
    /// one page, and a card that named the second address would be advertising a
    /// duplicate.
    fn for_page(title: &str, description: &str, url: &str, lang: Lang) -> Self {
        let image = format!("{SITE_URL}{}", og::SITE_CARD);

        Self {
            og_type: "website",
            og_title: title.to_owned(),
            og_description: description.to_owned(),
            og_url: url.to_owned(),
            og_image: image.clone(),
            og_image_alt: SITE_CARD_ALT.to_owned(),
            twitter_title: title.to_owned(),
            twitter_description: description.to_owned(),
            twitter_image: image,
            locale: lang.og_locale(),
            locale_alternates: lang.others().map(Lang::og_locale),
        }
    }
}

/// The site card's alternative text. What the card spells out is its own name.
const SITE_CARD_ALT: &str = "Chanson du fenua";

/// Decides the per-route half of `<head>`.
///
/// Topcoat 0.10 has **no per-page `<head>` API**. A view can declare a status
/// code and response headers, and nothing else document-level; a page cannot set
/// the `<title>` or add a `<meta>`, and the layout cannot ask it for one. So the
/// head belongs to the layout and the per-route facts are decided here, from the
/// path.
///
/// **A song page costs one extra read.** The layout cannot see the row the page
/// loaded, so it loads it again by the same primary key. That is a deliberate
/// trade rather than an oversight: the read is an in-process, indexed lookup on
/// a database that is compiled into this binary, and the alternative is a head
/// that is wrong for every song, which is the whole reason step 7 has a metadata
/// section.
///
/// A path this function cannot resolve — a song that does not exist, an
/// unpublished one, a path below `/himene/` with more segments — falls back to
/// the site head. The page itself is what turns those into a 404.
async fn document_head(cx: &Cx, lang: Lang) -> DocumentHead {
    let path = uri(cx).path();

    // What a song URL is is not decided here: the Markdown layer asks the same
    // question about the same path, and two answers would be one page and its
    // other form disagreeing about which URLs exist. The segment is a slug or an
    // id, so the lookup is the resolver's and not `db::song`'s.
    if let Some(segment) = negotiation::song_segment(path) {
        if let Ok(Some(found)) = db::song_at(state::db(cx).pool(), segment).await
            && found.song().is_published()
        {
            return song_head(found.song(), lang);
        }

        // A song URL is also the only path *in the router* that can still fail:
        // the page reads the same row and raises `NotFoundError`, which the
        // error boundary below answers with the branded 404. So a row that is
        // missing or unpublished is not the site's head — it is the 404's.
        return not_found_head(lang);
    }

    // A page of the index — `/himene/page/{n}`. It gets a head of its own rather
    // than the index's, because a series of pages that all answer with one title
    // and one description is one page in a search engine's eyes — the duplicate
    // cluster the scope names. The number is read here, and how many pages the
    // catalogue has with it, so a number it does not have is not a page at all:
    // it is the branded 404 the page handler raises, and it gets the 404's head.
    if songs::page_segment(path).is_some() {
        let pages = match db::counts(state::db(cx).pool()).await {
            Ok(counts) => songs::page_count(counts.songs),
            // The page's own read fails the same way, so the response is a 500
            // with nothing of the catalogue in it: the site's head is the honest
            // one, and the error is the page's to raise.
            Err(_) => return site_head(path, lang),
        };

        return match songs::page_number(path).filter(|number| (2..=pages).contains(number)) {
            Some(number) => index_head(number, pages, lang),
            None => not_found_head(lang),
        };
    }

    // An artist's page. Read here for the head's sake, exactly as a song's row is
    // read for its own: the layout cannot see the row the page loaded, and a
    // `MusicGroup` that named a different artist than the page prints would be a
    // lie told to a machine. An id that resolves to nothing is the 404 the page
    // handler raises, so it gets the 404's head.
    if let Some(id) = artiste::segment(path) {
        return match db::artist(state::db(cx).pool(), id).await {
            Ok(Some(row)) => artist_head(cx, &row, lang).await,
            // A read that fails is the page's own read failing too: the response
            // is a 500 with nothing of the catalogue in it, and the site's head is
            // the honest one.
            _ => not_found_head(lang),
        };
    }

    // The search page. `noindex` and **no canonical**, the multi-lyric page's
    // decision made for the same reason (see `pages::recherche`): the URL space is
    // every string a person could type, and a canonical URL on a page a crawler is
    // told not to index names a preferred address for nothing. The bare form gets
    // the same head as a search, because there is nothing about a form to
    // describe; the title is chrome and follows the request's language.
    if path == recherche::PATH {
        return DocumentHead {
            title: format!("{} | {TITLE}", i18n::text(lang, Key::SearchTitle)),
            description: None,
            canonical: None,
            noindex: true,
            social: None,
            jsonld: None,
        };
    }

    site_head(path, lang)
}

/// The `<head>` of an artist's page.
///
/// Its own title, its own description, its own canonical URL and its own card,
/// and — the reason this page exists in the search's eyes — its own structured
/// data: a `MusicGroup` for the artist and an `ItemList` of their songs, from the
/// same read the page renders ([`artiste::jsonld`]).
///
/// The songs are read a second time here, after the page's own read. That is the
/// trade the song head already makes and documents: the layout cannot see the
/// row the page loaded, and the alternative is a head written for no artist in
/// particular.
async fn artist_head(cx: &Cx, artist: &Artist, lang: Lang) -> DocumentHead {
    let listed = match db::songs_by_artist(state::db(cx).pool(), &artist.get_id()).await {
        Ok(listed) => listed,
        // The page's own read fails the same way, so the response is a 500 with
        // nothing of the catalogue in it.
        Err(_) => return site_head(&artiste::path_of(&artist.get_id()), lang),
    };

    let name = artist.get_fullname();
    let title = format!("{name} | {TITLE}");
    let description = artiste::description(&name);
    let canonical = i18n::url(lang, &artiste::path_of(&artist.get_id()));

    DocumentHead {
        title: title.clone(),
        description: Some(description.clone()),
        canonical: Some(canonical.clone()),
        noindex: false,
        social: Some(SocialCards::for_page(
            &title,
            &description,
            &canonical,
            lang,
        )),
        jsonld: Some(artiste::jsonld(artist, &canonical, &listed)),
    }
}

/// The `<head>` of one page of the index.
///
/// Its own title and its own description, and they differ from page 1's by
/// exactly one thing: the number. There is no new *word* in either — a page
/// number is a numeral, so the title is the index's own heading with `(2/3)`
/// after it and the description is the catalogue's own sentence with the same
/// — which is what keeps this from being a fourth French sentence to translate
/// for a fact that has no words in it.
///
/// The canonical URL is the page's own path, never the index's and never page
/// 1's: each page of a series consolidates to itself, which is the whole point
/// of serving the numbers on real paths — see [`songs::page_path`].
fn index_head(number: u32, pages: u32, lang: Lang) -> DocumentHead {
    let title = format!(
        "{} ({number}/{pages}) | {TITLE}",
        i18n::text(lang, Key::IndexTitle)
    );
    let description = format!("{} ({number}/{pages})", songs::DESCRIPTION);

    page_head(
        title,
        &description,
        &i18n::url(lang, &songs::page_path(number)),
        lang,
        None,
    )
}

/// The `<head>` of a song page, from the song's own metadata.
///
/// The canonical URL is the one this response is served at: the slug, under the
/// request's language prefix. A song's page is the same document in every
/// language — the lyric is never translated, only the chrome around it changes —
/// but each of the three addresses is the canonical of *that* page, and the
/// alternates are what tie them together. The id and the retired-slug forms are
/// not addresses at all; `routes::negotiation` has already sent them here.
fn song_head(song: &Song, lang: Lang) -> DocumentHead {
    let canonical = i18n::url(lang, &song.get_path());
    let meta = song.get_meta_data(&canonical);

    DocumentHead {
        title: meta.page_title.clone(),
        description: Some(meta.meta_description.clone()),
        noindex: false,
        // The song's canonical URL and its `og:url` are the same thing, which is
        // what v3 emitted — and what the identity rule in `fixing-metadata`
        // asks for.
        canonical: Some(canonical),
        jsonld: Some(meta.meta_jsonld.clone()),
        social: Some(SocialCards::for_song(&meta, lang)),
    }
}

/// The `<head>` of everything that is not a song, by which route it is.
///
/// **One title per URL.** v3 set the site's name once, on the app root, and v4
/// inherited the consequence: `/`, `/aepa` and `/himene` answered with the same
/// `<title>`, which tells a search engine that three addresses are one page. The
/// front door keeps the name; the others say what they are and then name the
/// site, in the chrome's own words, so the title follows the page's language the
/// way the rest of the chrome does.
///
/// The same three pages get a description, a canonical URL and a social card
/// from [`page_head`], because having prose and being shareable are the same
/// condition. `/aepa` is the exception that proves the shape: it is the front
/// page under a second URL, so it shares the description and the canonical that
/// points home, and differs in the one field where two URLs must differ.
///
/// **The canonical URL carries the language prefix.** `/himene` and `/ty/himene`
/// are one page in two languages, and the prefixed form is the one that is
/// canonical; the bare URL is the `x-default` and says so. That is the trade the
/// scope names explicitly: one canonical per page, and the bare URL declaring
/// it rather than competing with it.
fn site_head(path: &str, lang: Lang) -> DocumentHead {
    // `/` and `/aepa` are one page under two URLs. v3 declared `/aepa` the
    // duplicate, and its canonical URL carries no trailing slash — that is the
    // form the live site emits, so that is the form kept.
    if matches!(path, HOME | AEPA_PATH) {
        let title = if path == AEPA_PATH {
            format!("{} | {TITLE}", i18n::text(lang, Key::NavHome))
        } else {
            TITLE.to_owned()
        };

        // The front door describes the site once, and `/aepa` — the same page
        // under a second URL — does not repeat it: what the structured data
        // describes is the site, and two copies on two URLs is one description
        // competing with itself.
        let jsonld = (path == HOME).then(website_jsonld);

        return page_head(
            title,
            home::copy::DESCRIPTION,
            &i18n::url(lang, HOME),
            lang,
            jsonld,
        );
    }

    if path == songs::PATH {
        return page_head(
            format!("{} | {TITLE}", i18n::text(lang, Key::IndexTitle)),
            songs::DESCRIPTION,
            &i18n::url(lang, songs::PATH),
            lang,
            None,
        );
    }

    // The multi-lyric page — a chosen set of songs, or the picker that builds
    // one. `noindex`, and **no canonical**, decided together and deliberately
    // (see `pages::pluriel`): the URL space is every ordered subset of the
    // catalogue, so an index full of selections would be duplicate content built
    // out of the sheets it quotes, and a canonical URL would name a preferred
    // address for a page a crawler is being told not to index. What the title
    // says is what the page is, and the songs inside it are the sheets' own
    // pages — linked, canonical, and indexable.
    if path == pluriel::PATH {
        return DocumentHead {
            title: format!("{} | {TITLE}", i18n::text(lang, Key::PlurielTitle)),
            description: None,
            canonical: None,
            noindex: true,
            social: None,
            jsonld: None,
        };
    }

    // The create-song page is the one page here that is not written to be found:
    // it holds a form. It says what it is for, which is what a `<title>` is for,
    // and it is the only page that is kept out of an index.
    if path == editor::PATH {
        return DocumentHead {
            title: format!("{} | {TITLE}", i18n::text(lang, Key::AddLyrics)),
            description: None,
            canonical: None,
            noindex: true,
            social: None,
            jsonld: None,
        };
    }

    // The support page. A page with prose of its own — what it is for, and the
    // addresses — so it takes a description, a canonical URL and a card, the
    // same way the index and an artist's page do. Its title is chrome; its
    // description is the catalogue's own French, like every other page's.
    if path == support::PATH {
        return page_head(
            format!("{} | {TITLE}", i18n::text(lang, Key::SupportTitle)),
            support::DESCRIPTION,
            &i18n::url(lang, support::PATH),
            lang,
            None,
        );
    }

    // A path no route claims never reaches the layout at all — the router
    // answers it — so what arrives here is a page rendered outside the three
    // above, and the honest head for one is the site's name and nothing more
    // than that.
    DocumentHead {
        title: TITLE.to_owned(),
        description: None,
        canonical: None,
        noindex: false,
        social: None,
        jsonld: None,
    }
}

/// The request's path as *this page's* identity: what its `hreflang` cluster and
/// its `x-default` name.
///
/// For every page but one that is the path alone. A query string is a view of a
/// document rather than a second document — that is the rule `?tr=` on a sheet
/// follows, which is why a transposed sheet declares the untransposed URL as its
/// canonical and its alternates name the same sheet in the other two languages.
///
/// The multi-lyric page is the exception, and it is what makes the rule visible:
/// there the query **is** the page — `/himene/pluriel` with no selection is the
/// picker, and a selection is a different document — so dropping it would make
/// every selection declare the picker in three languages as its own alternate.
fn addressed_path(cx: &Cx) -> String {
    let request = uri(cx);
    let path = request.path();

    match request.query() {
        Some(query) if path == pluriel::PATH => format!("{path}?{query}"),
        _ => path.to_owned(),
    }
}

/// The `<head>` of a page with prose: a title of its own, a description, the
/// canonical URL, and the site's card.
///
/// One function rather than a field per route, because these are the same
/// decision four times over — a page that is worth reading is worth a snippet
/// and a card, and the three pages this step fixes were each missing a
/// different one of the four.
fn page_head(
    title: String,
    description: &str,
    url: &str,
    lang: Lang,
    jsonld: Option<String>,
) -> DocumentHead {
    DocumentHead {
        social: Some(SocialCards::for_page(&title, description, url, lang)),
        title,
        description: Some(description.to_owned()),
        canonical: Some(url.to_owned()),
        noindex: false,
        jsonld,
    }
}

/// The front door's structured data: the site, and the box that searches it.
///
/// A `WebSite` node with a `SearchAction` is what makes a search box appear in a
/// result for the site's own name, and it is a statement about the site rather
/// than about a page — so it is written on `/` alone, in the language-neutral
/// form (`x-default`), because the action it describes is available in every
/// language at the same URL.
///
/// `urlTemplate` names [`recherche::PATH`] through its own constant, so the
/// template and the page cannot drift; the placeholder is schema.org's own
/// required name, not a translatable string.
fn website_jsonld() -> String {
    serde_json::json!({
        "@context": "https://schema.org",
        "@type": "WebSite",
        "name": TITLE,
        "url": SITE_URL,
        "inLanguage": Lang::DEFAULT.code(),
        "potentialAction": {
            "@type": "SearchAction",
            "target": {
                "@type": "EntryPoint",
                "urlTemplate": format!("{}{}?{}={{search_term_string}}", SITE_URL, recherche::PATH, recherche::PARAM),
            },
            "query-input": "required name=search_term_string",
        },
    })
    .to_string()
}

/// The `<head>` of a URL that names no page: a song that is not published, or a
/// page of the index the catalogue does not have.
///
/// The page it lands on is the branded 404, so its title is the 404's own
/// headline rather than the site's: an address a crawler may still hold from an
/// old link must not answer with the front door's title, which is what made a
/// missing song a duplicate of `/`. No canonical either — there is nothing here
/// to consolidate, and the same rule the create-song page follows.
fn not_found_head(lang: Lang) -> DocumentHead {
    DocumentHead {
        title: format!("{} | {TITLE}", i18n::text(lang, Key::NotFoundTitle)),
        description: None,
        canonical: None,
        noindex: true,
        social: None,
        jsonld: None,
    }
}

/// The layout every page renders inside.
///
/// Registered at `/`, so it wraps the whole site. It is discovered by
/// `topcoat::router::RouterBuilderDiscoverExt::discover` — see `crate::router`.
#[layout("/")]
pub async fn root_layout(cx: &Cx, slot: Slot<'_>) -> Result<impl View> {
    let stylesheet = tailwind::stylesheet!();
    let lang = i18n::resolve(cx);
    let head = document_head(cx, lang).await;
    let home_link = href!(home::home);
    let path = addressed_path(cx);
    // The 404's way home carries the reader's language too, and the closure
    // below cannot borrow `cx` to build it, so it is resolved here.
    let home_href = i18n::link(cx, &home_link.resolve(cx));

    Ok(view! {
        <!DOCTYPE html>
        // `class="dark"` is kept from v3. It is inert — Tailwind v4's `dark:`
        // variant compiles to `@media (prefers-color-scheme: dark)` and v4 uses
        // no `dark:` utilities at all — but it is the conventional marker, and
        // removing it would be a change to the markup that buys nothing.
        //
        // The scheme itself is declared in the generated stylesheet: `build.rs`
        // writes a base rule alongside the `@theme` block. A dark document that
        // did not declare its scheme gets light native scrollbars and form
        // controls, which reads as a rendering fault.
        //
        // `lang` is the request's language, not a constant: it is what tells a
        // screen reader and a search engine which of the site's two languages
        // this response is written in. See `crate::i18n`.
        <html lang=(lang.code()) class="dark">
            <head>
                <meta charset="utf-8"/>
                <meta name="viewport" content="width=device-width, initial-scale=1"/>
                <meta name="theme-color" content="#081418"/>
                <title>(head.title)</title>
                // `<head>` elements are rendered where they are written —
                // Topcoat has no mechanism to hoist a page's `<meta>` up here,
                // which is the whole reason `document_head` exists.
                match head.description {
                    Some(description) => <meta name="description" content=(description)/>,
                    None => "",
                }
                match head.canonical {
                    Some(canonical) => <link rel="canonical" href=(canonical)/>,
                    None => "",
                }
                // The one page here that is not written to be found. It is a
                // `<meta>` and not a `Disallow` because a disallowed URL is never
                // fetched, and a directive no crawler reads is not a directive —
                // see `crate::routes::robots`, which is where the two agree.
                match head.noindex {
                    true => <meta name="robots" content="noindex, follow"/>,
                    false => "",
                }
                // The language alternates. Every page exists in all three
                // languages at a prefix of its own, and the prefix is the only
                // difference: so each page names all of them, itself included,
                // which is what a `hreflang` cluster is and what tells a search
                // engine that the three URLs are one page rather than duplicates
                // competing for the same query.
                //
                // The URLs are origin-qualified because a search engine reads
                // them out of context, and `x-default` points at the page with
                // no prefix — the form a reader who has expressed no preference
                // should land on, and the one Cloudflare may cache for everyone.
                for alternate in Lang::ALL {
                    <link
                        rel="alternate"
                        hreflang=(alternate.code())
                        href=(i18n::url(alternate, &path))
                    />
                }
                <link rel="alternate" hreflang="x-default" href=(i18n::absolute(&path))/>
                // The social tags, only on a song page. v3 declared them on the
                // song route, so the home page has never carried them.
                match head.social {
                    Some(cards) => {
                        <meta property="fb:app_id" content="383599779228826"/>
                        <meta property="fb:pages" content="109134754150923"/>
                        <meta property="og:type" content=(cards.og_type)/>
                        <meta property="og:locale" content=(cards.locale)/>
                        for alternate in cards.locale_alternates {
                            <meta property="og:locale:alternate" content=(alternate)/>
                        }
                        <meta property="og:title" content=(cards.og_title)/>
                        <meta property="og:description" content=(cards.og_description)/>
                        <meta property="og:url" content=(cards.og_url)/>
                        <meta property="og:image" content=(cards.og_image)/>
                        <meta property="og:image:alt" content=(cards.og_image_alt)/>
                        <meta property="og:image:width" content="1200"/>
                        <meta property="og:image:height" content="630"/>
                        <meta property="og:image:type" content="image/png"/>
                        <meta name="twitter:card" content="summary_large_image"/>
                        <meta name="twitter:title" content=(cards.twitter_title)/>
                        <meta name="twitter:description" content=(cards.twitter_description)/>
                        <meta name="twitter:image" content=(cards.twitter_image)/>
                        <meta name="twitter:image:width" content="1200"/>
                        <meta name="twitter:image:height" content="630"/>
                        <meta name="twitter:creator" content="@raonagos"/>
                        <meta name="twitter:site" content="@raonagos"/>
                    },
                    None => "",
                }
                // Structured data. Rendered *unescaped*, because a `<script>`
                // is a raw-text element: HTML entity escaping inside one is not
                // decoded by the browser, so an escaped JSON-LD block would be
                // broken JSON rather than a safe one. The escaping that matters
                // happened in the domain — `Song::to_jsonld` serialises through
                // `serde_json`, and the only string in it a visitor can write
                // is the chord-free lyric, which has already been through
                // `ammonia`. See `Unescaped`'s contract in `topcoat::view`.
                match head.jsonld {
                    Some(jsonld) => {
                        <script type="application/ld+json">
                            (Unescaped::new_unchecked(jsonld))
                        </script>
                    },
                    None => "",
                }
                <link rel="shortcut icon" href=(assets::ICON_LIGHT) r#type="image/x-icon" sizes="32x32" media="(prefers-color-scheme: light)"/>
                <link rel="shortcut icon" href=(assets::ICON_DARK) r#type="image/x-icon" sizes="32x32" media="(prefers-color-scheme: dark)"/>
                <link rel="stylesheet" href=(stylesheet)/>
                // The `@font-face` rules, served from this binary at
                // `/_topcoat/fonts/…`.
                //
                // `preload: false` deliberately. `link(font:)` would otherwise
                // emit a `rel="preload"` per face, and a preload bypasses
                // `unicode-range` — it fetches the file unconditionally. Three
                // families across two subsets is ten faces, so the default would
                // force ten downloads on every page, including the Latin
                // Extended files that only a song with macrons ever needs.
                // Letting the browser find the faces through the stylesheet
                // costs one round trip and fetches only what the page draws.
                topcoat::font::link(font: fonts::LITERATA, preload: false)
                topcoat::font::link(font: fonts::FRAUNCES, preload: false)
                topcoat::font::link(font: fonts::JETBRAINS_MONO, preload: false)
            </head>
            <body class=(theme::SHELL)>
                header()
                <main class=(theme::MAIN)>
                    // A 404 from any page becomes a page. Every other error is
                    // rethrown and answered the way Topcoat would have.
                    error_boundary(
                        fallback: |error| {
                            if error.downcast_ref::<NotFoundError>().is_none() {
                                return Err(error);
                            }
                            Ok(view! {
                                (StatusCode::NOT_FOUND)
                                <section class=(theme::NOT_FOUND)>
                                    <h1 class=(theme::H1)>
                                        (i18n::text(lang, Key::NotFoundTitle))
                                    </h1>
                                    <p class=(theme::LEAD)>
                                        (i18n::text(lang, Key::NotFoundBody))
                                    </p>
                                    <a href=(home_href) class=(class!(theme::BUTTON_PRIMARY, theme::FOCUS))>
                                        (i18n::text(lang, Key::NotFoundCta))
                                    </a>
                                </section>
                            })
                        },
                        (slot)
                    )
                </main>
                footer()
            </body>
        </html>
    })
}

/// Site header: logo, menu toggle, nav.
///
/// No JavaScript. The toggle is a checkbox and the nav's open state is
/// Tailwind's `peer-checked` variant — see [`theme::NAV_TOGGLE`], which records
/// why v3's scripted hamburger was replaced rather than ported.
#[component]
pub async fn header(cx: &Cx) -> Result<impl View> {
    let lang = i18n::resolve(cx);
    let home_link = href!(home::home);
    let aepa_link = href!(home::aepa);
    let songs_link = href!(songs::songs);

    let on_aepa = aepa_link.is_current(cx);
    let on_home = home_link.is_current(cx);
    let on_songs = songs_link.is_current(cx) || names_the_index(uri(cx).path());

    // v3 linked "Accueil" at `/aepa` and nothing at `/`. Both are the same page,
    // so both light up for it.
    let on_accueil = on_aepa || on_home;

    // Every link the chrome emits is the canonical, language-prefixed form of
    // the page: a reader on `/` who clicks "Chanson" lands on `/fr/himene`, which
    // is the address that page is published at. `is_current` is asked of the
    // route rather than of the URL, so the highlight survives the prefix.
    let home_href = i18n::link(cx, &home_link.resolve(cx));
    let aepa_href = i18n::link(cx, &aepa_link.resolve(cx));
    let songs_href = i18n::link(cx, &songs_link.resolve(cx));

    Ok(view! {
        <header class=(theme::HEADER)>
            <div class=(theme::HEADER_INNER)>
                <div>
                    <a href=(home_href) class=(class!(theme::FOCUS))>
                        <img
                            class=(theme::LOGO)
                            src=(assets::LOGO)
                            width="48"
                            height="48"
                            alt="logo chanson du fenua"
                        />
                    </a>
                </div>
                <span class="flex-1"></span>
                language_switcher()
                // Must stay a *previous sibling* of the nav for `peer-checked`
                // to reach it.
                <input id="nav-toggle" type="checkbox" class=(theme::NAV_TOGGLE)/>
                <label for="nav-toggle" aria-label="Toggle menu" class=(theme::HAMBURGER_BUTTON)>
                    <span class=(theme::HAMBURGER_BAR)></span>
                    <span class=(theme::HAMBURGER_BAR_ANIMATED)></span>
                    <span class=(theme::HAMBURGER_BAR_ANIMATED)></span>
                </label>
                <nav id="navigation" class=(theme::NAV)>
                    <span class=(theme::NAV_SPACER)></span>
                    <a
                        href=(aepa_href)
                        aria-current=(on_accueil.then_some("page"))
                        class=(class!(
                            theme::NAV_LINK,
                            theme::FOCUS,
                            theme::NAV_LINK_CURRENT if on_accueil,
                        ))
                    >
                        (i18n::text(lang, Key::NavHome))
                    </a>
                    <a
                        href=(songs_href)
                        aria-current=(on_songs.then_some("page"))
                        class=(class!(
                            theme::NAV_LINK,
                            theme::FOCUS,
                            theme::NAV_LINK_CURRENT if on_songs,
                        ))
                    >
                        (i18n::text(lang, Key::NavSongs))
                    </a>
                </nav>
            </div>
        </header>
    })
}

/// The language switcher: this page, in each of the site's three languages.
///
/// Three real links with no script anywhere near them. The set of addresses a
/// page exists at is exactly what a crawler needs to see, so it is written into
/// the HTML of every page rather than drawn by a menu that only opens for a
/// pointer. `hreflang` says which language a link leads to and `lang` says which
/// language its own label is written in — the pair is what lets a screen reader
/// say *Reo Tahiti* in Tahitian while reading an English page.
///
/// The current language is a link like the others. A switcher that turns it into
/// plain text reads as "you cannot go here", and `/fr/himene` linked from
/// `/fr/himene` is what makes the three addresses one cluster instead of a
/// one-way door. The one difference is `aria-current`, which is what tells a
/// reader which of the three they are on when the underline is not enough.
///
/// The three URLs are [`Lang::ALL`] under their own prefixes — the same list the
/// document head writes `hreflang` from, and the same `addressed_path`, so the
/// switcher and the cluster cannot disagree about which page is being switched:
/// a reader who chooses a language on a selection keeps the selection.
#[component]
pub async fn language_switcher(cx: &Cx) -> Result<impl View> {
    let lang = i18n::resolve(cx);
    let path = addressed_path(cx);

    Ok(view! {
        <nav class=(theme::LANGUAGE_SWITCH) aria-label=(i18n::text(lang, Key::Language))>
            for target in Lang::ALL {
                <a
                    href=(i18n::at(target, &path))
                    hreflang=(target.code())
                    lang=(target.code())
                    aria-current=((target == lang).then_some("true"))
                    class=(class!(
                        theme::LANGUAGE_LINK,
                        theme::FOCUS,
                        theme::LANGUAGE_LINK_CURRENT if target == lang,
                    ))
                >
                    (target.name())
                </a>
            }
        </nav>
    })
}

/// Whether a path is the song index — `/himene`, or one of its later pages.
///
/// Topcoat's `is_current` compares the **handler the router matched**, and a page
/// of the index is a handler of its own, so `href!(songs::songs)` answers `false`
/// on `/himene/page/2` and the nav would go dark on every page but the first.
/// This is the widening the scope asks for, and it is asked of the *path* rather
/// than of the handler because there is no handler for "the index" — there are
/// two, and the pages they serve are one series.
///
/// The path here is the request's own, after the language layer has rewritten
/// `/ty/himene/page/2` to `/himene/page/2`, so a prefixed page highlights its nav
/// for the same reason the unprefixed one does.
fn names_the_index(path: &str) -> bool {
    path == songs::PATH || songs::page_segment(path).is_some()
}

/// The site footer. v3's wording, kept, including the link to the maintainer's site.
#[component]
pub async fn footer() -> Result<impl View> {
    Ok(view! {
        <footer class=(theme::FOOTER)>
            <p>
                "2024 Chanson du fenua. Tous droits réservés "
                <a
                    class=(theme::LINK)
                    href="https://www.rao-nagos.pf"
                    target="_blank"
                    rel="noopener noreferrer"
                >
                    "❤️"
                </a>
                ". Contributing to this "
                <a
                    class=(theme::LINK)
                    href="https://github.com/raonagos/chansondufenua"
                    target="_blank"
                    rel="noopener noreferrer"
                >
                    "project"
                </a>
                "."
            </p>
        </footer>
    })
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use super::*;
    use crate::domain::song::DESCRIPTION_MAX;

    /// Every page the router serves, in the order this module decides them.
    const PAGES: [&str; 5] = [HOME, AEPA_PATH, songs::PATH, editor::PATH, support::PATH];

    /// `<title>` is the one field a search result leads with, and two URLs
    /// answering with the same one tells a crawler they are the same page. Three
    /// of these used to answer with the site's own name.
    #[test]
    fn no_two_pages_share_a_title() {
        let titles: Vec<String> = PAGES
            .iter()
            .map(|path| site_head(path, Lang::Fr).title)
            .collect();
        let unique: BTreeSet<&String> = titles.iter().collect();

        assert_eq!(unique.len(), titles.len(), "{titles:?}");
    }

    /// The pages that have prose to offer a search engine carry all four
    /// fields — a description inside the budget a snippet is read at, the
    /// canonical URL, and a card whose `og:url` is that same URL. Each of them
    /// was missing a different one in v4, and the support page (step 30) joined
    /// them by having all four from its first day.
    #[test]
    fn the_prose_pages_have_a_description_a_canonical_and_a_card() {
        for path in [HOME, AEPA_PATH, songs::PATH, support::PATH] {
            let head = site_head(path, Lang::Fr);
            let description = head.description.expect("a description");
            let canonical = head.canonical.expect("a canonical URL");
            let cards = head.social.expect("social cards");

            assert!(
                description.chars().count() <= DESCRIPTION_MAX,
                "{path} describes itself in {} characters",
                description.chars().count()
            );
            assert!(canonical.starts_with(SITE_URL), "{path}: {canonical}");
            assert_eq!(cards.og_url, canonical, "{path}");
            assert_eq!(cards.og_type, "website", "{path}");
            assert_eq!(cards.og_description, description, "{path}");
            assert_eq!(cards.twitter_description, description, "{path}");
            assert!(
                cards.og_image.starts_with(SITE_URL),
                "{path}: {}",
                cards.og_image
            );
            assert!(!head.noindex, "{path}");
        }
    }

    /// The front page under its second address is the same page, so it points at
    /// the same canonical URL and offers the same description — and it still
    /// names itself in its own title.
    #[test]
    fn the_two_front_page_urls_point_home() {
        let home = site_head(HOME, Lang::Fr);
        let aepa = site_head(AEPA_PATH, Lang::Fr);

        assert_eq!(
            home.canonical.as_deref(),
            Some(i18n::url(Lang::Fr, HOME).as_str())
        );
        assert_eq!(home.canonical, aepa.canonical);
        assert_eq!(home.description, aepa.description);
        assert_ne!(home.title, aepa.title);
    }

    /// The front door names the site and the box that searches it, and `/aepa`
    /// — the same page under a second URL — does not repeat either.
    ///
    /// The `SearchAction`'s template is the search page's own path and parameter,
    /// through their constants: a template and a form that disagree would send a
    /// client to a URL this site does not answer.
    #[test]
    fn the_front_door_describes_the_site_and_its_search() {
        let home = site_head(HOME, Lang::Fr);
        let jsonld = home.jsonld.expect("the front door carries structured data");
        let document: serde_json::Value = serde_json::from_str(&jsonld).expect("valid JSON");

        assert_eq!(document["@type"], "WebSite");
        assert_eq!(document["url"], SITE_URL);
        assert_eq!(document["name"], TITLE);
        assert_eq!(
            document["potentialAction"]["@type"], "SearchAction",
            "the search box is missing"
        );
        let template = format!(
            "{}{}?{}={{search_term_string}}",
            SITE_URL,
            recherche::PATH,
            recherche::PARAM
        );
        assert_eq!(
            document["potentialAction"]["target"]["urlTemplate"], template,
            "the template does not name the search page"
        );
        assert_eq!(
            document["potentialAction"]["query-input"],
            "required name=search_term_string"
        );

        assert!(
            site_head(AEPA_PATH, Lang::Fr).jsonld.is_none(),
            "/aepa repeats the site description"
        );
        assert!(site_head(songs::PATH, Lang::Fr).jsonld.is_none());
    }

    /// The index is the one page that is not the front door and has an address of
    /// its own to canonicalise to.
    #[test]
    fn the_index_canonicalises_to_its_own_path() {
        let head = site_head(songs::PATH, Lang::Fr);

        assert_eq!(
            head.canonical,
            Some(i18n::url(Lang::Fr, songs::PATH).to_owned())
        );
        assert_ne!(head.canonical, Some(format!("{SITE_URL}{}", songs::PATH)));
    }

    /// A page of the index is its own page: its own number in the title and the
    /// description, and its own URL as the canonical — never the index's, which
    /// would make page 2 a duplicate of page 1 and the series a cluster
    /// competing with itself.
    #[test]
    fn a_page_of_the_index_names_its_own_number_and_its_own_url() {
        let second = index_head(2, 3, Lang::Fr);

        assert_eq!(second.title, format!("Toutes les chansons (2/3) | {TITLE}"));
        assert!(
            second
                .description
                .as_deref()
                .expect("a description")
                .ends_with("(2/3)")
        );
        assert_eq!(
            second.canonical,
            Some(i18n::url(Lang::Fr, "/himene/page/2"))
        );
        assert_eq!(
            second.social.as_ref().expect("cards").og_url,
            second.canonical.clone().expect("a canonical URL")
        );
        assert!(!second.noindex);

        // Two pages of one series share no field a search engine leads with.
        let third = index_head(3, 3, Lang::Fr);
        assert_ne!(second.title, third.title);
        assert_ne!(second.description, third.description);
        assert_ne!(second.canonical, third.canonical);

        // And page 1 is not one of these: its address is `/himene`, and it keeps
        // the head it has always had — the index's title, with no number in it.
        assert_eq!(songs::page_path(1), songs::PATH);
        assert_eq!(
            site_head(songs::PATH, Lang::Fr).title,
            format!("Toutes les chansons | {TITLE}")
        );
    }

    /// The nav's "Chanson" link is current on the index **and on its later
    /// pages**. Topcoat compares the handler it matched, and a page of the index
    /// is a handler of its own, so without this widening the nav would go dark
    /// from page 2 on — the defect the scope names.
    #[test]
    fn the_header_stays_current_on_every_page_of_the_index() {
        for path in [songs::PATH, "/himene/page/2", "/himene/page/43"] {
            assert!(names_the_index(path), "{path}");
        }

        // A song under the same prefix is not the index, and neither is the
        // multi-lyric page: the nav would be lying about which page the reader
        // is on.
        for path in [
            HOME,
            AEPA_PATH,
            "/himene/ahani-e",
            "/himene/pluriel",
            "/himene/sitemap.xml",
            editor::PATH,
        ] {
            assert!(!names_the_index(path), "{path}");
        }
    }

    /// **One canonical per page, and it is the prefixed URL.** The bare URL is
    /// the `x-default`, not a second canonical, and the three languages each
    /// name themselves.
    #[test]
    fn every_page_canonicalises_to_its_own_language_prefix() {
        for lang in Lang::ALL {
            for path in [HOME, AEPA_PATH, songs::PATH, support::PATH] {
                let head = site_head(path, lang);
                let canonical = head.canonical.expect("a canonical URL");

                // `/aepa` is the front page under a second URL: it canonicalises
                // to the front page, in the language it was served in.
                let canonical_path = if path == AEPA_PATH { HOME } else { path };
                assert_eq!(
                    canonical,
                    i18n::url(lang, canonical_path),
                    "{path} in {}",
                    lang.code()
                );
                assert!(
                    i18n::at(lang, canonical_path).starts_with(lang.prefix()),
                    "{path} in {}: {canonical}",
                    lang.code()
                );
                assert_eq!(head.social.expect("cards").og_url, canonical);
            }
        }
    }

    /// **The switcher and the `hreflang` cluster are the same three URLs.** Both
    /// are built from [`Lang::ALL`] and both address a page under its language's
    /// own prefix, so the served markup points a reader and a crawler at the same
    /// three addresses. Two lists that can drift are one list that has drifted;
    /// this is the check that they have not.
    ///
    /// It also pins the two properties a switcher is easy to get wrong: it
    /// addresses *this* page (not the front door, and not the other language's
    /// index), and its own language is among its links rather than left out.
    #[test]
    fn the_switcher_and_the_hreflang_cluster_name_the_same_urls() {
        for path in [
            HOME,
            AEPA_PATH,
            songs::PATH,
            editor::PATH,
            support::PATH,
            "/himene/ahani-e",
        ] {
            let links: Vec<(Lang, String)> = Lang::ALL
                .iter()
                .map(|lang| (*lang, i18n::at(*lang, path)))
                .collect();

            assert_eq!(links.len(), Lang::ALL.len());

            for (lang, href) in &links {
                assert!(
                    href.starts_with(lang.prefix()),
                    "{path} in {}: {href}",
                    lang.code()
                );
                assert!(
                    href.ends_with(path) || path == HOME,
                    "{path} in {}",
                    lang.code()
                );
                // The head writes the same address, qualified.
                assert_eq!(
                    i18n::url(*lang, path),
                    format!("{SITE_URL}{href}"),
                    "{path} in {}",
                    lang.code()
                );
            }

            // The current language is one of the links, not a hole in the row.
            assert!(links.iter().any(|(lang, _)| *lang == Lang::DEFAULT));
        }
    }

    /// A language's own name is what the link says: the switcher is the one
    /// place the chrome does not translate, and that is deliberate.
    #[test]
    fn the_switcher_labels_each_language_in_its_own_words() {
        for lang in Lang::ALL {
            assert!(!lang.name().trim().is_empty());
            assert_ne!(lang.name(), i18n::text(lang, Key::Language));
        }
    }

    /// The create-song page is the one page kept out of an index; everything else
    /// the router serves may be indexed, and a 404 never is.
    #[test]
    fn only_the_create_song_page_is_kept_out_of_an_index() {
        for path in PAGES {
            assert_eq!(
                site_head(path, Lang::Fr).noindex,
                path == editor::PATH,
                "{path}"
            );
        }

        assert!(not_found_head(Lang::Fr).noindex);
    }

    /// A song URL that names no published song is answered by the branded 404, so
    /// its head is the 404's: its own headline, and not the front door's title.
    #[test]
    fn a_missing_song_gets_the_not_found_head() {
        let head = not_found_head(Lang::Fr);

        assert!(
            head.title
                .contains(i18n::text(Lang::Fr, Key::NotFoundTitle))
        );
        assert!(head.title.contains(TITLE));
        assert!(head.description.is_none());
        assert!(head.canonical.is_none());
        assert!(head.social.is_none());
    }
}
