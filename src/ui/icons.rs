//! The site's own drawings: the chain marks on the support page — `/tauturu` —
//! and the heart the footer's link to that page carries.
//!
//! Six marks, one per chain the site accepts support on, drawn here as geometry:
//! a 20×20 viewBox, one colour — `currentColor`, so a mark follows the theme the
//! page hands it — and nothing else. No external asset, no CDN, no copied brand
//! file, no image pipeline. These are the site's own drawings of shapes a reader
//! already knows: Bitcoin's barred B, Ethereum's diamond, Polygon's hexagon,
//! BNB's four rhombi, Avalanche's notched triangle, Solana's three slanted bars.
//! None of them claims to be a logo, and none is a load-bearing control.
//!
//! # Decoration, and only ever decoration
//!
//! A mark is drawn `aria-hidden="true"` and carries no text of its own: the
//! chain's name is printed in words by the page, from [`Mark::name`], in the same
//! card. `pages::support`'s tests fail if an entry grows a mark whose name its
//! own label does not contain, so an unlabelled shape cannot ship, and no mark
//! can be the only place a chain is named.
//!
//! # Drawings that are not chain marks
//!
//! Three of them, each drawn by the rules above and each beside a word that
//! carries its meaning, so none has to be read as a logo:
//!
//! [`DONATE`] is the footer's link to `/tauturu` — *support the site*, which the
//! reviewer asked for "hiding on an icon and the text `donate` … `don` … or
//! `tautururaa`". Its meaning is the catalog's word beside it
//! ([`Key::FooterDonate`](crate::i18n::Key::FooterDonate)), in the page's own
//! language, which is why a drawing here never has to be translated.
//!
//! [`SHARE`] and [`FACEBOOK`] are the song page's share row (step 42), which the
//! reviewer asked for: "a social share icon to share the song". The arrow is
//! what sharing a page looks like, and the `f` is the shape a reader knows the
//! network by — drawn here, not fetched, for the reason the chain marks are:
//! an inline `<svg>`, one colour, no request, no brand file. The word beside the
//! `f` is the network's own name (`crate::ui::share::NETWORK`), which is the
//! same word in every language.
//!
//! A word beside a drawing is the whole contract, so nothing here may be the
//! only place something is named.
//!
//! The two social drawings are **line drawings**: `fill="none"` and one
//! `stroke="currentColor"`. A heart is a solid shape and an arrow is a line, and
//! each is drawn the way it reads; the colour is still one value and still the
//! theme's, and a test below asserts that for all of them either way.
//!
//! # Why the markup is a string
//!
//! So it can be checked without rendering the page. This codebase has no cheap
//! way to render a view under `cargo test` — the layout's assets are read from
//! disk — so the tests below assert on the exact bytes that reach the browser,
//! and `.run/step30b.sh` asserts that the six of them, and no seventh, are in the
//! served HTML. The strings are compile-time constants and nothing in them comes
//! from a request, which is what makes passing them through as raw markup safe.

use std::fmt;

/// A chain the site accepts support on, as the mark drawn beside its address.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Mark {
    Bitcoin,
    Ethereum,
    Polygon,
    Bnb,
    Avalanche,
    Solana,
}

/// Every mark, for the exhaustiveness checks in tests.
pub const ALL: [Mark; 6] = [
    Mark::Bitcoin,
    Mark::Ethereum,
    Mark::Polygon,
    Mark::Bnb,
    Mark::Avalanche,
    Mark::Solana,
];

impl Mark {
    /// The chain's name in words — the text a mark is decoration *for*.
    ///
    /// These are the same words `pages::support`'s labels use (`Bitcoin`,
    /// `Solana`, and the four EVM names), which is what the page's own test
    /// compares them against. A proper name, never translated.
    pub fn name(self) -> &'static str {
        match self {
            Mark::Bitcoin => "Bitcoin",
            Mark::Ethereum => "Ethereum",
            Mark::Polygon => "Polygon",
            Mark::Bnb => "BNB Chain",
            Mark::Avalanche => "Avalanche",
            Mark::Solana => "Solana",
        }
    }

    /// The mark as one complete `<svg>` element, ready to sit in the page.
    pub fn svg(self) -> &'static str {
        match self {
            Mark::Bitcoin => BITCOIN,
            Mark::Ethereum => ETHEREUM,
            Mark::Polygon => POLYGON,
            Mark::Bnb => BNB,
            Mark::Avalanche => AVALANCHE,
            Mark::Solana => SOLANA,
        }
    }
}

impl fmt::Display for Mark {
    /// The name, so a mark can be named in a message without reaching for
    /// [`Mark::name`] by hand.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.name())
    }
}

