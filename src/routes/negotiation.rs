//! `Accept: text/markdown`, the `Link` headers that advertise it, and the one
//! spelling of a song URL.
//!
//! The site's prose pages have two representations and one URL each. Asked for
//! HTML — which is every browser, and the default — a page renders inside the
//! site's chrome. Asked for `text/markdown` it answers with the same page as one
//! Markdown document: for a song the title, the credits, the lyric with its
//! chords inline as `[Eb]`, and the canonical URL; for the front page and the
//! index the same prose and the same list of songs, every title a link, and the
//! index's pages one page at a time with their numbers named. That is the
//! difference between an agent reading the content and an agent scraping it out
//! of markup.
//!
//! **A song URL has one spelling, and this layer is what enforces it.** A song
//! is published at `/himene/{slug}`; `/himene/{id}` and a slug the song used to
//! have are answered `301` to the current address, in *either* representation —
//! so no cache, index or reader ever sees the same sheet at two URLs. The
//! redirect is issued here rather than in the page because a page is wrapped by
//! the layout: it would carry the site's whole chrome on a response whose only
//! content is where to go next, and because the Markdown form is served by this
//! layer too and would otherwise be the one spelling that did *not* move.
//!
//! **Which pages have a Markdown form** is one list, spelled in the private
//! `document` below: a song, the front page (its canonical `/faariiraa`, and
//! the root `/` that serves it), the index and its later pages, a selection on
//! the book, an artist's `/taata-himene/{id}`, and a *search* on `/paimi`. `links` is the same
//! list read the other way round — a page with two representations names the one
//! a given response is not — so a header promising a variant this layer does not
//! serve cannot be written. A *form* is on neither list: the create-song page,
//! the book's picker and `/paimi` with no needle are the same
//! kind of thing, and none of them has a second representation to promise. Nor is
//! a URL that names nothing: an artist id with no row, or a page of the index the
//! catalogue does not have, promises the sitemap and nothing else.
//!
//! **The language is the request's, and this layer now knows it.** The Markdown
//! form used to be written in the default language, because the language came
//! from a cookie the jar holds *inside* this layer; `routes::language` carries
//! it in the request context instead, and `i18n::resolve` reads it. So the
//! Markdown and HTML forms of one address are written in one language, which is
//! what they were always supposed to be.
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
//! * **No database read on the common path.** Every request that is not a song
//!   URL — the images, the API, the sitemaps, `robots.txt`, the two front-page
//!   URLs, the index — never opens the pool: the negotiation and the
//!   canonicalisation are both decided from the request's own headers and path
//!   first. A song URL is the exception, and it has to be: "is this the address
//!   the song is published at" is a fact about the row. It is one indexed lookup
//!   on a table of 43 rows, and the page reads the same row anyway.
//! * **A request that asks for Markdown on a path with no Markdown form — or
//!   names a song that is missing or a draft — falls through to the router**,
//!   which is what turns it into the site's 404. The layer does not invent an
//!   error response of its own.
//! * **The links are on the response, not on the representation.** They are
//!   queued through `response_headers`, so they survive whatever the response
//!   turns out to be, errors included. A redirect is the one exception: it
//!   returns before the queue is built, because a `Link` header describing a
//!   representation of the URL you are being sent away from is noise.

use topcoat::{
    Result,
    context::Cx,
    router::{
        Body, HeaderValue, Layer, LayerFuture, Next, Path, StatusCode, header,
        request::{headers, uri},
        response::{IntoResponse, Response, response_headers},
    },
};

use crate::db::{self, SongOrder};
use crate::domain::Song;
use crate::domain::chord;
use crate::domain::song::SITE_URL;
use crate::i18n::{self, Key, Lang};
use crate::pages::{
    artist,
    book,
    home,
    // The two front-page URLs. Their owner is `pages::home` — the page that
    // `#[page("/…")]` declares them in — because the layout matches the request
    // path against the same two constants to decide the document head.
    home::{PATH as HOME, ROOT},
    search,
    songs,
    support,
};
use crate::routes::{api, card, catalog, llms, sitemap};
use crate::state;

/// The media type this layer serves on request.
const MARKDOWN: &str = "text/markdown; charset=utf-8";

/// The same media type without its charset, for the places that compare or
/// name a type rather than set one: [`links`]' `alternate` values.
const MARKDOWN_TYPE: &str = "text/markdown";

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
            // The query, read once: a selection lives in it, so the Markdown form
            // and the `Link` headers both have to name the same one the reader
            // sent. `?tr=` is read out of it by the two functions that need it.
            let query = uri(cx).query().unwrap_or("");
            let wanted = header(cx, header::ACCEPT);
            // The language the request named in its URL, carried here by
            // `routes::language`; the default when it named none.
            let lang = i18n::resolve(cx);

            // The song this URL names, if it names one, resolved once for all
            // three decisions below: whether the URL has to move, whether the
            // Markdown form exists, and what the `Link` headers should promise.
            // Reading the row twice would be two answers to one question.
            let addressed = match song_segment(path) {
                Some(segment) => db::song_at(state::db(cx).pool(), segment).await?,
                None => None,
            };
            // A draft is not published, so it is not a page: it 404s, and
            // nothing is promised about it.
            let addressed = addressed.filter(|found| found.song().is_published());

            // The artist this URL names, if it names one — resolved once for the
            // same reason as the song above. The page handler reads the row too
            // and is what raises the 404, but the Markdown form and the `Link`
            // headers are decided *here*, before the handler runs, and a header
            // promising a Markdown twin for a URL that 404s is this module's
            // oldest lie.
            let artist = match artist::segment(path) {
                Some(id) => db::artist(state::db(cx).pool(), id).await?,
                None => None,
            };

            // The one URL a song is published at, decided before the
            // representation is chosen: an id URL and a retired slug are a 301
            // whether the client asked for HTML or for Markdown, so neither form
            // can be served — or cached, or indexed — under a second spelling.
            if let Some(found) = &addressed
                && !found.is_canonical()
            {
                // One URL per page: the id form moves to the slug's own
                // address, and the chrome follows the reader's language.
                let target = found.song().get_path();
                return Ok(moved_permanently(&target));
            }

            // The index is the one other URL with two spellings. Page 1 is
            // published at `/himene` and nowhere else, so `/himene/page/1` is the
            // id form of a song over again — and it gets the same answer, before
            // the representation is chosen, so neither spelling can be served,
            // cached or indexed.
            if songs::page_number(path) == Some(1) {
                return Ok(moved_permanently(songs::PATH));
            }

            // The page of the index this URL names, if it names one, resolved
            // once for the three decisions below — the Markdown form, the `Link`
            // headers, and the 404 a number the catalogue does not have is
            // answered with. One read of the catalogue's size, in one place,
            // because three answers to "does this page exist" would be three
            // chances to disagree.
            let page = page_of_index(cx, path).await?;

            if prefers_markdown(wanted.as_deref())
                && let Some(text) =
                    document(cx, lang, path, addressed.as_ref(), page, artist.as_ref()).await?
            {
                queue_links(
                    cx,
                    links(path, true, query, addressed.as_ref(), page, artist.as_ref()),
                )?;
                return markdown(cx, text);
            }

            let response = next.run(cx, body).await?;

            // Only a document gets a document's links. The API, the sitemaps,
            // `robots.txt` and the served stylesheet all pass through here too,
            // and none of them is described by them.
            if is_html(&response) {
                queue_links(
                    cx,
                    links(
                        path,
                        false,
                        query,
                        addressed.as_ref(),
                        page,
                        artist.as_ref(),
                    ),
                )?;
            }

            Ok(response)
        })
    }
}

