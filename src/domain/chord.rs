//! The chord wheel: one semitone at a time, and the name each step gets.
//!
//! A chord sheet is written in one key and sung in another. `?tr=` on a song
//! page moves every chord in the sheet that many semitones up or down, and moves
//! nothing else — the words are the words.
//!
//! # The wheel
//!
//! Twelve pitch classes, each spelled once. That table is the whole naming
//! policy, and it lives here and nowhere else: the corpus writes both flats and
//! sharps (`Bb` 48 times, `F#` 33, `C#` 4, `Db` 16), so a transposed chord has
//! to pick a spelling, and picking at the call site is how one sheet ends up
//! with two names for the same chord on the same line.
//!
//! **At offset zero nothing is rewritten.** The author's spelling is the sheet's
//! spelling, so a song opens on exactly the chords that were typed, and a reader
//! who steps away and steps back lands on the original page. A step is a
//! function of the *stored* chord and the offset, never of the step before it.
//! (The *root* is still read in the page's language — that is the display rule
//! below, and it moves no chord: `Bb` is `Si b` and `C#` is `Do #`.)
//!
//! # What a chord label may hold
//!
//! More than one chord. The corpus's `<sup>` spans hold 44 distinct strings, all
//! of them read off the dump in `.run/inbox/` rather than guessed:
//!
//! * a root, with or without an accidental, then a quality suffix — `C`, `F#m`,
//!   `Abm`, `Em7`, `C#m`, `Db7`;
//! * two chords in one span, separated by whitespace, often a long run of it:
//!   `Db Ab`, `Dm     C`, `Am     F     G7`. The spacing is the author's layout
//!   and is carried through untouched;
//! * a slash chord — `B/Eb`, `A/B` — and both halves move;
//! * nothing else. Every capital `A`–`G` inside a chord span is a root, because
//!   a `<sup>` holds chords and only chords.
//!
//! The scanner leans on that last fact: inside a chord label a capital `A`–`G`
//! begins a chord, the run of letters and digits after the root is its quality
//! and is preserved verbatim, and every other character — a space, a slash — is
//! copied. A label that holds no chord at all (`x2`, `N.C.`) is returned
//! unchanged, which is why the scanner can be pointed at a `<sup>` without
//! asking what is in it.
//!
//! # How the page writes a chord
//!
//! A chord sheet is read by musicians who do not share a language, so a label is
//! split between the part a language says and the part every chart says the same
//! way:
//!
//! * **the root is the language's own name for it** — French solfège (`Do Ré Mi
//!   Fa Sol La Si`), the Tahitian the reviewer gave (`Tō Rē Mī Fā Hō Rā Tī`), or
//!   the letters every chart prints (`C D E F G A B`);
//! * **the accidental is ASCII, and it is the same on all three**: a sharp is `#`
//!   and a flat is `b` — never a word and never a glyph, the two the reviewer
//!   struck out, "too little to read";
//! * **the accidental and the quality ride in an `<i>` tag**, the root upright:
//!   `#m7` is slanted and `Do` is not. That is the reviewer's "world-wide
//!   language of artists", and it is the same four characters on a French page
//!   as on a Tahitian one. (The tag is written `<i>` and not the utility that
//!   word spells — `theme.rs` writes that name as `concat!("ital", "ic")` for the
//!   same reason: Tailwind scans this text, and the bare word would put a rule
//!   nothing uses into every visitor's stylesheet.)
//!
//! So [`label`] returns **markup**, not text, and the page hands it to the
//! renderer unescaped. Everything in a label that this module did not write — the
//! spaces between two chords, a slash, the `N.C.` of a bar with no chord — is
//! escaped on the way out, so a stored label cannot leave its own element. No
//! face the site ships has a slanted cut, so the browser synthesises the oblique:
//! see [`crate::ui::theme::CHORD`].
//!
//! The names are display only: the Markdown and JSON forms of the same page keep
//! the canonical spelling, because those are the forms a machine reads and a
//! chord it reads should be the one the author typed.
//!
//! This is the one place in [`crate::domain`] that reads the site's [`Lang`],
//! and it does so because the spelling of a chord *is* a language matter: the
//! alternative is a caller that re-parses a label to find its roots, which is the
//! duplication the wheel exists to prevent.

