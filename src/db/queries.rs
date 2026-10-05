//! Every SQL statement in the application lives here.
//!
//! Ported from v3's SurrealDB layer: `fn::create_song` (find-or-create artists,
//! then insert), the `published = true` filter that v3 applied to every read,
//! and the `created_at DESC` ordering that `/himene` uses. The read path
//! deliberately mirrors the live ordering — verified against the running site
//! before it was written down, not assumed.

use chrono::{DateTime, SecondsFormat, Utc};
use sqlx::{FromRow, SqlitePool};

use crate::domain::song::ARTISTS_MAX;
use crate::domain::{AppError, Artist, Song};

use super::{DbError, DbResult};

// ---------------------------------------------------------------------------
// Timestamps
// ---------------------------------------------------------------------------

/// RFC 3339 with nanoseconds, always `Z`.
///
/// Nanosecond precision is not vanity: v3 stored `time::now()` values such as
/// `2025-03-09T20:09:12.123456789Z`, and rounding them on import would make the
/// migrated rows differ from the ones the live site is serving.
pub(crate) fn fmt_dt(t: DateTime<Utc>) -> String {
    t.to_rfc3339_opts(SecondsFormat::Nanos, true)
}

pub(crate) fn parse_dt(s: &str) -> DbResult<DateTime<Utc>> {
    DateTime::parse_from_rfc3339(s)
        .map(|t| t.with_timezone(&Utc))
        .map_err(|_| DbError::Timestamp(s.to_owned()))
}

// ---------------------------------------------------------------------------
// Rows
// ---------------------------------------------------------------------------

#[derive(FromRow)]
pub(crate) struct ArtistRow {
    pub id: String,
    pub fullname: String,
    pub created_at: String,
    pub updated_at: String,
}

impl ArtistRow {
    pub(crate) fn into_artist(self) -> DbResult<Artist> {
        Ok(Artist::new(
            self.id,
            self.fullname,
            parse_dt(&self.created_at)?,
            parse_dt(&self.updated_at)?,
        ))
    }
}

/// One row of the song ⟕ artist join — i.e. one *credit*, not one song.
///
/// A song with three artists produces three rows carrying identical song
/// columns. [`assemble`] folds them back into one [`Song`].
#[derive(FromRow)]
struct SongArtistRow {
    id: String,
    title: String,
    lyrics: String,
    view_count: i64,
    published: i64,
    created_at: String,
    updated_at: String,
    artist_id: Option<String>,
    artist_fullname: Option<String>,
    artist_created_at: Option<String>,
    artist_updated_at: Option<String>,
}

impl SongArtistRow {
    fn artist(&self) -> DbResult<Option<Artist>> {
        let (Some(id), Some(fullname)) = (&self.artist_id, &self.artist_fullname) else {
            // LEFT JOIN miss: the song simply has no credited artist. This is a
            // real case in the data (one of the 43 migrated songs has none).
            return Ok(None);
        };
        Ok(Some(Artist::new(
            id.clone(),
            fullname.clone(),
            parse_dt(self.artist_created_at.as_deref().unwrap_or_default())?,
            parse_dt(self.artist_updated_at.as_deref().unwrap_or_default())?,
        )))
    }
}

/// Shared projection. `position` is ordered on but not selected — the row order
/// is what carries it.
const SONGS_SELECT: &str = "\
SELECT s.id, s.title, s.lyrics, s.view_count, s.published, s.created_at, s.updated_at,
       a.id AS artist_id, a.fullname AS artist_fullname,
       a.created_at AS artist_created_at, a.updated_at AS artist_updated_at
FROM song s
LEFT JOIN song_artist sa ON sa.song_id = s.id
LEFT JOIN artist a ON a.id = sa.artist_id";

/// How `/himene` and the home page order songs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SongOrder {
    /// What the live site does today on `/himene`.
    #[default]
    Newest,
    Oldest,
    MostViewed,
    Title,
}

