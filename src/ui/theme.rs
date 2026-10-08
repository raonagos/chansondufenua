//! Design tokens — the whole of v4's styling vocabulary.
//!
//! Every entry is a Tailwind utility list stored as a [`StaticClass`], so the
//! markup in `src/ui/` and `src/pages/` reads as composition rather than as
//! markup full of class soup. v3 expressed a similar vocabulary as `@apply`
//! blocks in `style/tailwind.scss`; this module keeps the *mechanism* and
//! repaints the design on purpose.
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

/// The language switcher: a globe that opens the flag list, in the header of
/// every page.
///
/// **In the header row, not inside [`NAV`].** The nav below `md` is a disclosure
/// with a fixed open height, and a third line inside it would either overflow
/// that box or change it on every screen; here the switcher sits beside the logo
/// on every width, always drawn, and needs no script to reach. It is chrome, so
/// [`HEADER`]'s `print:hidden` already keeps it out of a printed songbook.
///
/// The colour lives on the container, for the reason [`NAV`]'s does: the summary
/// is composed with a colour state of its own, and two colour utilities in one
/// class list are resolved by stylesheet order rather than by intent. The globe
/// is drawn in `currentColor`, so it takes this value too.
///
/// `relative` because the flag list hangs from this box: the nav is what does not
/// move when the disclosure opens, so the list is positioned against it rather
/// than against the header.
///
/// `flex-none` rather than the more familiar no-shrink utility: that one's name
/// contains the text of a colour this palette does not define, and
/// `every_colour_used_by_a_token_exists` reads class *names* as text — it would
/// read that utility as `ink-0` and fail.
pub const LANGUAGE_SWITCH: StaticClass =
    class!("relative mr-3 md:mr-6 flex flex-none items-center text-sm text-mist-300");

/// The control that opens the flag list: the globe, and the word that names it.
///
/// A `<summary>`, so the disclosure is the browser's own and needs no script:
/// closed it shows the globe, and open it heads the list below. Its name is
/// [`VISUALLY_HIDDEN`] — a drawing cannot be read aloud, so the control is named
/// by the catalog's word for *language*, in the page's own language.
///
/// `cursor-pointer` because a `<summary>` is a control and does not say so with a
/// pointer on its own. `list-none` and the WebKit rule remove the browser's
/// marker: the globe is this design's affordance, and a second, browser-styled
/// one beside it would be a triangle nothing here chose. The rule is an arbitrary
/// variant because the marker was never a class of ours to name.
pub const LANGUAGE_SUMMARY: StaticClass = class!(
    "flex cursor-pointer list-none items-center transition-colors hover:text-mist-100 \
     [&::-webkit-details-marker]:hidden"
);

/// The flag list: one flag per language, hanging under the summary.
///
/// Composed with [`CARD`], the site's one raised panel, so the list is the same
/// surface as everything else that floats over a page; this token adds where it
/// sits and how the flags are stacked. `right-0` because the switcher is at the
/// right of the header — a list aligned to its left edge would hang off the page
/// on a phone — and the z-index clears the header's, where the mobile nav's own
/// box reaches up beside it.
///
/// The padding is this token's rather than [`CARD`]'s for [`CARD_ROOMY`]'s
/// reason: the panel is the surface and the padding is chosen where it is placed.
pub const LANGUAGE_MENU: StaticClass =
    class!("absolute right-0 top-full z-50 mt-2 flex flex-col items-end gap-2 p-2");

/// One flag in the list.
///
/// A bare box: the drawing is the whole of it. `rounded-xs` is there for the
/// current language's ring, so the ring follows the flag's own corner instead of
/// drawing a rectangle around it.
pub const LANGUAGE_FLAG: StaticClass =
    class!("block rounded-xs transition-opacity hover:opacity-80");

/// The language the page is already in.
///
/// **A ring, not the underline [`NAV_LINK_CURRENT`] uses.** A link's content is a
/// drawing now, and `text-decoration` has no text to underline — marked the old
/// way, the current language would be marked invisibly, which is the failure
/// [`NAV_LINK_CURRENT`]'s own comment warns about. The accent is the one the
/// nav's underline and every focus ring here use, so the three agree.
pub const LANGUAGE_FLAG_CURRENT: StaticClass = class!("ring-2 ring-tahiti-400");

