//! The home page — `/faariiraa`, and the same page again at the root `/`.
//!
//! Transcribed from v3's `HomePage` (`app/src/pages/index.rs`): the hero, the
//! three "why" cards, the synopsis, the two song tables, and the closing call to
//! action.
//!
//! Three things differ from v3, each deliberate:
//!
//! * **The rows are served, not streamed.** v3 asked for each table over the
//!   network from the browser (a `Resource` inside `<Suspense>`), so the first
//!   byte carried two empty `<table>` elements and the songs arrived afterwards.
//!   A crawler, an agent, or a reader with JavaScript off saw no songs at all.
//!   Here both queries run before the page renders.
//! * **The rows are links, not scripts.** v3 put `onclick="window.location=…"`
//!   on every `<tr>` — with `role="button"` and `tabindex="0"` to match — and
//!   *also* put correct `<a href>` elements inside it. v4 keeps the anchors and
//!   drops the rest: the same destination, reachable without executing anything.
//! * **The surfaces are the v4 panel.** v3's cards and table panels were a
//!   translucent wash under a frosted backdrop; v4 uses the one raised-panel
//!   token the header and footer already speak. This is the one
//!   *visible* change in the step.
//!
//! The copy is v3's, byte for byte. It lives in [`copy`] as constants rather than
//! as literals in the markup so that the `<meta name="description">` the layout
//! emits for this page is built from the page's own sentences, and a test can
//! prove that it still is.
//!
//! The page has a second representation: `routes/negotiation.rs` answers
//! `Accept: text/markdown` with the same prose and the same two tables as one
//! Markdown document, and builds it from [`copy`] and `ROWS` rather than from a
//! second copy of either.

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
use crate::pages::songs;
use crate::state;
use crate::ui::theme;

/// The page's prose, in the language the response is written in.
///
/// A module rather than a flat list of constants so that a call site reads
/// `copy::HERO_TITLE` and never has to wonder whether `HERO_TITLE` was the words
/// or the styling — `theme::HERO_TITLE` is the styling.
///
/// **The prose is no longer one language.** Until v4.2 every sentence here was a
/// `const` in French — v3's words, kept byte for byte — and the site served them
/// to an English reader as they were. The reviewer read them that way ("I read
/// the descriptions are in french … so it needs to translate all things it can
/// be translate") and they now live in `locales/*.toml` with the chrome, under
/// their own keys: see [`Key::PROSE`](crate::i18n::Key::PROSE). The French is
/// still v3's, character for character; the English beside it in `en.toml` is
/// new, and a Tahitian page reads the English (his rule for prose — see
/// [`crate::i18n`]).
///
/// Two strings are *not* here, because neither is a translation unit: the site's
/// name ([`copy::HERO_TITLE`]) and a brand ([`copy::FOOT_FACEBOOK`]). The two
/// button labels are chrome like every other control's and live in
/// [`crate::i18n`] too.
pub mod copy {
    use crate::i18n::{self, Key, Lang};

    /// The hero's headline — the site's own name, which is not translated.
    pub const HERO_TITLE: &str = "Chanson du fenua";

    /// The first closing button. A brand name, so it is the same in both
    /// languages and stays a constant rather than becoming a key.
    pub const FOOT_FACEBOOK: &str = "Facebook";

