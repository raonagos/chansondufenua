//! The song sheet — `/himene/{slug}`.
//!
//! Transcribed from v3's `SongPage` (`app/src/pages/himene/song.rs`): the title,
//! a link to the create-song page, and the lyric with its chords. Four things
//! differ, each deliberate:
//!
//! * **The lyric is rendered from data, not injected as HTML.** v3 dropped
//!   `inner_html` straight into the template, so the only way to place a chord
//!   was a CSS rule matching a `<sup>`. v4 parses the same sanitised HTML into
//!   lines of text and chord spans ([`Song::lyrics_lines`]), then hangs each
//!   chord from the syllable it was written against (`chunks`) and renders the
//!   result with the site's own tokens.
//! * **The credits are shown.** v3 carried the artists only inside the page's
//!   metadata. The index lists them, so a reader arriving from the index has
//!   already seen them; one arriving from a search result never did.
//! * **A missing or unpublished song is a 404, not an empty page.** v3 asked the
//!   browser for the song and rendered nothing at all when the request failed.
//! * **A served sheet counts as a view**, where v3 incremented the count inside
//!   its own read function, before it knew whether there was a song to read.
//! * **The URL is the slug**, and this page is reached by one: the negotiation
//!   layer answers `/himene/{id}` and a retired slug with a `301` before routing
//!   has a say, so what arrives here is already the canonical address. The page
//!   still resolves the segment rather than trusting that, because "trusting
//!   that" is how a page ends up 404ing when a layer is reordered.
//!
//! The page's `<head>` is not here. Topcoat 0.10 has no per-page head API, so
//! the layout decides it from the request path and the same row — see
//! `src/ui/layout.rs`.
//!
//! Note the naming constraint that every page in this directory shares:
//! `#[page("/himene/{slug}")]` emits a unit struct named after its handler, in
//! this module's *type* namespace, so a local binding called `song` would be
//! read as a pattern matching it.

use topcoat::{
    Result,
    context::Cx,
    router::{error::RouterErrorExt, page, path_param, request::uri},
    view::{Unescaped, View, class, component, view},
};

use crate::db;
use crate::domain::chord;
use crate::domain::song::{LyricLine, LyricSpan, Song};
use crate::i18n::{self, Key};
use crate::pages::artist;
use crate::state;
use crate::ui::autoscroll::{self, speed_bar};
use crate::ui::share::row as share_row;
use crate::ui::theme;

use std::collections::VecDeque;

// The `{slug}` in this page's path.
//
// Public because the index and the home page build their links with `href!`,
// which fills a route's parameters by naming the type this declares. That is the
// point of declaring it at all: the URL shape lives in one place, in the
// `#[page]` attribute below, and no other module spells it out. What goes in it
// is [`Song::get_segment`] — the slug, or the id for a song that has none.
path_param!(pub slug);

/// `/himene/{slug}` — one song, with its chords.
///
/// **Unpublished is not found.** [`db::song_at`] deliberately returns drafts as
/// well, so that the editor can see them; this page is the public one and asks
/// whether the song is published before it renders anything.
#[page("/himene/{slug}")]
pub async fn song(cx: &Cx) -> Result<impl View> {
    let segment: &str = path_param::<Slug>(cx);

    let sheet = db::song_at(state::db(cx).pool(), segment)
        .await?
        .filter(|found| found.song().is_published())
        .map(db::Addressed::into_song)
        .ok_or_not_found()?;

    // Serving the sheet is what counts as a view, which is v3's rule: its
    // `fn::get_song_fetch_artist` opened with `UPDATE song SET view_count += 1`.
    // The v4 difference is the order — v3 incremented *before* it read, so a
    // request for an id that did not exist incremented nothing and a request for
    // a draft still counted. Here only a sheet that is about to be served counts,
    // which is what the home page's most-viewed table means by a view.
    //
    // The count is keyed by the song's **id**, not by the segment that arrived:
    // the address is the slug and the identifier is the id, and this write is
    // about the row.
    //
    // Awaited rather than detached: this is an in-process write to a database
    // compiled into the binary, and a response that says "200" should mean the
    // view it reports was recorded.
    db::increment_view_count(state::db(cx).pool(), &sheet.get_id()).await?;

    Ok(view! { sheet_body(sheet: sheet) })
}