use crate::domain::song::{LyricLine, LyricSpan};
use crate::i18n::Lang;

/// How far a sheet may be moved, either way.
///
/// Eleven, so no key is out of reach: a reader who needs the chord a semitone
/// down from the top of the wheel is a step away from the bottom. Pressing on at
/// the limit does nothing — the link is served, and the sheet it returns is the
/// one already on screen.
pub const MAX_OFFSET: i32 = 11;

/// The twelve pitch classes, each spelled once — see the module docs.
///
/// The flat/sharp choice is the corpus's own: `Eb` 30 times against no `D#`,
/// `Ab` 26 against no `G#` (except in `G#m`), `Bb` 48 against no `A#`, `F#` 33
/// against `Gb` 7, and `C#` — the one the plan names as a French example — over
/// `Db`'s 16, because it is the spelling a reader transposing *up* from `C`
/// expects to meet first.
const WHEEL: [&str; 12] = [
    "C", "C#", "D", "Eb", "E", "F", "F#", "G", "Ab", "A", "Bb", "B",
];

/// The seven letters: the letter, the name it is read by in each of the site's
/// languages — in [`Lang::ALL`]'s order, fr / ty / en — and the semitone it
/// names with `C` = 0.
///
/// The Tahitian is the reviewer's own list (`C=Tō, D=Rē, E=Mī, F=Fā, G=Hō,
/// A=Rā, Si=Tī`), given on the pull request rather than left to the fallback the
/// prose takes: a note name is not a sentence, and there is no English one to
/// fall back to.
const LETTERS: [(u8, [&str; 3], i32); 7] = [
    (b'C', ["Do", "Tō", "C"], 0),
    (b'D', ["Ré", "Rē", "D"], 2),
    (b'E', ["Mi", "Mī", "E"], 4),
    (b'F', ["Fa", "Fā", "F"], 5),
    (b'G', ["Sol", "Hō", "G"], 7),
    (b'A', ["La", "Rā", "A"], 9),
    (b'B', ["Si", "Tī", "B"], 11),
];

/// The column of [`LETTERS`] a language's root names live in.
///
/// A match rather than a lookup in [`Lang::ALL`], so that a fourth language
/// cannot be added without giving every root a name in it — the same reason
/// `ui::flags` matches on the language to choose its drawing.
const fn column(lang: Lang) -> usize {
    match lang {
        Lang::Fr => 0,
        Lang::Ty => 1,
        Lang::En => 2,
    }
}

/// The row of [`LETTERS`] that names `byte`, or `None` for anything else.
fn letter(byte: u8) -> Option<&'static (u8, [&'static str; 3], i32)> {
    LETTERS.iter().find(|(candidate, _, _)| *candidate == byte)
}

/// The step an accidental asks for.
fn accidental(byte: Option<u8>) -> i32 {
    match byte {
        Some(b'#') => 1,
        Some(b'b') => -1,
        _ => 0,
    }
}

/// Whether a chord may begin here: at the start of the label, or after the one
/// thing the author used to separate two of them.
///
/// The corpus separates a run of chords with whitespace — often a long run of
/// it, which is the author's layout — and a slash chord's two halves with `/`.
/// The rule is written as a whitelist rather than as "not a letter" for one
/// reason: `N.C.` is a chord sheet's "no chord", and a scanner that took the `C`
/// in it for a root would move a label that holds no chord at all.
fn starts_a_root(before: Option<char>) -> bool {
    match before {
        None => true,
        Some('/') => true,
        Some(ch) => ch.is_whitespace(),
    }
}

