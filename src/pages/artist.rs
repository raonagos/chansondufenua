//! An artist's page — `/taata-himene/{id}`.
//!
//! The scope's second item: artist pages are **real indexable URLs**, not only a
//! filter on the index. A reader who follows a name on a song sheet lands here
//! and finds that artist's songs, each a link to its own sheet, and a crawler
//! finds a page with its own head, its own canonical URL and its own structured
//! data.
//!
//! # The URL is the id, and that is deliberate
//!
//! Songs moved to slugs ([`crate::domain::slug`]) because a song's title is the
//! thing a reader shares. An artist's name is not a song's title: there is no
//! rename history to keep alive, no author-facing label to mint a slug from, and
//! the id is already stable and opaque. So `/taata-himene/{id}` names the row and
//! nothing else has to be true of it — no second namespace next to the songs'
//! slugs. (The prefix itself moved once, from v4.1's French `/artiste/`; the
//! retired address is one row of `pages::retired`, not a slugger's problem.)
//!
//! # One artist, one page
//!
//! The page is served when the id names a row and is the branded 404 when it does
//! not — the same rule a song follows, and for the same reason: a page that
//! renders an empty shell for an unknown id is a page a crawler will index and a
//! reader will leave.
//!
//! # What is here
//!
//! The heading is the artist's name. Under it, that artist's published songs in
//! the index's own rows — the same table, the same tokens, the same links, so an
//! artist page and the index cannot disagree about what a song looks like. The
//! `<head>` is the layout's, built from the same row (Topcoat 0.10 has no
//! per-page head API); [`jsonld`] is the structured data that head carries.
//!
//! Note the naming constraint every page in this directory shares:
//! `#[page("/taata-himene/{id}")]` emits a unit struct named after its handler —
//! `artist` — in this module's *type* namespace, so a local binding of that name
//! would be read as a pattern matching it. The page's row lives in `found`.

use topcoat::{
    Result,
    context::Cx,
    router::{error::RouterErrorExt, href, page, path_param},
    view::{View, class, component, view},
};

use crate::db;
use crate::domain::{Artist, Song};
use crate::i18n::{self, Key};
use crate::pages::songs::song_row;
use crate::state;
use crate::ui::theme;

/// The path prefix, in one place.
///
/// `routes::language` decides whether a request path is language-scoped and
/// `routes::negotiation` decides whether it has a Markdown form; neither calls
/// the handler, and both need the *shape*. `#[page]` cannot take a constant — it
/// is a macro over a literal path — so this restates it, as `pages::songs::PATH`
/// does.
pub const PREFIX: &str = "/taata-himene/";

// The `{id}` in this page's path.
//
// Public because a song's credits link here through `href!`, which fills a
// route's parameters by naming the type this declares: the URL shape lives in
// the `#[page]` attribute and no other module spells it out.
path_param!(pub id);

/// The id a `/taata-himene/{id}` path names, if the path has that shape.
///
/// *Shape*, not existence: the segment is returned as it arrived, and the read
/// is what decides whether it names a row. One segment and nothing else —
/// `/taata-himene/a/b` is nobody's page.
pub fn segment(path: &str) -> Option<&str> {
    let segment = path.strip_prefix(PREFIX)?;

    (!segment.is_empty() && !segment.contains('/')).then_some(segment)
}

/// The root-relative address of an artist's page.
///
/// The id, not a slug: see the module docs. Used for the links the site emits and
/// for the canonical URL the head names in the request's language.
pub fn path_of(id: &str) -> String {
    format!("{PREFIX}{id}")
}

/// An artist's page — the one place a credit becomes a link.
///
/// Through `href!`, so `/taata-himene/{id}` is spelled in this page's own attribute and
/// nowhere else: a song's credits, the index's artist column and a search result
/// all come through here, and a route that moved would move them with it. One
/// URL per page means the request's language has nothing to add to it.
pub fn link(cx: &Cx, row: &Artist) -> String {
    href!(self::artist, Id(row.get_id())).resolve(cx)
}