/// The framed B of Bitcoin: a stem standing proud of its two lobes, top and
/// bottom, which is the whole difference between this shape and a plain B.
const BITCOIN: &str = r#"<svg viewBox="0 0 20 20" width="20" height="20" aria-hidden="true" fill="none" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round"><path d="M7.5 2v16"/><path d="M7.5 4.6h3.1a2.4 2.4 0 0 1 0 4.8H7.5"/><path d="M7.5 9.4h3.7a2.5 2.5 0 0 1 0 5H7.5"/></svg>"#;

/// Ethereum's diamond: the solid upper half and the thin lower wedge below it.
const ETHEREUM: &str = r#"<svg viewBox="0 0 20 20" width="20" height="20" aria-hidden="true" fill="currentColor"><path d="M10 1.6 15.9 10.1 10 13.6 4.1 10.1Z"/><path d="M10 15.3 15.9 11.8 10 18.4 4.1 11.8Z"/></svg>"#;

/// Polygon's hexagon, drawn rather than filled so it cannot be mistaken for a
/// diamond at a glance.
const POLYGON: &str = r#"<svg viewBox="0 0 20 20" width="20" height="20" aria-hidden="true" fill="none" stroke="currentColor" stroke-width="1.6" stroke-linejoin="round"><path d="M10 2.2 16.8 6.1v7.8L10 17.8 3.2 13.9V6.1Z"/></svg>"#;

/// BNB Chain's four rhombi, in the two-by-two the chain's own mark arranges.
const BNB: &str = r#"<svg viewBox="0 0 20 20" width="20" height="20" aria-hidden="true" fill="currentColor"><path d="M6.6 3.2 9.2 5.8 6.6 8.4 4 5.8Z"/><path d="M13.4 3.2 16 5.8 13.4 8.4 10.8 5.8Z"/><path d="M6.6 11.4 9.2 14 6.6 16.6 4 14Z"/><path d="M13.4 11.4 16 14 13.4 16.6 10.8 14Z"/></svg>"#;

/// Avalanche's triangle, with the notch that keeps it from reading as any
/// triangle at all.
const AVALANCHE: &str = r#"<svg viewBox="0 0 20 20" width="20" height="20" aria-hidden="true" fill="currentColor"><path d="M10 2.4 17.6 16.6H12.9L10 11.4 7.1 16.6H2.4Z"/></svg>"#;

/// Solana's three slanted bars, cut the way the chain's mark cuts them.
const SOLANA: &str = r#"<svg viewBox="0 0 20 20" width="20" height="20" aria-hidden="true" fill="currentColor"><path d="M6.6 4.2h9.4l-2.6 2.3H4Z"/><path d="M4 9h9.4l-2.6 2.3H1.4Z"/><path d="M6.6 13.8h9.4l-2.6 2.3H4Z"/></svg>"#;

/// The heart the footer's link to the support page is drawn with.
///
/// A heart is the one shape a reader already reads as *give*, and it is here
/// rather than in a page module because it is a drawing and not a page: the same
/// box the marks use, the same `currentColor` that follows the theme, the same
/// `aria-hidden`, and nothing in it a reader could be asked to translate.
///
/// Two semicircles and one point: each lobe is one arc on a circle a radius from
/// the middle, and the two sides are straight lines that meet at the point
/// below them, so the whole shape is four numbers and nothing is free-hand. It
/// is a little smaller than a mark, because the word it sits beside is
/// footer-sized text rather than a card's heading; the viewBox is the marks'
/// own and the size attributes scale it.
pub const DONATE: &str = r#"<svg viewBox="0 0 20 20" width="16" height="16" aria-hidden="true" fill="currentColor"><path d="M10 17.2 3.6 8a3.2 3.2 0 0 1 6.4 0 3.2 3.2 0 0 1 6.4 0Z"/></svg>"#;

/// The arrow the share row's first link is drawn with: a page leaving a box.
///
/// A line drawing rather than a filled one, because that is the shape sharing a
/// page is: an arrow up out of an open tray. Three strokes — the shaft, the
/// head, and the tray's own three sides — so the tray reads as open at the top
/// and the arrow as coming out of it. The corners are drawn as cubics rather
/// than as arcs, so every number below is one the module's box test can read.
///
/// The size, the box and the colour are [`DONATE`]'s: it sits beside a word at
/// the same text size, in the same row shape.
pub const SHARE: &str = r#"<svg viewBox="0 0 20 20" width="16" height="16" aria-hidden="true" fill="none" stroke="currentColor" stroke-width="1.7" stroke-linecap="round" stroke-linejoin="round"><path d="M10 3V12.6"/><path d="M6.4 6.6L10 3l3.6 3.6"/><path d="M4.8 10.8v3.8c0 1.5 1.2 2.8 2.8 2.8h4.8c1.6 0 2.8-1.3 2.8-2.8v-3.8"/></svg>"#;

