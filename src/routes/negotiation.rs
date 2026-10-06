//! `Accept: text/markdown`, and the `Link` headers that advertise it.
//!
//! The site's prose pages have two representations and one URL each. Asked for
//! HTML — which is every browser, and the default — a page renders inside the
//! site's chrome. Asked for `text/markdown` it answers with the same page as one
//! Markdown document: for a song the title, the credits, the lyric with its
//! chords inline as `[Eb]`, and the canonical URL; for the front page and the
//! index the same prose and the same catalogue, every title a link. That is the
//! difference between an agent reading the content and an agent scraping it out
//! of markup.
//!
//! **Which pages have a Markdown form** is one list, spelled in the private
//! `document` below: a song, the front page (`/`, and the `/aepa` duplicate of
//! it), and the index. `links` is the same list read the other way round — a
//! page with two representations names the one a given response is not — so a
//! header promising a variant this layer does not serve cannot be written.
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
//! * **No database read on the common path.** The Markdown branch reads what the
//!   document it is building needs, once; every other request — HTML, images,
//!   the API — never opens the pool. The negotiation is decided from the
//!   request's own headers and path first.
//! * **A request that asks for Markdown on a path with no Markdown form — or
//!   names a song that is missing or a draft — falls through to the router**,
//!   which is what turns it into the site's 404. The layer does not invent an
//!   error response of its own.
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

use crate::db::{self, SongOrder};
use crate::domain::Song;
use crate::domain::song::SITE_URL;
use crate::i18n::{self, Key, Lang};
use crate::pages::{
    home,
    // The two front-page URLs. Their owner is `pages::home` — the page that
    // `#[page("/…")]` declares them in — because the layout matches the request
    // path against the same two constants to decide the document head.
    home::{AEPA_PATH as AEPA, PATH as HOME},
    songs,
};
use crate::routes::{api, card, catalog, llms, sitemap};
use crate::state;

/// The media type this layer serves on request.
const MARKDOWN: &str = "text/markdown; charset=utf-8";

/// The same media type without its charset, for the places that compare or
/// name a type rather than set one: [`links`]' `alternate` values.
const MARKDOWN_TYPE: &str = "text/markdown";

