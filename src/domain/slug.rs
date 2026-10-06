//! Slugs: the URL a song is published under.
//!
//! A song's address is its title, transliterated — `/himene/ahani-e` for
//! «&nbsp;'Āhani e&nbsp;». The rule lives here, in one place, because three call
//! paths need the same answer: the importer, the create-song form, and the test
//! below that runs it over the whole corpus. Uniqueness is *not* here — it needs
//! the database — and is settled by `db::queries::assign_slug`.
//!
//! Three decisions, each read off the 43 titles of the 2025-03-22 export rather
//! than guessed:
//!
//! * **Accents are folded, not dropped.** `Māmā Tahiti` becomes `mama-tahiti`.
//!   A macron marks a long vowel; the vowel either side of it is the word.
//!   The table below covers everything that export contains — `ā ē ī ō ū ē` —
//!   and the rest of the Latin alphabet a French or Tahitian title can need.
//! * **The ʻokina is a letter, not a separator.** This corpus writes it as an
//!   ASCII apostrophe (`Fa'aea mai`, `Te ta''ata hara nei`), and dropping it
//!   keeps the word whole: `faaea-mai`. Treating it as punctuation would produce
//!   `fa-aea-mai`, which is not how anyone spells the word.
//! * **Everything else is a word break.** Spaces, hyphens and dashes all collapse
//!   to one `-`, so `Tangata huruhuru - Ko Kiko` is `tangata-huruhuru-ko-kiko`.
//!
//! A title with no Latin letters at all — nothing in this corpus, but nothing
//! stops one — slugifies to the empty string and the caller gives the song no
//! slug: it is then published under its id, which is where v3 published it.
//! That is deliberate. Inventing a transliteration for a script the table does
//! not know would mint a URL its author could not have typed, and one that
//! nobody asked for.

/// The longest slug this site will mint.
///
/// `song.title` allows 255 characters; a URL does not want them. The cut lands
/// on a `-` boundary so the result is still whole words, and collisions are the
/// caller's problem either way — truncation can only make them more likely, and
/// [`crate::db::queries::assign_slug`] is what settles them.
const SLUG_MAX: usize = 80;

/// The slug `title` earns, or the empty string when it has no Latin letters.
///
/// Lowercase, ASCII, `-`-separated, never leading or trailing a separator, at
/// most `SLUG_MAX` characters. Deterministic: the same title always gives the
/// same slug, which is what lets the importer re-run over a dump without moving
/// a single URL.
pub fn slugify(title: &str) -> String {
    let mut out = String::with_capacity(title.len());
    let mut pending_break = false;

    for ch in title.chars() {
        // `to_lowercase` yields an iterator: one character can fold to several
        // (`İ` is `i` plus a combining dot), and dropping the extras would drop
        // a letter of the title.
        for lower in ch.to_lowercase() {
            if let Some(ascii) = latin_ascii(lower) {
                push_word(&mut out, &mut pending_break);
                out.push_str(ascii);
            } else if is_okina(lower) {
                // Dropped rather than turned into a break: see the module doc.
            } else if lower.is_ascii_alphanumeric() {
                push_word(&mut out, &mut pending_break);
                out.push(lower);
            } else {
                // A break, but only between two words: a leading separator would
                // be trimmed off again at the end anyway.
                pending_break = !out.is_empty();
            }
        }
    }

    truncate(out)
}

/// Starts a new word: the break that was held back becomes one `-`.
fn push_word(out: &mut String, pending_break: &mut bool) {
    if *pending_break {
        out.push('-');
    }
    *pending_break = false;
}

/// Cuts a slug at a word boundary, at most [`SLUG_MAX`] characters.
///
/// The input is ASCII by construction, so bytes are characters here and a slice
/// cannot land inside one.
fn truncate(slug: String) -> String {
    if slug.len() <= SLUG_MAX {
        return slug;
    }

    let cut = &slug[..SLUG_MAX];
    match cut.rfind('-') {
        Some(at) if at > 0 => cut[..at].to_owned(),
        _ => cut.to_owned(),
    }
}

