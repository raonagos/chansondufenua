//! Search — `/paimi`.
//!
//! Transcribed from v3's `Search` page. One address, two states: the form, and
//! the results when the URL carries a needle. `/paimi` is *pā'imi*, to search —
//! the reviewer's own word for this page, and one of the six addresses he named.
//!
//! One box that searches the two things a reader knows about a song: its title
//! and who sang it. The matching is accent- and ʻokina-insensitive, because that
//! is how the corpus is written — `Māmā Tahiti`, `'Āhani e`, `Te ta''ata hara
//! nei` — and how nobody types: a reader looking for `'Āhani e` types `ahani`,
//! and the index folds it ([`crate::db::search_songs`]).
//!
//! # The query is the address
//!
//! `/paimi?q=ahani` — a `GET` form on this page, and the same parameter read
//! back from the URL. That is the multi-lyric page's pattern
//! ([`crate::pages::book`]) for the same reason: a reader can keep, print or
//! send the result, and the page needs no script to do it.
//!
//! # The head is `noindex`, and there is no canonical
//!
//! Decided here, once, both ways, and the same decision the multi-lyric page
//! makes: the URL space is every string a person could type — unbounded, mostly
//! empty, and every non-empty subset of it a duplicate of the catalogue it
//! quotes. An index full of those competes with the song pages they point at, so
//! the page carries `noindex, follow` and **no canonical** (a canonical on a page
//! a crawler is told not to index names a preferred address for nothing). The
//! songs and the artists in the results are links to pages that *are* canonical
//! and *are* indexable, which is where the value is.
//!
//! # Three forms, joined by the `Link` headers
//!
//! A *search* — a URL with a needle in it — has all three of the site's
//! representations: this HTML page, the same results as one Markdown document
//! (`routes::negotiation::search_document`), and the same rows as JSON at
//! [`crate::routes::api`]'s `/api/search?q=…`. The bare page is the form and has
//! no Markdown twin, the same rule the create-song form and the multi-lyric
//! picker follow: a form is not prose.
//!
//! Note the naming constraint every page in this directory shares:
//! `#[page("/paimi")]` emits a unit struct named after its handler —
//! `search` — in this module's *type* namespace, so a local binding of that
//! name would be read as a pattern matching it.

use sqlx::SqlitePool;
use topcoat::{
    Result,
    context::Cx,
    router::{page, request::uri},
    view::{View, class, view},
};

use crate::db::{self, DbResult};
use crate::domain::{Artist, Song};
use crate::i18n::{self, Key};
use crate::pages::artist;
use crate::pages::songs::song_row;
use crate::state;
use crate::ui::theme;

/// `/paimi` — the address, in one place.
///
/// `routes::language` decides whether a request path is language-scoped, and this
/// is the constant it reads. `#[page]` cannot take a constant — it is a macro
/// over a literal path — so this restates it, as `pages::songs::PATH` does.
pub const PATH: &str = "/paimi";

/// The query parameter the needle travels in.
///
/// One letter in the URL and one word in the form: the form's field is `name="q"`
/// and this reader reads `q`, so the two cannot drift about what a search is
/// called.
pub const PARAM: &str = "q";

/// How many songs and how many artists one search answers with.
///
/// Twenty, the index's own page size: a search that listed more than a page of a
/// catalogue this size would be a second, worse index rather than a shortcut to
/// the first.
pub const LIMIT: i64 = 20;

/// What a search found, in the two halves the page shows.
///
/// One type because three callers need the same pair — the page, the Markdown
/// document and the JSON read — and three separate pairs of calls is how the
/// HTML and the JSON come to disagree about what the URL named.
pub struct Results {
    pub songs: Vec<Song>,
    pub artists: Vec<Artist>,
}

/// Run one search: the catalogue's titles and the artists' names.
///
/// The one place a needle becomes rows, so the page, its Markdown twin and
/// `/api/search` answer the same question the same way.
pub async fn run(pool: &SqlitePool, needle: &str) -> DbResult<Results> {
    Ok(Results {
        songs: db::search_songs(pool, needle, LIMIT).await?,
        artists: db::search_artists(pool, needle, LIMIT).await?,
    })
}

/// The raw `q` value a query string carries, exactly as it arrived.
///
/// Raw, because the value is also what the published URLs carry
/// ([`query_string`]): re-encoding a decoded needle would give the page a second
/// spelling of the URL the reader is already on.
fn raw(query: &str) -> Option<&str> {
    query
        .split('&')
        .filter(|pair| !pair.is_empty())
        .find_map(|pair| {
            let (name, value) = pair.split_once('=')?;
            (name == PARAM && !value.trim().is_empty()).then_some(value)
        })
}