/// The sheet itself: a header, then the lyric.
///
/// A `#[component]` rather than a plain helper, because `view!` needs the
/// request context and only a component or a page binds one.
#[component]
pub async fn sheet_body(cx: &Cx, sheet: Song) -> Result<impl View> {
    let lang = i18n::resolve(cx);
    let title = sheet.get_title();
    // Each credit is a link to its own page (v4.1 step 29), the same address a
    // search result uses: an artist's name is how a reader finds the rest of
    // what they sang, and it is how a crawler finds the artist page at all.
    let artists = sheet
        .get_artists()
        .iter()
        .map(|artist| (artist::link(cx, artist), artist.get_fullname()))
        .collect::<Vec<(String, String)>>();

    // `?tr=` — the reader's own key. Read from the request and applied to the
    // stored chords, so a step is a function of the sheet and the offset and
    // never of the step before it: at zero the author's spelling comes back
    // untouched, which is what makes stepping away and back land on the page the
    // reader opened. The root is then read in the page's language and the
    // accidental with the quality ride in an `<i>`; see `crate::domain::chord`.
    let offset = chord::offset(uri(cx).query().unwrap_or(""));
    let lines = sheet
        .lyrics_lines()
        .into_iter()
        .map(|line| chord::localised_line(line, offset, lang))
        .collect::<Vec<_>>();
    let transpose = i18n::text(lang, Key::Transpose);
    let step = if offset == 0 {
        "0".to_owned()
    } else {
        format!("{offset:+}")
    };

    Ok(view! {
        <article class=(theme::SONG_SHEET)>
            <header class=(theme::SONG_HEAD)>
                <div class=(theme::SONG_HEADING)>
                    <h1 class=(theme::SONG_TITLE)>(title)</h1>
                    // An uncredited song renders no line at all, rather than an
                    // empty paragraph with a margin under it.
                    if !artists.is_empty() {
                        <p class=(theme::SONG_ARTISTS)>
                            for (index, (url, name)) in artists.into_iter().enumerate() {
                                if index > 0 { ", " }
                                <a href=(url) class=(theme::LINK)>(name)</a>
                            }
                        </p>
                    }
                </div>
                // The two steps are real links, not a control a script has to
                // wire: the page has to transpose with JavaScript off, and a
                // link is the one control that always works. Each one carries
                // the step it leads to, so the number a reader sees and the
                // address they are sent to are the same fact.
                <div class=(theme::TRANSPOSE) role="group" aria-label=(transpose)>
                    <span class=(theme::TRANSPOSE_LABEL) aria-hidden="true">(transpose)</span>
                    <a
                        href=(step_link(&sheet.get_path(), offset - 1))
                        class=(class!(theme::TRANSPOSE_LINK, theme::FOCUS))
                        aria-label=(i18n::text(lang, Key::TransposeDown))
                    >("\u{2212}")</a>
                    <span class=(theme::TRANSPOSE_VALUE)>(step)</span>
                    <a
                        href=(step_link(&sheet.get_path(), offset + 1))
                        class=(class!(theme::TRANSPOSE_LINK, theme::FOCUS))
                        aria-label=(i18n::text(lang, Key::TransposeUp))
                    >("+")</a>
                </div>
                // The reader's other control over the lyric: a speed bar that
                // crawls the sheet. It is the same control the book carries —
                // `ui::autoscroll` owns the panel, the words and the script,
                // because the control is the reader's and not this page's — and
                // it is drawn only by that script, so a sheet read with
                // JavaScript off shows no button that could not work.
                speed_bar()
                // Handing the sheet to somebody else. `ui::share` owns the row
                // and the two addresses; what this page hands it is its own
                // **canonical** URL — the address this sheet consolidates to,
                // built by the same call the page's `<head>` builds it with, so
                // a sharing link and a crawler are told the same URL.
                share_row(url: i18n::absolute(&sheet.get_path()))
                <a
                    href=(crate::pages::editor::PATH)
                    class=(class!(theme::BUTTON_SMALL, theme::FOCUS))
                >
                    (i18n::text(lang, Key::AddLyrics))
                </a>
            </header>

            <div class=(theme::LYRICS)>
                for line in lines {
                    lyric_line(line: line)
                }
            </div>

            <script type="text/javascript">(Unescaped::new_unchecked(autoscroll::SCRIPT))</script>
        </article>
    })
}

