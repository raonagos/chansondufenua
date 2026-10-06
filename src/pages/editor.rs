//! The create-song page — `/himene/api`.
//!
//! Transcribed from v3's `CreateSongPage`
//! (`app/src/pages/himene/createsong.rs`): a title, a credit field fed by a
//! `<datalist>` of every artist, and a lyric editor with a chord palette over
//! it.
//!
//! Five things differ from v3, each deliberate:
//!
//! * **It is one route with two methods, not a page plus a server function.**
//!   v3's `<ActionForm>` posted to a Leptos `#[server]` endpoint that returned
//!   a serialized `Song` and set a `Location` header for the browser to follow.
//!   Here the page itself answers `POST` and replies `303` — the same
//!   Post/Redirect/Get shape, without the second protocol. A submission that the
//!   domain rejects is answered `400` *with the form filled back in*, where v3
//!   answered a bare error and lost everything the author had typed.
//! * **The chord tools are ordinary JavaScript, in the page.** v3's were Rust
//!   compiled to WebAssembly and hydrated over the server's markup; v4 is
//!   server-rendered with no client build step, so the same behaviour is ~50
//!   lines of vanilla script. Nothing about the site depends on it running: it
//!   is the editor, and an editor needs a browser.
//! * **The artist field is one `<input>`, with the chips drawn from it.** v3
//!   kept a hidden field and an empty visible one, which meant a submission from
//!   a browser with the script switched off carried no artists at all. Here the
//!   field the browser posts is the field a person typed in, and the chips are a
//!   view of its value — pressing one takes a name back out of it.
//! * **A chord is inserted at the caret, in flow.** v3 replaced the selected
//!   range with the chord, which deleted whatever was selected, and positioned
//!   the chords absolutely inside the editable area — so the caret and the chord
//!   were never quite in the same place. The *sheet* still draws a chord above
//!   its syllable; the editor is where words are typed, so it says "this is a
//!   chord" in colour and monospace and leaves the geometry alone.
//! * **A chord is ordinary content of the editable area.** v3 intercepted
//!   `keydown` while the caret sat inside a `<sup>`: it swallowed the keystroke,
//!   and read `Backspace`/`Delete` there as "remove the chord". Here a chord is a
//!   node like any other — a click puts the caret inside it, a keystroke extends
//!   its text, and backspacing past one removes it, which is what the browser
//!   already does. v3 also tagged a custom chord with `data-god`, a marker no
//!   stylesheet reads; that one is simply gone.
//!
//! The route is `/himene/api` because v3's was: the home page's closing button
//! and every song sheet link to it, and those URLs are in the wild.
//!
//! Note the naming constraint every page in this directory shares:
//! `#[page(…)]` emits a unit struct named after its handler, in this module's
//! *type* namespace, so a local binding called `editor` would be read as a
//! pattern matching it.

use topcoat::{
    Result,
    context::Cx,
    router::{
        Body, Method, StatusCode,
        content::Form,
        error::see_other,
        href, page,
        request::{FromRequest, method},
    },
    view::{Unescaped, View, class, component, view},
};

use serde::Deserialize;

use crate::db::{self, DbError};
use crate::domain::{AppError, Artist, sanitise_lyrics};
use crate::i18n::{self, Key};
use crate::log;
use crate::pages::song as sheet;
use crate::state;
use crate::ui::theme;

/// The route, in one place.
///
/// The `#[page]` attribute cannot take a constant — it is a macro over a literal
/// path — so this restates what the attribute declares and is what every other
/// reference to the page uses: the form's own `action`, and the layout's title
/// rule. Two spellings of a URL are one too many.
pub const PATH: &str = "/himene/api";

/// The root chords the palette offers, in v3's order.
///
/// A fixed list, not a query: a chord is a property of music theory, not of this
/// database. The seven naturals are the buttons; anything else is typed into the
/// custom field beside them.
const CHORDS: [&str; 7] = ["C", "D", "E", "F", "G", "A", "B"];

