//! The addresses the site used to publish, and where they went.
//!
//! v4.2 moved every page that was addressed in French (or in v3's
//! transliteration of a Tahitian word) onto the Tahitian address the reviewer
//! named: `/recherche` → `/paimi`, `/soutenir` → `/tauturu`,
//! `/artiste/{id}` → `/taata-himene/{id}`, `/himene/pluriel` → `/puta-himene`,
//! and `/aepa` → `/faariiraa`. His own rule for the move is that nothing a
//! reader has already linked may break, so **every old address answers `301` to
//! the new one** rather than 404ing — and this module is the one table that
//! says which is which.
//!
//! # Why it is a file of its own
//!
//! A retired path is not a page: no handler declares it, no `<head>` is written
//! for it, and [`crate::routes::language`] answers it *before* the router sees
//! the request. What is left of it is a fact about addresses, and the pages'
//! own modules are where the addresses live — `pages::home::PATH`,
//! `pages::book::PATH` — so the table that retires an address belongs beside
//! them rather than in the layer that acts on it. Every successor here is a
//! page's own constant, so a page that moves again moves its retired address
//! with it.
//!
//! # What it is not
//!
//! Not a rewrite of every URL that ever existed: only the addresses that were
//! *published* are here. A retired address is answered for good and for
//! everybody (see [`crate::routes::language`]), so a row that never shipped
//! would be a redirect inventing a past for a page that did not have one.

use crate::pages::{artist, book, home, search, support};

/// The addresses that were published before v4.2, and the page that answers
/// them now.
///
/// In the order the site grew them. The strings are data — an address a reader
/// may still hold — so they are spelled here and nowhere else.
pub const RETIRED: [(&str, &str); 4] = [
    // v3's transliteration of *fa'ari'ira'a*, and until v4.2 the front page's
    // Tahitian address: `/aepa` and `/` were the same page under two URLs, and
    // v4.2 keeps the path the reviewer named as the standard.
    ("/aepa", home::PATH),
    ("/recherche", search::PATH),
    ("/soutenir", support::PATH),
    ("/himene/pluriel", book::PATH),
];

/// The artist prefix as it used to be spelled.
///
/// A prefix rather than a whole path, and so not a row of [`RETIRED`]: an
/// artist's page is `/artiste/{id}` for every artist, and the table would have
/// to hold one row per row in the database. [`successor`] moves the prefix and
/// keeps the id.
pub const ARTIST_PREFIX: &str = "/artiste/";

/// Where `path` is answered now, if it is a retired address.
///
/// `None` for everything else, including a path that merely resembles one:
/// `/artiste/a/b` is nobody's page and does not become `/taata-himene/a/b` by
/// being moved — [`crate::pages::artist::segment`]'s own rule, restated here
/// because a redirect is a promise that the target answers.
///
/// The successor is root-relative and is a page's own address: a language is no
/// longer part of a URL, so there is nothing for the caller to put back in front
/// of it.
pub fn successor(path: &str) -> Option<String> {
    if let Some((_, to)) = RETIRED.iter().find(|(from, _)| *from == path) {
        return Some((*to).to_owned());
    }

    let id = path.strip_prefix(ARTIST_PREFIX)?;
    (!id.is_empty() && !id.contains('/')).then(|| format!("{}{id}", artist::PREFIX))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every retired address moves to a page, and every successor is one of the
    /// pages' own constants — a table entry that named a path no route declares
    /// would be a `301` to a `404`, which is worse than the `404` it replaced.
    #[test]
    fn every_retired_address_moves_to_a_page() {
        let pages = [home::PATH, book::PATH, search::PATH, support::PATH];

        for (from, to) in RETIRED {
            assert!(from.starts_with('/'), "{from}");
            assert!(
                pages.contains(&to),
                "{from} moves to {to}, which no page declares"
            );
            assert_ne!(from, to, "{from} is its own successor");
        }

        // No address appears twice, so no request has two answers.
        let mut seen = RETIRED.map(|(from, _)| from).to_vec();
        seen.sort_unstable();
        seen.dedup();
        assert_eq!(seen.len(), RETIRED.len());
    }

    /// The artist's prefix is not a whole path: the id travels with it.
    #[test]
    fn an_artist_keeps_their_id_across_the_move() {
        assert_eq!(
            successor("/artiste/1kvm9y2tcplm43wgeuni").as_deref(),
            Some("/taata-himene/1kvm9y2tcplm43wgeuni")
        );
        assert_eq!(successor("/taata-himene/abc"), None);
        assert_eq!(successor("/artiste/"), None);
        assert_eq!(successor("/artiste"), None);
        assert_eq!(successor("/artiste/a/b"), None);
    }

    /// The reader answers the retired addresses and nothing else: a page that
    /// is live now, a machine surface, and a path that was never published are
    /// all left to the router.
    #[test]
    fn only_the_published_old_addresses_move() {
        for (from, _) in RETIRED {
            assert!(successor(from).is_some(), "{from}");
        }

        for live in [
            home::PATH,
            home::ROOT,
            book::PATH,
            search::PATH,
            support::PATH,
            "/himene",
            "/himene/ahani-e",
            "/himene/sitemap.xml",
            "/api/songs",
            "/robots.txt",
            "/sitemap.xml",
            "/",
        ] {
            assert_eq!(successor(live), None, "{live}");
        }
    }
}