    /// The hero's standfirst.
    pub fn hero_subtitle(lang: Lang) -> &'static str {
        i18n::text(lang, Key::HomeHeroSubtitle)
    }

    /// The three "why" cards, in v3's order: title, then body.
    pub fn cards(lang: Lang) -> [(&'static str, &'static str); 3] {
        [
            (
                i18n::text(lang, Key::HomeCardSongsTitle),
                i18n::text(lang, Key::HomeCardSongsBody),
            ),
            (
                i18n::text(lang, Key::HomeCardLyricsTitle),
                i18n::text(lang, Key::HomeCardLyricsBody),
            ),
            (
                i18n::text(lang, Key::HomeCardChordsTitle),
                i18n::text(lang, Key::HomeCardChordsBody),
            ),
        ]
    }

    /// The synopsis between the hero and the tables.
    pub fn synopsis(lang: Lang) -> &'static str {
        i18n::text(lang, Key::HomeSynopsis)
    }

    /// The first table's title — five newest songs.
    pub fn table_latest(lang: Lang) -> &'static str {
        i18n::text(lang, Key::HomeTableLatest)
    }

    /// The second table's title — five most-viewed songs.
    pub fn table_most_viewed(lang: Lang) -> &'static str {
        i18n::text(lang, Key::HomeTableMostViewed)
    }

    /// The closing block.
    pub fn foot_title(lang: Lang) -> &'static str {
        i18n::text(lang, Key::HomeFootTitle)
    }

    /// The line under it.
    pub fn foot_text(lang: Lang) -> &'static str {
        i18n::text(lang, Key::HomeFootText)
    }

    /// The home page's `<meta name="description">`.
    ///
    /// The hero's standfirst, then the tagline — the sentence the front page does
    /// not show, which says what the site is. 125 characters in French, 103 in
    /// English, inside the
    /// [`DESCRIPTION_MAX`](crate::domain::song::DESCRIPTION_MAX) a snippet is
    /// read at.
    ///
    /// **Not v3's**, and that is the fix: v3 pasted the synopsis, then the first
    /// card's body, then the tagline into this field — 842 characters, of which
    /// a search engine shows the first ~155, so every result for the front door
    /// led with two paragraphs that never said "chansons tahitiennes". The prose
    /// itself is untouched: it is still the page, [`synopsis`] and all.
    ///
    /// Assembled from the two keys rather than written out, because the pieces
    /// and the description cannot drift if there is only one of each;
    /// `the_description_is_the_copy_it_claims_to_be` is what says so.
    pub fn description(lang: Lang) -> String {
        format!(
            "{} — {}",
            hero_subtitle(lang),
            i18n::text(lang, Key::HomeTagline)
        )
    }
}

/// How many songs each table lists. v3 asked both for five.
///
/// `pub(crate)` because the same page has a second representation: the Markdown
/// document in `routes/negotiation.rs` shows the same two tables, and a second
/// table size would make the two forms of one page disagree about it.
pub(crate) const ROWS: i64 = 5;

/// `/faariiraa` — the front page's own address, and the path `#[page]` below
/// declares.
///
/// A constant as well as an attribute, because the layout matches the request
/// path against it to decide this page's `<head>`: `#[page]` is a macro over a
/// literal and cannot take one, so the constant restates it — the same
/// arrangement `pages::songs::PATH` and `pages::editor::PATH` have.
///
/// The Tahitian spelling, and the one the site is published at: the reviewer's
/// list of addresses (`/faariiraa`, `/himene`, `/taata-himene`, `/puta-himene`,
/// `/paimi`, `/tauturu`) and the URL standard he named for this one. `/aepa`,
/// v3's transliteration of the same word, moved here for good — see
/// [`crate::pages::retired`].
pub const PATH: &str = "/faariiraa";

/// `/` — the root, which serves the front page and names [`PATH`] canonical.
///
/// Kept as a route rather than redirected: the root is where a reader who knows
/// the domain and nothing else lands, and a domain that answers 200 with its own
/// front page is a domain that works. What it does *not* do is compete with
/// [`PATH`]: its `<head>` names the canonical address, so the two URLs are one
/// page to a crawler rather than two.
pub const ROOT: &str = "/";

/// `/faariiraa` — the front page.
#[page("/faariiraa")]
pub async fn home() -> Result<impl View> {
    Ok(view! { home_body() })
}

/// `/` — the same page under the root, which canonicalises to [`PATH`].
#[page("/")]
pub async fn root() -> Result<impl View> {
    Ok(view! { home_body() })
}

