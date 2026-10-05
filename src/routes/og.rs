//! The social cards — `/drive/genog/…` and `/drive/gentw/…`.
//!
//! v3 had these two routes and rendered them with `headless_chrome`: it wrote
//! the lyric into a 1200×630 (Twitter: 1200×628) page, launched Chromium dark,
//! screenshotted it and answered with the PNG (`main:server/src/image.rs`).
//! That means a browser on the server — several hundred megabytes of one, plus a
//! process per card and a known way to hang — to draw black text on white.
//!
//! `PLAN.md` §7 named the three ways out (pure-Rust rendering, pre-generated
//! static cards, keep Chromium) and recommended the first. This is it: the card
//! is painted into an in-memory canvas by `fontdue` and encoded by `png`, both
//! pure Rust, and `/drive/…` needs nothing installed beside the binary.
//!
//! **What the card says is v3's, exactly: the lyric and nothing else.** No title,
//! no artist line, no site mark — v3 drew the lyric into a centred box and that
//! is what the cards in the wild look like. Whether a card *should* name the song
//! is a product question and not this step's; it is flagged in `PLAN.md` §23
//! rather than answered here.
//!
//! Three things differ from v3, all deliberate:
//!
//! * **The type fits the lyric.** v3 pinned the size at `2rem` and let
//!   `overflow: hidden` crop whatever did not fit, which a long song loses its
//!   middle to. Here the largest size between [`MAX_FONT_SIZE`] and
//!   [`MIN_FONT_SIZE`] that fits both the height and the widest line is chosen,
//!   so the common song is drawn whole. A song too long even for the floor is
//!   still centred and still cropped, the way v3 cropped it.
//! * **A stale version is a 404, not a 500.** The timestamp in the path is the
//!   song's own `updated_at` in microseconds, so that editing a song moves its
//!   card to a URL no cache has seen. v3 answered a mismatch with a 500 — a
//!   `map_err` on any failure at all. A 500 tells a crawler the server is broken;
//!   a 404 says the version it asked for does not exist, which is what is true.
//! * **The card is cacheable for a year.** The URL contains the version, so the
//!   bytes under it can never go stale, and v3 (which had a CDN in front) meant
//!   the same thing. `immutable` is the part that stops a browser revalidating.
//!
//! The font is the other thing this step had to decide, and it is worth writing
//! down because it looks like a random dependency. The site's own faces are
//! three Fontsource families served as `woff2`, which no pure-Rust rasteriser
//! reads, and a font fetched at *build* time would make the build depend on a
//! URL that can disappear. `epaint_default_fonts` is a data-only crate — the
//! four fonts an egui app falls back to, as `&'static [u8]` — and its
//! `UBUNTU_LIGHT` covers Latin Extended-A, which is what the corpus needs
//! (macrons and the ʻokina). See [`drawn`] for what happens to the handful of
//! characters it does not have.

use std::sync::OnceLock;

use bytes::Bytes;
use fontdue::{Font, FontSettings};
use topcoat::{
    Result,
    context::Cx,
    router::{
        Body,
        error::RouterErrorExt,
        header, path_param,
        response::{IntoResponse, Response},
        route,
    },
};

use crate::db;
use crate::domain::Song;
use crate::domain::song::LyricSpan;
use crate::state;

/// The Open Graph card's prefix, spelled the way v3's route and
/// [`Song::get_meta_data`]'s `og:image` both spell it.
pub const OG_PREFIX: &str = "/drive/genog";

/// The Twitter card's prefix — the same card, two pixels shorter, as in v3.
pub const TW_PREFIX: &str = "/drive/gentw";

/// What both routes answer with. `og:image:type` on the page says the same.
const PNG: &str = "image/png";

/// One year, and no revalidation: the URL names the version of the song.
const CACHE_CONTROL: &str = "public, max-age=31536000, immutable";

/// The white the card is painted on, and therefore the empty pixel.
const WHITE: u8 = 255;

/// The gap between the lyric and the edge of the card, in pixels.
///
/// v3 had no margin — its box was the viewport — but it also cropped, so the
/// margin only shows on a song that fits, where it is the difference between a
/// card and a wall of text.
const MARGIN: f32 = 48.0;

