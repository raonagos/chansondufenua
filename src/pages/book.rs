//! Several songs on one page — `/puta-himene`, the book.
//!
//! A reader picks songs and reads their lyrics one after another, on one page,
//! with the chords where they were written. That is the whole feature, and the
//! page is arranged around one sentence of the scope: *printing is the point*. A
//! singer printing a booklet of four songs should not have to print four pages
//! and staple them.
//!
//! # The selection is the URL
//!
//! `/puta-himene?s=te-here&s=ahani-e` — one `s` per chosen song, **in reading
//! order**. A query string rather than a path, deliberately: a path-shaped
//! selection (`/puta-himene/te-here+ahani-e`) would have to be told apart from a
//! slug by more rules than it is worth — and a selection is not an address of its
//! own anyway, being every ordered subset of the catalogue — while a query names
//! the set and keeps each song's own address intact.
//!
//! The selection is a **set**: a segment repeated in the URL is read once, and
//! two segments that name the same song (a slug and its id) are one sheet.
//! Silence would be the wrong answer either way — a reader who asked for the same
//! song twice asked for it once, and the order they first named it in is the order
//! they read in.
//!
//! # The head is `noindex`, and there is no canonical
//!
//! Decided here, once, both ways:
//!
//! * **`noindex`**, because the URL space is every ordered subset of the
//!   catalogue — thousands of documents, of which a handful will ever be linked,
//!   each of them quoting lyrics that already have a page of their own. An index
//!   full of those is duplicate content competing with the sheets they copy, and
//!   a crawler following the page still reaches every song: the titles are links
//!   to `/himene/{slug}` and the directive is `noindex, follow`.
//! * **No canonical**, because there is no preferred URL for a selection: the
//!   selection *is* the URL. A `<link rel="canonical">` on a page that also says
//!   `noindex` would be telling a crawler to index a different address while
//!   telling it not to index this one.
//!
//! That is the same decision the create-song form makes, for the same reason —
//! this page is not written to be found — and `ui::layout::document_head` is
//! where the two are one branch each.
//!
//! # What is here, and what is not
//!
//! * **No transposition.** `?tr=` is the sheet's control and this page has none:
//!   the chords are the author's, the way the Markdown and JSON forms of a
//!   selection are. A reader who wants a different key follows a title to its
//!   sheet, which is where the control and the reader's own choice live.
//! * **Auto-scroll, where there is something to read.** A selection carries the
//!   sheet's own speed bar — [`crate::ui::autoscroll`], not a copy of it, because
//!   the control is the reader's and not either page's. The reviewer asked for
//!   exactly this on 2026-10-07: auto-scroll "is a reading aid for the lyrics,
//!   not a song-picker", so it belongs on the page where a reader reads several
//!   lyrics and *not* on the picker, which is a form with no lyric to crawl.
//!   The bar arrives `hidden` and the shared script un-hides it, so a selection
//!   read with JavaScript off draws no unusable control — the same guarantee a
//!   sheet makes, made the same way.
//! * **No view is counted.** A served sheet counts as a view ([`crate::pages::song`]),
//!   and this page serves the lyric without being the song's page: counting here
//!   would make one reader of a four-song selection four views of four songs, and
//!   the most-viewed table on the front page would report a bookmarklet's habits
//!   rather than the site's readers. The titles are links; following one is a view.
//!
//! # The other two forms
//!
//! A selection has all three of the site's representations, joined by the `Link`
//! headers: this HTML page, the same sheets as one Markdown document
//! (`routes::negotiation::selection_document`, served on `Accept: text/markdown`),
//! and the same rows as JSON at `/api/songs?s=…`. The picker — the page with no
//! selection — has no Markdown form, for the reason the create-song form has
//! none: it is a form, not prose.
//!
//! Note the naming constraint this directory's pages share: `#[page(...)]` emits
//! a unit struct named after its handler in this module's *type* namespace, so a
//! local binding called `book` would be read as a pattern matching it.

use topcoat::{
    Result,
    context::Cx,
    router::{error::RouterErrorExt, page, request::uri},
    view::{Unescaped, View, class, component, view},
};

use sqlx::SqlitePool;

use crate::db::{self, DbResult, SongOrder};
use crate::domain::Song;
use crate::domain::chord;
use crate::i18n::{self, Key};
use crate::pages::song::lyric_line;
use crate::state;
use crate::ui::autoscroll::{self, speed_bar};
use crate::ui::theme;

/// `/puta-himene` — the address, in one place.
///
/// Two other modules name it: `routes::language`'s `is_page` (a page is
/// language-scoped, and this one is a page) and `routes::negotiation`, which
/// serves the Markdown form of a selection. `#[page]` cannot take a constant —
/// it is a macro over a literal path — so the constant restates it, as
/// `pages::editor::PATH` does.
pub const PATH: &str = "/puta-himene";

