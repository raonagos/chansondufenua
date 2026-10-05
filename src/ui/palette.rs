// Design tokens, as Rust data.
//
// This file is read by TWO compilers:
//
//   * `src/ui/mod.rs` includes it as an ordinary module, so the palette is part
//     of the crate and unit tests can assert against it.
//   * `build.rs` `include!`s it verbatim, and writes the `@theme` block Tailwind
//     needs into `$OUT_DIR`. The `include!` is why this file has no `//!`
//     inner docs and no `use` statements: it must compile as the first items of
//     a build-script crate root as happily as it does as a module.
//
// Defining the palette exactly once is the point. Tailwind publishes the
// `--color-*` variables from what `build.rs` derives here, and `class!` bodies
// in `theme.rs` refer to those same names; if the two ever drifted, a class
// would silently resolve to nothing and the element would render unstyled.
// `every_colour_used_by_a_token_exists` in `theme.rs` closes that gap.

/// Three ramps, each with one job.
///
/// **`tahiti-*` is the accent.** These are v3's hex values, byte for byte, so
/// the lagoon cyan that names the site survives the rewrite. They are NOT
/// Tailwind's built-in `cyan-*`, which shares the hue and lightness but carries
/// higher chroma (cyan-700 is oklch 52% 0.105 223.1 against `#0e7490`'s oklch
/// 52.0% 0.094 223.1). Aliasing to `cyan-*` would silently repaint the site,
/// and `tahiti-1000` has no built-in equivalent at all.
///
/// `tahiti-200` and `tahiti-300` are the one addition: a dark page needs a step
/// *lighter* than `tahiti-400` to move to on hover, and v3 — which was designed
/// light-first — never needed one. The three unnumbered entries (`tahiti`,
/// `tahiti-light`, `tahiti-dark`) are v3's aliases, kept so the names in the
/// original stylesheet remain valid.
///
/// **`ink-*` is the surface.** A deep, faintly blue-green neutral: the page is
/// `ink-950`, a panel is `ink-900`, a hovered row is `ink-800`, and `ink-700` /
/// `ink-600` are hairlines. v3 had no such ramp — it floated translucent
/// `neutral-200/10` washes over a full-bleed cyan gradient — and that is the
/// single largest difference in v4's look. A translucent surface takes its value
/// from whatever gradient passes behind it, so the same card reads darker at the
/// top of the page than at the foot, and text contrast is unknowable. These are
/// opaque, so every ratio below is a fact rather than an average.
///
/// **`mist-*` is the text.** Three steps, all measured against `ink-900`:
/// `mist-100` 14.6:1, `mist-300` 9.5:1, `mist-500` 5.5:1. v3's own near-white body
/// text over its mid-gradient cyan was about **1.9:1** — below every threshold in
/// WCAG, and the reason the live site is hard to read in daylight.
///
/// (Nothing in this file's comments spells a utility name. Tailwind v4 scans
/// prose, so a utility named in a comment is *emitted into the stylesheet*
/// whether or not anything uses it — which is how a paragraph of explanation
/// becomes a kilobyte of dead CSS.)
///
/// Adding a colour costs a line here and nothing anywhere else; using one that
/// is not here is what `every_colour_used_by_a_token_exists` refuses.
pub const PALETTE: &[(&str, &str)] = &[
    // Accent — the lagoon, v3's values.
    ("tahiti-light", "#22d3ee"),
    ("tahiti", "#06b6d4"),
    ("tahiti-dark", "#0e7490"),
    ("tahiti-200", "#a5f3fc"),
    ("tahiti-300", "#67e8f9"),
    ("tahiti-400", "#22d3ee"),
    ("tahiti-500", "#06b6d4"),
    ("tahiti-600", "#0891b2"),
    ("tahiti-700", "#0e7490"),
    ("tahiti-800", "#155e75"),
    ("tahiti-900", "#164e63"),
    ("tahiti-1000", "#0a222c"),
    // Surfaces — the page, its panels, and the two hairline steps.
    ("ink-950", "#081418"),
    ("ink-900", "#0e2028"),
    ("ink-800", "#143039"),
    ("ink-700", "#1d4049"),
    ("ink-600", "#2a5560"),
    // Text.
    ("mist-100", "#eaf1f3"),
    ("mist-300", "#b3c6cc"),
    ("mist-500", "#81979e"),
];

/// v3 applied `* { font-family: Roboto, Arial, serif }` by hand.
///
/// v3 also asked Google Fonts for *Roboto Serif* while writing
/// `font-family: Roboto, Arial, serif` — two different families, so the
/// downloaded font was never applied and no visitor ever saw it. v4 fixes that
/// by naming the family it actually ships; see `PLAN.md` §15.
///
/// **`FONT_SANS` is the body face, and it is a serif.** The name is Tailwind's,
/// not a description: `--font-sans` is what Tailwind's preflight reads for the
/// document default (`--default-font-family`), so whatever is named here
/// inherits everywhere, form controls included. Renaming the variable to
/// `--font-body` would mean overriding `--default-font-family` to match, for no
/// gain.
///
/// The three names below are hand-written string literals because `build.rs`
/// cannot see a `Font`, which is a compile-time value from the crate. The
/// duplication is checked rather than trusted: `family_names_match_the_palette`
/// in `src/ui/fonts.rs` asserts each one equals the family of the font actually
/// declared there — the exact drift that broke v3.
///
/// **Only weights the faces actually carry may be asked for.** Fraunces ships
/// 400 and 700 here, Literata 400 and 600, JetBrains Mono 400 alone, and no
/// family ships a slanted face — see [`crate::ui::fonts`]. Asking the display
/// face for a semibold would be answered by the browser's synthesiser rather
/// than drawn by the type designer, and asking for a slant would shear the
/// upright. Both are banned by
/// `every_weight_a_token_asks_for_is_a_face_that_ships`.
pub const FONT_SANS: &str = "Literata";
pub const FONT_DISPLAY: &str = "Fraunces";
pub const FONT_MONO: &str = "JetBrains Mono";

/// Corner radii, in the `--radius-*` namespace, so Tailwind emits one radius
/// utility per name below.
///
/// Depth in a dark theme is mostly this: without a radius and a border,
/// elevation has nothing to sit on. Tailwind's own radius steps are a little
/// tight for card-shaped content at these sizes.
pub const RADII: &[(&str, &str)] = &[("card", "0.75rem"), ("panel", "1.25rem")];

/// Shadows, in the `--shadow-*` namespace, so Tailwind emits one shadow utility
/// per name below.
///
/// Two layers each, and deliberately restrained: a tight contact shadow for the
/// edge plus a wide, low-alpha one for the lift. On a dark background a single
/// wide, soft shadow reads as a grey smudge, so the contact layer is what keeps the
/// element looking attached to the page.
///
/// Both use `rgb(0 0 0 / …)` rather than the palette, because a shadow is
/// absence of light and does not take the hue of the surface it falls on.
pub const SHADOWS: &[(&str, &str)] = &[
    (
        "card",
        "0 1px 2px rgb(0 0 0 / 0.30), 0 10px 30px -12px rgb(0 0 0 / 0.55)",
    ),
    (
        "lift",
        "0 2px 6px rgb(0 0 0 / 0.35), 0 20px 45px -18px rgb(0 0 0 / 0.65)",
    ),
];