/// The largest type a card is drawn in, and the smallest it is shrunk to.
///
/// v3's `2rem` is 32 px, inside this range, which is why an average song looks
/// the size it used to.
const MAX_FONT_SIZE: f32 = 40.0;
const MIN_FONT_SIZE: f32 = 14.0;

/// Line height as a multiple of the type size.
///
/// v3's lyric box was `font-size: 2rem` with `padding: 4px 0` a line: 32 px type
/// in a ~46 px slot, a ratio of about 1.45. Kept, because it is what makes the
/// card read like the one it replaces.
const LINE_HEIGHT: f32 = 1.45;

// The `{timestamp}` and `{id}` of the two paths below.
path_param!(timestamp);
path_param!(id);

/// `GET /drive/genog/{timestamp}/himene/{id}` — the Open Graph card.
#[route(GET "/drive/genog/{timestamp}/himene/{id}")]
async fn open_graph(cx: &Cx) -> Result<Response> {
    serve(cx, Card::OpenGraph).await
}

/// `GET /drive/gentw/{timestamp}/himene/{id}` — the Twitter card.
#[route(GET "/drive/gentw/{timestamp}/himene/{id}")]
async fn twitter_card(cx: &Cx) -> Result<Response> {
    serve(cx, Card::Twitter).await
}

/// Both routes, once the path has been read.
///
/// Four things have to hold before a card is drawn, and each of them is a 404:
/// the id names a song, the song is published, the timestamp is the song's
/// current `updated_at`, and the timestamp is a number at all. Anything else is
/// answered exactly as `pages::song` answers it, so a card URL and a page URL for
/// the same missing song agree.
async fn serve(cx: &Cx, card: Card) -> Result<Response> {
    let id: &str = path_param::<Id>(cx);
    let claimed: i64 = path_param::<Timestamp>(cx).parse().ok().ok_or_not_found()?;

    let sheet = db::song(state::db(cx).pool(), id)
        .await?
        .filter(Song::is_published)
        .filter(|song| song.get_uat_timestamp() == claimed)
        .ok_or_not_found()?;

    (
        [
            (header::CONTENT_TYPE, header::HeaderValue::from_static(PNG)),
            (
                header::CACHE_CONTROL,
                header::HeaderValue::from_static(CACHE_CONTROL),
            ),
        ],
        Body::from(Bytes::from(card.render(&sheet))),
    )
        .into_response(cx)
}

/// Which of the two cards is being drawn.
///
/// The only difference is the height, and it is v3's: 630 px is the Open Graph
/// ratio, 628 px is X's, and v3 declared both to the width of a pixel.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Card {
    OpenGraph,
    Twitter,
}

impl Card {
    /// The viewport v3 screenshotted.
    fn size(self) -> (u32, u32) {
        match self {
            Self::OpenGraph => (1200, 630),
            Self::Twitter => (1200, 628),
        }
    }

    /// The card, as PNG bytes.
    fn render(self, sheet: &Song) -> Vec<u8> {
        let (width, height) = self.size();
        let lines = lyric_lines(sheet);
        let font = font();
        let size = fit(&lines, font, width, height);

        encode(&draw(&lines, font, width, height, size), width, height)
    }
}

/// The bundled face, parsed once for the process.
///
/// `expect` rather than a fallback: this is a `&'static [u8]` compiled into the
/// binary, so a parse failure is a broken build and not a request-time
/// condition. It is also the only way the typeface can be wrong, and a card that
/// quietly lost its text would be worse than a server that refused to start.
fn font() -> &'static Font {
    static FONT: OnceLock<Font> = OnceLock::new();

    FONT.get_or_init(|| {
        Font::from_bytes(epaint_default_fonts::UBUNTU_LIGHT, FontSettings::default())
            .expect("the bundled card font parses")
    })
}