/// The address of this sheet one step away from `offset`.
///
/// `?tr=0` is left off: the untransposed sheet is this page's canonical URL, and
/// a link to `?tr=0` would be a second address for the page the reader is
/// already on. A step past the end of the wheel clamps to it — the link is still
/// served, and the sheet it returns is the one on screen.
fn step_link(path: &str, offset: i32) -> String {
    let stepped = offset.clamp(-chord::MAX_OFFSET, chord::MAX_OFFSET);
    let base = path.to_owned();
    if stepped == 0 {
        base
    } else {
        format!("{base}?tr={stepped}")
    }
}

/// One line of the lyric.
///
/// A blank line is the same element with the verse-break height instead of the
/// line height, so the loop has one shape rather than two. The two tokens are
/// composed rather than chosen between, which is why
/// `tokens_meant_to_be_composed_never_contradict_each_other` watches them.
///
/// The line is rendered from `Chunk`s rather than straight from [`LyricSpan`]s
/// because on this page a chord and its syllable are *one box*: the chord is
/// positioned against the character it was written over, so the two have to be
/// rendered together. See `chunks` for where that character comes from. (Both
/// are private, so they are named here rather than linked: a public doc that
/// links to a private item is a rustdoc warning.)
///
/// The anchor and the chord are written on one line on purpose: whitespace
/// between them would be part of the syllable's box, and would move the lyric's
/// own letters apart.
///
/// A chord keeps v3's `data-nosnippet`. That attribute is not decoration: a
/// search engine that quotes the page prints the chords inline with the words,
/// where they read as typos.
///
/// The label is written **unescaped**, because it is markup: [`chord::label`]
/// puts the accidental and the quality in an `<i>` tag, which is what the
/// reviewer asked for — the root is the word the language says, and `#m7` is the
/// same four characters on every sheet. It is the one string in this component
/// that is not escaped, and it is safe because the chord module writes every tag
/// in it itself and escapes every character of the label it did not write. See
/// [`crate::domain::chord`].
#[component]
pub async fn lyric_line(line: LyricLine) -> Result<impl View> {
    let blank = line.is_empty();
    let placed = chunks(&line);

    Ok(view! {
        <div class=(class!(theme::LYRIC_LINE, theme::LYRIC_GAP if blank))>
            for chunk in placed {
                match chunk {
                    Chunk::Text(text) => { (text) },
                    Chunk::Syllable { anchor, chord } => {
                        <span class=(theme::CHORDED)>(anchor)<sup class=(theme::CHORD) data-nosnippet="true">(Unescaped::new_unchecked(chord))</sup></span>
                    },
                }
            }
        </div>
    })
}

// ---------------------------------------------------------------------------
// Placing the chords
// ---------------------------------------------------------------------------

/// A gap the author held open in a lyric line.
///
/// The corpus writes its gaps with no-break spaces, and that is what a chord
/// with no text on either side hangs from — see [`chunks`].
const GAP: char = '\u{a0}';

/// One piece of a rendered line: lyric words, or a chord with the box it hangs
/// from.
///
/// The sheet renders these rather than [`LyricSpan`]s because a chord and its
/// syllable are one box on the page: a chord rendered as a *sibling* of the text
/// would have nothing to be positioned against but the whole line.
#[derive(Debug, PartialEq, Eq)]
enum Chunk {
    /// Lyric text with nothing above it.
    Text(String),
    /// The syllable a chord hangs from, and the chord. The anchor is one
    /// character — or one gap — lifted out of the text so the chord can be drawn
    /// over it.
    Syllable { anchor: String, chord: String },
}

