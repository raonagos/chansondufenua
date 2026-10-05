//! Design tokens — the whole of v4's styling vocabulary.
//!
//! Every entry is a Tailwind utility list stored as a [`StaticClass`], so the
//! markup in `src/ui/` and `src/pages/` reads as composition rather than as
//! markup full of class soup. v3 expressed a similar vocabulary as `@apply`
//! blocks in `style/tailwind.scss`; this module keeps the *mechanism* and
//! repaints the design on purpose — see `PLAN.md` §15 for what changed and why.
//!
//! Three rules this module exists to enforce:
//!
//! 1. **No styling lives outside it.** Pages compose tokens; they do not invent
//!    colours or spacing. If a page needs a new look, the token belongs here.
//! 2. **Class names stay literal.** Tailwind scans source text for class names;
//!    it cannot see one assembled at runtime. Every name below is inside a
//!    `class!("…")` literal, which is why no const is built with `format!` or
//!    `concat!`.
//! 3. **The classes that must not collide do not collide.** A composed token
//!    list is resolved by stylesheet order, not by intent, so two utilities
//!    setting the same property is a coin toss. Tokens that are meant to be
//!    composed with each other are tested for that — see
//!    `tokens_meant_to_be_composed_never_contradict_each_other`.
//!
//! Colours (`tahiti-*`, `ink-*`, `mist-*`) are published from
//! [`crate::ui::palette`], which `build.rs` turns into Tailwind's `@theme` block
//! at build time. See that file for the ramps and the measured contrast of each
//! text step.
//!
//! **A class name written in a comment is emitted into the stylesheet too** —
//! Tailwind v4 scans prose. So the comments below *describe* utilities instead
//! of spelling them, except where the name is one a token in this file actually
//! uses.

use topcoat::view::{StaticClass, class};

// ---------------------------------------------------------------------------
// Document shell
// ---------------------------------------------------------------------------

/// `<body>`.
///
/// **Flat, and dark, unconditionally.** v3 painted a full-bleed diagonal
/// gradient from mid cyan to deep teal and floated translucent panels over it.
/// Two things were wrong with that beyond taste: the gradient's midpoint sat
/// behind the body text and gave it roughly **1.9:1** contrast, and every
/// translucent surface took its value from whatever part of the gradient
/// happened to pass behind it — so the same card read differently at the top of
/// a page than at the foot, and no text ratio was knowable in advance.
///
/// Here the page is one opaque value and every ratio in the design is a
/// measured fact. There is no light variant: v4 is dark-only, so no token in
/// this file carries a colour-scheme variant, and the document declares its
/// scheme in `src/ui/layout.rs`.
///
/// The print entries are the point of a songbook: printing v3 put a dark
/// gradient behind the lyrics and burned the palette on paper.
pub const SHELL: StaticClass = class!(
    "min-h-screen flex flex-col bg-ink-950 text-mist-100 antialiased \
     print:bg-white print:text-black"
);

// ---------------------------------------------------------------------------
// Header and navigation
// ---------------------------------------------------------------------------

/// `<header>`. Hidden in print: a printed song needs the lyrics, not the chrome.
///
/// Sticky, with a translucent base and a frost: the page scrolls under it, and
/// the header stays legible without becoming an opaque band. The base is the
/// page colour at 85%, which is enough to read text over without hiding what is
/// behind it. The hairline says where the element stops; the shadow would say it
/// is above the page, and on a dark, flat page the hairline does that job alone
/// — so there is no shadow here.
pub const HEADER: StaticClass = class!(
    "sticky top-0 z-40 bg-ink-950/85 backdrop-blur-md border-b border-ink-700 \
     print:hidden"
);

/// The flex row inside `<header>`: logo hard left, everything else right.
pub const HEADER_INNER: StaticClass =
    class!("container mx-auto flex items-center justify-end flex-wrap px-4 py-3");

/// The site logo in the header. A square, so it is sized with the one utility
/// that sets both axes and cannot drift into a rectangle.
pub const LOGO: StaticClass = class!("size-12");

/// The menu toggle — a checkbox, not a button.
///
/// v3's hamburger was a `<div role="button">` toggling classes through Leptos
/// signals, because Leptos shipped a client runtime anyway. v4 has no client
/// runtime and the site is meant to be readable without executing anything, so
/// the toggle is the oldest script-free disclosure there is: a checkbox whose
/// checked state drives the nav through Tailwind's peer variant.
///
/// The visually-hidden utility keeps it invisible but focusable; the one that
/// hides it from `md` up keeps an invisible focusable control off the desktop
/// layout.
pub const NAV_TOGGLE: StaticClass = class!("peer sr-only md:hidden");

/// The label that draws the hamburger and toggles [`NAV_TOGGLE`].
///
/// A pointer cursor, because a `<label>` is not a `<button>` and does not say so
/// on its own. The focus ring is on the label rather than the checkbox: the
/// checkbox is visually hidden, so a ring on it would be invisible.
pub const HAMBURGER_BUTTON: StaticClass = class!(
    "md:hidden flex flex-col items-end space-y-1 cursor-pointer \
     peer-focus-visible:ring-2 peer-focus-visible:ring-tahiti-400"
);

