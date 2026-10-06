//! `Accept: text/markdown`, and the `Link` headers that advertise it.
//!
//! A song page has two representations and one URL. Asked for HTML — which is
//! every browser, and the default — it renders the sheet inside the site's
//! chrome. Asked for `text/markdown` it answers with the same song as one
//! Markdown document: the title, the credits, the lyric with its chords inline
//! as `[Eb]`, and the canonical URL. That is the highest-value item on the
//! list, because it is the difference between an agent reading the lyric and
//! an agent scraping it out of markup.
//!
//! **Why this is a layer and not a page.** A `#[page]` renders a view, and every
//! layout whose path is a prefix of the page's wraps it; the site's layout sits
//! at `/`. So a page *cannot* return a document that is not the HTML chrome — and
//! the two representations differ in their media type, which a view cannot
//! change. A pathless [`Layer`] sits outside all of it: it reads the request
//! before routing has a say, and answers the Markdown requests itself. It is
//! registered by hand in [`crate::router`] because `#[layer]` always carries a
//! path, and this one deliberately carries none.
//!
//! The same layer queues the `Link` headers (RFC 8288) on the HTML responses it
//! passes through, because they say the same thing the negotiation does: this
//! page has a Markdown form, a sitemap, a machine-readable description. They are
//! written here rather than in the layout so that both halves of the claim live
//! in one file — a header promising a `text/markdown` variant that the layer
//! does not serve would be worse than no header.
//!
//! Three properties the code below is arranged to keep:
//!
//! * **No database read on the common path.** The Markdown branch reads the song
//!   once; every other request — HTML, images, the API — never opens the pool.
//!   The negotiation is decided from the request's own headers and path first.
//! * **A request that asks for Markdown but names no song, or a draft, falls
//!   through to the router**, which is what turns it into the site's 404. The
//!   layer does not invent an error response of its own.
//! * **The links are on the response, not on the representation.** They are
//!   queued through `response_headers`, so they survive whatever the response
//!   turns out to be, errors included.

use topcoat::{
    Result,
    context::Cx,
    router::{
        Body, Layer, LayerFuture, Next, Path, header,
        request::{headers, uri},
        response::{IntoResponse, Response, response_headers},
    },
};

use crate::db;
use crate::domain::Song;
use crate::domain::song::SITE_URL;
use crate::routes::{api, sitemap};
use crate::state;

/// The media type this layer serves on request.
const MARKDOWN: &str = "text/markdown; charset=utf-8";

/// The song page's own prefix, and the one place this layer looks for a song.
///
/// `pages::song` declares the same path with `#[page("/himene/{id}")]`. The two
/// are not shared because the layer needs the prefix, not the route, and
/// `path_param!` hands a matched segment rather than the path parts of it. The
/// failure mode of a drift between them is a Markdown variant that 404s while
/// the sheet still renders, which `.run/step10.sh` asserts against.
const SONG_PREFIX: &str = "/himene/";

/// Marks the layout's output as a document this layer's links describe.
const HTML: &str = "text/html";

/// The negotiator: one Markdown branch, one pass-through with `Link` headers.
///
/// A unit value — it holds no state, and the request context is where everything
/// it reads lives.
pub struct Negotiation;

impl Layer for Negotiation {
    /// `None`: this layer is outside every route, not under a path prefix.
    fn path(&self) -> Option<&Path> {
        None
    }

    fn handle<'a>(&'a self, cx: &'a Cx, body: Body, next: Next<'a>) -> LayerFuture<'a> {
        Box::pin(async move {
            let path = uri(cx).path();
            let wanted = header(cx, header::ACCEPT);

            if prefers_markdown(wanted.as_deref())
                && let Some(id) = song_id(path)
                && let Some(sheet) = db::song(state::db(cx).pool(), id)
                    .await?
                    .filter(Song::is_published)
            {
                queue_links(cx, path, true)?;
                return markdown(cx, &sheet);
            }

            let response = next.run(cx, body).await?;

            // Only a document gets a document's links. The API, the sitemaps,
            // `robots.txt` and the served stylesheet all pass through here too,
            // and none of them is described by them.
            if is_html(&response) {
                queue_links(cx, path, false)?;
            }

            Ok(response)
        })
    }
}

/// One request header, as a string, if it is there and readable.
fn header(cx: &Cx, name: header::HeaderName) -> Option<String> {
    headers(cx)
        .get(name)
        .and_then(|value| value.to_str().ok())
        .map(str::to_owned)
}

/// The id of the song a path names, or `None` if it names no song.
///
/// One segment below the prefix and nothing else. `/himene/{id}/anything` is
/// nobody's page and answers `None`; `/himene/sitemap.xml` *is* one segment, but
/// no song carries that id, so the read below finds nothing and the request
/// falls through to the router either way — which is what knows what both of
/// them are.
fn song_id(path: &str) -> Option<&str> {
    let id = path.strip_prefix(SONG_PREFIX)?;
    (!id.is_empty() && !id.contains('/')).then_some(id)
}