/// The needle a search is about, or `None` when the page is the form.
///
/// Percent-decoded and trimmed: `?q=ahani`, `?q=ahani+`, and `?q=%27Ahani` are
/// one search, and a value that decodes to nothing but space is no search at all
/// rather than a search for nothing.
pub fn needle(query: &str) -> Option<String> {
    let decoded = decode(raw(query)?);
    let trimmed = decoded.trim();

    (!trimmed.is_empty()).then(|| trimmed.to_owned())
}

/// The `q=…` string the page publishes for the search a query string names.
///
/// The raw value again, so the `Link` header names the URL the reader is on
/// rather than a re-encoded near-miss of it. `None` when the query names no
/// search: a form has no machine-readable form to describe.
///
/// The result is safe in a header because its input is: the query string came
/// from a request's URI, which the HTTP parser allows no space or control
/// character into.
pub fn query_string(query: &str) -> Option<String> {
    let value = raw(query)?;
    needle(query)?;

    Some(format!("{PARAM}={value}"))
}

/// Percent-decode a query value, treating `+` as a space.
///
/// Hand-rolled like the other query readers in this crate (`book::selection`,
/// `chord::offset`, `routes::language::explicit`): this runs before any extractor
/// and wants one raw value, not a typed struct, and the alternative is a
/// dependency for twenty lines of ASCII. A malformed escape (`%zz`, a trailing
/// `%`) is kept as the literal character it was rather than being dropped — the
/// search then answers "nothing found", which is the honest answer to a needle
/// that spells nothing.
fn decode(value: &str) -> String {
    let bytes = value.as_bytes();
    let mut out: Vec<u8> = Vec::with_capacity(bytes.len());
    let mut index = 0;

    while index < bytes.len() {
        match bytes[index] {
            b'%' if index + 2 < bytes.len() => {
                match (hex(bytes[index + 1]), hex(bytes[index + 2])) {
                    (Some(high), Some(low)) => {
                        out.push(high * 16 + low);
                        index += 3;
                    }
                    _ => {
                        out.push(b'%');
                        index += 1;
                    }
                }
            }
            b'+' => {
                out.push(b' ');
                index += 1;
            }
            byte => {
                out.push(byte);
                index += 1;
            }
        }
    }

    // Lossy on purpose: a lone `%E4` is not a character, and a search for the
    // bytes that remain is a better answer than a 500.
    String::from_utf8_lossy(&out).into_owned()
}

/// One hex digit, if the byte is one.
fn hex(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        b'A'..=b'F' => Some(byte - b'A' + 10),
        _ => None,
    }
}

/// `/paimi` — the form, and the results when the URL carries a needle.
///
/// One page, two states, decided by the query string and nothing else: no
/// needle renders the form, a needle renders what it found. They are one address
/// because the reader builds the URL in the form and reads it back on the page,
/// the same arrangement `/puta-himene` makes.
#[page("/paimi")]
pub async fn search(cx: &Cx) -> Result<impl View> {
    let lang = i18n::resolve(cx);
    let query = uri(cx).query().unwrap_or("");

    // `value` is what the form shows back: the needle the reader typed, so a
    // search can be refined rather than retyped. Empty on the bare page.
    let value = needle(query).unwrap_or_default();
    let found = match needle(query) {
        Some(needle) => Some(run(state::db(cx).pool(), &needle).await?),
        None => None,
    };

    Ok(view! {
        <div class=(theme::PAGE)>
            <h1 class=(theme::H1)>(i18n::text(lang, Key::SearchTitle))</h1>

            <form method="get" action=(i18n::link(cx, PATH)) class=(theme::FORM_PANEL)>
                <div class=(theme::FORM_FIELD)>
                    <label class=(theme::FORM_LABEL) for="q">
                        (i18n::text(lang, Key::SearchLabel))
                    </label>
                    <input
                        id="q"
                        type="search"
                        name=(PARAM)
                        value=(value)
                        class=(class!(theme::FORM_INPUT, theme::FOCUS))
                    />
                </div>
                <div class=(class!(theme::FORM_SUBMIT, theme::BOOK_ENTRY))>
                    <button type="submit" class=(class!(theme::FORM_SUBMITTER, theme::FOCUS))>
                        (i18n::text(lang, Key::SearchSubmit))
                    </button>
                </div>
            </form>

            match found {
                Some(found) => {
                    if found.songs.is_empty() && found.artists.is_empty() {
                        <p class=(theme::LEAD)>(i18n::text(lang, Key::SearchEmpty))</p>
                    } else {
                        if !found.songs.is_empty() {
                            <h2 class=(theme::PANEL_TITLE)>
                                (i18n::text(lang, Key::SearchSongs))
                            </h2>
                            <div class=(theme::INDEX_PANEL)>
                                <table class="w-full">
                                    <thead>
                                        <tr>
                                            <th class=(theme::INDEX_HEAD)>
                                                (i18n::text(lang, Key::IndexColumnTitle))
                                            </th>
                                            <th class=(class!(
                                                theme::INDEX_HEAD,
                                                theme::INDEX_COLUMN_ARTIST,
                                            ))>
                                                (i18n::text(lang, Key::IndexColumnArtist))
                                            </th>
                                        </tr>
                                    </thead>
                                    <tbody>
                                        for song in found.songs {
                                            song_row(song: song)
                                        }
                                    </tbody>
                                </table>
                            </div>
                        }
                        if !found.artists.is_empty() {
                            <h2 class=(theme::PANEL_TITLE)>
                                (i18n::text(lang, Key::SearchArtists))
                            </h2>
                            <ul class=(theme::LEAD)>
                                for artist in found.artists {
                                    <li>
                                        <a href=(artist_link(cx, &artist)) class=(theme::LINK)>
                                            (artist.get_fullname())
                                        </a>
                                    </li>
                                }
                            </ul>
                        }
                    }
                },
                None => "",
            }
        </div>
    })
}