/// The query parameter that names one chosen song.
///
/// Short, because it appears once per song: four songs cost twelve characters of
/// URL rather than forty. Slugs are `[a-z0-9-]` (`domain::slug`), so a value never
/// needs percent-encoding and the reader that reads it does not decode anything.
const SELECTION: &str = "s";

/// The songs a query string selects, in the order it names them.
///
/// A hand-rolled reader, like [`chord::offset`] and `routes::language`'s
/// `explicit`, and for the same reason: this runs before any extractor and wants
/// a *list* out of the query, not one typed field. A pair with no `=` is read as
/// an empty value and dropped, so `?s` and `?s=` are the same non-selection as an
/// absent parameter, and a segment repeated in the URL is kept once — the
/// selection is a set.
///
/// Nothing here decides whether the segments name anything: a selection is a URL
/// shape, and `resolve` is the read.
pub fn selection(query: &str) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();

    for pair in query.split('&').filter(|pair| !pair.is_empty()) {
        let (name, value) = pair.split_once('=').unwrap_or((pair, ""));
        if name != SELECTION {
            continue;
        }
        let value = value.trim();
        if value.is_empty() || out.iter().any(|kept| kept == value) {
            continue;
        }
        out.push(value.to_owned());
    }

    out
}

/// The query string that names `segments` — the inverse of [`selection`].
///
/// Used for the URLs the page *publishes*: the `Link` header that names the
/// JSON read of this selection and the `alternate` that names its Markdown
/// form must name the selection the reader is looking at, and a link built from
/// the page's own path alone would promise the picker instead.
pub fn query(segments: &[String]) -> String {
    segments
        .iter()
        .map(|segment| format!("{SELECTION}={segment}"))
        .collect::<Vec<String>>()
        .join("&")
}

/// The songs a selection names, in reading order — or `None` for a selection the
/// site cannot serve.
///
/// `None` when any segment names no song at all or names a draft. **A hole is
/// not dropped silently**: a selection is a promise about what the page carries,
/// and a page that quietly served three of the four songs a reader asked for
/// would be worse than one that says the URL is wrong. Both callers turn `None`
/// into the 404 — the page into the branded one, the JSON read into its own.
///
/// Two segments that name the same song are one sheet, because that is what the
/// reader asked for: an id URL and a slug URL are two spellings of one address
/// (`db::Addressed::is_canonical` is the difference, and it does not matter
/// here — both name the same words).
///
/// `pub(crate)` because the JSON read of a selection is the same selection:
/// `routes::api` asks this function rather than resolving the segments a second
/// time, so the two representations cannot disagree about which songs the URL
/// named.
pub(crate) async fn resolve(pool: &SqlitePool, segments: &[String]) -> DbResult<Option<Vec<Song>>> {
    let mut out: Vec<Song> = Vec::new();

    for row in db::songs_at(pool, segments).await? {
        // A hole anywhere is the whole selection's answer: the page does not
        // serve part of what its URL asked for.
        let Some(found) = row else {
            return Ok(None);
        };
        let sheet = found.into_song();
        if !sheet.is_published() {
            return Ok(None);
        }
        if out.iter().any(|kept| kept.get_id() == sheet.get_id()) {
            continue;
        }
        out.push(sheet);
    }

    Ok(Some(out))
}

