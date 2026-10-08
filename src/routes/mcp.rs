//! The read-only MCP server — `POST /mcp`.
//!
//! Model Context Protocol over the Streamable HTTP transport: stateless,
//! JSON-RPC 2.0, JSON responses only (no SSE, therefore no `GET /mcp`). Two
//! tools, both reads:
//!
//! * `list_songs(page, per_page)` — one page of the published catalogue.
//! * `get_song(id)` — one song: the title, the credits, the lyric with its
//!   chords inline, and the canonical URL, as the Markdown sheet.
//!
//! **Read-only is the whole design.** "Add lyrics by AI" is a different promise
//! from "read lyrics by AI": a public, unauthenticated write endpoint is a spam
//! funnel until it has an auth model, a moderation queue and a rate limit, and
//! none of those is here. There is therefore no session, no `Mcp-Session-Id`, no
//! login, and nothing that can change the catalogue. The create-song page stays
//! the one way a song is contributed, which is also what `robots.txt` says.
//!
//! **Why the protocol is hand-rolled.** Four methods and two tools do not need
//! an SDK, and an SDK would want to own the HTTP transport that the router
//! already owns — this repository is one small static binary on purpose. What is
//! written here is the part the spec makes normative: the version handshake, the
//! two tool definitions, the error codes, and the transport rule that a
//! notification is answered with `202` and no body.
//!
//! **What the tools return.** `list_songs` answers with the catalogue as JSON —
//! a list is data, and the ids in it are what `get_song` takes. `get_song`
//! answers with the site's Markdown sheet, the same document the
//! `Accept: text/markdown` negotiation serves at the song's URL, because the
//! lyric with its chords is the payload and Markdown keeps it readable. The
//! lyrics are never translated; the MCP layer has no locale at all.
//!
//! The endpoint is exempt from the router's cross-origin policy (see
//! [`crate::router`]): a browser-based MCP client sends an `Origin`, and this
//! route has no state to forge against.

use serde_json::{Map, Value, json};
use topcoat::{
    Result,
    context::Cx,
    router::{
        Body, StatusCode, header,
        request::{Bytes, headers},
        response::{IntoResponse, Response},
        route,
    },
};

use crate::db::{self, SongOrder};
use crate::domain::Song;
use crate::log;
use crate::routes::api::{SongJson, VERSION};
use crate::routes::negotiation::song_document;
use crate::state;

/// The endpoint. `POST` only: without SSE there is no `GET` stream to open.
pub const PATH: &str = "/mcp";

/// The server's name, as the protocol calls it — a stable identifier, not a
/// title. [`SERVER_TITLE`] is what a client shows a person.
///
/// `pub(crate)` because the server card ([`crate::routes::card`]) publishes the
/// same two strings: a discovery document that renamed the server would describe
/// an endpoint a client cannot then identify.
pub(crate) const SERVER_NAME: &str = "chansondufenua";
pub(crate) const SERVER_TITLE: &str = "Chanson du fenua";

/// The newest revision of the protocol this server implements.
const LATEST_PROTOCOL: &str = "2025-06-18";

/// Every revision whose request shapes this server actually speaks.
///
/// A client that asks for one of these gets it back; one that asks for anything
/// else gets [`LATEST_PROTOCOL`], which the spec says is the client's cue to
/// decide whether it can go on. The three differ in ways this server does not
/// use — SSE resumability, batch support, sampling — so the same handlers are
/// correct for all of them.
///
/// `pub(crate)`: the server card lists these as the versions the advertised
/// endpoint supports, and a card that promised a revision the handshake would
/// refuse is the one failure a discovery document can cause on its own.
pub(crate) const SUPPORTED_PROTOCOLS: &[&str] = &["2025-06-18", "2025-03-26", "2024-11-05"];

/// The header a client sends after `initialize` to pin the negotiated revision.
const PROTOCOL_HEADER: &str = "mcp-protocol-version";

const JSON: &str = "application/json";