/// Whether the client asked for Markdown **in preference to** HTML.
///
/// Quality values are honoured, the way [`crate::i18n`]'s language negotiation
/// honours them, and the comparison is strict. So:
///
/// * `text/markdown` alone → Markdown. Nothing else was offered.
/// * `text/markdown, text/html;q=0.8` → Markdown.
/// * `text/html, text/markdown;q=0.8` → HTML. Asking for both and weighting HTML
///   is not a request for Markdown.
/// * `*/*`, or no `Accept` at all → HTML. An unqualified request is a browser.
///
/// `text/markdown;q=0` is the specification's "explicitly not acceptable" and is
/// treated exactly like an absent entry.
fn prefers_markdown(accept: Option<&str>) -> bool {
    let Some(accept) = accept else {
        return false;
    };

    let markdown = quality(accept, "text/markdown").unwrap_or(0.0);
    markdown > 0.0 && markdown > quality(accept, "text/html").unwrap_or(0.0)
}

/// The quality `accept` gives `media`, if it names it at all.
///
/// The first entry wins a repeat, which is what a header that named the same
/// type twice would mean. A malformed `q` is treated as the default rather than
/// as zero, matching [`crate::i18n`]'s reading of `Accept-Language`.
fn quality(accept: &str, media: &str) -> Option<f32> {
    accept.split(',').find_map(|entry| {
        let mut fields = entry.split(';');
        let name = fields.next()?.trim();
        if !name.eq_ignore_ascii_case(media) {
            return None;
        }

        Some(
            fields
                .find_map(|field| field.trim().strip_prefix("q="))
                .and_then(|value| value.trim().parse::<f32>().ok())
                .unwrap_or(1.0),
        )
    })
}