/// The letterform the share row's second link is drawn with: an `f`.
///
/// The shape a reader knows the network by, drawn as geometry and not copied
/// from anyone: a stem, a crossbar, and the ascender's hook bending right at the
/// top — which is what tells an `f` from a `t`, and what the module's own
/// rasterised check of this path was looking at. The word beside it is the
/// network's name; a shape on its own would claim to be a logo, and nothing in
/// this module does.
pub const FACEBOOK: &str = r#"<svg viewBox="0 0 20 20" width="16" height="16" aria-hidden="true" fill="none" stroke="currentColor" stroke-width="1.7" stroke-linecap="round" stroke-linejoin="round"><path d="M8.2 17.4V7.6c0-3 1.8-4.4 4.2-4.4h1.4"/><path d="M5.4 10.2h6.2"/></svg>"#;

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeSet;

    /// Every mark is decoration that draws geometry and nothing else: a closed
    /// `<svg>` of the agreed box, hidden from assistive technology because the
    /// page prints its name in words beside it, one colour that follows the
    /// theme, and at least one shape.
    #[test]
    fn every_mark_is_hidden_geometry_in_the_theme_colour() {
        for mark in ALL {
            let svg = mark.svg();

            assert!(svg.starts_with("<svg "), "{mark}: {svg}");
            assert!(svg.ends_with("</svg>"), "{mark}: {svg}");
            assert!(svg.contains("viewBox=\"0 0 20 20\""), "{mark}");
            assert!(svg.contains("aria-hidden=\"true\""), "{mark}");
            assert!(
                svg.contains("fill=\"currentColor\"") || svg.contains("stroke=\"currentColor\""),
                "{mark} does not follow the theme's colour"
            );
            assert!(svg.matches("<path ").count() >= 1, "{mark} draws nothing");
        }
    }

    /// Decoration carries no words and reaches nothing: no `<title>` or `<text>`
    /// for a reader to have to ignore, no second source to fetch, no referenced
    /// file, no copied brand asset, and no class — the page's own token is what
    /// styles the row.
    #[test]
    fn no_mark_carries_words_a_reference_or_a_class() {
        for mark in ALL {
            let svg = mark.svg();
            for banned in [
                "<title", "<text", "<use", "<image", "href", "xlink", "url(", "http", "class=",
            ] {
                assert!(!svg.contains(banned), "{mark} contains {banned:?}: {svg}");
            }
        }
    }

    /// The numbers in every `d` attribute of a mark, in the order they appear.
    ///
    /// The separator is `" d=\""` and not `"d=\""`: the shorter one also matches
    /// the `d` of `aria-hidden="true"`, and of a stroke drawing's own
    /// `stroke-width="1.7"` — which is how this helper first read *true* as a
    /// path number (step 42, the share row's two stroke drawings).
    fn path_numbers(svg: &str) -> Vec<f32> {
        let mut numbers = Vec::new();

        for data in svg.split(" d=\"").skip(1) {
            let data = data.split('"').next().unwrap();
            let mut current = String::new();
            for character in data.chars() {
                if character.is_ascii_digit() || character == '.' {
                    current.push(character);
                } else if character == '-' && !current.is_empty() {
                    // A `-` between two numbers is the next number's sign and
                    // not part of this one: paths are written without spaces
                    // where they can be (`c0-3 1.8-4.4`), which is legal SVG and
                    // is how the share row's two drawings are written.
                    numbers.push(current.parse().expect("a path number"));
                    current.clear();
                    current.push(character);
                } else if character == '-' {
                    current.push(character);
                } else if !current.is_empty() {
                    numbers.push(current.parse().expect("a path number"));
                    current.clear();
                }
            }
            if !current.is_empty() {
                numbers.push(current.parse().expect("a path number"));
            }
        }

        numbers
    }

    /// Every mark stays inside the 20×20 it declares. A coordinate past the box
    /// is the one way simple geometry goes wrong *silently* — the shape is cut
    /// off and only a browser would ever say so. This reads the numbers, so it
    /// cannot follow a path (a relative step is negative by design, and a short
    /// walk out of the box is not what it is looking for): what it catches is a
    /// decimal point in the wrong place, which is the mistake that gets made.
    #[test]
    fn no_number_in_a_mark_is_larger_than_its_box() {
        for mark in ALL {
            let numbers = path_numbers(mark.svg());

            assert!(!numbers.is_empty(), "{mark} draws nothing");
            for number in numbers {
                assert!(
                    number.abs() <= 20.0,
                    "{mark} draws {number}, larger than its 0..20 box"
                );
            }
        }
    }

    /// Six marks, six drawings, six names: a copy-paste that left two chains
    /// sharing a shape, or two marks sharing a name, fails here.
    #[test]
    fn the_marks_are_six_different_shapes_with_six_different_names() {
        let mut shapes = BTreeSet::new();
        let mut names = BTreeSet::new();

        for mark in ALL {
            assert!(
                shapes.insert(mark.svg()),
                "{mark} draws another chain's mark"
            );
            assert!(names.insert(mark.name()), "two chains are called {mark}");
        }

        assert_eq!(shapes.len(), ALL.len());
        assert_eq!(names.len(), ALL.len());
    }

    /// The names are the page's own words, spelled as its labels spell them —
    /// `BNB Chain` with its Chain, which is the one a paraphrase would drop.
    #[test]
    fn the_names_are_the_ones_the_support_page_labels() {
        assert_eq!(
            ALL.map(Mark::name),
            [
                "Bitcoin",
                "Ethereum",
                "Polygon",
                "BNB Chain",
                "Avalanche",
                "Solana"
            ]
        );
    }

    /// The footer's heart is the same kind of drawing a mark is, by the same
    /// rules: a closed `<svg>` of the agreed box, hidden from assistive
    /// technology, one colour that follows the theme, geometry and nothing else
    /// — no words of its own, no referenced file, no class — and every number in
    /// it inside the box it declares.
    ///
    /// It is also not a mark: a footer that drew a shape one of the six chain
    /// cards draws would be a drawing of a coin with nothing saying which — the
    /// one thing this module's contract forbids.
    #[test]
    fn the_footer_heart_is_a_decoration_the_way_a_mark_is() {
        assert!(DONATE.starts_with("<svg "), "{DONATE}");
        assert!(DONATE.ends_with("</svg>"), "{DONATE}");
        assert!(DONATE.contains("viewBox=\"0 0 20 20\""), "{DONATE}");
        assert!(DONATE.contains("aria-hidden=\"true\""), "{DONATE}");
        assert!(DONATE.contains("fill=\"currentColor\""), "{DONATE}");

        for banned in [
            "<title", "<text", "<use", "<image", "href", "xlink", "url(", "http", "class=",
        ] {
            assert!(!DONATE.contains(banned), "the heart contains {banned:?}");
        }

        let numbers = path_numbers(DONATE);
        assert!(!numbers.is_empty(), "the heart draws nothing");
        for number in numbers {
            assert!(
                number.abs() <= 20.0,
                "the heart draws {number}, larger than its 0..20 box"
            );
        }

        for mark in ALL {
            assert_ne!(DONATE, mark.svg(), "the heart is {mark}'s own shape");
        }
    }

    /// The share row's two drawings (step 42) are decorations by the same rules,
    /// drawn the other way: **a line, not a fill**.
    ///
    /// An arrow and an `f` are strokes, and the difference is deliberate — a
    /// heart is a solid shape and an arrow is a line, and each is drawn the way
    /// it reads. What does not change is the contract: the box, one colour from
    /// the theme and no other, `aria-hidden`, no words of their own, nothing to
    /// fetch, no class, and every number inside the box.
    #[test]
    fn the_share_row_drawings_are_lines_in_the_theme_colour() {
        for (label, svg) in [("share", SHARE), ("facebook", FACEBOOK)] {
            assert!(svg.starts_with("<svg "), "{label}: {svg}");
            assert!(svg.ends_with("</svg>"), "{label}: {svg}");
            assert!(svg.contains("viewBox=\"0 0 20 20\""), "{label}: {svg}");
            assert!(svg.contains("aria-hidden=\"true\""), "{label}: {svg}");
            assert!(svg.contains("fill=\"none\""), "{label} is filled: {svg}");
            assert!(
                svg.contains("stroke=\"currentColor\""),
                "{label} does not follow the theme's colour: {svg}"
            );
            assert!(
                !svg.contains("fill=\"currentColor\""),
                "{label} is drawn twice: {svg}"
            );

            for banned in [
                "<title", "<text", "<use", "<image", "href", "xlink", "url(", "http", "class=",
            ] {
                assert!(!svg.contains(banned), "{label} contains {banned:?}: {svg}");
            }

            let numbers = path_numbers(svg);
            assert!(!numbers.is_empty(), "{label} draws nothing");
            for number in numbers {
                assert!(
                    number.abs() <= 20.0,
                    "{label} draws {number}, larger than its 0..20 box"
                );
            }

            for mark in ALL {
                assert_ne!(svg, mark.svg(), "{label} draws {mark}'s own shape");
            }
            assert_ne!(svg, DONATE, "{label} draws the footer's heart");
        }

        // Two links, two shapes: a copy-paste that left the arrow in the `f`
        // would be a row of two of the same drawing.
        assert_ne!(SHARE, FACEBOOK, "both links are drawn the same way");
    }
}
