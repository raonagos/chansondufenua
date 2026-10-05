//! The site's two languages, and the strings in the chrome that depend on them.
//!
//! Topcoat 0.10 has no localization support — it is on the project's roadmap,
//! not in the crate — so this module is homegrown and dependency-free beyond
//! what the framework already exposes: an enum of languages, an enum of message
//! keys, and one exhaustive match per language. A key with no translation is a
//! **compile error** rather than a blank line on the page, which is the whole
//! reason the catalog is an enum and not a lookup map: the compiler checks what
//! a map would let rot silently. See `PLAN.md` §2.4.
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
//! # Reo Tahiti
//!
//! The Tahitian catalog is a first pass, written without a native speaker to
//! check it. The vocabulary is deliberately small and conservative — `hīmene`
//! (song), `fa'aea` (welcome/home), `parau hīmene` (lyrics, the phrase the
//! song page's own metadata already uses), `tāpiri` (to add) — and the strings
//! are short enough to correct in place without touching any code. A native
//! review is the next action, not a prerequisite for the plumbing.
//!
//! # Resolution
//!
//! §2.4's order: an explicit `?lang=` wins, then the `lang` cookie, then the
//! request's `Accept-Language`, then French. Only the two languages the site
//! speaks are ever selected: a reader whose browser asks for `en-US` gets
//! French, which is what the site's own chrome is written in.
//!
//! An explicit `?lang=` is written back as a cookie. Without that the choice
//! would last exactly one page, because no link in the chrome carries the
//! parameter — `?lang=ty` on `/` would be Tahitian, and the first click would
//! be French again.

use serde::Deserialize;
use topcoat::{
    context::Cx,
    cookie::{Cookie, Cookies, SameSite, cookies, time},
    router::{
        header, parse_query_params,
        request::{headers, uri},
    },
};

/// The name of the cookie an explicit language choice is kept in.
pub const COOKIE: &str = "lang";

/// The languages the site speaks.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Lang {
    /// French — the chrome's own language, and the default.
    Fr,
    /// Tahitian, *Reo Tahiti*.
    Ty,
}

impl Lang {
    /// Every language, in the order the site presents them.
    ///
    /// Used to write the `hreflang` alternates, so the set of alternates and
    /// the set of languages are the same list and cannot drift apart.
    pub const ALL: [Lang; 2] = [Lang::Fr, Lang::Ty];

    /// The tag the language is named by: `html lang`, `hreflang`, `?lang=`, and
    /// the cookie's value.
    pub const fn code(self) -> &'static str {
        match self {
            Lang::Fr => "fr",
            Lang::Ty => "ty",
        }
    }

    /// The `og:locale` value: a tag with a region.
    ///
    /// `ty_PF` and `fr_FR` are v3's values. The region is the one the language
    /// is written *for*, not the region the reader is in — the site is Tahitian
    /// and Polynesian French, and it says so in its cards.
    pub const fn og_locale(self) -> &'static str {
        match self {
            Lang::Fr => "fr_FR",
            Lang::Ty => "ty_PF",
        }
    }

    /// The other language — the `hreflang` alternate and `og:locale:alternate`.
    pub const fn other(self) -> Self {
        match self {
            Lang::Fr => Lang::Ty,
            Lang::Ty => Lang::Fr,
        }
    }

    /// Reads a language out of a tag: `fr`, `fr-FR`, `ty_PF`, `TY`.
    ///
    /// Only the primary subtag is examined — where a reader is changes nothing
    /// about which of the site's two languages they should get — and anything
    /// the site does not speak is [`None`], so that the next source in the
    /// resolution order is tried. `?lang=en` is therefore not an error: it just
    /// does not choose, exactly as `Accept-Language: en-US` would not.
    pub fn from_tag(tag: &str) -> Option<Self> {
        let primary = tag.trim().split(['-', '_']).next()?.to_ascii_lowercase();

        match primary.as_str() {
            "fr" => Some(Lang::Fr),
            "ty" => Some(Lang::Ty),
            _ => None,
        }
    }
}

/// A string in the chrome, as opposed to the content it surrounds.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Key {
    /// The header link to the front page.
    NavHome,
    /// The header link to the song index.
    NavSongs,
    /// The 404's heading.
    NotFoundTitle,
    /// The 404's explanation.
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
}

