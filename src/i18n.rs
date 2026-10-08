//! The site's three languages, the strings in the chrome that depend on them,
//! and **where a request's language comes from**.
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
//! The chrome and the labels around the content — the navigation, the 404, the
//! fixed action labels on the home, index and song pages — **and, since v4.2,
//! the site's own prose**: the front page's sentences, the footer's, and the
//! `<meta name="description">` each page hands a search engine. A page served in
//! English says everything the site writes in English; the reviewer's "I read
//! the descriptions are in french" is what that answers.
//!
//! The prose is translated once, into English. Its Tahitian is the English, by
//! the same rule — "if don't know how to translate in tahitian, just rely on
//! english version" — so none of it has a line in `locales/ty.toml` and the
//! catalog's `ty → en → fr` order does the rest: see [`Key::PROSE`], which is the
//! list and the assertion.
//!
//! What is *not* translated: the song lyrics, which are the content; a proper
//! name, which is not a translation unit — the site's own name, `Facebook`, the
//! four chains on the support page; and the catalogue's own words as they are
//! stored, which the Markdown and JSON forms hand over unchanged.
//!
//! The one place the chrome names a *language* is the switcher, and its links
//! are each language's own name rather than a translation of it —
//! [`Lang::name`] says why they are outside this catalog. The switcher's
//! container is a label like any other, so its accessible name is a key here.
//!
//! # Reo Tahiti, and English
//!
//! The Tahitian catalog is a first pass, written without a native speaker to
//! check it. The vocabulary is deliberately small and conservative — `hīmene`
//! (song), `fa'aea` (welcome/home), `parau hīmene` (lyrics, the phrase the
//! song page's own metadata already uses), `tāpiri` (to add) — and the strings
//! are short enough to correct in `locales/ty.toml` alone, which is a file a
//! translator can open, not three `match` arms in this one. A native review is
//! the next action, not a prerequisite for the plumbing.
//!
//! The English catalog is a first pass too, and plainer than the other two on
//! purpose: the site's French is warm and idiomatic and its English has no
//! settled tone yet, so these are the shortest words that say the same thing.
//! Refining them is writing, and it is a separate decision from the plumbing.
//!
//! # Where the language comes from
//!
//! **One URL per page.** A page has exactly one address — the Tahitian one the
//! reviewer named — and no language is spelled in it. `/fr/himene`, `/ty/himene`
//! and `/en/himene` are not three pages; they were, until v4.2 retired them for
//! good (see [`crate::routes::language`]), and `/himene?lang=fr` is retired with
//! them.
//!
//! The language a response is *written in* is resolved, once, in this order:
//!
//! 1. **the `lang` cookie** — a reader who has chosen a language keeps it;
//! 2. **`Accept-Language`** — "default is the system", so a browser that has
//!    said which language it reads gets it;
//! 3. **[`Lang::DEFAULT`]** (French), when neither names one of the site's
//!    languages.
//!
//! So the same URL can answer two readers in two languages, and **every page
//! therefore says `Vary: Cookie, Accept-Language`** (set by
//! [`crate::routes::language::LanguageLayer`]). That is the trade the scope
//! names explicitly: a shared cache must key on the cookie and the header, and
//! Cloudflare is configured to. An earlier version of this module refused to
//! read either, to make the bare URL cacheable by everybody; the review chose
//! "default is the system" over that, and the decision is recorded rather than
//! silently reversed.

use topcoat::{
    context::Cx,
    router::{
        header,
        request::{headers, uri},
    },
};

use crate::domain::song::SITE_URL;

pub mod catalog;

pub use catalog::{init, text};

/// The name of the cookie a reader's language choice is kept in.
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
    /// The switcher is built from this list and so is the `og:locale` pair;
    /// there is no second list to drift from it.
    pub const ALL: [Lang; 3] = [Lang::Fr, Lang::Ty, Lang::En];

    /// The language a request is served in when it names none.
    ///
    /// The last of the three resolutions — after the cookie and after
    /// `Accept-Language` — and the one a client that says nothing gets. See the
    /// module docs.
    pub const DEFAULT: Lang = Lang::Fr;

    /// The tag the language is named by: `html lang`, `hreflang`, the
    /// `Accept-Language` tag it matches, and the cookie's value.
    pub const fn code(self) -> &'static str {
        match self {
            Lang::Fr => "fr",
            Lang::Ty => "ty",
            Lang::En => "en",
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
    /// The `og:locale:alternate` set — the languages the *document* is
    /// available in, which is what the Open Graph property says. (It is not a
    /// list of URLs: a page has one address now, and the alternate locales are
    /// the languages the same page can be served in.)
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
    /// site does not speak is [`None`]. Used for the cookie's value and for each
    /// tag of `Accept-Language`.
    pub fn from_tag(tag: &str) -> Option<Self> {
        match tag
            .trim()
            .split(['-', '_'])
            .next()?
            .to_ascii_lowercase()
            .as_str()
        {
            "fr" => Some(Lang::Fr),
            "ty" => Some(Lang::Ty),
            "en" => Some(Lang::En),
            _ => None,
        }
    }
}