/// Text for assistive technology only: the words that name a drawing.
///
/// Tailwind's own visually-hidden utility, named once here so that no view spells
/// a class — [`NAV_TOGGLE`] uses the same one to hide a checkbox that is still
/// focusable, and the switcher uses it for the two names its pictures cannot
/// carry: the control's, and each flag's.
pub const VISUALLY_HIDDEN: StaticClass = class!("sr-only");

/// The header's search box: one field, from every page, to `/paimi`.
///
/// A row of two — the field and the button. Below `md` it takes a line of its
/// own (`basis-full`), because a phone's header has no room left beside the
/// logo, the switcher and the hamburger and a field squeezed between them would
/// be narrower than the word it takes; from `md` up it sits in the header row,
/// between the language switcher and the nav's links, where the row has space to
/// spare. That order is the markup's own: the box is written after the hamburger
/// and before the nav, so the phone's header breaks into the chrome's row and
/// then this one.
///
/// **In the header row, not inside the nav.** The nav below `md` is a
/// disclosure, so a box in it would be behind the hamburger — and a search a
/// reader has to open a menu to find is the complaint this step answers.
pub const SEARCH_FORM: StaticClass =
    class!("flex items-center gap-2 max-md:basis-full max-md:mt-3 md:mr-6");

/// The header's search field.
///
/// The form's field grows to the row it is in and stops at a fixed width from
/// `md` up, so the header is the same width on every page it appears on
/// regardless of how long a word the catalog holds: `flex-1` below `md` fills
/// the phone's line, and `md:w-56` is the desktop box. `min-w-0` because a flex
/// item refuses to shrink below its content without it.
///
/// The rest is [`FORM_INPUT`]'s surface at the header's own scale — same border,
/// same recessed field, same muted placeholder — written out rather than
/// composed, for the reason that token's doc gives: a [`StaticClass`] is one
/// string literal and cannot inherit another's classes.
pub const SEARCH_INPUT: StaticClass = class!(
    "flex-1 min-w-0 appearance-none rounded-md border border-ink-600 bg-ink-950 px-3 \
     py-1.5 text-sm text-mist-100 shadow-sm placeholder:text-mist-500 md:w-56 md:flex-none"
);

/// The header's search button.
///
/// [`FORM_SUBMITTER`]'s accent fill at the size of a control that sits in the
/// chrome: the same two colours, so one button in the header is recognisably the
/// same action as the one on the search page.
pub const SEARCH_SUBMIT: StaticClass = class!(
    "cursor-pointer rounded-md bg-tahiti-500 px-3 py-1.5 text-sm font-semibold \
     text-ink-950 transition-colors hover:bg-tahiti-300"
);

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

/// The footer's link to the support page: a heart, and the word beside it.
///
/// Composed with [`LINK`], which carries the word's colour, underline and focus
/// ring — this token is only the row's shape, and it adds nothing LINK would
/// have to fight over. Two things it says: the drawing and the word sit on one
/// line with a gap between them rather than on two, and the link is set apart
/// from the sentence above it by the one margin that is this link's own. An
/// inline flex box, because that is what holds an icon and a word on one
/// baseline; the footer's own centring applies to it like any other inline
/// content.
///
/// The gap is written as a flex gap rather than a space in the markup: the
/// two halves are an element and a text node, and whitespace between them is the
/// browser's to collapse.
pub const SUPPORT_LINK: StaticClass = class!("mt-6 inline-flex items-center gap-2");

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
/// prose to be read line by line — v3's own width, kept.
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
///
/// The panel prints white. A browser does not print the page's background, so
/// the sheet's dark panel and its light text would print as a blank page — see
/// [`LYRICS`], [`SONG_TITLE`] and [`CHORD`] for the other half of that.
pub const SONG_SHEET: StaticClass = class!(
    "rounded-card border border-ink-700 bg-ink-900 shadow-card p-6 md:p-10 \
     print:bg-white"
);

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
pub const SONG_TITLE: StaticClass = class!(
    "font-display text-3xl md:text-4xl font-bold tracking-tight text-balance text-mist-100 \
     print:text-black"
);