/// `301` to `location`, with no body.
///
/// `pub(crate)` because `routes::language` issues the same redirect for the
/// addresses the site published before v4.2: one spelling of "this document
/// moved, for good" rather than two that can drift about the code, the body or
/// the absence of a `Cache-Control`.
///
/// **301, not 308.** Topcoat's own `redirect_permanent` is a 308, which is the
/// right code for a *method-preserving* move and the wrong one here: what is
/// being retired is one address of a `GET`-only document, and 301 is the code
/// every crawler has consolidated on since before 308 existed. The plan says
/// 301 for the same reason.
///
/// The target is root-relative — the site's own spelling for a redirect, and one
/// that does not bake in a host the request may not have arrived on. Slugs are
/// `[a-z0-9-]` by construction (`domain::slug`), so the header value cannot be
/// malformed and there is nothing to percent-encode.
pub(crate) fn moved_permanently(location: &str) -> Response {
    let mut response = Response::new(Body::empty());
    *response.status_mut() = StatusCode::MOVED_PERMANENTLY;
    response.headers_mut().insert(
        header::LOCATION,
        HeaderValue::from_str(location).expect("a slug is a valid header value"),
    );
    response
}

/// One request header, as a string, if it is there and readable.
fn header(cx: &Cx, name: header::HeaderName) -> Option<String> {
    headers(cx)
        .get(name)
        .and_then(|value| value.to_str().ok())
        .map(str::to_owned)
}

/// The song a path names — its slug, or an id or retired slug that has to move.
///
/// One segment below the prefix and nothing else. `/himene/{slug}/anything` is
/// nobody's page; `/himene/sitemap.xml` *is* one segment and is a route of its
/// own, and so are the create-song page and the multi-lyric page — the router
/// prefers a literal segment to a parameter, so each of them reaches its own
/// handler and never `#[page("/himene/{slug}")]`. They are excluded here rather than left for the
/// database read to reject, because the read is not the only caller: [`links`]
/// writes promises from this answer, and a page advertising a Markdown form it
/// does not have is a header lying about the response it arrived on.
///
/// `pub(crate)` because the layout asks the same question: it decides the
/// document head from the path, and a song URL that names no published song
/// becomes the 404's head. Two answers to "is this a song URL" would be a page
/// and its `<head>` disagreeing about which pages exist. For the same reason the
/// three literal paths are excluded here rather than left to the read: the
/// document head and the Markdown form are both decided from this answer, and a
/// create-song page or a selection advertised as a song would be a `<head>`
/// written for a page that does not exist.
pub(crate) fn song_segment(path: &str) -> Option<&str> {
    let segment = path.strip_prefix(SONG_PREFIX)?;
    let is_song = !segment.is_empty()
        && !segment.contains('/')
        && path != crate::pages::editor::PATH
        && path != sitemap::SONGS_PATH
        && path != book::PATH;

    is_song.then_some(segment)
}

/// The page of the index a path names, and how many pages the catalogue has.
///
/// `None` when the path is not the index at all. `Some((number, pages))` when it
/// is — including a number the catalogue does not have, which is `Some` on
/// purpose: "page 7 of 3" is a page URL, and what to do about it is the 404's
/// business, not this reader's. Page 1 is page 1 whether it arrived as `/himene`
/// or as `/himene/page/1`, the second of which is 301'd before anything reads
/// this.
///
/// One function and one read of the catalogue's size, because three answers are
/// built from it: the 301, the Markdown document, and the `Link` headers. Two of
/// them disagreeing about whether a page exists is a page whose Markdown twin
/// 404s while its HTML promises it.
async fn page_of_index(cx: &Cx, path: &str) -> Result<Option<(u32, u32)>> {
    if path != songs::PATH && songs::page_segment(path).is_none() {
        return Ok(None);
    }

    let pages = songs::page_count(db::counts(state::db(cx).pool()).await?.songs);
    // A segment that will not parse is `0` rather than an error: it cannot be a
    // page, and the handler's own `path_param!` is what turns it into the 404.
    let number = if path == songs::PATH {
        1
    } else {
        songs::page_number(path).unwrap_or(0)
    };

    Ok(Some((number, pages)))
}

