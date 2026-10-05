//! The page shell: one layout, wrapping every page.
//!
//! Transcribed from v3's `app.rs` (`shell`) + `components/body/{mod,header}.rs`,
//! with five differences, each deliberate and each explained where it happens:
//!
//! * the menu toggle is a checkbox instead of a scripted button,
//! * v3's Google Fonts request is replaced by three self-hosted families,
//! * the document is dark-only, and says so,
//! * the document head is decided from the request path by [`document_head`],
//!   because Topcoat has no per-page `<head>` API, and
//! * the slot is wrapped in an [`error_boundary`], so a page that fails with a
//!   [`NotFoundError`] renders the site's own 404 instead of Topcoat's bare
//!   default. That covers *raised* errors only — a URL matching no route never
//!   reaches the layout at all, which is why `pages::not_found!("/")` also
//!   exists; see `PLAN.md` §17.
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
use crate::pages::{home, songs};
use crate::state;
use crate::ui::{fonts, theme};

/// The document title, for every page that is not a song.
///
/// v3's `<Title text="Chanson du Fenua"/>`: set once on the app root. The
/// capital F is v3's, not a typo — the song page writes the lowercase one, and
/// both are in the wild.
const TITLE: &str = "Chanson du Fenua";

/// Everything the layout needs to write `<head>`.
///
/// Built by [`document_head`] from the request path. The three fields are
/// optional because they are per-route, not per-site: only a song page carries
/// social cards and structured data, and only the home page and its duplicate
/// carry a description.
struct DocumentHead {
    title: String,
    description: Option<String>,
    /// The preferred URL of a page reachable at more than one.
    canonical: Option<String>,
    social: Option<SocialCards>,
    /// A schema.org `application/ld+json` payload, rendered verbatim.
    jsonld: Option<String>,
}

/// The Open Graph and Twitter tags, which v3 emitted per song.
struct SocialCards {
    og_title: String,
    og_description: String,
    og_url: String,
    og_image: String,
    og_image_alt: String,
    twitter_title: String,
    twitter_description: String,
    twitter_image: String,
}

impl SocialCards {
    /// Reads the cards off a song's metadata.
    ///
    /// v3 built the same set from the same `MetaSongData`; the two `twitter_`
    /// fields that differ from their Open Graph twins are kept distinct rather
    /// than aliased, because the image URLs genuinely differ (`/drive/gentw/`
    /// against `/drive/genog/`) and the card is a wider crop.
    fn for_song(meta: &crate::domain::song::MetaSongData) -> Self {
        Self {
            og_title: meta.page_title.clone(),
            og_description: meta.meta_og_description.clone(),
            og_url: meta.meta_og_url.clone(),
            og_image: meta.meta_img_url_og.clone(),
            og_image_alt: meta.meta_og_img_alt.clone(),
            twitter_title: meta.page_title.clone(),
            twitter_description: meta.meta_og_description.clone(),
            twitter_image: meta.meta_img_url_tw.clone(),
        }
    }
}

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
async fn document_head(cx: &Cx) -> DocumentHead {
    let path = uri(cx).path();

    if let Some(id) = path.strip_prefix("/himene/")
        && !id.is_empty()
        && !id.contains('/')
        && let Ok(Some(song)) = db::song(state::db(cx).pool(), id).await
        && song.is_published()
    {
        return song_head(&song);
    }

    site_head(path)
}

/// The `<head>` of a song page, from the song's own metadata.
fn song_head(song: &Song) -> DocumentHead {
    let meta = song.get_meta_data();

    DocumentHead {
        title: meta.page_title.clone(),
        description: Some(meta.meta_description.clone()),
        // The song's canonical URL and its `og:url` are the same thing, which is
        // what v3 emitted — and what the identity rule in `fixing-metadata`
        // asks for.
        canonical: Some(meta.meta_og_url.clone()),
        jsonld: Some(meta.meta_jsonld.clone()),
        social: Some(SocialCards::for_song(&meta)),
    }
}

/// The `<head>` of everything that is not a song.
fn site_head(path: &str) -> DocumentHead {
    // `/` and `/aepa` are one page under two URLs. v3 declared `/aepa` the
    // duplicate, and its canonical URL carries no trailing slash — that is the
    // form the live site emits, so that is the form kept.
    let home = matches!(path, "/" | "/aepa");

    DocumentHead {
        title: TITLE.to_owned(),
        description: home.then(|| home::copy::DESCRIPTION.to_owned()),
        canonical: (path == "/aepa").then(|| SITE_URL.to_owned()),
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
    let head = document_head(cx).await;
    let home_link = href!(home::home);

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
        <html lang="fr" class="dark">
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
                // The social tags, only on a song page. v3 declared them on the
                // song route, so the home page has never carried them.
                match head.social {
                    Some(cards) => {
                        <meta property="fb:app_id" content="383599779228826"/>
                        <meta property="fb:pages" content="109134754150923"/>
                        <meta property="og:type" content="website"/>
                        <meta property="og:locale" content="ty_PF"/>
                        <meta property="og:locale:alternate" content="fr_FR"/>
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
                        <meta name="twitter:image:height" content="628"/>
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
                <link rel="shortcut icon" href="/logos/logo_b32.ico" r#type="image/x-icon" sizes="32x32" media="(prefers-color-scheme: light)"/>
                <link rel="shortcut icon" href="/logos/logo_w32.ico" r#type="image/x-icon" sizes="32x32" media="(prefers-color-scheme: dark)"/>
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
                                    <h1 class=(theme::H1)>"La page n'existe pas."</h1>
                                    <p class=(theme::LEAD)>
                                        "Cette page n'existe pas, ou n'existe plus."
                                    </p>
                                    <a href=(home_link) class=(class!(theme::BUTTON_PRIMARY, theme::FOCUS))>
                                        "Retour à l'accueil"
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
    let home_link = href!(home::home);
    let aepa_link = href!(home::aepa);
    let songs_link = href!(songs::songs);

    let on_aepa = aepa_link.is_current(cx);
    let on_home = home_link.is_current(cx);
    let on_songs = songs_link.is_current(cx);

    // v3 linked "Accueil" at `/aepa` and nothing at `/`. Both are the same page,
    // so both light up for it.
    let on_accueil = on_aepa || on_home;

    Ok(view! {
        <header class=(theme::HEADER)>
            <div class=(theme::HEADER_INNER)>
                <div>
                    <a href=(home_link) class=(class!(theme::FOCUS))>
                        <img
                            class=(theme::LOGO)
                            src="/logos/logo_w144.webp"
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
                        href=(aepa_link)
                        aria-current=(on_accueil.then_some("page"))
                        class=(class!(
                            theme::NAV_LINK,
                            theme::FOCUS,
                            theme::NAV_LINK_CURRENT if on_accueil,
                        ))
                    >
                        "Accueil"
                    </a>
                    <a
                        href=(songs_link)
                        aria-current=(on_songs.then_some("page"))
                        class=(class!(
                            theme::NAV_LINK,
                            theme::FOCUS,
                            theme::NAV_LINK_CURRENT if on_songs,
                        ))
                    >
                        "Chanson"
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
