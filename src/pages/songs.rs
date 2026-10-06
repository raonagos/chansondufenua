//! The song index — `/himene`.
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
//! The index has a second representation: `routes/negotiation.rs` answers
//! `Accept: text/markdown` with the same list as one Markdown document, from the
//! same unbounded read — every published song, newest first.
//!
//! (Note the phrasing: v3's heading size is *described*, not spelled. Tailwind
//! scans the crate's source text for class names, and a utility named in a doc
//! comment is emitted into the stylesheet whether or not anything uses it.)
//!
//! **Local bindings here cannot be called `songs`.** `#[page("/himene")]` emits
//! a unit struct named after its handler, in this module, in the *type*
//! namespace. `let songs = …` is then read as a pattern matching that unit
//! struct rather than as a new binding, and the compiler reports a type error
//! several lines away from the cause. The page's rows live in `listed`.

use topcoat::{
    Result,
    context::Cx,
    router::{href, page},
    view::{View, class, component, view},
};

use crate::db::{self, SongOrder};
use crate::domain::Song;
use crate::i18n::{self, Key};
use crate::pages::song as sheet;
use crate::state;
use crate::ui::theme;

/// `/himene` — every published song, newest first.
///
/// **Unbounded, on purpose.** v3 asked for `page 0, limit 255`: a pagination
/// call with no pagination behind it. `app/src/app.rs` registers `himene/""` and
/// `himene/:id` and never a page segment, so `use_params_map().get("page")` in
/// that page could only ever read `0` — the 255th song would have been invisible
/// with nothing to page to. A list that promises every song should not have a
/// silent ceiling; when the catalogue outgrows one page, the page segment
/// arrives with the pagination it implies.
///
/// The ordering is v3's `ORDER BY created_at DESC` — the same
/// [`SongOrder::Newest`] the home page's "Les dernières ajouts" asks for.
///
/// The path is a constant as well as an attribute, because two other modules
/// name it: `robots.txt`'s `Link` headers (`routes/negotiation.rs`) and the
/// fixed-pages sitemap. `#[page]` cannot take a constant — it is a macro over a
/// literal path — so the constant restates it, as `pages::editor::PATH` does.
pub const PATH: &str = "/himene";

/// The index's `<meta name="description">`.
///
/// The page's own heading, then what the page holds. Same budget as a song's:
/// [`DESCRIPTION_MAX`](crate::domain::song::DESCRIPTION_MAX) is what a search
/// engine shows.
///
/// French, like the rest of this page's prose: the lyrics are never translated
/// and neither is the catalogue's own copy. The `<title>` above does follow the
/// request's language, because a title is chrome.
pub const DESCRIPTION: &str =
    "Toutes les chansons du fenua : paroles et accords des chansons tahitiennes et polynésiennes.";

#[page("/himene")]
pub async fn songs(cx: &Cx) -> Result<impl View> {
    let lang = i18n::resolve(cx);
    let listed = db::songs(state::db(cx).pool(), SongOrder::Newest, None).await?;

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
        .map(|artist| artist.get_fullname())
        .collect::<Vec<String>>()
        .join(", ");

    Ok(view! {
        <tr class=(theme::INDEX_ROW)>
            <td class=(theme::INDEX_CELL)>
                <a href=(url)>(title)</a>
            </td>
            <td class=(class!(theme::INDEX_CELL, theme::INDEX_COLUMN_ARTIST))>(artists)</td>
        </tr>
    })
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use super::*;
    use crate::db::{Db, fixtures};

    /// The index promises *every* published song, so the read has to lose none
    /// and duplicate none. The two-credit fixture is why both halves are checked
    /// here: one join row per credit is exactly the shape step 3a's `LIMIT` bug
    /// mishandled, and an unbounded read goes through the same `assemble`.
    #[tokio::test]
    async fn the_index_lists_every_published_song_once() {
        let db = Db::open_in_memory().await.expect("in-memory database");
        fixtures::seed(db.pool()).await.expect("seed fixtures");

        let listed = db::songs(db.pool(), SongOrder::Newest, None)
            .await
            .expect("the index query");

        assert_eq!(listed.len(), fixtures::SONGS.len());
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

        let listed = db::songs(db.pool(), SongOrder::Newest, None)
            .await
            .expect("the index query");

        let mut expected: Vec<(&str, &str)> = fixtures::SONGS
            .iter()
            .map(|fixture| (fixture.created_at, fixture.id))
            .collect();
        // The fixture timestamps all share one format, so this is the same
        // descending comparison SQLite and SurrealDB both make on the string.
        expected.sort();
        expected.reverse();
        let expected: Vec<String> = expected.iter().map(|(_, id)| (*id).to_string()).collect();

        let actual: Vec<String> = listed.iter().map(Song::get_id).collect();
        assert_eq!(actual, expected, "the index is not newest first");
    }
}
