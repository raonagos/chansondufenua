//! Web fonts, declared in Rust and served from this binary.
//!
//! Three families, each with one job, and every one of them chosen against a
//! constraint v3 could not have known about because it never rendered a macron
//! in a font it actually loaded.
//!
//! # The macron constraint
//!
//! Tahitian orthography marks long vowels with a macron — `ā ē ī ō ū`. The real
//! corpus carries **323** of them (`ā` ×164, `ō` ×102, `ē` ×24, `ū` ×12,
//! `ī` ×10, `Ā` ×10, `Ē` ×1). These are *precomposed* codepoints in Latin
//! Extended-A (U+0100–U+017F), not base letters plus a combining mark, so a font
//! subset that omits that block cannot draw them.
//!
//! Fontsource splits faces by character subset, and the split is a trap in both
//! directions — verified against the vendored catalog in `topcoat-font`, not
//! assumed:
//!
//! * **`Latin` alone does not carry macrons.** Its range is
//!   `U+0000-00FF, U+0131, U+0152-0153, …` — it includes the *combining* macron
//!   U+0304 but no precomposed `ā`. The browser would fall back to another font
//!   mid-word.
//! * **`LatinExt` alone does not carry plain ASCII.** Its range is
//!   `U+0100-024F` and friends — no `A`, no `z`, no `’`.
//!
//! So every face here declares **both** subsets. Declaring only the family
//! default would silently drop macrons; declaring only the extension would
//! silently drop the alphabet. `unicode-range` keeps the cost honest: the
//! browser fetches the extension file only on a page that actually contains a
//! macron.
//!
//! # Self-hosted
//!
//! `host: Asset` bundles the `.woff2` files and serves them from this process,
//! so there is no request to a third party on page load, no dependency on a CDN
//! reachable from Tahiti, and no `fonts.googleapis.com` round trip to leak a
//! visitor's IP. The font *stylesheet* is served at `/_topcoat/fonts/…` with a
//! content-hashed, immutable `Cache-Control`.
//!
//! Registration is automatic: `fontsource_font!` files each constant in an
//! `inventory`, and `RouterBuilderDiscoverExt::discover` calls `discover_fonts`,
//! so these constants need no wiring in [`crate::router`].
//!
//! # Which family for what
//!
//! * [`LITERATA`] — body and lyrics. Commissioned for long-form reading, with
//!   diacritics drawn by a type designer rather than synthesised. It is what
//!   `--font-sans` resolves to, so Tailwind's preflight inherits it everywhere.
//! * [`FRAUNCES`] — display: page titles, song titles, section headings. An
//!   old-style serif with enough warmth to keep a cyan gradient from reading as
//!   a dashboard.
//! * [`JETBRAINS_MONO`] — chord superscripts and small technical labels, where
//!   a monospace keeps the labels from colliding with the lyric they sit over.
//!
//! The names that reach CSS live in [`crate::ui::palette`], because `build.rs`
//! needs them as literals and cannot see a `Font`. That duplication is checked,
//! not trusted — see `family_names_match_the_palette` below.

use topcoat::font::{Font, fontsource::fontsource_font};

/// Body text and lyrics. Naming the family in the palette as `FONT_SANS` is
/// historical: it is what Tailwind's `--font-sans` resolves to, which is the
/// variable preflight reads, regardless of the face being a serif.
pub const LITERATA: Font = fontsource_font!(
    LITERATA,
    weight: [400, 600],
    style: Normal,
    subset: [Latin, LatinExt],
    host: Asset,
);

/// Display face for titles and headings.
pub const FRAUNCES: Font = fontsource_font!(
    FRAUNCES,
    weight: [400, 700],
    style: Normal,
    subset: [Latin, LatinExt],
    host: Asset,
);

/// Chord labels and small technical text. One weight: the labels are small and
/// always the same emphasis.
pub const JETBRAINS_MONO: Font = fontsource_font!(
    JETBRAINS_MONO,
    weight: 400,
    style: Normal,
    subset: [Latin, LatinExt],
    host: Asset,
);