/// The lyric as plain lines, chords dropped.
///
/// [`Song::lyrics_lines`] keeps the two apart precisely so a caller can have one
/// without the other; this is the card asking for the words. A verse break stays
/// on the card as an empty line, because that is where the space between verses
/// comes from.
///
/// **Whitespace is trimmed, the inside of a line is not touched.** The corpus
/// spaces chords against their syllable with runs of `&nbsp;`, and v3 drew those
/// gaps; collapsing them would move the words of a line relative to each other.
/// Only the ends are trimmed, where a chord at the end of a line leaves a gap
/// that nothing follows.
fn lyric_lines(sheet: &Song) -> Vec<String> {
    sheet
        .lyrics_lines()
        .iter()
        .map(|line| {
            let text: String = line
                .iter()
                .filter_map(|span| match span {
                    LyricSpan::Text(text) => Some(text.as_str()),
                    LyricSpan::Chord(_) => None,
                })
                .collect();

            text.trim().to_owned()
        })
        .collect()
}

/// The type size the lyric is drawn at: the largest that fits.
///
/// Both constraints matter. Height is the obvious one — a long song needs small
/// type. Width is the one v3 could not have: a browser wraps, and this does not,
/// so a single over-long line would otherwise run off the card. A song that fits
/// at neither the maximum nor anywhere above [`MIN_FONT_SIZE`] is drawn at the
/// floor and cropped, which is v3's behaviour and the honest one: no card can
/// show an arbitrary amount of text legibly.
fn fit(lines: &[String], font: &Font, width: u32, height: u32) -> f32 {
    let area_width = width as f32 - 2.0 * MARGIN;
    let area_height = height as f32 - 2.0 * MARGIN;

    let mut size = MAX_FONT_SIZE;
    while size > MIN_FONT_SIZE {
        let tallest = lines.len() as f32 * size * LINE_HEIGHT;
        let widest = lines
            .iter()
            .map(|line| text_width(line, font, size))
            .fold(0.0_f32, f32::max);

        if tallest <= area_height && widest <= area_width {
            return size;
        }

        size -= 1.0;
    }

    MIN_FONT_SIZE
}

/// How wide `line` is when drawn at `size`, pen advances only.
///
/// Kerning is left out on purpose: this is only ever compared against the card's
/// width with a margin's slack, and `fontdue`'s `horizontal_kern` is a per-pair
/// lookup that would cost more than the few pixels it would win.
fn text_width(line: &str, font: &Font, size: f32) -> f32 {
    line.chars()
        .filter_map(|character| drawn(character, font))
        .map(|character| font.metrics(character, size).advance_width)
        .sum()
}

/// The character actually drawn for `c`, or `None` if nothing can be.
///
/// The bundled face covers Latin and Latin Extended-A, which is the corpus's
/// alphabet — but a font is not a promise, and Tahitian is written with
/// characters a Latin-only face does not have. Dropping one out of a word would
/// corrupt it, so each gets a covered relative instead: a macron vowel loses its
/// macron, and the two turned commas — `ʻ` U+02BB, the ʻokina, and `ʼ` U+02BC —
/// become an apostrophe. `PLAN.md` §23 records which of them the bundled face
/// has as glyphs of its own, because that decides whether the substitution is
/// ever reached and the answer is a fact about the font, not about the site.
///
/// `None` is the last resort: a character with no glyph and no relative is left
/// out of the drawing, and [`text_width`] skips it in the same way, so the layout
/// stays truthful about what is on the card.
fn drawn(c: char, font: &Font) -> Option<char> {
    if font.has_glyph(c) {
        return Some(c);
    }

    let relative = match c {
        'ā' => 'a',
        'ē' => 'e',
        'ī' => 'i',
        'ō' => 'o',
        'ū' => 'u',
        'Ā' => 'A',
        'Ē' => 'E',
        'Ī' => 'I',
        'Ō' => 'O',
        'Ū' => 'U',
        '\u{02bb}' | '\u{02bc}' | '\u{2018}' | '\u{2019}' => '\'',
        _ => return None,
    };

    font.has_glyph(relative).then_some(relative)
}