impl Key {
    /// Every key, for exhaustiveness checks in tests.
    pub const ALL: [Key; 17] = [
        Key::NavHome,
        Key::NavSongs,
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
    ];

    /// The French words. v3's, byte for byte.
    fn fr(self) -> &'static str {
        match self {
            Key::NavHome => "Accueil",
            Key::NavSongs => "Chanson",
            Key::NotFoundTitle => "La page n'existe pas.",
            Key::NotFoundBody => "Cette page n'existe pas, ou n'existe plus.",
            Key::NotFoundCta => "Retour à l'accueil",
            Key::AddLyrics => "Ajouter des paroles",
            Key::IndexTitle => "Toutes les chansons",
            Key::IndexColumnTitle => "Titre",
            Key::IndexColumnArtist => "Artiste",
            Key::IndexEmpty => "Pas de chanson",
            Key::HomeDiscover => "Découvrir les chansons",
            Key::HomeStart => "C'est parti !",
            Key::FieldLyrics => "Paroles",
            Key::Save => "Enregistrer",
            Key::SaveError => {
                "La chanson n'a pas été enregistrée. Vérifiez le titre et les paroles."
            }
            Key::RemoveArtist => "Retirer cet artiste",
            Key::AddChord => "Ajouter cet accord",
        }
    }

    /// The Tahitian words. See the module docs: a first pass, awaiting a native
    /// speaker's review, and short enough to correct here alone.
    fn ty(self) -> &'static str {
        match self {
            Key::NavHome => "Fa'aea",
            Key::NavSongs => "Hīmene",
            Key::NotFoundTitle => "'Aita te 'api",
            Key::NotFoundBody => "'Aita teie 'api e vai ra.",
            Key::NotFoundCta => "Ho'i i te fa'aea",
            Key::AddLyrics => "Tāpiri i te parau hīmene",
            Key::IndexTitle => "Te mau hīmene ato'a",
            Key::IndexColumnTitle => "I'oa",
            Key::IndexColumnArtist => "Ta'ata hīmene",
            Key::IndexEmpty => "'Aita hīmene",
            Key::HomeDiscover => "'Ite i te mau hīmene",
            Key::HomeStart => "Haere tātou!",
            Key::FieldLyrics => "Parau hīmene",
            Key::Save => "Tāpiri",
            Key::SaveError => "'Aita te hīmene i tāpiri. Hi'opoa i te i'oa e te parau hīmene.",
            Key::RemoveArtist => "Rave i teie ta'ata hīmene",
            Key::AddChord => "Tāpiri i teie accord",
        }
    }
}

/// The words for `key` in `lang`.
///
/// The two matches inside [`Key`] are over the whole key enum, so adding a key
/// is a compile error until both languages have a string for it. This function
/// is the only way to read the catalog.
pub fn text(lang: Lang, key: Key) -> &'static str {
    match lang {
        Lang::Fr => key.fr(),
        Lang::Ty => key.ty(),
    }
}

/// The language to serve this request in.
///
/// Cheap enough to call wherever it is needed — it parses a query string and
/// reads two headers — and called from the layout and from each page that has
/// a label, rather than being threaded down as a parameter. Nothing here
/// touches the database or the network.
pub fn resolve(cx: &Cx) -> Lang {
    let explicit = parse_query_params::<Query>(cx)
        .ok()
        .and_then(|query| query.lang.as_deref().and_then(Lang::from_tag));

    let jar = cookies(cx);
    let stored = jar
        .get(COOKIE)
        .and_then(|cookie| Lang::from_tag(cookie.value()));

    let accept = headers(cx)
        .get(header::ACCEPT_LANGUAGE)
        .and_then(|value| value.to_str().ok());

    let lang = explicit
        .or(stored)
        .or_else(|| accept.and_then(preferred))
        .unwrap_or(Lang::Fr);

    // Remember an explicit choice. Skipped when the cookie already says the
    // same thing, so clicking a `?lang=` link twice does not restate the
    // cookie, and a page that calls `resolve` more than once — the layout, the
    // header, the page — still writes it once. `Path=/` matters: without it a
    // choice made on `/himene/x` would not reach `/`.
    if let Some(chosen) = explicit
        && stored != Some(chosen)
    {
        jar.add(
            Cookie::build((COOKIE, chosen.code()))
                .path("/")
                .max_age(time::Duration::days(365))
                .same_site(SameSite::Lax)
                .build(),
        );
    }

    lang
}