/// The tool names, in one place: the definitions and the dispatcher both use
/// them, and a drift between the two would be a tool a client can see and not
/// call.
const LIST_TOOL: &str = "list_songs";
const GET_TOOL: &str = "get_song";

/// `list_songs` defaults and ceiling. The page is bounded because the answer is
/// one text block and a client should not be handed 43 lyrics in a listing.
///
/// The default is the index's own page size rather than a second twenty, because
/// this tool's promise is that a client paging through it and a reader paging
/// through `/himene` see the same thing — which is a promise about the *window*,
/// and two constants are two windows waiting to drift apart.
const DEFAULT_PER_PAGE: i64 = crate::pages::songs::PAGE_SIZE;
const MAX_PER_PAGE: i64 = 100;

// JSON-RPC 2.0 error codes. The five the spec defines; no others are invented.
const PARSE_ERROR: i64 = -32700;
const INVALID_REQUEST: i64 = -32600;
const METHOD_NOT_FOUND: i64 = -32601;
const INVALID_PARAMS: i64 = -32602;
const INTERNAL_ERROR: i64 = -32603;

/// What `initialize` tells a client about this server, in one paragraph.
const INSTRUCTIONS: &str = "\
Read-only access to Chanson du fenua, a songbook of Tahitian songs with lyrics \
and chords. Use list_songs to page through the catalogue and get_song to read \
one song as a Markdown chord sheet. The lyrics are published exactly as their \
authors wrote them and are never translated; the site's own words — its chrome \
and its pages — exist in French, Tahitian and English. Nothing here can add, \
change or delete a song.";

/// `POST /mcp` — one JSON-RPC message in, one out.
///
/// Stateless by construction: nothing is remembered between requests, so any
/// instance can answer any request and there is no session to expire.
#[route(POST "/mcp")]
async fn handle(cx: &Cx, body: Bytes) -> Result<Response> {
    // The transport asks a server to refuse a revision it does not implement.
    // Checked only when the header is present, because a client is not required
    // to send it — and one that does not is still speaking a version this
    // server has just agreed to in `initialize`.
    if let Some(version) = request_version(cx)
        && !SUPPORTED_PROTOCOLS.contains(&version.as_str())
    {
        return respond(
            cx,
            StatusCode::BAD_REQUEST,
            &error_body(
                Value::Null,
                INVALID_REQUEST,
                format!(
                    "Unsupported MCP-Protocol-Version {version:?}; supported: {}",
                    SUPPORTED_PROTOCOLS.join(", ")
                ),
            ),
        );
    }

    let (method, params, id) = match read_message(&body) {
        Message::Request { method, params, id } => (method, params, id),
        // A notification or a JSON-RPC response: 202, no body, by definition.
        Message::NoReply => return StatusCode::ACCEPTED.into_response(cx),
        Message::Invalid(error) => return respond(cx, StatusCode::BAD_REQUEST, &error),
    };

    let reply = match method.as_str() {
        "initialize" => success(id, initialize(params.as_ref())),
        "ping" => success(id, json!({})),
        "tools/list" => success(id, tools()),
        "tools/call" => match call_tool(cx, params.as_ref()).await {
            Ok(value) => success(id, value),
            Err(error) => failure(id, error),
        },
        // A tool is not a method: `tools/call` with a name nobody defines is
        // `-32602` and the message is the spec's own wording.
        _ => failure(
            id,
            RpcError {
                code: METHOD_NOT_FOUND,
                message: format!("Method not found: {method}"),
            },
        ),
    };

    // Errors of this shape are part of a 200 answer, not an HTTP failure: the
    // request was understood and answered; the *method* is what was wrong.
    respond(cx, StatusCode::OK, &reply)
}

// ---------------------------------------------------------------------------
// The wire, read
// ---------------------------------------------------------------------------

