//! The terms — `/te-mau-parau-tumu-e-te-ture`.
//!
//! The owner's own name for the page, and *te mau parau tumu e te ture* is what
//! it means: the truth accepted by all, and the law. What it carries is six
//! statements about the site and nothing else — who publishes it, whose the
//! lyrics are, what its code is licensed under, what is not kept, how a rights
//! holder gets a song taken down, and what to do about a lyric that is wrong.
//!
//! # Every sentence is the owner's, and none of them is mine
//!
//! This is the page on which that matters most: a false sentence here is a
//! promise about rights. So the words are his, in his own words, and they are
//! the scope's — "the site is hosted and edited by Rao Nagos", "the lyrics are
//! not ours", the two addresses a rights holder writes to, "no sensitive user
//! information is backed up on this site", and "we are not responsible if a
//! lyric is wrong" — with the two additions he invited: that the code is
//! GPL-3.0 ([`crate::domain::song::SITE_URL`]'s project, GPL-3.0 in `LICENSE`)
//! and that a correction is an invitation rather than a complaint procedure.
//!
//! Nothing else is claimed. The page does not say the site sets no cookie — it
//! sets one, the reader's own language ([`crate::i18n::COOKIE`]), which is not
//! the "sensitive user information" the statement is about — and it does not
//! describe a takedown *process*, because there is not one: there is an address
//! and a promise to act.
//!
//! # The head is `noindex, follow`, and there is no canonical
//!
//! The same shape [`crate::pages::book`] and [`crate::pages::search`] use, and
//! for the same reason: the page is not written to be found. A reader reaches it
//! from the footer, from a link in a rights conversation, and — the one search
//! engine that finds it — from the site's own name; an index entry is not what
//! it is for, and a canonical URL on a page a crawler is told not to index names
//! a preferred address for nothing. `follow` is the half that matters: the two
//! mail addresses and the site's own pages are reached from here.
//!
//! # In no sitemap, in no `llms.txt`
//!
//! Both are lists of what the site *offers*, and this page is offered to nobody:
//! it is reached from the footer on every page, which is where a reader who
//! wants it goes. `.run/step46.sh` asserts both halves against the served
//! documents rather than against this comment.
//!
//! Note the naming constraint every page in this directory shares:
//! `#[page("/te-mau-parau-tumu-e-te-ture")]` emits a unit struct named after its
//! handler — `terms` — in this module's *type* namespace, so a local binding of
//! that name would be read as a pattern matching it.

use topcoat::{
    Result,
    context::Cx,
    router::page,
    view::{View, view},
};

use crate::i18n::{self, Key, Lang};
use crate::ui::theme;

/// `/te-mau-parau-tumu-e-te-ture` — the address, in one place.
///
/// `routes::language` decides whether a request path is language-scoped, and
/// this is the constant it reads. `#[page]` cannot take one — it is a macro over
/// a literal path — so this restates it, as `pages::songs::PATH` does.
pub const PATH: &str = "/te-mau-parau-tumu-e-te-ture";

/// The day this page's words were last changed: the day it was written.
///
/// ISO, and deliberately not a translated date: `2026-10-08` reads the same in
/// all three languages, and a month name would be three more words to keep in
/// step with a fact that has none. The label in front of it is the catalog's
/// ([`Key::TermsUpdated`]), because a label is prose and prose follows the
/// reader's language.
///
/// It moves when a statement here moves, not when the site is deployed: it is a
/// claim about this page's text, which is the only thing a reader can check.
pub const UPDATED: &str = "2026-10-08";

/// The statements, in the order the page prints them.
///
/// One list rather than six `<p>` elements in the markup: the page's whole
/// content is a sequence of the owner's sentences, and a test that reads this
/// list is a test that knows what the page says. The order is the reading order
/// the page was written in — who publishes it, then whose the lyrics are, then
/// what happens when one of them is wrong — and not a priority.
pub const STATEMENTS: [Key; 6] = [
    Key::TermsPublisher,
    Key::TermsLyrics,
    Key::TermsCode,
    Key::TermsData,
    Key::TermsRemoval,
    Key::TermsCorrections,
];

/// The page's own name, in the reader's language.
///
/// One function for the three places that need it: the page's `<title>`, its
/// `<h1>`, and the footer's link to it. A page with two names is a page whose
/// link and heading disagree.
pub fn title(lang: Lang) -> &'static str {
    i18n::text(lang, Key::TermsTitle)
}