/// The ASCII spelling of a Latin letter that carries a diacritic, if this is one.
///
/// Only the lowercase forms are listed: [`slugify`] folds case before asking, so
/// `Ā` reaches here as `ā`. Ligatures and the sharp s expand to two letters, as
/// they do in every other URL on the web.
fn latin_ascii(c: char) -> Option<&'static str> {
    let ascii = match c {
        'à' | 'á' | 'â' | 'ã' | 'ä' | 'å' | 'ā' | 'ă' | 'ą' => "a",
        'ç' | 'ć' | 'ĉ' | 'ċ' | 'č' => "c",
        'ď' | 'đ' => "d",
        'è' | 'é' | 'ê' | 'ë' | 'ē' | 'ĕ' | 'ė' | 'ę' | 'ě' => "e",
        'ĝ' | 'ğ' | 'ġ' | 'ģ' => "g",
        'ĥ' | 'ħ' => "h",
        'ì' | 'í' | 'î' | 'ï' | 'ĩ' | 'ī' | 'ĭ' | 'į' | 'ı' => "i",
        'ĵ' => "j",
        'ķ' => "k",
        'ĺ' | 'ļ' | 'ľ' | 'ł' => "l",
        'ñ' | 'ń' | 'ņ' | 'ň' => "n",
        'ò' | 'ó' | 'ô' | 'õ' | 'ö' | 'ø' | 'ō' | 'ŏ' | 'ő' => "o",
        'ŕ' | 'ŗ' | 'ř' => "r",
        'ś' | 'ŝ' | 'ş' | 'š' => "s",
        'ţ' | 'ť' | 'ŧ' => "t",
        'ù' | 'ú' | 'û' | 'ü' | 'ũ' | 'ū' | 'ŭ' | 'ů' | 'ű' | 'ų' => "u",
        'ŵ' => "w",
        'ý' | 'ÿ' | 'ŷ' => "y",
        'ź' | 'ż' | 'ž' => "z",
        'æ' => "ae",
        'œ' => "oe",
        'ß' => "ss",
        _ => return None,
    };

    Some(ascii)
}

