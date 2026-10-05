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
///
/// v3's header wash — `tahiti-900` at 30% — and its drop shadow are the base,
/// now given depth: `backdrop-blur-md` frosts the gradient behind it as the
/// page scrolls under, `border-b border-white/10` is a hairline edge, and
/// `shadow-black/20` is the lift. On a dark page a border and a shadow do
/// different jobs — the border says where the element stops, the shadow says it
/// is above the page — and v3 had only the second, which is why its header read
/// flat.
pub const HEADER: StaticClass = class!(
    "bg-tahiti-900/40 backdrop-blur-md p-4 border-b border-white/10 \
     shadow-lg shadow-black/20 print:hidden"
);

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

/// `<footer>`. The header's treatment, mirrored: `border-t` instead of
/// `border-b`, and `tahiti-1000` rather than `tahiti-900` so the page closes
/// darker than it opens.
pub const FOOTER: StaticClass = class!(
    "bg-tahiti-1000/60 backdrop-blur-md border-t border-white/10 p-8 text-center \
     text-xs print:hidden"
);

/// A link inside the footer.
pub const FOOTER_LINK: StaticClass = class!("hover:underline");

// ---------------------------------------------------------------------------
// Typography
// ---------------------------------------------------------------------------

/// A page title — the one place the display face is right.
///
/// A heading has room for a face with character; running body text does not,
/// which is why the two are separate families rather than one. `text-balance`
/// stops a two- or three-word title from breaking with a single orphan word,
/// and `tracking-tight` is what keeps a serif at display size from looking
/// loose — the opposite of what body copy wants.
pub const H1: StaticClass = class!(
    "font-display text-4xl md:text-5xl lg:text-6xl font-bold tracking-tight \
     text-balance mb-6"
);

/// The standfirst under a page title.
///
/// `text-pretty` rather than `text-balance`: it only avoids a last-line orphan,
/// which is the right trade for a paragraph. `max-w-prose` is the measure — a
/// line of 65-odd characters, which is the whole reason this is a token instead
/// of a one-off.
pub const LEAD: StaticClass =
    class!("text-lg md:text-xl text-neutral-300 text-pretty max-w-prose mx-auto mb-10");

// ---------------------------------------------------------------------------
// Surfaces
// ---------------------------------------------------------------------------

/// A raised panel — the unit of depth in this design.
///
/// Three utilities, each load-bearing. `border border-white/10` is a hairline
/// that survives whatever the gradient puts behind it. `bg-tahiti-1000/40` is a
/// wash darker than the page, which is what actually separates a panel from its
/// background in a dark theme: a shadow alone reads as a smudge, because there
/// is no light to be occluded. `shadow-card` adds the contact edge on top.
///
/// No `backdrop-blur`: a page can hold a dozen cards and a blur behind each one
/// costs the compositor far more than the frosted chrome, which there is exactly
/// two of.
///
/// **No padding.** The home page's cards and table panels want `p-8`, the song
/// page's panel will want `p-6`, and two padding utilities in one class list are
/// resolved by stylesheet order rather than by intent. So the surface is one
/// token and the padding is chosen where the panel is placed — see
/// [`CARD_ROOMY`], which is the one padding that is needed twice already.
pub const CARD: StaticClass =
    class!("rounded-card border border-white/10 bg-tahiti-1000/40 shadow-card");

/// A [`CARD`] with the roomier padding the home page's panels want — the three
/// cards and the two table panels are one surface at one size, so they get one
/// name.
///
/// **`CARD`'s classes are repeated here, and that is not an oversight.** A
/// [`StaticClass`] is a single string literal — `class!` can compose entries in
/// an attribute position, where the type is inferred, but not into a `const`,
/// which is pinned to `Class<Unescaped<PromotedStr>>`. Composition is therefore
/// impossible for a token and the `.0`-style alternative (build the string in a
/// `LazyLock`) would take the class out of Tailwind's reach, since Tailwind
/// scans for *literal* text. So the surface is written twice and
/// `the_panel_variants_keep_the_card_surface` is what makes the copy safe:
/// lose a utility from one and the test names it.
pub const CARD_ROOMY: StaticClass =
    class!("rounded-card border border-white/10 bg-tahiti-1000/40 shadow-card p-8");

// ---------------------------------------------------------------------------
// Home page
// ---------------------------------------------------------------------------

/// The hero: the page title, its standfirst, and the call to action.
pub const HERO: StaticClass = class!("text-center mb-20");

/// The hero's headline.
///
/// Larger than [`H1`] and tinted rather than plain, because on the home page the
/// title *is* the page — there is no content above it to compete with.
pub const HERO_TITLE: StaticClass = class!("text-5xl font-bold dark:text-tahiti mb-6");

/// The hero's standfirst. Italic: it is a subtitle, not a lede. [`LEAD`] is the
/// token for the latter.
pub const HERO_SUBTITLE: StaticClass = class!("text-2xl dark:text-gray-400 mb-10 italic");