/// An artist's page, in the request's language — [`artist::link`]'s, so a search
/// result and a song's credits point at the same address.
pub fn artist_link(cx: &Cx, artist: &Artist) -> String {
    artist::link(cx, artist)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The parameter the form writes is the parameter the reader reads: a form
    /// naming anything else would submit a query this page reads as "no search".
    #[test]
    fn a_search_is_named_by_one_parameter() {
        assert_eq!(PARAM, "q");
    }

    /// A needle reaches the page from every spelling a browser or a client can
    /// send — encoded, plus-encoded, with junk beside it — and decodes to one
    /// string. Nothing at all, and whitespace, are the form.
    #[test]
    fn the_needle_is_read_and_decoded_from_wherever_it_sits() {
        assert_eq!(needle("q=ahani").as_deref(), Some("ahani"));
        assert_eq!(needle("q=%27%C4%80hani+e").as_deref(), Some("'Āhani e"));
        assert_eq!(needle("lang=ty&q=ahani").as_deref(), Some("ahani"));
        assert_eq!(needle("q=a+b&tr=2").as_deref(), Some("a b"));

        for query in ["", "q", "q=", "q=+", "q=%20", "q=   ", "other=ahani"] {
            assert_eq!(needle(query), None, "{query:?} is not a search");
        }
    }

    /// A malformed escape is kept literally rather than dropped, and a lone
    /// `%` at the end of a value is the `%` it is.
    #[test]
    fn a_malformed_escape_is_not_a_crash() {
        assert_eq!(needle("q=%zz").as_deref(), Some("%zz"));
        assert_eq!(needle("q=100%").as_deref(), Some("100%"));
        assert_eq!(decode("%"), "%");
        assert_eq!(decode("%2"), "%2");
        assert_eq!(decode("%C4%81"), "ā");
    }

    /// The query string the page publishes is the reader's own, so a `Link`
    /// header names the URL the response arrived on.
    #[test]
    fn the_published_query_is_the_one_that_arrived() {
        assert_eq!(
            query_string("q=%27%C4%81hani+e").as_deref(),
            Some("q=%27%C4%81hani+e")
        );
        assert_eq!(query_string("lang=ty&q=ahani").as_deref(), Some("q=ahani"));
        assert_eq!(query_string("q="), None);
        assert_eq!(query_string("tr=2"), None);
    }

    /// The search answers both halves from one needle, and a needle that matches
    /// nothing is two empty lists rather than an error.
    #[tokio::test]
    async fn one_needle_answers_with_songs_and_artists() {
        let db = crate::db::Db::open_in_memory()
            .await
            .expect("in-memory database");
        crate::db::fixtures::seed(db.pool())
            .await
            .expect("seed fixtures");

        let found = run(db.pool(), "mama").await.expect("the read");
        assert!(
            found
                .songs
                .iter()
                .any(|song| song.get_title() == "Māmā Tahiti"),
            "the title search lost the accent"
        );

        let nobody = run(db.pool(), "zzzzzz").await.expect("the read");
        assert!(nobody.songs.is_empty());
        assert!(nobody.artists.is_empty());
    }
}