/// One posted body, classified before anything has to be answered.
enum Message {
    /// A request: it names a method and carries an id to answer.
    Request {
        method: String,
        params: Option<Value>,
        id: Value,
    },
    /// A notification, or a JSON-RPC response. The transport answers `202` with
    /// no body, whatever it says.
    NoReply,
    /// Not a JSON-RPC 2.0 request object. The value is the error to send with a
    /// `400`.
    Invalid(Value),
}

/// Read one HTTP body as one JSON-RPC message.
fn read_message(body: &[u8]) -> Message {
    let Ok(message) = serde_json::from_slice::<Value>(body) else {
        return Message::Invalid(error_body(Value::Null, PARSE_ERROR, "Parse error"));
    };

    let Some(object) = message.as_object() else {
        return Message::Invalid(error_body(
            Value::Null,
            INVALID_REQUEST,
            "Invalid Request: expected one JSON-RPC object; batching was removed \
             in protocol 2025-06-18",
        ));
    };

    if object.get("jsonrpc").and_then(Value::as_str) != Some("2.0") {
        return Message::Invalid(error_body(
            id_of(object),
            INVALID_REQUEST,
            "Invalid Request: \"jsonrpc\" must be exactly \"2.0\"",
        ));
    }

    let Some(method) = object.get("method") else {
        // No method at all: this is a JSON-RPC *response*. A stateless server
        // has sent nothing to correlate it with, so the transport's answer is
        // `202` and the message is dropped.
        return Message::NoReply;
    };
    let Some(method) = method.as_str() else {
        return Message::Invalid(error_body(
            id_of(object),
            INVALID_REQUEST,
            "Invalid Request: \"method\" must be a string",
        ));
    };

    let Some(id) = object.get("id").cloned() else {
        // No id: a notification. No response is sent for one, whatever it asks.
        // `notifications/initialized` is the only notification a client of this
        // server sends.
        return Message::NoReply;
    };

    Message::Request {
        method: method.to_owned(),
        params: object.get("params").cloned(),
        id,
    }
}

/// The id a rejected message carried, or `null` — which is what the spec asks a
/// server to echo when it cannot tell.
fn id_of(object: &Map<String, Value>) -> Value {
    object.get("id").cloned().unwrap_or(Value::Null)
}

/// The MCP revision the client pinned on this request, if it sent one.
fn request_version(cx: &Cx) -> Option<String> {
    headers(cx)
        .get(PROTOCOL_HEADER)
        .and_then(|value| value.to_str().ok())
        .map(str::to_owned)
}

// ---------------------------------------------------------------------------
// The wire, written
// ---------------------------------------------------------------------------

/// A JSON-RPC reply: always `200`, always `application/json`.
///
/// The status is the transport's answer ("I understood you"); the code inside is
/// the protocol's ("what you asked for was wrong"). Keeping the two apart is
/// what lets a client tell a dead server from a bad request.
fn respond(cx: &Cx, status: StatusCode, body: &Value) -> Result<Response> {
    (
        status,
        [(header::CONTENT_TYPE, JSON)],
        Body::from(body.to_string()),
    )
        .into_response(cx)
}

/// A successful reply to `id`.
fn success(id: Value, result: Value) -> Value {
    json!({ "jsonrpc": "2.0", "id": id, "result": result })
}

/// A protocol error in reply to `id`.
fn failure(id: Value, error: RpcError) -> Value {
    error_body(id, error.code, error.message)
}

fn error_body(id: Value, code: i64, message: impl Into<String>) -> Value {
    json!({
        "jsonrpc": "2.0",
        "id": id,
        "error": { "code": code, "message": message.into() },
    })
}

/// A protocol-level fault: the request was not answerable as written.
#[derive(Debug)]
struct RpcError {
    code: i64,
    message: String,
}

impl RpcError {
    fn invalid_params(message: impl Into<String>) -> Self {
        Self {
            code: INVALID_PARAMS,
            message: message.into(),
        }
    }
}

// ---------------------------------------------------------------------------
// The methods
// ---------------------------------------------------------------------------

