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
/// Setting `--font-sans` instead lets Tailwind's preflight inherit the same
/// stack everywhere, including form controls, without the `*` selector.
///
/// Note this is Tailwind's *default* `sans` stack, replaced. v3 also asked
/// Google Fonts for *Roboto Serif*, which this stack never referenced — see
/// PLAN.md §15.
pub const FONT_SANS: &str = "Roboto, Arial, serif";