/// The Markdown form of a path, if the path has one.
///
/// This is the list the module docs promise, in one place, and it is the same
/// list [`links`] reads when it decides whether a page has an `alternate`. A
/// path that is not in it answers `None` — the API, the two sitemaps,
/// `robots.txt`, `llms.txt`, the create-song form, the multi-lyric page's picker
/// and the served stylesheet are configured data and forms, not prose, and none
/// of them has a Markdown form to serve.
///
/// The reads happen here, in the branch that needs them, and only there: a
/// request for HTML — which is every browser — never reaches this function.
///
/// A song is resolved by [the handler](Negotiation::handle) and handed in, not
/// read again here: the segment in the path is a slug, and reading it as an id
/// would 404 the Markdown form of every song on the site.
async fn document(
    cx: &Cx,
    lang: Lang,
    path: &str,
    addressed: Option<&db::Addressed>,
    page: Option<(u32, u32)>,
    artist: Option<&crate::domain::Artist>,
) -> Result<Option<String>> {
    if song_segment(path).is_some() {
        // The page's own step, from the page's own URL. A song is the one
        // document here that has a second dimension, and the two forms of it
        // have to describe the same chords or the `Vary: Accept` between them
        // is a lie.
        let offset = chord::offset(uri(cx).query().unwrap_or(""));
        return Ok(addressed.map(|found| song_document(found.song(), offset)));
    }

    // The root is the front page under a second URL, and the same document:
    // everything but the canonical URL is identical, and the document names the
    // canonical host either way.
    if path == HOME || path == ROOT {
        return home_document(cx, lang).await.map(Some);
    }

    // The index and its later pages: `/himene` is page 1 and `/himene/page/{n}`
    // is page *n*. Which numbers exist is the caller's answer, read from the same
    // catalogue size the `Link` headers were built from — and a number the
    // catalogue does not have is not a document, so a request for Markdown on one
    // falls through to the router and becomes the same 404 the HTML gets.
    if let Some((number, pages)) = page {
        if !(1..=pages).contains(&number) {
            return Ok(None);
        }

        return index_document(cx, lang, number, pages).await.map(Some);
    }

    // The multi-lyric page. A *selection* is prose and has a document — every
    // chosen lyric, in the reading order the URL names it in. The picker is a
    // form and has none, the same rule the create-song page follows: the reader
    // of that page gets the HTML or nothing, and the `Link` headers say the same
    // (see [`links`]).
    //
    // A selection that does not resolve answers `None`, so a request for Markdown
    // on it falls through to the router and becomes the site's 404 — the page's
    // own answer, for the page's own reason.
    if path == book::PATH {
        let segments = book::selection(uri(cx).query().unwrap_or(""));
        if segments.is_empty() {
            return Ok(None);
        }
        let sheets = book::resolve(state::db(cx).pool(), &segments).await?;
        return Ok(sheets.map(|sheets| selection_document(&sheets, lang, &segments)));
    }

    // An artist's page. A prose page — the artist's name and their songs — so it
    // has a Markdown twin, from the same read the page renders. An id the caller
    // could not resolve answers `None`, and the request falls through to the
    // router and becomes the same 404 the HTML gets.
    if artist::segment(path).is_some() {
        let Some(row) = artist else {
            return Ok(None);
        };

        let listed = db::songs_by_artist(state::db(cx).pool(), &row.get_id()).await?;
        return Ok(Some(artist_document(row, &listed, lang)));
    }

    // The search page: a *search* is prose and has a document — the two halves of
    // what it found. The bare form has none, the picker's rule again.
    if path == search::PATH {
        let query = uri(cx).query().unwrap_or("");
        let Some(needle) = search::needle(query) else {
            return Ok(None);
        };

        let found = search::run(state::db(cx).pool(), &needle).await?;
        return Ok(Some(search_document(&needle, &found, lang, query)));
    }

    // The support page: the heading, the sentence under it, and the addresses.
    // Prose, so it has a Markdown twin; no read and no second state, because the
    // page is the same document in every language but its chrome.
    if path == support::PATH {
        return Ok(Some(support_document(lang)));
    }

    Ok(None)
}