/// Who wrote or sings the song, under its title.
///
/// **Not in v3**, which carried the credits only inside the page's metadata.
/// The index shows them in a column, so a reader arriving from the index already
/// knows them, and a reader arriving from a search result never saw them at all.
/// They are the song's own data, rendered where they belong.
pub const SONG_ARTISTS: StaticClass = class!("text-lg text-mist-300 print:text-black");

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
/// *above* its syllable, so the half of each line box above the text is where it
/// lives. [`CHORD`] takes that space out of the flow, so the lyric itself reads
/// continuously — "Hina'a" and "ro" stay adjacent even though a chord sits
/// between them in the source.
///
/// Black on paper for the reason [`SONG_SHEET`] gives: left in the dark page's
/// own text colour, the printed sheet is a white page with nothing on it.
pub const LYRICS: StaticClass = class!("text-lg leading-loose text-mist-100 print:text-black");

/// One line of lyric.
///
/// The chords are drawn from their own syllable ([`CHORDED`]), so the line is no
/// longer the box they are measured against. It stays positioned anyway, as it
/// was in v3, because of the one chord that hangs from a gap rather than from a
/// letter: if an engine drops the collapsed box at the end of a line, the line
/// is what keeps the chord inside its own row instead of leaving it measured
/// against the page.
pub const LYRIC_LINE: StaticClass = class!("relative");

/// The blank line between two verses.
///
/// A fixed step rather than an empty line of the lyric's own height: a verse
/// break is a visual pause, and a full line box would read as a missing line.
pub const LYRIC_GAP: StaticClass = class!("h-6");

/// The syllable a chord is drawn over: a box around the character — or the gap —
/// the chord was written against.
///
/// **An inline box, not an atomic one**, although sizing the syllable as a box
/// of its own is the obvious way to write this. An atomic inline is a
/// line-breaking opportunity on both sides in every browser (UAX #14 treats it
/// as an object replacement), and this corpus writes most of its chords *inside*
/// a word — `rei<sup>F</sup>nes` — so an atomic syllable would let a narrow
/// screen break the word at the chord. A positioned inline forms the same
/// containing block for absolutely positioned children, and adds no break where
/// there was none.
///
/// One em tall, not the lyric's doubled leading: the chord is then drawn from
/// the top of the glyph box rather than the top of the line box, so it lands in
/// the leading space — clear of the words below it and of the descenders of the
/// line above. The line box does not grow: the inline box is shorter than the
/// line's strut, and [`CHORD`] is out of the flow entirely.
pub const CHORDED: StaticClass = class!("relative leading-none");

/// A chord, drawn above the syllable it was written against.
///
/// Absolutely positioned, so it adds no width: the lyric underneath reads as one
/// unbroken line, which is what makes a chord sheet usable for singing. It is
/// anchored to the *syllable's* box and centred over it, rather than pulled back
/// over the lyric by a fixed negative margin — which is what put v3's chords
/// over the middle of a word instead of over its vowel.
///
/// The top offset is reset to `auto` and that is load-bearing, not tidiness:
/// Tailwind's preflight already positions `sup` and gives it a top offset, so a
/// chord with a bottom offset *and* preflight's top offset would be stretched
/// between the two. The leading is reset to one, or the chord would inherit the
/// lyric's doubled leading and drift down into the words.
///
/// Monospace, because chord labels are short, stacked and must not collide with
/// the words they sit over. At 14px against the lyric's 18px it is larger than
/// v3's 12.8px, and as large as it goes without neighbours colliding in the
/// corpus's densest lines. No weight is added because there is none to add: the
/// mono face ships one (see `crate::ui::fonts`), and asking for a bold one would
/// be answered by the browser's synthesiser rather than the type designer.
///
/// `tahiti-200` measures 13.4:1 against the sheet's `ink-900`, against
/// `tahiti-300`'s 11.5:1 — the lightest stop the accent ramp has, so a chord
/// still reads as the accent without v3's washed-out look. On paper it becomes
/// `tahiti-900`, 9.1:1 on white; left as `tahiti-200` a printed chord would be
/// invisible.
pub const CHORD: StaticClass = class!(
    "absolute top-auto bottom-full left-1/2 -translate-x-1/2 font-mono text-sm \
     leading-none whitespace-nowrap text-tahiti-200 print:text-tahiti-900"
);

/// The transposition control in a sheet header: the label, the two step links
/// and the step they are on.
///
/// A pill on the sheet's own surface, one size down from the header's button,
/// because it is a reader's control over the lyric rather than an action on the
/// song. It is hidden on paper: a printed sheet is a sheet at one offset, and a
/// row of buttons on it is ink spent on a control no reader of paper can press.
pub const TRANSPOSE: StaticClass = class!(
    "flex flex-none items-center gap-1 rounded-full border border-ink-700 bg-ink-800 \
     px-3 py-1 text-sm text-mist-300 print:hidden"
);

