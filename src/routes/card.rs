//! The MCP server card — `/.well-known/mcp/server-card.json`.
//!
//! One small JSON document that says *where* the site's MCP server is and which
//! protocol revisions it speaks, so a client can find the endpoint before it
//! opens a connection. It is the out-of-band half of [`crate::routes::mcp`]:
//! that module is the server, this is where a client is told the server exists.
//!
//! **It is honest, and that is the point.** A discovery document for a service
//! that is not there is a lie told to machines, and the first client that
//! follows one stops believing the next. Until step 14 this site had no MCP
//! server, so publishing a card would have been exactly that lie; it has one
//! now (read-only, two tools, `POST /mcp`), and this is the file advertising it.
//!
//! **Two shapes in one document, on purpose.** The Server Card extension
//! ([SEP-2127]) was finalised while this step was being written, and its wire
//! format is maintained in the extension repository rather than in the SEP, so
//! two readers disagree about what a card looks like:
//!
//! * the extension's own JSON Schema — `$schema`, `name` in `namespace/name`
//!   form, `version`, `description`, `remotes[]` with a transport type and
//!   endpoint. Those fields are written here because the schema requires them;
//! * the requirements a client testing for the service publishes —
//!   `serverInfo` with `name` and `version`, the transport `endpoint`, and
//!   `capabilities`. Those are written here too.
//!
//! Neither shape forbids the other's fields (the schema sets no
//! `additionalProperties: false`), and every value in both is the same true
//! statement about the same server, so the document is a superset rather than a
//! contradiction. The duplicated values are not typed twice: the `server_info`
//! and `capabilities` builders in [`crate::routes::mcp`] are the very functions
//! `initialize` answers with, so the card cannot drift from the handshake it
//! describes.
//!
//! **Why this path.** The extension reserves `<mcp endpoint>/server-card` as a
//! recommended default and argues against `.well-known` for an individual
//! server's card. `/mcp` is only two characters, so `/.well-known/mcp/…` at the
//! domain root is `/.well-known/mcp/server-card.json` here — and that is the
//! path the ecosystem's own checkers fetch. The extension permits a card at any
//! unreserved URI, so this is a placement, not a violation; a second copy at
//! `/mcp/server-card` was considered and left out, because two URLs for one
//! document is a canonical-URL question this step does not need to answer.
//!
//! **The response.** `application/mcp-server-card+json` when the caller asks for
//! it — that is the media type the extension defines, and a client that follows
//! the extension sends it — and `application/json` otherwise, which is what a
//! checker with a plain `GET` reads. `Cache-Control`, `ETag` with `If-None-Match`
//! and the CORS headers the extension requires are all here for the same reason:
//! a public, read-only document that machines fetch should be cacheable and
//! readable from a browser page.

use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

use serde_json::{Value, json};
use topcoat::{
    Result,
    context::Cx,
    router::{
        Body, HeaderMap, HeaderName, HeaderValue, StatusCode, header,
        request::headers,
        response::{IntoResponse, Response},
        route,
    },
};

use crate::domain::song::SITE_URL;
use crate::routes::api::VERSION;
use crate::routes::mcp;

/// `/.well-known/mcp/server-card.json` — the path, in one place.
pub const PATH: &str = "/.well-known/mcp/server-card.json";

/// The media type the Server Card extension defines for this document.
pub(crate) const MEDIA_TYPE: &str = "application/mcp-server-card+json";

/// What a caller that did not ask for the card's own type is handed.
const JSON: &str = "application/json";

/// The schema this document declares it conforms to.
///
/// Fixed by the extension: the `/v1/` family under `static.modelcontextprotocol.io`.
/// A card naming anything else is claiming a shape that was never published.
const SCHEMA: &str = "https://static.modelcontextprotocol.io/schemas/v1/server-card.schema.json";

/// The server's identifier, in the `namespace/name` form the schema requires:
/// the registrable domain reversed, then the resource it serves.
const NAME: &str = "pf.chansondufenua/songs";

/// The transport this server is reached over, in the schema's vocabulary.
const TRANSPORT: &str = "streamable-http";

/// One sentence about the server, for a client that shows a card to a person.
/// The schema caps this at 100 characters.
const DESCRIPTION: &str = "Read-only access to Chanson du fenua: Tahitian songs with their chords.";

/// The card changes only when the site is redeployed, so an hour is a ceiling a
/// client can rely on without being handed a stale endpoint.
const CACHE: &str = "public, max-age=3600";