/// A filled, pill-shaped call to action — the hero's "Découvrir les chansons".
pub const BUTTON_PRIMARY: StaticClass = class!(
    "bg-tahiti-700 hover:bg-tahiti-800 dark:bg-tahiti-600 dark:hover:bg-tahiti-500 \
     font-bold py-3 px-8 rounded-full text-lg transition-colors duration-300 shadow-lg"
);

/// The long synopsis between the hero and the tables.
///
/// `max-w-[800px]` rather than `max-w-prose`: v3's measure, kept, because this
/// is a display paragraph and not body copy — see `PLAN.md` §16.
pub const SYNOPSIS: StaticClass = class!("mx-auto max-w-[800px] leading-7 text-xl");

/// The three-card row: one column on mobile, three from `md` up.
pub const CARD_GRID: StaticClass = class!("grid md:grid-cols-3 gap-10 mb-20");

/// A heading inside a panel — a card title, a table title.
pub const PANEL_TITLE: StaticClass = class!("text-2xl font-semibold mb-4 dark:text-tahiti-400");

/// The panel a home-page table sits in.
///
/// `md:max-w-max` is what stops the panel stretching to the full width of the
/// page: five rows should be as wide as the rows, not as wide as the window.
pub const TABLE_PANEL: StaticClass = class!(
    "rounded-card border border-white/10 bg-tahiti-1000/40 shadow-card p-8 \
     max-md:text-center md:max-w-max"
);

/// A home-page section holding one table.
pub const TABLE_SECTION: StaticClass = class!("mb-20");

/// A table row.
///
/// v3 made the whole row a click target (`role="button"` + `onclick`) and tinted
/// it on hover. v4 keeps the tint — it still reads as one row responding — but
/// the row is no longer clickable: only the two links inside it are, which is
/// what makes the row reachable by a crawler and by a keyboard.
pub const TABLE_ROW: StaticClass = class!("hover:text-tahiti-400 transition-colors");

/// A plain cell in a home-page table.
pub const TABLE_CELL: StaticClass = class!("p-4");

/// The cell holding the lyric line.
///
/// `max-md:hidden` is v3's own rule, spelled there as a `row-lyrics` class: on a
/// phone the lyric column is noise beside the title, so it is dropped and the
/// row becomes one link.
pub const TABLE_LYRICS_CELL: StaticClass = class!("p-4 max-md:hidden");

/// The lyric line itself.
///
/// The **whole** chord-free lyric is in the markup — v3 did not truncate on the
/// server either — and `truncate` is what makes it one line. The width is
/// load-bearing: with no width, `truncate` has nothing to cut against and the
/// cell would stretch the table instead. v3's per-breakpoint values, kept.
pub const TABLE_LYRICS: StaticClass =
    class!("inline-block truncate pt-[9px] max-md:w-[110px] md:w-[500px] lg:w-[800px]");

/// The closing call to action at the foot of the home page.
pub const FOOT_SECTION: StaticClass = class!("text-center");

/// Its headline.
pub const FOOT_TITLE: StaticClass = class!("text-4xl font-bold mb-8");

/// The line beneath it.
pub const FOOT_TEXT: StaticClass = class!("text-xl dark:text-gray-400 mb-10");

/// The row of closing buttons.
pub const FOOT_LINKS: StaticClass =
    class!("flex max-md:flex-col gap-4 items-center justify-center");

/// A pale, filled button — the lighter of the two closing buttons.
pub const BUTTON_LIGHT: StaticClass = class!(
    "bg-neutral-200 text-tahiti-600 dark:text-tahiti-800 hover:bg-tahiti-600 \
     dark:hover:bg-tahiti-800 hover:text-neutral-200 font-bold py-3 px-8 rounded-full \
     text-lg transition duration-300 shadow-lg"
);

/// An outlined button.
pub const BUTTON_OUTLINE: StaticClass = class!(
    "border-2 border-neutral-200 hover:border-tahiti-600 dark:hover:border-tahiti-800 \
     hover:text-tahiti-600 dark:hover:text-tahiti-800 hover:bg-neutral-200 font-bold \
     py-3 px-8 rounded-full text-lg transition duration-300"
);

// ---------------------------------------------------------------------------
// Errors
// ---------------------------------------------------------------------------