/// The word naming the transposition control.
///
/// Inside [`TRANSPOSE`]'s colour and size; the row's own label, so it is never
/// the loudest thing in it.
pub const TRANSPOSE_LABEL: StaticClass = class!("mr-1");

/// One step link of the transposition control — down, or up.
///
/// Full-contrast against the label, because it is the part that is pressable,
/// and `flex-none` so a long label does not squeeze it at a narrow width.
pub const TRANSPOSE_LINK: StaticClass =
    class!("flex-none rounded-full px-2 text-mist-100 transition-colors hover:text-tahiti-300");

/// The step the sheet is on, between the two links.
///
/// Monospace and tabular, so the number does not shift the links either side of
/// it as it changes width.
pub const TRANSPOSE_VALUE: StaticClass =
    class!("w-8 text-center font-mono tabular-nums text-mist-100");

/// The auto-scroll control in a sheet header: the reader's speed bar.
///
/// [`TRANSPOSE`]'s pill again, beside it on the same row, because both are the
/// reader's controls over the lyric rather than actions on the song. The pill is
/// drawn only once a script has un-hidden it — the attribute that hides it is
/// the one the script removes — so a sheet read with JavaScript off carries no
/// control that cannot work, and the page is what it was before this step. On
/// paper it is hidden for [`TRANSPOSE`]'s reason: a speed bar is for a screen.
///
/// `flex-none` rather than its `shrink` synonym for the reason
/// [`LANGUAGE_SWITCH`] gives.
pub const AUTOSCROLL: StaticClass = class!(
    "flex flex-none items-center gap-2 rounded-full border border-ink-700 bg-ink-800 \
     px-3 py-1 text-sm text-mist-300 print:hidden"
);

/// The word naming what the speed bar sets.
///
/// A real `<label>` for the range rather than a caption beside it, so it names
/// the control for a pointer and for a screen reader with no second string: the
/// pill's own name is read aloud only. Inside [`TRANSPOSE`]'s colour and size.
pub const AUTOSCROLL_SPEED: StaticClass = class!("mr-1");

/// The speed bar itself: a native range input, restyled and not rewritten.
///
/// The browser's own range is the one speed selector a phone, a keyboard and a
/// screen reader all already know how to work, and it is operable without a
/// script — the script only reads its value. Only the accent colour and the
/// track's width are set, so the control keeps the platform's hit area and
/// keyboard behaviour.
pub const AUTOSCROLL_RANGE: StaticClass = class!("h-4 w-20 cursor-pointer accent-tahiti-300");

/// The auto-scroll button: start, and stop.
///
/// [`TRANSPOSE_LINK`]'s shape, one word wider, with the word kept on one line:
/// the label changes from the start word to the stop word while the sheet is
/// moving, and a button that re-wrapped would shift the row each time.
pub const AUTOSCROLL_BUTTON: StaticClass = class!(
    "flex-none rounded-full px-2 whitespace-nowrap text-mist-100 \
     transition-colors hover:text-tahiti-300"
);

// ---------------------------------------------------------------------------
// Several songs on one page
// ---------------------------------------------------------------------------

/// One chosen song on `/puta-himene`: the step between two sheets, and the
/// rule that keeps a lyric off a page break.
///
/// Composed with [`SONG_SHEET`], which brings the panel and the print colours —
/// the pair is pinned in `tokens_meant_to_be_composed_never_contradict_each_other`.
/// The two do not compete: this adds spacing and a break rule, and the sheet owns
/// the surface. On paper the break rule is the whole point of the token — a
/// songbook whose songs are cut in half by the printer is the defect this page
/// was asked for in the first place.
pub const BOOK_ITEM: StaticClass = class!("mb-8 print:break-inside-avoid");

/// The index's link to the picker.
///
/// Under the table rather than in the nav: choosing several songs to read
/// together is the index's own next step, not a fourth section of the site.
pub const BOOK_ENTRY: StaticClass = class!("mt-6 text-center");

/// One song in the picker: a checkbox, its title, and its credits.
///
/// The rule between rows is the reason this is a token rather than a bare list
/// item — the last row drops it, the way [`INDEX_ROW`] does, so the panel's own
/// edge is the only line at the foot.
pub const BOOK_PICK_ROW: StaticClass =
    class!("flex items-center gap-3 border-b border-ink-700 py-3 last:border-b-0");