/// What the browser posts.
///
/// Every field defaults, so that a submission missing one is a *validation*
/// answer — with the form handed back — rather than a deserialization error with
/// nothing to show for it. The names are the form's own: `title`, `artists`,
/// `lyrics`.
#[derive(Debug, Default, Clone, Deserialize)]
pub struct NewSong {
    /// The song's title.
    #[serde(default)]
    pub title: String,
    /// The credit field: fullnames separated by commas, which is exactly what
    /// [`db::create_song`] consumes.
    #[serde(default)]
    pub artists: String,
    /// The lyric, as the editor's `innerHTML`: one `<div>` per line, one `<sup>`
    /// per chord.
    #[serde(default)]
    pub lyrics: String,
}

/// `/himene/api` — the create-song form, and the submission it posts.
///
/// A `GET` renders the empty form; a `POST` creates the song or hands the form
/// back. The two share a body because they share everything worth sharing: the
/// page, the fields, and the artist list the credit field autocompletes against.
///
/// The body is read **only** for a `POST`. `Form` on a `GET` reads the *query
/// string*, and `/himene/api?lang=ty` is a query string that is not a song.
/// A page whose attribute names more than one method takes **no comma** between
/// the method list and the path — `#[page([GET, POST] "/himene/api")]`. The
/// documented `#[page([GET, POST], "/…")]` form does not compile in 0.10:
/// the attribute parser reads the methods, then peeks for the path literal, and
/// a comma is not part of either production.
#[page([GET, POST] "/himene/api")]
pub async fn editor(cx: &Cx, body: Body) -> Result<impl View> {
    // What a rejected submission is handed back with. On the common path — a
    // `GET`, or a `POST` that worked — these stay at their defaults and cost
    // nothing.
    let mut input = NewSong::default();
    let mut rejected = false;

    if method(cx) == Method::POST {
        let Form(posted) = Form::<NewSong>::from_request(cx, body).await?;

        match db::create_song(
            state::db(cx).pool(),
            &posted.title,
            &posted.lyrics,
            &posted.artists,
        )
        .await
        {
            // 303, not v3's 302. Both are followed with a `GET` by every browser
            // that matters, and 303 is the code that *says* so — the whole point
            // of the redirect is that the browser must not re-post the form.
            //
            // The address is the song's own, which is now the slug the write
            // minted for it (`db::create_song` → `queries::assign_slug`): the
            // author lands on the URL the song is published at rather than being
            // sent through a second redirect from the id.
            Ok(song) => {
                let url = href!(sheet::song, sheet::Slug(song.get_segment())).resolve(cx);
                return Err(see_other(url).into());
            }
            Err(DbError::Domain(cause @ AppError::Invalid { .. })) => {
                // The domain said no, which is the author's problem and not a
                // server fault — so a 400, and the form goes back with the
                // submission still in it.
                //
                // The reason is deliberately not shown. `Song::validate` writes
                // it for a log line (`expected 100..=6000 characters, got 42`),
                // and the page names the two fields a person can fix instead.
                //
                // To the journal, not to the response: the access layer logs the
                // `400`, and this line is the *why* behind it — which no status
                // code carries.
                log::warn(format_args!("create-song rejected: {cause}"));
                input = posted;
                rejected = true;
            }
            Err(error) => return Err(error.into()),
        }
    }

    let artists = db::artists(state::db(cx).pool()).await?;

    Ok(view! {
        form_body(artists: artists, input: input, rejected: rejected)
    })
}