/// `/te-mau-parau-tumu-e-te-ture` — the page.
#[page("/te-mau-parau-tumu-e-te-ture")]
pub async fn terms(cx: &Cx) -> Result<impl View> {
    let lang = i18n::resolve(cx);

    Ok(view! {
        <div class=(theme::PAGE)>
            <h1 class=(theme::H1)>(title(lang))</h1>
            <p class=(theme::LEAD)>(i18n::text(lang, Key::TermsIntro))</p>

            // The statements, in their own column: each is one sentence and one
            // claim, and the spacing between them is what keeps them from being
            // read as one paragraph — which is what they would be if a rights
            // holder had to work out where one promise ended and the next began.
            <div class=(theme::TERMS_PROSE)>
                for key in STATEMENTS {
                    <p>(i18n::text(lang, key))</p>
                }
            </div>

            // When these words last changed. Set apart and quieter than the
            // statements above, because it is a fact about the page rather than
            // one of the claims in it.
            <p class=(theme::TERMS_UPDATED)>
                (i18n::text(lang, Key::TermsUpdated))
                " "
                (UPDATED)
            </p>
        </div>
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The path is the owner's, one segment, and no path of this site may begin
    /// with a language code: `/te…` is *te*, the article, and the layer that
    /// retires the old `/fr`, `/ty` and `/en` spellings reads the first segment
    /// of every path — see `routes::language::split_prefix`.
    #[test]
    fn the_path_is_what_the_route_declares() {
        assert_eq!(PATH, "/te-mau-parau-tumu-e-te-ture");
        assert!(PATH.starts_with('/') && !PATH.contains('{'), "{PATH}");
        assert_eq!(PATH.matches('/').count(), 1, "{PATH}");
    }

    /// The date is a date, and it is not in the future: it moves when the words
    /// on this page move, and it is the only claim here that can silently rot.
    #[test]
    fn the_date_is_written_the_one_way_all_three_languages_read() {
        assert_eq!(UPDATED.len(), 10, "{UPDATED}");
        let parts: Vec<&str> = UPDATED.split('-').collect();
        assert_eq!(parts.len(), 3, "{UPDATED}");
        assert_eq!(parts[0], "2026", "{UPDATED}");
        assert!(
            (1..=12).contains(&parts[1].parse::<u32>().expect("a month")),
            "{UPDATED}"
        );
        assert!(
            (1..=31).contains(&parts[2].parse::<u32>().expect("a day")),
            "{UPDATED}"
        );
    }

    /// The page's whole content is this list, so the list is where a test can
    /// see it: six statements, no repeat, and each one written in all three
    /// languages (the Tahitian as the English, by the catalog's own rule).
    #[test]
    fn the_six_statements_are_all_there_and_all_readable() {
        assert_eq!(STATEMENTS.len(), 6);

        let mut seen = std::collections::BTreeSet::new();
        for key in STATEMENTS {
            assert!(seen.insert(key.name()), "{key:?} is stated twice");
            for lang in Lang::ALL {
                assert!(
                    !i18n::text(lang, key).trim().is_empty(),
                    "{key:?} is silent in {}",
                    lang.code()
                );
            }
            // Prose: the Tahitian is the English rather than a guess.
            assert_eq!(
                i18n::text(Lang::Ty, key),
                i18n::text(Lang::En, key),
                "{key:?} was given a Tahitian sentence nobody wrote"
            );
        }
    }

    /// What the page promises about rights, checked as words rather than as
    /// intent: both addresses are there, and the promise is removal rather than
    /// a procedure nobody has described.
    ///
    /// This is the page's one load-bearing sentence. A rights holder who cannot
    /// find where to write does not write, and a "process" invented here would
    /// be a promise the site cannot keep.
    #[test]
    fn the_two_addresses_a_rights_holder_writes_to_are_both_there() {
        for lang in [Lang::Fr, Lang::En] {
            let line = i18n::text(lang, Key::TermsRemoval);

            for address in ["contact@chansondufenua.pf", "contact@rao-nagos.pf"] {
                assert!(line.contains(address), "{}: {address}", lang.code());
            }
            assert!(
                line.contains("contact@chansondufenua.pf")
                    && line.find("contact@chansondufenua.pf") < line.find("contact@rao-nagos.pf"),
                "{}: the fallback address is offered first",
                lang.code()
            );
        }

        // The page never claims a procedure, a delay or a form: there is an
        // address and a promise to act.
        for lang in Lang::ALL {
            let text = i18n::text(lang, Key::TermsRemoval).to_lowercase();
            for invented in ["formulaire", "délai", "48 h", "form", "within "] {
                assert!(!text.contains(invented), "{}: {invented}", lang.code());
            }
        }
    }

    /// The statements that are the owner's own are still his — byte for byte,
    /// in French, which is the language he wrote them in. A later run that
    /// "improves" one of these changes what the site promises.
    #[test]
    fn the_owners_own_sentences_are_the_ones_that_ship() {
        const FRENCH: [(Key, &str); 4] = [
            (
                Key::TermsPublisher,
                "Le site est hébergé et édité par Rao Nagos.",
            ),
            (
                Key::TermsLyrics,
                "Les paroles ne sont pas les nôtres : elles appartiennent à leurs auteurs.",
            ),
            (
                Key::TermsData,
                "Aucune information sensible vous concernant n'est sauvegardée sur ce site.",
            ),
            (Key::TermsCode, "Le code du site est sous licence GPL-3.0."),
        ];

        for (key, words) in FRENCH {
            assert_eq!(i18n::text(Lang::Fr, key), words, "{key:?} moved");
        }
    }

    /// One name for the page, whatever asks for it: the heading, the `<title>`
    /// the layout writes, and the footer's link are one string per language.
    ///
    /// French and English differ — a page whose two names agreed would be a
    /// translation nobody did — and the Tahitian is the English, which is the
    /// catalog's rule for prose rather than a third name.
    #[test]
    fn the_page_has_one_name_in_every_language() {
        let names: Vec<&str> = Lang::ALL.iter().map(|lang| title(*lang)).collect();

        assert_eq!(names.len(), 3);
        assert_ne!(names[0], names[1], "the French and the English agree");
        assert_eq!(
            names[1], names[2],
            "the Tahitian is its own sentence rather than the English"
        );
        for name in names {
            assert!(!name.trim().is_empty(), "{name:?}");
            assert!(
                name.chars().count() <= crate::domain::song::DESCRIPTION_MAX,
                "{name:?} is too long to be a heading"
            );
        }
    }
}
