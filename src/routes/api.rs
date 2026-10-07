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
//! * **`id` is the key, `url` is the address.** The address is the slug
//!   (`/himene/ahani-e`), which is what a reader's link points at; the id is what
//!   this API's single-song read is keyed by, so `/api/songs/{id}` never has to
//!   move when a title changes. Two fields, two jobs, neither derived from the
//!   other.
//! * **A missing song is a JSON 404**, not the site's HTML one. The error body
//!   is part of the surface: a caller that asked for JSON should not have to
//!   parse a document to find out it was wrong.
//! * **The catalogue takes an optional selection.** `?s={slug}` repeated is the
//!   multi-lyric page's own URL (`pages::book`), and asking this endpoint for
//!   it answers the same songs in the order the query named them, **with their
//!   lyrics** — that page's whole content is the lyrics, so its JSON form has
//!   nothing to say without them. A selection that names an unpublished song is
//!   the same 404 as a missing id: the page does not serve a selection with a
//!   hole in it, and neither does this. The parameter is read by
//!   `book::selection`, so the page and this read cannot disagree about which
//!   query string is a selection.
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
        request::uri,
        response::{IntoResponse, Response},
        route,
    },
};

use crate::db::{self, SongOrder};
use crate::domain::song::SITE_URL;
use crate::domain::{Artist, Song};
use crate::i18n;
use crate::pages::{artist, book, search, support};
use crate::state;

/// The catalogue.
pub const PATH: &str = "/api/songs";

/// The search endpoint: `/api/search?q=…`.
///
/// The machine-readable half of [`crate::pages::search`]: the same needle, the
/// same two reads, the same limit. The page's own form and this endpoint share
/// `search::needle`, so the string a browser submits and the string a client
/// sends are read identically.
pub const SEARCH_PATH: &str = "/api/search";

/// The health probe.
pub const HEALTH_PATH: &str = "/api/health";

/// The support page's addresses, as JSON: `/api/support`.
///
/// The machine-readable half of [`crate::pages::support`]: the same three
/// addresses the page prints, byte for byte, and the page's own URL. An address
/// is exactly the thing a program must not retype from a rendering — and the EVM
/// one's case *is* its checksum — so it is published as data rather than only as
/// text inside markup.
pub const SUPPORT_PATH: &str = "/api/support";

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
    /// The song page's address, on the canonical host.
    ///
    /// The page's own URL — the slug — which since v4.2 is the one address the
    /// sheet has: the chrome's language is resolved per request and is not part
    /// of the URL, so this document names the same address the page's
    /// `<link rel="canonical">` does.
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

/// One artist, as the search answers it.
///
/// A shape of its own rather than `serde` on [`Artist`], for the reason
/// [`SongJson`] gives: the entity's fields are private and the API is a
/// published shape. There is no `/api/artists/{id}` — the machine-readable
/// resource for an artist is its *page*, so the row carries the page's URL and
/// nothing else the page does not already say.
#[derive(Debug, Serialize)]
pub(crate) struct ArtistJson {
    id: String,
    name: String,
    /// The artist page's address, on the canonical host: `/taata-himene/{id}`,
    /// the same URL the search page links to and the page canonicalises to.
    url: String,
}

