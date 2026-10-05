//! The song sheet — `/himene/{id}`.
//!
//! Transcribed from v3's `SongPage` (`app/src/pages/himene/song.rs`): the title,
//! a link to the create-song page, and the lyric with its chords. Three things
//! differ, each deliberate:
//!
//! * **The lyric is rendered from data, not injected as HTML.** v3 dropped
//!   `inner_html` straight into the template, so every visual decision about a
//!   chord had to be a CSS rule matching a `<sup>`. v4 parses the same sanitised
//!   HTML into lines of text and chord spans ([`Song::lyrics_lines`]) and renders
//!   them with the site's own tokens — see `PLAN.md` §19.
//! * **The credits are shown.** v3 carried the artists only inside the page's
//!   metadata. The index lists them, so a reader arriving from the index has
//!   already seen them; one arriving from a search result never did.
//! * **A missing or unpublished song is a 404, not an empty page.** v3 asked the
//!   browser for the song and rendered nothing at all when the request failed.
//! * **A served sheet counts as a view**, where v3 incremented the count inside
//!   its own read function, before it knew whether there was a song to read.
//!
//! The page's `<head>` is not here. Topcoat 0.10 has no per-page head API, so
//! the layout decides it from the request path and the same row — see
//! `src/ui/layout.rs`.
//!
//! Note the naming constraint that every page in this directory shares:
//! `#[page("/himene/{id}")]` emits a unit struct named after its handler, in
//! this module's *type* namespace, so a local binding called `song` would be
//! read as a pattern matching it.

use topcoat::{
    Result,
    context::Cx,
    router::{error::RouterErrorExt, page, path_param},
    view::{View, class, component, view},
};

use crate::db;
use crate::domain::song::{LyricLine, LyricSpan, Song};
use crate::i18n::{self, Key};
use crate::state;
use crate::ui::theme;

// The `{id}` in this page's path.
//
// Public because the index and the home page build their links with `href!`,
// which fills a route's parameters by naming the type this declares. That is the
// point of declaring it at all: the URL shape lives in one place, in the
// `#[page]` attribute below, and no other module spells it out.
path_param!(pub id);

/// `/himene/{id}` — one song, with its chords.
///
/// **Unpublished is not found.** [`db::song`] deliberately returns drafts as
/// well, so that the editor can see them; this page is the public one and asks
/// whether the song is published before it renders anything.
#[page("/himene/{id}")]
pub async fn song(cx: &Cx) -> Result<impl View> {
    let id: &str = path_param::<Id>(cx);

    let sheet = db::song(state::db(cx).pool(), id)
        .await?
        .filter(Song::is_published)
        .ok_or_not_found()?;

    // Serving the sheet is what counts as a view, which is v3's rule: its
    // `fn::get_song_fetch_artist` opened with `UPDATE song SET view_count += 1`.
    // The v4 difference is the order — v3 incremented *before* it read, so a
    // request for an id that did not exist incremented nothing and a request for
    // a draft still counted. Here only a sheet that is about to be served counts,
    // which is what the home page's most-viewed table means by a view.
    //
    // Awaited rather than detached: this is an in-process write to a database
    // compiled into the binary, and a response that says "200" should mean the
    // view it reports was recorded.
    db::increment_view_count(state::db(cx).pool(), id).await?;

    Ok(view! { sheet_body(sheet: sheet) })
}

/// The sheet itself: a header, then the lyric.
///
/// A `#[component]` rather than a plain helper, because `view!` needs the
/// request context and only a component or a page binds one.
#[component]
pub async fn sheet_body(cx: &Cx, sheet: Song) -> Result<impl View> {
    let lang = i18n::resolve(cx);
    let title = sheet.get_title();
    let artists = sheet
        .get_artists()
        .iter()
        .map(|artist| artist.get_fullname())
        .collect::<Vec<String>>()
        .join(", ");
    let lines = sheet.lyrics_lines();

    Ok(view! {
        <article class=(theme::SONG_SHEET)>
            <header class=(theme::SONG_HEAD)>
                <div class=(theme::SONG_HEADING)>
                    <h1 class=(theme::SONG_TITLE)>(title)</h1>
                    // An uncredited song renders no line at all, rather than an
                    // empty paragraph with a margin under it.
                    if !artists.is_empty() {
                        <p class=(theme::SONG_ARTISTS)>(artists)</p>
                    }
                </div>
                <a
                    href="/himene/api"
                    class=(class!(theme::BUTTON_SMALL, theme::FOCUS))
                >
                    (i18n::text(lang, Key::AddLyrics))
                </a>
            </header>

            <div class=(theme::LYRICS)>
                for line in lines {
                    lyric_line(line: line)
                }
            </div>
        </article>
    })
}

/// One line of the lyric.
///
/// A blank line is the same element with the verse-break height instead of the
/// line height, so the loop has one shape rather than two. The two tokens are
/// composed rather than chosen between, which is why
/// `tokens_meant_to_be_composed_never_contradict_each_other` watches them.
///
/// A chord is a `<sup>`, and it keeps v3's `data-nosnippet`. That attribute is
/// not decoration: a search engine that quotes the page prints the chords inline
/// with the words, where they read as typos.
#[component]
pub async fn lyric_line(line: LyricLine) -> Result<impl View> {
    let blank = line.is_empty();

    Ok(view! {
        <div class=(class!(theme::LYRIC_LINE, theme::LYRIC_GAP if blank))>
            for span in line {
                match span {
                    LyricSpan::Text(text) => { (text) },
                    LyricSpan::Chord(chord) => {
                        <sup class=(theme::CHORD) data-nosnippet="true">(chord)</sup>
                    },
                }
            }
        </div>
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::{Db, fixtures};

    /// The page renders only what is published. This is the rule
    /// [`db::song`]'s doc comment hands to its caller, asserted at the caller:
    /// the read returns the draft, and the filter the page applies rejects it.
    #[tokio::test]
    async fn a_draft_is_readable_but_not_publishable() {
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

        let draft = db::song(db.pool(), "hidden0000000000000")
            .await
            .expect("the read")
            .expect("the draft is readable");
        assert!(
            !draft.is_published(),
            "the page's filter can only reject a song it can see"
        );

        let visible = db::song(db.pool(), fixtures::SONGS[0].id)
            .await
            .expect("the read")
            .expect("a published fixture");
        assert!(visible.is_published());
    }

    /// [`lyric_line`] chooses the verse-break height from emptiness alone, so an
    /// empty line and a line of nothing but chords must not be confused.
    #[test]
    fn a_blank_line_is_an_empty_span_list() {
        let sheet = Song::new(
            "id".to_owned(),
            "Titre".to_owned(),
            "<div>one</div><div><br></div><div>two</div>".to_owned(),
            1,
            Vec::new(),
            true,
            chrono::Utc::now(),
            chrono::Utc::now(),
        );

        let lines = sheet.lyrics_lines();
        assert_eq!(lines.len(), 3);
        assert!(lines[1].is_empty());
        assert!(!lines[0].is_empty());
    }
}