/// Rewrites every chord root in a chord label and leaves the rest alone.
///
/// `replace` is handed the root's letter, its accidental, and the quality suffix
/// that follows — `m`, `7` or `m7`, or nothing. It is handed the suffix rather
/// than left to append it because everything after the root is written as one
/// run: the accidental and the quality go in a single tag, so `Si<i>bm</i>` is
/// one tag and `Do<i>#m</i>` is another, and only the callback knows which of the
/// two it is writing.
///
/// `markup` says what the returned string is going to be read as. When it is
/// HTML, every character the callback did not write is escaped: a label is data
/// (`&lt;` in the sheet is a `<` in the label) and the page is handed the result
/// unescaped. The other readers — [`transpose`] and the Markdown and JSON forms
/// built on it — want the label's own bytes, so they say `false`.
fn map_roots(
    label: &str,
    markup: bool,
    mut replace: impl FnMut(u8, Option<u8>, &str) -> String,
) -> String {
    let mut out = String::with_capacity(label.len());
    let mut previous: Option<char> = None;
    let mut chars = label.chars().peekable();

    while let Some(ch) = chars.next() {
        let root =
            ch.is_ascii_alphabetic() && starts_a_root(previous) && letter(ch as u8).is_some();

        if !root {
            if markup {
                escape_into(&mut out, ch);
            } else {
                out.push(ch);
            }
            previous = Some(ch);
            continue;
        }

        let accidental_of_root = match chars.peek() {
            Some(&next @ ('#' | 'b')) => {
                chars.next();
                Some(next as u8)
            }
            _ => None,
        };

        // The quality suffix — the run of letters and digits after the root — is
        // carried through untouched.
        let mut suffix = String::new();
        while let Some(&next) = chars.peek() {
            if !next.is_ascii_alphanumeric() {
                break;
            }
            suffix.push(next);
            chars.next();
        }

        out.push_str(&replace(ch as u8, accidental_of_root, &suffix));
        previous = suffix.chars().next_back().or(Some(ch));
    }

    out
}

/// Copies one character of a label into markup, escaping the four that would
/// otherwise let a stored label out of the element it is written in.
fn escape_into(out: &mut String, ch: char) {
    match ch {
        '<' => out.push_str("&lt;"),
        '>' => out.push_str("&gt;"),
        '&' => out.push_str("&amp;"),
        '"' => out.push_str("&quot;"),
        _ => out.push(ch),
    }
}

/// Transposes every chord in a label by `offset` semitones.
pub fn transpose(label: &str, offset: i32) -> String {
    map_roots(label, false, |root, accidental_of_root, suffix| {
        let (_, _, pitch) = *letter(root).expect("only letters are handed back");
        let stepped = (pitch + accidental(accidental_of_root) + offset).rem_euclid(12);
        format!("{}{suffix}", WHEEL[stepped as usize])
    })
}

/// A label's canonical spelling: the author's at offset zero, the wheel's
/// otherwise.
pub fn spell(label: &str, offset: i32) -> String {
    if offset == 0 {
        label.to_owned()
    } else {
        transpose(label, offset)
    }
}

/// One label as the page should read it: the root in `lang`, then the accidental
/// and the quality in an `<i>`.
///
/// The root is the language's name for the letter — solfège, the reviewer's
/// Tahitian, or the letter itself — and everything after it is the accidental
/// and the quality, which are the same on every sheet: `Do<i>#m</i>`,
/// `Tō<i>#m</i>` and `C<i>#m</i>` are the same chord. The tag is left off when
/// there is nothing to put in it, so a plain `Sol` is a plain `Sol`.
fn localised(label: &str, lang: Lang) -> String {
    map_roots(label, true, |root, accidental_of_root, suffix| {
        let (_, names, _) = *letter(root).expect("only letters are handed back");
        let name = names[column(lang)];

        let mut tail = String::new();
        if let Some(accidental) = accidental_of_root {
            tail.push(accidental as char);
        }
        tail.push_str(suffix);

        if tail.is_empty() {
            name.to_owned()
        } else {
            format!("{name}<i>{tail}</i>")
        }
    })
}

/// One chord label as the page should read it: the wheel first, the language
/// second.
pub fn label(text: &str, offset: i32, lang: Lang) -> String {
    localised(&spell(text, offset), lang)
}

/// A whole lyric line with every chord prepared.
///
/// Text spans come back byte-for-byte: the lyrics are never touched, only the
/// chords drawn over them.
pub fn localised_line(line: LyricLine, offset: i32, lang: Lang) -> LyricLine {
    line.into_iter()
        .map(|span| match span {
            LyricSpan::Chord(chord) => LyricSpan::Chord(label(&chord, offset, lang)),
            text => text,
        })
        .collect()
}