/// The language a Markdown document is written in.
///
/// The default one, always. This layer answers outside the cookie layer — the
/// jar is installed *inside* it, so [`crate::i18n::resolve`], which reads it,
/// would panic here — and the language the Markdown form is written in is
/// therefore the site's default. What that costs is small and bounded: the
/// lyrics are the content and are never translated, the front page's prose is
/// French in every language, and the handful of labels the documents quote are
/// chrome. The language-in-the-URL step is what closes the gap: once `/ty/…`
/// exists, the language is a prefix of the path this layer already reads.
const MARKDOWN_LANG: Lang = Lang::Fr;

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
                && let Some(text) = document(cx, path).await?
            {
                queue_links(cx, path, true)?;
                return markdown(cx, text);
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
/// nobody's page; `/himene/sitemap.xml` *is* one segment and is a route of its
/// own, and so is the create-song page — the router prefers a literal segment to
/// a parameter, so both reach their own handlers and never
/// `#[page("/himene/{id}")]`. They are excluded here rather than left for the
/// database read to reject, because the read is not the only caller: [`links`]
/// writes promises from this answer, and a page advertising a Markdown form it
/// does not have is a header lying about the response it arrived on.
///
/// `pub(crate)` because the layout asks the same question: it decides the
/// document head from the path, and a song URL that names no published song
/// becomes the 404's head. Two answers to "is this a song URL" would be a page
/// and its `<head>` disagreeing about which pages exist.
pub(crate) fn song_id(path: &str) -> Option<&str> {
    let id = path.strip_prefix(SONG_PREFIX)?;
    let is_song = !id.is_empty()
        && !id.contains('/')
        && path != crate::pages::editor::PATH
        && path != sitemap::SONGS_PATH;

    is_song.then_some(id)
}

/// The Markdown form of a path, if the path has one.
///
/// This is the list the module docs promise, in one place, and it is the same
/// list [`links`] reads when it decides whether a page has an `alternate`. A
/// path that is not in it answers `None` — the API, the two sitemaps,
/// `robots.txt`, `llms.txt`, the create-song form and the served stylesheet are
/// configured data and forms, not prose, and none of them has a Markdown form to
/// serve.
///
/// The reads happen here, in the branch that needs them, and only there: a
/// request for HTML — which is every browser — never reaches this function.
async fn document(cx: &Cx, path: &str) -> Result<Option<String>> {
    if let Some(id) = song_id(path) {
        let sheet = db::song(state::db(cx).pool(), id)
            .await?
            .filter(Song::is_published);
        return Ok(sheet.as_ref().map(song_document));
    }

    // `/aepa` is the front page under a second URL, and the same document:
    // everything but the canonical URL is identical, and the document names the
    // canonical host either way.
    if path == HOME || path == AEPA {
        return home_document(cx).await.map(Some);
    }

    if path == songs::PATH {
        return index_document(cx).await.map(Some);
    }

    Ok(None)
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

/// The front page as one Markdown document — `/`, and `/aepa` with it.
///
/// The same content the HTML page carries, in the same order: the hero, the
/// three cards, the synopsis, the two tables and the closing block. The prose is
/// [`home::copy`]'s, which is what the HTML renders, so the two cannot drift;
/// the two button labels are chrome and come from [`crate::i18n`] like the
/// page's.
///
/// **The tables list titles, not lyric lines.** The HTML's newest table leads
/// with the lyric because it can truncate it to one line with a utility class;
/// in Markdown there is nothing to truncate with, and five inlined lyrics would
/// make this an anthology rather than a front page. Each title is a link, and
/// every one of those links has its own Markdown form with the whole lyric in
/// it — which is where a reader who wants the words is going anyway.
async fn home_document(cx: &Cx) -> Result<String> {
    let pool = state::db(cx).pool();
    let latest = db::songs(pool, SongOrder::Newest, Some(home::ROWS)).await?;
    let most_viewed = db::songs(pool, SongOrder::MostViewed, Some(home::ROWS)).await?;

    let mut out = format!("# {}\n\n", home::copy::HERO_TITLE);
    out.push_str(&format!("_{}_\n\n", home::copy::HERO_SUBTITLE));
    out.push_str(&format!(
        "[{}]({SITE_URL}{})\n\n",
        i18n::text(MARKDOWN_LANG, Key::HomeDiscover),
        songs::PATH
    ));

    for (title, text) in home::copy::CARDS {
        out.push_str(&format!("## {title}\n\n{text}\n\n"));
    }

    out.push_str(&format!("{}\n\n", home::copy::SYNOPSIS));
    out.push_str(&format!("## {}\n\n", home::copy::TABLE_LATEST));
    out.push_str(&song_list(&latest));
    out.push_str(&format!("\n\n## {}\n\n", home::copy::TABLE_MOST_VIEWED));
    out.push_str(&song_list(&most_viewed));
    out.push_str(&format!("\n\n## {}\n\n", home::copy::FOOT_TITLE));
    out.push_str(&format!("{}\n\n", home::copy::FOOT_TEXT));
    // The two closing buttons. The first is a brand name and is not translated;
    // the second is the create-song page's label.
    out.push_str(&format!(
        "[{}](https://facebook.com/chansondufenua) · [{}]({SITE_URL}{})\n",
        home::copy::FOOT_FACEBOOK,
        i18n::text(MARKDOWN_LANG, Key::HomeStart),
        crate::pages::editor::PATH
    ));
    out.push_str(&format!("\nSource: {SITE_URL}\n"));

    Ok(out)
}

/// The song index as one Markdown document — `/himene`.
///
/// Every published song, newest first, in the order the HTML table shows them
/// and from the same unbounded read: an index that promises every song has to
/// list every song in both of its representations. The heading is chrome, and so
/// is the line an empty catalogue gets.
async fn index_document(cx: &Cx) -> Result<String> {
    let listed = db::songs(state::db(cx).pool(), SongOrder::Newest, None).await?;

    let mut out = format!("# {}\n\n", i18n::text(MARKDOWN_LANG, Key::IndexTitle));
    if listed.is_empty() {
        out.push_str(&format!("{}\n", i18n::text(MARKDOWN_LANG, Key::IndexEmpty)));
    } else {
        out.push_str(&song_list(&listed));
        out.push('\n');
    }
    out.push_str(&format!("\nSource: {SITE_URL}{}\n", songs::PATH));

    Ok(out)
}

/// A list of songs, one line each: the title as a link, then its credits.
///
/// The URL is absolute, built from [`SITE_URL`] the way every other URL this
/// site publishes is: a Markdown document is the thing most likely to be quoted
/// away from the response it arrived on, and a root-relative link in an agent's
/// context resolves against nothing.
fn song_list(listed: &[Song]) -> String {
    listed
        .iter()
        .map(|song| {
            let artists = song
                .get_artists()
                .iter()
                .map(|artist| artist.get_fullname())
                .collect::<Vec<String>>()
                .join(", ");
            let title = link_text(&song.get_title());
            let url = song.get_url();

            if artists.is_empty() {
                format!("- [{title}]({url})")
            } else {
                format!("- [{title}]({url}) — {artists}")
            }
        })
        .collect::<Vec<String>>()
        .join("\n")
}

/// A song title as Markdown link text.
///
/// Two adjustments, both about staying inside one list entry: the three
/// characters that would otherwise end the link early are backslash-escaped, and
/// any run of whitespace — a newline in a title would end the entry — folds to a
/// single space.
fn link_text(title: &str) -> String {
    let mut out = String::with_capacity(title.len());
    let mut pending_space = false;

    for ch in title.chars() {
        if ch.is_whitespace() {
            pending_space = true;
            continue;
        }
        if pending_space && !out.is_empty() {
            out.push(' ');
        }
        pending_space = false;

        if matches!(ch, '\\' | '[' | ']') {
            out.push('\\');
        }
        out.push(ch);
    }

    out
}

/// The Markdown response.
///
/// `Vary: Accept` is not optional here: the same URL answers with two media
/// types, and without it a cache is entitled to hand a browser the Markdown one.
/// The HTML branch carries it too, set by Topcoat's own view response.
///
/// `x-markdown-tokens` is the token-count hint the Markdown-for-Agents
/// convention asks for, and it is an estimate [`estimated_tokens`] explains.
fn markdown(cx: &Cx, text: String) -> Result<Response> {
    (
        [
            (
                header::CONTENT_TYPE,
                header::HeaderValue::from_static(MARKDOWN),
            ),
            (header::VARY, header::HeaderValue::from_static("Accept")),
            (
                header::HeaderName::from_static("x-markdown-tokens"),
                header::HeaderValue::from(estimated_tokens(&text)),
            ),
        ],
        Body::from(text),
    )
        .into_response(cx)
}

/// The document's size in tokens, as an estimate.
///
/// There is no tokenizer in this binary and there should not be one: a
/// tokenizer belongs to a model, and this header is the framework-agnostic hint
/// the convention asks for — "the estimated number of tokens in the Markdown
/// document". The estimate is one token per four UTF-8 bytes, rounded up, which
/// is the ratio the reference page for the convention shows in its own example
/// (2,899 bytes answered with `x-markdown-tokens: 725`). It is a heuristic and
/// the header's own name says so: a caller that needs an exact count has the
/// document itself.
fn estimated_tokens(text: &str) -> u64 {
    (text.len() as u64).div_ceil(4)
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
/// * Every page with a Markdown form names the other of its two representations
///   under `alternate` — a song, the front page, its `/aepa` duplicate, and the
///   index. That is the same list [`document`] serves, and it is derived from
///   `served_markdown` rather than from a second table.
/// * A song names its JSON read under `describedby`.
/// * The song index names the catalogue, which is the machine-readable form of
///   the same list.
/// * The home page describes the MCP server: `service-desc` is the registered
///   relation for a resource that describes a service, and the card at
///   [`crate::routes::card::PATH`] is that description. It is one link, on the
///   front door, because the card is about the site and not about whichever page
///   a visitor happened to land on.
/// * The home page also names the API catalog ([`crate::routes::catalog::PATH`])
///   under `api-catalog` — RFC 9727 §3's recommendation, and the entry point a
///   client follows to find the OpenAPI description — and its own written
///   description, `llms.txt`, under `describedby`. Both are site-level promises
///   and both are therefore on `/` alone, like the card.
/// * Nothing else: nothing else on this site has a machine-readable form that a
///   `Link` to something absent would describe.
fn links(path: &str, served_markdown: bool) -> Vec<String> {
    let sitemap_link = format!("<{SITE_URL}{}>; rel=\"sitemap\"", sitemap::PATH);

    // The representation a caller did not get. `alternate` is the registered
    // relation for another form of the same resource, and the type is what says
    // which one.
    let other = if served_markdown { HTML } else { MARKDOWN_TYPE };
    let alternate = |url: String| format!("<{url}>; rel=\"alternate\"; type=\"{other}\"");

    if let Some(id) = song_id(path) {
        return vec![
            sitemap_link,
            format!("<{SITE_URL}{}/{id}>; rel=\"describedby\"", api::PATH),
            alternate(format!("{SITE_URL}{SONG_PREFIX}{id}")),
        ];
    }

    match path {
        songs::PATH => vec![
            sitemap_link,
            format!("<{SITE_URL}{}>; rel=\"describedby\"", api::PATH),
            alternate(format!("{SITE_URL}{}", songs::PATH)),
        ],
        // `/aepa` is the front page under a second URL, so it has the same two
        // representations; it does not repeat the card link, which describes
        // the site and is promised once, on `/`.
        AEPA => vec![sitemap_link, alternate(format!("{SITE_URL}{AEPA}"))],
        HOME => vec![
            sitemap_link,
            alternate(format!("{SITE_URL}{HOME}")),
            card_link(),
            catalog_link(),
            describedby_link(),
        ],
        _ => vec![sitemap_link],
    }
}

/// The MCP server card, as a `Link` value.
///
/// Absolute and typed: a caller that finds this header without ever having heard
/// of the site must be able to fetch the card and know what it is reading, and
/// `service-desc` (RFC 8631) is the registered relation for exactly that.
fn card_link() -> String {
    format!(
        "<{SITE_URL}{}>; rel=\"service-desc\"; type=\"{}\"",
        card::PATH,
        card::MEDIA_TYPE
    )
}

/// The API catalog, as a `Link` value.
///
/// RFC 9727 §3's own recommendation: the site names the catalog document that
/// lists its APIs, so a client that found the front page can find the API
/// without knowing the well-known path. The type is the one the catalog is
/// served as.
fn catalog_link() -> String {
    format!(
        "<{SITE_URL}{}>; rel=\"api-catalog\"; type=\"{}\"",
        catalog::PATH,
        catalog::MEDIA_TYPE
    )
}

/// The site's written description of itself, as a `Link` value.
///
/// `describedby` is the registered relation for "the target describes the link's
/// context", and `llms.txt` is this site's description: what it is, where its
/// content is, and what a program can read. One link, on the front door, for the
/// same reason the card is: it describes the site, not the page a visitor landed
/// on.
fn describedby_link() -> String {
    format!(
        "<{SITE_URL}{}>; rel=\"describedby\"; type=\"text/plain\"",
        llms::PATH
    )
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
    /// page; both answer `None`. The two paths below the prefix that the router
    /// serves itself — the songs sitemap and the create-song page — answer
    /// `None` too, even though each is one segment: they are pages, not songs,
    /// and the layer must not describe them as songs.
    #[test]
    fn only_a_single_segment_under_himene_is_a_song() {
        assert_eq!(
            song_id("/himene/8nntgjk4rl5dbp67c6en"),
            Some("8nntgjk4rl5dbp67c6en")
        );
        assert_eq!(song_id("/himene/sitemap.xml"), None);
        assert_eq!(song_id(crate::pages::editor::PATH), None);
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

        // The index names the catalogue and its own Markdown form.
        let index = links(songs::PATH, false);
        assert_eq!(index.len(), 3);
        assert_eq!(index[0], song[0]);
        assert_eq!(
            index[1],
            format!("<{SITE_URL}{}>; rel=\"describedby\"", api::PATH)
        );
        assert_eq!(
            index[2],
            format!(
                "<{SITE_URL}{}>; rel=\"alternate\"; type=\"{MARKDOWN_TYPE}\"",
                songs::PATH
            )
        );

        // The front door names the MCP server card under `service-desc`, the API
        // catalog under `api-catalog` (RFC 9727 §3), and its own written
        // description under `describedby` — plus its Markdown form; the page that
        // is the same page under another URL names its Markdown form but does not
        // repeat the site-level links, because they describe the site and one
        // link on one URL is the whole promise.
        let home = links(HOME, false);
        assert_eq!(home.len(), 5);
        assert_eq!(home[0], song[0]);
        assert_eq!(
            home[1],
            format!("<{SITE_URL}{HOME}>; rel=\"alternate\"; type=\"{MARKDOWN_TYPE}\"")
        );
        assert_eq!(
            home[2],
            format!(
                "<{SITE_URL}{}>; rel=\"service-desc\"; type=\"{}\"",
                card::PATH,
                card::MEDIA_TYPE
            )
        );
        assert_eq!(home[2], card_link());
        assert_eq!(
            home[3],
            format!(
                "<{SITE_URL}{}>; rel=\"api-catalog\"; type=\"{}\"",
                catalog::PATH,
                catalog::MEDIA_TYPE
            )
        );
        assert_eq!(home[3], catalog_link());
        assert_eq!(
            home[4],
            format!(
                "<{SITE_URL}{}>; rel=\"describedby\"; type=\"text/plain\"",
                llms::PATH
            )
        );
        assert_eq!(home[4], describedby_link());

        let aepa = links(AEPA, false);
        assert_eq!(aepa.len(), 2);
        assert_eq!(
            aepa[1],
            format!("<{SITE_URL}{AEPA}>; rel=\"alternate\"; type=\"{MARKDOWN_TYPE}\"")
        );

        // A path with no Markdown form promises nothing but the sitemap.
        assert_eq!(links(crate::pages::editor::PATH, false).len(), 1);
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

    /// A list entry is one line, whatever the title is: the characters that
    /// would end the link early are escaped, and a newline in a title becomes a
    /// space rather than a second list item.
    #[test]
    fn a_title_cannot_break_its_list_entry() {
        assert_eq!(link_text("Te here"), "Te here");
        assert_eq!(link_text("  spaced   out  "), "spaced out");
        assert_eq!(link_text("a[b]c"), "a\\[b\\]c");
        assert_eq!(link_text("back\\slash"), "back\\\\slash");
        assert_eq!(link_text("two\nlines"), "two lines");
    }

    /// The token estimate is the convention's own ratio: one token per four
    /// UTF-8 bytes, rounded up, and never a bigger number than the document
    /// could hold as words.
    #[test]
    fn the_token_estimate_is_bytes_over_four_rounded_up() {
        assert_eq!(estimated_tokens(""), 0);
        assert_eq!(estimated_tokens("abc"), 1);
        assert_eq!(estimated_tokens("abcd"), 1);
        assert_eq!(estimated_tokens("abcde"), 2);

        // Multi-byte text is measured in bytes, which is what the reference
        // page's own example does (2,899 B → 725 tokens).
        assert_eq!(estimated_tokens("é"), 1);
        assert_eq!(estimated_tokens(&"é".repeat(4)), 2);
    }

    /// The list writer: one line per song, the title a link, the credits after
    /// it, and an absolute URL that survives being quoted out of context.
    #[test]
    fn the_song_list_links_every_title_absolutely() {
        let song = Song::new(
            "8nntgjk4rl5dbp67c6en".to_owned(),
            "Te here".to_owned(),
            "<div>Hina'a</div>".to_owned(),
            1,
            Vec::new(),
            true,
            chrono::Utc::now(),
            chrono::Utc::now(),
        );

        let listed = song_list(std::slice::from_ref(&song));
        assert_eq!(listed, format!("- [Te here]({})", song.get_url()));
        assert!(listed.contains(SITE_URL), "{listed}");
    }
}