/// One bar of the hamburger.
pub const HAMBURGER_BAR: StaticClass = class!("block h-[3px] w-8 rounded-full bg-mist-100");

/// The animated bars. Split from [`HAMBURGER_BAR`] so the two that move are
/// named separately from the one that does not.
pub const HAMBURGER_BAR_ANIMATED: StaticClass =
    class!("block h-[3px] w-8 rounded-full bg-mist-100 transition-all");

/// The nav.
///
/// **The link colour lives here, not on [`NAV_LINK`].** A nav link is always
/// composed with one of two colour states, and a token that set its own colour
/// would collide with the current-page one: two colour utilities in one list are
/// resolved by stylesheet order, not by which came second. Colouring the
/// container instead means the state token is the only thing that ever speaks
/// about colour, and `nav_link_states_never_contradict_the_base` holds the line.
///
/// The two height utilities are the disclosure: the closed state is zero height,
/// and the checked-peer one overrides it. Both are scoped below `md`, so on
/// larger screens the nav is simply always open and the toggle is not rendered.
pub const NAV: StaticClass = class!(
    "text-mist-300 max-md:basis-full flex space-x-6 max-md:flex-col max-md:items-end \
     max-md:overflow-y-auto transition-all max-md:h-0 peer-checked:max-md:h-[120px]"
);

/// The spacer that pushes the first nav link below the hamburger on mobile.
pub const NAV_SPACER: StaticClass = class!("max-md:mt-6");

/// A nav link. Layout and motion only — the colour comes from [`NAV`].
pub const NAV_LINK: StaticClass =
    class!("max-md:py-2 text-lg lg:text-xl transition-colors hover:text-mist-100");

/// A nav link on the current page. v3 had no such state — Topcoat can know,
/// because a link can ask the request whether it is current.
///
/// Underlined in the accent, not merely recoloured: colour is already spoken for
/// by the link's own hover, and a page that only changes shade is a page a
/// colour-blind reader cannot find.
pub const NAV_LINK_CURRENT: StaticClass =
    class!("text-mist-100 underline decoration-tahiti-400 decoration-2 underline-offset-8");

// ---------------------------------------------------------------------------
// Page and footer
// ---------------------------------------------------------------------------

/// `<main>`.
pub const MAIN: StaticClass = class!("flex-auto container mx-auto p-4");

/// The wrapper every page body sits in, so page padding is one decision rather
/// than four. v3 repeated it as `.home, .all-song, .song, .create-song`.
pub const PAGE: StaticClass = class!("max-md:px-0 max-md:py-0 px-8 py-12");

/// `<footer>`.
///
/// Closed by a hairline and nothing else. v3's footer was a filled band a step
/// darker than the header; on a flat page the band and the page are the same
/// colour, so a rule is the honest way to say the document has ended — and it
/// saves an element-sized paint for nothing.
pub const FOOTER: StaticClass =
    class!("border-t border-ink-700 p-8 text-center text-sm text-mist-500 print:hidden");

// ---------------------------------------------------------------------------
// Typography
// ---------------------------------------------------------------------------

/// A page title — the one place the display face is right.
///
/// A heading has room for a face with character; running body text does not,
/// which is why the two are separate families rather than one. Weight is
/// whatever the family actually ships: asking the display face for a semibold
/// would be answered by the browser's synthesiser rather than the type
/// designer — see `crate::ui::fonts` and the weight test in this module.
///
/// Text is balanced so a two- or three-word title does not break with a single
/// orphan word, and tightened, because a serif at display size looks loose at
/// the default tracking — the opposite of what body copy wants.
pub const H1: StaticClass = class!(
    "font-display text-4xl md:text-5xl font-bold tracking-tight text-balance \
     text-mist-100 mb-6"
);

/// The standfirst under a page title.
///
/// Pretty rather than balanced: it only avoids a last-line orphan, which is the
/// right trade for a paragraph. The measure is a line of 65-odd characters,
/// which is the whole reason this is a token instead of a one-off.
pub const LEAD: StaticClass =
    class!("text-lg md:text-xl text-mist-300 text-pretty max-w-prose mx-auto mb-10");

/// A link in running content — a table cell, a footer, a caption.
///
/// Underlined always, not only on hover: a link that is only discoverable by
/// pointing at it is invisible to a keyboard, a crawler and a touch screen. The
/// rule is quiet against the surrounding text and takes the accent only when
/// the pointer is on it, so a page of forty song titles reads as forty titles
/// rather than forty cyan words.
pub const LINK: StaticClass = class!(
    "text-mist-100 underline decoration-ink-600 underline-offset-4 transition-colors \
     hover:text-tahiti-300 hover:decoration-tahiti-400 \
     focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-tahiti-400"
);

/// The keyboard focus ring, for the interactive elements that are not [`LINK`].
///
/// Named once so that every control in the site can be shown to be reachable
/// without tabbing through it by hand. An outline rather than a ring: an outline
/// follows the element's own radius, and it is not painted over by a later
/// background.
pub const FOCUS: StaticClass = class!(
    "focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-tahiti-400"
);

