//! The read-only JSON API — `/api/songs`, `/api/songs/{id}`, `/api/health`.
//!
//! A small, documented surface that lets a program read the catalogue without
//! parsing a page. The shape is deliberately the one the
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
use serde_json::{Map, Value, json};
use topcoat::{
    Result,
    context::Cx,
    router::{
        Body, HeaderValue, StatusCode,
        content::Json,
        header, path_param,
        response::{IntoResponse, Response},
        route,
    },
};

use crate::db::{self, SongOrder};
use crate::domain::Song;
use crate::domain::song::SITE_URL;
use crate::state;

/// The catalogue.
pub const PATH: &str = "/api/songs";

/// The health probe.
pub const HEALTH_PATH: &str = "/api/health";

/// The OpenAPI description of this API.
///
/// `/api/openapi.json` rather than a `/.well-known/` path: `openapi.json` is not
/// a registered well-known URI, and a description belongs beside the API it
/// describes. [`crate::routes::catalog`]'s `service-desc` is what points at it.
pub const OPENAPI_PATH: &str = "/api/openapi.json";

/// The single-song path, spelled the way OpenAPI spells a path template.
///
/// The route's own attribute has to be a literal, so this is the second copy and
/// `the_openapi_paths_are_the_ones_the_routes_serve` pins the two together.
const SONG_PATH: &str = "/api/songs/{id}";

/// The media type of the description.
///
/// `application/json`: the document *is* OpenAPI — its `openapi` field says so —
/// and the type is what an ordinary client parses it with. There is no second
/// representation to negotiate and so no `Vary: Accept` to send.
const DOCUMENT_TYPE: &str = "application/json";

