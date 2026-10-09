//! Every SQL statement in the application lives here.
//!
//! Ported from v3's SurrealDB layer: `fn::create_song` (find-or-create artists,
//! then insert), the `published = true` filter that v3 applied to every read,
//! and the `created_at DESC` ordering that `/himene` uses. The read path
//! deliberately mirrors the live ordering — verified against the running site
//! before it was written down, not assumed.
//!
//! A song's *address* is minted here too: `assign_slug` turns a title into the
//! slug it is published under, and [`song_at`] resolves a URL segment back to
//! its song. The rule that shapes a title is `domain::slug`; what lives here is
//! the part that is a fact about the tables — uniqueness, and the history that
//! stops a retired address being handed to a different song.

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
    /// Empty for a song the slug rule could not name, and for one written
    /// straight to the table before the backfill ran. [`finish`] turns it into
    /// `None`; nothing downstream has to know which of the two it was.
    slug: String,
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
SELECT s.id, s.slug, s.title, s.lyrics, s.view_count, s.published, s.created_at, s.updated_at,
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

/// [`page_select`] with an offset, for the reads that page through the catalogue.
///
/// The same sub-select, because SQLite spells an offset and a limit in one
/// clause and there is no `OFFSET` without a `LIMIT`. Still before the join, for
/// [`page_select`]'s reason: the page must be chosen from songs.
fn page_select_offset(keys: &str) -> String {
    format!("SELECT s.id FROM song s WHERE s.published = 1 ORDER BY {keys} LIMIT ?1 OFFSET ?2")
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
        // The column's empty string is the schema's "no slug", not a slug of
        // nothing: a song whose title slugifies to nothing is addressed by its
        // id, exactly as a v3 row was.
        (!head.slug.is_empty()).then_some(head.slug),
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

/// One page of the published catalogue, with its credits, in `order`.
///
/// `limit` counts songs, not credit rows, for [`songs`]'s reason — the offset is
/// applied to the same ids-only sub-select, so the join can never move a song
/// between pages or spend two rows of the page on one song's credits.
///
/// Read by the MCP catalogue (`src/routes/mcp.rs`), whose `list_songs` takes a
/// page number, and by the index's own pages (`pages::songs`, which owns
/// [`PAGE_SIZE`](crate::pages::songs::PAGE_SIZE) and the offset both of them
/// use). One read, two surfaces, so the page an MCP client gets and the page a
/// reader gets are the same twenty songs.
pub async fn songs_page(
    pool: &SqlitePool,
    order: SongOrder,
    limit: i64,
    offset: i64,
) -> DbResult<Vec<Song>> {
    let keys = order.song_keys();
    let sql = format!(
        "{SONGS_SELECT} WHERE s.id IN ({}) ORDER BY {keys}, sa.position",
        page_select_offset(keys)
    );

    let rows = sqlx::query_as::<_, SongArtistRow>(&sql)
        .bind(limit)
        .bind(offset)
        .fetch_all(pool)
        .await?;

    assemble(rows)
}

/// Every published song credited to one artist, newest first.
///
/// The artist page's list. The window is the whole catalogue on purpose — an
/// artist with forty songs is still one page — and the filter is a sub-select on
/// `song_artist` rather than a second join, so the `LIMIT`-counts-songs rule that
/// [`songs`] documents holds here too by construction.
pub async fn songs_by_artist(pool: &SqlitePool, artist_id: &str) -> DbResult<Vec<Song>> {
    let keys = SongOrder::Newest.song_keys();
    let sql = format!(
        "{SONGS_SELECT} WHERE s.published = 1 AND s.id IN \
         (SELECT song_id FROM song_artist WHERE artist_id = ?1) \
         ORDER BY {keys}, sa.position"
    );

    let rows = sqlx::query_as::<_, SongArtistRow>(&sql)
        .bind(artist_id)
        .fetch_all(pool)
        .await?;

    assemble(rows)
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

/// A song reached by the URL segment that named it.
///
/// One song URL has two spellings — the slug, which is canonical, and the id,
/// which is what v3 published and what a song with no slug still uses — and
/// three callers have to agree which one they are looking at: the negotiation
/// layer (which turns the other spelling into a 301), the layout (which decides
/// the document head from the path alone) and the page. So the resolution is one
/// function, and [`Addressed::is_canonical`] is the whole answer to "does this
/// URL need to move".
#[derive(Debug, Clone)]
pub struct Addressed {
    song: Song,
    canonical: bool,
}

impl Addressed {
    /// Build one directly.
    ///
    /// [`song_at`] is the real caller; this exists so the layers and the layout
    /// can be tested on a resolved song without a database behind them.
    #[cfg(test)]
    pub(crate) fn new(song: Song, canonical: bool) -> Self {
        Self { song, canonical }
    }

    /// The row.
    pub fn song(&self) -> &Song {
        &self.song
    }

    /// The row, consumed.
    pub fn into_song(self) -> Song {
        self.song
    }

    /// Whether the segment that named this song *is* the song's address.
    ///
    /// False for an id URL of a song that has a slug, and false for a retired
    /// slug — a title the song used to have. Both are a 301 to
    /// [`Song::get_path`], issued directly to the current address: nothing here
    /// ever chains through an intermediate one.
    pub fn is_canonical(&self) -> bool {
        self.canonical
    }
}

/// The song a URL segment names — its slug, its id, or a slug it used to have.
///
/// `None` when the segment names no song at all, which is what lets the caller
/// fall through to the site's 404 rather than invent an error response.
///
/// Drafts are returned: like [`song`], this is the read, not the policy.
pub async fn song_at(pool: &SqlitePool, segment: &str) -> DbResult<Option<Addressed>> {
    // A slug first. It is the namespace the site publishes, `song_slug` holds
    // every slug ever assigned, and [`assign_slug`] refuses to mint one that is
    // a song id — so the two lookups cannot both hit.
    let by_slug: Option<String> =
        sqlx::query_scalar("SELECT song_id FROM song_slug WHERE slug = ?1")
            .bind(segment)
            .fetch_optional(pool)
            .await?;

    let found = match by_slug {
        Some(id) => song(pool, &id).await?,
        None => song(pool, segment).await?,
    };
    let Some(song) = found else {
        return Ok(None);
    };

    let canonical = match song.get_slug() {
        Some(slug) => slug == segment,
        // No slug: the id is the address, and `get_segment` says so.
        None => song.get_id() == segment,
    };

    Ok(Some(Addressed { song, canonical }))
}

/// The songs a list of URL segments names — one entry per segment, in order.
///
/// The bulk form of [`song_at`], for the book (`/puta-himene`):
/// the same rule (slug, then id, then a retired slug) applied to a list. Written
/// as a loop over [`song_at`] rather than as one `IN (…)` on purpose — "a slug,
/// else an id, else a slug it used to have" is not a predicate SQL can be handed
/// for a *list*, and a second implementation of it is exactly the drift
/// [`song_at`]'s own doc comment warns about.
///
/// `None` where a segment names nothing. That is not an error here: the caller
/// decides what a selection with a hole in it means, and both callers
/// (`pages::book` and the JSON read) answer it with the 404 rather than
/// dropping the song.
///
/// Drafts come back with everything else, for [`song`]'s reason: this is the
/// read, not the policy.
pub async fn songs_at(pool: &SqlitePool, segments: &[String]) -> DbResult<Vec<Option<Addressed>>> {
    let mut out: Vec<Option<Addressed>> = Vec::with_capacity(segments.len());
    for segment in segments {
        out.push(song_at(pool, segment).await?);
    }

    Ok(out)
}

/// The slug a title earns, made unique against every slug the site has ever
/// used.
///
/// The rule itself is [`crate::domain::slug::slugify`]; this adds the two things
/// that need the database:
///
/// * **A collision gets a number.** `te-here-fenua`, then `te-here-fenua-2`,
///   then `-3`. The corpus has no two titles that slugify the same (asserted in
///   `domain::slug`), so this is for the title someone adds tomorrow.
/// * **A retired slug is never re-issued.** The candidate is checked against
///   `song_slug`, which holds every slug ever assigned and never loses one — so
///   a slug freed by a rename goes to nobody, and the URL that used to point at
///   one song can never quietly start pointing at another. The song's own rows
///   are excluded, which is what makes re-running the importer a no-op and what
///   lets a title changed back to an old name reclaim its old address.
///
/// Returns the slug written, or `None` when the title has no Latin letters and
/// so has no slug to write.
pub(crate) async fn assign_slug(
    conn: &mut sqlx::SqliteConnection,
    song_id: &str,
    title: &str,
) -> DbResult<Option<String>> {
    let base = crate::domain::slug::slugify(title);
    if base.is_empty() {
        return Ok(None);
    }

    // 99 spellings of one title is not a corpus, it is a bug: fall back to
    // something the id guarantees is unique rather than loop forever.
    let mut candidate = None;
    for n in 1..=99 {
        let attempt = match n {
            1 => base.clone(),
            n => format!("{base}-{n}"),
        };
        if !slug_taken(conn, &attempt, song_id).await? {
            candidate = Some(attempt);
            break;
        }
    }
    let slug = candidate.unwrap_or_else(|| format!("{base}-{song_id}"));

    sqlx::query("UPDATE song SET slug = ?1 WHERE id = ?2")
        .bind(&slug)
        .bind(song_id)
        .execute(&mut *conn)
        .await?;
    // `OR IGNORE`: the song's own earlier row is already here, and a row that is
    // here must never be replaced — that is the whole of "retired slugs stay
    // retired".
    sqlx::query("INSERT OR IGNORE INTO song_slug (slug, song_id) VALUES (?1, ?2)")
        .bind(&slug)
        .bind(song_id)
        .execute(&mut *conn)
        .await?;

    Ok(Some(slug))
}

/// Whether `candidate` is spoken for by another song.
///
/// Two ways it can be: another song holds it in `song_slug` — now or in the
/// past, which is the same table — or it is an existing song's id, which would
/// make one URL mean two things.
async fn slug_taken(
    conn: &mut sqlx::SqliteConnection,
    candidate: &str,
    song_id: &str,
) -> DbResult<bool> {
    let taken: Option<i64> = sqlx::query_scalar(
        "SELECT 1 FROM song_slug WHERE slug = ?1 AND song_id <> ?2 \
         UNION ALL SELECT 1 FROM song WHERE id = ?1 LIMIT 1",
    )
    .bind(candidate)
    .bind(song_id)
    .fetch_optional(&mut *conn)
    .await?;

    Ok(taken.is_some())
}

/// Give a slug to every song that has none, and report how many there were.
///
/// The one-time half of migration `0002_slugs.sql`, in Rust because the rule is
/// in Rust: SQLite has no honest way to spell the transliteration table, and a
/// nested `replace()` chain in the migration would be a second implementation of
/// the rule, free to drift from the first.
///
/// Run from `Db::open`, so a database written before the slug column existed —
/// the working copy's own `data/chansondufenua.db`, or a v4.0 file on the
/// droplet — is addressed by slug from the first request after it is opened. It
/// is idempotent: a database that has slugs has nothing for this to do, and the
/// importer assigns them itself on the way in.
pub async fn backfill_slugs(pool: &SqlitePool) -> DbResult<usize> {
    let missing: Vec<(String, String)> =
        sqlx::query_as("SELECT id, title FROM song WHERE slug = '' ORDER BY title, id")
            .fetch_all(pool)
            .await?;
    if missing.is_empty() {
        return Ok(0);
    }

    let mut tx = pool.begin().await?;
    for (id, title) in &missing {
        assign_slug(&mut tx, id, title).await?;
    }
    tx.commit().await?;

    Ok(missing.len())
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

/// One artist by id, published or not — the read, not the policy.
///
/// The artist page (`pages::artist`) is what decides that an id it cannot
/// resolve is a 404; `routes::negotiation` reads the same row so its `Link`
/// headers do not promise a Markdown form for a page that is not served.
pub async fn artist(pool: &SqlitePool, id: &str) -> DbResult<Option<Artist>> {
    let row = sqlx::query_as::<_, ArtistRow>(
        "SELECT id, fullname, created_at, updated_at FROM artist WHERE id = ?1",
    )
    .bind(id)
    .fetch_optional(pool)
    .await?;

    row.map(ArtistRow::into_artist).transpose()
}

/// The FTS5 `MATCH` expression for a needle a person typed, or `None` when the
/// needle carries no text worth searching for.
///
/// Shared by the artist autocomplete and the catalogue search so the two cannot
/// disagree about what a typed string means: both wrap the needle in a quoted
/// phrase — so FTS5 operators (`*`, `NEAR(`, `"`) are read as text rather than
/// as syntax — and append `*` so the last word is a prefix.
///
/// `None` for a needle with no alphanumeric character at all: the quoted form of
/// `""`, `*` or `()` tokenizes to nothing, and FTS5 answers that with a syntax
/// error rather than with nothing found. A caller that asked for `*` is owed an
/// empty result, not a 500.
pub(crate) fn fts_match(query: &str) -> Option<String> {
    let needle = query.trim();
    if !needle.chars().any(char::is_alphanumeric) {
        return None;
    }

    // FTS5 string literal: wrap in double quotes so operator characters are
    // treated as text, and double any embedded quote. Then `*` for prefix.
    Some(format!("\"{}\"*", needle.replace('"', "\"\"")))
}

/// Artist autocomplete for the editor's credit field.
///
/// Uses the FTS5 index rather than `LIKE`, for the same reason v3 used a
/// `punct_lower_ascii` SEARCH ANALYZER: so `mahoi` finds `Mā'ohi`. Diacritics are
/// stripped and case is folded, but punctuation is *not* — `unicode61` splits on
/// it, so a name containing an ʻokina is indexed as separate tokens and only the
/// trailing token is reachable by prefix. See the tests.
pub async fn search_artists(pool: &SqlitePool, query: &str, limit: i64) -> DbResult<Vec<Artist>> {
    // A blank needle is not a search: the editor's autocomplete shows the whole
    // list until something is typed. That is this function's own rule and not
    // [`fts_match`]'s, which answers `None` for anything with no searchable
    // text in it.
    if query.trim().is_empty() {
        return artists(pool).await;
    }

    let Some(match_expr) = fts_match(query) else {
        return Ok(Vec::new());
    };

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
// Search
// ---------------------------------------------------------------------------

/// The published songs whose **title** matches a typed needle, newest first.
///
/// Title only — the lyrics are not indexed, for the reason
/// `migrations/0003_search.sql` records: a common French word in a 6000-character
/// lyric would return the whole catalogue. Diacritics and case are folded by the
/// index, so `mama` finds `Māmā Tahiti` and `ahani` finds `'Āhani e`.
///
/// The ids come from the FTS index and the rows from the same projection the
/// index page uses, so a hit carries its credits and a draft can never appear:
/// both halves of that sentence are the point of searching through `song` rather
/// than through the index alone.
pub async fn search_songs(pool: &SqlitePool, query: &str, limit: i64) -> DbResult<Vec<Song>> {
    let Some(match_expr) = fts_match(query) else {
        return Ok(Vec::new());
    };

    let keys = SongOrder::Newest.song_keys();
    let sql = format!(
        "{SONGS_SELECT} WHERE s.published = 1 AND s.id IN \
         (SELECT s2.id FROM song_fts JOIN song s2 ON s2.rowid = song_fts.rowid \
          WHERE song_fts MATCH ?1) \
         ORDER BY {keys}, sa.position LIMIT ?2"
    );

    let rows = sqlx::query_as::<_, SongArtistRow>(&sql)
        .bind(match_expr)
        .bind(limit)
        .fetch_all(pool)
        .await?;

    assemble(rows)
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

    // Refuse before opening a transaction. The slug is not this stub's business
    // — it is minted below, once the row exists — so it passes `None`.
    Song::new(
        String::new(),
        None,
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

    // The song row exists first, because `song_slug.song_id` is a foreign key:
    // the address is written by the same function the importer and the backfill
    // use, so a new song cannot be minted an address by a third rule.
    assign_slug(&mut tx, &song_id, &title).await?;

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

    // -- slugs --------------------------------------------------------------

    /// The two spellings of one song URL, and which of them is the address.
    #[tokio::test]
    async fn a_song_resolves_by_slug_and_by_id_and_only_the_slug_is_canonical() {
        let db = seeded().await;
        let fixture = &fixtures::SONGS[0];

        let by_slug = song_at(db.pool(), fixture.slug).await.unwrap().unwrap();
        assert!(by_slug.is_canonical());
        assert_eq!(by_slug.song().get_id(), fixture.id);
        assert_eq!(
            by_slug.song().get_path(),
            format!("/himene/{}", fixture.slug)
        );

        // The id URL resolves to the same row and says it has to move.
        let by_id = song_at(db.pool(), fixture.id).await.unwrap().unwrap();
        assert!(!by_id.is_canonical());
        assert_eq!(by_id.song().get_id(), fixture.id);
        assert_eq!(by_id.song().get_path(), format!("/himene/{}", fixture.slug));

        // A segment that names nobody is not an error and not a song.
        assert!(
            song_at(db.pool(), "does-not-exist")
                .await
                .unwrap()
                .is_none()
        );
    }

    /// A list of segments resolves the way the single one does — the same order
    /// out as in, a hole where a segment names nothing, and the canonical flag
    /// carried per row so the caller can tell an id URL from an address.
    ///
    /// This is the read behind `/puta-himene`: the page's own order is the
    /// reader's reading order, so an implementation that sorted or deduped here
    /// would be reordering a selection the reader made.
    #[tokio::test]
    async fn a_list_of_segments_keeps_its_order_and_its_holes() {
        let db = seeded().await;
        let first = &fixtures::SONGS[0];
        let second = &fixtures::SONGS[1];

        let segments = vec![
            second.slug.to_owned(),
            "does-not-exist".to_owned(),
            first.id.to_owned(),
            first.slug.to_owned(),
        ];
        let found = songs_at(db.pool(), &segments).await.unwrap();
        assert_eq!(found.len(), segments.len());

        assert_eq!(
            found[0].as_ref().map(|row| row.song().get_id()),
            Some(second.id.to_owned())
        );
        assert!(found[1].is_none(), "a segment that names nobody is a hole");
        assert_eq!(
            found[2].as_ref().map(|row| row.song().get_id()),
            Some(first.id.to_owned())
        );
        assert!(
            !found[2].as_ref().unwrap().is_canonical(),
            "an id URL is not the song's address, even in a selection"
        );
        assert!(found[3].as_ref().unwrap().is_canonical());

        assert!(songs_at(db.pool(), &[]).await.unwrap().is_empty());
    }

    /// Seeding leaves the fixtures with the slugs the rule produces for their
    /// titles, and the history rows that make them resolve.
    #[tokio::test]
    async fn the_fixtures_carry_their_slugs_and_their_history() {
        let db = seeded().await;

        for fixture in fixtures::SONGS {
            let found = song(db.pool(), fixture.id).await.unwrap().unwrap();
            assert_eq!(found.get_slug().as_deref(), Some(fixture.slug));
            assert!(
                song_at(db.pool(), fixture.slug).await.unwrap().is_some(),
                "{} is not reachable by its slug",
                fixture.slug
            );
        }
    }

    /// A rename moves the song and leaves the old slug pointing at it: the
    /// address that used to work redirects instead of 404ing, and the redirect
    /// names the *current* slug rather than the retired one — there is nothing
    /// to chain through.
    #[tokio::test]
    async fn a_renamed_song_keeps_its_retired_slug_pointing_at_it() {
        let db = seeded().await;
        let fixture = &fixtures::SONGS[2]; // "Te here fenua" → te-here-fenua

        let mut tx = db.pool().begin().await.unwrap();
        let title = "Te here fenua nei";
        sqlx::query("UPDATE song SET title = ?1 WHERE id = ?2")
            .bind(title)
            .bind(fixture.id)
            .execute(&mut *tx)
            .await
            .unwrap();
        let slug = assign_slug(&mut tx, fixture.id, title).await.unwrap();
        tx.commit().await.unwrap();

        assert_eq!(slug.as_deref(), Some("te-here-fenua-nei"));

        let retired = song_at(db.pool(), fixture.slug).await.unwrap().unwrap();
        assert!(!retired.is_canonical(), "a retired slug is not the address");
        assert_eq!(retired.song().get_id(), fixture.id);
        assert_eq!(
            retired.song().get_path(),
            "/himene/te-here-fenua-nei",
            "the redirect has to land on the current address in one hop"
        );

        // ...and the current one is canonical, so it does not redirect to itself.
        let current = song_at(db.pool(), "te-here-fenua-nei")
            .await
            .unwrap()
            .unwrap();
        assert!(current.is_canonical());
    }

    /// The rule the whole history table exists for: a slug, once published, is
    /// spoken for. A new song with the old title gets a numbered address rather
    /// than stealing the URL — which would silently repoint every link anyone
    /// ever made to the first song.
    #[tokio::test]
    async fn a_retired_slug_is_never_issued_to_another_song() {
        let db = seeded().await;
        let first = fixtures::SONGS[2].id; // holds "te-here-fenua"
        let retired = fixtures::SONGS[2].slug;

        // Move the first song off its slug...
        let mut tx = db.pool().begin().await.unwrap();
        sqlx::query("UPDATE song SET title = 'Autre titre' WHERE id = ?1")
            .bind(first)
            .execute(&mut *tx)
            .await
            .unwrap();
        assign_slug(&mut tx, first, "Autre titre").await.unwrap();
        tx.commit().await.unwrap();

        // ...and give the freed title to a brand-new song.
        let created = create_song(db.pool(), "Te here fenua", &lyrics_of(120), "")
            .await
            .unwrap();

        assert_eq!(created.get_slug().as_deref(), Some("te-here-fenua-2"));
        // The URL still belongs to the song that published it.
        let still = song_at(db.pool(), retired).await.unwrap().unwrap();
        assert_eq!(still.song().get_id(), first);
        assert_ne!(still.song().get_id(), created.get_id());
    }

    /// Assigning is idempotent, which is what lets the importer run over the same
    /// dump again without moving a single URL.
    #[tokio::test]
    async fn assigning_the_same_title_twice_keeps_the_same_slug() {
        let db = seeded().await;
        let fixture = &fixtures::SONGS[1];

        let mut tx = db.pool().begin().await.unwrap();
        let again = assign_slug(&mut tx, fixture.id, fixture.title)
            .await
            .unwrap();
        tx.commit().await.unwrap();

        assert_eq!(
            again.as_deref(),
            Some(fixture.slug),
            "the slug moved on a second assignment"
        );
    }

    /// A title with no Latin letters earns no slug, and the song keeps the
    /// address v3 gave it.
    #[tokio::test]
    async fn a_song_whose_title_has_no_latin_letters_keeps_its_id() {
        let db = fresh().await;
        let created = create_song(db.pool(), "日本語のうた", &lyrics_of(120), "")
            .await
            .unwrap();

        assert_eq!(created.get_slug(), None);
        assert_eq!(created.get_path(), format!("/himene/{}", created.get_id()));

        // ...and it is canonical there: nothing to redirect.
        let found = song_at(db.pool(), &created.get_id())
            .await
            .unwrap()
            .unwrap();
        assert!(found.is_canonical());
    }

    /// The backfill is the Rust half of migration `0002_slugs.sql`: a row written
    /// straight to the table — which is what a v4.0 database holds — gets its
    /// address when the database is opened, once.
    #[tokio::test]
    async fn the_backfill_addresses_rows_that_never_had_a_slug() {
        let db = fresh().await;
        sqlx::query(
            "INSERT INTO song (id, title, lyrics, view_count, published, created_at, updated_at) \
             VALUES ('old0000000000000000', 'Māmā Tahiti', ?1, 1, 1, ?2, ?2)",
        )
        .bind(lyrics_of(120))
        .bind("2024-01-01T00:00:00.000000000Z")
        .execute(db.pool())
        .await
        .unwrap();

        assert_eq!(backfill_slugs(db.pool()).await.unwrap(), 1);
        let addressed = song(db.pool(), "old0000000000000000")
            .await
            .unwrap()
            .unwrap();
        assert_eq!(addressed.get_slug().as_deref(), Some("mama-tahiti"));

        // Nothing left to do the second time, and nothing moved.
        assert_eq!(backfill_slugs(db.pool()).await.unwrap(), 0);
    }

    /// A slug can never be an id, or one URL would be two things.
    #[tokio::test]
    async fn a_slug_is_never_minted_that_is_a_song_id() {
        let db = fresh().await;
        let id = "abcdefghijklmnopqrst";
        sqlx::query(
            "INSERT INTO song (id, slug, title, lyrics, view_count, published, created_at, updated_at) \
             VALUES (?1, '', ?1, ?2, 1, 1, ?3, ?3)",
        )
        .bind(id)
        .bind(lyrics_of(120))
        .bind("2024-01-01T00:00:00.000000000Z")
        .execute(db.pool())
        .await
        .unwrap();

        let mut tx = db.pool().begin().await.unwrap();
        let slug = assign_slug(&mut tx, id, id).await.unwrap();
        tx.commit().await.unwrap();

        assert_eq!(slug.as_deref(), Some("abcdefghijklmnopqrst-2"));
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

    /// Paging is a partition: pages do not overlap, do not skip a song, and the
    /// two-credit song still arrives whole.
    ///
    /// The offset lives in the ids-only sub-select, so a page is a set of songs
    /// before the join decorates it — the same bug class as the `LIMIT` one
    /// above, one level along.
    #[tokio::test]
    async fn paging_covers_the_catalogue_once_and_keeps_credits_whole() {
        let db = seeded().await;
        let all = songs(db.pool(), SongOrder::Newest, None).await.unwrap();
        let per_page: i64 = 5;

        let mut seen: Vec<String> = Vec::new();
        let mut page: i64 = 0;
        loop {
            let got = songs_page(db.pool(), SongOrder::Newest, per_page, page * per_page)
                .await
                .unwrap();
            if got.is_empty() {
                break;
            }
            seen.extend(got.iter().map(Song::get_id));
            page += 1;
            assert!(page < 20, "paging did not terminate");
        }

        let expected: Vec<String> = all.iter().map(Song::get_id).collect();
        assert_eq!(seen, expected, "the pages are not the catalogue, in order");

        // Past the end is empty: not an error, and not a wrap back to the top.
        assert!(
            songs_page(db.pool(), SongOrder::Newest, per_page, 1_000)
                .await
                .unwrap()
                .is_empty()
        );

        // A page of one still carries the busiest song's two credits.
        let first = songs_page(db.pool(), SongOrder::MostViewed, 1, 0)
            .await
            .unwrap();
        assert_eq!(first.len(), 1);
        assert_eq!(first[0].get_artists().len(), 2);
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