impl SongOrder {
    /// The ordering keys, unqualified by `ORDER BY` and without `sa.position`.
    ///
    /// Note the `s.id` tiebreaker. Without it two songs sharing a `created_at`
    /// would interleave their credit rows (the sort would fall through to
    /// `sa.position`, which is per-song), and [`assemble`] would emit one
    /// half-song per row.
    ///
    /// `sa.position` is deliberately *not* here. This ordering has to be usable
    /// *before* the join, where the credit table is not in scope yet — see
    /// [`songs`]. It is appended at the one call site that has run the join.
    fn song_keys(self) -> &'static str {
        match self {
            Self::Newest => "s.created_at DESC, s.id",
            Self::Oldest => "s.created_at ASC, s.id",
            Self::MostViewed => "s.view_count DESC, s.id",
            Self::Title => "s.title COLLATE NOCASE ASC, s.id",
        }
    }
}

/// The sub-select that picks one *page* of songs, before any join.
///
/// Ids only. [`SONGS_SELECT`] decorates that page with credits afterwards, and
/// the decoration must not be able to change which songs are on the page.
fn page_select(keys: &str) -> String {
    format!("SELECT s.id FROM song s WHERE s.published = 1 ORDER BY {keys} LIMIT ?1")
}

/// Fold credit rows into songs, preserving both the row order and each song's
/// credit order.
fn assemble(rows: Vec<SongArtistRow>) -> DbResult<Vec<Song>> {
    let mut out: Vec<Song> = Vec::new();
    let mut current: Option<(SongArtistRow, Vec<Artist>)> = None;

    for row in rows {
        let starts_new_song = current
            .as_ref()
            .map(|(head, _)| head.id != row.id)
            .unwrap_or(true);

        // Read the credit *before* the row is moved into `current`, and note
        // that this has to happen for the row that opens a song too — that row
        // carries the song's first credit. Skipping it (by `continue`-ing here)
        // silently drops one artist from every song.
        let credit = row.artist()?;

        if starts_new_song {
            if let Some((head, artists)) = current.take() {
                out.push(finish(head, artists)?);
            }
            current = Some((row, Vec::new()));
        }

        if let Some(artist) = credit
            && let Some((_, artists)) = current.as_mut()
        {
            artists.push(artist);
        }
    }

    if let Some((head, artists)) = current.take() {
        out.push(finish(head, artists)?);
    }
    Ok(out)
}

fn finish(head: SongArtistRow, artists: Vec<Artist>) -> DbResult<Song> {
    Ok(Song::new(
        head.id,
        head.title,
        head.lyrics,
        head.view_count.max(1) as u32,
        artists,
        head.published != 0,
        parse_dt(&head.created_at)?,
        parse_dt(&head.updated_at)?,
    ))
}

// ---------------------------------------------------------------------------
// Reads
// ---------------------------------------------------------------------------

/// Every published song, with its credits, in `order`.
///
/// `limit` counts **songs**, not rows, and that is the whole reason this is not
/// a bare `LIMIT`. The read joins one row per *credit*, so a song with two
/// credited artists contributes two rows — and `LIMIT 5` over that join returns
/// four songs for the price of five. The bug was live, not theoretical: the home
/// page's most-viewed table rendered four rows until this was fixed, because the
/// busiest song in the export has two credits. `page_select` picks the page of
/// songs first; the join only decorates it.
///
/// `a_limited_read_counts_songs_not_credit_rows` is the regression test.
pub async fn songs(pool: &SqlitePool, order: SongOrder, limit: Option<i64>) -> DbResult<Vec<Song>> {
    let keys = order.song_keys();
    let sql = match limit {
        Some(_) => format!(
            "{SONGS_SELECT} WHERE s.id IN ({}) ORDER BY {keys}, sa.position",
            page_select(keys)
        ),
        None => format!("{SONGS_SELECT} WHERE s.published = 1 ORDER BY {keys}, sa.position"),
    };

    let mut query = sqlx::query_as::<_, SongArtistRow>(&sql);
    if let Some(n) = limit {
        query = query.bind(n);
    }

    assemble(query.fetch_all(pool).await?)
}