/// `/puta-himene` — a chosen set of songs, or the form that chooses them.
///
/// One page, two states, decided by the query string and by nothing else: a
/// selection renders the sheets, and no selection renders the picker. They are
/// one view rather than two pages because they are one address — the reader
/// builds the URL in the form and reads it back on the page, and a page that
/// moved between the two would lose the address the form just made.
///
/// **The picker is a `GET` form.** Its answer is a URL a reader can keep, print
/// or send to someone else, which is the whole reason the selection lives in the
/// URL: no script, no `fetch`, no second request, and the boxes are `name="s"` —
/// the same parameter [`selection`] reads, so the form and the parser cannot
/// drift about what a chosen song is called.
///
/// The catalogue is read only for the picker: it is the list of songs to choose
/// *from*, and a selection never lists it.
///
/// The chosen song is the song page's sheet — see `chosen_sheet` — with the
/// page's own `<h1>` above it, because this page's title is the page's and each
/// song's is a section of it.
#[page("/puta-himene")]
pub async fn book(cx: &Cx) -> Result<impl View> {
    let lang = i18n::resolve(cx);
    let segments = selection(uri(cx).query().unwrap_or(""));

    // A selection, resolved — and `None` when the URL names no selection at all,
    // which is the picker rather than an empty one. A selection with a hole in it
    // is the branded 404 (`ok_or_not_found`), the same answer a missing song
    // gets: this page does not serve part of what its URL asked for.
    let chosen = if segments.is_empty() {
        None
    } else {
        Some(
            resolve(state::db(cx).pool(), &segments)
                .await?
                .ok_or_not_found()?,
        )
    };

    let listed = if chosen.is_none() {
        db::songs(state::db(cx).pool(), SongOrder::Newest, None).await?
    } else {
        Vec::new()
    };

    Ok(view! {
        <div class=(theme::PAGE)>
            <h1 class=(theme::H1)>(i18n::text(lang, Key::BookTitle))</h1>

            match chosen {
                Some(sheets) => {
                    // The reader's own control over the selection, before the
                    // first sheet: the bar the sheet carries, from the one module
                    // that owns it. It arrives hidden, and the script at the foot
                    // of this page is the only thing that draws it.
                    <div class=(theme::BOOK_HEAD)>
                        speed_bar()
                    </div>
                    for sheet in sheets {
                        chosen_sheet(sheet: sheet)
                    }
                    <script type="text/javascript">(Unescaped::new_unchecked(autoscroll::SCRIPT))</script>
                },
                None => {
                    <p class=(theme::LEAD)>(i18n::text(lang, Key::BookHint))</p>

                    if listed.is_empty() {
                        // Nothing to choose from. The index's own line, because it
                        // is the same fact, and a second sentence for it would be a
                        // second thing to translate.
                        <p class=(theme::INDEX_EMPTY)>(i18n::text(lang, Key::IndexEmpty))</p>
                    } else {
                        <form
                            method="get"
                            action=(PATH)
                            class=(theme::FORM_PANEL)
                        >
                            for sheet in listed {
                                <label class=(theme::BOOK_PICK_ROW)>
                                    <input
                                        type="checkbox"
                                        name=(SELECTION)
                                        value=(sheet.get_segment())
                                        class=(class!(theme::BOOK_PICK_BOX, theme::FOCUS))
                                    />
                                    <span>(sheet.get_title())</span>
                                    <span class=(theme::BOOK_PICK_ARTISTS)>(artists_of(&sheet))</span>
                                </label>
                            }
                            <div class=(class!(theme::FORM_SUBMIT, theme::BOOK_ENTRY))>
                                <button
                                    type="submit"
                                    class=(class!(theme::FORM_SUBMITTER, theme::FOCUS))
                                >
                                    (i18n::text(lang, Key::BookRead))
                                </button>
                            </div>
                        </form>
                    }
                },
            }
        </div>
    })
}

/// One chosen song: its title, its credits, and its whole lyric with the chords.
///
/// The chords are prepared here the way the sheet prepares them — the same words
/// for the same chords in the same language, from the same
/// [`chord::localised_line`] at the author's own offset. The Markdown and JSON
/// forms of this page keep the canonical spelling, for the reason they do
/// everywhere else: those are the forms a machine reads.
#[component]
async fn chosen_sheet(cx: &Cx, sheet: Song) -> Result<impl View> {
    let lang = i18n::resolve(cx);
    let title = sheet.get_title();
    let url = sheet.get_path();
    let artists = artists_of(&sheet);
    let lines = sheet
        .lyrics_lines()
        .into_iter()
        .map(|line| chord::localised_line(line, 0, lang))
        .collect::<Vec<_>>();

    Ok(view! {
        <article class=(class!(theme::SONG_SHEET, theme::BOOK_ITEM))>
            <header class=(theme::SONG_HEAD)>
                <div class=(theme::SONG_HEADING)>
                    <h2 class=(theme::SONG_TITLE)>
                        <a href=(url) class=(theme::LINK)>(title)</a>
                    </h2>
                    // An uncredited song renders no line, the same rule the sheet
                    // keeps: an empty paragraph carries a margin of its own.
                    if !artists.is_empty() {
                        <p class=(theme::SONG_ARTISTS)>(artists)</p>
                    }
                </div>
            </header>

            <div class=(theme::LYRICS)>
                for line in lines {
                    lyric_line(line: line)
                }
            </div>
        </article>
    })
}