/// `initialize` — the handshake.
///
/// The revision is echoed when it is one this server speaks, and otherwise the
/// newest one it does; the spec makes the *client* the one that decides whether
/// it can go on. `capabilities.tools` is declared because the tools exist;
/// `listChanged` is `false` because the list is compiled in and never changes.
fn initialize(params: Option<&Value>) -> Value {
    let requested = params
        .and_then(|params| params.get("protocolVersion"))
        .and_then(Value::as_str);
    let version = requested
        .filter(|version| SUPPORTED_PROTOCOLS.contains(version))
        .unwrap_or(LATEST_PROTOCOL);

    json!({
        "protocolVersion": version,
        "capabilities": capabilities(),
        "serverInfo": server_info(),
        "instructions": INSTRUCTIONS,
    })
}

/// What this server says about itself: the name a client keys on, the title it
/// shows a person, and the version.
///
/// `pub(crate)` because the server card ([`crate::routes::card`]) publishes
/// exactly this object. The extension requires a card not to contradict the live
/// handshake — but the card is fetched *before* the client connects, so the only
/// way to keep the promise is for both surfaces to be the same code, which is
/// what this function is.
pub(crate) fn server_info() -> Value {
    json!({
        "name": SERVER_NAME,
        "title": SERVER_TITLE,
        "version": VERSION,
    })
}

/// The primitives this server declares, shared with the server card for the same
/// reason [`server_info`] is: one object, two surfaces that cannot disagree.
///
/// `tools` alone. There are no resources, no prompts and no logging here, and a
/// capability declared but not implemented is a client's next timeout.
pub(crate) fn capabilities() -> Value {
    json!({ "tools": { "listChanged": false } })
}

/// `tools/list` — the two reads.
///
/// No cursor: two tools fit in one answer, and the spec only asks for pagination
/// when a list can be long. The schemas are the contract a client validates
/// against, so they carry the bounds the handlers enforce.
fn tools() -> Value {
    json!({
        "tools": [
            {
                "name": LIST_TOOL,
                "title": "List songs",
                "description": "One page of the published catalogue, newest first. \
                    Each entry carries the id, title, credited artists, URL \
                    and view count. Pass an id to get_song to read the lyrics.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "page": {
                            "type": "integer",
                            "minimum": 1,
                            "default": 1,
                            "description": "1-based page number."
                        },
                        "per_page": {
                            "type": "integer",
                            "minimum": 1,
                            "maximum": MAX_PER_PAGE,
                            "default": DEFAULT_PER_PAGE,
                            "description": "Songs per page, 1 to 100."
                        }
                    },
                    "additionalProperties": false
                },
                "annotations": {
                    "readOnlyHint": true,
                    "idempotentHint": true,
                    "openWorldHint": false
                }
            },
            {
                "name": GET_TOOL,
                "title": "Get a song",
                "description": "One published song as a Markdown chord sheet: the title, \
                    the credited artists, the lyrics with chords inline as [Eb], and the \
                    canonical URL. The lyrics are never translated.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "id": {
                            "type": "string",
                            "minLength": 1,
                            "description": "The song's address segment or its id: the \
                                slug from a list_songs `url`, or the id it returns. \
                                Both resolve, and so does a slug the song used to \
                                have."
                        }
                    },
                    "required": ["id"],
                    "additionalProperties": false
                },
                "annotations": {
                    "readOnlyHint": true,
                    "idempotentHint": true,
                    "openWorldHint": false
                }
            }
        ]
    })
}

/// The tool a name selects, if it names one.
fn tool(name: &str) -> Option<Tool> {
    match name {
        LIST_TOOL => Some(Tool::List),
        GET_TOOL => Some(Tool::Get),
        _ => None,
    }
}

enum Tool {
    List,
    Get,
}

