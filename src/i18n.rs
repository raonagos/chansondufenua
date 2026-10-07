//! The site's three languages, the strings in the chrome that depend on them,
//! and the URLs they are addressed by.
//!
//! Topcoat 0.10 has no localization support — it is on the project's roadmap,
//! not in the crate — so this module is homegrown: an enum of languages, an
//! enum of message keys, and one `locales/<lang>.toml` per language, read **at
//! runtime** by [`catalog`].
//!
//! The enum is the *list* of keys and nothing else. Adding a key is a code
//! change; translating one is not, and that split is the point of this module.
//! The Tahitian is a first pass waiting for a native speaker and the French is
//! the maintainer's, so the people who would correct a sentence are not the
//! people who run `cargo build` — the words live in files read once at startup,
//! and correcting one is an edit and a restart. No compiler, no toolchain on the
//! box that serves the site.
//!
//! A key with no translation used to be a **compile error**, and that is what
//! the files cost. The boot checks what the compiler used to: [`catalog::Catalog::load`]
//! refuses a file that names a key no variant owns, and refuses a `fr.toml` or
//! `en.toml` that is missing one. `ty.toml` is deliberately exempt — an
//! unfinished Tahitian catalog is this catalog's normal state, and a line that
//! is not there is served from English. See [`catalog`] for the lookup order and
//! where the files are found.
//!
//! # What is translated, and what is not
//!
//! Only the chrome and the labels around the content: the navigation, the 404,
//! and the fixed action labels on the home, index and song pages. The song
//! lyrics are the content and are never translated; neither is the French prose
//! the home page has carried since v3 (the hero, the three cards, the synopsis,
//! the teaching paragraph). Translating prose is writing, and a rewrite should
//! not put new words in the maintainer's mouth — that is a separate decision
//! from giving the chrome a language.
//!
//! The one deliberate omission inside the chrome is the footer sentence. It is
//! not a label: it is a single sentence with two links spliced into the middle
//! of it, and splitting it into catalog fragments would leave both languages
//! ungrammatical. It stays exactly as v3 wrote it, English words and all.
//!
//! The one place the chrome names a *language* is the switcher (step 23), and
//! its links are each language's own name rather than a translation of it —
//! [`Lang::name`] says why they are outside this catalog. The switcher's
//! container is a label like any other, so its accessible name is a key here.
//!
//! # Reo Tahiti, and English
//!
//! The Tahitian catalog is a first pass, written without a native speaker to
//! check it. The vocabulary is deliberately small and conservative — `hīmene`
//! (song), `fa'aea` (welcome/home), `parau hīmene` (lyrics, the phrase the
//! song page's own metadata already uses), `tāpiri` (to add) — and the strings
//! are short enough to correct in `locales/ty.toml` alone, which is now a file
//! a translator can open, not three `match` arms in this one. A native review is
//! the next action, not a prerequisite for the plumbing.
//!
//! The English catalog is a first pass too, and plainer than the other two on
//! purpose: the site's French is warm and idiomatic and its English has no
//! settled tone yet, so these are the shortest words that say the same thing.
//! Refining them is writing, and it is a separate decision from the plumbing.
//!
//! # Where the language comes from
//!
//! The **URL**, and nothing else. Every page exists at `/fr/…`, `/ty/…` and
//! `/en/…`; the bare URL serves [`Lang::DEFAULT`] and names the prefixed URL as
//! its canonical. That is the whole resolution.
//!
//! The rule an earlier version of this module broke: **one URL, one response**.
//! v4 negotiated the language from `?lang=`, then the cookie, then
//! `Accept-Language`, which made `/` answer differently to two readers of the
//! same address. Cloudflare would cache whichever language it saw first and
//! serve it to everyone. So the sniffing is gone: `?lang=` is a *redirect* to
//! the prefixed URL (`routes::language`), the cookie remembers that choice, and
//! neither reaches a page's rendering. `Accept-Language` is not read at all —
//! deliberately, and this module's tests pin it.

use topcoat::{
    context::{Cx, try_request_context},
    router::request::uri,
};

use crate::domain::song::SITE_URL;

pub mod catalog;

pub use catalog::{init, text};

/// The name of the cookie an explicit language choice is kept in.
pub const COOKIE: &str = "lang";

