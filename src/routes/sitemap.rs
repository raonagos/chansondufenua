//! The two sitemaps — `/sitemap.xml` and `/himene/sitemap.xml`.
//!
//! v3 wrote both of them out of the database at deploy time and served the
//! result as files. Here they are routes like everything else, which is the
//! point: a song added through the form is in the sitemap on the next request
//! rather than at the next deploy, and no build step can forget to regenerate
//! one.
//!
//! **Two sitemaps, not a sitemap index.** The two documents are `/sitemap.xml`
//! for the site's fixed pages and `/himene/sitemap.xml` for the songs. A
//! `<sitemapindex>` document would have to live at one of those two URLs and
//! would push the other somewhere new, which is not worth it for two files —
//! and both URLs are advertised in `robots.txt` today, in v3's copy and in the
//! live site's. What *is* dropped from v3 is its hand-written `lastmod`: see
//! below.
//!
//! One deliberate difference from v3. Its home entry carried
//! `<lastmod>2024-08-21</lastmod>`, a date written into the generator and never
//! touched again. Here both fixed pages carry the newest song's `updated_at`,
//! because that is genuinely when their content last changed — both are lists of
//! songs. A site with no songs carries no `lastmod` at all rather than a
//! fabricated one.
//!
//! `/aepa` is not listed. It is the home page under a second URL and its own
//! canonical link says so; a sitemap lists canonical URLs.
//!
//! **"Canonical" means the language-prefixed form.** A page's canonical URL is
//! `/fr/…`, `/ty/…` or `/en/…` ([`crate::i18n::url`]); the bare URL is the
//! language-neutral `x-default` that serves the default and names the prefixed
//! form. A sitemap that listed the bare URL would hand a crawler a URL whose own
//! `<link rel="canonical">` pointed somewhere else — the one thing this module's
//! `/aepa` rule exists to avoid. So both sitemaps list
//! [`Lang::DEFAULT`](crate::i18n::Lang::DEFAULT)'s addresses; the other two
//! languages are discovered from each page's `hreflang` cluster, which is what
//! that cluster is for.

use topcoat::{
    Result,
    context::Cx,
    router::{
        content::sitemap::{ChangeFrequency, Sitemap, SitemapUrl},
        route,
    },
};

use crate::db::{SongOrder, songs};
use crate::i18n::{self, Lang};
use crate::pages::{home, songs as index};
use crate::state;

/// The fixed-pages sitemap.
pub const PATH: &str = "/sitemap.xml";

/// The songs sitemap.
///
/// A path under `/himene`, which is the song page's own prefix: the router
/// prefers a literal segment to a parameter, so `/himene/sitemap.xml` reaches
/// this route and never `#[page("/himene/{id}")]`. `.run/step10.sh` asserts that
/// against the running app rather than trusting it.
pub const SONGS_PATH: &str = "/himene/sitemap.xml";

/// `GET /sitemap.xml` — the pages that are not songs.
///
/// The two entries come from the site's own constants rather than being written
/// out, so a route that moves takes its sitemap entry with it.
#[route(GET "/sitemap.xml")]
async fn fixed_pages(cx: &Cx) -> Result<Sitemap> {
    let updated = newest_update(cx).await?;

    Ok(Sitemap::new()
        .url(entry(i18n::url(Lang::DEFAULT, home::PATH), updated).priority(1.0))
        .url(entry(i18n::url(Lang::DEFAULT, index::PATH), updated)))
}