/// The form itself.
///
/// A `#[component]` rather than a plain helper, because `view!` needs the request
/// context and only a component or a page binds one.
#[component]
pub async fn form_body(
    cx: &Cx,
    artists: Vec<Artist>,
    input: NewSong,
    rejected: bool,
) -> Result<impl View> {
    let lang = i18n::resolve(cx);

    // What was typed, sanitised — on the way *back* to the browser it is no more
    // trustworthy than it was on the way in. Empty on the first render, which is
    // the common case and costs one `ammonia::clean`.
    //
    // It seeds the *hidden field*, not the editable area: the script copies the
    // field into the editor when it starts, so the editor never sees a value the
    // domain has not already refused or accepted. Without the script there is
    // still a lyric in the form — it is simply not being edited.
    let lyrics = sanitise_lyrics(&input.lyrics);

    Ok(view! {
        <div class=(theme::PAGE)>
            <h1 class=(theme::H1)>(i18n::text(lang, Key::AddLyrics))</h1>

            if rejected {
                // The status and the sentence are one decision, which is why they
                // are one branch: `Song::validate` reports one rule at a time, and
                // a form that says "no" without saying which way to go next is
                // where v3 left its author.
                (StatusCode::BAD_REQUEST)
                <p class=(theme::FORM_ERROR) role="alert">
                    (i18n::text(lang, Key::SaveError))
                </p>
            }

            <form method="post" action=(PATH) class=(theme::FORM_PANEL)>
                <div class=(theme::FORM_FIELD)>
                    <label for="song-title" class=(theme::FORM_LABEL)>
                        (i18n::text(lang, Key::IndexColumnTitle))
                    </label>
                    <input
                        id="song-title"
                        name="title"
                        type="text"
                        autocomplete="off"
                        class=(class!(theme::FORM_INPUT, theme::FOCUS))
                        value=(input.title.clone())
                    />
                </div>

                <div class=(theme::FORM_FIELD)>
                    <label for="song-artists" class=(theme::FORM_LABEL)>
                        (i18n::text(lang, Key::IndexColumnArtist))
                    </label>
                    <input
                        id="song-artists"
                        name="artists"
                        type="text"
                        list="artist-list"
                        autocomplete="off"
                        class=(class!(theme::FORM_INPUT, theme::FOCUS))
                        value=(input.artists.clone())
                    />
                    // Every artist in the catalogue, for the browser's own
                    // autocomplete. The list is the whole table rather than the
                    // top ten: an artist who is not in it is not in it for a
                    // reason, and a truncated one invites a near-duplicate name.
                    //
                    // `db::search_artists` exists for exactly this and is not
                    // used here: it is for a client that queries as the author
                    // types, and there is no such client. The whole list costs
                    // one indexed read and no round trips.
                    <datalist id="artist-list">
                        for artist in artists {
                            <option value=(artist.get_fullname())></option>
                        }
                    </datalist>
                    <ul id="artist-chips" class=(theme::FORM_CHIPS)></ul>
                    // The chip the script clones for each picked name. A
                    // `<template>` rather than a class string built in the
                    // script: Tailwind reads the crate's *text* for class names,
                    // and a class assembled at runtime is a class the stylesheet
                    // never carries.
                    <template id="artist-chip">
                        <li class=(theme::FORM_CHIP)>
                            <span></span>
                            <button
                                type="button"
                                class=(class!(theme::FORM_CHIP_DELETE, theme::FOCUS))
                                aria-label=(i18n::text(lang, Key::RemoveArtist))
                            >
                                <svg
                                    xmlns="http://www.w3.org/2000/svg"
                                    fill="none"
                                    viewBox="0 0 24 24"
                                    stroke-width="1.5"
                                    stroke="currentColor"
                                    class="size-6"
                                    aria-hidden="true"
                                >
                                    <path stroke-linecap="round" stroke-linejoin="round" d="M6 18 18 6M6 6l12 12"></path>
                                </svg>
                            </button>
                        </li>
                    </template>
                </div>

                <div class=(theme::FORM_FIELD)>
                    <label for="lyrics-editor" class=(theme::FORM_LABEL)>
                        (i18n::text(lang, Key::FieldLyrics))
                    </label>
                    <div class=(theme::FORM_CHORD_ROW)>
                        for chord in CHORDS {
                            <button
                                type="button"
                                data-chord=(chord)
                                class=(class!(theme::FORM_CHORD_BUTTON, theme::FOCUS))
                            >
                                (chord)
                            </button>
                        }
                        // The three modifiers, v3's, in v3's order and with v3's
                        // letters. They are *state* for the next chord button
                        // pressed, read by the script and not posted with the
                        // form: what the form carries is the lyric, chords and
                        // all.
                        <label for="chord-minor" class=(theme::FORM_CHBX_LABEL)>
                            <span>"m"</span>
                            <input id="chord-minor" type="checkbox"/>
                        </label>
                        <label for="chord-diez" class=(theme::FORM_CHBX_LABEL)>
                            <span>"#"</span>
                            <input id="chord-diez" type="checkbox"/>
                        </label>
                        <label for="chord-bemol" class=(theme::FORM_CHBX_LABEL)>
                            <span>"b"</span>
                            <input id="chord-bemol" type="checkbox"/>
                        </label>
                        <div class=(theme::FORM_CUSTOM)>
                            <input
                                id="chord-custom"
                                type="text"
                                autocomplete="off"
                                placeholder="Sol7b"
                                class=(class!(theme::FORM_CUSTOM_INPUT, theme::FOCUS))
                            />
                            <button
                                id="chord-custom-add"
                                type="button"
                                class=(class!(theme::FORM_CUSTOM_BUTTON, theme::FOCUS))
                                aria-label=(i18n::text(lang, Key::AddChord))
                            >
                                <svg
                                    xmlns="http://www.w3.org/2000/svg"
                                    fill="none"
                                    viewBox="0 0 24 24"
                                    stroke-width="1.5"
                                    stroke="currentColor"
                                    class="size-6"
                                    aria-hidden="true"
                                >
                                    <path stroke-linecap="round" stroke-linejoin="round" d="M12 4.5v15m7.5-7.5h-15"></path>
                                </svg>
                            </button>
                        </div>
                    </div>

                    // What the form posts. A `contenteditable` is a view of the
                    // lyric, not a field: a form control has a value and this has
                    // markup, so the script mirrors one into the other on every
                    // keystroke — v3's arrangement, kept, and the reason a
                    // submission needs a browser.
                    <input id="lyrics-value" type="hidden" name="lyrics" value=(lyrics)/>
                    <div
                        id="lyrics-editor"
                        contenteditable="true"
                        spellcheck="false"
                        class=(theme::FORM_EDITOR)
                    ></div>
                </div>

                <div class=(theme::FORM_SUBMIT)>
                    <button type="submit" class=(class!(theme::FORM_SUBMITTER, theme::FOCUS))>
                        (i18n::text(lang, Key::Save))
                    </button>
                </div>
            </form>

            <script type="text/javascript">(Unescaped::new_unchecked(EDITOR_JS))</script>
        </div>
    })
}