/// Places every chord in `line` over the syllable it was written against.
///
/// This corpus writes a chord *after* the syllable it marks — `rei<sup>F</sup>nes`
/// is the F on the "i" of "reines" — so a chord hangs from the text before it.
/// The rules below are read off the dump that ships in `.run/inbox/` rather than
/// guessed, over 1,088 chord spans in 548 lines:
///
/// * **1,006 follow a run of text.** The anchor is that run's last vowel. In 842
///   of them the vowel is also the run's last character; in 164 the run ends in a
///   consonant (`Ratatum<sup>A</sup>`), and there the vowel is the syllable the
///   author is singing, while the consonant is not.
/// * **58 follow a run that ends in a gap.** A chord written after a gap marks a
///   change *at the gap*, not a chord on the syllable before it, so it hangs from
///   the gap itself. It prefers a no-break space when the run has one, because a
///   collapsible space at the end of a line is dropped and would take the chord's
///   box with it — 42 of those 58 runs end in one, the corpus's own way of
///   holding a line's spacing open.
/// * **36 have no text before them at all**, a chord at the head of a line
///   (`<sup>D</sup>'Ua rere mai ra`). It hangs from the *first vowel of the text
///   that follows*, which is the syllable it sounds on.
///
/// A chord with no text on either side — a line made only of chords — cannot
/// occur in this corpus (no line is), and is still not an error: it hangs from a
/// [`GAP`] of its own.
fn chunks(line: &LyricLine) -> Vec<Chunk> {
    let mut out: Vec<Chunk> = Vec::new();
    let mut pending = String::new();
    let mut waiting: VecDeque<String> = VecDeque::new();

    for span in line {
        match span {
            LyricSpan::Text(text) => {
                let mut rest = text.clone();
                // A chord with nothing before it hangs from the head of the run
                // that follows.
                while let Some(chord) = waiting.front().cloned() {
                    let Some((before, anchor, after)) = split_leading_anchor(&rest) else {
                        break;
                    };
                    if !before.is_empty() {
                        out.push(Chunk::Text(before));
                    }
                    out.push(Chunk::Syllable { anchor, chord });
                    waiting.pop_front();
                    rest = after;
                }
                pending.push_str(&rest);
            }
            LyricSpan::Chord(chord) => {
                if !waiting.is_empty() {
                    // Nothing in this run for the chords already waiting: this
                    // one queues behind them, and the run stays as it is.
                    if !pending.is_empty() {
                        out.push(Chunk::Text(std::mem::take(&mut pending)));
                    }
                    waiting.push_back(chord.clone());
                    continue;
                }
                match split_trailing_anchor(&pending) {
                    Some((before, anchor, after)) => {
                        if !before.is_empty() {
                            out.push(Chunk::Text(before));
                        }
                        out.push(Chunk::Syllable {
                            anchor,
                            chord: chord.clone(),
                        });
                        pending = after;
                    }
                    None => waiting.push_back(chord.clone()),
                }
            }
        }
    }

    for chord in waiting.drain(..) {
        out.push(Chunk::Syllable {
            anchor: GAP.to_string(),
            chord,
        });
    }
    if !pending.is_empty() {
        out.push(Chunk::Text(pending));
    }
    out
}

/// Splits `text` — the run before a chord — into what comes before the syllable,
/// the syllable itself, and what comes after it.
///
/// `None` means there is nothing here to hang from, and the chord has to wait for
/// the text that follows it.
fn split_trailing_anchor(text: &str) -> Option<(String, String, String)> {
    if text.is_empty() {
        return None;
    }
    let body = text.trim_end_matches(char::is_whitespace);
    let at = if body.is_empty() {
        // Nothing but a gap: the chord hangs from the gap.
        text.rfind(is_hard_gap).unwrap_or_else(|| last_char(text))
    } else if body.len() < text.len() {
        // A gap at the end: the chord marks the change at the gap.
        text.rfind(is_hard_gap)
            .filter(|at| *at >= body.len())
            .unwrap_or_else(|| last_char(text))
    } else {
        // A run of text: the syllable is its last vowel, and a run with no vowel
        // at all (a consonant, an ʻokina) is its last character.
        last_vowel(body).unwrap_or_else(|| last_char(body))
    };
    Some(split_around(text, at))
}

