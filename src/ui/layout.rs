//! The page shell: one layout, wrapping every page.
//!
//! Transcribed from v3's `app.rs` (`shell`) + `components/body/{mod,header}.rs`,
//! with three differences, each deliberate and each explained where it happens:
//!
//! * the menu toggle is a checkbox instead of a scripted button,
//! * the dead `class="dark"` on `<html>` is kept, and
//! * v3's Google Fonts request is replaced by three self-hosted families.
//!
//! The font change is the one visible redesign in this step. v3 asked Google
//! for *Roboto Serif* and then wrote `font-family: Roboto, Arial, serif` — a
//! different family — so the file it downloaded was never applied to anything.
//! v4 names the families it actually ships and serves them from this binary;
//! see [`crate::ui::fonts`].
//!
//! Everything else is the same markup with the same tokens, so this step is a
//! change of mechanism rather than of design.

use topcoat::{
    Result,
    context::Cx,
    router::{Slot, href, layout},
    tailwind,
    view::{View, class, component, view},
};

use crate::pages::{home, songs};
use crate::ui::{fonts, theme};

/// The layout every page renders inside.
///
/// Registered at `/`, so it wraps the whole site. It is discovered by
/// `topcoat::router::RouterBuilderDiscoverExt::discover` — see `crate::router`.
#[layout("/")]
pub async fn root_layout(slot: Slot<'_>) -> Result<impl View> {
    let stylesheet = tailwind::stylesheet!();

    Ok(view! {
        <!DOCTYPE html>
        // `class="dark"` is kept from v3 for markup parity. It does nothing:
        // Tailwind v4's `dark:` compiles to `@media (prefers-color-scheme: dark)`,
        // which is what v3's own stylesheet contained — `.dark` appeared zero
        // times in it. Dark mode follows the OS. Kept so this step changes no
        // visible behaviour; whether to force dark mode is a design call, and
        // `PLAN.md` §14 records it as one.
        <html lang="fr" class="dark">
            <head>
                <meta charset="utf-8"/>
                <meta name="viewport" content="width=device-width, initial-scale=1"/>
                <meta name="theme-color" content="#0891b2" media="(prefers-color-scheme: light)"/>
                <meta name="theme-color" content="#155e75" media="(prefers-color-scheme: dark)"/>
                <title>"Chanson du fenua"</title>
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
                <main class=(theme::MAIN)>(slot)</main>
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
                    <a href=(home_link)>
                        <img
                            class=(theme::LOGO)
                            src="/logos/logo_w144.webp"
                            width="64"
                            height="64"
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
                        class=(class!(theme::NAV_LINK, theme::NAV_LINK_CURRENT if on_accueil))
                    >
                        "Accueil"
                    </a>
                    <a
                        href=(songs_link)
                        aria-current=(on_songs.then_some("page"))
                        class=(class!(theme::NAV_LINK, theme::NAV_LINK_CURRENT if on_songs))
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
                    class=(theme::FOOTER_LINK)
                    href="https://www.rao-nagos.pf"
                    target="_blank"
                    rel="noopener noreferrer"
                >
                    "❤️"
                </a>
                ". Contributing to this "
                <a
                    class=(theme::FOOTER_LINK)
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
