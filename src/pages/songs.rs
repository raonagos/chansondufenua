//! The song index — `/himene`, and its later pages, `/himene/page/{n}`.
//!
//! Transcribed from v3's `AllSongPage` (`app/src/pages/himene/allsong.rs`): one
//! heading, one table, one row per song, the artist column dropped on a phone.
//!
//! Two things differ from v3, both deliberate and both following step 5:
//!
//! * **The rows are served, not streamed.** v3 asked for the list from the
//!   browser — a `Resource` inside `<Suspense>` — so the first byte carried
//!   `<td>Chargement...</td>` and the songs arrived afterwards, from a second
//!   request. A crawler, an agent, or a reader with JavaScript off saw an index
//!   with no songs in it. The live page still does this: its rows are assembled
//!   after `</body>` in the wire format. Here the query runs before the page
//!   renders.
//! * **The rows are links, not scripts.** v3 put `onclick="window.location=…"`
//!   on every `<tr>`, with `role="button"` and `tabindex="0"` to match, *and*
//!   correct `<a href>` elements inside. v4 keeps the anchors and drops the
//!   rest. Gone with them is `aria-label="Go to the song …"` — an English
//!   sentence on a French page, describing a role that no longer exists.
//!
//! The surface is the v4 panel; the page title is the v4 [`theme::H1`] token
//! rather than v3's smaller heading, because that is what the v4 typography
//! direction is for.
//!
//! **The index is paginated** (v4.1 step 27): `/himene` is page 1 and
//! `/himene/page/{n}` is page *n*, both server-rendered, with one link per page
//! under the table. Each page is self-canonical, names its own number in its
//! title, and a number the catalogue does not have is the branded 404 rather
//! than a clamp — see [`PAGE_SIZE`] for how many songs a page holds and why.
//!
//! Every page of the index has a second representation: `routes/negotiation.rs`
//! answers `Accept: text/markdown` with the same page as one Markdown document,
//! from the same read.
//!
//! (Note the phrasing: v3's heading size is *described*, not spelled. Tailwind
//! scans the crate's source text for class names, and a utility named in a doc
//! comment is emitted into the stylesheet whether or not anything uses it. The
//! same trap makes a class named in prose here a class the served CSS carries.)
//!
//! **Local bindings here cannot be called `songs` or `page`.** `#[page("/himene")]`
//! and `#[page("/himene/page/{page}")]` each emit a unit struct named after its
//! handler, in this module, in the *type* namespace. `let songs = …` is then read
//! as a pattern matching that unit struct rather than as a new binding, and the
//! compiler reports a type error several lines away from the cause. The page's
//! rows live in `listed`; its own number lives in `number`.

use topcoat::{
    Result,
    context::Cx,
    router::{error::RouterErrorExt, href, page, path_param},
    view::{View, class, component, view},
};

use crate::db::{self, SongOrder};
use crate::domain::Song;
use crate::i18n::{self, Key};
use crate::pages::{artist, book, song as sheet};
use crate::state;
use crate::ui::theme;

// The `{page}` in the paginated route's path.
//
// Typed, so the parse happens once, in the framework, and a segment that is not
// a `u32` is a not-found response rather than a panic or a zero: `/himene/page/x`
// and `/himene/page/99999999999999` are URLs this site does not have, and the
// branded 404 is the honest answer to both. Named `page` so the type is `Page`,
// which is what the paginated handler below reads through `path_param::<Page>`.
path_param!(pub page: u32, error = not_found);

/// `/himene` — page 1 of the index, and the address the first page is published
/// at.
///
/// **Page 1 is `/himene`**, on his own wording in `.run/v4.1-scope.md`: the page
/// segment exists for the pages after it. `/himene/page/1` is therefore the same
/// kind of address as a song's id form — one page under a second URL — and
/// `routes/negotiation.rs` answers it with a `301` to this path, for HTML and
/// for Markdown alike. (That is also why the page handler below serves 2..=last
/// and not 1.)
///
/// The path is a constant as well as an attribute, because three other modules
/// name it: `robots.txt`'s `Link` headers and the fixed-pages sitemap
/// (`routes/negotiation.rs`, `routes/sitemap.rs`), the document head and the
/// header's current-page test (`ui/layout.rs`). `#[page]` cannot take a
/// constant — it is a macro over a literal path — so the constant restates it,
/// as `pages::editor::PATH` does.
pub const PATH: &str = "/himene";