/// [`split_trailing_anchor`] for a chord written *before* the text it belongs to:
/// a chord at the head of a line hangs from the first vowel of the run that
/// follows it, or from its first letter if the run has no vowel.
///
/// `None` for a run of nothing but gaps: there is no syllable in it, and the
/// chord keeps waiting for text that has one.
fn split_leading_anchor(text: &str) -> Option<(String, String, String)> {
    let head = text.trim_start_matches(char::is_whitespace);
    if head.is_empty() {
        return None;
    }
    let offset = text.len() - head.len();
    let at = head
        .char_indices()
        .find(|(_, c)| is_vowel(*c))
        .map_or(offset, |(at, _)| at + offset);
    Some(split_around(text, at))
}

/// Cuts `text` into what is before the character at `at`, that character, and
/// what follows it.
fn split_around(text: &str, at: usize) -> (String, String, String) {
    let end = at + char_at(text, at).len_utf8();
    (
        text[..at].to_owned(),
        text[at..end].to_owned(),
        text[end..].to_owned(),
    )
}

/// The character at byte index `at`, which callers take from `char_indices`.
fn char_at(text: &str, at: usize) -> char {
    text[at..].chars().next().unwrap_or('\u{fffd}')
}

/// The byte index of the last character of `text`.
fn last_char(text: &str) -> usize {
    text.char_indices().next_back().map_or(0, |(at, _)| at)
}

/// The byte index of the last vowel in `text`.
fn last_vowel(text: &str) -> Option<usize> {
    text.char_indices()
        .filter(|(_, c)| is_vowel(*c))
        .map(|(at, _)| at)
        .next_back()
}

/// Whether `c` is a vowel of the two languages this corpus is written in.
///
/// French and Reo Tahiti share the Latin vowels; the set adds `y`, which the
/// corpus uses as one ("rythme"), and the accented and macroned letters it
/// actually contains — `À à è é ê î ï ù û`, `Ā ā Ē ē ī ō ū`. No combining marks
/// appear anywhere in the 43 songs, so one character is one letter here.
///
/// A letter missing from this set is not a crash: its syllable falls back to the
/// run's last character, which puts the chord one character to the right of
/// where it belongs.
fn is_vowel(c: char) -> bool {
    matches!(
        c.to_lowercase().next().unwrap_or(c),
        'a' | 'e'
            | 'i'
            | 'o'
            | 'u'
            | 'y'
            | 'à'
            | 'á'
            | 'â'
            | 'ä'
            | 'ã'
            | 'å'
            | 'è'
            | 'é'
            | 'ê'
            | 'ë'
            | 'ì'
            | 'í'
            | 'î'
            | 'ï'
            | 'ò'
            | 'ó'
            | 'ô'
            | 'ö'
            | 'õ'
            | 'ù'
            | 'ú'
            | 'û'
            | 'ü'
            | 'ÿ'
            | 'ā'
            | 'ē'
            | 'ī'
            | 'ō'
            | 'ū'
    )
}