// ---------------------------------------------------------------------------
// Surfaces
// ---------------------------------------------------------------------------

/// A raised panel — the unit of depth in this design.
///
/// Two utilities, each load-bearing. The hairline survives whatever is behind
/// it, and the fill is a step *lighter* than the page rather than a wash over
/// it: on a flat dark page, a panel has to be a different value, and a
/// translucent one only appears to be. The shadow is a contact edge on top of
/// that.
///
/// No frost of its own: a page can hold a dozen cards, and a filtered backdrop
/// costs the compositor far more than the two pieces of chrome that have one.
///
/// **No padding.** The home page's cards want the roomy step, a song sheet wants
/// its own, and two padding utilities in one class list are resolved by
/// stylesheet order rather than by intent. So the surface is one token and the
/// padding is chosen where the panel is placed — see [`CARD_ROOMY`], which is
/// the one padding needed twice already.
pub const CARD: StaticClass = class!("rounded-card border border-ink-700 bg-ink-900 shadow-card");

/// A [`CARD`] with the roomier padding the home page's panels want — the three
/// cards and the two table panels are one surface at one size, so they get one
/// name.
///
/// **`CARD`'s classes are repeated here, and that is not an oversight.** A
/// [`StaticClass`] is a single string literal — `class!` can compose entries in
/// an attribute position, where the type is inferred, but not into a `const`,
/// which is pinned to `Class<Unescaped<PromotedStr>>`. Composition is therefore
/// impossible for a token and the runtime-built alternative would take the class
/// out of Tailwind's reach, since Tailwind scans for *literal* text. So the
/// surface is written twice and `the_panel_variants_keep_the_card_surface` is
/// what makes the copy safe: lose a utility from one and the test names it.
pub const CARD_ROOMY: StaticClass =
    class!("rounded-card border border-ink-700 bg-ink-900 shadow-card p-8");

// ---------------------------------------------------------------------------
// Home page
// ---------------------------------------------------------------------------

/// The hero: the page title, its standfirst, and the call to action.
pub const HERO: StaticClass = class!("text-center mb-20");

/// The hero's headline.
///
/// A step larger than [`H1`], because on the home page the title *is* the page —
/// there is no content above it to compete with. Display face, like every other
/// heading: v3 asked for it in the body face, which is the one place its
/// typography direction was not applied.
pub const HERO_TITLE: StaticClass = class!(
    "font-display text-5xl md:text-6xl lg:text-7xl font-bold tracking-tight \
     text-balance text-mist-100 mb-6"
);

/// The hero's standfirst.
///
/// Not slanted, though v3 set it so. That slant would be synthesised: no face of
/// any of the three families ships an oblique, so the browser would shear the
/// upright and the result reads as a mistake. Smaller and muted instead, which
/// is a contrast in weight and colour rather than in slant.
pub const HERO_SUBTITLE: StaticClass = class!("text-xl md:text-2xl text-mist-300 mb-10");

/// A filled, pill-shaped call to action — the hero's "Découvrir les chansons".
///
/// **Dark text on the accent, not white on it.** The accent is a light cyan; a
/// near-black label on it measures 7.7:1, while white would measure 2.4:1. This
/// is also the one place on the page where the accent fills a shape, so the
/// colour stays a signal rather than a decoration.
pub const BUTTON_PRIMARY: StaticClass = class!(
    "inline-block rounded-full bg-tahiti-500 px-8 py-3 text-lg font-semibold \
     text-ink-950 shadow-lg transition-colors hover:bg-tahiti-300"
);

/// The long synopsis between the hero and the tables.
///
/// A wider measure than body copy's, because this is a display paragraph and not
/// prose to be read line by line — v3's own width, kept, see `PLAN.md` §16.
/// Muted, with generous leading, so a hundred words do not read as a wall.
pub const SYNOPSIS: StaticClass = class!("mx-auto max-w-[800px] text-lg leading-8 text-mist-300");

/// The three-card row: one column on mobile, three from `md` up.
pub const CARD_GRID: StaticClass = class!("grid md:grid-cols-3 gap-10 mb-20");

/// A heading inside a panel — a card title, a table title.
///
/// Display face and full-strength text. v3 tinted every one of them with the
/// accent, which spent the whole page's colour budget on four headings and left
/// nothing to mark what was actually clickable.
pub const PANEL_TITLE: StaticClass = class!("font-display text-2xl font-bold text-mist-100 mb-4");

/// The panel a home-page table sits in.
///
/// The max-width utility is what stops the panel stretching to the full width of
/// the page: five rows should be as wide as the rows, not as wide as the window.
pub const TABLE_PANEL: StaticClass = class!(
    "rounded-card border border-ink-700 bg-ink-900 shadow-card p-8 \
     max-md:text-center md:max-w-max"
);

/// A home-page section holding one table.
pub const TABLE_SECTION: StaticClass = class!("mb-20");

/// A table row.
///
/// v3 made the whole row a click target with a script and a role, and tinted it
/// on hover. v4 keeps the tint — it still reads as one row responding — but the
/// row is no longer clickable: only the links inside it are, which is what makes
/// the row reachable by a crawler and by a keyboard.
pub const TABLE_ROW: StaticClass = class!("transition-colors hover:bg-ink-800");