/// The card's response headers, in one shape: what the document is, how long it
/// may be cached, its validator, and the cross-origin permissions the extension
/// requires of a card endpoint.
///
/// A map rather than an array because a `304` carries no representation: the
/// `Content-Type` is left off the validator-only answer, and the access log then
/// reports it as a body it did not send.
fn respond_headers(content_type: Option<&'static str>, tag: &str) -> HeaderMap {
    let mut headers = HeaderMap::new();

    if let Some(content_type) = content_type {
        headers.insert(header::CONTENT_TYPE, HeaderValue::from_static(content_type));
    }
    headers.insert(header::CACHE_CONTROL, HeaderValue::from_static(CACHE));
    headers.insert(
        header::ETAG,
        HeaderValue::from_str(tag).expect("a quoted hex tag is a valid header value"),
    );
    headers.insert(
        header::ACCESS_CONTROL_ALLOW_ORIGIN,
        HeaderValue::from_static("*"),
    );
    headers.insert(
        header::ACCESS_CONTROL_ALLOW_METHODS,
        HeaderValue::from_static("GET"),
    );
    headers.insert(
        header::ACCESS_CONTROL_ALLOW_HEADERS,
        HeaderValue::from_static("Content-Type, If-None-Match"),
    );
    headers.insert(
        header::ACCESS_CONTROL_EXPOSE_HEADERS,
        HeaderValue::from_static("ETag"),
    );

    headers
}

/// `GET /.well-known/mcp/server-card.json` — the document, and a `304` when the
/// caller already has this exact one.
#[route(GET "/.well-known/mcp/server-card.json")]
async fn server_card(cx: &Cx) -> Result<Response> {
    let body = document().to_string();
    let tag = etag(&body);
    let served = wanted(request_header(cx, header::ACCEPT).as_deref());

    if matches_tag(request_header(cx, header::IF_NONE_MATCH).as_deref(), &tag) {
        return (StatusCode::NOT_MODIFIED, respond_headers(None, &tag), ()).into_response(cx);
    }

    (StatusCode::OK, card_headers(served, &tag), Body::from(body)).into_response(cx)
}

/// The document itself.
///
/// Built from the constants rather than a checked-in file: a `server-card.json`
/// in the repository is a second place for the endpoint and the protocol
/// revisions to live, and one of the two would go stale. Separate from the
/// handler so a test reads it without a request, the way `robots.txt` and
/// `llms.txt` are.
fn document() -> Value {
    let endpoint = format!("{SITE_URL}{}", mcp::PATH);

    json!({
        "$schema": SCHEMA,
        "name": NAME,
        "title": mcp::SERVER_TITLE,
        "description": DESCRIPTION,
        "version": VERSION,
        "websiteUrl": SITE_URL,
        "remotes": [{
            "type": TRANSPORT,
            "url": endpoint,
            "supportedProtocolVersions": mcp::SUPPORTED_PROTOCOLS,
        }],
        // The second shape: what a client probing for this service reads.
        "serverInfo": mcp::server_info(),
        "transport": { "type": TRANSPORT, "endpoint": endpoint },
        "capabilities": mcp::capabilities(),
    })
}

/// The response headers, given the media type being served and the validator.
fn card_headers(content_type: &'static str, tag: &str) -> HeaderMap {
    respond_headers(Some(content_type), tag)
}

/// One request header, as a string, if it is there and readable.
fn request_header(cx: &Cx, name: HeaderName) -> Option<String> {
    headers(cx)
        .get(name)
        .and_then(|value| value.to_str().ok())
        .map(str::to_owned)
}

/// The media type for an `Accept` value.
///
/// The card's own type wins when it is named at all, quality value or not: a
/// client that asks for `application/mcp-server-card+json` has told this server
/// it reads the extension's documents, and the only other offer — a generic
/// `application/json` — is the same bytes. Everything else, including no
/// `Accept` at all, gets JSON.
fn wanted(accept: Option<&str>) -> &'static str {
    let names_it = accept.is_some_and(|accept| {
        accept.split(',').any(|entry| {
            entry
                .split(';')
                .next()
                .is_some_and(|name| name.trim().eq_ignore_ascii_case(MEDIA_TYPE))
        })
    });

    if names_it { MEDIA_TYPE } else { JSON }
}

/// A validator for the document's bytes.
///
/// The extension does not prescribe the tag's form, only that it identifies the
/// selected representation. It is computed rather than hand-written so a change
/// to any field of the card changes it; nothing about it is stable across
/// builds, which is fine — a client that revalidates after a redeploy is a
/// client that gets the current document.
fn etag(body: &str) -> String {
    let mut hasher = DefaultHasher::new();
    body.hash(&mut hasher);

    format!("\"{:016x}\"", hasher.finish())
}