impl ArtistJson {
    pub(crate) fn of(artist: &Artist) -> Self {
        Self {
            id: artist.get_id(),
            name: artist.get_fullname(),
            url: i18n::absolute(&artist::path_of(&artist.get_id())),
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

/// `GET /api/songs` — the published catalogue, newest first, or one selection.
///
/// The catalogue's order is the index's, so a caller paging through the two sees
/// the same thing. The envelope carries the count as well as the list: a program
/// that wants to know whether it has them all should not have to count them.
///
/// `?s={slug}`, repeated, is the multi-lyric page's URL: the same songs in the
/// order the query named them, each with its lyric, and a 404 for a selection
/// that names no published song. Two shapes, one path, for the same reason the
/// site has one multi-lyric page: a selection is a way of reading the catalogue,
/// not a second resource.
#[route(GET "/api/songs")]
async fn catalogue(cx: &Cx) -> Result<(StatusCode, Json<serde_json::Value>)> {
    let selected = book::selection(uri(cx).query().unwrap_or(""));

    if !selected.is_empty() {
        let query = book::query(&selected);
        return Ok(
            match book::resolve(state::db(cx).pool(), &selected).await? {
                Some(sheets) => (
                    StatusCode::OK,
                    Json(serde_json::json!({
                        "count": sheets.len(),
                        "songs": sheets.iter().map(SongJson::full).collect::<Vec<_>>(),
                    })),
                ),
                // The message names the selection rather than the offending segment:
                // the read is `pages::book`'s and it answers with the whole
                // selection's fate by design, so naming one segment here would mean
                // resolving them a second time in this file.
                None => (
                    StatusCode::NOT_FOUND,
                    Json(serde_json::json!({
                        "error": "not_found",
                        "message": format!("no published song for the selection {query:?}"),
                    })),
                ),
            },
        );
    }

    let listed = db::songs(state::db(cx).pool(), SongOrder::Newest, None).await?;

    Ok((
        StatusCode::OK,
        Json(serde_json::json!({
            "count": listed.len(),
            "songs": listed.iter().map(SongJson::summary).collect::<Vec<_>>(),
        })),
    ))
}

/// `GET /api/search` — the catalogue's titles and the artists' names, by needle.
///
/// The same two reads the search page makes, from the same function
/// ([`search::run`]), so the HTML and the JSON cannot disagree about what a
/// needle found. The matching is accent- and ʻokina-insensitive: `ahani` finds
/// `'Āhani e` and `mama` finds `Māmā Tahiti`.
///
/// **A missing needle is a `400`, not an empty answer.** "Nothing was typed" and
/// "nothing was found" are different facts, and a caller that forgot the
/// parameter wants to be told so rather than handed two empty arrays it will
/// report as success. The body is this API's own error shape, so a client that
/// asked for JSON does not have to parse a document to find out it was wrong.
#[route(GET "/api/search")]
async fn search_results(cx: &Cx) -> Result<(StatusCode, Json<serde_json::Value>)> {
    let query = uri(cx).query().unwrap_or("");

    let Some(needle) = search::needle(query) else {
        return Ok((
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({
                "error": "invalid_request",
                "message": format!("no search term: pass ?{}=…", search::PARAM),
            })),
        ));
    };

    let found = search::run(state::db(cx).pool(), &needle).await?;

    Ok((
        StatusCode::OK,
        Json(serde_json::json!({
            "query": needle,
            "songs": found.songs.iter().map(SongJson::summary).collect::<Vec<_>>(),
            "artists": found.artists.iter().map(ArtistJson::of).collect::<Vec<_>>(),
        })),
    ))
}

/// `GET /api/support` — the addresses the site accepts support at.
///
/// The page's own three entries, from the page's own table
/// ([`crate::pages::support::ADDRESSES`]), so the JSON and the HTML cannot
/// disagree about an address — which, here, is the difference between money
/// arriving and money going nowhere. The strings are passed through untouched:
/// nothing in this file re-cases, trims or reformats them.
///
/// `page` is the address of the page that prints them, the same form every other
/// `url` in this API uses: the page's own URL, which is what it canonicalises to.
#[route(GET "/api/support")]
async fn support_addresses() -> Result<Json<serde_json::Value>> {
    Ok(Json(serde_json::json!({
        "page": i18n::absolute(support::PATH),
        "count": support::ADDRESSES.len(),
        "addresses": support::ADDRESSES.iter().map(|entry| serde_json::json!({
            "label": entry.label,
            "address": entry.address,
        })).collect::<Vec<_>>(),
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
    paths.insert(SEARCH_PATH.to_owned(), json!({ "get": search_operation() }));
    paths.insert(HEALTH_PATH.to_owned(), json!({ "get": health_operation() }));
    paths.insert(
        SUPPORT_PATH.to_owned(),
        json!({ "get": support_operation() }),
    );

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

/// `GET /api/songs` — the catalogue, or one selection of it.
fn catalogue_operation() -> Value {
    json!({
        "operationId": "listSongs",
        "summary": "Every published song, newest first — or the songs a selection names",
        "description": "The same list the index page shows, from the same read. Drafts are not published and are not in it. With `s` repeated, the answer is the multi-lyric page's own URL: those songs, in the order the query named them, each with its lyrics, and the same 404 an unknown id gets when one of them is not published.",
        "parameters": [{
            "name": "s",
            "in": "query",
            "required": false,
            "description": "A song's slug, once per chosen song. Repeating it selects several songs in reading order — the same query string `/puta-himene` serves as a page. Two segments that name one song are one selection; a segment that names no published song is the 404.",
            "schema": { "type": "array", "items": { "type": "string" } },
            "style": "form",
            "explode": true
        }],
        "responses": {
            "200": {
                "description": "The published catalogue, or the songs the selection named",
                "content": {
                    "application/json": {
                        "schema": { "$ref": "#/components/schemas/Catalogue" }
                    }
                }
            },
            "404": {
                "description": "The selection names no published song",
                "content": {
                    "application/json": {
                        "schema": { "$ref": "#/components/schemas/Error" }
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

/// `GET /api/search` — the needle, and the two halves it found.
fn search_operation() -> Value {
    json!({
        "operationId": "search",
        "summary": "Search song titles and artist names",
        "description": "One needle, two reads: the published songs whose title matches and the artists whose name matches. Matching folds case and diacritics and treats the ʻokina as a separator, so `ahani` finds `'Āhani e` and `mama` finds `Māmā Tahiti`. The same needle, the same two reads and the same limit the search page uses, so the JSON and the HTML cannot disagree. A missing or blank needle is a 400: nothing typed and nothing found are different answers.",
        "parameters": [{
            "name": "q",
            "in": "query",
            "required": true,
            "description": "What to search for. Percent-encoded by the client, as any query value is; `+` is read as a space.",
            "schema": { "type": "string" }
        }],
        "responses": {
            "200": {
                "description": "The songs and the artists the needle matched, each half possibly empty",
                "content": {
                    "application/json": {
                        "schema": { "$ref": "#/components/schemas/Search" }
                    }
                }
            },
            "400": {
                "description": "No search term was given",
                "content": {
                    "application/json": {
                        "schema": { "$ref": "#/components/schemas/Error" }
                    }
                }
            }
        }
    })
}

/// `GET /api/support` — the site's support addresses.
fn support_operation() -> Value {
    json!({
        "operationId": "support",
        "summary": "The addresses the site accepts support at",
        "description": "The same three addresses the support page prints, from the page's own table: one for Bitcoin, one for Solana, and one that is the same address on Ethereum, Polygon, BNB Chain and Avalanche. The strings are passed through untouched — case matters on the third, where the mixed case is its EIP-55 checksum.",
        "responses": {
            "200": {
                "description": "The addresses, and the page that prints them",
                "content": {
                    "application/json": {
                        "schema": { "$ref": "#/components/schemas/Support" }
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
                "id": { "type": "string", "description": "The stable key: 20 characters of [0-9a-z]. It is not the page's address — `url` is." },
                "title": { "type": "string" },
                "artists": { "type": "array", "items": { "type": "string" } },
                "url": { "type": "string", "format": "uri", "description": "The song page's own address, on the canonical host. It is the URL the page canonicalises to; `/himene/{id}` answers a 301 to it." },
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
        "Artist": {
            "type": "object",
            "required": ["id", "name", "url"],
            "properties": {
                "id": { "type": "string", "description": "The stable key, as the artist's page is addressed by it: `/taata-himene/{id}`." },
                "name": { "type": "string", "description": "The credited name, as the songs print it." },
                "url": { "type": "string", "format": "uri", "description": "The artist page's own address, on the canonical host — the URL the page canonicalises to." }
            }
        },
        "Search": {
            "type": "object",
            "required": ["query", "songs", "artists"],
            "properties": {
                "query": { "type": "string", "description": "The needle, decoded — what was searched for, not the raw query string." },
                "songs": { "type": "array", "items": { "$ref": "#/components/schemas/Song" }, "description": "The published songs whose title matched, newest first, without lyrics." },
                "artists": { "type": "array", "items": { "$ref": "#/components/schemas/Artist" }, "description": "The artists whose name matched, best match first." }
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
        },
        "Support": {
            "type": "object",
            "required": ["count", "page", "addresses"],
            "properties": {
                "page": { "type": "string", "format": "uri", "description": "The support page's own address, on the canonical host — the URL the page canonicalises to." },
                "count": { "type": "integer" },
                "addresses": { "type": "array", "items": { "$ref": "#/components/schemas/SupportAddress" } }
            }
        },
        "SupportAddress": {
            "type": "object",
            "required": ["label", "address"],
            "properties": {
                "label": { "type": "string", "description": "The chains the address is for, as the page names them. A proper name, not a translated label." },
                "address": { "type": "string", "description": "The address, exactly as it must be used. Case is significant: the EVM address's mixed case is its EIP-55 checksum." }
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
            Some(fixture.slug.to_owned()),
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

    /// The URL a caller is handed is the page's own address, on the canonical
    /// host: the slug as the page, the sitemap and `llms.txt` spell it — one
    /// spelling, because the page canonicalises to it.
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
        assert_eq!(SEARCH_PATH, "/api/search");
        assert_eq!(SUPPORT_PATH, "/api/support");
        // The endpoint and the page read the same parameter, so the JSON a
        // client is told to send is the query string a browser's form submits.
        assert_eq!(search::PARAM, "q");
        assert!(VERSION.starts_with('4'));
    }

    /// A search answers the page's own two reads, and an artist is handed the
    /// address of the page that describes them — the page's own URL, the same
    /// one every other `url` field in this API names.
    #[tokio::test]
    async fn a_search_answers_the_pages_own_reads() {
        let db = Db::open_in_memory().await.expect("in-memory database");
        fixtures::seed(db.pool()).await.expect("seed fixtures");

        let found = search::run(db.pool(), "mama").await.expect("the read");
        assert!(
            found
                .songs
                .iter()
                .any(|song| song.get_title() == "Māmā Tahiti"),
            "the accent-insensitive title match is gone"
        );

        let body = serde_json::json!({
            "query": "mama",
            "songs": found.songs.iter().map(SongJson::summary).collect::<Vec<_>>(),
            "artists": found.artists.iter().map(ArtistJson::of).collect::<Vec<_>>(),
        });

        let artist = ArtistJson::of(&fixtures_artist());
        assert_eq!(artist.id, fixtures::ARTISTS[0].id);
        assert_eq!(artist.name, fixtures::ARTISTS[0].fullname);
        assert_eq!(
            artist.url,
            format!("{SITE_URL}{}", artist::path_of(fixtures::ARTISTS[0].id))
        );
        assert!(body["songs"].as_array().is_some());
        assert!(body["artists"].as_array().is_some());
    }

    /// A `crate::domain::Artist` built from the fixture table, for the JSON shape
    /// tests — the rows come from the database in production and from here in a
    /// unit test, which is the only difference.
    fn fixtures_artist() -> crate::domain::Artist {
        crate::domain::Artist::new(
            fixtures::ARTISTS[0].id.to_owned(),
            fixtures::ARTISTS[0].fullname.to_owned(),
            chrono::Utc::now(),
            chrono::Utc::now(),
        )
    }

    /// The catalogue's selection read answers the same rows the page serves, in
    /// the query's own order — and a hole in the selection is the 404 rather than
    /// a shortened list.
    ///
    /// The read is `pages::book`'s, which is the point of this test: the page
    /// and the API cannot answer a selection differently, because there is one
    /// implementation of "which songs does this URL name".
    #[tokio::test]
    async fn a_selection_is_the_pages_own_read() {
        let db = Db::open_in_memory().await.expect("in-memory database");
        fixtures::seed(db.pool()).await.expect("seed fixtures");

        let second = fixtures::SONGS[1].slug.to_owned();
        let first = fixtures::SONGS[0].slug.to_owned();
        let selected = vec![second, first];

        let sheets = book::resolve(db.pool(), &selected)
            .await
            .expect("the read")
            .expect("both are published");
        assert_eq!(sheets.len(), 2);

        let body = serde_json::json!({
            "count": sheets.len(),
            "songs": sheets.iter().map(SongJson::full).collect::<Vec<_>>(),
        });
        assert_eq!(body["songs"][0]["id"], fixtures::SONGS[1].id);
        assert_eq!(body["songs"][1]["id"], fixtures::SONGS[0].id);
        // The lyric is the reason this read exists: the multi-lyric page's JSON
        // form has nothing to say without it.
        assert!(
            body["songs"][0]["lyrics_markdown"].is_string(),
            "the selection read left the lyric out"
        );

        assert!(
            book::resolve(db.pool(), &["no-such-song".to_owned()])
                .await
                .expect("the read")
                .is_none(),
            "a hole in the selection would have been served as a shorter list"
        );
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
        let mut served = vec![PATH, SONG_PATH, SEARCH_PATH, HEALTH_PATH, SUPPORT_PATH];
        served.sort_unstable();
        assert_eq!(paths.keys().map(String::as_str).collect::<Vec<_>>(), served);

        for (path, item) in paths {
            let operations = item.as_object().expect("a path item");
            assert_eq!(operations.keys().collect::<Vec<_>>(), vec!["get"], "{path}");
            for operation in operations.values() {
                assert!(operation["responses"]["200"].is_object(), "{path}");
            }
        }

        // The single-song read takes the id its path template names; the
        // catalogue takes the selection, which is the multi-lyric page's query
        // string and not a path segment at all. Both have a second answer.
        assert_eq!(
            paths[SONG_PATH]["get"]["parameters"][0]["name"], "id",
            "the path template has no matching parameter"
        );
        assert!(paths[SONG_PATH]["get"]["responses"]["404"].is_object());

        let selection = &paths[PATH]["get"]["parameters"][0];
        assert_eq!(selection["name"], "s");
        assert_eq!(selection["in"], "query");
        assert_eq!(selection["schema"]["type"], "array");
        assert_eq!(selection["explode"], true);
        assert!(paths[PATH]["get"]["responses"]["404"].is_object());
        assert_eq!(
            selection["name"], "s",
            "the parameter name is the one `pages::book` reads"
        );
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