/// The picker's checkbox.
///
/// `flex-none` rather than its `shrink` synonym for the reason
/// [`LANGUAGE_SWITCH`] gives — and the box is a square, so it is sized in one
/// utility rather than two. The accent colour is the same one the song sheet's
/// speed bar uses, so the site's controls agree about what is picked.
pub const BOOK_PICK_BOX: StaticClass = class!("size-4 flex-none accent-tahiti-300");

/// The credits beside a title in the picker.
///
/// Muted, and smaller than the title they follow: on this page the title is what
/// the reader is choosing between, and two songs by one artist are common enough
/// that the credits have to be readable without being the point.
pub const BOOK_PICK_ARTISTS: StaticClass = class!("text-sm text-mist-500");

// ---------------------------------------------------------------------------
// Pagination
// ---------------------------------------------------------------------------

/// The pagination nav under the index table.
///
/// A row of page numbers, centred under the table it pages through. Hidden on
/// paper for the reason the header and the footer are: a printed catalogue does
/// not have a page 2 to turn to, and a row of links in the margin of a songbook
/// is chrome that has stopped meaning anything.
pub const PAGINATION: StaticClass =
    class!("mt-6 flex flex-wrap items-center justify-center gap-2 print:hidden");

/// One page number in the nav — a link to a page that is not the one served.
///
/// A rounded outline rather than a bare number, so the numbers read as a row of
/// controls instead of a stray column of digits; the colour is the muted one the
/// nav links use, and the accent appears on hover only.
pub const PAGINATION_LINK: StaticClass = class!(
    "inline-flex items-center justify-center rounded-full border border-ink-600 px-3 py-1 \
     text-sm font-semibold text-mist-300 transition-colors hover:border-tahiti-400 \
     hover:text-tahiti-300"
);

/// The page the reader is on.
///
/// Not a link, and deliberately not composed with [`PAGINATION_LINK`]: a page
/// that linked to itself would be a link that goes nowhere new, and the two
/// tokens would then compete over the same properties — the trap the nav's own
/// state pair is written around. `aria-current` is what tells a screen reader
/// which number this is, the same way it does in the language switcher.
pub const PAGINATION_CURRENT: StaticClass = class!(
    "inline-flex items-center justify-center rounded-full border border-tahiti-400 bg-ink-800 \
     px-3 py-1 text-sm font-semibold text-tahiti-200"
);

// ---------------------------------------------------------------------------
// Create-song form
// ---------------------------------------------------------------------------

/// The panel the create-song form sits in.
///
/// [`CARD_ROOMY`]'s surface again, written out for the reason that token gives:
/// a [`StaticClass`] cannot be composed from constants. The bottom margin is
/// v3's `mb-4` on the same element.
pub const FORM_PANEL: StaticClass =
    class!("rounded-card border border-ink-700 bg-ink-900 shadow-card p-8 mb-4");

/// One labelled field: the space under every label-and-control pair.
pub const FORM_FIELD: StaticClass = class!("mb-4");

/// A form label.
///
/// Small and muted against the page title, because a form is read as a list of
/// captions rather than as prose.
pub const FORM_LABEL: StaticClass = class!("mb-2 block text-sm font-semibold text-mist-300");

/// A text input in the form.
///
/// The panel is `ink-900`, so a field is *darker* than its panel — the reverse
/// of the usual "input is lighter" convention, and the right way round on a
/// design where the panel is already a step up from the page. No focus rule
/// here: every call site composes [`FOCUS`], which is one ring definition for
/// the whole site.
pub const FORM_INPUT: StaticClass = class!(
    "w-full appearance-none rounded-md border border-ink-600 bg-ink-950 px-3 py-2 \
     text-mist-100 shadow-sm placeholder:text-mist-500"
);

/// The chip row under the artist field — hidden entirely while it is empty.
///
/// `empty:hidden` is what keeps an empty row from leaving a rounded strip of
/// panel under the input: with no children the list takes no space at all.
pub const FORM_CHIPS: StaticClass =
    class!("mt-2 flex max-w-max flex-wrap gap-2 rounded-md bg-ink-950 p-2 empty:hidden");

/// One picked artist.
pub const FORM_CHIP: StaticClass = class!(
    "flex max-w-max items-center rounded-sm border border-ink-600 px-4 py-2 \
     text-mist-100"
);