/// The paginated shape: `/himene/page/{n}`, and the prefix every reader of it
/// strips.
///
/// The second constant restating a path this file declares, for the same reason
/// [`PATH`] gives: `routes/language.rs` decides whether a request path is a page
/// at all, `routes/negotiation.rs` decides whether it has a Markdown form and
/// what its `Link` headers promise, and `ui/layout.rs` decides its `<head>` —
/// and all three need to know the shape, not the number inside it.
pub const PAGE_PREFIX: &str = "/himene/page/";

/// How many songs one page of the index holds.
///
/// **Twenty, which is the number the MCP catalogue already used.**
/// `routes/mcp.rs`'s `list_songs` defaults to it, and its own doc comment
/// promises that a client paging through MCP and a reader paging through
/// `/himene` see the same thing — a promise that only holds while both agree
/// about the window, so MCP imports *this* constant rather than repeating the
/// number. Forty-three songs make three pages, 20 + 20 + 3.
///
/// **v3 asked for `page 0, limit 255`** — a pagination call with no pagination
/// behind it. `app/src/app.rs` registered `himene/""` and `himene/:id` and never
/// a page segment, so `use_params_map().get("page")` in that page could only
/// ever read `0`: the 255th song would have been invisible with nothing to page
/// to, and the page that promised every song had a silent ceiling instead. The
/// older comment here refused to make that promise until the page segment
/// arrived. It has arrived, with the pagination it implies, which is what keeps
/// the read below honest rather than merely larger.
pub const PAGE_SIZE: i64 = 20;

/// The index's `<meta name="description">`.
///
/// The page's own heading, then what the page holds. Same budget as a song's:
/// [`DESCRIPTION_MAX`](crate::domain::song::DESCRIPTION_MAX) is what a search
/// engine shows.
///
/// French, like the rest of this page's prose: the lyrics are never translated
/// and neither is the catalogue's own copy. The `<title>` above does follow the
/// request's language, because a title is chrome. Pages after the first append
/// their own number to this sentence — see `ui/layout.rs`'s `index_head`.
pub const DESCRIPTION: &str =
    "Toutes les chansons du fenua : paroles et accords des chansons tahitiennes et polynésiennes.";

/// The page segment of a `/himene/page/{n}` path, if the path has that shape.
///
/// *Shape*, not number: the segment is returned as it arrived, so a caller can
/// tell "a page of the index, and the number in it is rubbish" from "not a page
/// of the index at all". That distinction is what keeps `/himene/page/x` the 404
/// of the page's own route — with the index's own chrome — rather than a path
/// nothing claims.
///
/// `/himene/page` and `/himene/page/` are *not* this shape: the first is a
/// one-segment path and therefore a song URL as far as `song_segment` is
/// concerned (a song called `page`, which no song is), and the second has no
/// segment at all. Both belong to the router's own 404.
pub fn page_segment(path: &str) -> Option<&str> {
    let segment = path.strip_prefix(PAGE_PREFIX)?;

    (!segment.is_empty() && !segment.contains('/')).then_some(segment)
}

/// The page number a path names, when it names one it could be about.
///
/// `None` for every other path, for a segment that is not a number, and for one
/// too large for a `u32` — the last two are what `path_param!` itself answers
/// with a not-found, so the two layers agree about which URLs exist. `1` is a
/// number here: the page it names exists, at another address, and the 301 is
/// [`page_path`]'s caller's job.
pub fn page_number(path: &str) -> Option<u32> {
    page_segment(path)?.parse().ok()
}

/// The path page `number` is published at: [`PATH`] for page 1, `/himene/page/{n}`
/// for the rest.
///
/// One function rather than a format string at each call site, because four of
/// them build this URL: the nav's links, the Markdown document's page line, the
/// canonical URL in the document head, and the 301's target for `/himene/page/1`.
pub fn page_path(number: u32) -> String {
    if number <= 1 {
        PATH.to_owned()
    } else {
        format!("{PAGE_PREFIX}{number}")
    }
}

/// How many pages a catalogue of `total` songs has.
///
/// At least one, so an empty catalogue is one empty page rather than none: a
/// site with nothing published still has an index, and the alternative — zero
/// pages, every number out of range — would 404 the index itself.
pub fn page_count(total: u32) -> u32 {
    total.div_ceil(PAGE_SIZE as u32).max(1)
}