/// The languages the site speaks.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Lang {
    /// French — the chrome's own language, and the default.
    Fr,
    /// Tahitian, *Reo Tahiti*.
    Ty,
    /// English.
    En,
}

impl Lang {
    /// Every language, in the order the site presents them.
    ///
    /// Used to write the `hreflang` alternates, so the set of alternates and
    /// the set of languages are the same list and cannot drift apart.
    pub const ALL: [Lang; 3] = [Lang::Fr, Lang::Ty, Lang::En];

    /// The language the bare URL serves.
    ///
    /// Not negotiable, and not a preference: the bare URL is one address with
    /// one response, and this is it. See the module docs.
    pub const DEFAULT: Lang = Lang::Fr;

    /// The tag the language is named by: `html lang`, `hreflang`, `?lang=`, and
    /// the cookie's value.
    pub const fn code(self) -> &'static str {
        match self {
            Lang::Fr => "fr",
            Lang::Ty => "ty",
            Lang::En => "en",
        }
    }

    /// The URL prefix every one of this language's pages carries.
    ///
    /// The prefix *is* the address: `/fr/himene` and `/himene` are one page
    /// with one canonical URL, and it is the prefixed one.
    pub const fn prefix(self) -> &'static str {
        match self {
            Lang::Fr => "/fr",
            Lang::Ty => "/ty",
            Lang::En => "/en",
        }
    }

    /// The `og:locale` value: a tag with a region.
    ///
    /// `ty_PF` and `fr_FR` are v3's values. The region is the one the language
    /// is written *for*, not the region the reader is in — the site is Tahitian
    /// and Polynesian French, and it says so in its cards. `en_US` is the
    /// conventional default for a language with no region of its own; the
    /// spelling of the chrome is neutral.
    pub const fn og_locale(self) -> &'static str {
        match self {
            Lang::Fr => "fr_FR",
            Lang::Ty => "ty_PF",
            Lang::En => "en_US",
        }
    }

    /// The language's name **in its own words**: the switcher's link text.
    ///
    /// Deliberately outside the catalog. A language names itself, so
    /// `Français` is the same word on a French page and on an English one —
    /// translating it would be a claim about the reader rather than a name, and
    /// an English page that called the third language "Tahitian" would be
    /// spelling a language it does not speak. The three are the names the site's
    /// cards already carry.
    pub const fn name(self) -> &'static str {
        match self {
            Lang::Fr => "Français",
            Lang::Ty => "Reo Tahiti",
            Lang::En => "English",
        }
    }

    /// The languages that are not this one, in [`ALL`](Self::ALL)'s order.
    ///
    /// The `og:locale:alternate` set. Two languages would have allowed a single
    /// [`Lang`] here; three do not.
    pub const fn others(self) -> [Lang; 2] {
        match self {
            Lang::Fr => [Lang::Ty, Lang::En],
            Lang::Ty => [Lang::Fr, Lang::En],
            Lang::En => [Lang::Fr, Lang::Ty],
        }
    }

    /// Reads a language out of a tag: `fr`, `fr-FR`, `ty_PF`, `TY`.
    ///
    /// Only the primary subtag is examined — where a reader is changes nothing
    /// about which of the site's languages they should get — and anything the
    /// site does not speak is [`None`].
    pub fn from_tag(tag: &str) -> Option<Self> {
        let primary = tag.trim().split(['-', '_']).next()?.to_ascii_lowercase();

        Self::from_code(&primary)
    }

    /// Reads a language out of a URL segment: the `/ty` of `/ty/himene`.
    ///
    /// Exact, unlike [`from_tag`](Self::from_tag): a path segment is one of the
    /// three prefixes or it is not a language, and `ty-PF` is not a directory.
    pub fn from_code(segment: &str) -> Option<Self> {
        match segment {
            "fr" => Some(Lang::Fr),
            "ty" => Some(Lang::Ty),
            "en" => Some(Lang::En),
            _ => None,
        }
    }
}

/// The language a request is being served in, carried through an internal
/// rewrite by [`crate::routes::language`].
///
/// A newtype rather than a bare [`Lang`] because the request context is
/// type-keyed: a `Lang` in it would be one bare enum value with no way to say
/// what put it there.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Language(pub Lang);