/// The artist page's `<meta name="description">`.
///
/// The name first — what a reader searching for the songs is looking for — then
/// what the page holds. French, like the index's and every song's own: the
/// catalogue's copy is not the chrome, and the languages never reach it. Same
/// budget as a song's ([`DESCRIPTION_MAX`](crate::domain::song::DESCRIPTION_MAX)),
/// and the name gives way first, so a very long credit is cut rather than the
/// sentence being left half-written.
pub fn description(name: &str) -> String {
    let tail = " : paroles et accords de ses chansons, à retrouver sur Chanson du fenua.";
    let tail = crate::domain::song::truncate_chars(tail, crate::domain::song::DESCRIPTION_MAX);
    let room = crate::domain::song::DESCRIPTION_MAX - tail.chars().count();

    format!("{}{tail}", crate::domain::song::truncate_chars(name, room))
}

/// The structured data an artist's page carries.
///
/// Two nodes under one `@graph`:
///
/// * a **`MusicGroup`** naming the artist the page is about. The corpus's credits
///   are overwhelmingly bands and duos, which is what the type is for; it is also
///   the type the plan names.
/// * an **`ItemList`** of the songs under it, in the order the page lists them.
///   A machine reading the page has the list in the markup either way, but an
///   `ItemList` says *which* list on the page is the catalogue — the heading and
///   the table are prose and a table to a crawler.
///
/// Every URL is absolute, matching the JSON API's `url` field and the pages'
/// own canonical URLs: this is a statement about the URL space, not about the
/// response being served — which is why it is the same in every language the
/// page can be written in. The `song` rows come from the same read the page
/// renders, so the list and the markup cannot disagree.
pub fn jsonld(row: &Artist, url: &str, songs: &[Song]) -> String {
    let items = songs
        .iter()
        .enumerate()
        .map(|(index, song)| {
            serde_json::json!({
                "@type": "ListItem",
                "position": index + 1,
                "url": i18n::absolute(&song.get_path()),
                "name": song.get_title(),
            })
        })
        .collect::<Vec<_>>();

    serde_json::json!({
        "@context": "https://schema.org",
        "@graph": [
            {
                "@type": "MusicGroup",
                "@id": url,
                "name": row.get_fullname(),
                "url": url,
            },
            {
                "@type": "ItemList",
                "name": row.get_fullname(),
                "itemListElement": items,
            },
        ],
    })
    .to_string()
}

/// `/taata-himene/{id}` — one artist and the songs credited to them.
///
/// An id that names no row is the branded 404: the same `ok_or_not_found` a
/// missing song raises, so both travel through the layout's error boundary and
/// wear the site's chrome.
#[page("/taata-himene/{id}")]
pub async fn artist(cx: &Cx) -> Result<impl View> {
    let id: &str = path_param::<Id>(cx);

    let found = db::artist(state::db(cx).pool(), id)
        .await?
        .ok_or_not_found()?;

    Ok(view! { artist_body(row: found) })
}