/// The chip's remove button.
///
/// A real `<button>`, not v3's `role="button"` on the `<li>`: the chip is a
/// label, and the one thing it does — take the artist off the list — is the
/// thing that should be focusable.
pub const FORM_CHIP_DELETE: StaticClass =
    class!("cursor-pointer text-red-400 transition-colors hover:text-red-300");

/// The row of chord buttons above the editor.
pub const FORM_CHORD_ROW: StaticClass = class!("mb-2 flex flex-wrap-reverse");

/// A chord button.
///
/// v3's fill, kept: the chord row is the one place in the form where the accent
/// is a surface, because pressing a chord is the form's primary repeated action.
pub const FORM_CHORD_BUTTON: StaticClass = class!(
    "cursor-pointer bg-tahiti-700 px-4 py-2 font-semibold text-mist-100 \
     transition-colors hover:bg-tahiti-900"
);

/// The label wrapping one modifier checkbox (`m`, `b`, `#`).
pub const FORM_CHBX_LABEL: StaticClass = class!(
    "flex cursor-pointer items-center gap-2 bg-tahiti-700 px-2 font-semibold text-mist-100 \
     transition-colors hover:bg-tahiti-900"
);

/// The custom-chord field and its insert button, kept together on one line.
pub const FORM_CUSTOM: StaticClass = class!("flex md:ml-2");

/// The custom-chord text input — narrow, because a chord is two or three
/// characters and a full-width field would read as another lyric box.
pub const FORM_CUSTOM_INPUT: StaticClass = class!(
    "max-w-[17ch] appearance-none rounded-sm border border-ink-600 bg-ink-950 px-3 py-2 \
     text-mist-100 shadow-sm placeholder:text-mist-500"
);

/// The button that inserts a custom chord.
pub const FORM_CUSTOM_BUTTON: StaticClass =
    class!("cursor-pointer bg-tahiti-700 px-2 text-mist-100 transition-colors hover:bg-tahiti-900");

/// The lyric editor: a `contenteditable` surface.
///
/// `min-h` rather than `h`: a lyric is longer than the box, and the box should
/// grow with it instead of scrolling inside itself. Doubled leading for the same
/// reason the sheet has it — the chords are drawn above their line.
///
/// The two arbitrary variants style the chords the editor inserts. They are
/// **in flow**, unlike [`CHORD`] on the song page, and that is deliberate: a
/// chord absolutely positioned over a line is right for singing from and wrong
/// for editing — the caret would sit somewhere other than where the text
/// appears. The editor tints and monospaces a chord so it reads as markup rather
/// than as a syllable; the sheet is where it becomes a chord above the word.
pub const FORM_EDITOR: StaticClass = class!(
    "min-h-[200px] rounded-md border border-ink-600 bg-ink-950 p-4 text-lg leading-loose \
     text-mist-100 [&_sup]:font-mono [&_sup]:text-tahiti-300"
);

/// The form's submit row.
pub const FORM_SUBMIT: StaticClass = class!("flex items-center justify-between");

/// The save button.
///
/// The accent fill, one step larger than [`BUTTON_SMALL`]: this is the one
/// action the page exists for.
pub const FORM_SUBMITTER: StaticClass = class!(
    "cursor-pointer rounded-full bg-tahiti-500 px-8 py-3 text-lg font-semibold text-ink-950 \
     shadow-lg transition-colors hover:bg-tahiti-300"
);

/// What the form says when the domain rejected the submission.
///
/// Above the form and not beside a field: [`Song::validate`] reports the first
/// rule that failed, and the message it carries is written for a log line
/// (`expected 100..=6000 characters, got 42`) rather than for a visitor. The
/// page names the two fields a person can actually fix instead.
pub const FORM_ERROR: StaticClass =
    class!("mb-4 rounded-md border border-red-400 bg-ink-950 p-4 text-red-300");

// ---------------------------------------------------------------------------
// Support
// ---------------------------------------------------------------------------

/// A donation address on the support page: the address itself.
///
/// A block, so the copy control can sit under it, and it wraps wherever the
/// characters run out rather than pushing a 44-character string past the edge of
/// its panel — a phone is exactly where an address is hardest to read and
/// easiest to mistype, and an address that has to be scrolled sideways is one
/// that gets copied by hand off the end of a line.
///
/// The monospace face the chords and the controls already use, because an
/// address is a string of characters to be compared one at a time, not a word.
/// Full-strength text: this is the page's content.
pub const SUPPORT_ADDRESS: StaticClass = class!("block font-mono text-sm break-all text-mist-100");