/// Whether `c` is one of the marks a Tahitian or French title writes an ʻokina
/// or an apostrophe with.
///
/// The corpus uses the ASCII apostrophe; the others are what a browser, a phone
/// keyboard or a copy from a word processor produces instead, and a URL that
/// changed because of the *keyboard* would be a defect.
fn is_okina(c: char) -> bool {
    matches!(
        c,
        '\'' | '\u{2018}' | '\u{2019}' | '\u{02bc}' | '\u{02bb}' | '`' | '\u{b4}'
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeSet;

    /// Every song title in the 2025-03-22 export, as the dump writes it.
    ///
    /// The rule is pinned against the corpus rather than against invented
    /// examples: these are the 43 titles the site actually has, and the two
    /// properties that matter are asserted over all of them — that each one
    /// produces a slug, and that no two of them collide. A collision here would
    /// not be a crash; it would be one of two songs quietly addressed as
    /// `-2`, which is why it is worth knowing the corpus does not have one.
    const CORPUS: &[&str] = &[
        "'Aremiti fa'ahe'e",
        "'Ua mo'e ho'i 'oe iā'u",
        "'Ua ro'ohia tā'u tino",
        "'Ua topa te hau",
        "'Āhani e",
        "Au manava",
        "Aue ho'i Eritana",
        "De Tahiti jusqu'au Wallis",
        "Dis-moi Tiau",
        "E hīmene",
        "E tamaiti ori noa",
        "E tumu ha'ari teie",
        "E ua parau e hia na 'oe",
        "Fa'aea mai",
        "Fakateretere",
        "Ha'avitviti",
        "Je te promets",
        "Kāveka kura ora",
        "Kāveka te manu",
        "Laisse moi cette nuit encore",
        "Māmā Tahiti",
        "Marchons ensemble",
        "Mihi au ia 'oe",
        "Moerava",
        "Mou mai ana",
        "Nā te Ra'i",
        "N'est-ce pas le plaisir",
        "No 'oe te Atua",
        "Pahoho",
        "Pape meha'i",
        "Pere Te Auahi",
        "Ratatum et ratamtam",
        "Tahiti nui",
        "Taku Tamahine Here Ia",
        "Tangata huruhuru - Ko Kiko",
        "Te here fenua",
        "Te ta''ata hara nei",
        "Te tama Mā'ohi",
        "Te tama o te mahana",
        "Tō'u ia hīro'a",
        "Une île bordée de bleu",
        "Vai Tahiti Nui Here",
        "Vaihiria",
    ];

    /// The example the plan was written around: the apostrophe and the macron
    /// both disappear into a readable URL.
    #[test]
    fn the_rule_is_the_one_the_step_was_specified_with() {
        assert_eq!(slugify("'Āhani e"), "ahani-e");
    }

    #[test]
    fn accents_fold_to_their_ascii_letter() {
        assert_eq!(slugify("Māmā Tahiti"), "mama-tahiti");
        assert_eq!(slugify("E hīmene"), "e-himene");
        assert_eq!(slugify("Tō'u ia hīro'a"), "tou-ia-hiroa");
        assert_eq!(slugify("Une île bordée de bleu"), "une-ile-bordee-de-bleu");
        assert_eq!(slugify("Kāveka te manu"), "kaveka-te-manu");
        // ...including the capitalised spelling, which folds before it is mapped.
        assert_eq!(slugify("Ā Ē Ī Ō Ū"), "a-e-i-o-u");
        // Ligatures expand, as they do everywhere else on the web.
        assert_eq!(slugify("Œuvre, cœur & sœur"), "oeuvre-coeur-soeur");
    }

    /// The apostrophe joins, it does not separate: `faaea-mai`, not `fa-aea-mai`.
    #[test]
    fn the_okina_is_a_letter_of_the_word() {
        assert_eq!(slugify("Fa'aea mai"), "faaea-mai");
        assert_eq!(slugify("Te ta''ata hara nei"), "te-taata-hara-nei");
        assert_eq!(slugify("N'est-ce pas le plaisir"), "nest-ce-pas-le-plaisir");
        // The four marks another keyboard might produce instead of the ASCII one.
        assert_eq!(slugify("Fa\u{2019}aea mai"), "faaea-mai");
        assert_eq!(slugify("Fa\u{02bb}aea mai"), "faaea-mai");
        assert_eq!(slugify("Fa\u{02bc}aea mai"), "faaea-mai");
        assert_eq!(slugify("Fa\u{2018}aea mai"), "faaea-mai");
    }

    #[test]
    fn punctuation_and_spacing_become_one_separator() {
        assert_eq!(
            slugify("Tangata huruhuru - Ko Kiko"),
            "tangata-huruhuru-ko-kiko"
        );
        assert_eq!(slugify("Dis-moi Tiau"), "dis-moi-tiau");
        assert_eq!(slugify("  Espaces   partout  "), "espaces-partout");
        assert_eq!(
            slugify("De Tahiti jusqu'au Wallis"),
            "de-tahiti-jusquau-wallis"
        );
    }

    /// A title of nothing but punctuation has no slug at all: the song keeps its
    /// id URL rather than getting an invented word.
    #[test]
    fn a_title_with_no_latin_letters_has_no_slug() {
        assert_eq!(slugify(""), "");
        assert_eq!(slugify("..."), "");
        assert_eq!(slugify("！！"), "");
        assert_eq!(slugify("日本語のうた"), "");
    }

    #[test]
    fn a_long_title_is_cut_at_a_word_boundary() {
        let title = "mot ".repeat(100); // 400 characters, all words
        let slug = slugify(&title);

        assert!(slug.len() <= SLUG_MAX, "{} characters", slug.len());
        assert!(!slug.ends_with('-'), "{slug:?}");
        assert!(slug.starts_with("mot-mot-mot"), "{slug:?}");
        // Cut on a boundary, not mid-word: what is left is whole repetitions.
        assert_eq!(slug, "mot-".repeat(19) + "mot");
    }

    /// The property the whole step rests on: every title in the corpus gets a
    /// slug, and no two songs would be minted the same one.
    #[test]
    fn the_corpus_yields_one_distinct_slug_per_song() {
        let slugs: Vec<String> = CORPUS.iter().map(|title| slugify(title)).collect();

        assert_eq!(slugs.len(), 43, "the corpus changed size");
        for (title, slug) in CORPUS.iter().zip(&slugs) {
            assert!(!slug.is_empty(), "{title:?} produced no slug");
            assert!(
                slug.chars()
                    .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-'),
                "{title:?} produced a character a URL path should not carry: {slug:?}"
            );
        }

        let distinct: BTreeSet<&String> = slugs.iter().collect();
        assert_eq!(
            distinct.len(),
            slugs.len(),
            "two songs of the corpus share a slug"
        );
    }

    /// Idempotence is what lets the importer re-run over the same dump without
    /// moving a URL: the same title always gives the same slug.
    #[test]
    fn the_same_title_always_gives_the_same_slug() {
        for title in CORPUS {
            assert_eq!(slugify(title), slugify(title));
        }
    }
}