/// Whether an `If-None-Match` value names `tag`.
///
/// A list, because the header is defined as one, and a weak tag (`W/"…"`)
/// compares equal by its opaque part: weak validation is exactly what a
/// byte-identical document needs.
fn matches_tag(asked: Option<&str>, tag: &str) -> bool {
    asked.is_some_and(|asked| {
        asked
            .split(',')
            .any(|value| value.trim().trim_start_matches("W/").trim() == tag)
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use regex_lite::Regex;

    /// The card says what the extension's schema requires it to say.
    ///
    /// Every constraint below is quoted from the published schema: the `$schema`
    /// URL is pinned by a pattern, `name` is `namespace/name` in reverse DNS,
    /// `description` is capped at 100 characters, and a remote needs a transport
    /// type from the schema's two-value enum and an `http(s)` URL.
    #[test]
    fn the_card_satisfies_the_schema_it_declares() {
        let card = document();

        assert_eq!(
            card["$schema"],
            "https://static.modelcontextprotocol.io/schemas/v1/server-card.schema.json"
        );

        let name = card["name"].as_str().expect("name is a string");
        assert!(
            Regex::new("^[a-zA-Z0-9.-]+/[a-zA-Z0-9._-]+$")
                .expect("the schema's pattern compiles")
                .is_match(name),
            "{name} is not namespace/name"
        );

        let description = card["description"].as_str().expect("a description");
        assert!(
            (1..=100).contains(&description.len()),
            "{} characters is outside the schema's 1..=100",
            description.len()
        );

        assert!(card["version"].as_str().is_some_and(|v| !v.is_empty()));
        assert_eq!(card["websiteUrl"], SITE_URL);

        let remotes = card["remotes"].as_array().expect("remotes is an array");
        assert_eq!(remotes.len(), 1);
        assert!(
            ["sse", "streamable-http"].contains(&remotes[0]["type"].as_str().unwrap()),
            "the transport is outside the schema's enum"
        );

        let url = remotes[0]["url"].as_str().expect("a url");
        assert!(url.starts_with("https://"), "{url}");
        assert_eq!(url, format!("{SITE_URL}{}", mcp::PATH));
        assert_eq!(
            remotes[0]["supportedProtocolVersions"],
            json!(mcp::SUPPORTED_PROTOCOLS)
        );
    }

    /// The second shape: the fields a client probing for an MCP service reads.
    /// The endpoint is a URL, and the capabilities name the one primitive this
    /// server has.
    #[test]
    fn the_card_carries_the_probing_client_shape() {
        let card = document();

        assert_eq!(card["serverInfo"]["name"], mcp::SERVER_NAME);
        assert_eq!(card["serverInfo"]["version"], VERSION);
        assert_eq!(card["transport"]["type"], TRANSPORT);
        assert_eq!(
            card["transport"]["endpoint"],
            format!("{SITE_URL}{}", mcp::PATH)
        );
        assert_eq!(card["capabilities"]["tools"]["listChanged"], false);
    }

    /// The card cannot contradict the handshake, because the two halves of it
    /// are the handshake's own builders. This is the extension's consistency
    /// requirement, enforced by construction rather than by a comment.
    #[test]
    fn the_card_is_the_handshake_it_describes() {
        let card = document();

        assert_eq!(card["serverInfo"], mcp::server_info());
        assert_eq!(card["capabilities"], mcp::capabilities());
        assert_eq!(card["version"], mcp::server_info()["version"]);
    }

    /// The path and the two media types. A card served at the wrong path is not
    /// found, and one served as `text/html` is not read.
    #[test]
    fn the_path_and_the_media_types_are_the_ones_that_were_promised() {
        assert_eq!(PATH, "/.well-known/mcp/server-card.json");
        assert_eq!(MEDIA_TYPE, "application/mcp-server-card+json");
        assert_eq!(mcp::PATH, "/mcp");
    }

    /// Content negotiation: only a caller that names the extension's type gets
    /// it; a browser, a scanner, or nothing at all gets plain JSON.
    #[test]
    fn the_card_media_type_is_served_only_on_request() {
        let asked = [
            "application/mcp-server-card+json",
            "application/mcp-server-card+json, application/json;q=0.5",
            "  APPLICATION/MCP-SERVER-CARD+JSON  ",
            "text/html, application/mcp-server-card+json",
        ];
        for accept in asked {
            assert_eq!(wanted(Some(accept)), MEDIA_TYPE, "{accept}");
        }

        let plain = [
            "application/json",
            "text/html",
            "application/mcp-server-card",
            "*/*",
        ];
        for accept in plain {
            assert_eq!(wanted(Some(accept)), JSON, "{accept}");
        }

        assert_eq!(wanted(None), JSON);
    }

    /// `If-None-Match`: the tag alone, a list containing it, and the weak form
    /// all match; anything else does not, and an absent header never does.
    #[test]
    fn a_client_that_already_has_this_card_gets_a_304() {
        let tag = etag(&document().to_string());

        assert!(matches_tag(Some(&tag), &tag));
        assert!(matches_tag(Some(&format!("\"other\", {tag}")), &tag));
        assert!(matches_tag(Some(&format!("W/{tag}")), &tag));
        assert!(matches_tag(Some(&format!(" {tag} ")), &tag));

        assert!(!matches_tag(Some("\"other\""), &tag));
        assert!(!matches_tag(Some("*"), &tag));
        assert!(!matches_tag(None, &tag));
    }

    /// The validator is derived from the bytes: a document that changed is a
    /// document whose tag changed, and two builds of the same constants agree.
    #[test]
    fn the_validator_follows_the_document() {
        let card = document().to_string();
        assert_eq!(card, document().to_string());
        assert_eq!(etag(&card), etag(&card));

        assert_ne!(etag("one"), etag("two"));
        assert!(etag(&card).starts_with('"') && etag(&card).ends_with('"'));
    }
}