/// The body both routes render.
///
/// Named `home_body` rather than `body` on purpose: `#[page]` expands to code
/// that names Topcoat's `Body` type, and a component called `body` collides with
/// it.
///
/// A `#[component]` rather than a plain helper: `view!` needs the request
/// context, which `#[page]` and `#[component]` bind and a bare `async fn` does
/// not.
///
/// **No `canonical` parameter.** v3 threaded one down from the route to decide
/// whether to emit `<link rel="canonical">`. v4's layout decides that from the
/// request path, so both routes render exactly the same page and neither has to
/// know which one it is.
#[component]
pub async fn home_body(cx: &Cx) -> Result<impl View> {
    let pool = state::db(cx).pool();
    let lang = i18n::resolve(cx);
    let latest = db::songs(pool, SongOrder::Newest, Some(ROWS)).await?;
    let most_viewed = db::songs(pool, SongOrder::MostViewed, Some(ROWS)).await?;
    let songs_link = href!(songs::songs);
    // The chrome's links are the pages' own addresses — one URL per page, so
    // there is no language to put in front of them.
    let songs_href = songs_link.resolve(cx);
    let editor_href = crate::pages::editor::PATH.to_owned();

    Ok(view! {
        <div class=(theme::PAGE)>
            <section class=(theme::HERO)>
                <h1 class=(theme::HERO_TITLE)>(copy::HERO_TITLE)</h1>
                <p class=(theme::HERO_SUBTITLE)>(copy::hero_subtitle(lang))</p>
                <a
                    href=(songs_href)
                    class=(class!(theme::BUTTON_PRIMARY, theme::FOCUS))
                >
                    (i18n::text(lang, Key::HomeDiscover))
                </a>
            </section>

            <section class=(theme::CARD_GRID)>
                for &(title, text) in copy::cards(lang).iter() {
                    <div class=(theme::CARD_ROOMY)>
                        <h2 class=(theme::PANEL_TITLE)>(title)</h2>
                        <p>(text)</p>
                    </div>
                }
            </section>

            <section class="mb-20">
                <p class=(theme::SYNOPSIS)>(copy::synopsis(lang))</p>
            </section>

            // "Les dernières ajouts" and "Les plus vues". The same panel, the
            // same five rows, two orderings — and deliberately not the same
            // column order. v3 led the newest table with the lyric line and the
            // most-viewed table with the title; kept.
            <section class=(theme::TABLE_SECTION)>
                <div class=(class!(theme::TABLE_PANEL, "md:text-right", "md:ml-auto"))>
                    <h2 class=(theme::PANEL_TITLE)>(copy::table_latest(lang))</h2>
                    <table class="max-md:mr-auto ml-auto">
                        <tbody>
                            for song in latest {
                                song_row(song: song, inverse: true)
                            }
                        </tbody>
                    </table>
                </div>
            </section>

            <section class=(theme::TABLE_SECTION)>
                <div class=(theme::TABLE_PANEL)>
                    <h2 class=(theme::PANEL_TITLE)>(copy::table_most_viewed(lang))</h2>
                    <table class="max-md:mx-auto">
                        <tbody>
                            for song in most_viewed {
                                song_row(song: song)
                            }
                        </tbody>
                    </table>
                </div>
            </section>

            <section class=(theme::FOOT_SECTION)>
                <h2 class=(theme::FOOT_TITLE)>(copy::foot_title(lang))</h2>
                <p class=(theme::FOOT_TEXT)>(copy::foot_text(lang))</p>
                <div class=(theme::FOOT_LINKS)>
                    <a
                        href="https://facebook.com/chansondufenua"
                        class=(class!(theme::BUTTON_LIGHT, theme::FOCUS))
                    >
                        (copy::FOOT_FACEBOOK)
                    </a>
                    // v3's second closing button, pointed at the same URL: the
                    // create-song page. That page arrives in step 9; until then
                    // the layout's branded 404 catches the gap.
                    <a
                        href=(editor_href)
                        class=(class!(theme::BUTTON_OUTLINE, theme::FOCUS))
                    >
                        (i18n::text(lang, Key::HomeStart))
                    </a>
                </div>
            </section>
        </div>
    })
}