/// `tools/call` — dispatch, then the tool's own argument rules.
async fn call_tool(cx: &Cx, params: Option<&Value>) -> std::result::Result<Value, RpcError> {
    let object = params
        .and_then(Value::as_object)
        .ok_or_else(|| RpcError::invalid_params("Invalid params: tools/call requires an object"))?;

    let name = object
        .get("name")
        .and_then(Value::as_str)
        .ok_or_else(|| RpcError::invalid_params("Invalid params: tools/call requires \"name\""))?;

    // `arguments` is optional in the schema and absent when a tool takes none.
    let arguments = object
        .get("arguments")
        .cloned()
        .unwrap_or_else(|| json!({}));

    match tool(name) {
        Some(Tool::List) => list_songs(cx, &arguments).await,
        Some(Tool::Get) => get_song(cx, &arguments).await,
        // The spec's own example for an unknown tool is `-32602`, and a tool
        // name is an argument like any other.
        None => Err(RpcError::invalid_params(format!("Unknown tool: {name}"))),
    }
}

/// `list_songs` — one page of the catalogue, newest first.
///
/// The order is the index's, so a client paging through this and a reader
/// paging through `/himene` see the same thing. A page past the end is not an
/// error: it is an empty `songs` list, and `total`/`total_pages` say why.
async fn list_songs(cx: &Cx, arguments: &Value) -> std::result::Result<Value, RpcError> {
    let object = arguments.as_object().ok_or_else(|| {
        RpcError::invalid_params("Invalid params: list_songs arguments must be an object")
    })?;

    let page = integer_argument(object, "page", 1, 1, i64::MAX)?;
    let per_page = integer_argument(object, "per_page", DEFAULT_PER_PAGE, 1, MAX_PER_PAGE)?;

    let pool = state::db(cx).pool();
    // `page` and `per_page` are both at least 1, so neither the subtraction nor
    // the multiplication can go negative; `saturating_mul` only guards a page
    // number so large it would wrap.
    let offset = (page - 1).saturating_mul(per_page);

    let listed = db::songs_page(pool, SongOrder::Newest, per_page, offset)
        .await
        .map_err(internal)?;
    let total = db::counts(pool).await.map_err(internal)?.songs;

    let document = json!({
        "page": page,
        "per_page": per_page,
        "total": total,
        "total_pages": total.div_ceil(per_page as u32),
        "count": listed.len(),
        "songs": listed.iter().map(SongJson::summary).collect::<Vec<_>>(),
    });

    Ok(text_result(document.to_string()))
}

/// `get_song` — one published song, as the site's Markdown sheet.
///
/// A song that is not there is a *tool* error, not a protocol error: the
/// argument was well formed and the answer is "no such song", which is the same
/// either way (`-32602` would tell a client its request was malformed, and it
/// was not).
///
/// **The argument is a slug or an id.** `list_songs` returns both (`url` and
/// `id`), and a client that followed a link has the slug, so both have to
/// resolve: [`db::song_at`] is the same resolution the site's own URLs use, and a
/// slug the song used to have resolves too rather than failing.
///
/// **A draft is not found**, the rule the sheet and the JSON API both keep:
/// [`db::song_at`] returns drafts so the editor can see them, and this surface is
/// public.
async fn get_song(cx: &Cx, arguments: &Value) -> std::result::Result<Value, RpcError> {
    let object = arguments.as_object().ok_or_else(|| {
        RpcError::invalid_params("Invalid params: get_song arguments must be an object")
    })?;

    let id = object
        .get("id")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|id| !id.is_empty())
        .ok_or_else(|| RpcError::invalid_params("Invalid params: get_song requires \"id\""))?;

    let found = db::song_at(state::db(cx).pool(), id)
        .await
        .map_err(internal)?
        .map(db::Addressed::into_song)
        .filter(Song::is_published);

    Ok(match found {
        // No step: MCP is not a page and a tool call carries no view of one.
        // An agent asking for a song wants the chords the author wrote.
        Some(sheet) => text_result(song_document(&sheet, 0)),
        None => error_result(format!(
            "No published song with id or slug {id:?}. Use {LIST_TOOL} to find one."
        )),
    })
}

