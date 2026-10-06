//! The API catalog — `/.well-known/api-catalog`.
//!
//! One small document that lists this site's APIs and, for each, where its
//! description and its status live. It is the well-known URI [RFC 9727] defines
//! for exactly this job, served in the format that RFC mandates: a Linkset
//! ([RFC 9264]) as `application/linkset+json`, with the profile parameter
//! naming [RFC 9727] so a caller can tell which reading of the format is meant.
//!
//! **It advertises the API that exists, and only that.** The rule the discovery
//! work keeps is that a document describing a service the site does not offer is
//! a lie told to machines, and the first client that follows one stops believing
//! the next. So the single entry anchors at the catalogue endpoint
//! ([`crate::routes::api::PATH`]) and its three members are routes this crate
//! serves: the OpenAPI description, `llms.txt` as the human-readable
//! documentation, and the health probe. `every_member_is_a_route_this_crate_serves`
//! is what holds that — the document is built from the routes' own constants,
//! and the test reads them back.
//!
//! **Why `llms.txt` is the `service-doc`.** RFC 8631's `service-doc` is the
//! link that a person follows to read about the API, and this site has exactly
//! one document written for that reader: `llms.txt`, which says what the
//! catalogue is, where each form of a song lives, and what a program can fetch.
//! A second copy of that prose only for this link would be a second document to
//! keep in step.
//!
//! **What this is not.** It is not the homepage's `service-desc`, which names
//! the MCP server card ([`crate::routes::card`]) and is promised once, on the
//! front door. The homepage carries `api-catalog` pointing here, and that is the
//! relation [RFC 9727] §3 adds to the four RFC 8288 already in use.

use serde_json::{Value, json};
use topcoat::{
    Result,
    context::Cx,
    router::{
        Body, HeaderValue, header,
        response::{IntoResponse, Response},
        route,
    },
};

use crate::domain::song::SITE_URL;
use crate::routes::{api, llms};

/// `/.well-known/api-catalog` — the path, in one place.
pub const PATH: &str = "/.well-known/api-catalog";

/// The media type [RFC 9727] §4.2 requires the document to be served as.
///
/// `pub(crate)` because the homepage's `api-catalog` `Link` names it: the header
/// and the response have to spell the type the same way.
pub(crate) const MEDIA_TYPE: &str = "application/linkset+json";

/// The profile URI [RFC 9727] §4.2 recommends the media type carry.
pub(crate) const PROFILE: &str = "https://www.rfc-editor.org/info/rfc9727";

/// How long the document may be cached. It changes only on a redeploy, so an
/// hour is a ceiling a client can rely on.
const CACHE: &str = "public, max-age=3600";

/// `GET /.well-known/api-catalog` — the catalog.
///
/// One representation, so no `Vary`: the format is fixed by the RFC that defines
/// the URI, and a caller that asked for something else is handed the document
/// the URI is specified to return.
#[route(GET "/.well-known/api-catalog")]
async fn api_catalog(cx: &Cx) -> Result<Response> {
    (
        [
            (
                header::CONTENT_TYPE,
                HeaderValue::from_str(&content_type())?,
            ),
            (header::CACHE_CONTROL, HeaderValue::from_static(CACHE)),
        ],
        Body::from(document().to_string()),
    )
        .into_response(cx)
}

/// The `Content-Type` value: the media type and the profile the RFC asks for.
fn content_type() -> String {
    format!("{MEDIA_TYPE}; profile=\"{PROFILE}\"")
}

