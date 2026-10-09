//! The switcher's drawings: a globe, and one flag per language.
//!
//! Four shapes, drawn here as geometry rather than fetched: a 24×16 box for a
//! flag, a 20×20 one for the globe, inline in the served HTML of every page. No
//! external asset, no CDN, no copied brand file, no image pipeline — the rule
//! [`crate::ui::icons`] follows for the chain marks.
//!
//! # Why a drawing and not the flag emoji
//!
//! A regional-indicator pair (the flag a keyboard makes out of two letters) is
//! shorter to write and *not* the same thing: it is rendered by a font, and the
//! fonts that have the flags are not on every platform — the same page shows a
//! flag on one reader's machine and two letters on another's. A drawing renders
//! where the site does.
//!
//! # The colours are the flags' own
//!
//! Every drawing in [`crate::ui::icons`] takes `currentColor`, so a mark follows
//! whatever the theme is doing. A flag cannot: the whole of a flag is *which
//! colours it is*, and one repainted in the site's accent would be a different
//! flag. So the fills below are the flags', written once, and the palette is
//! deliberately not the place for them — `--color-*` names the design's colours,
//! and a flag is not one of them.
//!
//! # Decoration, and only ever decoration
//!
//! Every drawing is `aria-hidden="true"` and carries no words of its own: the
//! words that name a language are the switcher link's own, [`Lang::name`] in the
//! language's own spelling, and the control that opens the list is named from the
//! catalog. A reader who cannot see the flag still hears which language the link
//! leads to.
//!
//! # Why the markup is a string
//!
//! For [`crate::ui::icons`]' reason: this codebase has no cheap way to render a
//! view under `cargo test` — the layout's assets are read from disk — so the
//! drawings are constants the tests below can inspect, and `.run/step35.sh`
//! asserts that all four of them, and no fifth, reach the served HTML. The
//! strings are compile-time constants and nothing in them comes from a request,
//! which is what makes passing them through as raw markup safe.
//!
//! **The raw-string delimiters carry two hashes on purpose**: a flag's own markup
//! contains a colour attribute, and `fill="#…` ends a one-hash raw string.

use crate::i18n::Lang;

/// The flag a switcher link is: the whole of the link's picture.
///
/// One per language, and the match is exhaustive on purpose — a fourth language
/// cannot be added without drawing its flag.
pub fn svg(lang: Lang) -> &'static str {
    match lang {
        Lang::Fr => FRANCE,
        Lang::Ty => FRENCH_POLYNESIA,
        Lang::En => UNITED_KINGDOM,
    }
}

/// The globe on the control that opens the list.
///
/// The one drawing here that is not a flag: a circle, a meridian and a parallel
/// in `currentColor`, so it follows the header like every other piece of chrome,
/// and it says *language* where three flags could not — a closed list has to
/// carry the idea, not one of the choices.
pub const GLOBE: &str = r##"<svg viewBox="0 0 20 20" width="18" height="18" aria-hidden="true" fill="none" stroke="currentColor" stroke-width="1.5"><circle cx="10" cy="10" r="7.3"/><path d="M2.7 10h14.6"/><ellipse cx="10" cy="10" rx="3.1" ry="7.3"/></svg>"##;

/// France: the blue, white and red vertical bands, edge to edge.
const FRANCE: &str = r##"<svg viewBox="0 0 24 16" width="30" height="20" aria-hidden="true"><rect width="8" height="16" fill="#002395"/><rect x="8" width="8" height="16" fill="#ffffff"/><rect x="16" width="8" height="16" fill="#ed2939"/></svg>"##;

/// French Polynesia: the red, white and red bands, with the emblem's disc in the
/// middle of them — the pahi under sail, the sea below it, and the five stars of
/// the five archipelagos. The disc is what tells this flag from the red and white
/// bands it would otherwise share with another country's; the colours are the
/// emblem's own, and the disc stays inside the white band, where the artwork keeps
/// it.
///
/// Drawn for the size it is seen at rather than for the artwork it comes from. On
/// the page the disc is under seven pixels across, so the sea is one lens of blue,
/// the pahi is one orange sail over one dark hull line, and each of the five stars
/// is one dot: a traced star, a traced sail and a traced wave would be the same
/// pixels smeared, which is what "it renders ugly" was.
const FRENCH_POLYNESIA: &str = r##"<svg viewBox="0 0 24 16" width="30" height="20" aria-hidden="true"><rect width="24" height="5.333" fill="#ce1126"/><rect y="5.333" width="24" height="5.334" fill="#ffffff"/><rect y="10.667" width="24" height="5.333" fill="#ce1126"/><path d="M9.5 8.9c0.07 1.05 1.19 1.77 2.5 1.77s2.43 -0.72 2.5 -1.77z" fill="#083e9d"/><path d="M10.4 9.4h3.2" stroke="#630810" stroke-width="0.6"/><path d="M12 5.6 13.5 7.5h-3z" fill="#ff9d11" stroke="#ce1126" stroke-width="0.5"/><circle cx="10.2" cy="8.25" r="0.28" fill="#630810"/><circle cx="11.1" cy="8.25" r="0.28" fill="#630810"/><circle cx="12" cy="8.25" r="0.28" fill="#630810"/><circle cx="12.9" cy="8.25" r="0.28" fill="#630810"/><circle cx="13.8" cy="8.25" r="0.28" fill="#630810"/></svg>"##;