/// The language the request asked for in its URL, if it named one.
///
/// The bare URL names none, and answers in [`Lang::DEFAULT`] — which is why
/// this is an `Option` and [`resolve`] is not.
pub fn requested(cx: &Cx) -> Option<Lang> {
    try_request_context::<Language>(cx).map(|carried| carried.0)
}

/// The language to serve this request in.
///
/// Cheap enough to call wherever it is needed and called from the layout and
/// from each page that has a label, rather than being threaded down as a
/// parameter. It reads one request-context value and nothing else: no query
/// string, no header, no cookie. See the module docs for why that is the whole
/// rule.
pub fn resolve(cx: &Cx) -> Lang {
    requested(cx).unwrap_or(Lang::DEFAULT)
}

/// `path` as it is addressed in `lang`: the prefix, then the path.
///
/// The root is the prefix alone — `/fr`, not `/fr/` — because that is the form
/// every link, canonical and sitemap entry uses, and a page with two spellings
/// is the thing this module exists to avoid.
pub fn at(lang: Lang, path: &str) -> String {
    match path {
        "/" => lang.prefix().to_owned(),
        path => format!("{}{path}", lang.prefix()),
    }
}

/// The absolute URL of `path` in `lang`.
pub fn url(lang: Lang, path: &str) -> String {
    format!("{SITE_URL}{}", at(lang, path))
}

/// The absolute URL of `path` with no language prefix — what `x-default` names.
pub fn absolute(path: &str) -> String {
    match path {
        "/" => SITE_URL.to_owned(),
        path => format!("{SITE_URL}{path}"),
    }
}

/// `path` as it is addressed in this request's language: every link the chrome
/// emits goes through here.
pub fn link(cx: &Cx, path: &str) -> String {
    at(resolve(cx), path)
}

/// The request's path, with the query string.
///
/// What [`crate::routes::language`] rewrites from and what it redirects to.
pub fn path_and_query(cx: &Cx) -> String {
    let uri = uri(cx);
    match uri.query() {
        Some(query) => format!("{}?{query}", uri.path()),
        None => uri.path().to_owned(),
    }
}

/// A string in the chrome, as opposed to the content it surrounds.
///
/// The list of them, and only the list: what each one *says* is in
/// `locales/<lang>.toml`, read at startup by [`catalog`]. A variant's own name,
/// in snake_case, is the line it is written on there — see [`Key::name`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Key {
    /// The header link to the front page.
    NavHome,
    /// The header link to the song index.
    NavSongs,
    /// The language switcher's accessible name — read aloud, never drawn.
    Language,
    /// The 404's heading.
    NotFoundTitle,
    /// The 404's explanation — the line under the heading, and the one the joke
    /// lives in. It has to be true of every way a reader reaches a 404 (a song
    /// that is missing or unpublished, a page of the index that is out of
    /// range, a multi-lyric selection with a hole, or a URL no route knows), so
    /// it names the two ways a page goes missing rather than this reader's
    /// particular mistake.
    NotFoundBody,
    /// The 404's button back to the front page.
    NotFoundCta,
    /// The song page's button to the create-song page.
    AddLyrics,
    /// The song index's heading.
    IndexTitle,
    /// The song index's first column.
    IndexColumnTitle,
    /// The song index's second column.
    IndexColumnArtist,
    /// What the song index says when there is nothing to list.
    IndexEmpty,
    /// The home page's hero button.
    HomeDiscover,
    /// The home page's closing button.
    HomeStart,
    /// The label of the create-song form's lyric field.
    FieldLyrics,
    /// The create-song form's submit button.
    Save,
    /// What the create-song form says when the domain rejected the song.
    SaveError,
    /// The create-song form's chip delete button, read aloud.
    RemoveArtist,
    /// The create-song form's custom-chord insert button, read aloud.
    AddChord,
    /// The song page's transposition control — its visible label and the
    /// group's accessible name.
    Transpose,
    /// The control's step-down link, read aloud.
    TransposeDown,
    /// The control's step-up link, read aloud.
    TransposeUp,
    /// The song page's auto-scroll control — the speed bar's own name, read
    /// aloud rather than drawn.
    Scroll,
    /// The word beside the speed bar: what the slider sets.
    ScrollSpeed,
    /// The auto-scroll button while the sheet is still.
    ScrollStart,
    /// The auto-scroll button while the sheet is moving.
    ///
    /// A stop rather than a pause in name only: pressing it again resumes from
    /// where the reader is, which is what a pause does, and "stop" is the word
    /// the two other catalogs already have for a control that is not moving.
    ScrollStop,
    /// The heading of `/himene/pluriel` — the page that reads several songs one
    /// after another, and the picker that builds the selection.
    BookTitle,
    /// What the picker asks for: tick the songs, then read them together.
    BookHint,
    /// The picker's submit button.
    BookRead,
    /// The index's link to the picker.
    ///
    /// A word of its own rather than [`BookTitle`](Self::BookTitle): a
    /// heading names a page, and a link has to say what following it does.
    BookOpen,
    /// The heading of `/recherche` — the search page, in both of its states.
    SearchTitle,
    /// The label of the search form's one field: what the box searches.
    SearchLabel,
    /// The search form's submit button.
    SearchSubmit,
    /// The results' songs section.
    SearchSongs,
    /// The results' artists section.
    SearchArtists,
    /// What a search says when neither half found anything.
    SearchEmpty,
    /// The heading and `<title>` of `/soutenir` — the page that says how to
    /// support the site.
    SupportTitle,
    /// The sentence under that heading: what the page is for. One line, and no
    /// claim about the site beyond being free of advertising.
    SupportIntro,
    /// The support page's copy control, while the address is where it was.
    SupportCopy,
    /// The same control, once the address has been copied.
    SupportCopied,
}