/// Whether `c` is a space the browser cannot collapse: what this corpus uses to
/// hold a gap open in a lyric line.
fn is_hard_gap(c: char) -> bool {
    c.is_whitespace() && c != ' ' && c != '\t' && c != '\n' && c != '\r'
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::{Db, fixtures};

    /// The page renders only what is published. This is the rule
    /// [`db::song`]'s doc comment hands to its caller, asserted at the caller:
    /// the read returns the draft, and the filter the page applies rejects it.
    #[tokio::test]
    async fn a_draft_is_readable_but_not_publishable() {
        let db = Db::open_in_memory().await.expect("in-memory database");
        fixtures::seed(db.pool()).await.expect("seed fixtures");
        sqlx::query(
            "INSERT INTO song (id, title, lyrics, view_count, published, created_at, updated_at) \
             VALUES ('hidden0000000000000', 'Un brouillon', ?1, 1, 0, ?2, ?2)",
        )
        .bind("z".repeat(crate::domain::song::LYRICS_MIN))
        .bind("2026-01-01T00:00:00.000000000Z")
        .execute(db.pool())
        .await
        .expect("insert a draft");

        let draft = db::song(db.pool(), "hidden0000000000000")
            .await
            .expect("the read")
            .expect("the draft is readable");
        assert!(
            !draft.is_published(),
            "the page's filter can only reject a song it can see"
        );

        let visible = db::song(db.pool(), fixtures::SONGS[0].id)
            .await
            .expect("the read")
            .expect("a published fixture");
        assert!(visible.is_published());
    }

    /// [`lyric_line`] chooses the verse-break height from emptiness alone, so an
    /// empty line and a line of nothing but chords must not be confused.
    #[test]
    fn a_blank_line_is_an_empty_span_list() {
        let sheet = Song::new(
            "id".to_owned(),
            Some("titre".to_owned()),
            "Titre".to_owned(),
            "<div>one</div><div><br></div><div>two</div>".to_owned(),
            1,
            Vec::new(),
            true,
            chrono::Utc::now(),
            chrono::Utc::now(),
        );

        let lines = sheet.lyrics_lines();
        assert_eq!(lines.len(), 3);
        assert!(lines[1].is_empty());
        assert!(!lines[0].is_empty());
    }

    // ---- placing the chords ------------------------------------------------

    /// A sheet whose whole lyric is `markup`, as the importer would store it.
    fn sheet_of(markup: &str) -> Song {
        Song::new(
            "id".to_owned(),
            Some("titre".to_owned()),
            "Titre".to_owned(),
            markup.to_owned(),
            1,
            Vec::new(),
            true,
            chrono::Utc::now(),
            chrono::Utc::now(),
        )
    }

    /// The first line of `markup`, as the page receives it.
    fn line_of(markup: &str) -> LyricLine {
        sheet_of(markup)
            .lyrics_lines()
            .into_iter()
            .next()
            .expect("one line")
    }

    /// The rule this page exists to get right: the chord hangs from the vowel it
    /// was written against. v3 dragged it back over the lyric by a fixed half a
    /// rem, which is not a place in a word.
    #[test]
    fn a_chord_hangs_from_the_vowel_it_was_written_against() {
        assert_eq!(
            chunks(&line_of("Le reine des rei<sup>F</sup>nes")),
            vec![
                Chunk::Text("Le reine des re".to_owned()),
                Chunk::Syllable {
                    anchor: "i".to_owned(),
                    chord: "F".to_owned()
                },
                Chunk::Text("nes".to_owned()),
            ]
        );
    }

    /// A run that ends in a consonant still hands the chord to its vowel. The
    /// corpus's `Ratatum<sup>A</sup>` is an A on the syllable being sung, not on
    /// the `m` it happens to be typed next to.
    #[test]
    fn a_chord_on_a_run_that_ends_in_a_consonant_hangs_from_its_vowel() {
        assert_eq!(
            chunks(&line_of("Ratatum<sup>A</sup>, ratatum")),
            vec![
                Chunk::Text("Ratat".to_owned()),
                Chunk::Syllable {
                    anchor: "u".to_owned(),
                    chord: "A".to_owned()
                },
                Chunk::Text("m, ratatum".to_owned()),
            ]
        );
    }

    /// A chord written after a gap marks the change at the gap, so it hangs from
    /// the gap's own no-break space — which is also what keeps it clear of the
    /// chord before it instead of stacking two labels on one character.
    #[test]
    fn a_chord_after_a_gap_hangs_from_the_gap() {
        assert_eq!(
            chunks(&line_of("pape<sup>D</sup>&nbsp; &nbsp;<sup>G</sup>")),
            vec![
                Chunk::Text("pap".to_owned()),
                Chunk::Syllable {
                    anchor: "e".to_owned(),
                    chord: "D".to_owned()
                },
                Chunk::Text("\u{a0} ".to_owned()),
                Chunk::Syllable {
                    anchor: "\u{a0}".to_owned(),
                    chord: "G".to_owned()
                },
            ]
        );
    }

    /// A chord at the head of a line has nothing before it: it hangs from the
    /// first vowel of the text that follows — the syllable it sounds on. What
    /// comes before that vowel (`'Ua`'s ʻokina) stays where the author put it.
    #[test]
    fn a_chord_at_the_head_of_a_line_hangs_from_the_vowel_after_it() {
        assert_eq!(
            chunks(&line_of("<sup>D</sup>'Ua rere mai ra")),
            vec![
                Chunk::Text("'".to_owned()),
                Chunk::Syllable {
                    anchor: "U".to_owned(),
                    chord: "D".to_owned()
                },
                Chunk::Text("a rere mai ra".to_owned()),
            ]
        );
    }

    /// A run with no vowel at all — a lone consonant, the ʻokina — falls back to
    /// its last character, which is the only syllable it has. 21 chord spans in
    /// the dump are written like this.
    #[test]
    fn a_chord_after_a_run_with_no_vowel_hangs_from_its_last_character() {
        assert_eq!(
            chunks(&line_of("R<sup>Eb</sup>iro riro")),
            vec![
                Chunk::Syllable {
                    anchor: "R".to_owned(),
                    chord: "Eb".to_owned()
                },
                Chunk::Text("iro riro".to_owned()),
            ]
        );
    }

    /// The one property that matters more than any of the above: placing a chord
    /// moves a character into the chord's box, and loses nothing. Asserted over
    /// every fixture, because a rule that ate a syllable would still pass the
    /// cases above.
    ///
    /// The fixture corpus contains no line made only of chords — the one case
    /// where a chord brings an anchor of its own, which is asserted separately.
    #[test]
    fn placing_the_chords_keeps_every_character_of_the_corpus() {
        for fixture in fixtures::SONGS {
            for line in sheet_of(fixture.lyrics).lyrics_lines() {
                let placed: String = chunks(&line)
                    .iter()
                    .map(|chunk| match chunk {
                        Chunk::Text(text) => text.as_str(),
                        Chunk::Syllable { anchor, .. } => anchor.as_str(),
                    })
                    .collect();
                // Only the words: a chord label is not lyric text, and the
                // renderer does not put one back into the line.
                let original: String = line
                    .iter()
                    .filter_map(|span| match span {
                        LyricSpan::Text(text) => Some(text.as_str()),
                        LyricSpan::Chord(_) => None,
                    })
                    .collect();

                assert_eq!(
                    placed, original,
                    "a syllable of {:?} was lost or moved",
                    fixture.title
                );
            }
        }
    }

    /// ...and every chord in the corpus comes out exactly once, in the order it
    /// went in: the count is what says the rule that chooses the anchor never
    /// drops one.
    #[test]
    fn every_chord_in_the_corpus_becomes_one_syllable_in_order() {
        let mut chords = 0;
        let mut placed = 0;

        for fixture in fixtures::SONGS {
            for line in sheet_of(fixture.lyrics).lyrics_lines() {
                let chunks = chunks(&line);
                let labels: Vec<&str> = chunks
                    .iter()
                    .filter_map(|chunk| match chunk {
                        Chunk::Syllable { chord, .. } => Some(chord.as_str()),
                        Chunk::Text(_) => None,
                    })
                    .collect();
                let expected: Vec<&str> = line
                    .iter()
                    .filter_map(|span| match span {
                        LyricSpan::Chord(chord) => Some(chord.as_str()),
                        LyricSpan::Text(_) => None,
                    })
                    .collect();
                assert_eq!(
                    labels, expected,
                    "the chords of {:?} changed order",
                    fixture.title
                );

                chords += expected.len();
                placed += labels.len();
            }
        }

        assert!(chords > 50, "the fixtures stopped carrying chords");
        assert_eq!(placed, chords, "a chord was dropped or doubled");
    }

    /// The line the corpus does not have: nothing but chords. There is no text
    /// to hang from on either side, so each chord takes a gap of its own — the
    /// same no-break space the corpus writes a gap with.
    #[test]
    fn a_line_of_chords_alone_still_gives_each_chord_a_box() {
        assert_eq!(
            chunks(&line_of("<sup>C</sup> <sup>G</sup>")),
            vec![
                Chunk::Text(" ".to_owned()),
                Chunk::Syllable {
                    anchor: "\u{a0}".to_owned(),
                    chord: "C".to_owned()
                },
                Chunk::Syllable {
                    anchor: "\u{a0}".to_owned(),
                    chord: "G".to_owned()
                },
            ]
        );
    }
}