/// The artist's page body: the name, then their songs in the index's own rows.
///
/// Reusing [`song_row`] rather than writing a second song list is the point: a
/// row that shows the title as a link and the credits beside it is the same fact
/// on both pages, and the artist page having its own idea of a row is how the
/// two would drift.
#[component]
pub async fn artist_body(cx: &Cx, row: Artist) -> Result<impl View> {
    let lang = i18n::resolve(cx);
    let listed = db::songs_by_artist(state::db(cx).pool(), &row.get_id()).await?;

    Ok(view! {
        <div class=(theme::PAGE)>
            <h1 class=(theme::H1)>(row.get_fullname())</h1>
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::{Db, fixtures};

    /// The shape reader, on every URL an artist page can arrive as — including
    /// the two that are *not* one segment, which the router answers itself.
    #[test]
    fn a_path_names_an_artist_only_when_it_has_that_shape() {
        assert_eq!(
            segment("/taata-himene/1kvm9y2tcplm43wgeuni"),
            Some("1kvm9y2tcplm43wgeuni")
        );
        // A segment with no `/` in it is a segment, whatever it holds: whether it
        // names a row is the read's question.
        assert_eq!(segment("/taata-himene/nope"), Some("nope"));

        assert_eq!(segment("/taata-himene/"), None);
        assert_eq!(segment("/taata-himene"), None);
        assert_eq!(segment("/taata-himene/a/b"), None);
        assert_eq!(segment("/himene/ahani-e"), None);
        assert_eq!(segment(PREFIX), None);
    }

    #[test]
    fn the_path_is_the_prefix_and_the_id() {
        assert_eq!(path_of("abc"), "/taata-himene/abc");
        assert_eq!(
            segment(&path_of("abc")),
            Some("abc"),
            "the builder and the reader disagree"
        );
    }

    /// The description leads with the name and stays inside a snippet's budget,
    /// even for a name at the schema's own ceiling — a name that swallowed the
    /// sentence would be cut rather than the sentence being left half-written.
    #[test]
    fn the_description_leads_with_the_name_and_fits_a_snippet() {
        let short = description("Maruia");
        assert!(short.starts_with("Maruia"), "{short}");
        assert!(short.contains("paroles"), "{short}");

        for name in ["Maruia", &"n".repeat(crate::domain::artist::FULLNAME_MAX)] {
            assert!(
                description(name).chars().count() <= crate::domain::song::DESCRIPTION_MAX,
                "the description is over budget for {name:?}"
            );
        }
    }

    /// An artist and their songs: the songs are the published ones credited to
    /// that artist, newest first, and another artist's songs never leak in.
    #[tokio::test]
    async fn the_page_lists_the_artists_songs_and_only_those() {
        let db = Db::open_in_memory().await.expect("in-memory database");
        fixtures::seed(db.pool()).await.expect("seed fixtures");

        let credited = fixtures::ARTISTS
            .iter()
            .find(|credit| {
                fixtures::SONGS
                    .iter()
                    .any(|song| song.artists.contains(&credit.fullname))
            })
            .expect("a credited fixture artist");

        let listed = db::songs_by_artist(db.pool(), credited.id)
            .await
            .expect("the artist's songs");

        assert!(
            !listed.is_empty(),
            "the fixture credits nobody: {}",
            credited.fullname
        );
        for song in &listed {
            assert!(
                song.get_artists()
                    .iter()
                    .any(|credit| credit.get_id() == credited.id),
                "{} is not credited to {}",
                song.get_title(),
                credited.fullname
            );
        }

        // Every song the fixtures credit to this artist is here, and no song
        // appears twice.
        let expected: Vec<&str> = fixtures::SONGS
            .iter()
            .filter(|song| song.artists.contains(&credited.fullname))
            .map(|song| song.id)
            .collect();
        for id in expected {
            assert_eq!(
                listed.iter().filter(|song| song.get_id() == id).count(),
                1,
                "{id} is missing or doubled"
            );
        }

        // An id that names nobody reads as nothing, not as an error.
        assert!(
            db::songs_by_artist(db.pool(), "nosuchartist00000000")
                .await
                .expect("the read")
                .is_empty()
        );
        assert!(
            db::artist(db.pool(), "nosuchartist00000000")
                .await
                .expect("the read")
                .is_none()
        );
        assert_eq!(
            db::artist(db.pool(), credited.id)
                .await
                .expect("the read")
                .expect("the row")
                .get_fullname(),
            credited.fullname
        );
    }

    /// The structured data names the artist, the page and the songs, and every
    /// URL in it is absolute — it is read from the markup, out of context.
    #[tokio::test]
    async fn the_structured_data_names_the_list_the_page_renders() {
        let db = Db::open_in_memory().await.expect("in-memory database");
        fixtures::seed(db.pool()).await.expect("seed fixtures");

        let credited = &fixtures::ARTISTS[0];
        let row = db::artist(db.pool(), credited.id)
            .await
            .expect("the read")
            .expect("the row");
        let listed = db::songs_by_artist(db.pool(), credited.id)
            .await
            .expect("the artist's songs");
        let url = format!("{}{}", crate::domain::song::SITE_URL, path_of(credited.id));

        let document: serde_json::Value =
            serde_json::from_str(&jsonld(&row, &url, &listed)).expect("valid JSON");

        assert_eq!(document["@context"], "https://schema.org");
        let graph = document["@graph"].as_array().expect("@graph is an array");
        assert_eq!(graph[0]["@type"], "MusicGroup");
        assert_eq!(graph[0]["name"], credited.fullname);
        assert_eq!(graph[0]["url"], url);

        assert_eq!(graph[1]["@type"], "ItemList");
        let items = graph[1]["itemListElement"]
            .as_array()
            .expect("the item list");
        assert_eq!(items.len(), listed.len());
        for (index, song) in listed.iter().enumerate() {
            assert_eq!(items[index]["position"], index + 1);
            assert_eq!(items[index]["name"], song.get_title());
            assert!(
                items[index]["url"]
                    .as_str()
                    .is_some_and(|url| url.starts_with(crate::domain::song::SITE_URL)),
                "a list item is not absolute"
            );
        }
    }
}