/// One song by id, published or not — the page decides whether to 404, and the
/// editor needs to see drafts.
pub async fn song(pool: &SqlitePool, id: &str) -> DbResult<Option<Song>> {
    let sql = format!("{SONGS_SELECT} WHERE s.id = ?1 ORDER BY sa.position");
    let rows = sqlx::query_as::<_, SongArtistRow>(&sql)
        .bind(id)
        .fetch_all(pool)
        .await?;
    Ok(assemble(rows)?.into_iter().next())
}

/// How much the public catalogue holds.
///
/// Two numbers and no rows. `/api/health` reports them, and the point of the
/// probe is that it reads the database — a process that answers while its
/// database is unreachable is exactly the failure a health check exists to
/// catch — without putting 43 lyrics in memory to say "43".
///
/// Songs are counted the way the site serves them: drafts are not part of the
/// catalogue, so they are not part of its size.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Counts {
    pub songs: u32,
    pub artists: u32,
}

/// One round trip for both counts.
///
/// Sub-selects rather than two queries: they cannot disagree about the moment
/// they describe, which two sequential reads can, and they are one statement for
/// SQLite to plan.
pub async fn counts(pool: &SqlitePool) -> DbResult<Counts> {
    let (songs, artists): (i64, i64) = sqlx::query_as(
        "SELECT (SELECT COUNT(*) FROM song WHERE published = 1), (SELECT COUNT(*) FROM artist)",
    )
    .fetch_one(pool)
    .await?;

    Ok(Counts {
        songs: songs.max(0) as u32,
        artists: artists.max(0) as u32,
    })
}

/// Every artist, alphabetically, case-insensitively.
pub async fn artists(pool: &SqlitePool) -> DbResult<Vec<Artist>> {
    let rows = sqlx::query_as::<_, ArtistRow>(
        "SELECT id, fullname, created_at, updated_at FROM artist \
         ORDER BY fullname COLLATE NOCASE",
    )
    .fetch_all(pool)
    .await?;
    rows.into_iter().map(ArtistRow::into_artist).collect()
}

/// Artist autocomplete for the editor's credit field.
///
/// Uses the FTS5 index rather than `LIKE`, for the same reason v3 used a
/// `punct_lower_ascii` SEARCH ANALYZER: so `mahoi` finds `Mā'ohi`. Diacritics are
/// stripped and case is folded, but punctuation is *not* — `unicode61` splits on
/// it, so a name containing an ʻokina is indexed as separate tokens and only the
/// trailing token is reachable by prefix. See the tests.
pub async fn search_artists(pool: &SqlitePool, query: &str, limit: i64) -> DbResult<Vec<Artist>> {
    let needle = query.trim();
    if needle.is_empty() {
        return artists(pool).await;
    }

    // FTS5 string literal: wrap in double quotes so operator characters are
    // treated as text, and double any embedded quote. Then `*` for prefix.
    let match_expr = format!("\"{}\"*", needle.replace('"', "\"\""));

    let rows = sqlx::query_as::<_, ArtistRow>(
        "SELECT a.id, a.fullname, a.created_at, a.updated_at \
         FROM artist_fts f \
         JOIN artist a ON a.rowid = f.rowid \
         WHERE artist_fts MATCH ?1 \
         ORDER BY bm25(artist_fts), a.fullname COLLATE NOCASE \
         LIMIT ?2",
    )
    .bind(match_expr)
    .bind(limit)
    .fetch_all(pool)
    .await?;

    rows.into_iter().map(ArtistRow::into_artist).collect()
}

// ---------------------------------------------------------------------------
// Writes
// ---------------------------------------------------------------------------

/// A fresh 20-character id, matching the shape of the v3 record keys.
///
/// Done in SQL (`randomblob` → hex) rather than by pulling in an RNG crate:
/// 80 bits of randomness is far more than enough for a songbook, and it keeps
/// the dependency list at zero for this.
pub(crate) async fn new_id<'e, E>(executor: E) -> DbResult<String>
where
    E: sqlx::Executor<'e, Database = sqlx::Sqlite>,
{
    Ok(sqlx::query_scalar("SELECT lower(hex(randomblob(10)))")
        .fetch_one(executor)
        .await?)
}

