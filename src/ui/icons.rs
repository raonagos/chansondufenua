//! The chain marks on the support page — `/soutenir`.
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
    fn path_numbers(svg: &str) -> Vec<f32> {
        let mut numbers = Vec::new();

        for data in svg.split("d=\"").skip(1) {
            let data = data.split('"').next().unwrap();
            let mut current = String::new();
            for character in data.chars() {
                if character.is_ascii_digit() || character == '.' || character == '-' {
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
}