impl Key {
    /// Every key, for exhaustiveness checks in tests.
    pub const ALL: [Key; 39] = [
        Key::NavHome,
        Key::NavSongs,
        Key::Language,
        Key::NotFoundTitle,
        Key::NotFoundBody,
        Key::NotFoundCta,
        Key::AddLyrics,
        Key::IndexTitle,
        Key::IndexColumnTitle,
        Key::IndexColumnArtist,
        Key::IndexEmpty,
        Key::HomeDiscover,
        Key::HomeStart,
        Key::FieldLyrics,
        Key::Save,
        Key::SaveError,
        Key::RemoveArtist,
        Key::AddChord,
        Key::Transpose,
        Key::TransposeDown,
        Key::TransposeUp,
        Key::Scroll,
        Key::ScrollSpeed,
        Key::ScrollStart,
        Key::ScrollStop,
        Key::BookTitle,
        Key::BookHint,
        Key::BookRead,
        Key::BookOpen,
        Key::SearchTitle,
        Key::SearchLabel,
        Key::SearchSubmit,
        Key::SearchSongs,
        Key::SearchArtists,
        Key::SearchEmpty,
        Key::SupportTitle,
        Key::SupportIntro,
        Key::SupportCopy,
        Key::SupportCopied,
    ];

    /// The key's name: the line it is spelled by in `locales/*.toml`, and the
    /// name it is reported by in a test.
    ///
    /// snake_case, and one string for the life of the program — a name is not a
    /// translation. It is also the catalog's last resort: a key that no file
    /// carries is served as its own name, which is a bug a reader can report,
    /// rather than a blank a reader cannot.
    pub const fn name(self) -> &'static str {
        match self {
            Key::NavHome => "nav_home",
            Key::NavSongs => "nav_songs",
            Key::Language => "language",
            Key::NotFoundTitle => "not_found_title",
            Key::NotFoundBody => "not_found_body",
            Key::NotFoundCta => "not_found_cta",
            Key::AddLyrics => "add_lyrics",
            Key::IndexTitle => "index_title",
            Key::IndexColumnTitle => "index_column_title",
            Key::IndexColumnArtist => "index_column_artist",
            Key::IndexEmpty => "index_empty",
            Key::HomeDiscover => "home_discover",
            Key::HomeStart => "home_start",
            Key::FieldLyrics => "field_lyrics",
            Key::Save => "save",
            Key::SaveError => "save_error",
            Key::RemoveArtist => "remove_artist",
            Key::AddChord => "add_chord",
            Key::Transpose => "transpose",
            Key::TransposeDown => "transpose_down",
            Key::TransposeUp => "transpose_up",
            Key::Scroll => "scroll",
            Key::ScrollSpeed => "scroll_speed",
            Key::ScrollStart => "scroll_start",
            Key::ScrollStop => "scroll_stop",
            Key::BookTitle => "book_title",
            Key::BookHint => "book_hint",
            Key::BookRead => "book_read",
            Key::BookOpen => "book_open",
            Key::SearchTitle => "search_title",
            Key::SearchLabel => "search_label",
            Key::SearchSubmit => "search_submit",
            Key::SearchSongs => "search_songs",
            Key::SearchArtists => "search_artists",
            Key::SearchEmpty => "search_empty",
            Key::SupportTitle => "support_title",
            Key::SupportIntro => "support_intro",
            Key::SupportCopy => "support_copy",
            Key::SupportCopied => "support_copied",
        }
    }

    /// The key that `name` names, or nothing when no key owns it.
    ///
    /// How a file is checked against the enum: a line the catalog cannot place
    /// is a typo, and the boot refuses it rather than quietly translating
    /// nothing.
    pub fn from_name(name: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|key| key.name() == name)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every key has a string in every language, and no two languages say the
    /// same thing.
    ///
    /// The completeness half is what the boot also checks for the two files the
    /// lookup falls back to (`fr.toml`, `en.toml` — see
    /// [`catalog::Catalog::load`]); this test is the shipped-files half of it,
    /// and reads the words through the same runtime catalog a page does. The
    /// difference half is what catches a key that was given its French words and
    /// a placeholder equal to them.
    #[test]
    fn every_key_is_translated_and_actually_differs() {
        for key in Key::ALL {
            for lang in Lang::ALL {
                assert!(
                    !text(lang, key).trim().is_empty(),
                    "{key:?} has no words in {}",
                    lang.code()
                );
            }

            for (first, second) in [
                (Lang::Fr, Lang::Ty),
                (Lang::Fr, Lang::En),
                (Lang::Ty, Lang::En),
            ] {
                assert_ne!(
                    text(first, key),
                    text(second, key),
                    "{key:?} says the same thing in {} and {}",
                    first.code(),
                    second.code()
                );
            }
        }
    }

    /// A name is the line a key is written on in `locales/*.toml`: one per key,
    /// snake_case, and stable. A duplicate would silently shadow a key in every
    /// file, and a name the file does not spell would make the key unfindable.
    #[test]
    fn every_key_is_named_once_and_in_snake_case() {
        let mut names = std::collections::BTreeSet::new();

        for key in Key::ALL {
            let name = key.name();

            assert!(
                !name.is_empty()
                    && name
                        .chars()
                        .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_'),
                "{key:?} is named {name:?}, which is not a snake_case key"
            );
            assert!(names.insert(name), "{name:?} names two keys");
        }

        assert_eq!(names.len(), Key::ALL.len());

        for key in Key::ALL {
            assert_eq!(Key::from_name(key.name()), Some(key), "{key:?}");
        }
        assert_eq!(Key::from_name("navhome"), None);
        assert_eq!(Key::from_name(""), None);
    }

    /// The words a file carries are the words a page says — accents, em dashes
    /// and `'okina` included. The Tahitian line is the one that would break
    /// first if the catalog mishandled UTF-8 or TOML's escapes.
    #[test]
    fn the_files_hand_back_their_diacritics_byte_for_byte() {
        const PINNED: [(Lang, Key, &str); 8] = [
            (Lang::Fr, Key::NavHome, "Accueil"),
            (Lang::Fr, Key::IndexTitle, "Toutes les chansons"),
            (Lang::Ty, Key::NavSongs, "Hīmene"),
            (Lang::Ty, Key::NavHome, "Fa'aea"),
            (Lang::Ty, Key::IndexColumnArtist, "Ta'ata hīmene"),
            (Lang::Ty, Key::SupportCopy, "Rave"),
            (Lang::En, Key::NavSongs, "Songs"),
            (Lang::En, Key::HomeStart, "Let's go!"),
        ];

        for (lang, key, words) in PINNED {
            assert_eq!(text(lang, key), words, "{} {key:?}", lang.code());
        }
    }

    /// **The French chrome, frozen at step 21.** Step 23 added the English
    /// strings and the language switcher, step 24 the transposition control,
    /// step 25 the auto-scroll control and step 26 the multi-lyric page; none of
    /// them may put a different word in a French page than the one step 21
    /// shipped, and a translation that quietly "improved" the French is exactly
    /// the change this table catches.
    ///
    /// Step 28 is the one deliberate exception: the 404's body was rewritten on
    /// purpose, so it moves out of this table and is pinned in its own, in all
    /// three languages. Everything else here is still frozen at step 21.
    ///
    /// The catalog is allowed to grow — a new control's own label is new chrome,
    /// not a new translation of old chrome — so the table also pins *which* keys
    /// were added rather than only the words that were not.
    ///
    /// Step 31 moved the words out of three `match` arms and into
    /// `locales/fr.toml`, without changing one of them: this table is what says
    /// so, and it is the only reason the move is checkable at all. A French page
    /// is byte-for-byte the page step 30b served.
    #[test]
    fn adding_chrome_did_not_change_a_word_of_french() {
        const FRENCH_AT_STEP_21: [(Key, &str); 16] = [
            (Key::NavHome, "Accueil"),
            (Key::NavSongs, "Chanson"),
            (Key::NotFoundTitle, "La page n'existe pas."),
            (Key::NotFoundCta, "Retour à l'accueil"),
            (Key::AddLyrics, "Ajouter des paroles"),
            (Key::IndexTitle, "Toutes les chansons"),
            (Key::IndexColumnTitle, "Titre"),
            (Key::IndexColumnArtist, "Artiste"),
            (Key::IndexEmpty, "Pas de chanson"),
            (Key::HomeDiscover, "Découvrir les chansons"),
            (Key::HomeStart, "C'est parti !"),
            (Key::FieldLyrics, "Paroles"),
            (Key::Save, "Enregistrer"),
            (
                Key::SaveError,
                "La chanson n'a pas été enregistrée. Vérifiez le titre et les paroles.",
            ),
            (Key::RemoveArtist, "Retirer cet artiste"),
            (Key::AddChord, "Ajouter cet accord"),
        ];

        for (key, words) in FRENCH_AT_STEP_21 {
            assert_eq!(text(Lang::Fr, key), words, "{key:?} moved");
        }

        // Twenty-two keys have been added since: the switcher's own label (step
        // 23), the transposition control's label and its two step links (step
        // 24), the auto-scroll control's four words (step 25), the multi-lyric
        // page's four (step 26), the search page's six (step 29), and the
        // support page's four (step 30). Every one of them is a new control or a
        // new page, not a reworded old line.
        const ADDED_SINCE: [Key; 22] = [
            Key::Language,
            Key::Transpose,
            Key::TransposeDown,
            Key::TransposeUp,
            Key::Scroll,
            Key::ScrollSpeed,
            Key::ScrollStart,
            Key::ScrollStop,
            Key::BookTitle,
            Key::BookHint,
            Key::BookRead,
            Key::BookOpen,
            Key::SearchTitle,
            Key::SearchLabel,
            Key::SearchSubmit,
            Key::SearchSongs,
            Key::SearchArtists,
            Key::SearchEmpty,
            Key::SupportTitle,
            Key::SupportIntro,
            Key::SupportCopy,
            Key::SupportCopied,
        ];

        // Step 28's own work, and the only French in this test that is not
        // step 21's: the 404's body, the one line the "funny 404" step rewrote
        // on purpose. Pinned in all three languages because the joke is the
        // deliverable — a later run may move it, but only deliberately.
        const NOT_FOUND_AT_STEP_28: [(Lang, &str); 3] = [
            (
                Lang::Fr,
                "Rien à lire ici — l'adresse s'est peut-être perdue en chemin, ou la chanson a pris le large.",
            ),
            (
                Lang::Ty,
                "'Aita e mea e hi'o i reira — ua reva paha te hīmene i te moana.",
            ),
            (
                Lang::En,
                "Nothing to read here — the address may have lost its way, or the song has gone to sea.",
            ),
        ];

        for (lang, words) in NOT_FOUND_AT_STEP_28 {
            assert_eq!(text(lang, Key::NotFoundBody), words, "the 404 moved");
        }

        // The 404's body is one key pinned in every language, so it counts once
        // towards the catalog even though the table has one row per language.
        assert_eq!(NOT_FOUND_AT_STEP_28.len(), Lang::ALL.len());
        assert_eq!(
            Key::ALL.len(),
            FRENCH_AT_STEP_21.len() + ADDED_SINCE.len() + 1
        );
        for added in ADDED_SINCE {
            assert!(
                !FRENCH_AT_STEP_21.iter().any(|(key, _)| *key == added),
                "{added:?} is not new chrome"
            );
        }
    }

    /// A language's own name is not a translation: it is one word per language,
    /// the same on every page, and that is why it is not in the catalog.
    #[test]
    fn a_language_names_itself_in_its_own_words() {
        let names: Vec<&str> = Lang::ALL.iter().map(|lang| lang.name()).collect();

        assert_eq!(names, ["Français", "Reo Tahiti", "English"]);
        assert_eq!(
            names
                .iter()
                .collect::<std::collections::BTreeSet<_>>()
                .len(),
            names.len(),
            "two languages answer to the same name: {names:?}"
        );
    }

    /// The tag reader accepts what the site writes and rejects what it does
    /// not speak — including the region forms a browser sends, and case, which
    /// `Accept-Language` has been known to vary.
    #[test]
    fn a_tag_names_a_language_or_nothing() {
        for (tag, expected) in [
            ("fr", Some(Lang::Fr)),
            ("fr-FR", Some(Lang::Fr)),
            ("fr_FR", Some(Lang::Fr)),
            ("FR", Some(Lang::Fr)),
            (" ty_PF ", Some(Lang::Ty)),
            ("ty", Some(Lang::Ty)),
            ("en", Some(Lang::En)),
            ("en-US", Some(Lang::En)),
            ("EN", Some(Lang::En)),
            ("pt", None),
            ("", None),
            ("-", None),
        ] {
            assert_eq!(Lang::from_tag(tag), expected, "tag {tag:?}");
        }
    }

    /// A URL segment is exact: the three prefixes, or nothing. `ty-PF` is not a
    /// directory, and neither is a song slug that happens to look like one.
    #[test]
    fn a_path_segment_names_one_of_the_three_or_nothing() {
        for (segment, expected) in [
            ("fr", Some(Lang::Fr)),
            ("ty", Some(Lang::Ty)),
            ("en", Some(Lang::En)),
            ("FR", None),
            ("ty-PF", None),
            ("ahani-e", None),
            ("", None),
        ] {
            assert_eq!(Lang::from_code(segment), expected, "segment {segment:?}");
        }
    }

    /// The other languages are exactly the ones not chosen, and every language
    /// is in [`Lang::ALL`] — the list the layout writes `hreflang` from.
    #[test]
    fn the_others_are_the_two_not_chosen() {
        assert_eq!(Lang::Fr.others(), [Lang::Ty, Lang::En]);
        assert_eq!(Lang::Ty.others(), [Lang::Fr, Lang::En]);
        assert_eq!(Lang::En.others(), [Lang::Fr, Lang::Ty]);
        assert_eq!(Lang::ALL.len(), 3);

        for lang in Lang::ALL {
            assert!(!lang.others().contains(&lang));
            assert_eq!(lang.others().len(), Lang::ALL.len() - 1);
        }
    }

    /// The default has to be *in* the list, or the bare URL would be a language
    /// with no canonical URL and no `hreflang`.
    #[test]
    fn the_default_language_is_one_of_them() {
        assert!(Lang::ALL.contains(&Lang::DEFAULT));
    }

    /// The address of a page: the prefix, then the path — and the root is the
    /// prefix alone, which is the form every link and sitemap entry uses.
    #[test]
    fn a_page_in_a_language_is_addressed_by_its_prefix() {
        assert_eq!(at(Lang::Fr, "/"), "/fr");
        assert_eq!(at(Lang::Ty, "/"), "/ty");
        assert_eq!(at(Lang::En, "/himene"), "/en/himene");
        assert_eq!(at(Lang::Ty, "/himene/ahani-e"), "/ty/himene/ahani-e");
        assert_eq!(at(Lang::Fr, "/aepa"), "/fr/aepa");

        assert_eq!(url(Lang::Ty, "/himene"), format!("{SITE_URL}/ty/himene"));
        assert_eq!(absolute("/"), SITE_URL);
        assert_eq!(absolute("/himene"), format!("{SITE_URL}/himene"));
    }

    /// No language's prefix is a prefix of another's, so stripping one can
    /// never leave another behind — the property `routes::language` relies on.
    #[test]
    fn no_prefix_is_a_prefix_of_another() {
        for first in Lang::ALL {
            for second in Lang::ALL {
                if first != second {
                    assert!(
                        !second.prefix().starts_with(first.prefix()),
                        "{} contains {}",
                        second.prefix(),
                        first.prefix()
                    );
                }
            }
        }
    }
}