/// A plain cell in a home-page table.
pub const TABLE_CELL: StaticClass = class!("p-4");

/// The cell holding the lyric line.
///
/// v3's own rule, spelled there as a `row-lyrics` class: on a phone the lyric
/// column is noise beside the title, so it is dropped and the row becomes one
/// link.
pub const TABLE_LYRICS_CELL: StaticClass = class!("p-4 max-md:hidden");

/// The lyric line itself.
///
/// The **whole** chord-free lyric is in the markup — v3 did not truncate on the
/// server either — and the truncation utility is what makes it one line. The
/// width is load-bearing: with no width, truncation has nothing to cut against
/// and the cell would stretch the table instead. v3's per-breakpoint values,
/// kept.
pub const TABLE_LYRICS: StaticClass =
    class!("inline-block truncate pt-[9px] max-md:w-[110px] md:w-[500px] lg:w-[800px]");

/// The closing call to action at the foot of the home page.
pub const FOOT_SECTION: StaticClass = class!("text-center");

/// Its headline.
pub const FOOT_TITLE: StaticClass =
    class!("font-display text-3xl md:text-4xl font-bold tracking-tight text-balance mb-8");

/// The line beneath it.
pub const FOOT_TEXT: StaticClass = class!("text-lg md:text-xl text-mist-300 mb-10");

/// The row of closing buttons.
pub const FOOT_LINKS: StaticClass =
    class!("flex max-md:flex-col gap-4 items-center justify-center");

/// A pale, filled button — the lighter of the two closing buttons.
///
/// A solid light fill rather than v3's hover-swapped pair of colour utilities:
/// this is the *secondary* action, and the accent belongs to the primary one.
pub const BUTTON_LIGHT: StaticClass = class!(
    "inline-block rounded-full bg-mist-100 px-8 py-3 text-lg font-semibold text-ink-950 \
     shadow-lg transition-colors hover:bg-tahiti-300"
);

/// An outlined button.
pub const BUTTON_OUTLINE: StaticClass = class!(
    "inline-block rounded-full border-2 border-ink-600 px-8 py-3 text-lg font-semibold \
     text-mist-100 transition-colors hover:border-tahiti-400 hover:text-tahiti-300"
);

// ---------------------------------------------------------------------------
// Song index
// ---------------------------------------------------------------------------

/// The panel the `/himene` table sits in.
///
/// [`CARD`]'s surface, written out again for the reason [`CARD_ROOMY`] gives —
/// a [`StaticClass`] cannot be composed from constants — plus the overflow rule,
/// which is the only thing clipping the table's square corners to the panel's
/// radius. **No padding:** on this page the cells carry it, so padding here
/// would inset the table's own edge and leave the heading row floating inside a
/// margin.
pub const INDEX_PANEL: StaticClass =
    class!("rounded-card border border-ink-700 bg-ink-900 shadow-card overflow-hidden");

/// A heading cell of the index table.
///
/// Muted and small, so the column labels sit below the song titles they
/// describe. The table is a list of songs, not a data grid; the headings are
/// there so a screen reader can name each column.
pub const INDEX_HEAD: StaticClass =
    class!("px-6 py-3 text-left text-sm font-semibold text-mist-500");

/// The artist column, dropped below `md`.
///
/// v3 spelled this rule once per element — on the heading and on every cell. It
/// is a property of the *column*, so it is one token here, composed with
/// [`INDEX_HEAD`] and [`INDEX_CELL`] at the two places the column is declared.
pub const INDEX_COLUMN_ARTIST: StaticClass = class!("hidden md:table-cell");

/// A row of the index table.
///
/// The row's rule is dropped on the last one. v3's panel had no border of its
/// own, so a rule under every row had nothing to collide with; the v4 panel is
/// bordered, and on the last row the row's rule and the panel's edge land a
/// pixel apart and read as a double line.
pub const INDEX_ROW: StaticClass =
    class!("border-b border-ink-700 transition-colors last:border-b-0 hover:bg-ink-800");

/// A cell of the index table.
pub const INDEX_CELL: StaticClass = class!("px-6 py-4 whitespace-nowrap");

/// The one row shown when nothing is published — v3's "Pas de chanson".
pub const INDEX_EMPTY: StaticClass = class!("py-4 text-center text-mist-500");

// ---------------------------------------------------------------------------
// Song page
// ---------------------------------------------------------------------------

/// The song sheet: one panel holding the title and the whole lyric.
///
/// Roomier than [`CARD_ROOMY`] from `md` up, because this is the one page a
/// visitor came to read rather than scan, and a wall of lyric against a panel
/// edge is tiring.
pub const SONG_SHEET: StaticClass =
    class!("rounded-card border border-ink-700 bg-ink-900 shadow-card p-6 md:p-10");

/// The sheet's header: the song's name on the left, the actions on the right.
///
/// Wraps rather than reverses on a phone. v3 flipped this row into a column with
/// the *button* first, so a narrow screen led with "Ajouter des paroles" above
/// the name of the song it would add them to.
pub const SONG_HEAD: StaticClass = class!("mb-10 flex flex-wrap items-center gap-x-6 gap-y-4");