/// The catalog document.
///
/// A Linkset with one entry: the context (`anchor`) is the catalogue endpoint,
/// and the members are the three routes that describe it or report on it. Every
/// URL is absolute, built from [`SITE_URL`] the way every other published URL
/// is: a catalog is quoted away from its own response more often than not.
///
/// Separate from the handler so a test reads it without a request, the way the
/// MCP card, `robots.txt` and `llms.txt` are read.
fn document() -> Value {
    json!({
        "linkset": [{
            "anchor": format!("{SITE_URL}{}", api::PATH),
            "service-desc": [{
                "href": format!("{SITE_URL}{}", api::OPENAPI_PATH),
                "type": "application/json",
            }],
            "service-doc": [{
                "href": format!("{SITE_URL}{}", llms::PATH),
                "type": "text/plain",
            }],
            "status": [{
                "href": format!("{SITE_URL}{}", api::HEALTH_PATH),
                "type": "application/json",
            }],
        }]
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The path and the two media types. A catalog at another path is not found,
    /// and one served without the profile is not read as an API catalog.
    #[test]
    fn the_path_and_the_media_types_are_the_ones_the_rfc_names() {
        assert_eq!(PATH, "/.well-known/api-catalog");
        assert_eq!(MEDIA_TYPE, "application/linkset+json");
        assert_eq!(PROFILE, "https://www.rfc-editor.org/info/rfc9727");
        assert_eq!(
            content_type(),
            "application/linkset+json; profile=\"https://www.rfc-editor.org/info/rfc9727\""
        );
    }

    /// The document has the shape the RFC requires: a `linkset` array, one entry
    /// per API, each with an `anchor` and the relations the scanner looks for.
    #[test]
    fn the_document_is_a_linkset_with_the_relations_the_rfc_names() {
        let document = document();
        let linkset = document["linkset"].as_array().expect("linkset is an array");

        assert_eq!(linkset.len(), 1, "one API, one entry");
        let entry = &linkset[0];

        let anchor = entry["anchor"].as_str().expect("an anchor URL");
        assert!(anchor.starts_with(SITE_URL), "{anchor}");
        assert_eq!(anchor, format!("{SITE_URL}{}", api::PATH));

        for relation in ["service-desc", "service-doc", "status"] {
            let links = entry[relation]
                .as_array()
                .unwrap_or_else(|| panic!("{relation} is an array"));
            assert!(!links.is_empty(), "{relation} names nothing");
            for link in links {
                let href = link["href"].as_str().unwrap_or_default();
                assert!(href.starts_with(SITE_URL), "{relation}: {href}");
                assert!(link["type"].is_string(), "{relation}: {href} has no type");
            }
        }
    }

    /// Every member is a route this crate serves.
    ///
    /// This is the honesty gate, not a formality: the members are written from
    /// the routes' own constants, and the test says which constants they are, so
    /// a catalog that grew a link to a document nobody serves fails here rather
    /// than in a client that followed it.
    #[test]
    fn every_member_is_a_route_this_crate_serves() {
        let document = document();
        let entry = &document["linkset"][0];

        let served = |relation: &str| -> Vec<String> {
            entry[relation]
                .as_array()
                .expect("an array")
                .iter()
                .map(|link| link["href"].as_str().expect("an href").to_owned())
                .collect()
        };

        assert_eq!(
            served("service-desc"),
            vec![format!("{SITE_URL}{}", api::OPENAPI_PATH)]
        );
        assert_eq!(
            served("service-doc"),
            vec![format!("{SITE_URL}{}", llms::PATH)]
        );
        assert_eq!(
            served("status"),
            vec![format!("{SITE_URL}{}", api::HEALTH_PATH)]
        );
    }

    /// The members' declared types are the types the routes answer with. A
    /// catalog that said `text/html` for a JSON document would be a smaller lie
    /// than a 404, but the same kind.
    #[test]
    fn the_types_are_the_types_those_routes_serve() {
        let document = document();
        let typed = |relation: &str| -> String {
            document["linkset"][0][relation][0]["type"]
                .as_str()
                .expect("a type")
                .to_owned()
        };

        assert_eq!(typed("service-desc"), "application/json");
        assert_eq!(typed("service-doc"), "text/plain");
        assert_eq!(typed("status"), "application/json");
    }
}