/// The card's pixels: white, with the lines in black.
///
/// The canvas is greyscale, one byte a pixel, because the card has two colours
/// and v3's did too. A glyph is drawn by taking away ink: the face's coverage
/// becomes the pixel's darkness, so antialiasing is exactly `255 - coverage` on
/// a white ground. `min` rather than assignment where two glyphs overlap, since
/// black over black is still black.
///
/// The block is centred vertically as v3 centred it, `/himene/`'s lyric
/// included — which means a song that could not be fitted is cropped evenly at
/// the top and the bottom rather than being allowed to run off one end.
fn draw(lines: &[String], font: &Font, width: u32, height: u32, size: f32) -> Vec<u8> {
    let mut canvas = vec![WHITE; width as usize * height as usize];

    let line_height = size * LINE_HEIGHT;
    let top = (height as f32 - lines.len() as f32 * line_height) / 2.0;
    // The baseline sits one ascent below the top of its slot. A face without
    // horizontal line metrics is theoretical — `horizontal_line_metrics` returns
    // `None` only for a font with no `hhea`/`OS/2` at all — so the fallback is
    // the usual 0.8 em rather than an error path.
    let ascent = font
        .horizontal_line_metrics(size)
        .map(|metrics| metrics.ascent)
        .unwrap_or(size * 0.8);

    for (index, line) in lines.iter().enumerate() {
        let baseline = top + index as f32 * line_height + ascent;
        let mut pen = (width as f32 - text_width(line, font, size)) / 2.0;

        for character in line.chars().filter_map(|c| drawn(c, font)) {
            let (metrics, bitmap) = font.rasterize(character, size);
            let left = (pen + metrics.xmin as f32).round() as i32;
            // `fontdue` reports the glyph's box from the baseline upwards;
            // the canvas counts downwards from the top.
            let glyph_top = (baseline - metrics.ymin as f32 - metrics.height as f32).round() as i32;

            for row in 0..metrics.height {
                let y = glyph_top + row as i32;
                if y < 0 || y >= height as i32 {
                    continue;
                }

                for column in 0..metrics.width {
                    let x = left + column as i32;
                    if x < 0 || x >= width as i32 {
                        continue;
                    }

                    let coverage = bitmap[row * metrics.width + column];
                    let pixel = &mut canvas[y as usize * width as usize + x as usize];
                    *pixel = (*pixel).min(WHITE - coverage);
                }
            }

            pen += metrics.advance_width;
        }
    }

    canvas
}