/// One required-by-type integer argument, defaulted and bounded.
///
/// A JSON number that is not an integer (`2.5`) is refused rather than
/// truncated, and a string is refused rather than coerced: a caller that sent
/// either is not sending `page`, whatever it meant.
fn integer_argument(
    object: &Map<String, Value>,
    name: &str,
    default: i64,
    min: i64,
    max: i64,
) -> std::result::Result<i64, RpcError> {
    let Some(raw) = object.get(name) else {
        return Ok(default);
    };

    let value = raw.as_i64().ok_or_else(|| {
        RpcError::invalid_params(format!("Invalid params: \"{name}\" must be an integer"))
    })?;

    if value < min || value > max {
        return Err(RpcError::invalid_params(format!(
            "Invalid params: \"{name}\" must be between {min} and {max}, got {value}"
        )));
    }

    Ok(value)
}

/// A tool result that carries text.
fn text_result(text: String) -> Value {
    json!({ "content": [{ "type": "text", "text": text }] })
}

/// A tool result that carries the reason it failed.
///
/// `isError: true` is the spec's "the tool ran and could not answer" — distinct
/// from a protocol error, and distinct on purpose: a client can show this to a
/// model without treating the session as broken.
fn error_result(text: String) -> Value {
    json!({
        "content": [{ "type": "text", "text": text }],
        "isError": true,
    })
}

