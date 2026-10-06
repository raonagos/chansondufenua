//! `/llms.txt`.
//!
//! The convention from <https://llmstxt.org>: one small text document, at a
//! predictable path, that says what the site is and where its content lives —
//! the prose half of what `/robots.txt` and the JSON API say the other way
//! round. It is written for a reader (a model given the URL, an agent deciding
//! whether this site answers a question), so it is a document and not a
//! manifest.
//!
//! The links are absolute. The convention's own examples use relative ones and
//! both work for a caller that already knows the origin, but an absolute URL
//! survives being quoted out of context — which is exactly what happens to a
//! file like this.
//!
//! Like `robots.txt`, this is a `#[route]`: it is a document, but not a page,
//! and it must not be wrapped in the site's HTML chrome.

use topcoat::{Result, router::route};

use crate::domain::song::SITE_URL;
use crate::pages::{editor, songs};
use crate::routes::{api, card, catalog, mcp, sitemap};

/// `/llms.txt` — the path, in one place.
pub const PATH: &str = "/llms.txt";

/// `GET /llms.txt`.
///
/// A plain `String`, which `IntoResponse` sends as `text/plain; charset=utf-8`.
/// The convention does not ask for a custom type, and `text/plain` is what a
/// caller reads it with.
#[route(GET "/llms.txt")]
async fn llms() -> Result<String> {
    Ok(body())
}

/// The document, with its links resolved against the canonical host.
fn body() -> String {
    format!(
        "\
# Chanson du fenua

> A collection of French Polynesian songs: the lyrics, with the chords written
> where they are played. The catalogue is small, public and non-commercial.
> Every page is a server-rendered document — nothing here needs JavaScript to be
> read.

## Where the content is

- [Song index]({SITE_URL}{songs}): every published song, newest first.
- [One song]({SITE_URL}/himene/{{id}}): the lyric with its chords. Add
  `?lang=ty` for the Tahitian chrome; the lyrics themselves are as written.

## For a program

- [Songs]({SITE_URL}{api}): the catalogue as JSON — id, title, artists, URL,
  view count, timestamps. [Health]({SITE_URL}{health}) reports the counts and the
  version.
- [One song as JSON]({SITE_URL}{api}/{{id}}): the same, plus the lyrics as
  Markdown with the chords inline.
- [OpenAPI description]({SITE_URL}{openapi}): the JSON API's paths and shapes,
  for a client that generates code from it.
- [API catalog]({SITE_URL}{catalog}): the API as a linkset (RFC 9727) — the
  OpenAPI description, this file, and the health probe.
- Send `Accept: text/markdown` to a song page for the same document as Markdown.
- [MCP server]({SITE_URL}{mcp}): a read-only Model Context Protocol server with
  two tools — `list_songs` pages the catalogue, `get_song` reads one sheet. Its
  [server card]({SITE_URL}{card}) names the endpoint and the protocol versions.
- [Sitemaps]({SITE_URL}{sitemap}): the site's fixed pages; the songs are at
  {SITE_URL}{song_sitemap}.

## Writing

- [Add a song]({SITE_URL}{editor}): a form. It is the only writable route, and
  `robots.txt` keeps crawlers out of it.

## Policy

Search engines may index and link to this site, and agents may read it. The
lyrics are the credited artists' work and are not training data:
`Content-Signal: search=yes, ai-input=yes, ai-train=no` in
[{SITE_URL}/robots.txt]({SITE_URL}/robots.txt).
",
        songs = songs::PATH,
        api = api::PATH,
        openapi = api::OPENAPI_PATH,
        catalog = catalog::PATH,
        health = api::HEALTH_PATH,
        sitemap = sitemap::PATH,
        song_sitemap = sitemap::SONGS_PATH,
        editor = editor::PATH,
        mcp = mcp::PATH,
        card = card::PATH,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The document is Markdown, and its one required element is a heading —
    /// a caller that finds no `# ` at the top has not found the file it asked
    /// for.
    #[test]
    fn the_document_opens_with_a_heading_and_a_summary() {
        let text = body();
        let mut lines = text.lines();

        assert_eq!(lines.next(), Some("# Chanson du fenua"));
        assert_eq!(lines.next(), Some(""));
        assert!(lines.next().is_some_and(|line| line.starts_with("> ")));
    }

    /// Every entry point it names is a route the crate registers. The point of
    /// the file is to be followed, so a link to a 404 is a defect and not a
    /// typo.
    ///
    /// Its own address is not in the list: a file that linked to itself would be
    /// making a link its reader already has.
    #[test]
    fn every_entry_point_is_a_served_path() {
        let text = body();
        let served = [
            (songs::PATH, "/himene"),
            (api::PATH, "/api/songs"),
            (api::HEALTH_PATH, "/api/health"),
            (api::OPENAPI_PATH, "/api/openapi.json"),
            (catalog::PATH, "/.well-known/api-catalog"),
            (sitemap::PATH, "/sitemap.xml"),
            (sitemap::SONGS_PATH, "/himene/sitemap.xml"),
            (editor::PATH, "/himene/api"),
            (mcp::PATH, "/mcp"),
            (card::PATH, "/.well-known/mcp/server-card.json"),
        ];

        for (constant, expected) in served {
            assert_eq!(constant, expected);
            assert!(
                text.contains(&format!("{SITE_URL}{constant}")),
                "llms.txt does not link to {constant}"
            );
        }

        assert_eq!(PATH, "/llms.txt");
    }
}