/// `GET /himene/sitemap.xml` — every published song.
///
/// `db::songs` with no limit is the published catalogue, in the order the index
/// shows. The sitemap does not care about that order; going through the one
/// function that owns the read means a song cannot be listed here and missing
/// from the index, and the credits come along for the ride.
///
/// v3's per-song hints are kept: `weekly` and `0.9`, against the fixed pages'
/// `monthly` and `1.0`.
#[route(GET "/himene/sitemap.xml")]
async fn song_pages(cx: &Cx) -> Result<Sitemap> {
    let listed = songs(state::db(cx).pool(), SongOrder::Newest, None).await?;

    Ok(Sitemap::new().urls(listed.iter().map(|song| {
        entry(
            i18n::url(Lang::DEFAULT, &song.get_path()),
            Some(song.get_updated_at()),
        )
        .change_frequency(ChangeFrequency::Weekly)
        .priority(0.9)
    })))
}

/// One entry of either sitemap.
///
/// A free function so the fixed pages cannot drift apart in their hints, and so
/// `last_modified` is applied in one place. A missing date leaves the element
/// out entirely, which is the format's own way of saying "unknown".
fn entry(location: String, updated: Option<chrono::DateTime<chrono::Utc>>) -> SitemapUrl {
    let url = SitemapUrl::new(location).change_frequency(ChangeFrequency::Monthly);
    match updated {
        Some(updated) => url.last_modified(updated),
        None => url,
    }
}

/// When the newest published song last changed, if there is one.
///
/// The home page and the index are lists of songs, so this is their `lastmod`
/// too. A limited read rather than the whole catalogue: the answer is the first
/// row, and `db::songs` already filters drafts out of it.
async fn newest_update(cx: &Cx) -> Result<Option<chrono::DateTime<chrono::Utc>>> {
    Ok(songs(state::db(cx).pool(), SongOrder::Newest, Some(1))
        .await?
        .first()
        .map(|song| song.get_updated_at()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::{Db, fixtures};

    /// The parameterised song route must not swallow the sitemap. This is the
    /// half of that claim a unit test can hold: the path is under `/himene/`,
    /// and what follows it is not one of the slugs or ids in the corpus.
    #[test]
    fn the_songs_sitemap_is_not_a_song_address() {
        let segment = SONGS_PATH
            .strip_prefix("/himene/")
            .expect("the sitemap is under the song prefix");

        for song in fixtures::SONGS {
            assert_ne!(segment, song.slug, "{segment} is a fixture's slug");
            assert_ne!(segment, song.id, "{segment} is a fixture's id");
        }
    }

    /// Every address the sitemap will put in a `<loc>` is a legal path segment:
    /// lowercase ASCII, digits and hyphens. Asserted against the rule as well as
    /// the fixtures' own literals, because the sitemap lists slugs now and an
    /// illegal one would need escaping in the URL.
    #[test]
    fn every_fixture_slug_is_a_legal_path_segment() {
        for song in fixtures::SONGS {
            assert_eq!(crate::domain::slug::slugify(song.title), song.slug);
            assert!(
                song.slug
                    .chars()
                    .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-'),
                "{}",
                song.slug
            );
            assert!(!song.slug.is_empty(), "{}", song.title);
        }
    }

    /// A draft is not published, so it is not in the catalogue the sitemap is
    /// built from — and a draft dated newer than every fixture must not become
    /// the fixed pages' `lastmod`.
    #[tokio::test]
    async fn a_draft_never_becomes_the_lastmod() {
        let db = Db::open_in_memory().await.expect("in-memory database");
        fixtures::seed(db.pool()).await.expect("seed fixtures");

        sqlx::query(
            "INSERT INTO song (id, title, lyrics, view_count, published, created_at, updated_at) \
             VALUES ('draft0000000000000', 'Un brouillon', ?1, 1, 0, ?2, ?2)",
        )
        .bind("z".repeat(crate::domain::song::LYRICS_MIN))
        .bind("2030-01-01T00:00:00.000000000Z")
        .execute(db.pool())
        .await
        .expect("insert a draft");

        let listed = songs(db.pool(), SongOrder::Newest, Some(1))
            .await
            .expect("the newest song");

        assert_eq!(listed.len(), 1);
        assert!(listed[0].is_published());
        assert_ne!(listed[0].get_id(), "draft0000000000000");
    }
}