/// How far into the catalogue page `number` starts.
///
/// The one place the offset is computed: the page, the Markdown document of that
/// page and MCP's `list_songs` all have to agree about where a page begins, and
/// a second spelling of `(n - 1) * PAGE_SIZE` is how one of them ends up one row
/// off. Saturating, so page 0 (which no URL serves) reads the first page's
/// window rather than a negative offset SQLite would take literally.
pub fn offset(number: u32) -> i64 {
    i64::from(number.saturating_sub(1)) * PAGE_SIZE
}

/// `/himene` — page 1: every published song's first page, newest first.
#[page("/himene")]
pub async fn songs() -> Result<impl View> {
    Ok(view! { index_body(number: 1) })
}

/// `/himene/page/{n}` — page *n* of the index, for *n* ≥ 2.
///
/// **A number this route does not own is a 404, and page 1 is one of them.**
/// The index's first page is published at [`PATH`] and nowhere else, so
/// `/himene/page/1` is not served here even if it somehow reaches this handler
/// with the negotiation layer's `301` out of the way — serving it would be the
/// second address of one page that the whole page-segment design exists to
/// avoid.
///
/// The parse is `path_param!`'s, with `error = not_found`, so `/himene/page/x`
/// and `/himene/page/99999999999999` come back as the branded 404 through the
/// layout's error boundary rather than as a 500 or a bare router 404. What
/// remains — the range — is [`index_body`]'s, because the catalogue's size is a
/// read and the shape is not.
#[page("/himene/page/{page}")]
pub async fn page(cx: &Cx) -> Result<impl View> {
    let number = *path_param::<Page>(cx)?;
    (number >= 2).then_some(()).ok_or_not_found()?;

    Ok(view! { index_body(number: number) })
}

/// One page of the index: the heading, the table, the pagination, and the way to
/// the multi-lyric page.
///
/// The two page handlers above are thin because everything except "which page"
/// is the same for all of them, and a second copy of this markup is how page 3
/// ends up missing the picker's link or the artist column.
///
/// **Out of range is the branded 404, not a clamp** — his decision, and the one
/// this read enforces: `pages` is read before the songs are, so a number past
/// the end never reaches the table.
#[component]
pub async fn index_body(cx: &Cx, number: u32) -> Result<impl View> {
    let lang = i18n::resolve(cx);
    let pool = state::db(cx).pool();
    let pages = page_count(db::counts(pool).await?.songs);
    (1..=pages)
        .contains(&number)
        .then_some(())
        .ok_or_not_found()?;

    let listed = db::songs_page(pool, SongOrder::Newest, PAGE_SIZE, offset(number)).await?;

    // The nav, as data: the number, the URL it is published at in this request's
    // language, and whether it is the page being served. Built here rather than
    // in the markup because the URL comes from `page_path` and the language, and
    // because a page that is not served is not a link.
    let numbers: Vec<(u32, String, bool)> = (1..=pages)
        .map(|n| (n, i18n::link(cx, &page_path(n)), n == number))
        .collect();

    Ok(view! {
        <div class=(theme::PAGE)>
            <h1 class=(theme::H1)>(i18n::text(lang, Key::IndexTitle))</h1>
            <div class=(theme::INDEX_PANEL)>
                <table class="w-full">
                    <thead>
                        <tr>
                            <th class=(theme::INDEX_HEAD)>
                                (i18n::text(lang, Key::IndexColumnTitle))
                            </th>
                            <th class=(class!(theme::INDEX_HEAD, theme::INDEX_COLUMN_ARTIST))>
                                (i18n::text(lang, Key::IndexColumnArtist))
                            </th>
                        </tr>
                    </thead>
                    <tbody>
                        if listed.is_empty() {
                            <tr>
                                <td colspan="2" class=(theme::INDEX_EMPTY)>
                                    (i18n::text(lang, Key::IndexEmpty))
                                </td>
                            </tr>
                        } else {
                            for song in listed {
                                song_row(song: song)
                            }
                        }
                    </tbody>
                </table>
            </div>
            // The pagination, under the table it pages through: one link per
            // page, and the page being served marked rather than linked — a link
            // to the page the reader is already on goes nowhere new, and
            // `aria-current` is what tells a screen reader which number that is.
            //
            // Not a `rel=prev`/`next` pair: Google dropped them as a
            // consolidation signal, so the pages stand on their own canonical
            // URLs instead (see `ui/layout.rs`), and the numbers here are the
            // crawlable links between them.
            //
            // The whole row is chrome: it prints as nothing, like the header and
            // the footer.
            if pages > 1 {
                <nav class=(theme::PAGINATION) aria-label=(i18n::text(lang, Key::IndexTitle))>
                    for (n, url, current) in numbers {
                        if current {
                            <span aria-current="page" class=(theme::PAGINATION_CURRENT)>(n)</span>
                        } else {
                            <a href=(url) class=(class!(theme::PAGINATION_LINK, theme::FOCUS))>(n)</a>
                        }
                    }
                </nav>
            }
            // The index's own next step: a reader who has just scanned the
            // catalogue can choose several songs and read them together. A plain
            // link rather than a control of its own, and the page it opens is
            // `noindex` — see `pages::book`.
            <p class=(theme::BOOK_ENTRY)>
                <a href=(i18n::link(cx, book::PATH)) class=(class!(theme::BUTTON_OUTLINE, theme::FOCUS))>
                    (i18n::text(lang, Key::BookOpen))
                </a>
            </p>
        </div>
    })
}

