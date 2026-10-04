//! Design tokens — the whole of v4's styling vocabulary.
//!
//! Every entry is a Tailwind utility list stored as a [`StaticClass`], so the
//! markup in `src/ui/` and `src/pages/` reads as composition rather than as
//! markup full of class soup. v3 expressed the same tokens as `@apply` blocks in
//! `style/tailwind.scss`; they are transcribed here utility for utility, so the
//! rewrite is a repaint of the *mechanism*, not of the design.
//!
//! Two rules this module exists to enforce:
//!
//! 1. **No styling lives outside it.** Pages compose tokens; they do not invent
//!    colours or spacing. If a page needs a new look, the token belongs here.
//! 2. **Class names stay literal.** Tailwind scans source text for class names;
//!    it cannot see one assembled at runtime. Every name below is inside a
//!    `class!("…")` literal, which is why no const is built with `format!` or
//!    `concat!`.
//!
//! Colours (`tahiti-*`) are published from [`crate::ui::palette`], which
//! `build.rs` turns into Tailwind's `@theme` block at build time. See
//! `src/ui/palette.rs` for why they are v3's hex values rather than Tailwind's
//! `cyan-*`, and `build.rs` for why no `.css` file is involved.

use topcoat::view::{StaticClass, class};

// ---------------------------------------------------------------------------
// Document shell
// ---------------------------------------------------------------------------

/// `<body>`. v3: gradient background, `text-neutral-200`, and the
/// `min-height/display/flex-direction` trio from `style/main.scss` that made the
/// footer stick to the bottom of short pages.
///
/// The `print:` entries are new in v4, and they are the point of a songbook:
/// printing v3 put a dark gradient behind the lyrics and burned the palette on
/// paper. Print gets white paper and black ink.
pub const SHELL: StaticClass = class!(
    "min-h-screen flex flex-col bg-gradient-to-br from-tahiti-500 to-tahiti-800 \
     dark:from-tahiti-700 dark:to-tahiti-1000 text-neutral-200 \
     print:bg-none print:bg-white print:text-black"
);

// ---------------------------------------------------------------------------
// Header and navigation
// ---------------------------------------------------------------------------

/// `<header>`. Hidden in print: a printed song needs the lyrics, not the chrome.
pub const HEADER: StaticClass = class!("bg-tahiti-900/30 p-4 shadow-md print:hidden");

/// The flex row inside `<header>`: logo hard left, everything else right.
pub const HEADER_INNER: StaticClass =
    class!("container mx-auto flex items-center justify-end flex-wrap");

/// The site logo in the header.
pub const LOGO: StaticClass = class!("w-[64px] h-[64px]");

/// The menu toggle — a checkbox, not a button.
///
/// v3's hamburger was a `<div role="button">` toggling classes through Leptos
/// signals, because Leptos shipped a client runtime anyway. v4 has no client
/// runtime and the site is meant to be readable without executing anything, so
/// the toggle is the oldest script-free disclosure there is: a checkbox whose
/// `:checked` state drives the nav through Tailwind's `peer` variant.
///
/// `sr-only` keeps it invisible but focusable; `md:hidden` keeps an invisible
/// focusable control off the desktop layout.
pub const NAV_TOGGLE: StaticClass = class!("peer sr-only md:hidden");

/// The label that draws the hamburger and toggles [`NAV_TOGGLE`].
///
/// `cursor-pointer` because a `<label>` is not a `<button>` and does not say so
/// on its own. The focus ring is on the label rather than the checkbox: the
/// checkbox is `sr-only`, so a ring on it would be invisible.
pub const HAMBURGER_BUTTON: StaticClass = class!(
    "md:hidden flex flex-col items-end space-y-1 cursor-pointer peer-focus-visible:ring-2 peer-focus-visible:ring-neutral-200"
);

/// One bar of the hamburger.
pub const HAMBURGER_BAR: StaticClass = class!("block h-[4px] bg-neutral-200 w-[35px]");

/// The animated bars. Split from [`HAMBURGER_BAR`] so the two that move are
/// named separately from the one that does not.
pub const HAMBURGER_BAR_ANIMATED: StaticClass =
    class!("block h-[4px] bg-neutral-200 w-[35px] transition-all");