/// The United Kingdom: the blue field, the white saltire with the red one over
/// it, and the white cross with the red cross over that — the four strokes the
/// flag is made of, drawn in the order that draws it.
const UNITED_KINGDOM: &str = r##"<svg viewBox="0 0 24 16" width="30" height="20" aria-hidden="true"><rect width="24" height="16" fill="#012169"/><path d="M0 0 24 16M24 0 0 16" stroke="#ffffff" stroke-width="3.4"/><path d="M0 0 24 16M24 0 0 16" stroke="#c8102e" stroke-width="1.7"/><path d="M12 0v16M0 8h24" stroke="#ffffff" stroke-width="5.4"/><path d="M12 0v16M0 8h24" stroke="#c8102e" stroke-width="3.2"/></svg>"##;

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeSet;

    /// The box every flag is drawn in, and the size it renders at. One box and
    /// one size for all three: the switcher is a column of flags, and three
    /// different heights is the one thing a reader would notice.
    const BOX: &str = r##"viewBox="0 0 24 16" width="30" height="20""##;

    /// Three languages, three drawings: a copy-paste that left two of them
    /// sharing a shape fails here, and so does a flag drawn in another box than
    /// its neighbours'.
    #[test]
    fn every_language_has_a_flag_of_its_own() {
        let mut shapes = BTreeSet::new();

        for lang in Lang::ALL {
            let flag = svg(lang);

            assert!(flag.starts_with("<svg "), "{lang:?}: {flag}");
            assert!(flag.ends_with("</svg>"), "{lang:?}: {flag}");
            assert!(flag.contains(BOX), "{lang:?} is drawn in another box");
            assert!(flag.contains("aria-hidden=\"true\""), "{lang:?}");
            assert!(
                flag.matches('<').count() >= 4,
                "{lang:?} draws almost nothing"
            );
            assert!(
                shapes.insert(flag),
                "{lang:?} draws another language's flag"
            );
            assert_ne!(flag, GLOBE, "{lang:?} is drawn as the globe");
        }

        assert_eq!(shapes.len(), Lang::ALL.len());
    }

    /// Decoration carries no words and reaches nothing — [`crate::ui::icons`]'
    /// rule, for the same reasons: a flag with a `<title>` would be read twice, a
    /// referenced file would be a second request, and a class written here would
    /// be styling outside [`crate::ui::theme`].
    #[test]
    fn no_flag_carries_words_a_reference_or_a_class() {
        for lang in Lang::ALL {
            let flag = svg(lang);

            for banned in [
                "<title", "<text", "<use", "<image", "href", "xlink", "url(", "http", "class=",
            ] {
                assert!(
                    !flag.contains(banned),
                    "{lang:?} contains {banned:?}: {flag}"
                );
            }
        }
    }

    /// **A flag never follows the theme, and the globe always does.** A flag is
    /// the one drawing on this site whose colours are not the design's, so it is
    /// the one with no `currentColor` in it — and it has to paint something, or
    /// it is a hole in the header.
    #[test]
    fn a_flag_keeps_its_own_colours_and_the_globe_follows_the_theme() {
        for lang in Lang::ALL {
            let flag = svg(lang);

            assert!(
                !flag.contains("currentColor"),
                "{lang:?} takes the theme's colour"
            );
            assert!(
                flag.contains("fill=\"#") || flag.contains("stroke=\"#"),
                "{lang:?} paints nothing"
            );
        }

        assert!(
            GLOBE.contains("stroke=\"currentColor\""),
            "the globe does not follow the theme"
        );
        assert!(GLOBE.contains("aria-hidden=\"true\""), "{GLOBE}");
        assert!(
            GLOBE.starts_with("<svg ") && GLOBE.ends_with("</svg>"),
            "{GLOBE}"
        );
        assert!(GLOBE.contains("viewBox=\"0 0 20 20\""), "{GLOBE}");
    }

    /// Every number a flag's own geometry uses, from the attributes that place a
    /// shape and from `d` data.
    ///
    /// The `<svg>` element's own `width` and `height` are skipped: they are the
    /// rendered size (30×20 CSS pixels) of a drawing laid out in 24×16. Values
    /// that start with `#` are colours, not coordinates.
    fn geometry_numbers(flag: &str) -> Vec<f32> {
        let inner = &flag[flag.find('>').expect("an <svg> element") + 1..];
        let mut numbers = Vec::new();

        for (index, piece) in inner.split('"').enumerate() {
            // Attributes are the quoted pieces: `key="value"` alternates.
            if index % 2 == 0 || piece.starts_with('#') {
                continue;
            }

            let mut current = String::new();
            for character in piece.chars() {
                if character.is_ascii_digit() || character == '.' || character == '-' {
                    current.push(character);
                } else if !current.is_empty() {
                    numbers.push(current.parse().expect("a geometry number"));
                    current.clear();
                }
            }
            if !current.is_empty() {
                numbers.push(current.parse().expect("a geometry number"));
            }
        }

        numbers
    }

    /// Every drawing stays inside the box it declares. A coordinate past the box
    /// is the one way simple geometry goes wrong *silently* — the shape is cut
    /// off and only a browser would ever say so. This is
    /// `no_number_in_a_mark_is_larger_than_its_box`, for the flags.
    #[test]
    fn no_flag_draws_outside_its_box() {
        for lang in Lang::ALL {
            let numbers = geometry_numbers(svg(lang));

            assert!(!numbers.is_empty(), "{lang:?} draws nothing");
            for number in numbers {
                assert!(
                    number.abs() <= 24.0,
                    "{lang:?} draws {number}, larger than its 0..24 box"
                );
            }
        }
    }
}