/// One row of the index: the title, as a link, then its credited artists.
///
/// The artist cell holds an empty string when a song has no credit rather than
/// disappearing — v3 joined an empty list, and a missing `<td>` would shift the
/// row's remaining columns. Both fixtures with no artists depend on this.
#[component]
pub async fn song_row(cx: &Cx, song: Song) -> Result<impl View> {
    // The route's own parameter, so `/himene/` is spelled in one file: the index
    // and the home page both link through `pages::song`'s `#[page]` path. The
    // resolved address is then put through the request's language, so a reader
    // on `/ty/himene` stays in Tahitian when they open a song.
    let url = i18n::link(
        cx,
        &href!(sheet::song, sheet::Slug(song.get_segment())).resolve(cx),
    );
    let title = song.get_title();
    let artists = song
        .get_artists()
        .iter()
        .map(|artist| (artist::link(cx, artist), artist.get_fullname()))
        .collect::<Vec<(String, String)>>();

    Ok(view! {
        <tr class=(theme::INDEX_ROW)>
            <td class=(theme::INDEX_CELL)>
                <a href=(url)>(title)</a>
            </td>
            <td class=(class!(theme::INDEX_CELL, theme::INDEX_COLUMN_ARTIST))>
                for (index, (url, name)) in artists.into_iter().enumerate() {
                    if index > 0 { ", " }
                    <a href=(url) class=(theme::LINK)>(name)</a>
                }
            </td>
        </tr>
    })
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use super::*;
    use crate::db::{Db, fixtures};

    /// The index promises *every* published song, so the pages have to lose none
    /// and duplicate none. The two-credit fixture is why both halves are checked
    /// here: one join row per credit is exactly the shape step 3a's `LIMIT` bug
    /// mishandled, and a paged read goes through the same `assemble`.
    #[tokio::test]
    async fn the_first_page_lists_the_newest_songs_once_each() {
        let db = Db::open_in_memory().await.expect("in-memory database");
        fixtures::seed(db.pool()).await.expect("seed fixtures");

        let listed = db::songs_page(db.pool(), SongOrder::Newest, PAGE_SIZE, offset(1))
            .await
            .expect("the index's first page");

        // The window is the newest `PAGE_SIZE` of the catalogue — which is the
        // whole of it while the fixtures are smaller than a page, and the point
        // of the assertion either way.
        let everything = db::songs(db.pool(), SongOrder::Newest, None)
            .await
            .expect("the whole index");
        let expected: Vec<String> = everything
            .iter()
            .take(PAGE_SIZE as usize)
            .map(Song::get_id)
            .collect();

        assert_eq!(
            listed.iter().map(Song::get_id).collect::<Vec<_>>(),
            expected
        );
        let ids: BTreeSet<String> = listed.iter().map(Song::get_id).collect();
        assert_eq!(ids.len(), listed.len(), "a song arrived twice");

        let duo = listed
            .iter()
            .find(|song| song.get_id() == "4cfl27ia9hndgetgr1o7")
            .expect("the two-credit fixture");
        assert_eq!(duo.get_artists().len(), 2, "a credit was lost to the join");
    }

    /// Newest first — and specifically the ordering v3's index used, which is
    /// also what the home page's first table shows. The expected order is
    /// derived from the fixtures rather than written out, so editing a fixture's
    /// `created_at` cannot leave this test asserting yesterday's answer.
    #[tokio::test]
    async fn the_index_is_newest_first() {
        let db = Db::open_in_memory().await.expect("in-memory database");
        fixtures::seed(db.pool()).await.expect("seed fixtures");

        let listed = db::songs_page(db.pool(), SongOrder::Newest, PAGE_SIZE, offset(1))
            .await
            .expect("the index's first page");

        let mut expected: Vec<(&str, &str)> = fixtures::SONGS
            .iter()
            .map(|fixture| (fixture.created_at, fixture.id))
            .collect();
        // The fixture timestamps all share one format, so this is the same
        // descending comparison SQLite and SurrealDB both make on the string.
        expected.sort();
        expected.reverse();
        let expected: Vec<String> = expected
            .iter()
            .take(PAGE_SIZE as usize)
            .map(|(_, id)| (*id).to_string())
            .collect();

        let actual: Vec<String> = listed.iter().map(Song::get_id).collect();
        assert_eq!(actual, expected, "the index is not newest first");
    }

    /// Reading every page in order has to reproduce the whole index, in the same
    /// order and with no song on two pages. This is the invariant the page
    /// segment is for, and it is checked against the same constant the page uses
    /// rather than against a number written here.
    #[tokio::test]
    async fn every_song_is_on_exactly_one_page() {
        let db = Db::open_in_memory().await.expect("in-memory database");
        fixtures::seed(db.pool()).await.expect("seed fixtures");

        let everything = db::songs(db.pool(), SongOrder::Newest, None)
            .await
            .expect("the whole index");
        let expected: Vec<String> = everything.iter().map(Song::get_id).collect();

        let mut paged: Vec<String> = Vec::new();
        for number in 1..=page_count(expected.len() as u32) {
            // Not `page`: `#[page("/himene/page/{page}")]` put a unit struct of
            // that name in this module's type namespace, and the binding would be
            // read as a pattern match against it. See the module docs.
            let listed = db::songs_page(db.pool(), SongOrder::Newest, PAGE_SIZE, offset(number))
                .await
                .expect("a page of the index");
            paged.extend(listed.iter().map(Song::get_id));
        }

        assert_eq!(
            paged, expected,
            "pagination lost, doubled or reordered a song"
        );
    }

    /// The path reader, on every shape a URL can take — including the two that
    /// are *not* a page of the index, which is what keeps `/himene/page` a song
    /// URL (and so the 404 of a song that does not exist) rather than a page.
    #[test]
    fn a_page_url_is_the_page_it_names() {
        assert_eq!(page_path(1), PATH);
        assert_eq!(page_path(2), "/himene/page/2");
        assert_eq!(page_path(43), "/himene/page/43");

        assert_eq!(page_segment("/himene/page/2"), Some("2"));
        assert_eq!(page_segment("/himene/page/x"), Some("x"));
        assert_eq!(page_segment("/himene/page"), None);
        assert_eq!(page_segment("/himene/page/"), None);
        assert_eq!(page_segment("/himene/page/2/3"), None);
        assert_eq!(page_segment("/himene/ahani-e"), None);
        assert_eq!(page_segment(PATH), None);

        assert_eq!(page_number("/himene/page/2"), Some(2));
        // Page 1's number parses: it names a page, published at another address.
        assert_eq!(page_number("/himene/page/1"), Some(1));
        // Zero and a number no `u32` holds are the page's own business, and both
        // are out of range rather than unreadable.
        assert_eq!(page_number("/himene/page/0"), Some(0));
        assert_eq!(page_number("/himene/page/99999999999999"), None);
        assert_eq!(page_number("/himene/page/x"), None);
        assert_eq!(page_number("/himene/page"), None);
        assert_eq!(page_number(PATH), None);
    }

    /// A remainder gets its own page, an empty catalogue gets one page rather
    /// than none, and a page's offset is where the one before it ended.
    #[test]
    fn the_page_count_covers_a_remainder_and_an_empty_catalogue() {
        let size = PAGE_SIZE as u32;

        assert_eq!(page_count(0), 1, "an empty catalogue has no index");
        assert_eq!(page_count(1), 1);
        assert_eq!(page_count(size), 1);
        assert_eq!(page_count(size + 1), 2);
        assert_eq!(page_count(3 * size), 3);
        // The live catalogue: forty-three songs, three pages.
        assert_eq!(page_count(43), 3);

        assert_eq!(offset(1), 0);
        assert_eq!(offset(2), PAGE_SIZE);
        assert_eq!(offset(3), 2 * PAGE_SIZE);
        assert_eq!(
            offset(0),
            0,
            "no URL serves page 0, but it must not read backwards"
        );
    }
}