/// A greyscale canvas as a PNG.
///
/// Both `expect`s are about the sink, not the encoder: the only error `png`
/// reports while writing is an IO one, and the sink here is a `Vec<u8>`. A
/// failure would mean the crate's contract changed, which is a build to fix
/// rather than a request to answer with a 500.
fn encode(canvas: &[u8], width: u32, height: u32) -> Vec<u8> {
    let mut png = Vec::new();

    {
        let mut encoder = png::Encoder::new(&mut png, width, height);
        encoder.set_color(png::ColorType::Grayscale);
        encoder.set_depth(png::BitDepth::Eight);

        let mut writer = encoder.write_header().expect("a PNG header for a Vec");
        writer
            .write_image_data(canvas)
            .expect("a PNG body for a Vec");
    }

    png
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::fixtures;
    use crate::domain::song::SITE_URL;

    fn sheet(index: usize) -> Song {
        let fixture = &fixtures::SONGS[index];
        Song::new(
            fixture.id.to_owned(),
            fixture.title.to_owned(),
            fixture.lyrics.to_owned(),
            7,
            Vec::new(),
            true,
            chrono::Utc::now(),
            chrono::Utc::now(),
        )
    }

    /// The prefixes here are the prefixes the page's own meta tags build. A
    /// drift between them is a card URL that resolves on the page and 404s at
    /// the server, which is the one failure this step exists to prevent — and
    /// the live check in `.run/step11.sh` fetches the URL out of the served
    /// `og:image` rather than spelling it out, for the same reason.
    #[test]
    fn the_meta_urls_name_the_routes_this_module_serves() {
        let meta = sheet(0).get_meta_data();

        let og = meta
            .meta_img_url_og
            .strip_prefix(SITE_URL)
            .expect("absolute");
        let tw = meta
            .meta_img_url_tw
            .strip_prefix(SITE_URL)
            .expect("absolute");

        assert!(og.starts_with(OG_PREFIX), "{og}");
        assert!(tw.starts_with(TW_PREFIX), "{tw}");
        // And each carries the song's own version, which is what `serve` checks.
        assert!(og.ends_with(&sheet(0).get_id()));
    }

    /// Every character of every fixture can be drawn, either as itself or as the
    /// relative [`drawn`] substitutes. This is the test that would fail if the
    /// bundled face were swapped for one that does not cover the corpus.
    #[test]
    fn every_character_of_the_corpus_can_be_drawn() {
        let font = font();

        for (index, fixture) in fixtures::SONGS.iter().enumerate() {
            for character in fixture.lyrics.chars().filter(|c| !c.is_control()) {
                assert!(
                    drawn(character, font).is_some(),
                    "fixture {index} uses {character:?}, which the card cannot draw"
                );
            }
        }
    }

    /// Chords are on the sheet, not on the card. v3 hid them with
    /// `sup { display: none }`; here they are skipped before anything measures.
    #[test]
    fn the_card_carries_the_words_and_not_the_chords() {
        let sheet = Song::new(
            "8nntgjk4rl5dbp67c6en".to_owned(),
            "Te here".to_owned(),
            "<div>Hina'a<sup data-nosnippet=\"true\">Eb</sup>ro</div>".to_owned(),
            1,
            Vec::new(),
            true,
            chrono::Utc::now(),
            chrono::Utc::now(),
        );

        assert_eq!(lyric_lines(&sheet), vec!["Hina'aro".to_owned()]);
    }

    /// The type shrinks for a longer lyric, and never past the floor — the two
    /// ends of `fit`'s range.
    #[test]
    fn a_longer_lyric_is_drawn_smaller() {
        let font = font();
        let short = vec!["Te here".to_owned()];
        let long: Vec<String> = (0..40).map(|i| format!("line {i}")).collect();
        let huge: Vec<String> = (0..400).map(|i| format!("line {i}")).collect();

        let (width, height) = Card::OpenGraph.size();
        let short_size = fit(&short, font, width, height);
        let long_size = fit(&long, font, width, height);

        assert_eq!(short_size, MAX_FONT_SIZE);
        assert!(long_size < short_size, "{long_size} < {short_size}");
        assert_eq!(fit(&huge, font, width, height), MIN_FONT_SIZE);
    }

    /// The card is not blank, and it is not black. An empty canvas would pass
    /// every dimension check in this file and still be a broken card.
    #[test]
    fn a_card_is_white_paper_with_ink_on_it() {
        let font = font();
        let lines = lyric_lines(&sheet(0));
        let (width, height) = Card::OpenGraph.size();
        let canvas = draw(
            &lines,
            font,
            width,
            height,
            fit(&lines, font, width, height),
        );

        assert_eq!(canvas.len(), width as usize * height as usize);

        let ink = canvas.iter().filter(|pixel| **pixel != WHITE).count();
        assert!(ink > 0, "nothing was drawn");

        let paper = canvas.iter().filter(|pixel| **pixel == WHITE).count();
        assert!(paper > ink, "the card is more ink than paper");
    }

    /// What `serve` hands the response: a PNG whose header says what the page's
    /// `og:image:width` / `og:image:height` claim, and that it is greyscale.
    #[test]
    fn the_card_is_a_png_of_the_size_it_claims() {
        for (card, (width, height)) in
            [(Card::OpenGraph, (1200, 630)), (Card::Twitter, (1200, 628))]
        {
            assert_eq!(card.size(), (width, height));

            let bytes = card.render(&sheet(0));
            let decoded = png::Decoder::new(std::io::Cursor::new(&bytes))
                .read_info()
                .expect("the card decodes as a PNG");

            assert_eq!(decoded.info().width, width);
            assert_eq!(decoded.info().height, height);
            assert_eq!(decoded.info().color_type, png::ColorType::Grayscale);
        }
    }
}