/// The line above the addresses on the support page: what the money pays for.
///
/// A step louder than the standfirst above it — full-strength text against the
/// muted one — because it is the page's one warm line and everything below it is
/// a table of strings. The measure and the centring follow the standfirst's, so
/// the two read as a pair rather than as two blocks.
pub const SUPPORT_MONEY: StaticClass =
    class!("mx-auto mb-10 max-w-prose text-balance text-mist-100");

/// The row of chain marks inside a support card.
///
/// Decoration only: every mark is a [`crate::ui::icons`] drawing in
/// `currentColor`, and the chain's name is the card's own heading, so this token
/// is the whole of the marks' styling. The row is what keeps the four EVM chains
/// reading as four — laid out in a line, in the order the heading names them,
/// rather than run together into one wide shape.
pub const SUPPORT_MARKS: StaticClass = class!("mb-3 flex items-center gap-2 text-tahiti-400");

/// The copy control under an address.
///
/// [`BUTTON_OUTLINE`]'s shape one size down: it is a control on a three-line
/// panel, not a page's call to action, so it does not take the accent fill the
/// primary buttons have. It is drawn only by the page's own script — the
/// attribute that hides it is the one the script removes — so with JavaScript
/// off the page carries no control that cannot work.
pub const SUPPORT_COPY: StaticClass = class!(
    "mt-4 inline-block cursor-pointer rounded-full border-2 border-ink-600 px-4 py-1 \
     text-sm font-semibold text-mist-100 transition-colors \
     hover:border-tahiti-400 hover:text-tahiti-300"
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
        LANGUAGE_SWITCH,
        LANGUAGE_SUMMARY,
        LANGUAGE_MENU,
        LANGUAGE_FLAG,
        LANGUAGE_FLAG_CURRENT,
        VISUALLY_HIDDEN,
        SEARCH_FORM,
        SEARCH_INPUT,
        SEARCH_SUBMIT,
        MAIN,
        PAGE,
        FOOTER,
        SUPPORT_LINK,
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
        CHORDED,
        CHORD,
        TRANSPOSE,
        TRANSPOSE_LABEL,
        TRANSPOSE_LINK,
        TRANSPOSE_VALUE,
        AUTOSCROLL,
        AUTOSCROLL_SPEED,
        AUTOSCROLL_RANGE,
        AUTOSCROLL_BUTTON,
        BOOK_ITEM,
        BOOK_ENTRY,
        PAGINATION,
        PAGINATION_LINK,
        PAGINATION_CURRENT,
        BOOK_PICK_ROW,
        BOOK_PICK_BOX,
        BOOK_PICK_ARTISTS,
        FORM_PANEL,
        FORM_FIELD,
        FORM_LABEL,
        FORM_INPUT,
        FORM_CHIPS,
        FORM_CHIP,
        FORM_CHIP_DELETE,
        FORM_CHORD_ROW,
        FORM_CHORD_BUTTON,
        FORM_CHBX_LABEL,
        FORM_CUSTOM,
        FORM_CUSTOM_INPUT,
        FORM_CUSTOM_BUTTON,
        FORM_EDITOR,
        FORM_SUBMIT,
        FORM_SUBMITTER,
        FORM_ERROR,
        SUPPORT_ADDRESS,
        SUPPORT_COPY,
        SUPPORT_MONEY,
        SUPPORT_MARKS,
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
            // The multi-lyric page's sheets: the panel and the per-sheet
            // spacing-and-break rule, which must add to the panel rather than
            // fight it.
            ("chosen sheet", &SONG_SHEET, &BOOK_ITEM),
            // The pagination nav and its links: the container owns the layout
            // and the print hide, each link owns its own box.
            ("pagination nav", &PAGINATION, &PAGINATION_LINK),
            // The switcher's list: the raised panel, and the position and
            // stacking rule that hangs it under the summary.
            ("language menu", &CARD, &LANGUAGE_MENU),
            // A flag and the ring that marks the current one.
            ("language flag", &LANGUAGE_FLAG, &LANGUAGE_FLAG_CURRENT),
            // The header's search field and its button, each composed with the
            // one focus rule, which is a state and never a rest state.
            ("header search field", &SEARCH_INPUT, &FOCUS),
            ("header search button", &SEARCH_SUBMIT, &FOCUS),
            // The footer's link to the support page: the shared link styling,
            // and the row's own shape around the drawing beside the word.
            ("footer support link", &LINK, &SUPPORT_LINK),
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
    /// printed. Every piece of chrome must be gone and the background must be
    /// cancelled, or a printed lyric sheet is a dark rectangle.
    #[test]
    fn print_drops_chrome_and_background() {
        assert!(rendered(&HEADER).contains("print:hidden"));
        assert!(rendered(&FOOTER).contains("print:hidden"));
        assert!(rendered(&TRANSPOSE).contains("print:hidden"));
        assert!(rendered(&AUTOSCROLL).contains("print:hidden"));
        assert!(rendered(&PAGINATION).contains("print:hidden"));
        let shell = rendered(&SHELL);
        assert!(shell.contains("print:bg-white"));
        assert!(shell.contains("print:text-black"));
    }

    /// ...and the sheet repaints itself, which cancelling the page background
    /// alone does not do.
    ///
    /// Every colour the sheet uses is set on the element that carries the text,
    /// so it wins over anything the body says. A browser prints no background,
    /// so a printed sheet keeps those colours on white paper: `mist-100` at
    /// 1.03:1 and `tahiti-200` at 1.05:1, which is a blank page. Each of the
    /// three has to say what it becomes on paper, and the chord has to stay
    /// legible rather than merely dark.
    #[test]
    fn print_repaints_the_sheet_in_ink() {
        for (name, token) in [
            ("the lyric", &LYRICS),
            ("the title", &SONG_TITLE),
            ("the credits", &SONG_ARTISTS),
        ] {
            assert!(
                classes(token).contains(&"print:text-black"),
                "{name} would print as it looks on the dark page"
            );
        }
        assert!(
            classes(&SONG_SHEET).contains(&"print:bg-white"),
            "the sheet's panel would print as a dark rectangle"
        );
        assert!(
            classes(&CHORD).contains(&"print:text-tahiti-900"),
            "a chord would print in the colour that is invisible on white"
        );
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

    /// The lyric sheet only works if the three tokens agree about how a chord is
    /// drawn: the line has room above its text, the syllable is a box the chord
    /// can be measured against, and the chord is drawn above that box — centred
    /// over it, out of the flow, in the accent colour.
    ///
    /// The negative margin v3 used to drag a chord back over the lyric is gone
    /// on purpose: it was a fixed distance, so it dropped the chord wherever
    /// half a rem to the left of the flow happened to be — over the middle of
    /// the word, which is the defect this replaces. This test fails if it comes
    /// back.
    #[test]
    fn a_chord_is_drawn_from_the_syllable_it_was_written_against() {
        let line = classes(&LYRIC_LINE);
        let syllable = classes(&CHORDED);
        let chord = classes(&CHORD);

        assert!(
            line.contains(&"relative"),
            "a chord hanging from a gap has no box to fall back to inside its own line"
        );
        assert!(
            syllable.contains(&"relative"),
            "the syllable is not the box the chord is measured against"
        );
        assert!(
            syllable.contains(&"leading-none"),
            "the syllable would inherit the lyric's doubled leading, and its chord \
             would be drawn from the top of the line box instead of the glyph box"
        );
        assert!(
            chord.contains(&"absolute"),
            "a chord would add width to the line"
        );
        assert!(
            chord.contains(&"bottom-full"),
            "a chord would not be drawn from the top of its syllable"
        );
        assert!(
            chord.contains(&"left-1/2") && chord.contains(&"-translate-x-1/2"),
            "a chord is not centred over the syllable it belongs to"
        );
        assert!(
            chord.contains(&"top-auto"),
            "preflight already offsets a sup from the top, so the chord would be \
             stretched between the two offsets"
        );
        assert!(
            chord.contains(&"leading-none"),
            "a chord would inherit the lyric's doubled leading and drift into the words"
        );
        assert!(
            chord.iter().any(|c| c.starts_with("text-tahiti-")),
            "a chord is not told apart from the lyric"
        );
        assert!(
            !chord.iter().any(|c| c.starts_with(concat!("-ml", "-"))),
            "a fixed negative margin puts the chord where the margin lands, not \
             over the syllable it was written against"
        );
    }
}