/// The editor's script.
///
/// Unescaped into the page, like the JSON-LD block in the layout, because a
/// `<script>` is a raw-text element: entity-escaping the text would leave the
/// browser showing it rather than running it. Nothing here is derived from a
/// request, so there is nothing in it to escape.
///
/// It does three things, all of them editor-only: mirror the editable area into
/// the hidden field, insert a chord at the caret, and draw the artist chips from
/// the credit field. Every lookup is guarded — the script is also served on a
/// page whose editor was rejected, and a script that throws on a missing node
/// would take the rest of the page with it.
const EDITOR_JS: &str = r##"
(function () {
  var area = document.getElementById("lyrics-value");
  var box = document.getElementById("lyrics-editor");
  if (!area || !box) return;

  var sync = function () { area.value = box.innerHTML; };

  var insert = function (text) {
    var chord = document.createElement("sup");
    chord.setAttribute("data-nosnippet", "true");
    chord.appendChild(document.createTextNode(text));

    var selection = window.getSelection();
    var range = selection && selection.rangeCount ? selection.getRangeAt(0) : null;

    if (!range || !box.contains(range.commonAncestorContainer)) {
      box.appendChild(chord);
    } else {
      range.deleteContents();
      range.insertNode(chord);
      range.setStartAfter(chord);
      range.collapse(true);
      selection.removeAllRanges();
      selection.addRange(range);
    }

    sync();
    box.focus();
  };

  var named = function (base) {
    var minor = document.getElementById("chord-minor");
    var bemol = document.getElementById("chord-bemol");
    var diez = document.getElementById("chord-diez");

    if (bemol && bemol.checked) base += "b";
    else if (diez && diez.checked) base += "#";
    if (minor && minor.checked) base += "m";

    return base;
  };

  box.innerHTML = area.value;
  box.addEventListener("input", sync);

  var palette = box.parentNode.querySelectorAll("[data-chord]");
  for (var i = 0; i < palette.length; i++) {
    palette[i].addEventListener("click", function () {
      insert(named(this.getAttribute("data-chord")));
    });
  }

  var custom = document.getElementById("chord-custom");
  var customAdd = document.getElementById("chord-custom-add");
  var customInsert = function () {
    if (!custom) return;
    var value = custom.value.trim();
    if (!value) return;
    insert(value);
    custom.value = "";
    custom.focus();
  };
  if (customAdd) customAdd.addEventListener("click", customInsert);
  if (custom) {
    custom.addEventListener("keydown", function (event) {
      if (event.key !== "Enter") return;
      event.preventDefault();
      customInsert();
    });
  }

  var picker = document.getElementById("song-artists");
  var chips = document.getElementById("artist-chips");
  var template = document.getElementById("artist-chip");
  if (!picker || !chips || !template) return;

  var names = function () {
    return picker.value.split(",").map(function (name) {
      return name.trim();
    }).filter(function (name) { return name.length > 0; });
  };

  var draw = function () {
    while (chips.firstChild) chips.removeChild(chips.firstChild);

    names().forEach(function (name) {
      var chip = template.content.firstElementChild.cloneNode(true);
      var remove = chip.querySelector("button");
      chip.querySelector("span").textContent = name;

      remove.addEventListener("click", function () {
        picker.value = names().filter(function (other) {
          return other !== name;
        }).join(", ");
        draw();
      });

      chips.appendChild(chip);
    });
  };

  picker.addEventListener("change", draw);
  picker.addEventListener("keyup", function (event) {
    if (event.key === "," || event.key === "Enter") draw();
  });
  draw();
})();
"##;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::{Db, fixtures};

    /// The field names are the form's own and the defaults are what make a
    /// half-submitted form a validation answer rather than a parse error. A
    /// missing field must not be able to fail the request before the page can
    /// say what is wrong with it.
    #[test]
    fn every_field_of_the_form_is_optional_to_parse() {
        let posted: NewSong =
            serde_urlencoded_lite("title=Une+chanson&lyrics=%3Cdiv%3Ea%3C%2Fdiv%3E");

        assert_eq!(posted.title, "Une chanson");
        assert_eq!(posted.lyrics, "<div>a</div>");
        assert_eq!(
            posted.artists, "",
            "an absent field is empty, not a failure"
        );
    }

    /// The credit field is comma-separated, and `db::create_song` splits it the
    /// same way — so the round trip through the page cannot invent an artist or
    /// lose one.
    #[test]
    fn the_credit_field_splits_the_way_the_database_does() {
        let names = Artist::split_fullnames("2B Brothers Tahiti, Mahoi, ,Mahoi");

        assert_eq!(names, vec!["2B Brothers Tahiti", "Mahoi", "Mahoi"]);
    }

    /// The reason a rejected submission keeps its lyric: the validator's bounds
    /// start at `LYRICS_MIN` characters, so a title that is one character too
    /// short must not discard a whole verse the author typed.
    #[test]
    fn a_rejected_submission_still_has_its_lyric() {
        let posted = NewSong {
            title: "abc".to_owned(),
            artists: String::new(),
            lyrics: format!("<div>{}</div>", "a".repeat(200)),
        };

        assert!(posted.lyrics.contains("aaaaaaaaaa"));
        // ...and it is handed back sanitised, chords included.
        assert!(
            sanitise_lyrics("<div>x<sup data-nosnippet=\"true\">B</sup></div>")
                .contains("data-nosnippet")
        );
    }

    /// `db::create_song` is what the page calls, and the page's own test above
    /// only proves the field parsing. This is the end of the road: a valid
    /// submission becomes a row whose chords survive the round trip.
    #[tokio::test]
    async fn a_valid_submission_becomes_a_readable_song() {
        let db = Db::open_in_memory().await.expect("in-memory database");
        fixtures::seed(db.pool()).await.expect("seed fixtures");

        let lyrics = format!(
            "<div>Tere<sup data-nosnippet=\"true\">B</sup> mai</div><div>{}</div>",
            "la".repeat(80)
        );
        let created = db::create_song(db.pool(), "Tere mai te here", &lyrics, "Mahoi")
            .await
            .expect("a well-formed submission is accepted");

        assert_eq!(created.get_title(), "Tere mai te here");
        assert_eq!(created.get_artists().len(), 1);
        assert_eq!(created.get_artists()[0].get_fullname(), "Mahoi");

        let read = db::song(db.pool(), &created.get_id())
            .await
            .expect("read it back")
            .expect("it is there");
        assert!(read.lyrics_html().contains("data-nosnippet"));
        assert_eq!(
            read.lyrics_lines()[0][1],
            crate::domain::song::LyricSpan::Chord("B".to_owned())
        );
    }

    /// A submission the domain refuses is an `AppError`, not a database fault —
    /// which is what the page matches on to answer a 400 instead of a 500.
    #[tokio::test]
    async fn a_rejected_submission_is_the_domain_s_error() {
        let db = Db::open_in_memory().await.expect("in-memory database");
        fixtures::seed(db.pool()).await.expect("seed fixtures");

        // Too short for `TITLE_MIN`, which is the first rule checked.
        let rejected = db::create_song(db.pool(), "abc", &"l".repeat(200), "").await;
        assert!(matches!(
            rejected,
            Err(DbError::Domain(AppError::Invalid { field: "title", .. }))
        ));

        // ...and nothing was written: the validation runs before the transaction.
        let songs = db::songs(db.pool(), crate::db::SongOrder::Newest, None)
            .await
            .expect("list");
        assert_eq!(songs.len(), fixtures::SONGS.len());
    }

    /// The smallest URL-decoder the tests need: the crate deliberately does not
    /// carry `serde_urlencoded` in its own dependency list, and the code under
    /// test parses forms through Topcoat's extractor rather than through this.
    fn serde_urlencoded_lite(body: &str) -> NewSong {
        let mut song = NewSong::default();

        for pair in body.split('&') {
            let Some((key, value)) = pair.split_once('=') else {
                continue;
            };
            let value = percent_decode(value);
            match key {
                "title" => song.title = value,
                "artists" => song.artists = value,
                "lyrics" => song.lyrics = value,
                _ => {}
            }
        }

        song
    }

    fn percent_decode(value: &str) -> String {
        let bytes = value.as_bytes();
        let mut out = Vec::with_capacity(bytes.len());
        let mut i = 0;

        while i < bytes.len() {
            match bytes[i] {
                b'+' => {
                    out.push(b' ');
                    i += 1;
                }
                b'%' if i + 2 < bytes.len() => {
                    let hex = std::str::from_utf8(&bytes[i + 1..i + 3]).unwrap_or("");
                    match u8::from_str_radix(hex, 16) {
                        Ok(byte) => {
                            out.push(byte);
                            i += 3;
                        }
                        Err(_) => {
                            out.push(bytes[i]);
                            i += 1;
                        }
                    }
                }
                byte => {
                    out.push(byte);
                    i += 1;
                }
            }
        }

        String::from_utf8_lossy(&out).into_owned()
    }
}