/// The nav.
///
/// The two height utilities are the disclosure: `max-md:h-0` is the closed
/// state, and `peer-checked:max-md:h-[120px]` overrides it once the toggle is
/// checked. Both are `max-md:`-scoped, so on `md` and up the nav is simply
/// always open and the toggle is not rendered.
pub const NAV: StaticClass = class!(
    "max-md:basis-full flex space-x-4 max-md:flex-col max-md:items-end \
     max-md:overflow-y-auto transition-all max-md:h-0 peer-checked:max-md:h-[120px]"
);

/// The spacer that pushes the first nav link below the hamburger on mobile.
pub const NAV_SPACER: StaticClass = class!("max-md:mt-6");

/// A nav link.
pub const NAV_LINK: StaticClass =
    class!("max-md:py-2 text-xl lg:text-2xl hover:text-tahiti-400 transition-colors");

/// A nav link on the current page. v3 had no such state — Topcoat can know,
/// because `Href::is_current` answers from the request. Underline, not colour:
/// colour is already spoken for by `hover:`.
pub const NAV_LINK_CURRENT: StaticClass = class!("underline underline-offset-4");

// ---------------------------------------------------------------------------
// Page and footer
// ---------------------------------------------------------------------------

/// `<main>`.
pub const MAIN: StaticClass = class!("flex-auto container mx-auto p-4");

/// The wrapper every page body sits in, so page padding is one decision rather
/// than four. v3 repeated it as `.home, .all-song, .song, .create-song`.
pub const PAGE: StaticClass = class!("max-md:px-0 max-md:py-0 px-8 py-12");

/// `<footer>`.
pub const FOOTER: StaticClass = class!("bg-tahiti-900 p-8 text-center text-xs print:hidden");