/// The `tr` parameter of a query string, in semitones.
///
/// A query parameter and not a path segment, because it is not an address: the
/// page it renders is the same page, which is why the canonical URL leaves it
/// out and a crawler consolidates the two. Anything that is not a whole number —
/// a missing parameter, `tr=many`, a percent-encoded value — is the untransposed
/// sheet, and the offset is clamped to [`MAX_OFFSET`] so no link can ask for a
/// step the wheel cannot name.
pub fn offset(query: &str) -> i32 {
    query
        .split('&')
        .filter_map(|pair| pair.split_once('='))
        .find(|(name, _)| *name == "tr")
        .and_then(|(_, value)| value.trim().parse::<i32>().ok())
        .unwrap_or(0)
        .clamp(-MAX_OFFSET, MAX_OFFSET)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every distinct chord label in the 2025-03-22 export of the corpus, read
    /// off the dump rather than invented — 1,088 `<sup>` spans, 44 strings.
    /// The 44 labels are separated by `|` rather than written out as a slice, because
    /// eight of them hold spaces of their own — `Dm     C` is one label, not two.
    const CORPUS: &str = "C|D|A|G|F|E|B|Bb|F#|Eb|Ab|Bm|Em|Am|Db|F#m|C7|G7|C#m|Gm|Dm|Gb|Cm|Abm|G#m|Em7|F7|Fm|C#|A7|D7|Db Ab|Dm     C|B/Eb|Fm Cm|B7|Gbm|Am     F     G7|A/B|F#m       A7|F#m |Bbm|Ebm|Db7";

    /// The list above is the dump's vocabulary, pinned so that a label cannot
    /// quietly drop out of every test in this module.
    #[test]
    fn the_corpus_list_is_the_dumps_vocabulary() {
        let labels: std::collections::BTreeSet<&str> = CORPUS.split('|').collect();
        assert_eq!(labels.len(), 44, "not the dump's 44 distinct labels");
    }

    /// The corpus's own example, and the one the plan writes out by hand.
    #[test]
    fn c_to_c_sharp_to_d_and_back() {
        assert_eq!(transpose("C", 1), "C#");
        assert_eq!(transpose("C#", 1), "D");
        assert_eq!(transpose("D", -1), "C#");
        assert_eq!(transpose("C#", -1), "C");
    }

    /// A round trip is exact for every label in the corpus, whatever the wheel
    /// had to respell on the way: the pitch class is what moves, and a step back
    /// is the same pitch class again.
    #[test]
    fn a_step_up_and_the_same_step_down_land_on_the_same_chord() {
        for chord in CORPUS.split('|') {
            let canonical = transpose(chord, 0);
            for step in [-11, -7, -3, -1, 1, 2, 5, 11] {
                let there = transpose(chord, step);
                assert_eq!(
                    transpose(&there, -step),
                    canonical,
                    "{chord:?} + {step} then - {step} lost its way"
                );
            }
        }
    }

    /// Twelve steps is every pitch class, each spelled once: the wheel is a
    /// wheel, not a ramp.
    #[test]
    fn twelve_steps_name_twelve_chords() {
        let named: Vec<String> = (0..12).map(|step| transpose("C", step)).collect();
        let distinct: std::collections::BTreeSet<&String> = named.iter().collect();

        assert_eq!(named.len(), 12);
        assert_eq!(
            distinct.len(),
            12,
            "the wheel names a chord twice: {named:?}"
        );
        assert_eq!(transpose("C", 12), "C");
    }

    /// At zero the author's spelling is the sheet's, which is what makes the
    /// page a reader opens the page the author typed.
    ///
    /// The *root* is still read in the page's language — that is [`label`]'s job
    /// and the next test's — so this is asserted of [`spell`], the spelling the
    /// machine forms get.
    #[test]
    fn nothing_is_rewritten_at_zero() {
        for chord in CORPUS.split('|') {
            assert_eq!(spell(chord, 0), chord);
        }
    }

    /// A quality suffix is not a chord: `m`, `7` and `m7` survive the step.
    #[test]
    fn the_quality_survives_the_step() {
        assert_eq!(transpose("Abm", 1), "Am");
        assert_eq!(transpose("Em7", 2), "F#m7");
        assert_eq!(transpose("C#m", -2), "Bm");
        assert_eq!(transpose("Db7", -1), "C7");
        assert_eq!(transpose("Bbm", 2), "Cm");
    }

    /// A span may hold two chords and the whitespace between them, which is the
    /// author's spacing and is not the transposer's to tidy.
    #[test]
    fn a_run_of_chords_keeps_its_spacing() {
        assert_eq!(transpose("Db Ab", 1), "D A");
        assert_eq!(transpose("Dm     C", 2), "Em     D");
        assert_eq!(transpose("Am     F     G7", 2), "Bm     G     A7");
        assert_eq!(transpose("F#m ", 1), "Gm ");
    }

    /// A slash chord is two chords, and both move.
    #[test]
    fn a_slash_chord_transposes_both_halves() {
        assert_eq!(transpose("B/Eb", 1), "C/E");
        assert_eq!(transpose("A/B", -1), "Ab/Bb");
    }

    /// A `<sup>` that holds no chord is left exactly as it is. This is why the
    /// caller may point the scanner at a span without asking what is in it.
    #[test]
    fn a_span_with_no_chord_in_it_is_untouched() {
        for text in ["x2", "N.C.", "…", "(2x)"] {
            assert_eq!(transpose(text, 3), text);
            assert_eq!(label(text, -5, Lang::Fr), text);
        }
    }

    /// The reviewer's line, in the three languages at once: the root is the
    /// language's name, the accidental is ASCII, and the accidental with the
    /// quality ride in an `<i>` while the root does not.
    #[test]
    fn every_language_names_the_root_and_the_rest_is_the_same_everywhere() {
        assert_eq!(label("C#m7", 0, Lang::Fr), "Do<i>#m7</i>");
        assert_eq!(label("C#m7", 0, Lang::Ty), "Tō<i>#m7</i>");
        assert_eq!(label("C#m7", 0, Lang::En), "C<i>#m7</i>");

        assert_eq!(label("C#", 0, Lang::Fr), "Do<i>#</i>");
        assert_eq!(label("Bb", 0, Lang::Fr), "Si<i>b</i>");
        assert_eq!(label("Abm", 0, Lang::Fr), "La<i>bm</i>");
        assert_eq!(label("Bbm", 0, Lang::Fr), "Si<i>bm</i>");

        // A root with nothing after it is a bare name: no empty tag.
        assert_eq!(label("C", 0, Lang::Fr), "Do");
        assert_eq!(label("C", 0, Lang::Ty), "Tō");
        assert_eq!(label("C", 0, Lang::En), "C");
        assert_eq!(label("Ebm", 0, Lang::Ty), "Mī<i>bm</i>");
        assert_eq!(label("Gb", 0, Lang::En), "G<i>b</i>");

        // A step moves the chord and the spelling follows it, in the language.
        assert_eq!(label("C", 1, Lang::Fr), "Do<i>#</i>");
        assert_eq!(label("A", 1, Lang::Fr), "Si<i>b</i>");
        assert_eq!(label("F#m", 1, Lang::Fr), "Sol<i>m</i>");
        assert_eq!(label("Em7", 0, Lang::Fr), "Mi<i>m7</i>");
        assert_eq!(
            label("Am     F     G7", 2, Lang::Ty),
            "Tī<i>m</i>     Hō     Rā<i>7</i>"
        );
    }

    /// No flat glyph and no sharp *word* anywhere, in any language, at any step:
    /// the accidental is the character every chart uses. This assertion is the
    /// one place in the tree those two spellings appear — as the claim that they
    /// do not, which is why `.run/step39.sh` allowlists this line and nothing
    /// else.
    #[test]
    fn the_accidental_is_a_character_and_never_a_word() {
        for chord in CORPUS.split('|') {
            for lang in Lang::ALL {
                for step in [-7, -1, 0, 1, 5] {
                    let written = label(chord, step, lang);
                    assert!(
                        !written.contains('♭') && !written.contains("dièse"),
                        "{chord:?} + {step} in {} reads {written:?}",
                        lang.code()
                    );
                }
            }
        }
    }

    /// What is in the tag is an accidental and a quality — never a root. The
    /// root is the word the language says; everything else is the part of a
    /// chord every musician reads the same way.
    #[test]
    fn the_tag_holds_the_accidental_and_the_quality_and_never_the_root() {
        let roots: Vec<&str> = LETTERS
            .iter()
            .flat_map(|(_, names, _)| names.iter().copied())
            .collect();

        for chord in CORPUS.split('|') {
            for lang in Lang::ALL {
                for written in [label(chord, 0, lang), label(chord, 3, lang)] {
                    for tail in written.split("<i>").skip(1) {
                        let (inside, _) = tail.split_once("</i>").expect("an unclosed tag");
                        assert!(!inside.is_empty(), "{written:?} writes an empty tag");
                        assert!(
                            inside
                                .chars()
                                .all(|ch| ch.is_ascii_alphanumeric() || ch == '#'),
                            "{written:?} slants {inside:?}, which is not an accidental \
                             and a quality"
                        );
                        assert!(
                            !roots.contains(&inside),
                            "{written:?} slants {inside:?}, which is a root"
                        );
                    }
                }
            }
        }
    }

    /// A label is data, and the page is handed the tag it carries unescaped: a
    /// sheet whose own characters are markup characters is escaped, not obeyed.
    /// The roots around them are still read — the escaping is of the characters
    /// this module did not write and of nothing else.
    #[test]
    fn a_label_cannot_leave_the_element_it_is_written_in() {
        assert_eq!(label("C & D", 0, Lang::Fr), "Do &amp; Ré");
        assert_eq!(label("C < D", 0, Lang::Fr), "Do &lt; Ré");
        assert_eq!(label("C > D", 0, Lang::Ty), "Tō &gt; Rē");
        assert_eq!(localised("\"", Lang::En), "&quot;");

        // ...and the machine forms keep the label's own bytes.
        assert_eq!(spell("A < B", 0), "A < B");
        assert_eq!(transpose("A < B", 1), "Bb < C");
    }

    /// The other two languages read a chord the same way a French page does —
    /// the root in their words, the rest identical — and a machine reads what
    /// the author wrote.
    #[test]
    fn the_canonical_spelling_is_what_a_machine_gets() {
        assert_eq!(spell("C#", 0), "C#");
        assert_eq!(spell("C", 1), "C#");
        assert_eq!(label("C#", 0, Lang::En), "C<i>#</i>");
        assert_eq!(label("C", 1, Lang::En), "C<i>#</i>");
    }

    /// Only the chords of a line are prepared; the words come back unread.
    #[test]
    fn a_line_maps_its_chords_and_not_its_words() {
        let line = vec![
            LyricSpan::Text("'Āhani e".to_owned()),
            LyricSpan::Chord("Abm".to_owned()),
            LyricSpan::Text(" E rāve'a".to_owned()),
        ];

        assert_eq!(
            localised_line(line.clone(), 1, Lang::En),
            vec![
                LyricSpan::Text("'Āhani e".to_owned()),
                LyricSpan::Chord("A<i>m</i>".to_owned()),
                LyricSpan::Text(" E rāve'a".to_owned()),
            ]
        );
        assert_eq!(
            localised_line(line, 0, Lang::Fr),
            vec![
                LyricSpan::Text("'Āhani e".to_owned()),
                LyricSpan::Chord("La<i>bm</i>".to_owned()),
                LyricSpan::Text(" E rāve'a".to_owned()),
            ]
        );
    }

    /// The query parameter: a whole number, clamped, and nothing else counts.
    #[test]
    fn the_offset_comes_from_the_query_or_is_zero() {
        assert_eq!(offset(""), 0);
        assert_eq!(offset("tr=2"), 2);
        assert_eq!(offset("tr=-3"), -3);
        assert_eq!(offset("lang=fr&tr=5"), 5);
        assert_eq!(offset("tr=+1"), 1);
        assert_eq!(offset("tr=99"), MAX_OFFSET);
        assert_eq!(offset("tr=-99"), -MAX_OFFSET);
        assert_eq!(offset("tr=many"), 0);
        assert_eq!(offset("tr="), 0);
        assert_eq!(offset("tr=2.5"), 0);
        assert_eq!(offset("other=2"), 0);
    }
}