/// Every font this application ships.
pub const ALL: &[Font] = &[LITERATA, FRAUNCES, JETBRAINS_MONO];

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ui::palette;
    use topcoat::font::FontSource;

    /// The opening interval of latin-ext's vendored `unicode-range`. All three
    /// families resolve `latin-ext` to the same range, so one constant is
    /// enough; `U+0100` is `Ā`, the first macron vowel.
    const LATIN_EXT: &str = "U+0100-02BA";

    /// The opening interval of latin's vendored `unicode-range` — basic Latin,
    /// which the extension subset does *not* include.
    const LATIN: &str = "U+0000-00FF";

    fn range_of(face: &topcoat::font::FontFace) -> String {
        face.unicode_range()
            .expect("every face here selects a subset, so every face has a unicode-range")
            .to_string()
    }

    /// The family names in `palette.rs` are written by hand because `build.rs`
    /// cannot see a `Font`, and a drift between the two is invisible: the
    /// `@font-face` rules would name one family and every `font-family` rule
    /// would name another, so the font would download and never be used. That
    /// is precisely v3's bug — which asked for "Roboto Serif" and wrote
    /// `font-family: Roboto, Arial, serif` — so it is worth a test.
    #[test]
    fn family_names_match_the_palette() {
        assert_eq!(LITERATA.family(), palette::FONT_SANS);
        assert_eq!(FRAUNCES.family(), palette::FONT_DISPLAY);
        assert_eq!(JETBRAINS_MONO.family(), palette::FONT_MONO);
    }

    /// Both subsets must be present *across* a family's faces.
    ///
    /// Across, not within: a face carries exactly one subset, so checking each
    /// face for both would demand a merged range the catalog never produces.
    /// The pair are complementary and neither is a superset of the other —
    /// `latin` is `U+0000-00FF` and up, `latin-ext` starts at `U+0100` — so a
    /// family declaring only one of them renders half its text in a fallback
    /// face, and the half it drops is either every alphabet letter or every
    /// macron.
    #[test]
    fn every_family_covers_both_latin_ranges_between_its_faces() {
        for font in ALL {
            let ranges: Vec<String> = font.faces().iter().map(range_of).collect();

            assert!(
                ranges.iter().any(|range| range.contains(LATIN)),
                "{} has no basic-Latin face: {ranges:?}",
                font.family()
            );
            assert!(
                ranges.iter().any(|range| range.contains(LATIN_EXT)),
                "{} has no Latin Extended-A face, so macrons would fall back: {ranges:?}",
                font.family()
            );

            // The two subsets are separate files, not one merged range. If a
            // face ever carried both, `subset:` had stopped splitting and the
            // browser would be downloading the extension on every page.
            assert!(
                !ranges
                    .iter()
                    .any(|range| range.contains(LATIN) && range.contains(LATIN_EXT)),
                "{} merged its subsets into a single face: {ranges:?}",
                font.family()
            );
        }
    }

    /// Two weights for the body face (regular, semibold), two for display
    /// (regular, bold), one for mono, and a base plus an extension file for
    /// each — the arithmetic that makes `unicode-range` worth having.
    #[test]
    fn faces_are_the_expected_cross_product() {
        // weights × subsets, spelled out rather than multiplied: `1 * 2` is an
        // identity operation, and a number plus a message says which half was
        // dropped when it fails.
        assert_eq!(
            LITERATA.faces().len(),
            4,
            "regular + semibold, each in latin and latin-ext"
        );
        assert_eq!(
            FRAUNCES.faces().len(),
            4,
            "regular + bold, each in latin and latin-ext"
        );
        assert_eq!(
            JETBRAINS_MONO.faces().len(),
            2,
            "one weight, in latin and latin-ext"
        );
    }

    /// Nothing is fetched from a third party at page load. An `Asset` source
    /// resolves `as_str()` to `None`; a `str` source is the jsDelivr URL that
    /// `host: Asset` exists to avoid.
    #[test]
    fn no_face_points_at_a_cdn() {
        for font in ALL {
            for face in font.faces().iter() {
                for source in face.src().iter() {
                    match source {
                        FontSource::Url { url, .. } => assert!(
                            url.is_asset(),
                            "{} is not self-hosted: {url:?}",
                            font.family()
                        ),
                        FontSource::Local { name } => {
                            panic!("{} ships a local() source: {name}", font.family())
                        }
                    }
                }
            }
        }
    }
}