/// The language to serve this request in: cookie → `Accept-Language` → `fr`.
///
/// Read from the request's own headers, once, wherever a page or a Markdown
/// document needs a word of chrome. The order is the whole rule; see the module
/// docs for why it is this one and what it costs.
pub fn resolve(cx: &Cx) -> Lang {
    cookie(cx).or_else(|| accepted(cx)).unwrap_or(Lang::DEFAULT)
}

/// The language the request's `lang` cookie names, if it names one.
fn cookie(cx: &Cx) -> Option<Lang> {
    let header = headers(cx).get(header::COOKIE)?.to_str().ok()?;

    from_cookie(header)
}

/// The language the request's `Accept-Language` header prefers, if it prefers
/// one the site speaks.
fn accepted(cx: &Cx) -> Option<Lang> {
    let header = headers(cx).get(header::ACCEPT_LANGUAGE)?.to_str().ok()?;

    from_accept_language(header)
}

/// A `Cookie` header's value for [`COOKIE`], read by hand.
///
/// The jar is not registered with the router (see `crate::router`): this module
/// reads the one cookie it needs out of the header, so there is no second place
/// a cookie's value could come from.
fn from_cookie(cookies: &str) -> Option<Lang> {
    cookies.split(';').find_map(|pair| {
        let (name, value) = pair.split_once('=')?;
        (name.trim() == COOKIE)
            .then(|| Lang::from_tag(value.trim()))
            .flatten()
    })
}

/// The best of `Accept-Language`'s tags that the site speaks.
///
/// A hand-rolled reader, like the other query/header readers in this crate: the
/// header is a list of `tag;q=weight` pairs, the weights order the reader's
/// preferences, and `*` means "anything". Ties keep the order the client wrote
/// them in, which is what a client that sends equal weights means.
fn from_accept_language(header: &str) -> Option<Lang> {
    let mut ranked: Vec<(f32, usize, Lang)> = Vec::new();

    for (order, part) in header.split(',').enumerate() {
        let mut fields = part.split(';');
        let tag = fields.next().unwrap_or_default().trim();
        let weight = fields
            .find_map(|field| field.trim().strip_prefix("q="))
            .and_then(|value| value.trim().parse::<f32>().ok())
            .unwrap_or(1.0);

        if weight > 0.0
            && let Some(lang) = Lang::from_tag(tag)
        {
            ranked.push((weight, order, lang));
        }
    }

    ranked
        .into_iter()
        .max_by(|left, right| {
            // Higher weight first; on a tie the earlier tag. `total_cmp` rather
            // than `partial_cmp` because a `q` of `NaN` parses and must not
            // panic the site.
            left.0.total_cmp(&right.0).then(right.1.cmp(&left.1))
        })
        .map(|(_, _, lang)| lang)
}

/// The absolute URL of `path`, on the canonical host.
///
/// A page has one address, so this is the whole of "the URL of a page": the
/// scheme and host, then the path. The root is the host alone, the form every
/// canonical URL, `og:url` and sitemap entry uses.
pub fn absolute(path: &str) -> String {
    match path {
        "/" => SITE_URL.to_owned(),
        path => format!("{SITE_URL}{path}"),
    }
}