/// A link inside the footer.
pub const FOOTER_LINK: StaticClass = class!("hover:underline");

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use super::*;

    /// Every token this module publishes, so the completeness checks below
    /// cannot pass merely by forgetting one.
    const ALL_TOKENS: &[StaticClass] = &[
        SHELL,
        HEADER,
        HEADER_INNER,
        LOGO,
        NAV_TOGGLE,
        HAMBURGER_BUTTON,
        HAMBURGER_BAR,
        HAMBURGER_BAR_ANIMATED,
        NAV,
        NAV_SPACER,
        NAV_LINK,
        NAV_LINK_CURRENT,
        MAIN,
        PAGE,
        FOOTER,
        FOOTER_LINK,
    ];

    /// Renders a token to the class string the layout would put in the markup.
    ///
    /// `StaticClass` is `Class<Unescaped<PromotedStr>>`, so this is two `Deref`
    /// hops down to the `&'static str` that `class!` stored.
    fn rendered(token: &StaticClass) -> &'static str {
        **token.0
    }

    fn classes(token: &StaticClass) -> Vec<&str> {
        rendered(token).split(' ').collect()
    }

    #[test]
    fn tokens_are_never_empty() {
        for token in [SHELL, HEADER, NAV, MAIN, PAGE, FOOTER] {
            assert!(!rendered(&token).trim().is_empty());
        }
    }

    /// Tailwind scans for class names as literal text. If a token is ever built
    /// at runtime, the class silently disappears from the stylesheet and the
    /// element renders unstyled — no error, no test failure, just a page that
    /// looks wrong. Guard the invariant directly.
    #[test]
    fn tokens_contain_no_whitespace_runs_or_line_breaks() {
        for token in [SHELL, HEADER, HEADER_INNER, NAV, PAGE] {
            let value = rendered(&token);
            assert!(
                !value.contains('\n'),
                "line break inside a class list: {value:?}"
            );
            assert!(
                !value.contains("  "),
                "double space inside a class list: {value:?}"
            );
        }
    }

    /// Every `\`-continued literal in this module must join into a single
    /// space-separated list. A missing space silently glues two classes into
    /// one nonsense class name that Tailwind will never generate.
    #[test]
    fn continued_literals_join_with_single_spaces() {
        for token in [SHELL, NAV] {
            for name in classes(&token) {
                assert!(!name.is_empty(), "empty class name in {token:?}");
                assert!(
                    !name.starts_with('"'),
                    "an unbalanced literal leaked a quote: {name:?}"
                );
            }
        }
    }

    /// The disclosure only works if the two halves agree: an element with the
    /// `peer` class, and a nav whose closed height is overridden when that peer
    /// is checked. Lose either half and the menu silently stops opening.
    #[test]
    fn the_disclosure_is_wired_from_both_ends() {
        assert!(classes(&NAV_TOGGLE).contains(&"peer"));
        assert!(classes(&NAV).contains(&"peer-checked:max-md:h-[120px]"));

        // The closed state must be scoped to mobile too, or the nav would be
        // permanently collapsed on desktop.
        assert!(classes(&NAV).contains(&"max-md:h-0"));
        assert!(classes(&NAV).contains(&"peer-checked:max-md:h-[120px]"));
    }

    /// Both height utilities must live behind `max-md:`. Above `md` the nav is
    /// always open, so an unscoped `h-0` would hide the whole menu on desktop.
    #[test]
    fn nav_height_is_mobile_only() {
        for name in classes(&NAV) {
            if name.contains("h-0") || name.contains("h-[120px]") {
                assert!(
                    name.starts_with("max-md:h-") || name.starts_with("peer-checked:max-md:h-"),
                    "unscoped height utility would break the desktop nav: {name:?}"
                );
            }
        }
    }

    /// The dark gradient must stay behind `dark:`, or the site loses its
    /// light-mode look.
    #[test]
    fn body_gradient_has_both_light_and_dark_stops() {
        let shell = rendered(&SHELL);
        assert!(shell.contains("from-tahiti-500"));
        assert!(shell.contains("to-tahiti-800"));
        assert!(shell.contains("dark:from-tahiti-700"));
        assert!(shell.contains("dark:to-tahiti-1000"));
    }

    /// Print is the reason this rewrite has print tokens at all: a songbook gets
    /// printed. Both pieces of chrome must be gone and the gradient must be
    /// cancelled, or a printed lyric sheet is a dark rectangle.
    #[test]
    fn print_drops_chrome_and_background() {
        assert!(rendered(&HEADER).contains("print:hidden"));
        assert!(rendered(&FOOTER).contains("print:hidden"));
        let shell = rendered(&SHELL);
        assert!(shell.contains("print:bg-none"));
        assert!(shell.contains("print:bg-white"));
        assert!(shell.contains("print:text-black"));
    }

    /// Every `tahiti-*` name a token uses must exist in the palette `build.rs`
    /// publishes to Tailwind.
    ///
    /// A colour with no `--color-*` variable behind it is the quietest failure
    /// this module can have: the class name is still valid to write, Tailwind
    /// emits no rule for it, and the element renders unstyled with nothing
    /// logged anywhere. This is the test that makes the palette single-sourced
    /// from `src/ui/palette.rs` mean something.
    #[test]
    fn every_colour_used_by_a_token_exists() {
        let defined: BTreeSet<&str> = crate::ui::palette::PALETTE
            .iter()
            .map(|(name, _)| *name)
            .collect();
        let mut missing: BTreeSet<String> = BTreeSet::new();

        for name in ALL_TOKENS.iter().flat_map(|token| classes(token)) {
            let mut rest = name;
            while let Some(at) = rest.find("tahiti") {
                let tail = &rest[at..];
                let end = tail
                    .find(|c: char| !(c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-'))
                    .unwrap_or(tail.len());
                // Stop before an opacity modifier: `tahiti-900/30`.
                let colour = tail[..end].trim_end_matches('-');
                if !defined.contains(colour) {
                    missing.insert(colour.to_string());
                }
                rest = &tail[end..];
            }
        }

        assert!(
            missing.is_empty(),
            "tokens use colours with no palette entry: {missing:?}"
        );
    }

    /// `build.rs` writes the palette into the `@theme` block, so a duplicate
    /// name would let one definition silently shadow another.
    #[test]
    fn the_palette_defines_each_name_once() {
        let mut seen = BTreeSet::new();
        for (name, _) in crate::ui::palette::PALETTE {
            assert!(seen.insert(*name), "duplicate palette entry: {name}");
        }
        assert!(seen.contains("tahiti-1000"), "v3's darkest stop is missing");
    }
}
