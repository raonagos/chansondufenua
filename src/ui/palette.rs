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

/// v3's palette, transcribed from the `@theme` block in `style/tailwind.scss`.
///
/// The hex values are v3's, byte for byte, so v4 renders the same colours.
/// They are NOT Tailwind's built-in `cyan-*`, which shares the hue and lightness
/// but carries higher chroma (cyan-700 is oklch 52% 0.105 223.1 against
/// `#0e7490`'s oklch 52.0% 0.094 223.1). Aliasing to `cyan-*` would silently
/// repaint the site, and `tahiti-1000` has no built-in equivalent at all.
///
/// The three unnumbered entries (`tahiti`, `tahiti-light`, `tahiti-dark`) are
/// v3's aliases, kept so the names in the original stylesheet remain valid.
pub const PALETTE: &[(&str, &str)] = &[
    ("tahiti-light", "#22d3ee"),
    ("tahiti", "#06b6d4"),
    ("tahiti-dark", "#0e7490"),
    ("tahiti-400", "#22d3ee"),
    ("tahiti-500", "#06b6d4"),
    ("tahiti-600", "#0891b2"),
    ("tahiti-700", "#0e7490"),
    ("tahiti-800", "#155e75"),
    ("tahiti-900", "#164e63"),
    ("tahiti-1000", "#0a222c"),
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
pub const FONT_SANS: &str = "Literata";
pub const FONT_DISPLAY: &str = "Fraunces";
pub const FONT_MONO: &str = "JetBrains Mono";

/// Corner radii, in the `--radius-*` namespace, so Tailwind emits
/// `rounded-card` and `rounded-panel`.
///
/// Depth in a dark theme is mostly this: without a radius and a border,
/// elevation has nothing to sit on. Tailwind's defaults (`rounded-md` and up)
/// are a little tight for card-shaped content at these sizes.
pub const RADII: &[(&str, &str)] = &[("card", "0.75rem"), ("panel", "1.25rem")];

/// Shadows, in the `--shadow-*` namespace, so Tailwind emits `shadow-card` and
/// `shadow-lift`.
///
/// Two layers each, and deliberately restrained: a tight contact shadow for the
/// edge plus a wide, low-alpha one for the lift. On a dark background a single
/// large blur reads as a grey smudge, so the contact layer is what keeps the
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