/// The request's path, with the query string.
///
/// What the language switcher carries as the page to come back to: the
/// switcher's own address is not a page, so it cannot be reached and left the
/// way the languages used to be — the page has to be folded into the link.
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
    /// The heading of `/puta-himene` — the page that reads several songs one
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
    /// The heading of `/paimi` — the search page, in both of its states.
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
    /// The heading and `<title>` of `/tauturu` — the page that says how to
    /// support the site.
    SupportTitle,
    /// The sentence under that heading: what the page is for. One line, and no
    /// claim about the site beyond being free of advertising.
    SupportIntro,
    /// The support page's copy control, while the address is where it was.
    SupportCopy,
    /// The same control, once the address has been copied.
    SupportCopied,
    /// The front page's standfirst, under the site's name.
    HomeHeroSubtitle,
    /// The first card's heading — the songs.
    HomeCardSongsTitle,
    /// The first card's body.
    HomeCardSongsBody,
    /// The second card's heading — the lyrics.
    HomeCardLyricsTitle,
    /// The second card's body.
    HomeCardLyricsBody,
    /// The third card's heading — the chords.
    HomeCardChordsTitle,
    /// The third card's body.
    HomeCardChordsBody,
    /// The front page's long paragraph, between the cards and the two tables.
    HomeSynopsis,
    /// The heading over the newest songs.
    HomeTableLatest,
    /// The heading over the most-read songs.
    HomeTableMostViewed,
    /// The heading of the closing block.
    HomeFootTitle,
    /// The line under it.
    HomeFootText,
    /// The tail of the front page's `<meta name="description">` — the phrase
    /// that is not on the page, and the one that says what the site is.
    HomeTagline,
    /// The footer's first clause: the site, the year, and the rights.
    FooterRights,
    /// The footer's second clause — the link to the project on GitHub.
    ///
    /// A clause rather than the words around a link: the sentence has two links
    /// in it, and the whole of each clause is one link's text, so a translator
    /// writes two readable sentences instead of four fragments that have to fit
    /// between them.
    FooterContributing,
    /// The word beside the footer's link to the support page — the icon's text.
    ///
    /// The reviewer's own three: `don` in French, `donate` in English,
    /// `tautururaa` in Tahitian. Chrome and not prose, which is why it carries a
    /// `ty.toml` line: he gave the Tahitian himself, so the fallback rule that
    /// applies to prose does not apply to it (see [`Key::PROSE`]).
    FooterDonate,
    /// The song index's `<meta name="description">`.
    IndexDescription,
    /// The tail of an artist page's `<meta name="description">` — the artist's
    /// name goes in front of it.
    ArtistDescription,
    /// The support page's `<meta name="description">`.
    SupportDescription,
}

impl Key {
    /// Every key, for exhaustiveness checks in tests.
    pub const ALL: [Key; 58] = [
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
        Key::HomeHeroSubtitle,
        Key::HomeCardSongsTitle,
        Key::HomeCardSongsBody,
        Key::HomeCardLyricsTitle,
        Key::HomeCardLyricsBody,
        Key::HomeCardChordsTitle,
        Key::HomeCardChordsBody,
        Key::HomeSynopsis,
        Key::HomeTableLatest,
        Key::HomeTableMostViewed,
        Key::HomeFootTitle,
        Key::HomeFootText,
        Key::HomeTagline,
        Key::FooterRights,
        Key::FooterContributing,
        Key::FooterDonate,
        Key::IndexDescription,
        Key::ArtistDescription,
        Key::SupportDescription,
    ];

    /// The keys whose Tahitian is deliberately the English.
    ///
    /// Every other key is the chrome, and the chrome is a first pass written in
    /// all three languages. These are the maintainer's own prose — the front
    /// page's sentences, the footer's, and the descriptions a search engine
    /// reads — and the reviewer's rule for prose is the other way round: **where
    /// there are no Tahitian words, serve the English**. So none of them has a
    /// line in `locales/ty.toml`, the catalog's `ty → en → fr` order hands back
    /// the English, and `every_key_is_translated_and_actually_differs` asserts
    /// exactly that rather than treating the fallback as a missing translation.
    ///
    /// A line added to `ty.toml` for one of these is served like any other and
    /// has to be removed from this list — which is what makes the list a claim
    /// about the shipped files rather than a switch.
    pub const PROSE: [Key; 18] = [
        Key::HomeHeroSubtitle,
        Key::HomeCardSongsTitle,
        Key::HomeCardSongsBody,
        Key::HomeCardLyricsTitle,
        Key::HomeCardLyricsBody,
        Key::HomeCardChordsTitle,
        Key::HomeCardChordsBody,
        Key::HomeSynopsis,
        Key::HomeTableLatest,
        Key::HomeTableMostViewed,
        Key::HomeFootTitle,
        Key::HomeFootText,
        Key::HomeTagline,
        Key::FooterRights,
        Key::FooterContributing,
        Key::IndexDescription,
        Key::ArtistDescription,
        Key::SupportDescription,
    ];