/// The name-and-credits block inside [`SONG_HEAD`].
///
/// Full width on a phone, so the action drops to its own line rather than
/// squeezing in beside a long title; it takes the free space from `md` up.
pub const SONG_HEADING: StaticClass = class!("grow basis-full md:basis-0");

/// The song's title.
///
/// A song title is a heading, so it takes the display face like every other
/// heading on the site — v3 set it in the body face, at a fixed size, in a
/// slant that no shipped face can draw.
pub const SONG_TITLE: StaticClass =
    class!("font-display text-3xl md:text-4xl font-bold tracking-tight text-balance text-mist-100");

/// Who wrote or sings the song, under its title.
///
/// **Not in v3**, which carried the credits only inside the page's metadata.
/// The index shows them in a column, so a reader arriving from the index already
/// knows them, and a reader arriving from a search result never saw them at all.
/// They are the song's own data, rendered where they belong.
pub const SONG_ARTISTS: StaticClass = class!("text-lg text-mist-300");

/// The small filled button in a sheet header — "Ajouter des paroles".
///
/// [`BUTTON_PRIMARY`] one size down. Written out rather than shared, for the
/// reason [`CARD_ROOMY`] documents: a [`StaticClass`] cannot be composed from
/// constants.
pub const BUTTON_SMALL: StaticClass = class!(
    "inline-block rounded-full bg-tahiti-500 px-5 py-2 text-sm font-semibold \
     text-ink-950 transition-colors hover:bg-tahiti-300"
);

/// The lyric block.
///
/// Leading is roughly double, which is not decoration: every chord is drawn
/// *above* the line it belongs to, so the half of each line box above the text
/// is where they live. [`CHORD`] takes that space out of the flow, so the lyric
/// itself reads continuously — "Hina'a" and "ro" stay adjacent even though a
/// chord sits between them in the source.
pub const LYRICS: StaticClass = class!("text-lg leading-loose text-mist-100");

/// One line of lyric. Positioned, because the chords are placed against it.
pub const LYRIC_LINE: StaticClass = class!("relative");

/// The blank line between two verses.
///
/// A fixed step rather than an empty line of the lyric's own height: a verse
/// break is a visual pause, and a full line box would read as a missing line.
pub const LYRIC_GAP: StaticClass = class!("h-6");

