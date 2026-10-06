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
use crate::domain::song::{SITE_URL, Song};
use crate::i18n::{self, Key, Lang};
use crate::pages::{
    editor,
    home::{self, AEPA_PATH, PATH as HOME},
    songs,
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
        return missing_song_head(lang);
    }

    site_head(path, lang)
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

        return page_head(title, home::copy::DESCRIPTION, &i18n::url(lang, HOME), lang);
    }

    if path == songs::PATH {
        return page_head(
            format!("{} | {TITLE}", i18n::text(lang, Key::IndexTitle)),
            songs::DESCRIPTION,
            &i18n::url(lang, songs::PATH),
            lang,
        );
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

/// The `<head>` of a page with prose: a title of its own, a description, the
/// canonical URL, and the site's card.
///
/// One function rather than a field per route, because these are the same
/// decision four times over — a page that is worth reading is worth a snippet
/// and a card, and the three pages this step fixes were each missing a
/// different one of the four.
fn page_head(title: String, description: &str, url: &str, lang: Lang) -> DocumentHead {
    DocumentHead {
        social: Some(SocialCards::for_page(&title, description, url, lang)),
        title,
        description: Some(description.to_owned()),
        canonical: Some(url.to_owned()),
        noindex: false,
        jsonld: None,
    }
}

/// The `<head>` of a song URL that names no published song.
///
/// The page it lands on is the branded 404, so its title is the 404's own
/// headline rather than the site's: an address a crawler may still hold from an
/// old link must not answer with the front door's title, which is what made a
/// missing song a duplicate of `/`.
fn missing_song_head(lang: Lang) -> DocumentHead {
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
    let path = uri(cx).path();
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
                        href=(i18n::url(alternate, path))
                    />
                }
                <link rel="alternate" hreflang="x-default" href=(i18n::absolute(path))/>
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
    let on_songs = songs_link.is_current(cx);

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

/// Site footer. v3's wording, kept, including the link to the maintainer's site.
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
    const PAGES: [&str; 4] = [HOME, AEPA_PATH, songs::PATH, editor::PATH];

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

    /// The three pages that have prose to offer a search engine carry all four
    /// fields — a description inside the budget a snippet is read at, the
    /// canonical URL, and a card whose `og:url` is that same URL. Each of them
    /// was missing a different one in v4.
    #[test]
    fn the_prose_pages_have_a_description_a_canonical_and_a_card() {
        for path in [HOME, AEPA_PATH, songs::PATH] {
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

    /// **One canonical per page, and it is the prefixed URL.** The bare URL is
    /// the `x-default`, not a second canonical, and the three languages each
    /// name themselves.
    #[test]
    fn every_page_canonicalises_to_its_own_language_prefix() {
        for lang in Lang::ALL {
            for path in [HOME, AEPA_PATH, songs::PATH] {
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

        assert!(missing_song_head(Lang::Fr).noindex);
    }

    /// A song URL that names no published song is answered by the branded 404, so
    /// its head is the 404's: its own headline, and not the front door's title.
    #[test]
    fn a_missing_song_gets_the_not_found_head() {
        let head = missing_song_head(Lang::Fr);

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