    /// Whether this key's Tahitian is the English (see [`Key::PROSE`]).
    pub fn falls_back_to_english(self) -> bool {
        Self::PROSE.contains(&self)
    }

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
            Key::HomeHeroSubtitle => "home_hero_subtitle",
            Key::HomeCardSongsTitle => "home_card_songs_title",
            Key::HomeCardSongsBody => "home_card_songs_body",
            Key::HomeCardLyricsTitle => "home_card_lyrics_title",
            Key::HomeCardLyricsBody => "home_card_lyrics_body",
            Key::HomeCardChordsTitle => "home_card_chords_title",
            Key::HomeCardChordsBody => "home_card_chords_body",
            Key::HomeSynopsis => "home_synopsis",
            Key::HomeTableLatest => "home_table_latest",
            Key::HomeTableMostViewed => "home_table_most_viewed",
            Key::HomeFootTitle => "home_foot_title",
            Key::HomeFootText => "home_foot_text",
            Key::HomeTagline => "home_tagline",
            Key::FooterRights => "footer_rights",
            Key::FooterContributing => "footer_contributing",
            Key::FooterDonate => "footer_donate",
            Key::IndexDescription => "index_description",
            Key::ArtistDescription => "artist_description",
            Key::SupportDescription => "support_description",
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
    ///
    /// A [`Key::PROSE`] key is the deliberate exception on the Tahitian side: its
    /// Tahitian *is* the English, and asserting that is stronger than letting it
    /// fall outside the table — a prose key that quietly grew a `ty.toml` line
    /// would fail here, and that is the moment to move it out of [`Key::PROSE`].
    #[test]
    fn every_key_is_translated_and_actually_differs() {
        for key in Key::ALL {
            for lang in [Lang::Fr, Lang::En] {
                assert!(
                    !text(lang, key).trim().is_empty(),
                    "{key:?} has no words in {}",
                    lang.code()
                );
            }

            if key.falls_back_to_english() {
                assert_eq!(
                    text(Lang::Ty, key),
                    text(Lang::En, key),
                    "{key:?} is prose with words of its own in ty.toml — take it \
                     out of Key::PROSE"
                );
                assert_ne!(
                    text(Lang::Fr, key),
                    text(Lang::En, key),
                    "{key:?} says the same thing in French and in English"
                );
                continue;
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

    /// The prose list is a list of keys, and every one of them is a key: a name
    /// that no variant owns would make the fallback assertion above vacuous.
    #[test]
    fn the_prose_keys_are_keys_and_are_listed_once() {
        let mut names = std::collections::BTreeSet::new();

        for key in Key::PROSE {
            assert!(Key::ALL.contains(&key), "{key:?} is not a key");
            assert!(names.insert(key.name()), "{key:?} is listed twice");
        }

        assert_eq!(names.len(), Key::PROSE.len());
        assert!(
            Key::PROSE.len() < Key::ALL.len(),
            "every key cannot be prose"
        );
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
    /// so, and it is the only reason the move is checkable at all.
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

        // Step 38's one: the footer's link to the support page, which the
        // reviewer asked for as an icon and a word. Chrome, not prose — he gave
        // the Tahitian himself (`tautururaa`), so it is written in all three
        // files instead of falling back to English like the prose does.
        const ADDED_AT_STEP_38: [Key; 1] = [Key::FooterDonate];

        // Step 36's eighteen: the same move, for the maintainer's own prose —
        // the front page's sentences, the footer's two clauses, and the
        // descriptions a search engine reads. Not chrome, and pinned separately
        // for that reason: the French of every one of them is byte-for-byte what
        // v3 wrote, and the English beside it is new writing (see `Key::PROSE`).

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

        // The prose is the third group and the last: step 36's eighteen keys,
        // separate from the chrome because their Tahitian is the English. Every
        // one of them is in `Key::PROSE`, which is what makes the French above a
        // statement about the shipped files and not about a table in a test.
        const PROSE_AT_STEP_36: [Key; 18] = [
            Key::HomeHeroSubtitle,
            Key::HomeCardSongsTitle,
            Key::HomeCardSongsBody,
            Key::HomeCardLyricsTitle,
            Key::HomeCardLyricsBody,
            Key::HomeCardChordsTitle,
            Key::HomeCardChordsBody,
            Key::HomeSynopsis,
            Key::HomeTableLatest,
            Key::HomeTableMostViewed,
            Key::HomeFootTitle,
            Key::HomeFootText,
            Key::HomeTagline,
            Key::FooterRights,
            Key::FooterContributing,
            Key::IndexDescription,
            Key::ArtistDescription,
            Key::SupportDescription,
        ];

        for key in PROSE_AT_STEP_36 {
            assert!(
                key.falls_back_to_english(),
                "{key:?} is prose but not listed as prose"
            );
        }
        assert_eq!(PROSE_AT_STEP_36.len(), Key::PROSE.len());
        assert_eq!(
            Key::ALL.len(),
            FRENCH_AT_STEP_21.len()
                + ADDED_SINCE.len()
                + ADDED_AT_STEP_38.len()
                + 1
                + PROSE_AT_STEP_36.len()
        );
        for added in ADDED_SINCE.into_iter().chain(ADDED_AT_STEP_38) {
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
            ("*", None),
        ] {
            assert_eq!(Lang::from_tag(tag), expected, "tag {tag:?}");
        }
    }

    /// The other languages are exactly the ones not chosen, and every language
    /// is in [`Lang::ALL`] — the list the switcher and the `og:locale` pair are
    /// written from.
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

    /// The default is one of the three, and it is the last resort: a client that
    /// says nothing gets it.
    #[test]
    fn the_default_language_is_one_of_them() {
        assert!(Lang::ALL.contains(&Lang::DEFAULT));
        assert_eq!(from_accept_language(""), None);
        assert_eq!(from_cookie(""), None);
    }

    /// The cookie is read by name and only by name: a header of other cookies
    /// does not name a language, and an unknown value falls through to
    /// `Accept-Language` rather than being invented an answer.
    #[test]
    fn the_cookie_names_a_language_or_nothing() {
        assert_eq!(from_cookie("lang=ty"), Some(Lang::Ty));
        assert_eq!(from_cookie("a=1; lang=en; b=2"), Some(Lang::En));
        assert_eq!(from_cookie("lang=TY"), Some(Lang::Ty));
        assert_eq!(from_cookie("lang=pt"), None);
        assert_eq!(from_cookie("language=ty"), None);
        assert_eq!(from_cookie("lang"), None);
        assert_eq!(from_cookie("a=1; b=2"), None);
    }

    /// `Accept-Language` is a ranked list: the heaviest tag the site speaks
    /// wins, `q=0` means "not this one", and a tag the site does not speak is
    /// skipped rather than ending the search.
    #[test]
    fn the_header_ranks_the_languages_it_speaks() {
        assert_eq!(from_accept_language("ty"), Some(Lang::Ty));
        assert_eq!(
            from_accept_language("pt, ty;q=0.9, en;q=0.8"),
            Some(Lang::Ty)
        );
        assert_eq!(from_accept_language("en;q=0.4, ty;q=0.9"), Some(Lang::Ty));
        assert_eq!(from_accept_language("fr-CA,fr;q=0.9"), Some(Lang::Fr));
        assert_eq!(
            from_accept_language("en-GB,en;q=0.9,en-US;q=0.8"),
            Some(Lang::En)
        );
        assert_eq!(
            from_accept_language("da, en-gb;q=0.8, en;q=0.7"),
            Some(Lang::En)
        );
        // A client that refuses every language it names has named nothing.
        assert_eq!(from_accept_language("fr;q=0, ty;q=0"), None);
        assert_eq!(from_accept_language("*"), None);
        assert_eq!(from_accept_language("pt-BR"), None);
        assert_eq!(from_accept_language(""), None);
        // Ties keep the order the client wrote.
        assert_eq!(from_accept_language("en, ty"), Some(Lang::En));
        assert_eq!(from_accept_language("ty, en"), Some(Lang::Ty));
    }

    /// A page's absolute URL is the host and its own path, and the root is the
    /// host alone — the form every canonical URL, card and sitemap entry uses.
    #[test]
    fn a_pages_url_is_its_own_address() {
        assert_eq!(
            absolute("/himene/ahani-e"),
            format!("{SITE_URL}/himene/ahani-e")
        );
        assert_eq!(absolute("/faariiraa"), format!("{SITE_URL}/faariiraa"));
        assert_eq!(absolute("/"), SITE_URL);
        assert_eq!(
            absolute("/puta-himene?s=a"),
            format!("{SITE_URL}/puta-himene?s=a")
        );
    }
}