/// The `lang` parameter, as `?lang=ty` writes it.
///
/// Deliberately a `String` and not a [`Lang`]: an unsupported tag has to be
/// distinguishable from an absent one, because only the latter is a reason to
/// consult the cookie. Both end up as "no language chosen", but the *first* is
/// also "do not write a cookie", which `Option<Lang>` could not say.
#[derive(Deserialize)]
struct Query {
    lang: Option<String>,
}

/// The best of the site's languages from an `Accept-Language` header.
///
/// Quality values are honoured (`ty;q=0.9` loses to `fr;q=1.0`), the leftmost
/// entry wins a tie, and anything with `q=0` — "explicitly not acceptable" —
/// is skipped. Entries the site cannot serve are skipped rather than scored, so
/// `en;q=1.0, ty;q=0.5` is Tahitian and not, as a naive first-match would say,
/// French.
fn preferred(accept: &str) -> Option<Lang> {
    let mut best: Option<(f32, Lang)> = None;

    for entry in accept.split(',') {
        let mut fields = entry.split(';');
        let Some(tag) = fields.next() else { continue };
        let Some(lang) = Lang::from_tag(tag) else {
            continue;
        };

        let quality = fields
            .find_map(|field| field.trim().strip_prefix("q="))
            .and_then(|value| value.trim().parse::<f32>().ok())
            .unwrap_or(1.0);

        if quality <= 0.0 {
            continue;
        }

        // Strictly greater, so an earlier entry keeps a tie.
        if best.is_none_or(|(best_quality, _)| quality > best_quality) {
            best = Some((quality, lang));
        }
    }

    best.map(|(_, lang)| lang)
}

/// The path the request asked for, without its query string.
///
/// The language alternates are built from this: every URL on the site exists in
/// both languages, and the parameter is the only thing that distinguishes them.
pub fn path(cx: &Cx) -> &str {
    uri(cx).path()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every key has a string in both languages, and the two are not the same
    /// string. The first half the compiler already guarantees through the two
    /// matches; the second half is what catches a key that was given its French
    /// words and a Tahitian placeholder equal to them.
    #[test]
    fn every_key_is_translated_and_actually_differs() {
        for key in Key::ALL {
            let fr = text(Lang::Fr, key);
            let ty = text(Lang::Ty, key);

            assert!(!fr.trim().is_empty(), "{key:?} has no French words");
            assert!(!ty.trim().is_empty(), "{key:?} has no Tahitian words");
            assert_ne!(fr, ty, "{key:?} says the same thing in both languages");
        }
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
            ("en", None),
            ("en-US", None),
            ("", None),
            ("-", None),
        ] {
            assert_eq!(Lang::from_tag(tag), expected, "tag {tag:?}");
        }
    }

    /// The alternate is a swap, and every language is in [`Lang::ALL`] — the
    /// list the layout writes `hreflang` from.
    #[test]
    fn the_other_language_is_the_one_not_chosen() {
        assert_eq!(Lang::Fr.other(), Lang::Ty);
        assert_eq!(Lang::Ty.other(), Lang::Fr);
        assert_eq!(Lang::ALL.len(), 2);
        assert!(Lang::ALL.contains(&Lang::Fr) && Lang::ALL.contains(&Lang::Ty));
    }

    /// Quality values decide, and an entry the site cannot serve is skipped
    /// rather than treated as a fallback — the difference between honouring
    /// `Accept-Language` and ignoring it.
    #[test]
    fn accept_language_is_ranked_by_quality() {
        assert_eq!(
            preferred("ty-PF,ty;q=0.9,fr;q=0.8,en;q=0.7"),
            Some(Lang::Ty)
        );
        assert_eq!(preferred("fr;q=0.8,ty;q=0.9"), Some(Lang::Ty));
        assert_eq!(preferred("en;q=1.0, ty;q=0.5"), Some(Lang::Ty));
        assert_eq!(preferred("en-US,en;q=0.9"), None);
        assert_eq!(preferred("fr,ty"), Some(Lang::Fr), "a tie goes leftmost");
        assert_eq!(
            preferred("ty;q=0,fr"),
            Some(Lang::Fr),
            "q=0 is not a choice"
        );
        assert_eq!(preferred(""), None);
    }
}