/// One row of either table.
///
/// `inverse` chooses which column holds the lyric line: the newest table leads
/// with it, the most-viewed table leads with the title — v3's arrangement, kept.
///
/// The whole chord-free lyric is in the markup. v3 did not truncate on the
/// server either; the truncation utility in [`theme::TABLE_LYRICS`] is what
/// makes it one line, and the per-breakpoint width there is what gives it
/// something to cut against.
///
/// The URL comes from the song page's own route through `href!`, so `/himene/`
/// is spelled in exactly one file — see `src/pages/song.rs`. `cx` is not
/// optional for that: resolving a parameterised `href` needs the request.
#[component]
pub async fn song_row(cx: &Cx, song: Song, #[default] inverse: bool) -> Result<impl View> {
    // The address of the sheet, from the route's own parameter — the slug, or the
    // id for a song whose title earned no slug.
    let url = href!(sheet::song, sheet::Slug(song.get_segment())).resolve(cx);
    let title = song.get_title();
    let lyrics = song.clean_lyrics();

    Ok(view! {
        <tr class=(theme::TABLE_ROW)>
            if inverse {
                <td class=(theme::TABLE_LYRICS_CELL)>
                    <a
                        class=(class!(theme::TABLE_LYRICS, theme::LINK))
                        href=(url.clone())
                    >
                        (lyrics.clone())
                    </a>
                </td>
                <td class=(theme::TABLE_CELL)>
                    <a class=(theme::LINK) href=(url.clone())>(title.clone())</a>
                </td>
            } else {
                <td class=(theme::TABLE_CELL)>
                    <a class=(theme::LINK) href=(url.clone())>(title.clone())</a>
                </td>
                <td class=(theme::TABLE_LYRICS_CELL)>
                    <a
                        class=(class!(theme::TABLE_LYRICS, theme::LINK))
                        href=(url.clone())
                    >
                        (lyrics.clone())
                    </a>
                </td>
            }
        </tr>
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::{Db, fixtures};
    use crate::i18n::Lang;

    /// The description is the page's own two sentences, and it fits in a
    /// snippet — in every language the page is served in. Editing the standfirst
    /// or the tagline without editing the description is exactly the drift this
    /// catches, and the length is what v3's 842-character paragraph got wrong.
    ///
    /// The French is pinned to v3's own sentence: it is the one description a
    /// search engine has been reading for years, and a translation pass is not a
    /// rewrite of it.
    #[test]
    fn the_description_is_the_copy_it_claims_to_be() {
        for lang in Lang::ALL {
            let description = copy::description(lang);

            assert_eq!(
                description,
                format!(
                    "{} — {}",
                    copy::hero_subtitle(lang),
                    i18n::text(lang, Key::HomeTagline)
                ),
                "{lang:?}"
            );
            assert!(
                description.chars().count() <= crate::domain::song::DESCRIPTION_MAX,
                "{lang:?}: the description is longer than a snippet"
            );
            // A prose key's Tahitian is the English, so the two agree by rule.
            if lang == Lang::Ty {
                assert_eq!(description, copy::description(Lang::En));
            }
        }

        assert_eq!(
            copy::description(Lang::Fr),
            "L'élégance de la musique polynésienne — Chanson du fenua, retrouvez vos paroles de chanson tahitiennes et polynésiennes."
        );
    }

    /// Three cards, each with something to say — in every language, and with
    /// three different titles, because three cards saying the same thing are one
    /// card drawn three times. A card with an empty body would render as a
    /// heading over blank space and nothing else would notice.
    #[test]
    fn every_card_has_a_title_and_a_body() {
        for lang in Lang::ALL {
            let cards = copy::cards(lang);

            assert_eq!(cards.len(), 3, "v3 has three cards");

            let mut titles = std::collections::BTreeSet::new();
            for (title, text) in cards {
                assert!(!title.trim().is_empty(), "{lang:?}: a card has no title");
                assert!(!text.trim().is_empty(), "card {title:?} has no body");
                assert!(titles.insert(title), "{lang:?}: {title:?} is drawn twice");
            }
        }

        assert_eq!(copy::cards(Lang::Fr)[0].0, "Chansons exquises");
    }

    /// The page's two tables are two *orderings*, not two copies of one.
    ///
    /// With four fixtures this is the cheapest proof available that
    /// [`SongOrder::Newest`] and [`SongOrder::MostViewed`] actually disagree: if
    /// they agreed, the page would show the same songs twice and no test would
    /// say so.
    #[tokio::test]
    async fn the_two_tables_are_different_orderings() {
        let db = Db::open_in_memory().await.expect("in-memory database");
        fixtures::seed(db.pool()).await.expect("seed fixtures");

        let latest = db::songs(db.pool(), SongOrder::Newest, Some(ROWS))
            .await
            .expect("newest");
        let most_viewed = db::songs(db.pool(), SongOrder::MostViewed, Some(ROWS))
            .await
            .expect("most viewed");

        assert_eq!(latest.len(), fixtures::SONGS.len());
        assert_ne!(
            latest[0].get_id(),
            most_viewed[0].get_id(),
            "the first row of each table would be the same song"
        );

        // The most-viewed table's leader is the fixture with the highest count,
        // not merely some other song.
        let busiest = fixtures::SONGS
            .iter()
            .max_by_key(|song| song.view_count)
            .expect("a fixture")
            .id;
        assert_eq!(most_viewed[0].get_id(), busiest);
    }
}