/// Count a view. Returns the new count, or [`DbError::NotFound`].
pub async fn increment_view_count(pool: &SqlitePool, id: &str) -> DbResult<u32> {
    let updated: Option<i64> = sqlx::query_scalar(
        "UPDATE song SET view_count = view_count + 1 WHERE id = ?1 RETURNING view_count",
    )
    .bind(id)
    .fetch_optional(pool)
    .await?;

    updated
        .map(|n| n.max(1) as u32)
        .ok_or_else(|| DbError::NotFound {
            entity: "song",
            id: id.to_owned(),
        })
}

/// Create a song, finding or creating its artists first.
///
/// This is v3's `fn::create_song`, with the fixes the rewrite buys us:
///
/// * **Atomic.** v3's version ran as a sequence of separate statements, so a
///   failure part-way through could leave orphaned artists behind. Here the
///   whole thing is one transaction.
/// * **No duplicate credits.** Duplicate names in the form field are collapsed,
///   which also keeps the `song_artist` primary key from rejecting the insert
///   with an opaque constraint error.
/// * **Validated in the domain first.** Bounds are checked before a single write,
///   so a bad title is an `AppError`, not a SQLite `CHECK` failure.
///
/// Artist input is a comma-separated string, exactly as v3's editor produced it.
pub async fn create_song(
    pool: &SqlitePool,
    title: &str,
    lyrics: &str,
    fullnames: &str,
) -> DbResult<Song> {
    let title = title.trim().to_owned();
    let now = Utc::now();

    // Order-preserving dedupe of the requested credits.
    let mut requested: Vec<&str> = Vec::new();
    for name in Artist::split_fullnames(fullnames) {
        if !requested.contains(&name) {
            requested.push(name);
        }
    }

    // Refuse before opening a transaction.
    Song::new(
        String::new(),
        title.clone(),
        lyrics.to_owned(),
        1,
        Vec::new(),
        true,
        now,
        now,
    )
    .validate()?;
    if requested.len() > ARTISTS_MAX {
        return Err(AppError::invalid(
            "artists",
            format!("expected at most {ARTISTS_MAX}, got {}", requested.len()),
        )
        .into());
    }
    for name in &requested {
        Artist::validate_fullname(name)?;
    }

    let mut tx = pool.begin().await?;

    let mut credits: Vec<Artist> = Vec::with_capacity(requested.len());
    for name in &requested {
        let existing: Option<ArtistRow> = sqlx::query_as(
            "SELECT id, fullname, created_at, updated_at FROM artist WHERE fullname = ?1",
        )
        .bind(*name)
        .fetch_optional(&mut *tx)
        .await?;

        credits.push(match existing {
            Some(row) => row.into_artist()?,
            None => {
                let id = new_id(&mut *tx).await?;
                let stamp = fmt_dt(now);
                sqlx::query(
                    "INSERT INTO artist (id, fullname, created_at, updated_at) \
                     VALUES (?1, ?2, ?3, ?4)",
                )
                .bind(&id)
                .bind(*name)
                .bind(&stamp)
                .bind(&stamp)
                .execute(&mut *tx)
                .await?;
                Artist::new(id, (*name).to_owned(), now, now)
            }
        });
    }

    let song_id = new_id(&mut *tx).await?;
    let stamp = fmt_dt(now);
    sqlx::query(
        "INSERT INTO song (id, title, lyrics, view_count, published, created_at, updated_at) \
         VALUES (?1, ?2, ?3, 1, 1, ?4, ?5)",
    )
    .bind(&song_id)
    .bind(&title)
    .bind(lyrics)
    .bind(&stamp)
    .bind(&stamp)
    .execute(&mut *tx)
    .await?;

    for (position, artist) in credits.iter().enumerate() {
        sqlx::query("INSERT INTO song_artist (song_id, artist_id, position) VALUES (?1, ?2, ?3)")
            .bind(&song_id)
            .bind(artist.get_id())
            .bind(position as i64)
            .execute(&mut *tx)
            .await?;
    }

    tx.commit().await?;

    song(pool, &song_id).await?.ok_or(DbError::NotFound {
        entity: "song",
        id: song_id,
    })
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use super::*;
    use crate::db::{Db, fixtures};

    async fn fresh() -> Db {
        Db::open_in_memory().await.expect("in-memory database")
    }

    async fn seeded() -> Db {
        let db = fresh().await;
        fixtures::seed(db.pool()).await.expect("seed fixtures");
        db
    }

    async fn add_artist(db: &Db, id: &str, fullname: &str) {
        sqlx::query(
            "INSERT INTO artist (id, fullname, created_at, updated_at) VALUES (?1, ?2, ?3, ?3)",
        )
        .bind(id)
        .bind(fullname)
        .bind(fmt_dt(Utc::now()))
        .execute(db.pool())
        .await
        .expect("insert artist");
    }

    fn lyrics_of(len: usize) -> String {
        "y".repeat(len)
    }

    // -- ids ---------------------------------------------------------------

    #[tokio::test]
    async fn new_ids_are_20_lowercase_alphanumerics_and_unique() {
        let db = fresh().await;
        let mut seen = HashSet::new();
        for _ in 0..200 {
            let id = new_id(db.pool()).await.expect("id");
            assert_eq!(id.len(), 20, "{id}");
            assert!(
                id.chars()
                    .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit()),
                "unexpected character in {id}"
            );
            assert!(seen.insert(id.clone()), "generated {id} twice");
        }
    }

    // -- reads -------------------------------------------------------------

    #[tokio::test]
    async fn seeded_songs_come_back_newest_first() {
        let db = seeded().await;
        let songs = songs(db.pool(), SongOrder::Newest, None).await.unwrap();
        assert_eq!(songs.len(), fixtures::SONGS.len());

        let mut expected: Vec<&str> = fixtures::SONGS.iter().map(|s| s.id).collect();
        expected.sort_by(|a, b| {
            fixtures::by_id(b)
                .unwrap()
                .created_at
                .cmp(fixtures::by_id(a).unwrap().created_at)
                .then_with(|| a.cmp(b))
        });

        let got: Vec<String> = songs.iter().map(Song::get_id).collect();
        assert_eq!(got, expected);
    }

    #[tokio::test]
    async fn every_fixture_round_trips_byte_for_byte() {
        let db = seeded().await;
        for fixture in fixtures::SONGS {
            let found = song(db.pool(), fixture.id)
                .await
                .unwrap()
                .unwrap_or_else(|| panic!("{} is missing", fixture.id));
            assert_eq!(found.get_title(), fixture.title);
            assert_eq!(found.get_view_count(), fixture.view_count);
            // The whole point of storing lyrics raw: `<sup data-nosnippet>`
            // chord markup and macrons must survive untouched.
            assert_eq!(
                found.get_lyrics(),
                fixture.lyrics,
                "lyrics changed for {}",
                fixture.id
            );
        }
    }

    #[tokio::test]
    async fn credit_order_is_preserved() {
        let db = seeded().await;
        let found = song(db.pool(), "4cfl27ia9hndgetgr1o7")
            .await
            .unwrap()
            .unwrap();
        let names: Vec<String> = found
            .get_artists()
            .iter()
            .map(Artist::get_fullname)
            .collect();
        assert_eq!(names, ["Apatea Flores", "Teiho Tetoofa"]);
    }

    #[tokio::test]
    async fn a_song_may_have_no_credited_artist() {
        let db = seeded().await;
        let found = song(db.pool(), "luvw0mxon9hov8w6y2bk")
            .await
            .unwrap()
            .unwrap();
        assert!(found.get_artists().is_empty());
        assert_eq!(found.get_title(), "Ratatum et ratamtam");
    }

    #[tokio::test]
    async fn an_unknown_id_is_none_not_an_error() {
        let db = seeded().await;
        assert!(song(db.pool(), "does-not-exist").await.unwrap().is_none());
    }

    #[tokio::test]
    async fn unpublished_songs_are_hidden_from_the_list_but_still_readable() {
        let db = seeded().await;
        sqlx::query(
            "INSERT INTO song (id, title, lyrics, view_count, published, created_at, updated_at) \
             VALUES ('hidden0000000000000', 'A hidden song', ?1, 1, 0, ?2, ?2)",
        )
        .bind(lyrics_of(120))
        .bind("2026-01-01T00:00:00.000000000Z")
        .execute(db.pool())
        .await
        .unwrap();

        let listed = songs(db.pool(), SongOrder::Newest, None).await.unwrap();
        assert_eq!(listed.len(), fixtures::SONGS.len());
        assert!(
            song(db.pool(), "hidden0000000000000")
                .await
                .unwrap()
                .is_some()
        );
    }

    #[tokio::test]
    async fn limits_are_applied() {
        let db = seeded().await;
        let limited = songs(db.pool(), SongOrder::Newest, Some(2)).await.unwrap();
        assert_eq!(limited.len(), 2);
    }

    /// `limit` counts songs, not credit rows.
    ///
    /// The busiest fixture ("Te here fenua") carries **two** credits, so a
    /// `LIMIT` applied to the joined rows would spend both of them on that one
    /// song and return a single row for `Some(2)`. That is exactly what the home
    /// page's most-viewed table did before this was fixed: it asked for five and
    /// rendered four.
    #[tokio::test]
    async fn a_limited_read_counts_songs_not_credit_rows() {
        let db = seeded().await;

        let limited = songs(db.pool(), SongOrder::MostViewed, Some(2))
            .await
            .unwrap();
        assert_eq!(
            limited.len(),
            2,
            "the limit spent itself on one song's credits"
        );

        // ...and the two-credit song still arrives whole, both credits in order.
        assert_eq!(limited[0].get_artists().len(), 2);
        let names: Vec<String> = limited[0]
            .get_artists()
            .iter()
            .map(Artist::get_fullname)
            .collect();
        assert_eq!(names, ["Apatea Flores", "Teiho Tetoofa"]);

        // A limit larger than the table is still just the table.
        let all = songs(db.pool(), SongOrder::MostViewed, Some(99))
            .await
            .unwrap();
        assert_eq!(all.len(), fixtures::SONGS.len());
    }

    #[tokio::test]
    async fn ordering_by_view_count_is_descending() {
        let db = seeded().await;
        let ordered = songs(db.pool(), SongOrder::MostViewed, None).await.unwrap();
        let counts: Vec<u32> = ordered.iter().map(Song::get_view_count).collect();
        let mut sorted = counts.clone();
        sorted.sort_unstable_by(|a, b| b.cmp(a));
        assert_eq!(counts, sorted);
    }

    // -- searching ---------------------------------------------------------

    #[tokio::test]
    async fn search_folds_case_and_diacritics() {
        let db = fresh().await;
        add_artist(&db, "aaaaaaaaaaaaaaaaaaaa", "Barthélémy").await;
        add_artist(&db, "bbbbbbbbbbbbbbbbbbbb", "Théo Sulpice").await;
        add_artist(&db, "cccccccccccccccccccc", "Maruia").await;

        for needle in [
            "barthelemy",
            "Barthelemy",
            "BARTHELEMY",
            "barthélémy",
            "barth",
        ] {
            let hits = search_artists(db.pool(), needle, 10).await.unwrap();
            assert_eq!(hits.len(), 1, "needle {needle:?} matched {hits:?}");
            assert_eq!(hits[0].get_fullname(), "Barthélémy");
        }

        let hits = search_artists(db.pool(), "theo", 10).await.unwrap();
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].get_fullname(), "Théo Sulpice");
    }

    #[tokio::test]
    async fn search_splits_on_punctuation_exactly_as_v3_did() {
        let db = fresh().await;
        add_artist(&db, "dddddddddddddddddddd", "T'Angelo").await;

        // `unicode61` treats the apostrophe as a separator and indexes `t` and
        // `angelo` as two tokens, so only the trailing token is reachable by
        // prefix. This is *not* a regression — v3's `punct_lower_ascii`
        // analyzer used the PUNCT tokenizer and split identically. It is
        // recorded here so nobody "fixes" it by accident.
        assert!(
            search_artists(db.pool(), "tangelo", 10)
                .await
                .unwrap()
                .is_empty()
        );
        let hits = search_artists(db.pool(), "angelo", 10).await.unwrap();
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].get_fullname(), "T'Angelo");
    }

    #[tokio::test]
    async fn search_survives_fts_syntax_characters() {
        let db = fresh().await;
        add_artist(&db, "eeeeeeeeeeeeeeeeeeee", "Maruia").await;
        // Quoting the needle means FTS5 operators are treated as text. A user
        // typing `*` or `NEAR(` must get an empty result, never a 500.
        for needle in ["\"", "*", "NEAR(", "a OR b", "()", "^", ":", "-"] {
            search_artists(db.pool(), needle, 10)
                .await
                .unwrap_or_else(|e| panic!("needle {needle:?} broke the search: {e}"));
        }
    }

    #[tokio::test]
    async fn empty_search_returns_everyone() {
        let db = fresh().await;
        add_artist(&db, "ffffffffffffffffffff", "Maruia").await;
        add_artist(&db, "gggggggggggggggggggg", "Jonas").await;
        assert_eq!(search_artists(db.pool(), "   ", 10).await.unwrap().len(), 2);
    }

    #[tokio::test]
    async fn the_fts_index_tracks_updates_and_deletes() {
        let db = fresh().await;
        add_artist(&db, "hhhhhhhhhhhhhhhhhhhh", "Maruia").await;
        assert_eq!(
            search_artists(db.pool(), "maruia", 10).await.unwrap().len(),
            1
        );

        sqlx::query("UPDATE artist SET fullname = 'Hivarai' WHERE fullname = 'Maruia'")
            .execute(db.pool())
            .await
            .unwrap();
        assert!(
            search_artists(db.pool(), "maruia", 10)
                .await
                .unwrap()
                .is_empty()
        );
        assert_eq!(
            search_artists(db.pool(), "hivarai", 10)
                .await
                .unwrap()
                .len(),
            1
        );

        sqlx::query("DELETE FROM artist WHERE fullname = 'Hivarai'")
            .execute(db.pool())
            .await
            .unwrap();
        assert!(
            search_artists(db.pool(), "hivarai", 10)
                .await
                .unwrap()
                .is_empty()
        );
    }

    // -- writes ------------------------------------------------------------

    #[tokio::test]
    async fn incrementing_a_view_count_reports_the_new_value() {
        let db = seeded().await;
        let id = fixtures::SONGS[0].id;
        let before = song(db.pool(), id).await.unwrap().unwrap().get_view_count();

        let after = increment_view_count(db.pool(), id).await.unwrap();
        assert_eq!(after, before + 1);
        assert_eq!(
            song(db.pool(), id).await.unwrap().unwrap().get_view_count(),
            before + 1
        );
    }

    #[tokio::test]
    async fn incrementing_an_unknown_song_is_not_found() {
        let db = fresh().await;
        assert!(matches!(
            increment_view_count(db.pool(), "nope").await.unwrap_err(),
            DbError::NotFound { .. }
        ));
    }

    #[tokio::test]
    async fn create_song_reuses_an_existing_artist() {
        let db = seeded().await;
        let created = create_song(db.pool(), "Une nouvelle chanson", &lyrics_of(120), "Jonas")
            .await
            .unwrap();

        assert_eq!(created.get_id().len(), 20);
        // Jonas' own v3 id, not a fresh one: the find-or-create matched.
        assert_eq!(created.get_artists()[0].get_id(), "1kvm9y2tcplm43wgeuni");
        let all = artists(db.pool()).await.unwrap();
        assert_eq!(
            all.iter().filter(|a| a.get_fullname() == "Jonas").count(),
            1
        );
    }

    #[tokio::test]
    async fn create_song_trims_blanks_and_dedupes_credits() {
        let db = seeded().await;
        let created = create_song(
            db.pool(),
            "Deux nouveaux artistes",
            &lyrics_of(120),
            "Nouveau Un, ,  Nouveau Un ,Nouveau Deux,",
        )
        .await
        .unwrap();

        let names: Vec<String> = created
            .get_artists()
            .iter()
            .map(Artist::get_fullname)
            .collect();
        assert_eq!(names, ["Nouveau Un", "Nouveau Deux"]);
    }

    #[tokio::test]
    async fn create_song_works_with_no_artists_at_all() {
        let db = seeded().await;
        let created = create_song(db.pool(), "Sans artiste", &lyrics_of(120), "")
            .await
            .unwrap();
        assert!(created.get_artists().is_empty());
        assert!(created.is_published());
        assert_eq!(created.get_view_count(), 1);
    }

    #[tokio::test]
    async fn a_rejected_song_leaves_nothing_behind() {
        let db = seeded().await;
        let artists_before = artists(db.pool()).await.unwrap().len();

        // Title is one character short; the artist name is brand new and valid,
        // so a non-atomic implementation would create it and then fail.
        let error = create_song(db.pool(), "abc", &lyrics_of(120), "Un Nouvel Artiste")
            .await
            .unwrap_err();
        assert!(matches!(error, DbError::Domain(_)), "got {error:?}");

        assert_eq!(
            artists(db.pool()).await.unwrap().len(),
            artists_before,
            "a rejected song must not leave an artist behind"
        );
        assert_eq!(
            songs(db.pool(), SongOrder::Newest, None)
                .await
                .unwrap()
                .len(),
            fixtures::SONGS.len()
        );
    }

    #[tokio::test]
    async fn an_invalid_artist_name_is_a_domain_error() {
        let db = seeded().await;
        // Nothing short is invalid any more (the floor is 1), so the only way
        // to reach the rule from the form is past the ceiling.
        let too_long = "a".repeat(crate::domain::artist::FULLNAME_MAX + 1);
        let error = create_song(db.pool(), "Titre valide", &lyrics_of(120), &too_long)
            .await
            .unwrap_err();
        assert!(matches!(error, DbError::Domain(_)), "got {error:?}");
    }

    #[tokio::test]
    async fn title_length_is_counted_in_characters_not_bytes() {
        let db = fresh().await;

        // A title at the ceiling, in twice as many bytes. v3's ASSERT used
        // `string::len`, which counts characters; a byte-based CHECK would
        // reject this and quietly make long Tahitian titles unwritable.
        let max = crate::domain::song::TITLE_MAX;
        let title = "Ā".repeat(max);
        let created = create_song(db.pool(), &title, &lyrics_of(120), "")
            .await
            .unwrap();
        assert_eq!(created.get_title().chars().count(), max);
        assert_eq!(created.get_title().len(), max * 2);

        let too_long = "Ā".repeat(max + 1);
        assert!(matches!(
            create_song(db.pool(), &too_long, &lyrics_of(120), "")
                .await
                .unwrap_err(),
            DbError::Domain(_)
        ));
    }

    #[tokio::test]
    async fn a_created_song_is_immediately_readable_in_full() {
        let db = seeded().await;
        let created = create_song(db.pool(), "Te here fenua", &lyrics_of(150), "Jonas, Maruia")
            .await
            .unwrap();

        let reread = song(db.pool(), &created.get_id()).await.unwrap().unwrap();
        assert_eq!(reread.get_title(), "Te here fenua");
        assert_eq!(reread.get_lyrics(), lyrics_of(150));
        let names: Vec<String> = reread
            .get_artists()
            .iter()
            .map(Artist::get_fullname)
            .collect();
        assert_eq!(names, ["Jonas", "Maruia"]);

        // And it appears in the list, because it is published.
        let listed = songs(db.pool(), SongOrder::Newest, None).await.unwrap();
        assert_eq!(listed.len(), fixtures::SONGS.len() + 1);
    }
}
