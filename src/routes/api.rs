//! The read-only JSON API — `/api/songs`, `/api/songs/{id}`, `/api/health`.
//!
//! `PLAN.md` §6 asks for "a small, documented surface" that lets a program read
//! the catalogue without parsing a page. The shape is deliberately the one the
//! sitemap and the Markdown negotiation already assume: one URL per song, the
//! same ids, the same canonical host, and a `Link` header on the page that
//! points here (see [`crate::routes::negotiation`]).
//!
//! Three decisions worth naming:
//!
//! * **The lyrics are Markdown, and only on the single-song read.** The column
//!   holds sanitised HTML, which is what the sheet renders; a program wants
//!   words and `[chord]` marks, which is what [`Song::lyrics_markdown`] produces
//!   and what the `Accept: text/markdown` negotiation serves. The catalogue
//!   leaves the field out: 43 lyrics is most of a megabyte, and a listing is not
//!   a download.
//! * **Artists are names.** There is no artist resource to dereference and no
//!   page per artist, so an id here would advertise a relationship the API does
//!   not have. The sheet prints the same names, in the same order.
//! * **A missing song is a JSON 404**, not the site's HTML one. The error body
//!   is part of the surface: a caller that asked for JSON should not have to
//!   parse a document to find out it was wrong.
//!
//! Nothing here writes. The only writable route on the site is the create-song
//! form, and `robots.txt` keeps crawlers out of it.

use serde::Serialize;
use topcoat::{
    Result,
    context::Cx,
    router::{StatusCode, content::Json, path_param, route},
};

use crate::db::{self, SongOrder};
use crate::domain::Song;
use crate::state;

/// The catalogue.
pub const PATH: &str = "/api/songs";

/// The health probe.
pub const HEALTH_PATH: &str = "/api/health";

/// The crate version, read from `Cargo.toml`.
///
/// `env!` rather than a hand-kept constant: the version in `Cargo.toml` is the
/// one the release tags, and a second copy is a second thing to forget.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

// The `{id}` in the single-song path.
//
// Unparsed: a song id is opaque text, and the read is what decides whether it
// names anything.
path_param!(id);

/// One song, as the API writes it.
///
/// A type of its own rather than `serde` on [`Song`], because the two want
/// different things from the same row: the entity's own fields are private and
/// carry the raw HTML column, and this is a published shape that a caller can
/// rely on.
#[derive(Debug, Serialize)]
struct SongJson {
    id: String,
    title: String,
    /// Every credited artist's full name, in credit order.
    artists: Vec<String>,
    /// The song's canonical URL, on the canonical host.
    url: String,
    view_count: u32,
    created_at: String,
    updated_at: String,
    /// The lyric as Markdown, chords inline. Absent from the catalogue.
    #[serde(skip_serializing_if = "Option::is_none")]
    lyrics_markdown: Option<String>,
}

impl SongJson {
    /// The row without the lyric: what `/api/songs` lists.
    fn summary(sheet: &Song) -> Self {
        Self::of(sheet, None)
    }

    /// The row with the lyric as Markdown: what `/api/songs/{id}` answers.
    fn full(sheet: &Song) -> Self {
        Self::of(sheet, Some(sheet.lyrics_markdown()))
    }

    fn of(sheet: &Song, lyrics_markdown: Option<String>) -> Self {
        Self {
            id: sheet.get_id(),
            title: sheet.get_title(),
            artists: sheet
                .get_artists()
                .iter()
                .map(|artist| artist.get_fullname())
                .collect(),
            url: sheet.get_url(),
            view_count: sheet.get_view_count(),
            created_at: sheet.get_created_at().to_rfc3339(),
            updated_at: sheet.get_updated_at().to_rfc3339(),
            lyrics_markdown,
        }
    }
}

/// `GET /api/health` — whether the service is up, and how big the catalogue is.
///
/// The counts are read from the database on every probe, because "the process
/// answered" is not the interesting question: a binary serving 500s with an
/// unreachable database would pass that. The query behind this counts rather
/// than loads, and counts what the site serves — drafts are not songs the public
/// catalogue has.
#[route(GET "/api/health")]
async fn health(cx: &Cx) -> Result<Json<serde_json::Value>> {
    let counts = db::counts(state::db(cx).pool()).await?;

    Ok(Json(serde_json::json!({
        "status": "ok",
        "version": VERSION,
        "songs": counts.songs,
        "artists": counts.artists,
    })))
}