/// A chord, drawn over the syllable it is written against.
///
/// Absolutely positioned so it adds no width — the lyric underneath reads as one
/// unbroken line, which is what makes a chord sheet usable for singing. A
/// negative margin pulls it back over the preceding character, because the chord
/// marks where the harmony changes *on* a syllable rather than after it. The
/// leading is reset to one, or the chord would inherit the lyric's doubled
/// leading and drift down into the words.
///
/// Monospace, because chord labels are short, stacked and must not collide with
/// the words they sit over; the accent colour is what separates them from the
/// lyric without a box around each one.
pub const CHORD: StaticClass =
    class!("absolute top-1 -ml-2 font-mono text-[0.8rem] leading-none text-tahiti-300");

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
    use std::collections::{BTreeMap, BTreeSet};

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
        H1,
        LEAD,
        LINK,
        FOCUS,
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
        INDEX_PANEL,
        INDEX_HEAD,
        INDEX_COLUMN_ARTIST,
        INDEX_ROW,
        INDEX_CELL,
        INDEX_EMPTY,
        SONG_SHEET,
        SONG_HEAD,
        SONG_HEADING,
        SONG_TITLE,
        SONG_ARTISTS,
        BUTTON_SMALL,
        LYRICS,
        LYRIC_LINE,
        LYRIC_GAP,
        CHORD,
    ];

    /// Renders a token to the class string the layout would put in the markup.
    ///
    /// `StaticClass` is `Class<Unescaped<PromotedStr>>`, so this is two `Deref`
    /// hops down to the `&'static str` that `class!` stored.
    fn rendered(token: &StaticClass) -> &'static str {
        **token.0
    }

    fn classes(token: &StaticClass) -> Vec<&'static str> {
        rendered(token).split(' ').collect()
    }

    /// The colour families the palette or Tailwind publishes. Only these make a
    /// utility a *colour* utility; `text-lg` and `text-balance` share the prefix
    /// and are not colours at all.
    const COLOUR_FAMILIES: &[&str] = &[
        "ink",
        "mist",
        "tahiti",
        "neutral",
        "gray",
        "white",
        "black",
        "current",
        "transparent",
    ];

    /// The utility prefixes that can carry a colour.
    const COLOUR_PREFIXES: &[&str] = &[
        "text",
        "bg",
        "border",
        "decoration",
        "outline",
        "ring",
        "divide",
        "fill",
        "stroke",
    ];

    /// The CSS property a class sets, for the handful of properties two composed
    /// tokens actually compete over.
    ///
    /// Not a model of Tailwind: a general one would be a second implementation
    /// of the framework, and would rot. This covers colour, padding and
    /// background, which is where the real collisions are.
    fn property_of(class: &str) -> Option<&'static str> {
        if class.contains(':') {
            // A state variant (`hover:text-…`) applies in a state the base rule
            // does not, so the two cannot contradict each other.
            return None;
        }
        let (prefix, rest) = class.split_once('-')?;
        let family = rest.split('-').next().unwrap_or("");
        if COLOUR_PREFIXES.contains(&prefix) && COLOUR_FAMILIES.contains(&family) {
            return Some("colour");
        }
        if matches!(prefix, "p" | "px" | "py" | "pt" | "pb" | "pl" | "pr") {
            return Some("padding");
        }
        None
    }

    fn properties(token: &StaticClass) -> BTreeSet<&'static str> {
        classes(token).into_iter().filter_map(property_of).collect()
    }

    fn has_padding(token: &StaticClass) -> bool {
        classes(token)
            .iter()
            .any(|name| !name.contains(':') && name.starts_with('p'))
    }

    #[test]
    fn tokens_are_never_empty() {
        for token in ALL_TOKENS {
            assert!(
                !rendered(token).trim().is_empty(),
                "an empty token has no effect and no reason to exist"
            );
        }
    }

    /// Tailwind scans for class names as literal text. If a token is ever built
    /// at runtime, the class silently disappears from the stylesheet and the
    /// element renders unstyled — no error, no test failure, just a page that
    /// looks wrong. Guard the invariant directly.
    #[test]
    fn tokens_contain_no_whitespace_runs_or_line_breaks() {
        for token in ALL_TOKENS {
            let value = rendered(token);
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
    /// space-separated list. A missing space silently glues two classes into one
    /// nonsense class name that Tailwind will never generate.
    #[test]
    fn continued_literals_join_with_single_spaces() {
        for token in ALL_TOKENS {
            for name in classes(token) {
                assert!(!name.is_empty(), "empty class name in {token:?}");
                assert!(
                    !name.starts_with('"') && !name.ends_with('"'),
                    "an unbalanced literal leaked a quote: {name:?}"
                );
            }
        }
    }

    /// The disclosure only works if the two halves agree: an element with the
    /// peer class, and a nav whose closed height is overridden when that peer is
    /// checked. Lose either half and the menu silently stops opening.
    #[test]
    fn the_disclosure_is_wired_from_both_ends() {
        assert!(classes(&NAV_TOGGLE).contains(&"peer"));
        assert!(classes(&NAV).contains(&"peer-checked:max-md:h-[120px]"));

        // The closed state must be scoped to mobile too, or the nav would be
        // permanently collapsed on desktop.
        assert!(classes(&NAV).contains(&"max-md:h-0"));
        assert!(classes(&NAV).contains(&"peer-checked:max-md:h-[120px]"));
    }

    /// Both height utilities must live behind a mobile-only variant. Above `md`
    /// the nav is always open, so an unscoped zero height would hide the whole
    /// menu on desktop.
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

    /// A nav link is rendered as `[NAV_LINK, NAV_LINK_CURRENT if current]`, so
    /// if the two ever set the same property the winner is decided by stylesheet
    /// order rather than by which one is meant. The colour moved to [`NAV`] for
    /// exactly this reason; this test is what keeps it there.
    #[test]
    fn nav_link_states_never_contradict_the_base() {
        let shared: Vec<&str> = properties(&NAV_LINK)
            .intersection(&properties(&NAV_LINK_CURRENT))
            .copied()
            .collect();
        assert!(
            shared.is_empty(),
            "the nav link and its current state compete over {shared:?}"
        );

        // ...and the nav still carries the colour the link no longer does.
        assert!(
            classes(&NAV).contains(&"text-mist-300"),
            "no nav token sets the link colour, so links would inherit the body's"
        );
    }

    /// Every other pair of tokens the markup composes, checked the same way.
    #[test]
    fn tokens_meant_to_be_composed_never_contradict_each_other() {
        let pairs: &[(&str, &StaticClass, &StaticClass)] = &[
            ("index heading", &INDEX_HEAD, &INDEX_COLUMN_ARTIST),
            ("index cell", &INDEX_CELL, &INDEX_COLUMN_ARTIST),
            ("index row", &INDEX_ROW, &INDEX_CELL),
            ("song heading", &SONG_HEADING, &SONG_TITLE),
            ("song sheet", &SONG_SHEET, &SONG_HEADING),
        ];

        for (label, one, other) in pairs {
            let shared: Vec<&str> = properties(one)
                .intersection(&properties(other))
                .copied()
                .collect();
            assert!(shared.is_empty(), "{label} competes over {shared:?}");
        }
    }

    /// v4 is dark-only: no token may carry a colour-scheme variant, because
    /// there is no second scheme for it to resolve to, and the variant would
    /// make the token look conditional when it is not.
    #[test]
    fn no_token_is_conditional_on_a_colour_scheme() {
        for token in ALL_TOKENS {
            for name in classes(token) {
                assert!(
                    !name.starts_with("dark:"),
                    "a dark: variant in a dark-only design: {name:?}"
                );
            }
        }
    }

    /// Print is the reason this rewrite has print tokens at all: a songbook gets
    /// printed. Both pieces of chrome must be gone and the background must be
    /// cancelled, or a printed lyric sheet is a dark rectangle.
    #[test]
    fn print_drops_chrome_and_background() {
        assert!(rendered(&HEADER).contains("print:hidden"));
        assert!(rendered(&FOOTER).contains("print:hidden"));
        let shell = rendered(&SHELL);
        assert!(shell.contains("print:bg-white"));
        assert!(shell.contains("print:text-black"));
    }

    /// Every colour name a token uses must exist in the palette `build.rs`
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
            for colour in palette_names_in(name) {
                if !defined.contains(colour) {
                    missing.insert(colour.to_string());
                }
            }
        }

        assert!(
            missing.is_empty(),
            "tokens use colours with no palette entry: {missing:?}"
        );
    }

    /// Every `tahiti-…`, `ink-…` and `mist-…` name inside a class, with any
    /// opacity modifier and every surrounding utility stripped off.
    ///
    /// Hand-rolled rather than regex-based: `regex-lite` is a dependency of the
    /// crate, but a unit test that adds a regex engine to the *test* build to
    /// find three prefixes is not a trade worth making.
    fn palette_names_in(class: &str) -> Vec<&str> {
        const PREFIXES: &[&str] = &["tahiti", "ink-", "mist-"];
        let mut found = Vec::new();
        let mut rest = class;

        while let Some((at, _)) = PREFIXES
            .iter()
            .filter_map(|prefix| rest.find(prefix).map(|at| (at, prefix)))
            .min_by_key(|(at, _)| *at)
        {
            let tail = &rest[at..];
            let end = tail
                .find(|c: char| !(c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-'))
                .unwrap_or(tail.len());
            // Stop before an opacity modifier: `tahiti-900/30`.
            found.push(tail[..end].trim_end_matches('-'));
            rest = &tail[end..];
        }

        found
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
        for needed in [
            "ink-950", "ink-900", "ink-800", "ink-700", "mist-100", "mist-300", "mist-500",
        ] {
            assert!(
                seen.contains(needed),
                "the dark surface system needs {needed}"
            );
        }
    }

    /// Typography has to land somewhere, and the headings are where it lands.
    /// The display-face utility is generated from `--font-display`, which
    /// `build.rs` writes from `palette::FONT_DISPLAY`, which `src/ui/fonts.rs`
    /// checks against the family it actually declares. Break any link in that
    /// chain and the heading renders in the body face with no error anywhere.
    #[test]
    fn headings_use_the_display_face() {
        for heading in [H1, HERO_TITLE, PANEL_TITLE, SONG_TITLE, FOOT_TITLE] {
            assert!(
                classes(&heading).contains(&"font-display"),
                "a heading does not use the display face"
            );
        }
    }

    /// The body face is the *default*, applied by Tailwind's preflight from
    /// `--font-sans`. Asking for it by name would be a second, weaker copy of
    /// that decision — and would survive a change to the preflight default while
    /// quietly not following it.
    #[test]
    fn no_token_names_the_body_face_explicitly() {
        for token in ALL_TOKENS {
            assert!(
                !classes(token).contains(&BODY_FACE),
                "a token names the default face"
            );
        }
    }

    /// Every weight a token asks a family for must be a face that family ships.
    ///
    /// Browsers synthesise a missing weight by smearing the nearest one, and a
    /// missing oblique by shearing the upright. Neither is logged, neither fails
    /// a build, and both look like a typography mistake. The faces are read from
    /// `src/ui/fonts.rs` — the declaration the browser is actually sent — rather
    /// than from a list duplicated here, so adding a weight to a family is
    /// enough to make it available to a token.
    #[test]
    fn every_weight_a_token_asks_for_is_a_face_that_ships() {
        use crate::ui::{fonts, palette};

        let mut shipped: BTreeMap<&str, BTreeSet<u16>> = BTreeMap::new();
        let mut any_slanted = false;
        for font in fonts::ALL {
            for face in font.faces().iter() {
                if let Some(range) = face.weight() {
                    let start: u16 = range.start().to_string().parse().expect("a weight");
                    let end: u16 = range.end().to_string().parse().expect("a weight");
                    shipped
                        .entry(font.family())
                        .or_default()
                        .extend(start..=end);
                }
                any_slanted |= face
                    .style()
                    .is_some_and(|style| !matches!(style, topcoat::font::FontStyle::Normal));
            }
        }
        assert!(
            shipped.contains_key(palette::FONT_DISPLAY),
            "the font registry did not yield the display family"
        );

        for token in ALL_TOKENS {
            // A token names at most one family; the first one wins, and naming
            // none means the body face, which is the document default.
            let family = if classes(token).contains(&"font-display") {
                palette::FONT_DISPLAY
            } else if classes(token).contains(&"font-mono") {
                palette::FONT_MONO
            } else {
                palette::FONT_SANS
            };

            for name in classes(token) {
                if let Some(weight) = weight_of(name) {
                    assert!(
                        shipped
                            .get(family)
                            .is_some_and(|weights| weights.contains(&weight)),
                        "a token asks {family} for weight {weight}, which it does not ship \
                         (has {:?})",
                        shipped.get(family)
                    );
                }
                if name == SLANTED {
                    assert!(
                        any_slanted,
                        "a token asks for a slanted face, and no family ships one"
                    );
                }
            }
        }
    }

    /// The class names the guards below have to compare against, written so that
    /// Tailwind's scanner does not see them.
    ///
    /// Tailwind reads this file's *text*, not its meaning: every class-shaped
    /// word in it becomes a candidate whether or not anything uses it. A table
    /// listing the nine weight utilities would therefore put nine unusable rules
    /// into every stylesheet, and this module — which exists to keep the
    /// stylesheet honest — would be the thing making it dishonest. `concat!`
    /// resolves at compile time, so the constants are ordinary `&str` data; the
    /// only difference is what the scanner reads.
    const WEIGHT_UTILITIES: &[(&str, u16)] = &[
        (concat!("font-", "thin"), 100),
        (concat!("font-", "extralight"), 200),
        (concat!("font-", "light"), 300),
        (concat!("font-", "normal"), 400),
        (concat!("font-", "medium"), 500),
        (concat!("font-", "semibold"), 600),
        (concat!("font-", "bold"), 700),
        (concat!("font-", "extrabold"), 800),
        (concat!("font-", "black"), 900),
    ];

    /// The utility that asks for a slanted face, and the one that asks for the
    /// default one — the two names the guards need and no token uses.
    const SLANTED: &str = concat!("ital", "ic");
    const BODY_FACE: &str = concat!("font-", "sans");

    fn weight_of(class: &str) -> Option<u16> {
        WEIGHT_UTILITIES
            .iter()
            .find(|(name, _)| *name == class)
            .map(|(_, weight)| *weight)
    }

    /// [`CARD_ROOMY`], [`TABLE_PANEL`], [`INDEX_PANEL`] and [`SONG_SHEET`]
    /// repeat [`CARD`]'s surface classes because a `StaticClass` cannot be
    /// composed from constants — see the comment on [`CARD_ROOMY`]. This is the
    /// check that makes the repetition safe: drop a utility from one of them and
    /// this fails, rather than the panel quietly losing its border on the live
    /// site.
    #[test]
    fn the_panel_variants_keep_the_card_surface() {
        let surface = properties(&CARD);
        assert!(classes(&CARD).contains(&"rounded-card"));
        assert!(classes(&CARD).contains(&"shadow-card"));

        for variant in [CARD_ROOMY, TABLE_PANEL, INDEX_PANEL, SONG_SHEET] {
            let names = classes(&variant);
            for name in classes(&CARD) {
                assert!(names.contains(&name), "a panel variant lost {name:?}");
            }
        }

        for roomy in [CARD_ROOMY, TABLE_PANEL, SONG_SHEET] {
            assert!(has_padding(&roomy), "a roomy panel variant has no padding");
        }
        assert!(
            !has_padding(&INDEX_PANEL),
            "the cells carry the padding; the panel must not add its own"
        );
        assert!(surface.contains(&"colour"), "a card has no fill or border");

        let table = classes(&TABLE_PANEL);
        assert!(table.contains(&"max-md:text-center"));
        assert!(table.contains(&"md:max-w-max"));
    }

    /// The artist column is dropped on a phone, and only there. An unscoped hide
    /// renders a one-column table with no error anywhere, and a missing
    /// table-cell utility lets the hide win at every width.
    #[test]
    fn the_artist_column_hides_only_below_md() {
        assert_eq!(
            classes(&INDEX_COLUMN_ARTIST),
            ["hidden", "md:table-cell"],
            "the column hide has to be scoped below md"
        );
    }

    /// The last row drops its rule, so the panel's own border is the only line
    /// at the foot of the table. Lose this and every index page shows a double
    /// rule under the last song — nothing fails, it just looks wrong.
    #[test]
    fn the_last_index_row_has_no_rule() {
        assert!(classes(&INDEX_ROW).contains(&"last:border-b-0"));
    }

    /// The same invariant as `every_colour_used_by_a_token_exists`, for depth.
    /// A rounded-card with no `--radius-card` behind it emits no rule at all and
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

    /// One name per namespace, for the reason
    /// `the_palette_defines_each_name_once` gives about colours: a duplicate
    /// lets the first definition silently lose.
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

    /// The lyric sheet only works if the three tokens agree about what draws a
    /// chord: it is positioned against a positioned line, it takes the accent,
    /// and it resets the leading it would otherwise inherit from [`LYRICS`].
    #[test]
    fn the_lyric_sheet_places_chords_against_positioned_lines() {
        let line = classes(&LYRIC_LINE);
        let chord = classes(&CHORD);

        assert!(
            line.contains(&"relative"),
            "a chord has nothing to anchor to"
        );
        assert!(
            chord.contains(&"absolute"),
            "a chord would add width to the line"
        );
        assert!(
            chord.iter().any(|c| c.starts_with("-ml-")),
            "a chord would sit after its syllable instead of over it"
        );
        assert!(
            chord.contains(&"leading-none"),
            "a chord would inherit the lyric's doubled leading and drift into the words"
        );
        assert!(
            chord.iter().any(|c| c.starts_with("text-tahiti-")),
            "a chord is not told apart from the lyric"
        );
    }
}