/// The branded not-found page's wrapper.
///
/// v3's fallback route rendered the bare string "La page n'existe pas." with no
/// chrome; Topcoat's own default 404 is bare markup. This is the middle ground:
/// the same sentence, inside the site's shell.
pub const NOT_FOUND: StaticClass = class!("text-center py-16");

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
        H1,
        LEAD,
        CARD,
        CARD_ROOMY,
        HERO,
        HERO_TITLE,
        HERO_SUBTITLE,
        BUTTON_PRIMARY,
        SYNOPSIS,
        CARD_GRID,
        PANEL_TITLE,
        TABLE_PANEL,
        TABLE_SECTION,
        TABLE_ROW,
        TABLE_CELL,
        TABLE_LYRICS_CELL,
        TABLE_LYRICS,
        FOOT_SECTION,
        FOOT_TITLE,
        FOOT_TEXT,
        FOOT_LINKS,
        BUTTON_LIGHT,
        BUTTON_OUTLINE,
        NOT_FOUND,
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
        for token in [SHELL, HEADER, NAV, MAIN, PAGE, FOOTER, H1, LEAD, CARD] {
            assert!(!rendered(&token).trim().is_empty());
        }
    }

    /// Tailwind scans for class names as literal text. If a token is ever built
    /// at runtime, the class silently disappears from the stylesheet and the
    /// element renders unstyled — no error, no test failure, just a page that
    /// looks wrong. Guard the invariant directly.
    #[test]
    fn tokens_contain_no_whitespace_runs_or_line_breaks() {
        for token in [SHELL, HEADER, HEADER_INNER, NAV, PAGE, H1, LEAD, CARD] {
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
        for token in [SHELL, NAV, H1, LEAD, CARD] {
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

    /// Typography has to land somewhere, and the headings are where it lands.
    /// `font-display` is the utility Tailwind generates from `--font-display`,
    /// which `build.rs` writes from `palette::FONT_DISPLAY`, which
    /// `src/ui/fonts.rs` checks against the family it actually declares. Break
    /// any link in that chain and the heading renders in the body face with no
    /// error anywhere.
    #[test]
    fn headings_use_the_display_face() {
        assert!(classes(&H1).contains(&"font-display"));
    }

    /// The body face is the *default*, applied by Tailwind's preflight from
    /// `--font-sans`. Asking for it by name would be a second, weaker copy of
    /// that decision — and would survive a change to the preflight default while
    /// quietly not following it.
    #[test]
    fn no_token_names_the_body_face_explicitly() {
        for token in ALL_TOKENS {
            assert!(
                !classes(token).contains(&"font-sans"),
                "a token names the default face: {token:?}"
            );
        }
    }

    /// [`CARD_ROOMY`] and [`TABLE_PANEL`] repeat [`CARD`]'s surface classes
    /// because a `StaticClass` cannot be composed from constants — see the
    /// comment on [`CARD_ROOMY`]. This is the check that makes the repetition
    /// safe: drop a utility from one of them and this fails, rather than the
    /// panel quietly losing its border on the live site.
    #[test]
    fn the_panel_variants_keep_the_card_surface() {
        let surface: BTreeSet<&str> = classes(&CARD).into_iter().collect();
        assert!(surface.contains(&"rounded-card"));
        assert!(surface.contains(&"shadow-card"));

        for variant in [CARD_ROOMY, TABLE_PANEL] {
            let names: BTreeSet<&str> = classes(&variant).into_iter().collect();
            let missing: Vec<&&str> = surface.difference(&names).collect();
            assert!(missing.is_empty(), "a panel variant lost {missing:?}");
            // ... and every variant is the *roomy* one; none of them is a bare
            // CARD, which would render with its content touching the border.
            assert!(names.contains(&"p-8"), "a panel variant has no padding");
        }

        let table = classes(&TABLE_PANEL);
        assert!(table.contains(&"max-md:text-center"));
        assert!(table.contains(&"md:max-w-max"));
    }

    /// The same invariant as `every_colour_used_by_a_token_exists`, for depth.
    /// `rounded-card` with no `--radius-card` behind it emits no rule at all and
    /// the panel renders square — no error, no warning, just a page that looks
    /// wrong.
    #[test]
    fn the_card_uses_the_published_radius_and_shadow() {
        let card = classes(&CARD);
        assert!(card.contains(&"rounded-card"));
        assert!(card.contains(&"shadow-card"));

        let radii: BTreeSet<&str> = crate::ui::palette::RADII.iter().map(|(n, _)| *n).collect();
        let shadows: BTreeSet<&str> = crate::ui::palette::SHADOWS
            .iter()
            .map(|(n, _)| *n)
            .collect();
        assert!(radii.contains("card"), "no --radius-card in the palette");
        assert!(shadows.contains("card"), "no --shadow-card in the palette");
    }

    /// One name per namespace, for the reason `the_palette_defines_each_name_once`
    /// gives about colours: a duplicate lets the first definition silently lose.
    #[test]
    fn the_depth_namespaces_define_each_name_once() {
        for tokens in [crate::ui::palette::RADII, crate::ui::palette::SHADOWS] {
            let mut seen = BTreeSet::new();
            for (name, value) in tokens {
                assert!(seen.insert(*name), "duplicate depth token: {name}");
                assert!(!value.trim().is_empty(), "empty value for {name}");
                assert!(
                    !name.contains(' '),
                    "a depth token name with a space is not a usable class: {name:?}"
                );
            }
        }
    }
}