/// `GET /api/songs` — the published catalogue, newest first.
///
/// The order is the index's, so a caller paging through the two sees the same
/// thing. The envelope carries the count as well as the list: a program that
/// wants to know whether it has them all should not have to count them.
#[route(GET "/api/songs")]
async fn catalogue(cx: &Cx) -> Result<Json<serde_json::Value>> {
    let listed = db::songs(state::db(cx).pool(), SongOrder::Newest, None).await?;

    Ok(Json(serde_json::json!({
        "count": listed.len(),
        "songs": listed.iter().map(SongJson::summary).collect::<Vec<_>>(),
    })))
}

/// `GET /api/songs/{id}` — one song, with its lyric as Markdown.
///
/// **A draft is not found**, the same rule the sheet applies and for the same
/// reason: `db::song` deliberately returns drafts so the editor can see them,
/// and this surface is public.
#[route(GET "/api/songs/{id}")]
async fn one(cx: &Cx) -> Result<(StatusCode, Json<serde_json::Value>)> {
    let id: &str = path_param::<Id>(cx);

    let found = db::song(state::db(cx).pool(), id)
        .await?
        .filter(Song::is_published);

    Ok(match found {
        Some(sheet) => (
            StatusCode::OK,
            Json(serde_json::json!(SongJson::full(&sheet))),
        ),
        None => (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({
                "error": "not_found",
                "message": format!("no published song with id {id:?}"),
            })),
        ),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::{Db, fixtures};

    fn fixture_song() -> Song {
        let fixture = &fixtures::SONGS[0];
        Song::new(
            fixture.id.to_owned(),
            fixture.title.to_owned(),
            fixture.lyrics.to_owned(),
            7,
            Vec::new(),
            true,
            chrono::Utc::now(),
            chrono::Utc::now(),
        )
    }

    /// The catalogue leaves the lyric out and the single-song read keeps it in.
    /// Asserted on the serialized value rather than on the struct, because the
    /// field is dropped by `serde`, not by the constructor.
    #[test]
    fn the_lyric_is_in_the_one_song_read_and_absent_from_the_catalogue() {
        let sheet = fixture_song();

        let listed = serde_json::to_value(SongJson::summary(&sheet)).expect("serialize");
        assert!(listed.get("lyrics_markdown").is_none());
        assert_eq!(listed["id"], sheet.get_id());

        let detail = serde_json::to_value(SongJson::full(&sheet)).expect("serialize");
        assert!(
            detail["lyrics_markdown"]
                .as_str()
                .is_some_and(|lyric| !lyric.is_empty()),
            "the lyric is missing from the single-song read"
        );
    }

    /// The URL a caller is handed is the one the page and the sitemap use, on
    /// the canonical host. Three spellings of a song's address would be three
    /// things to keep in step.
    #[test]
    fn the_url_is_the_song_pages_own() {
        use crate::domain::song::SITE_URL;

        let sheet = fixture_song();
        let detail = serde_json::to_value(SongJson::full(&sheet)).expect("serialize");

        assert_eq!(detail["url"], sheet.get_url());
        assert!(
            detail["url"]
                .as_str()
                .is_some_and(|url| url.starts_with(SITE_URL))
        );
    }

    /// A draft is readable and not published — the rule the sheet's own test
    /// states, asserted here because this route is public too.
    #[tokio::test]
    async fn a_draft_is_not_published() {
        let db = Db::open_in_memory().await.expect("in-memory database");
        fixtures::seed(db.pool()).await.expect("seed fixtures");

        sqlx::query(
            "INSERT INTO song (id, title, lyrics, view_count, published, created_at, updated_at) \
             VALUES ('draft0000000000000', 'Un brouillon', ?1, 1, 0, ?2, ?2)",
        )
        .bind("z".repeat(crate::domain::song::LYRICS_MIN))
        .bind("2026-01-01T00:00:00.000000000Z")
        .execute(db.pool())
        .await
        .expect("insert a draft");

        let draft = db::song(db.pool(), "draft0000000000000")
            .await
            .expect("the read")
            .filter(Song::is_published);

        assert!(draft.is_none());
    }

    /// `/api/health` reports a count, so the count has to be of what the site
    /// serves. It is also the only place the crate reads a number without
    /// reading the rows, which is the whole point of the query.
    #[tokio::test]
    async fn health_counts_the_published_songs_it_can_serve() {
        let db = Db::open_in_memory().await.expect("in-memory database");
        fixtures::seed(db.pool()).await.expect("seed fixtures");

        let counts = db::counts(db.pool()).await.expect("the counts");
        assert_eq!(counts.songs as usize, fixtures::SONGS.len());
        assert!(counts.artists >= 1);
    }

    #[test]
    fn the_route_constants_are_the_paths_the_docs_claim() {
        assert_eq!(PATH, "/api/songs");
        assert_eq!(HEALTH_PATH, "/api/health");
        assert!(VERSION.starts_with('4'));
    }
}