/// A song's credits as one line, the way the sheet page writes them.
fn artists_of(sheet: &Song) -> String {
    sheet
        .get_artists()
        .iter()
        .map(|artist| artist.get_fullname())
        .collect::<Vec<String>>()
        .join(", ")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::{Db, fixtures};

    /// The parameter name is the form's and the parser's at once: a checkbox
    /// named anything else would submit a query this page reads as "nothing
    /// chosen".
    #[test]
    fn a_chosen_song_is_named_by_one_parameter() {
        assert_eq!(SELECTION, "s");
    }

    /// The selection is the URL, read left to right: the order a reader ticked
    /// the boxes in is the order they read in, so nothing may sort or group it.
    #[test]
    fn the_selection_is_read_in_the_order_it_is_named() {
        assert_eq!(selection("s=b&s=a&s=c"), ["b", "a", "c"]);
        assert_eq!(selection("s=a"), ["a"]);

        // Other parameters are not part of it, wherever they sit.
        assert_eq!(selection("s=a&tr=3&s=b"), ["a", "b"]);
        assert_eq!(selection("lang=ty&s=a"), ["a"]);
        assert_eq!(chord::offset("s=a&tr=3&s=b"), 3);
    }

    /// A repeat names one song once: the selection is a set, and a URL that
    /// names the same song twice is not a request to read it twice.
    #[test]
    fn a_repeated_segment_is_one_song() {
        assert_eq!(selection("s=a&s=a&s=b"), ["a", "b"]);
    }

    /// Nothing chosen, in every spelling a browser or a client can produce: an
    /// absent parameter, a bare one, an empty one, and a segment of spaces. All
    /// of them are the picker rather than a selection of nothing.
    #[test]
    fn an_empty_selection_is_not_a_selection() {
        for query in ["", "s", "s=", "s=&s=&s", "s=   ", "tr=2", "s%5B%5D=a"] {
            assert!(selection(query).is_empty(), "{query:?}");
        }
    }

    /// The query builder is the reader's inverse: what [`query`] writes is what
    /// [`selection`] reads back, which is what makes the `Link` header name the
    /// selection the reader is looking at.
    #[test]
    fn the_published_query_reads_back_as_the_same_selection() {
        let segments = vec!["te-here".to_owned(), "ahani-e".to_owned()];
        assert_eq!(query(&segments), "s=te-here&s=ahani-e");
        assert_eq!(selection(&query(&segments)), segments);
        assert_eq!(query(&[]), "");
    }

    /// A selection is served when every segment names a published song — and is
    /// a hole when one of them does not, whether it names nothing at all or
    /// names a draft.
    ///
    /// The draft is the reason this test is not "does it return the songs": a
    /// selection that dropped the unpublished one would serve the reader a page
    /// that quietly disagrees with its own URL.
    #[tokio::test]
    async fn a_selection_with_a_hole_is_not_served() {
        let db = Db::open_in_memory().await.expect("in-memory database");
        fixtures::seed(db.pool()).await.expect("seed fixtures");

        sqlx::query(
            "INSERT INTO song (id, title, lyrics, view_count, published, created_at, updated_at) \
             VALUES ('hidden0000000000000', 'Un brouillon', ?1, 1, 0, ?2, ?2)",
        )
        .bind("z".repeat(crate::domain::song::LYRICS_MIN))
        .bind("2026-01-01T00:00:00.000000000Z")
        .execute(db.pool())
        .await
        .expect("insert a draft");

        let slug = fixtures::SONGS[0].slug.to_owned();
        let id = fixtures::SONGS[0].id.to_owned();

        let served = resolve(db.pool(), std::slice::from_ref(&slug))
            .await
            .expect("the read");
        assert_eq!(served.expect("a published song").len(), 1);

        assert!(
            resolve(db.pool(), &["no-such-song".to_owned()])
                .await
                .expect("the read")
                .is_none(),
            "an unknown segment was dropped instead of refused"
        );
        assert!(
            resolve(db.pool(), &["hidden0000000000000".to_owned()])
                .await
                .expect("the read")
                .is_none(),
            "a draft was served on a public page"
        );

        // A hole anywhere in the list is the whole selection's answer, and the
        // songs that did resolve are not served beside it.
        assert!(
            resolve(db.pool(), &[slug, "no-such-song".to_owned()])
                .await
                .expect("the read")
                .is_none()
        );

        // One song reached two ways is one sheet, in the order it was first
        // named.
        let both = resolve(db.pool(), &[id, fixtures::SONGS[1].slug.to_owned()])
            .await
            .expect("the read")
            .expect("both are published");
        assert_eq!(both.len(), 2);
        assert_eq!(both[0].get_id(), fixtures::SONGS[0].id);
        let twice = resolve(
            db.pool(),
            &[
                fixtures::SONGS[0].slug.to_owned(),
                fixtures::SONGS[0].id.to_owned(),
            ],
        )
        .await
        .expect("the read")
        .expect("one song, two spellings");
        assert_eq!(twice.len(), 1);
    }
}