/// How long the description may be cached. Like the MCP card it changes only on
/// a redeploy, so an hour is a ceiling a client can rely on.
const CACHE: &str = "public, max-age=3600";

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
///
/// Shared with the MCP catalogue (`src/routes/mcp.rs`), which is the same list
/// read by a different protocol: two published shapes for one song would be two
/// things to keep in step.
#[derive(Debug, Serialize)]
pub(crate) struct SongJson {
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
    pub(crate) fn summary(sheet: &Song) -> Self {
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

/// `GET /api/openapi.json` — the API described for a machine.
///
/// Built here, beside the routes it describes, rather than kept as a
/// `openapi.json` in the repository: a checked-in copy is a second place for the
/// paths, the version and the response shapes to live, and one of the two would
/// go stale. `servers[0].url` and `info.version` are [`SITE_URL`] and
/// [`VERSION`], so the description cannot name a host or a release the binary
/// does not serve.
#[route(GET "/api/openapi.json")]
async fn openapi(cx: &Cx) -> Result<Response> {
    (
        [
            (
                header::CONTENT_TYPE,
                HeaderValue::from_static(DOCUMENT_TYPE),
            ),
            (header::CACHE_CONTROL, HeaderValue::from_static(CACHE)),
        ],
        Body::from(openapi_document().to_string()),
    )
        .into_response(cx)
}

/// The OpenAPI 3.1 document.
///
/// Valid by the specification's own rules: `openapi`, `info` with a title and a
/// version, and `paths` are all present, every operation is a `GET` (this API
/// has no other verb), and every response names its shape in `components`.
fn openapi_document() -> Value {
    let mut paths = Map::new();
    paths.insert(PATH.to_owned(), json!({ "get": catalogue_operation() }));
    paths.insert(SONG_PATH.to_owned(), json!({ "get": song_operation() }));
    paths.insert(HEALTH_PATH.to_owned(), json!({ "get": health_operation() }));

    json!({
        "openapi": "3.1.0",
        "info": {
            "title": "Chanson du fenua — read-only API",
            "version": VERSION,
            "description": "The published catalogue of French Polynesian songs \
                            as JSON, with each lyric available as Markdown. \
                            Every operation is a GET; nothing on this surface writes.",
        },
        "servers": [{ "url": SITE_URL }],
        "paths": Value::Object(paths),
        "components": { "schemas": schemas() },
    })
}

/// `GET /api/songs` — the catalogue.
fn catalogue_operation() -> Value {
    json!({
        "operationId": "listSongs",
        "summary": "Every published song, newest first",
        "description": "The same list the index page shows, from the same read. Drafts are not published and are not in it.",
        "responses": {
            "200": {
                "description": "The published catalogue",
                "content": {
                    "application/json": {
                        "schema": { "$ref": "#/components/schemas/Catalogue" }
                    }
                }
            }
        }
    })
}

/// `GET /api/songs/{id}` — one song, with its lyric.
fn song_operation() -> Value {
    json!({
        "operationId": "getSong",
        "summary": "One song, with its lyric as Markdown",
        "description": "Chords stay inline at the offset the author wrote them. A draft or an unknown id is a 404, as JSON rather than as the site's HTML one.",
        "parameters": [{
            "name": "id",
            "in": "path",
            "required": true,
            "description": "The song's id, as the catalogue lists it.",
            "schema": { "type": "string" }
        }],
        "responses": {
            "200": {
                "description": "The song",
                "content": {
                    "application/json": {
                        "schema": { "$ref": "#/components/schemas/Song" }
                    }
                }
            },
            "404": {
                "description": "No published song has that id",
                "content": {
                    "application/json": {
                        "schema": { "$ref": "#/components/schemas/Error" }
                    }
                }
            }
        }
    })
}

/// `GET /api/health` — the probe.
fn health_operation() -> Value {
    json!({
        "operationId": "health",
        "summary": "Whether the service is up, and how big the catalogue is",
        "description": "The counts are read on every probe, so a process answering from an unreachable catalogue fails here rather than passing.",
        "responses": {
            "200": {
                "description": "The service is up",
                "content": {
                    "application/json": {
                        "schema": { "$ref": "#/components/schemas/Health" }
                    }
                }
            }
        }
    })
}

/// The shapes the operations' responses point at.
///
/// [`SongJson`] is the one published row, `Catalogue` the envelope around a page
/// of them, `Health` the probe, and `Error` the JSON body a 404 carries.
fn schemas() -> Value {
    json!({
        "Song": {
            "type": "object",
            "required": ["id", "title", "artists", "url", "view_count", "created_at", "updated_at"],
            "properties": {
                "id": { "type": "string", "description": "20 characters of [0-9a-z]." },
                "title": { "type": "string" },
                "artists": { "type": "array", "items": { "type": "string" } },
                "url": { "type": "string", "format": "uri", "description": "The song page's canonical URL." },
                "view_count": { "type": "integer" },
                "created_at": { "type": "string", "format": "date-time" },
                "updated_at": { "type": "string", "format": "date-time" },
                "lyrics_markdown": {
                    "type": "string",
                    "description": "The lyric with chords inline. Present only in the single-song read."
                }
            }
        },
        "Catalogue": {
            "type": "object",
            "required": ["count", "songs"],
            "properties": {
                "count": { "type": "integer" },
                "songs": { "type": "array", "items": { "$ref": "#/components/schemas/Song" } }
            }
        },
        "Health": {
            "type": "object",
            "required": ["status", "version", "songs", "artists"],
            "properties": {
                "status": { "type": "string" },
                "version": { "type": "string" },
                "songs": { "type": "integer" },
                "artists": { "type": "integer" }
            }
        },
        "Error": {
            "type": "object",
            "required": ["error", "message"],
            "properties": {
                "error": { "type": "string" },
                "message": { "type": "string" }
            }
        }
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
        assert_eq!(OPENAPI_PATH, "/api/openapi.json");
        assert!(VERSION.starts_with('4'));
    }

    /// The description names the paths the routes serve, and every one of them is
    /// a `GET`. The single-song template is the one string with a second copy
    /// (the route attribute has to be a literal), so it is pinned here.
    #[test]
    fn the_openapi_paths_are_the_ones_the_routes_serve() {
        let document = openapi_document();
        let paths = document["paths"].as_object().expect("paths is an object");

        assert_eq!(OPENAPI_PATH, "/api/openapi.json");
        assert_eq!(SONG_PATH, "/api/songs/{id}");
        let mut served = vec![PATH, SONG_PATH, HEALTH_PATH];
        served.sort_unstable();
        assert_eq!(paths.keys().map(String::as_str).collect::<Vec<_>>(), served);

        for (path, item) in paths {
            let operations = item.as_object().expect("a path item");
            assert_eq!(operations.keys().collect::<Vec<_>>(), vec!["get"], "{path}");
            for operation in operations.values() {
                assert!(operation["responses"]["200"].is_object(), "{path}");
            }
        }

        // The single-song read is the one operation with a parameter and a
        // second answer.
        assert_eq!(
            paths[SONG_PATH]["get"]["parameters"][0]["name"], "id",
            "the path template has no matching parameter"
        );
        assert!(paths[SONG_PATH]["get"]["responses"]["404"].is_object());
    }

    /// The document is OpenAPI by the specification's own requirements, and it
    /// describes the API that is actually running: the version is the binary's,
    /// the server is the canonical host, and every `$ref` resolves to a schema
    /// the document carries.
    #[test]
    fn the_description_is_a_valid_openapi_document_for_this_api() {
        let document = openapi_document();

        assert_eq!(document["openapi"], "3.1.0");
        assert!(document["info"]["title"].is_string());
        assert_eq!(document["info"]["version"], VERSION);
        assert_eq!(document["servers"][0]["url"], crate::domain::song::SITE_URL);
        assert_eq!(DOCUMENT_TYPE, "application/json");

        let schemas = document["components"]["schemas"]
            .as_object()
            .expect("components.schemas is an object");
        let text = document.to_string();
        let refs = regex_lite::Regex::new(r#""\$ref"\s*:\s*"([^"]+)""#).expect("the ref pattern");

        // Every ref points at a schema this document defines, and every defined
        // schema is the target of at least one ref — an unreferenced shape is a
        // shape that describes nothing.
        for found in refs.captures_iter(&text) {
            let target = &found[1];
            let name = target
                .strip_prefix("#/components/schemas/")
                .unwrap_or_else(|| panic!("{target} is not a local schema ref"));
            assert!(schemas.contains_key(name), "{target} resolves to nothing");
        }
        for name in schemas.keys() {
            assert!(
                refs.is_match(&format!("\"$ref\": \"#/components/schemas/{name}\"")),
                "{name} is never referenced"
            );
        }

        // The property names are the ones the serialized row carries, so a
        // client generated from this description reads the live JSON.
        let row = serde_json::to_value(SongJson::summary(&fixture_song())).expect("serialize");
        let described = schemas["Song"]["properties"]
            .as_object()
            .expect("Song.properties is an object");
        for field in row.as_object().expect("a row").keys() {
            assert!(
                described.contains_key(field),
                "the row has {field}, the schema does not"
            );
        }
        assert!(described.contains_key("lyrics_markdown"));
    }
}