/// A database fault, on its way to the journal and to a generic reply.
///
/// The detail is logged, not returned: a SQLite message can name a path, and a
/// client only needs to know this side broke. It is still not an HTTP 500 — the
/// JSON-RPC conversation is intact and the code says whose fault it is.
fn internal(error: db::DbError) -> RpcError {
    log::error(format_args!("mcp: {error}"));
    RpcError {
        code: INTERNAL_ERROR,
        message: "Internal error: the catalogue could not be read".to_owned(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn request(body: &str) -> (String, Option<Value>, Value) {
        match read_message(body.as_bytes()) {
            Message::Request { method, params, id } => (method, params, id),
            _ => panic!("{body} was not read as a request"),
        }
    }

    /// The handshake echoes a revision the server speaks and offers its newest
    /// one when it does not — never a silent acceptance of anything.
    #[test]
    fn initialize_echoes_a_supported_version_and_falls_back_otherwise() {
        for version in SUPPORTED_PROTOCOLS {
            let params = json!({ "protocolVersion": version });
            let result = initialize(Some(&params));
            assert_eq!(result["protocolVersion"], *version, "{version}");
        }

        let future = json!({ "protocolVersion": "2099-01-01" });
        assert_eq!(
            initialize(Some(&future))["protocolVersion"],
            LATEST_PROTOCOL
        );

        // A client that sends nothing at all — malformed, but not a reason to
        // drop the connection: it gets the newest revision.
        assert_eq!(initialize(None)["protocolVersion"], LATEST_PROTOCOL);
    }

    /// The handshake declares exactly the capability the server has.
    #[test]
    fn initialize_declares_the_tools_capability_and_the_server_identity() {
        let result = initialize(None);

        assert_eq!(result["capabilities"]["tools"]["listChanged"], false);
        assert_eq!(result["serverInfo"]["name"], SERVER_NAME);
        assert_eq!(result["serverInfo"]["version"], VERSION);
        assert!(
            result["instructions"].as_str().is_some_and(|text| {
                text.contains("never translated") && text.contains("Read-only")
            }),
            "the instructions must say what the tools do and do not do"
        );
        // The three advertised capabilities the site does *not* have are absent,
        // not declared false: a client should not see a promise it can poke at.
        assert!(result["capabilities"].get("resources").is_none());
        assert!(result["capabilities"].get("prompts").is_none());
        assert!(result["capabilities"].get("logging").is_none());

        // And they are the *shared* objects, not copies. The server card
        // (`src/routes/card.rs`) publishes `server_info()` and `capabilities()`
        // before a client connects; if the handshake ever grew its own literals
        // again, the card would be describing a different server, and this is
        // the assertion that would notice.
        assert_eq!(result["serverInfo"], server_info());
        assert_eq!(result["capabilities"], capabilities());
        assert_eq!(server_info()["title"], SERVER_TITLE);
    }

    /// Two tools, both reads, named exactly as the dispatcher knows them.
    #[test]
    fn the_tool_list_is_two_reads_the_dispatcher_answers() {
        let listed = tools();
        let tools = listed["tools"].as_array().expect("a tools array");
        assert_eq!(tools.len(), 2);

        let names: Vec<&str> = tools
            .iter()
            .map(|tool| tool["name"].as_str().expect("a name"))
            .collect();
        assert_eq!(names, [LIST_TOOL, GET_TOOL]);

        for tool in tools {
            let name = tool["name"].as_str().expect("a name");
            assert!(
                super::tool(name).is_some(),
                "the definition {name} has no dispatcher arm"
            );
            assert_eq!(tool["annotations"]["readOnlyHint"], true, "{name}");
            assert_eq!(
                tool["inputSchema"]["type"], "object",
                "{name} must declare an object schema"
            );
        }

        // Every tool the dispatcher knows is advertised, either way round.
        assert!(tools.iter().any(|tool| tool["name"] == GET_TOOL));
    }

    /// The schemas carry the bounds the handlers enforce, so a client can
    /// validate before it ever sends a bad page.
    #[test]
    fn the_list_schema_bounds_what_the_handler_bounds() {
        let listed = tools();
        let list = &listed["tools"][0];
        let properties = &list["inputSchema"]["properties"];

        assert_eq!(properties["page"]["minimum"], 1);
        assert_eq!(properties["page"]["default"], 1);
        assert_eq!(properties["per_page"]["minimum"], 1);
        assert_eq!(properties["per_page"]["maximum"], MAX_PER_PAGE);
        assert_eq!(properties["per_page"]["default"], DEFAULT_PER_PAGE);
        assert_eq!(list["inputSchema"]["additionalProperties"], false);

        let get = &listed["tools"][1];
        assert_eq!(get["inputSchema"]["required"][0], "id");
        assert_eq!(get["inputSchema"]["properties"]["id"]["minLength"], 1);
    }

    /// The dispatcher is a whitelist: a name it does not define is refused, and
    /// nothing else is reachable by naming it.
    #[test]
    fn only_the_two_reads_are_dispatchable() {
        assert!(matches!(tool(LIST_TOOL), Some(Tool::List)));
        assert!(matches!(tool(GET_TOOL), Some(Tool::Get)));
        for name in ["", "delete_song", "create_song", "List_Songs", "tools/list"] {
            assert!(tool(name).is_none(), "{name} must not be a tool");
        }
    }

    /// The message reader: a well-formed request is read, a notification and a
    /// response are not answered, and everything else is refused with the code
    /// the spec assigns it.
    #[test]
    fn the_reader_sorts_requests_notifications_responses_and_nonsense() {
        let (method, params, id) = request(r#"{"jsonrpc":"2.0","id":1,"method":"tools/list"}"#);
        assert_eq!(method, "tools/list");
        assert_eq!(params, None);
        assert_eq!(id, json!(1));

        // A string id is echoed as a string; params arrive as they were sent.
        let (method, params, id) = request(
            r#"{"jsonrpc":"2.0","id":"abc","method":"tools/call","params":{"name":"list_songs"}}"#,
        );
        assert_eq!(method, "tools/call");
        assert_eq!(id, json!("abc"));
        assert_eq!(params.unwrap()["name"], LIST_TOOL);

        // Notifications: no id, so no answer.
        assert!(matches!(
            read_message(br#"{"jsonrpc":"2.0","method":"notifications/initialized"}"#),
            Message::NoReply
        ));
        // A JSON-RPC response: also nothing to do.
        assert!(matches!(
            read_message(br#"{"jsonrpc":"2.0","id":1,"result":{}}"#),
            Message::NoReply
        ));

        // A parse error and a batch: `-32700` and `-32600`, ids `null`.
        let Message::Invalid(parse) = read_message(b"{not json") else {
            panic!("garbage was not refused");
        };
        assert_eq!(parse["error"]["code"], PARSE_ERROR);
        assert_eq!(parse["id"], Value::Null);

        let Message::Invalid(batch) = read_message(br#"[{"jsonrpc":"2.0","id":1,"method":"x"}]"#)
        else {
            panic!("a batch was not refused");
        };
        assert_eq!(batch["error"]["code"], INVALID_REQUEST);

        // Right shape, wrong protocol: refused, and the id is echoed back so the
        // client can match the failure to its request.
        let Message::Invalid(bad_version) =
            read_message(br#"{"jsonrpc":"1.0","id":7,"method":"ping"}"#)
        else {
            panic!("a wrong jsonrpc version was not refused");
        };
        assert_eq!(bad_version["error"]["code"], INVALID_REQUEST);
        assert_eq!(bad_version["id"], json!(7));

        let Message::Invalid(bad_method) = read_message(br#"{"jsonrpc":"2.0","id":7,"method":42}"#)
        else {
            panic!("a non-string method was not refused");
        };
        assert_eq!(bad_method["error"]["code"], INVALID_REQUEST);
    }

    /// The argument reader defaults, bounds, and refuses what it cannot trust.
    #[test]
    fn integer_arguments_default_bound_and_refuse() {
        let empty = Map::new();
        assert_eq!(integer_argument(&empty, "page", 1, 1, 10).unwrap(), 1);

        let given = json!({ "page": 3, "per_page": "20" });
        let given = given.as_object().unwrap();
        assert_eq!(integer_argument(given, "page", 1, 1, 10).unwrap(), 3);

        // A string is not a number, and a float is not an integer.
        assert!(integer_argument(given, "per_page", 1, 1, 10).is_err());
        let float = json!({ "page": 2.5 });
        assert!(integer_argument(float.as_object().unwrap(), "page", 1, 1, 10).is_err());

        // Outside the bound is refused, not clamped.
        let zero = json!({ "page": 0 });
        assert!(integer_argument(zero.as_object().unwrap(), "page", 1, 1, 10).is_err());
        let over = json!({ "per_page": 101 });
        assert!(integer_argument(over.as_object().unwrap(), "per_page", 1, 1, 100).is_err());
    }

    /// The two result shapes: a text block, and a text block that says it
    /// failed. `isError` is the whole difference, and it must not appear on a
    /// success or be missing from a failure.
    #[test]
    fn a_tool_result_carries_text_and_flags_only_its_failures() {
        let ok = text_result("42 songs".to_owned());
        assert_eq!(ok["content"][0]["type"], "text");
        assert_eq!(ok["content"][0]["text"], "42 songs");
        assert!(ok.get("isError").is_none());

        let failed = error_result("no such song".to_owned());
        assert_eq!(failed["content"][0]["text"], "no such song");
        assert_eq!(failed["isError"], true);
    }

    /// The envelope is JSON-RPC 2.0 with the id it was given, whatever the id
    /// looked like.
    #[test]
    fn replies_echo_the_id_and_name_the_code() {
        let ok = success(json!("x"), json!({ "tools": [] }));
        assert_eq!(ok["jsonrpc"], "2.0");
        assert_eq!(ok["id"], json!("x"));
        assert!(ok.get("error").is_none());

        let err = failure(json!(3), RpcError::invalid_params("Unknown tool: nope"));
        assert_eq!(err["jsonrpc"], "2.0");
        assert_eq!(err["id"], json!(3));
        assert_eq!(err["error"]["code"], INVALID_PARAMS);
        assert_eq!(err["error"]["message"], "Unknown tool: nope");
    }

    /// The endpoint constant is the one the router registers and the one the
    /// server card will advertise.
    #[test]
    fn the_path_is_the_documented_one() {
        assert_eq!(PATH, "/mcp");
    }
}