/// The support page as one Markdown document — `/tauturu`.
///
/// The heading and the sentence under it are chrome and follow the language; the
/// chain names are proper nouns and the addresses are byte-for-byte the strings
/// the maintainer wrote. Each address is a fenced code block so a reader (or an
/// agent) can copy it without the markup around it, and the source line names the
/// page it came from.
fn support_document(lang: Lang) -> String {
    let mut out = format!(
        "# {}\n\n{}\n",
        i18n::text(lang, Key::SupportTitle),
        i18n::text(lang, Key::SupportIntro)
    );

    // The page's warm line, in the languages that have it — the same document as
    // the HTML, so a reader who asked for Markdown gets the same paragraph. The
    // Tahitian document has none, and that gap is the page's own decision, not a
    // formatting one.
    for line in support::money(lang) {
        out.push_str(&format!("\n{line}\n"));
    }

    for entry in support::ADDRESSES {
        out.push_str(&format!(
            "\n## {}\n\n```\n{}\n```\n",
            entry.label, entry.address
        ));
    }

    out.push_str(&format!("\nSource: {}\n", i18n::absolute(support::PATH)));

    out
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
/// [`Song::lyrics_markdown_at`] writes it — chords kept inline, at the step the
/// page is on and in the canonical spelling — and the canonical URL, so a
/// document quoted out of context still says where it came from.
///
/// `offset` is the page's `?tr=`, read by the caller. The Markdown form of a
/// transposed page is that page's chords, spelled the way the wheel spells them:
/// the solfège names are the French chrome's, and a machine reading this
/// document wants the chord the author's peers would write.
///
/// `pub(crate)` because the MCP server (`src/routes/mcp.rs`) answers `get_song`
/// with exactly this document: an agent reading a song over MCP and an agent
/// reading it with `Accept: text/markdown` must not get two different sheets.
/// There is no language to choose, either: a sheet is the title, the credits,
/// the lyric and its own address, none of which is chrome, and the lyrics are
/// never translated — so the document is the same for every reader, exactly as
/// the sheet's URL is.
pub(crate) fn song_document(sheet: &Song, offset: i32) -> String {
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
    out.push_str(&sheet.lyrics_markdown_at(offset));
    out.push_str(&format!(
        "\n\nSource: {}\n",
        i18n::absolute(&sheet.get_path())
    ));

    out
}

/// The chosen songs as one Markdown document — `/puta-himene?s=…`.
///
/// The pieces are [`song_document`]'s, once per song and in the reading order the
/// URL named: a `##` heading that is a link to the sheet that owns the song, the
/// credits on their own line in emphasis, and the whole lyric with its chords
/// inline at the author's own spelling — no `?tr=` here, because the page has no
/// transposition control and the document must describe the page it arrived
/// beside.
///
/// The heading is `##` rather than `#` because the document has a title of its
/// own: one `#` for the selection, one heading per song under it. A reader who
/// follows one of those links lands on the sheet, which has the same lyric and
/// its own canonical URL.
///
/// The source line names the *selection*, not the first song: this document is
/// about a URL, and a Markdown file quoted into a chat should say which URL it
/// came from.
fn selection_document(sheets: &[Song], lang: Lang, segments: &[String]) -> String {
    let mut out = format!("# {}\n\n", i18n::text(lang, Key::BookTitle));

    for sheet in sheets {
        let title = link_text(&sheet.get_title());
        out.push_str(&format!(
            "## [{title}]({})\n\n",
            i18n::absolute(&sheet.get_path())
        ));

        let artists = sheet
            .get_artists()
            .iter()
            .map(|artist| artist.get_fullname())
            .collect::<Vec<String>>()
            .join(", ");
        if !artists.is_empty() {
            out.push_str(&format!("_{artists}_\n\n"));
        }

        out.push_str(&format!("{}\n\n", sheet.lyrics_markdown()));
    }

    let url = i18n::absolute(&format!("{}?{}", book::PATH, book::query(segments)));
    out.push_str(&format!("Source: {url}\n"));

    out
}

/// An artist's page as one Markdown document — `/taata-himene/{id}`.
///
/// The name, then that artist's songs as the index writes them: one line each,
/// the title a link to its own sheet and the credits beside it. Read from the
/// same list the page renders, so the two cannot disagree about who the artist
/// is or what they wrote.
///
/// An artist with nothing published still has a document: the heading, the line
/// the index uses for an empty catalogue (the same fact, and one sentence rather
/// than two to translate), and the source URL.
fn artist_document(artist: &crate::domain::Artist, listed: &[Song], lang: Lang) -> String {
    let mut out = format!("# {}\n\n", link_text(&artist.get_fullname()));
    if listed.is_empty() {
        out.push_str(&format!("{}\n", i18n::text(lang, Key::IndexEmpty)));
    } else {
        out.push_str(&song_list(listed));
        out.push('\n');
    }

    out.push_str(&format!(
        "\nSource: {}\n",
        i18n::absolute(&artist::path_of(&artist.get_id()))
    ));

    out
}

/// A search as one Markdown document — `/paimi?q=…`.
///
/// The needle the document is about, then the two halves in the order the page
/// shows them: the songs as the index's own list, the artists as links to their
/// pages. An empty search says so in one line — the page's own sentence, because
/// it is the same fact.
///
/// The source line names the *search*, not the catalogue: this document is about
/// a URL, and a Markdown file quoted into a chat should say which URL it came
/// from.
fn search_document(needle: &str, found: &search::Results, lang: Lang, query: &str) -> String {
    let mut out = format!(
        "# {} : {}\n\n",
        i18n::text(lang, Key::SearchTitle),
        link_text(needle)
    );

    if found.songs.is_empty() && found.artists.is_empty() {
        out.push_str(&format!("{}\n", i18n::text(lang, Key::SearchEmpty)));
    } else {
        if !found.songs.is_empty() {
            out.push_str(&format!("## {}\n\n", i18n::text(lang, Key::SearchSongs)));
            out.push_str(&song_list(&found.songs));
            out.push('\n');
        }
        if !found.artists.is_empty() {
            out.push_str(&format!(
                "\n## {}\n\n",
                i18n::text(lang, Key::SearchArtists)
            ));
            for artist in &found.artists {
                out.push_str(&format!(
                    "- [{}]({})\n",
                    link_text(&artist.get_fullname()),
                    i18n::absolute(&artist::path_of(&artist.get_id()))
                ));
            }
        }
    }

    let source = match search::query_string(query) {
        Some(chosen) => format!("{}?{chosen}", search::PATH),
        None => search::PATH.to_owned(),
    };
    out.push_str(&format!("\nSource: {}\n", i18n::absolute(&source)));

    out
}

/// The front page as one Markdown document — `/faariiraa`, and the root `/`
/// that serves the same page.
///
/// The same content the HTML page carries, in the same order: the hero, the
/// three cards, the synopsis, the two tables and the closing block. The prose is
/// [`home::copy`]'s, in the language this response is written in, so the two
/// forms of the page cannot disagree about a word; the two button labels are
/// chrome and come from [`crate::i18n`] like the page's.
///
/// **The tables list titles, not lyric lines.** The HTML's newest table leads
/// with the lyric because it can truncate it to one line with a utility class;
/// in Markdown there is nothing to truncate with, and five inlined lyrics would
/// make this an anthology rather than a front page. Each title is a link, and
/// every one of those links has its own Markdown form with the whole lyric in
/// it — which is where a reader who wants the words is going anyway.
async fn home_document(cx: &Cx, lang: Lang) -> Result<String> {
    let pool = state::db(cx).pool();
    let latest = db::songs(pool, SongOrder::Newest, Some(home::ROWS)).await?;
    let most_viewed = db::songs(pool, SongOrder::MostViewed, Some(home::ROWS)).await?;

    let mut out = format!("# {}\n\n", home::copy::HERO_TITLE);
    out.push_str(&format!("_{}_\n\n", home::copy::hero_subtitle(lang)));
    out.push_str(&format!(
        "[{}]({})\n\n",
        i18n::text(lang, Key::HomeDiscover),
        i18n::absolute(songs::PATH)
    ));

    for (title, text) in home::copy::cards(lang) {
        out.push_str(&format!("## {title}\n\n{text}\n\n"));
    }

    out.push_str(&format!("{}\n\n", home::copy::synopsis(lang)));
    out.push_str(&format!("## {}\n\n", home::copy::table_latest(lang)));
    out.push_str(&song_list(&latest));
    out.push_str(&format!(
        "\n\n## {}\n\n",
        home::copy::table_most_viewed(lang)
    ));
    out.push_str(&song_list(&most_viewed));
    out.push_str(&format!("\n\n## {}\n\n", home::copy::foot_title(lang)));
    out.push_str(&format!("{}\n\n", home::copy::foot_text(lang)));
    // The two closing buttons. The first is a brand name and is not translated;
    // the second is the create-song page's label. The address under the brand is
    // the row's own constant (`ui::share::PAGE`), so the Markdown twin and the
    // button cannot come to point at two different pages.
    out.push_str(&format!(
        "[{}]({}) · [{}]({})\n",
        home::copy::FOOT_FACEBOOK,
        crate::ui::share::PAGE,
        i18n::text(lang, Key::HomeStart),
        i18n::absolute(crate::pages::editor::PATH)
    ));
    out.push_str(&format!("\nSource: {}\n", i18n::absolute(home::PATH)));

    Ok(out)
}

/// The song index as one Markdown document — `/himene`, and `/himene/page/{n}`.
///
/// One **page** of the catalogue, newest first, in the order the HTML table shows
/// them and from the same window: `/himene` is page 1, so a document that listed
/// the whole catalogue under the index's own URL would no longer be the twin of
/// the page it represents, and the `Vary: Accept` between them would be a lie.
///
/// The heading is chrome, the line an empty catalogue gets is chrome, and so is
/// the `Pages:` line — which is the same navigation the HTML nav is, because a
/// reader with no HTML still has to be able to reach page 3. The page the
/// document *is* is left unlinked there, the same rule the nav follows.
async fn index_document(cx: &Cx, lang: Lang, number: u32, pages: u32) -> Result<String> {
    let listed = db::songs_page(
        state::db(cx).pool(),
        SongOrder::Newest,
        songs::PAGE_SIZE,
        songs::offset(number),
    )
    .await?;

    let mut out = format!("# {}\n\n", i18n::text(lang, Key::IndexTitle));
    if listed.is_empty() {
        out.push_str(&format!("{}\n", i18n::text(lang, Key::IndexEmpty)));
    } else {
        out.push_str(&song_list(&listed));
        out.push('\n');
    }

    if pages > 1 {
        out.push_str(&format!("\nPages: {}\n", page_line(number, pages)));
    }
    out.push_str(&format!(
        "\nSource: {}\n",
        i18n::absolute(&songs::page_path(number))
    ));

    Ok(out)
}

/// The index's page numbers, as one Markdown line.
///
/// Numbers rather than words, like the nav above them: a page number needs no
/// translation, and the page the document *is* is emphasised rather than linked,
/// for the reason the nav marks it instead of linking it.
fn page_line(number: u32, pages: u32) -> String {
    (1..=pages)
        .map(|n| {
            if n == number {
                format!("**{n}**")
            } else {
                format!("[{}]({})", n, i18n::absolute(&songs::page_path(n)))
            }
        })
        .collect::<Vec<String>>()
        .join(" · ")
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
            let url = i18n::absolute(&song.get_path());

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

/// Queue a path's `Link` values on the response being built.
///
/// Separate `Link` headers rather than one comma-joined value: RFC 8288 permits
/// both, and separate ones keep the quoting inside each value simple.
///
/// Takes [`links`]' answer rather than its arguments, so the signature stays a
/// reader's and the *why* of each value stays in one function.
fn queue_links(cx: &Cx, values: Vec<String>) -> Result<()> {
    for value in values {
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
/// `addressed` is the song the path names, already resolved by the caller — a
/// `Link` header is a promise about a row, and this function has no business
/// reading one of its own. `page` is the same thing for a page of the index: the
/// number the URL names and how many the catalogue has, read once by the caller
/// for the same reason, because a page past the end is answered by the 404 and
/// the 404 promises only the sitemap.
///
/// `lang` is not a parameter any more, and its absence is the point: a page has
/// one URL since v4.2, so the address a `Link` header promises is the page's own
/// canonical URL and nothing else — there is no second spelling for a header to
/// point at, and no language to put in front of one.
///
/// * Every document names the sitemap.
/// * Every page with a Markdown form names the other of its two representations
///   under `alternate` — a song, the front page, the root that duplicates it, and the
///   index. That is the same list [`document`] serves, and it is derived from
///   `served_markdown` rather than from a second table.
/// * A song names its JSON read under `describedby`. That URL is keyed by the
///   song's **id**, which is the API's stable key: the page it arrived on is
///   named by a slug, and the two are not interchangeable.
/// * The song index — and every page of it — names the catalogue, which is the
///   machine-readable form of the list those pages are windows on.
/// * A **selection** on the multi-lyric page names its own JSON read under
///   `describedby` and its Markdown form under `alternate`, and both carry the
///   query string: the promise is about *this* selection, and a link built from
///   the path alone would name the picker — a different document, and one with
///   neither of those two forms.
/// * A **search** on `/paimi` does the same: its JSON read is
///   `/api/search?q=…` (the same needle, the same two reads) and its Markdown
///   twin carries the reader's own query string. The bare page is the form and
///   promises the sitemap alone.
/// * An **artist's page** names its Markdown twin. It has no JSON read of its
///   own: there is no `/api/artists/{id}`, and the machine-readable resource for
///   an artist is the page, whose structured data says who it is.
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
fn links(
    path: &str,
    served_markdown: bool,
    query: &str,
    addressed: Option<&db::Addressed>,
    page: Option<(u32, u32)>,
    artist: Option<&crate::domain::Artist>,
) -> Vec<String> {
    let sitemap_link = format!("<{SITE_URL}{}>; rel=\"sitemap\"", sitemap::PATH);

    // The representation a caller did not get. `alternate` is the registered
    // relation for another form of the same resource, and the type is what says
    // which one.
    let other = if served_markdown { HTML } else { MARKDOWN_TYPE };
    let alternate = |url: String| format!("<{url}>; rel=\"alternate\"; type=\"{other}\"");

    // An artist's page — `/taata-himene/{id}`. `artist` is the row the caller already
    // resolved: `None` means the URL names nobody, so the page handler raises the
    // branded 404 and this promises it nothing but the sitemap, the same rule a
    // song URL that names no published song follows.
    if artist::segment(path).is_some() {
        return match artist {
            Some(row) => vec![
                sitemap_link,
                alternate(i18n::absolute(&artist::path_of(&row.get_id()))),
            ],
            None => vec![sitemap_link],
        };
    }

    // The search page has two states, like the multi-lyric page's. A *search* has
    // both of its other forms — the same results as Markdown, and the same rows
    // as JSON at `/api/search` — and both carry the reader's own query string,
    // because the promise is about *this* search and a link built from the path
    // alone would name the bare form, which has neither. The bare form promises
    // the sitemap and nothing else: a form is not a document.
    if path == search::PATH {
        return match search::query_string(query) {
            Some(chosen) => vec![
                sitemap_link,
                format!(
                    "<{SITE_URL}{}?{chosen}>; rel=\"describedby\"",
                    api::SEARCH_PATH
                ),
                alternate(i18n::absolute(&format!("{}?{chosen}", search::PATH))),
            ],
            None => vec![sitemap_link],
        };
    }

    // A song that is there — a draft or a missing one has no links, the same rule
    // the other pages follow: nothing is described that is not served. Both URLs
    // come from the row: the JSON read is keyed by id (its stable key), and the
    // canonical form of the page is the slug, which the path alone cannot be
    // asked for — the segment in it may be an id on its way to a 301.
    if let Some(found) = addressed {
        let sheet = found.song();
        return vec![
            sitemap_link,
            format!(
                "<{SITE_URL}{}/{}>; rel=\"describedby\"",
                api::PATH,
                sheet.get_id()
            ),
            alternate(i18n::absolute(&sheet.get_path())),
        ];
    }

    // The multi-lyric page has two states, and the headers differ between them.
    // A selection has both of the other forms; the picker has neither, and
    // promises only the sitemap — the same single link the create-song page
    // gets, for the same reason: a form is not a document.
    if path == book::PATH {
        let segments = book::selection(query);
        if segments.is_empty() {
            return vec![sitemap_link];
        }

        let chosen = book::query(&segments);
        return vec![
            sitemap_link,
            format!("<{SITE_URL}{}?{chosen}>; rel=\"describedby\"", api::PATH),
            alternate(i18n::absolute(&format!("{}?{chosen}", book::PATH))),
        ];
    }

    // A page of the index — `/himene` is page 1, `/himene/page/{n}` is page *n*.
    // Each names the catalogue under `describedby` and its own Markdown twin
    // under `alternate`. The JSON read is the whole catalogue rather than one
    // page of it: the API takes no page parameter, and the catalogue is the
    // machine-readable form of the list a page holds a window on — a promise it
    // does not keep would be worse than no promise at all.
    //
    // A page the catalogue does not have is the 404 the page handler raises, and
    // the 404 is described by nothing but the sitemap: an `alternate` for an
    // address that serves no document is this module's oldest lie.
    if let Some((number, pages)) = page {
        if !(1..=pages).contains(&number) {
            return vec![sitemap_link];
        }

        return vec![
            sitemap_link,
            format!("<{SITE_URL}{}>; rel=\"describedby\"", api::PATH),
            alternate(i18n::absolute(&songs::page_path(number))),
        ];
    }

    // The support page. A document, so it names both of its other forms: the
    // Markdown twin under `alternate`, and the addresses as JSON under
    // `describedby`. It is the same document in every language — only its
    // chrome changes — so the alternate is its own URL in the language it was
    // served in.
    if path == support::PATH {
        return vec![
            sitemap_link,
            format!("<{SITE_URL}{}>; rel=\"describedby\"", api::SUPPORT_PATH),
            alternate(i18n::absolute(support::PATH)),
        ];
    }

    match path {
        // The root is the front page under a second URL, so it has the same two
        // representations; it does not repeat the site-level links, which
        // describe the site and are promised once, on its canonical address.
        ROOT => vec![sitemap_link, alternate(i18n::absolute(ROOT))],
        HOME => vec![
            sitemap_link,
            alternate(i18n::absolute(HOME)),
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
        // A slug and an id are both one segment: what the segment *is* is the
        // resolver's question, not this reader's.
        assert_eq!(song_segment("/himene/ahani-e"), Some("ahani-e"));
        assert_eq!(
            song_segment("/himene/8nntgjk4rl5dbp67c6en"),
            Some("8nntgjk4rl5dbp67c6en")
        );
        assert_eq!(song_segment("/himene/sitemap.xml"), None);
        assert_eq!(song_segment(crate::pages::editor::PATH), None);
        assert_eq!(song_segment("/himene/"), None);
        assert_eq!(song_segment("/himene"), None);
        assert_eq!(song_segment("/himene/a/b"), None);
        assert_eq!(song_segment("/api/songs/x"), None);
        assert_eq!(song_segment("/"), None);
    }

    /// A page of the index joins the same triple the index does, at its own
    /// URLs: the HTML names its Markdown twin, that twin names the HTML back, and
    /// both name the catalogue. A page the catalogue does not have promises only
    /// the sitemap — the response is the 404, and an `alternate` for an address
    /// that serves no document is the lie this module exists to avoid.
    #[test]
    fn every_page_of_the_index_names_its_own_markdown_and_its_own_json() {
        let second = links("/himene/page/2", false, "", None, Some((2, 3)), None);

        assert_eq!(second.len(), 3);
        assert_eq!(
            second[1],
            format!("<{SITE_URL}{}>; rel=\"describedby\"", api::PATH)
        );
        assert_eq!(
            second[2],
            format!(
                "<{}>; rel=\"alternate\"; type=\"{MARKDOWN_TYPE}\"",
                i18n::absolute("/himene/page/2")
            )
        );

        // The same page served as Markdown names the HTML form back — and the
        // two forms name one URL, not two.
        let as_markdown = links("/himene/page/2", true, "", None, Some((2, 3)), None);
        assert!(as_markdown[2].ends_with("rel=\"alternate\"; type=\"text/html\""));
        assert!(as_markdown[2].contains(i18n::absolute("/himene/page/2").as_str()));

        // The index itself is page 1 of the same series: same three links, at
        // the index's own URL.
        let first = links(songs::PATH, false, "", None, Some((1, 3)), None);
        assert_eq!(first[0], second[0]);
        assert_eq!(first[1], second[1]);
        assert!(first[2].contains(i18n::absolute(songs::PATH).as_str()));

        // Out of range, and a segment that is not a number: the 404's answer.
        for page in [Some((9, 3)), Some((0, 3)), None] {
            assert_eq!(
                links("/himene/page/9", false, "", None, page, None).len(),
                1
            );
        }
    }

    /// Every document names the sitemap; a song names its JSON read and its
    /// other representation; the index names the catalogue; nothing else is
    /// described by anything.
    ///
    /// Every value is absolute, because a header outlives the response it
    /// arrived on.
    #[test]
    fn the_links_say_what_the_page_is_and_where_its_other_forms_are() {
        // A published song, resolved: its URL is the slug and its JSON read is
        // keyed by the id, which is the distinction these three headers have to
        // keep — the page is named one way and the API another.
        let sheet = Song::new(
            "8nntgjk4rl5dbp67c6en".to_owned(),
            Some("te-here".to_owned()),
            "Te here".to_owned(),
            "<div>Hina'a</div>".to_owned(),
            1,
            Vec::new(),
            true,
            chrono::Utc::now(),
            chrono::Utc::now(),
        );
        let addressed = db::Addressed::new(sheet, true);
        let song = links("/himene/te-here", false, "", Some(&addressed), None, None);
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
                "<{}>; rel=\"alternate\"; type=\"text/markdown\"",
                i18n::absolute("/himene/te-here")
            )
        );

        // Serving Markdown flips the alternate to the HTML document, because the
        // other representation is now the one the caller did not get.
        let as_markdown = links("/himene/te-here", true, "", Some(&addressed), None, None);
        assert!(as_markdown[2].ends_with("rel=\"alternate\"; type=\"text/html\""));
        assert_eq!(as_markdown[0], song[0]);

        // A song URL that names no published song promises nothing: a draft is
        // not served, so there is nothing to describe.
        assert_eq!(
            links("/himene/te-here", false, "", None, None, None).len(),
            1
        );

        // The index names the catalogue and its own Markdown form.
        let index = links(songs::PATH, false, "", None, Some((1, 3)), None);
        assert_eq!(index.len(), 3);
        assert_eq!(index[0], song[0]);
        assert_eq!(
            index[1],
            format!("<{SITE_URL}{}>; rel=\"describedby\"", api::PATH)
        );
        assert_eq!(
            index[2],
            format!(
                "<{}>; rel=\"alternate\"; type=\"{MARKDOWN_TYPE}\"",
                i18n::absolute(songs::PATH)
            )
        );

        // The front door names the MCP server card under `service-desc`, the API
        // catalog under `api-catalog` (RFC 9727 §3), and its own written
        // description under `describedby` — plus its Markdown form; the root,
        // which is the same page under another URL, names its Markdown form but
        // does not repeat the site-level links, because they describe the site
        // and one link on the page's own canonical URL is the whole promise.
        let home = links(HOME, false, "", None, None, None);
        assert_eq!(home.len(), 5);
        assert_eq!(home[0], song[0]);
        assert_eq!(
            home[1],
            format!(
                "<{}>; rel=\"alternate\"; type=\"{MARKDOWN_TYPE}\"",
                i18n::absolute(HOME)
            )
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

        let root = links(ROOT, false, "", None, None, None);
        assert_eq!(root.len(), 2);
        assert_eq!(
            root[1],
            format!(
                "<{}>; rel=\"alternate\"; type=\"{MARKDOWN_TYPE}\"",
                i18n::absolute(ROOT)
            )
        );

        // A path with no Markdown form promises nothing but the sitemap.
        assert_eq!(
            links(crate::pages::editor::PATH, false, "", None, None, None).len(),
            1
        );
    }

    /// The multi-lyric page's two states, read out of the headers: a selection
    /// promises its JSON read and its Markdown form, **both carrying the
    /// query** — the promise is about *this* selection, and a link built from
    /// the path alone would name the picker, which is a different document with
    /// neither of those forms. The picker promises nothing but the sitemap, the
    /// same single `Link` the create-song page gets, because a form is not a
    /// document.
    #[test]
    fn a_selection_names_its_own_json_and_markdown_forms() {
        let chosen = links(book::PATH, false, "s=a&s=b", None, None, None);
        assert_eq!(chosen.len(), 3);
        assert_eq!(
            chosen[0],
            format!("<{SITE_URL}{}>; rel=\"sitemap\"", sitemap::PATH)
        );
        assert_eq!(
            chosen[1],
            format!("<{SITE_URL}{}?s=a&s=b>; rel=\"describedby\"", api::PATH)
        );
        assert_eq!(
            chosen[2],
            format!(
                "<{}>; rel=\"alternate\"; type=\"{MARKDOWN_TYPE}\"",
                i18n::absolute("/puta-himene?s=a&s=b")
            )
        );

        // Serving Markdown flips the alternate to the HTML document and keeps
        // the query — one URL per page, so there is no language in it.
        let as_markdown = links(book::PATH, true, "s=a&s=b", None, None, None);
        assert!(as_markdown[2].contains("/puta-himene?s=a&s=b"));
        assert!(as_markdown[2].ends_with("rel=\"alternate\"; type=\"text/html\""));

        // The picker: a path with no selection, and a query that is not one.
        for query in ["", "s=", "tr=3", "page=2"] {
            assert_eq!(
                links(book::PATH, false, query, None, None, None).len(),
                1,
                "{query:?}"
            );
        }
    }

    /// The Markdown form of a selection: a title, one heading per song with its
    /// title a link to the sheet, the credits, the lyric with its chords inline
    /// in the canonical spelling, and a source line that names the *selection*
    /// rather than the first song.
    #[test]
    fn the_selection_document_holds_every_chosen_lyric_in_order() {
        let sheet = |id: &str, slug: &str, title: &str, lyrics: &str| {
            Song::new(
                id.to_owned(),
                Some(slug.to_owned()),
                title.to_owned(),
                lyrics.to_owned(),
                1,
                Vec::new(),
                true,
                chrono::Utc::now(),
                chrono::Utc::now(),
            )
        };

        let first = sheet(
            "8nntgjk4rl5dbp67c6en",
            "te-here",
            "Te here",
            "<div>Hina'a<sup>Eb</sup>ro</div>",
        );
        let second = sheet(
            "4cfl27ia9hndgetgr1o7",
            "ahani-e",
            "Ahani e",
            "<div>Ua noa</div>",
        );
        let segments = vec!["te-here".to_owned(), "ahani-e".to_owned()];
        let text = selection_document(&[first, second], Lang::Fr, &segments);

        assert!(text.starts_with("# Plusieurs chansons\n\n"), "{text}");
        // The order is the URL's, not the catalogue's.
        let first_at = text.find("## [Te here](").expect("the first song");
        let second_at = text.find("## [Ahani e](").expect("the second song");
        assert!(first_at < second_at, "{text}");
        assert!(text.contains("Hina'a[Eb]ro"), "{text}");
        assert!(
            text.contains(&format!(
                "## [Te here]({})",
                i18n::absolute("/himene/te-here")
            )),
            "{text}"
        );
        assert!(text.ends_with(&format!(
            "Source: {}\n",
            i18n::absolute("/puta-himene?s=te-here&s=ahani-e")
        )));

        // Chrome, so it follows the language; the lyrics do not.
        let in_english = selection_document(
            &[sheet(
                "8nntgjk4rl5dbp67c6en",
                "te-here",
                "Te here",
                "<div>Hina'a<sup>Eb</sup>ro</div>",
            )],
            Lang::En,
            &["te-here".to_owned()],
        );
        assert!(
            in_english.starts_with("# Several songs\n\n"),
            "{in_english}"
        );
        assert!(in_english.contains("Hina'a[Eb]ro"), "{in_english}");
    }

    /// The document is Markdown a reader can use: an `# ` title, then the lyric
    /// with its chords, then the source.
    #[test]
    fn the_document_opens_with_the_title_and_closes_with_its_url() {
        let sheet = Song::new(
            "8nntgjk4rl5dbp67c6en".to_owned(),
            Some("te-here".to_owned()),
            "Te here".to_owned(),
            "<div>Hina'a<sup data-nosnippet=\"true\">Eb</sup>ro</div>".to_owned(),
            1,
            Vec::new(),
            true,
            chrono::Utc::now(),
            chrono::Utc::now(),
        );

        let text = song_document(&sheet, 0);
        let mut lines = text.lines();

        assert_eq!(lines.next(), Some("# Te here"));
        assert!(text.contains("Hina'a[Eb]ro"), "{text}");
        assert!(text.ends_with(&format!("Source: {}\n", i18n::absolute(&sheet.get_path()))));
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
            Some("te-here".to_owned()),
            "Te here".to_owned(),
            "<div>Hina'a</div>".to_owned(),
            1,
            Vec::new(),
            true,
            chrono::Utc::now(),
            chrono::Utc::now(),
        );

        let listed = song_list(std::slice::from_ref(&song));
        assert_eq!(
            listed,
            format!("- [Te here]({})", i18n::absolute(&song.get_path()))
        );
        assert!(listed.contains(SITE_URL), "{listed}");
    }

    /// The support page is a document like the others: three links — the
    /// sitemap, the JSON read of its addresses, and its Markdown twin — and a
    /// twin whose chrome follows the language while its addresses do not.
    #[test]
    fn the_support_page_names_its_two_other_forms() {
        let html = links(support::PATH, false, "", None, None, None);
        assert_eq!(html.len(), 3);
        assert_eq!(
            html[0],
            format!("<{SITE_URL}{}>; rel=\"sitemap\"", sitemap::PATH)
        );
        assert_eq!(
            html[1],
            format!("<{SITE_URL}{}>; rel=\"describedby\"", api::SUPPORT_PATH)
        );
        assert_eq!(
            html[2],
            format!(
                "<{}>; rel=\"alternate\"; type=\"{MARKDOWN_TYPE}\"",
                i18n::absolute(support::PATH)
            )
        );

        // Serving Markdown names the HTML document back: one address, and the
        // response's language is not part of it.
        let as_markdown = links(support::PATH, true, "", None, None, None);
        assert!(as_markdown[2].contains("/tauturu"), "{}", as_markdown[2]);
        assert!(as_markdown[2].ends_with("rel=\"alternate\"; type=\"text/html\""));

        // The document: the heading is chrome, the addresses are content.
        let french = support_document(Lang::Fr);
        assert!(french.starts_with("# Soutenir le site\n"), "{french}");
        assert!(french.ends_with(&format!("Source: {}\n", i18n::absolute(support::PATH))));
        // The money line is in the two languages that have it, where the HTML
        // page has it: under the standfirst and above the first address.
        assert!(
            french.contains("Votre soutien paie l'hébergement"),
            "{french}"
        );
        assert!(
            french.find("Votre soutien").unwrap() < french.find("## Bitcoin").unwrap(),
            "the money line is not above the addresses"
        );
        for entry in support::ADDRESSES {
            assert!(french.contains(entry.address), "{}", entry.address);
            assert!(french.contains(entry.label), "{}", entry.label);
        }

        let english = support_document(Lang::En);
        assert!(english.starts_with("# Support the site\n"), "{english}");
        assert!(
            english.contains("Your support pays for the hosting"),
            "{english}"
        );
        for entry in support::ADDRESSES {
            assert!(
                english.contains(entry.address),
                "an address was translated: {}",
                entry.address
            );
        }

        // The Tahitian document carries the Tahitian chrome and no money line:
        // there are no faithful words for it, and the page would rather miss one
        // than invent one.
        let tahitian = support_document(Lang::Ty);
        assert!(tahitian.starts_with("# Tauturu i te 'api\n"), "{tahitian}");
        assert!(
            !tahitian.contains("café") && !tahitian.contains("coffee"),
            "{tahitian}"
        );
    }
}