/// Whether a response is an HTML document.
fn is_html(response: &Response) -> bool {
    response
        .headers()
        .get(header::CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .is_some_and(|value| value.starts_with(HTML))
}

/// The song as one Markdown document.
///
/// The heading, the credits on their own line in emphasis, the lyric as
/// [`Song::lyrics_markdown`] writes it — chords kept inline at the offset the
/// author put them — and the canonical URL, so a document quoted out of context
/// still says where it came from.
///
/// `pub(crate)` because the MCP server (`src/routes/mcp.rs`) answers `get_song`
/// with exactly this document: an agent reading a song over MCP and an agent
/// reading it with `Accept: text/markdown` must not get two different sheets.
pub(crate) fn song_document(sheet: &Song) -> String {
    let artists = sheet
        .get_artists()
        .iter()
        .map(|artist| artist.get_fullname())
        .collect::<Vec<String>>()
        .join(", ");

    let mut out = format!("# {}\n\n", sheet.get_title());
    if !artists.is_empty() {
        out.push_str(&format!("_{artists}_\n\n"));
    }
    out.push_str(&sheet.lyrics_markdown());
    out.push_str(&format!("\n\nSource: {}\n", sheet.get_url()));

    out
}

/// The Markdown response.
///
/// `Vary: Accept` is not optional here: the same URL answers with two media
/// types, and without it a cache is entitled to hand a browser the Markdown one.
/// The HTML branch carries it too, set by Topcoat's own view response.
fn markdown(cx: &Cx, sheet: &Song) -> Result<Response> {
    (
        [
            (
                header::CONTENT_TYPE,
                header::HeaderValue::from_static(MARKDOWN),
            ),
            (header::VARY, header::HeaderValue::from_static("Accept")),
        ],
        Body::from(song_document(sheet)),
    )
        .into_response(cx)
}

/// Queue this path's `Link` headers on the response being built.
///
/// Separate `Link` headers rather than one comma-joined value: RFC 8288 permits
/// both, and separate ones keep the quoting inside each value simple.
fn queue_links(cx: &Cx, path: &str, markdown: bool) -> Result<()> {
    for value in links(path, markdown) {
        response_headers(cx).append(header::LINK, header::HeaderValue::from_str(&value)?);
    }

    Ok(())
}

/// The `Link` values a path deserves, given the representation being served.
///
/// Absolute, built from [`SITE_URL`] the way `robots.txt`'s `Sitemap`
/// directives and every `<loc>` are: a `Link` value is permitted a relative
/// reference, but a header is exactly the thing that gets quoted away from the
/// response it arrived on, and one that no longer resolves is a broken promise.
///
/// * Every document names the sitemap.
/// * A song names its JSON read under `describedby`, and the other of its two
///   representations under `alternate`.
/// * The song index names the catalogue, which is the machine-readable form of
///   the same list.
/// * Nothing else: the home page has no JSON counterpart, and a `Link` to
///   something that does not describe it is a wrong answer rather than a
///   missing one.
fn links(path: &str, served_markdown: bool) -> Vec<String> {
    let sitemap_link = format!("<{SITE_URL}{}>; rel=\"sitemap\"", sitemap::PATH);

    match song_id(path) {
        Some(id) => vec![
            sitemap_link,
            format!("<{SITE_URL}{}/{id}>; rel=\"describedby\"", api::PATH),
            format!(
                "<{SITE_URL}{SONG_PREFIX}{id}>; rel=\"alternate\"; type=\"{}\"",
                if served_markdown {
                    HTML
                } else {
                    "text/markdown"
                }
            ),
        ],
        None if path == crate::pages::songs::PATH => vec![
            sitemap_link,
            format!("<{SITE_URL}{}>; rel=\"describedby\"", api::PATH),
        ],
        None => vec![sitemap_link],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The negotiation, case by case. Each of these is a real header a client
    /// sends: the first is a plain `curl -H 'Accept: text/markdown'`, the last
    /// is every browser.
    #[test]
    fn markdown_is_served_only_to_a_client_that_asked_for_it_first() {
        let markdown = [
            "text/markdown",
            "text/markdown, text/html;q=0.8",
            "text/markdown;q=0.9, text/html;q=0.1",
            "TEXT/MARKDOWN",
            "  text/markdown  ",
        ];
        for accept in markdown {
            assert!(prefers_markdown(Some(accept)), "{accept}");
        }

        let html = [
            "text/html",
            "text/html, text/markdown;q=0.8",
            "text/markdown;q=0",
            "text/markdown;q=0, text/html",
            "*/*",
            "application/json",
            "text/markdown;q=0.5, text/html;q=0.5",
        ];
        for accept in html {
            assert!(!prefers_markdown(Some(accept)), "{accept}");
        }

        assert!(!prefers_markdown(None));
    }

    /// The path reader: one segment and no trailing slash.
    ///
    /// `/himene/` is the index's trailing slash and `/himene/a/b` is nobody's
    /// page; both answer `None`. `/himene/sitemap.xml` reads as one segment and
    /// is pinned down below, because it is the database read that rejects it and
    /// not the path — the router still has the route.
    #[test]
    fn only_a_single_segment_under_himene_is_a_song() {
        assert_eq!(
            song_id("/himene/8nntgjk4rl5dbp67c6en"),
            Some("8nntgjk4rl5dbp67c6en")
        );
        assert_eq!(song_id("/himene/sitemap.xml"), Some("sitemap.xml"));
        assert_eq!(song_id("/himene/"), None);
        assert_eq!(song_id("/himene"), None);
        assert_eq!(song_id("/himene/a/b"), None);
        assert_eq!(song_id("/api/songs/x"), None);
        assert_eq!(song_id("/"), None);
    }

    /// Every document names the sitemap; a song names its JSON read and its
    /// other representation; the index names the catalogue; nothing else is
    /// described by anything.
    ///
    /// Every value is absolute, because a header outlives the response it
    /// arrived on.
    #[test]
    fn the_links_say_what_the_page_is_and_where_its_other_forms_are() {
        let song = links("/himene/8nntgjk4rl5dbp67c6en", false);
        assert_eq!(song.len(), 3);
        assert_eq!(
            song[0],
            format!("<{SITE_URL}{}>; rel=\"sitemap\"", sitemap::PATH)
        );
        assert_eq!(
            song[1],
            format!(
                "<{SITE_URL}{}/8nntgjk4rl5dbp67c6en>; rel=\"describedby\"",
                api::PATH
            )
        );
        assert_eq!(
            song[2],
            format!(
                "<{SITE_URL}/himene/8nntgjk4rl5dbp67c6en>; rel=\"alternate\"; type=\"text/markdown\""
            )
        );

        // Serving Markdown flips the alternate to the HTML document, because the
        // other representation is now the one the caller did not get.
        let as_markdown = links("/himene/8nntgjk4rl5dbp67c6en", true);
        assert!(as_markdown[2].ends_with("rel=\"alternate\"; type=\"text/html\""));
        assert_eq!(as_markdown[0], song[0]);

        let index = links(crate::pages::songs::PATH, false);
        assert_eq!(index.len(), 2);
        assert_eq!(
            index[1],
            format!("<{SITE_URL}{}>; rel=\"describedby\"", api::PATH)
        );

        assert_eq!(links("/", false).len(), 1);
        assert_eq!(links("/aepa", false).len(), 1);
    }

    /// The document is Markdown a reader can use: an `# ` title, then the lyric
    /// with its chords, then the source.
    #[test]
    fn the_document_opens_with_the_title_and_closes_with_its_url() {
        let sheet = Song::new(
            "8nntgjk4rl5dbp67c6en".to_owned(),
            "Te here".to_owned(),
            "<div>Hina'a<sup data-nosnippet=\"true\">Eb</sup>ro</div>".to_owned(),
            1,
            Vec::new(),
            true,
            chrono::Utc::now(),
            chrono::Utc::now(),
        );

        let text = song_document(&sheet);
        let mut lines = text.lines();

        assert_eq!(lines.next(), Some("# Te here"));
        assert!(text.contains("Hina'a[Eb]ro"), "{text}");
        assert!(text.ends_with(&format!("Source: {}\n", sheet.get_url())));
        // No artist line for an uncredited song — the same rule the sheet keeps.
        assert!(!text.contains("\n_\n"), "{text}");
    }
}
