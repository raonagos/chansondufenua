//! `Song`: the entity and every rule that applies to it.
//!
//! Three renderings of the same `lyrics` value live here, and they are *not*
//! interchangeable:
//!
//! * [`Song::lyrics_html`] — sanitised HTML for the page. v3 rendered the raw
//!   column straight into the template, which is an XSS hole on a free-form
//!   "add the lyrics" form.
//! * [`Song::clean_lyrics`] — chord-free plain text, byte-for-byte what v3 put
//!   in `og:description`, and still what the JSON-LD `lyrics.text` carries.
//! * [`Song::lyrics_markdown`] — Markdown with chords kept inline, for the
//!   `Accept: text/markdown` negotiation in step 10. Its transposed sibling,
//!   [`Song::lyrics_markdown_at`], keeps the canonical spelling and comes from
//!   [`crate::domain::chord`], which is where the wheel lives.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use super::artist::Artist;
use super::chord;
use super::error::{AppError, AppResult};

type Datetime = DateTime<Utc>;

/// Canonical host. Every absolute URL the site emits is built from this.
pub const SITE_URL: &str = "https://www.chansondufenua.pf";

/// Schema bounds, mirrored by the `CHECK` constraints in
/// `migrations/0001_init.sql`.
///
/// `TITLE_MIN`, `LYRICS_MIN` and `LYRICS_MAX` are transcribed verbatim from v3.
/// `TITLE_MAX` and `ARTISTS_MAX` were changed on review (2026-10-04) and are
/// deliberately *not* v3's values.
pub const TITLE_MIN: usize = 4;
pub const TITLE_MAX: usize = 255;
pub const LYRICS_MIN: usize = 100;
pub const LYRICS_MAX: usize = 6000;
/// v3 allowed 75 (`ASSERT array::len($value) <= 75`).
/// The highest count across every song in the 2025-03-22 export is 2, so 10
/// leaves room to spare while keeping a runaway create-song request cheap to
/// reject.
pub const ARTISTS_MAX: usize = 10;

/// The budget for a song page's `<meta name="description">` and
/// `og:description`.
///
/// A search engine shows roughly 155 characters of a description. v3 put the
/// whole chord-free lyric there instead — 888 characters on a real song, opening
/// with the scaffold `Lyrics of | Paroles de | Parau hīmene nō`, which is neither
/// a sentence nor a language. See [`Song::get_meta_data`].
pub const DESCRIPTION_MAX: usize = 155;

#[derive(Debug, Default, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
/// Structure representing a song.
pub struct Song {
    id: String,
    /// The address this song is published under, built from its title by
    /// [`super::slug::slugify`]. `None` when the title has no Latin letters in
    /// it, and then the id is the address — see [`Song::get_path`].
    slug: Option<String>,
    title: String,
    lyrics: String,
    view_count: u32,
    artists: Vec<Artist>,
    published: bool,
    created_at: Datetime,
    updated_at: Datetime,
}

impl Song {
    /// Nine arguments is a lot, but this is v3's constructor signature plus the
    /// slug, kept intact so the port is auditable. It disappears once the
    /// repository builds songs from rows (step 3a).
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        id: String,
        slug: Option<String>,
        title: String,
        lyrics: String,
        view_count: u32,
        artists: Vec<Artist>,
        published: bool,
        created_at: Datetime,
        updated_at: Datetime,
    ) -> Self {
        Self {
            id,
            slug,
            title,
            lyrics,
            view_count,
            artists,
            published,
            created_at,
            updated_at,
        }
    }

    /// Retrieves the `id` of the song.
    ///
    /// The v3 record key, and the stable identifier of the JSON API and the MCP
    /// tools. It is no longer the song's address — [`Song::get_path`] is.
    pub fn get_id(&self) -> String {
        self.id.to_owned()
    }

    /// Retrieves the `slug` of the song, if it has one.
    pub fn get_slug(&self) -> Option<String> {
        self.slug.to_owned()
    }

    /// The path segment this song is addressed by: its slug, or its id when the
    /// title earned it no slug.
    ///
    /// This is what a link to the song is built from, and what the redirect from
    /// the id URL points at.
    pub fn get_segment(&self) -> String {
        self.slug.to_owned().unwrap_or_else(|| self.id.to_owned())
    }

    /// Retrieves the `title` of the song.
    pub fn get_title(&self) -> String {
        self.title.to_owned()
    }

    /// Retrieves the `lyrics` of the song.
    /// A string of Html element.
    pub fn get_lyrics(&self) -> String {
        self.lyrics.to_owned()
    }

    /// Retrieves the `view_count` of the song.
    pub fn get_view_count(&self) -> u32 {
        self.view_count
    }

    /// Whether the song is publicly listed. v3 filtered every read on
    /// `published = true`.
    pub fn is_published(&self) -> bool {
        self.published
    }

    /// Retrieves all the `artist` of the song.
    pub fn get_artists(&self) -> Vec<Artist> {
        self.artists.to_owned()
    }

    /// Retrieves the creation date of the song.
    pub fn get_created_at(&self) -> Datetime {
        self.created_at
    }

    /// Retrieves the last `update` of the song.
    pub fn get_updated_at(&self) -> Datetime {
        self.updated_at
    }

    /// Get a fix timestamp until the last `update`.
    ///
    /// Used by v3 to build the OG-image URL (`/drive/genog/{uat}/...`) so that a
    /// re-render of the image invalidates the CDN copy.
    pub fn get_uat_timestamp(&self) -> i64 {
        self.updated_at.timestamp_micros()
    }

    /// The root-relative path the song is served at, canonical form.
    ///
    /// One spelling of a song URL, used by every link the site emits and by the
    /// `Location` of the 301 from the id form. The absolute form is
    /// [`Song::get_url`]; this is what a `Location` header and an internal link
    /// want, because it does not name a host the request may not have arrived
    /// on.
    pub fn get_path(&self) -> String {
        format!("/himene/{}", self.get_segment())
    }

    /// The song's address — one URL per page, and the page, the API and the
    /// sitemap all spell it the same way. There is no second form to name: the
    /// response's language is resolved from the request's own headers, not from
    /// the URL.
    pub fn get_url(&self) -> String {
        format!("{SITE_URL}{}", self.get_path())
    }

    /// Rejects a song the database would reject, before it gets there.
    ///
    /// The bounds v3 expressed as SurrealDB `ASSERT` clauses, with `title` and
    /// the artist count adjusted on review — `title` 4..=255, `lyrics`
    /// 100..=6000, `view_count > 0`, at most 10 artists, each with a valid
    /// `fullname`.
    pub fn validate(&self) -> AppResult<()> {
        let title_len = self.title.trim().chars().count();
        if !(TITLE_MIN..=TITLE_MAX).contains(&title_len) {
            return Err(AppError::invalid(
                "title",
                format!("expected {TITLE_MIN}..={TITLE_MAX} characters, got {title_len}"),
            ));
        }

        let lyrics_len = self.lyrics.trim().chars().count();
        if !(LYRICS_MIN..=LYRICS_MAX).contains(&lyrics_len) {
            return Err(AppError::invalid(
                "lyrics",
                format!("expected {LYRICS_MIN}..={LYRICS_MAX} characters, got {lyrics_len}"),
            ));
        }

        if self.view_count == 0 {
            return Err(AppError::invalid("view_count", "expected a value > 0"));
        }

        if self.artists.len() > ARTISTS_MAX {
            return Err(AppError::invalid(
                "artists",
                format!("expected at most {ARTISTS_MAX}, got {}", self.artists.len()),
            ));
        }

        for artist in &self.artists {
            Artist::validate_fullname(&artist.get_fullname())?;
        }

        Ok(())
    }

    /// The lyrics as HTML that is safe to drop into a page.
    ///
    /// The *only* difference from [`ammonia::clean`]'s defaults is that
    /// `data-nosnippet` is kept on `<sup>` — v3 emits it on every chord so the
    /// chords stay out of search snippets, and ammonia strips all `data-*`
    /// attributes unless they are named. Dropping it would silently change how
    /// the site appears in Google.
    pub fn lyrics_html(&self) -> String {
        sanitise_lyrics(&self.lyrics)
    }

    /// Retrieves the `lyrics` of the song.
    /// A text only without Html element.
    ///
    /// The regex chain is v3's, in v3's order — including the odd-looking
    /// `"<.*?>" -> ", "` that turns every tag into a comma separator. That
    /// output is what `og:description` carries in the wild; `tests` asserts it
    /// against a live page, so this is a *parity* function, not a pretty one.
    pub fn clean_lyrics(&self) -> String {
        let mut lyrics = self.lyrics_html();

        // `<sup.*?</sup>` rather than v3's literal `<sup>`: v3 fed this the
        // output of `ammonia::clean`, which had already stripped the chord's
        // `data-nosnippet` attribute. We keep that attribute (see
        // `lyrics_html`), so the pattern has to tolerate it.
        let patterns = [
            (r"<sup.*?</sup>", ""),
            (r"<.*?>", ", "),
            (r"(, ){2,}", ", "),
            (r"&.*?;", " "),
            (r"\s+", " "),
        ];

        for (pattern, replacement) in patterns {
            if let Ok(re) = regex_lite::Regex::new(pattern) {
                lyrics = re.replace_all(&lyrics, replacement).to_string();
            }
        }

        lyrics.trim().trim_matches(',').trim().into()
    }

    /// The lyrics as Markdown, chords kept where the author put them.
    ///
    /// Chords live inline in `<sup data-nosnippet="true">` and often land
    /// *inside* a word (`Hina'a<sup>Eb</sup>ro` — the chord falls on the "ro").
    /// So this is a lossless transliteration, not a transformation: `<sup>` is
    /// rendered `[Eb]` at the same offset, lines become lines, and a
    /// `<div><br></div>` becomes a blank line.
    pub fn lyrics_markdown(&self) -> String {
        self.lyrics_markdown_at(0)
    }

    /// The same document with every chord moved by `offset` semitones.
    ///
    /// This is the transposed form of the page's Markdown representation, and
    /// its chords keep the canonical spelling: the wheel names the chord, the
    /// language never does (see [`crate::domain::chord`]). At offset zero it is
    /// byte-for-byte [`Song::lyrics_markdown`], which is what every machine
    /// surface reads.
    pub fn lyrics_markdown_at(&self, offset: i32) -> String {
        html_to_markdown(&self.lyrics_html(), offset)
    }

    /// The lyrics as lines of text and chords, for rendering.
    ///
    /// The input is [`Song::lyrics_html`], so `ammonia` has already decided
    /// what counts as markup before a single character is read here. What
    /// survives in the corpus — verified across all 43 songs of the 2025-03-22
    /// export — is `<div>` per line, `<br>` for a verse break, `<sup>` for a
    /// chord, `&nbsp;` runs for spacing, and one song that wraps a spacer in
    /// `<span><b>`. Anything else is treated as transparent: the tag is dropped
    /// and the text inside it kept.
    ///
    /// **Whitespace is preserved**, which is the difference from
    /// [`Song::clean_lyrics`]. Chords are positioned against the syllable they
    /// follow, so a run of non-breaking spaces is load-bearing: collapse it, and
    /// the chords pile up at the start of the line. A blank line is an empty
    /// vector, not a vector holding a space.
    ///
    /// This exists so the page can style a chord. v3 dropped sanitised HTML
    /// straight into the template, which meant every visual decision about a
    /// chord had to be a CSS rule matching a `<sup>` — the one place in the
    /// rewrite where styling would have escaped the token vocabulary.
    pub fn lyrics_lines(&self) -> Vec<LyricLine> {
        lyric_lines_of(&self.lyrics_html())
    }

    /// Convert the song into schema.org structure data markup.
    ///
    /// `url` is the canonical URL of the page this markup describes — the caller
    /// decides it, because a caller may have a URL the domain does not build
    /// (the API's own read, for one). Structure data that named a different
    /// address from the page's `<link rel="canonical">` would be two competing
    /// canonicals in one `<head>`.
    pub fn to_jsonld(&self, url: &str) -> String {
        use serde_json::json;

        let lyrics = json!({
            "@type": "CreativeWork",
            "text": self.clean_lyrics(),
        });

        let composers = self
            .get_artists()
            .iter()
            .map(|a| {
                json!({
                    "@type": "Person",
                    "name": a.get_fullname(),
                })
            })
            .collect::<Vec<_>>();

        let schema_music = json!({
            "@context": "https://schema.org/",
            "@type": "MusicComposition",
            "@id": url,
            "name": self.get_title(),
            "composer": composers,
            "lyrics": lyrics,
            "url": url,
        });

        schema_music.to_string()
    }

    /// Everything the page needs to fill `<head>`.
    ///
    /// `url` is the canonical URL of the page, as [`Song::to_jsonld`] takes it:
    /// `og:url` and the structure data have to name the address the page is
    /// published at — one address, since v4.2, whatever language the response is
    /// written in.
    pub fn get_meta_data(&self, url: &str) -> MetaSongData {
        let mut page_title = "Chanson du fenua".to_owned();

        let artists_name = self
            .artists
            .iter()
            .map(|a| a.get_fullname())
            .collect::<Vec<_>>()
            .join(", ");

        page_title = match artists_name.is_empty() {
            true => format!("{} | {page_title}", self.title),
            false => format!("{} - {} | {page_title}", self.title, artists_name),
        };

        // v3 wrote both of these as the whole chord-free lyric — 888 characters
        // on a real song, opening with the scaffold `Lyrics of | Paroles de |
        // Parau hīmene nō`, which is neither a sentence nor a language. A
        // description is a snippet: this says what the page is, leads with the
        // title, and stops inside the ~155 characters a search engine will show.
        // `clean_lyrics` is still what the JSON-LD carries, where the whole text
        // is the point.
        let description = description_sentence(&self.title, &artists_name);
        let meta_description = description.clone();
        let meta_og_description = description;

        let meta_og_url = url.to_owned();
        let uat = self.get_uat_timestamp();
        // The two card URLs stay keyed by `id`. They are not addresses of a
        // document — they are the cache key of a PNG, asked for by this page and
        // by no one else — and the id is the stable key. Moving them to the slug
        // form would re-render two images per song for no reader's benefit, and
        // they are served `immutable` for a year.
        let meta_img_url_og = format!("{SITE_URL}/drive/genog/{uat}/himene/{}", self.id);
        let meta_img_url_tw = format!("{SITE_URL}/drive/gentw/{uat}/himene/{}", self.id);
        let meta_og_img_alt = format!("Lyrics for {}", page_title);
        let meta_jsonld = self.to_jsonld(url);

        MetaSongData {
            page_title,
            meta_description,
            meta_jsonld,
            meta_og_description,
            meta_og_url,
            meta_img_url_og,
            meta_img_url_tw,
            meta_og_img_alt,
            song_title: self.get_title(),
            song_lyrics: self.lyrics_html(),
        }
    }
}

// html meta tag helper

/// The one-sentence description of a song page.
///
/// The title first — what a reader is looking for is the song — then what the
/// page holds (its lyrics and its chords), then who wrote it and where it lives.
/// French, like the front page's own description; the chrome's language is a
/// separate question that does not reach the content.
///
/// The result is at most [`DESCRIPTION_MAX`] characters. The tail is kept whole
/// and the title gives way first, so a very long title is cut rather than the
/// sentence being left half-written.
fn description_sentence(title: &str, artists: &str) -> String {
    let mut tail = String::from(" — paroles et accords");
    if !artists.is_empty() {
        tail.push_str(" de ");
        tail.push_str(artists);
    }
    tail.push_str(", à retrouver sur Chanson du fenua.");

    // Defensive: ten artists at the schema's own maximum could in principle
    // swallow the whole budget. The corpus never comes close — the longest title
    // in the 2025-03-22 export is 28 characters and the longest artist name 18 —
    // so the ordinary path is "the title fits, nothing is cut".
    let tail = truncate_chars(&tail, DESCRIPTION_MAX);
    let room = DESCRIPTION_MAX - tail.chars().count();

    format!("{}{tail}", truncate_chars(title, room))
}

/// The first `room` characters of `text`, with an ellipsis when it had to be cut.
///
/// Counts characters, not bytes: the corpus's titles carry macrons and `ʻokina`.
pub(crate) fn truncate_chars(text: &str, room: usize) -> String {
    if text.chars().count() <= room {
        return text.to_owned();
    }
    if room == 0 {
        return String::new();
    }

    let mut out: String = text.chars().take(room - 1).collect();
    out.push('…');
    out
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct MetaSongData {
    pub page_title: String,
    pub meta_description: String,
    pub meta_jsonld: String,
    pub meta_og_description: String,
    pub meta_og_url: String,
    pub meta_img_url_og: String,
    pub meta_img_url_tw: String,
    pub meta_og_img_alt: String,
    pub song_title: String,
    pub song_lyrics: String,
}

/// The sanitised HTML of a raw `lyrics` value.
///
/// The *only* difference from [`ammonia::clean`]'s defaults is that
/// `data-nosnippet` is kept on `<sup>` — v3 emits it on every chord so the
/// chords stay out of search snippets, and ammonia strips all `data-*`
/// attributes unless they are named. Dropping it would silently change how the
/// site appears in Google.
///
/// Free-standing rather than a method, for the create-song form: a submission
/// the domain rejected is handed straight back to the browser, and the editor
/// is seeded from this rather than from the field the browser posted — that
/// value is exactly as untrusted on the way back as it was on the way in.
pub fn sanitise_lyrics(lyrics: &str) -> String {
    let mut builder = ammonia::Builder::default();
    let mut generic_attributes = std::collections::HashSet::new();
    generic_attributes.insert("data-nosnippet");
    builder.generic_attributes(generic_attributes);
    builder.clean(lyrics).to_string()
}

/// One run of a lyric line: the words, or the chord written over them.
///
/// The distinction is the whole point of [`Song::lyrics_lines`]: a chord is
/// positioned against the syllable it marks, and the page can only place it if
/// it can tell a chord from the lyric around it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LyricSpan {
    /// Lyric text, spaced exactly as the author spaced it.
    Text(String),
    /// A chord label — `B`, `F#`, `Abm`.
    Chord(String),
}

/// One line of a song: alternating text and chords, in reading order.
///
/// A verse break is an empty `Vec`. Keeping that distinct from a line of spaces
/// is what lets the page give a break its own height instead of guessing.
pub type LyricLine = Vec<LyricSpan>;

/// Scanner turning the sanitised chord markup into lines of spans.
///
/// Hand-rolled for the same reason [`html_to_markdown`] is: the markup is tiny
/// and regular, and the one thing it must never do is lose a character. The
/// losslessness is asserted rather than assumed — `lyrics_lines_lose_no_text`
/// re-extracts the text with an independent scanner and compares, over every
/// fixture as well as the real markup below.
fn lyric_lines_of(html: &str) -> Vec<LyricLine> {
    let mut lines: Vec<LyricLine> = Vec::new();
    let mut current: LyricLine = Vec::new();
    let mut in_chord = false;
    let bytes = html.as_bytes();
    let mut i = 0;

    while i < bytes.len() {
        match bytes[i] {
            b'<' => match html[i..].find('>') {
                Some(offset) => {
                    let tag = &html[i + 1..i + offset];
                    let closing = tag.starts_with('/');
                    let name = tag
                        .trim_start_matches('/')
                        .split(|c: char| c.is_ascii_whitespace() || c == '/')
                        .next()
                        .unwrap_or("")
                        .to_ascii_lowercase();

                    match (name.as_str(), closing) {
                        ("sup", false) => in_chord = true,
                        ("sup", true) => in_chord = false,
                        // A block opens only if there is a line to close.
                        // Otherwise `<div>a</div><div>b</div>` would leave a
                        // blank between a and b.
                        ("div" | "p" | "li", false) => end_line_soft(&mut lines, &mut current),
                        ("div" | "p" | "li", true) | ("br", _) => {
                            end_line(&mut lines, &mut current);
                        }
                        _ => {}
                    }
                    i += offset + 1;
                }
                // Unterminated tag: stop rather than lose the tail.
                None => {
                    push_text(&mut current, in_chord, " ");
                    break;
                }
            },
            b'&' => match decode_entity(html, i) {
                Some((next, ch)) => {
                    push_char(&mut current, in_chord, ch);
                    i = next;
                }
                None => {
                    push_char(&mut current, in_chord, '&');
                    i += 1;
                }
            },
            _ => {
                // Take the whole run up to the next tag or entity at once: most
                // of a song is one of these, and a per-character path would
                // allocate a span per letter.
                let start = i;
                while i < bytes.len() && bytes[i] != b'<' && bytes[i] != b'&' {
                    i += 1;
                }
                push_text(&mut current, in_chord, &html[start..i]);
            }
        }
    }

    end_line(&mut lines, &mut current);
    normalize_lines(lines)
}

/// Appends text to the span it belongs to, opening a new span only when the
/// kind changes.
fn push_text(line: &mut LyricLine, chord: bool, text: &str) {
    if text.is_empty() {
        return;
    }
    match (line.last_mut(), chord) {
        (Some(LyricSpan::Text(existing)), false) => existing.push_str(text),
        (Some(LyricSpan::Chord(existing)), true) => existing.push_str(text),
        _ => line.push(span(chord, text.to_owned())),
    }
}

/// [`push_text`] for a single decoded character.
fn push_char(line: &mut LyricLine, chord: bool, ch: char) {
    match (line.last_mut(), chord) {
        (Some(LyricSpan::Text(existing)), false) => existing.push(ch),
        (Some(LyricSpan::Chord(existing)), true) => existing.push(ch),
        _ => line.push(span(chord, ch.to_string())),
    }
}

fn span(chord: bool, text: String) -> LyricSpan {
    if chord {
        LyricSpan::Chord(text)
    } else {
        LyricSpan::Text(text)
    }
}

/// Ends the current line and keeps it.
fn end_line(lines: &mut Vec<LyricLine>, current: &mut LyricLine) {
    trim_line_ends(current);
    lines.push(std::mem::take(current));
}

/// Ends the current line, keeping it only if it has content.
fn end_line_soft(lines: &mut Vec<LyricLine>, current: &mut LyricLine) {
    trim_line_ends(current);
    if !current.is_empty() {
        lines.push(std::mem::take(current));
    }
}

/// Drops ASCII spaces at the ends of a line, and any span they empty.
///
/// Only ASCII spaces: a non-breaking space at the edge of a line is how the
/// author pushed a chord into place, and trimming it would move the chord.
fn trim_line_ends(line: &mut LyricLine) {
    if let Some(LyricSpan::Text(text)) = line.first_mut() {
        *text = text.trim_start_matches(' ').to_owned();
    }
    if let Some(LyricSpan::Text(text)) = line.last_mut() {
        *text = text.trim_end_matches(' ').to_owned();
    }
    line.retain(|span| match span {
        LyricSpan::Text(text) | LyricSpan::Chord(text) => !text.is_empty(),
    });
}

/// Drops blank lines at the edges and collapses interior runs to one.
///
/// The corpus writes a verse break as `<div><br></div>`, which the scanner sees
/// as two line ends in a row, so an un-collapsed break would render as a hole in
/// the song. The leading edge is handled the same way because the markup often
/// opens with a break that carries no meaning.
fn normalize_lines(lines: Vec<LyricLine>) -> Vec<LyricLine> {
    let mut out: Vec<LyricLine> = Vec::new();
    let mut previous_blank = true; // so leading blank lines disappear

    for line in lines {
        if line.is_empty() {
            if !previous_blank {
                out.push(line);
                previous_blank = true;
            }
        } else {
            out.push(line);
            previous_blank = false;
        }
    }

    while out.last().is_some_and(|line| line.is_empty()) {
        out.pop();
    }

    out
}

/// Markdown renderer for the chord markup.
///
/// The markup is tiny and regular — `<div>` per line, `<br>` for a blank line,
/// `<sup>` for a chord — so this is a hand-rolled scanner rather than a regex
/// pile. Anything unrecognised is dropped rather than escaped: the input has
/// already been through [`ammonia`].
///
/// A chord is collected in a buffer of its own rather than written into the line
/// as it streams, so that the whole label can be moved by `offset` at `</sup>`:
/// a root is more than one character, and half a chord cannot be transposed.
/// At offset zero the two renderings are identical.
fn html_to_markdown(html: &str, offset: i32) -> String {
    let mut lines: Vec<String> = Vec::new();
    let mut current = String::new();
    let mut chord = String::new();
    let mut in_chord = false;
    let bytes = html.as_bytes();
    let mut i = 0;

    while i < bytes.len() {
        match bytes[i] {
            b'<' => match html[i..].find('>') {
                Some(offset_in) => {
                    let tag = &html[i + 1..i + offset_in];
                    let closing = tag.starts_with('/');
                    let name = tag
                        .trim_start_matches('/')
                        .split(|c: char| c.is_ascii_whitespace() || c == '/')
                        .next()
                        .unwrap_or("")
                        .to_ascii_lowercase();

                    match (name.as_str(), closing) {
                        ("sup", false) => {
                            in_chord = true;
                            chord.clear();
                            current.push('[');
                        }
                        ("sup", true) => {
                            in_chord = false;
                            current.push_str(&chord::spell(&chord, offset));
                            current.push(']');
                        }
                        // A `<div>` opens a line only if there is one to close —
                        // otherwise `<div>a</div><div>b</div>` would leave a
                        // spurious blank between a and b.
                        ("div" | "p", false) => flush_soft(&mut lines, &mut current),
                        ("div" | "p", true) | ("br", _) => flush_hard(&mut lines, &mut current),
                        _ => {}
                    }
                    i += offset_in + 1;
                }
                // Unterminated tag: stop rather than lose the tail.
                None => {
                    current.push(' ');
                    break;
                }
            },
            b'&' => match decode_entity(html, i) {
                Some((next, ch)) => {
                    if in_chord {
                        chord.push(ch);
                    } else {
                        current.push(ch);
                    }
                    i = next;
                }
                None => {
                    if in_chord {
                        chord.push('&');
                    } else {
                        current.push('&');
                    }
                    i += 1;
                }
            },
            _ => {
                let ch = html[i..].chars().next().unwrap();
                if in_chord {
                    chord.push(ch);
                } else {
                    current.push(ch);
                }
                i += ch.len_utf8();
            }
        }
    }
    lines.push(current);

    normalize(lines)
}

/// End a line, keeping it only if it has content.
fn flush_soft(lines: &mut Vec<String>, current: &mut String) {
    if !current.trim().is_empty() {
        lines.push(std::mem::take(current));
    } else {
        current.clear();
    }
}

/// End a line unconditionally — `<br>` and `</div>` mean "this line is over",
/// which is what turns `<div><br></div>` into a verse break.
fn flush_hard(lines: &mut Vec<String>, current: &mut String) {
    lines.push(std::mem::take(current));
}

/// Collapse whitespace, drop blank runs at the edges, and allow at most one
/// blank line between verses.
fn normalize(lines: Vec<String>) -> String {
    let mut out: Vec<String> = Vec::new();
    let mut previous_blank = true; // so leading blank lines disappear

    for line in lines {
        let collapsed = line.split_whitespace().collect::<Vec<_>>().join(" ");
        if collapsed.is_empty() {
            if !previous_blank {
                out.push(String::new());
                previous_blank = true;
            }
        } else {
            out.push(collapsed);
            previous_blank = false;
        }
    }

    while out.last().is_some_and(String::is_empty) {
        out.pop();
    }

    out.join("\n")
}

/// Decode the handful of entities ammonia emits. Returns the index *after* the
/// entity and the character, or `None` if this `&` is not an entity.
fn decode_entity(html: &str, start: usize) -> Option<(usize, char)> {
    let rest = &html[start..];
    let end = rest.find(';')?;
    if end > 12 {
        return None; // an entity name is never this long
    }

    let name = &rest[1..end];
    let ch = match name {
        // A *real* non-breaking space, not a space. It is how the corpus
        // positions a chord: HTML collapses a run of ordinary spaces but never
        // a run of these, so decoding this to ' ' would let the layout drop
        // exactly the spacing the author wrote. `html_to_markdown` collapses
        // whitespace afterwards and so is unaffected.
        "nbsp" => '\u{a0}',
        "amp" => '&',
        "lt" => '<',
        "gt" => '>',
        "quot" => '"',
        "apos" | "#39" => '\'',
        _ => {
            let digits = name.strip_prefix('#')?;
            let (radix, digits) = match digits.strip_prefix(['x', 'X']) {
                Some(hex) => (16, hex),
                None => (10, digits),
            };
            char::from_u32(u32::from_str_radix(digits, radix).ok()?)?
        }
    };

    Some((start + end + 1, ch))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Real markup, copied from the live page
    /// `https://www.chansondufenua.pf/himene/7114wvk91gffr2bj6wza` ("'Āhani e",
    /// 2B Brothers Tahiti) — chords inline in `<sup>`, lines in `<div>`.
    const REAL_LYRICS: &str = concat!(
        r#"'Āhani e<sup data-nosnippet="true">B</sup>&nbsp; &nbsp; &nbsp;<sup data-nosnippet="true">F#</sup>"#,
        r#"<div>E rāve'a<sup data-nosnippet="true">Abm</sup></div>"#,
        r#"<div>Nō te fa<sup data-nosnippet="true">E</sup>'aho'i te ta<sup data-nosnippet="true">B</sup>u i muri<sup data-nosnippet="true">F#</sup></div>"#,
        r#"<div>Hina'a<sup data-nosnippet="true">Eb</sup>ro ho'i au</div>"#,
    );

    fn song_with(lyrics: &str) -> Song {
        Song::new(
            "8nntgjk4rl5dbp67c6en".to_string(),
            Some("song-title".to_owned()),
            "Song Title".to_string(),
            lyrics.to_string(),
            100,
            vec![],
            true,
            Utc::now(),
            Utc::now(),
        )
    }

    #[test]
    fn song_creation() {
        let artist = Artist::new(
            "Artist ID".to_string(),
            "Artist Name".to_string(),
            Utc::now(),
            Utc::now(),
        );
        let song = Song::new(
            "Song ID".to_string(),
            Some("song-title".to_owned()),
            "Song Title".to_string(),
            "Song Lyrics".to_string(),
            100,
            vec![artist],
            true,
            Utc::now(),
            Utc::now(),
        );

        assert_eq!(song.get_id(), "Song ID");
        assert_eq!(song.get_title(), "Song Title");
        assert_eq!(song.get_lyrics(), "Song Lyrics");
        assert_eq!(song.get_view_count(), 100);
        assert!(song.is_published());
        assert_eq!(song.get_artists().len(), 1);
        assert!(song.get_updated_at() <= Utc::now());
    }

    // ---- clean_lyrics (parity with v3 / the live site) ----------------------

    #[test]
    fn clean_lyrics() {
        let song = song_with("<sup>Verse 1</sup> <div>Lyrics</div> &amp; more");

        let clean_lyrics = song.clean_lyrics();
        assert_eq!(clean_lyrics, "Lyrics, more");
    }

    #[test]
    fn clean_lyrics_matches_live_output() {
        // The oracle: the prefix of the real `og:description` v3 served for
        // /himene/7114wvk91gffr2bj6wza, up to the fourth line. `clean_lyrics` is
        // a parity function — v4.1 stopped putting its output in the page head
        // (see `description_sentence`), but the JSON-LD still carries it, so it
        // must keep matching what v3 produced.
        let song = song_with(REAL_LYRICS);
        assert_eq!(
            song.clean_lyrics(),
            "'Āhani e , E rāve'a, Nō te fa'aho'i te tau i muri, Hina'aro ho'i au"
        );
    }

    #[test]
    fn clean_lyrics_drops_chords_but_not_syllables() {
        // `Hina'a<sup>Eb</sup>ro` is one word broken by a chord. Removing the
        // chord must rejoin it, not insert a separator — the trap the v3 regex
        // order (`<sup.*?</sup>` before `<.*?>`) exists to avoid.
        let song = song_with("<div>Hina'a<sup data-nosnippet=\"true\">Eb</sup>ro ho'i au</div>");

        assert_eq!(song.clean_lyrics(), "Hina'aro ho'i au");
    }

    // ---- lyrics_html (sanitisation) ---------------------------------------

    #[test]
    fn lyrics_html_keeps_chords_and_nosnippet() {
        let song = song_with(REAL_LYRICS);
        let html = song.lyrics_html();

        assert!(html.contains(r#"<sup data-nosnippet="true">B</sup>"#));
        assert!(html.contains("<div>"));
    }

    #[test]
    fn lyrics_html_strips_scripts() {
        let song = song_with("<div>hi</div><script>alert(1)</script><div onclick=\"x\">y</div>");
        let html = song.lyrics_html();

        assert!(!html.contains("script"), "got: {html}");
        assert!(!html.contains("onclick"), "got: {html}");
        assert!(html.contains("hi"));
    }

    // ---- lyrics_markdown (new, for agents) --------------------------------

    #[test]
    fn markdown_keeps_chords_inline() {
        let song = song_with(REAL_LYRICS);

        assert_eq!(
            song.lyrics_markdown(),
            "'Āhani e[B] [F#]\nE rāve'a[Abm]\nNō te fa[E]'aho'i te ta[B]u i muri[F#]\nHina'a[Eb]ro ho'i au"
        );
    }

    /// The Markdown form can be stepped like the page, and its chords keep the
    /// canonical spelling: `Do dièse` is chrome, and a machine reading this wants
    /// `C#`. At zero it is the document it has always been, byte for byte.
    #[test]
    fn markdown_transposes_without_renaming_a_chord() {
        let song = song_with(REAL_LYRICS);

        assert_eq!(song.lyrics_markdown_at(0), song.lyrics_markdown());
        assert_eq!(
            song.lyrics_markdown_at(2),
            "'Āhani e[C#] [Ab]\nE rāve'a[Bbm]\nNō te fa[F#]'aho'i te ta[C#]u i muri[Ab]\nHina'a[F]ro ho'i au"
        );
        assert_eq!(
            song.lyrics_markdown_at(-1),
            "'Āhani e[Bb] [F]\nE rāve'a[Gm]\nNō te fa[Eb]'aho'i te ta[Bb]u i muri[F]\nHina'a[D]ro ho'i au"
        );
    }

    #[test]
    fn markdown_verse_break_is_a_blank_line() {
        let song = song_with("<div>Verse one</div><div><br></div><div>Verse two</div>");

        assert_eq!(song.lyrics_markdown(), "Verse one\n\nVerse two");
    }

    #[test]
    fn markdown_has_no_leading_or_trailing_blank() {
        let song = song_with("<div><br></div><div>Only line</div><div><br></div><div><br></div>");

        assert_eq!(song.lyrics_markdown(), "Only line");
    }

    #[test]
    fn markdown_decodes_entities() {
        let song = song_with("<div>a&nbsp;b &amp; c&nbsp; &nbsp;d&apos;e</div>");

        assert_eq!(song.lyrics_markdown(), "a b & c d'e");
    }

    // ---- metadata ---------------------------------------------------------

    #[test]
    fn song_metadata_without_artist() {
        let song = song_with("Song Lyrics");

        let meta_data = song.get_meta_data(&song.get_url());
        assert_eq!(meta_data.page_title, "Song Title | Chanson du fenua");
        assert_eq!(
            meta_data.meta_description,
            "Song Title — paroles et accords, à retrouver sur Chanson du fenua."
        );
        assert_eq!(meta_data.meta_og_description, meta_data.meta_description);
    }

    #[test]
    fn song_metadata() {
        let artist = Artist::new(
            "Artist ID".to_string(),
            "Artist Name".to_string(),
            Utc::now(),
            Utc::now(),
        );
        let song = Song::new(
            "Song ID".to_string(),
            Some("song-title".to_owned()),
            "Song Title".to_string(),
            "Song Lyrics".to_string(),
            100,
            vec![artist],
            true,
            Utc::now(),
            Utc::now(),
        );

        let meta_data = song.get_meta_data(&song.get_url());
        assert_eq!(
            meta_data.page_title,
            "Song Title - Artist Name | Chanson du fenua"
        );
        assert_eq!(
            meta_data.meta_description,
            "Song Title — paroles et accords de Artist Name, à retrouver sur Chanson du fenua."
        );
        assert_eq!(meta_data.meta_og_description, meta_data.meta_description);
        assert_eq!(
            meta_data.meta_og_url,
            "https://www.chansondufenua.pf/himene/song-title"
        );
        assert!(
            meta_data
                .meta_img_url_og
                .starts_with("https://www.chansondufenua.pf/drive/genog/")
        );
        assert!(
            meta_data
                .meta_img_url_tw
                .starts_with("https://www.chansondufenua.pf/drive/gentw/")
        );
    }

    /// The snippet is a sentence, not the document: it leads with the title,
    /// says what the page is, and stays inside the ~155 characters a search
    /// engine shows — which the whole chord-free lyric (888 characters on a real
    /// song) never did.
    #[test]
    fn the_description_leads_with_the_title_and_fits_a_snippet() {
        let song = song_with(REAL_LYRICS);
        let meta = song.get_meta_data(&song.get_url());

        assert!(
            meta.meta_description.starts_with("Song Title"),
            "{:?}",
            meta.meta_description
        );
        assert!(meta.meta_description.contains("paroles et accords"));
        assert!(
            meta.meta_description.chars().count() <= DESCRIPTION_MAX,
            "{} characters: {:?}",
            meta.meta_description.chars().count(),
            meta.meta_description
        );
        // The trap this step exists to close: the description used to *be* the
        // lyric.
        assert!(
            !meta.meta_description.contains("Hina'aro"),
            "{:?}",
            meta.meta_description
        );
        assert!(meta.meta_description.chars().count() < song.clean_lyrics().chars().count());
    }

    #[test]
    fn the_description_names_the_artists() {
        let mut song = song_with("Song Lyrics");
        for name in ["2B Brothers Tahiti", "T'Angelo"] {
            song.artists.push(Artist::new(
                name.to_string(),
                name.to_string(),
                Utc::now(),
                Utc::now(),
            ));
        }

        let meta = song.get_meta_data(&song.get_url());
        assert!(
            meta.meta_description
                .contains("paroles et accords de 2B Brothers Tahiti, T'Angelo"),
            "{:?}",
            meta.meta_description
        );
    }

    /// A title longer than the budget is cut, and the sentence survives whole:
    /// the ellipsis lands inside the title, never on the tail.
    #[test]
    fn a_long_title_gives_way_before_the_sentence() {
        let long = "Ā".repeat(TITLE_MAX);
        let song = Song::new(
            "8nntgjk4rl5dbp67c6en".to_string(),
            Some("song-title".to_owned()),
            long.clone(),
            "Song Lyrics".to_string(),
            100,
            vec![],
            true,
            Utc::now(),
            Utc::now(),
        );

        let description = song.get_meta_data(&song.get_url()).meta_description;
        assert_eq!(description.chars().count(), DESCRIPTION_MAX);
        assert!(description.ends_with(", à retrouver sur Chanson du fenua."));
        assert!(description.contains('…'));
        assert!(long.starts_with(&description[..description.find('…').unwrap()]));
    }

    #[test]
    fn jsonld_is_a_musiccomposition_with_the_clean_text() {
        let song = song_with("<div>Line and more</div>");
        let jsonld: serde_json::Value =
            serde_json::from_str(&song.to_jsonld(&song.get_url())).unwrap();

        assert_eq!(jsonld["@type"], "MusicComposition");
        assert_eq!(jsonld["name"], "Song Title");
        assert_eq!(jsonld["lyrics"]["@type"], "CreativeWork");
        assert_eq!(jsonld["lyrics"]["text"], "Line and more");
        assert_eq!(
            jsonld["url"],
            "https://www.chansondufenua.pf/himene/song-title"
        );
        assert!(jsonld["composer"].as_array().unwrap().is_empty());
    }

    /// v3 quirk, kept deliberately: the `&.*?;` -> `" "` rule runs *after*
    /// `<.*?>` -> `", "`, so named entities are deleted rather than decoded.
    /// `&amp;` therefore reads as a missing word in `og:description` and in the
    /// JSON-LD `text`. Not fixed in step 2 — parity first; this test is the
    /// marker so step 8/10 can decide to decode instead.
    #[test]
    fn clean_lyrics_drops_named_entities_v3_parity() {
        let song = song_with("<div>Line &amp; more</div>");

        assert_eq!(song.clean_lyrics(), "Line more");
        // ...while `lyrics_markdown`, which is new, decodes properly.
        assert_eq!(song.lyrics_markdown(), "Line & more");
    }

    #[test]
    fn jsonld_lists_composers() {
        let artist = Artist::new(
            "Artist ID".to_string(),
            "2B Brothers Tahiti".to_string(),
            Utc::now(),
            Utc::now(),
        );
        let mut song = song_with("Song Lyrics");
        song.artists.push(artist);

        let jsonld: serde_json::Value =
            serde_json::from_str(&song.to_jsonld(&song.get_url())).unwrap();
        assert_eq!(jsonld["composer"][0]["name"], "2B Brothers Tahiti");
    }

    // ---- identity: the slug, and the id it falls back to -------------------

    /// The address is the slug. `get_url` and `get_path` are the two spellings
    /// of it that the site emits — one for a canonical link, one for a
    /// `Location` header and an internal `href`.
    #[test]
    fn a_song_with_a_slug_is_addressed_by_its_slug() {
        let song = song_with("Song Lyrics");

        assert_eq!(song.get_slug(), Some("song-title".to_owned()));
        assert_eq!(song.get_segment(), "song-title");
        assert_eq!(song.get_path(), "/himene/song-title");
        assert_eq!(
            song.get_url(),
            "https://www.chansondufenua.pf/himene/song-title"
        );
        // ...and the id is still the stable key, not the address.
        assert_eq!(song.get_id(), "8nntgjk4rl5dbp67c6en");
    }

    /// A title with no Latin letters earns no slug (`slugify` returns nothing to
    /// build one from), and the song keeps the address v3 gave it rather than
    /// getting an invented one.
    #[test]
    fn a_song_with_no_slug_is_addressed_by_its_id() {
        let mut song = song_with("Song Lyrics");
        song.slug = None;

        assert_eq!(song.get_slug(), None);
        assert_eq!(song.get_segment(), "8nntgjk4rl5dbp67c6en");
        assert_eq!(song.get_path(), "/himene/8nntgjk4rl5dbp67c6en");
        assert_eq!(
            song.get_url(),
            "https://www.chansondufenua.pf/himene/8nntgjk4rl5dbp67c6en"
        );
    }

    // ---- validation -------------------------------------------------------

    #[test]
    fn validate_accepts_a_real_song() {
        let song = Song::new(
            "8nntgjk4rl5dbp67c6en".to_string(),
            Some("ahani-e".to_owned()),
            "'Āhani e".to_string(),
            REAL_LYRICS.to_string(),
            1,
            vec![],
            true,
            Utc::now(),
            Utc::now(),
        );

        assert!(song.validate().is_ok());
    }

    #[test]
    fn validate_rejects_out_of_bounds_values() {
        let mut song = song_with(&"x".repeat(LYRICS_MIN));
        assert!(song.validate().is_ok());

        song.title = "abc".to_string();
        assert!(matches!(
            song.validate(),
            Err(AppError::Invalid { field: "title", .. })
        ));

        song.title = "Song Title".to_string();
        song.lyrics = "short".to_string();
        assert!(matches!(
            song.validate(),
            Err(AppError::Invalid {
                field: "lyrics",
                ..
            })
        ));

        song.lyrics = "x".repeat(LYRICS_MIN);
        song.view_count = 0;
        assert!(matches!(
            song.validate(),
            Err(AppError::Invalid {
                field: "view_count",
                ..
            })
        ));
    }

    #[test]
    fn validate_rejects_too_many_artists() {
        let mut song = song_with(&"x".repeat(LYRICS_MIN));
        for n in 0..=ARTISTS_MAX {
            song.artists.push(Artist::new(
                format!("id-{n}"),
                "Artist Name".to_string(),
                Utc::now(),
                Utc::now(),
            ));
        }

        assert!(matches!(
            song.validate(),
            Err(AppError::Invalid {
                field: "artists",
                ..
            })
        ));
    }

    #[test]
    fn validate_propagates_the_artist_rule() {
        let mut song = song_with(&"x".repeat(LYRICS_MIN));
        song.artists.push(Artist::new(
            "id".to_string(),
            String::new(),
            Utc::now(),
            Utc::now(),
        ));

        assert!(matches!(
            song.validate(),
            Err(AppError::Invalid {
                field: "fullname",
                ..
            })
        ));
    }
    // ---- lyrics_lines (new, for rendering) --------------------------------

    /// The chord sits *between* the syllables it joins, and the line survives
    /// as one line. This is the shape the page's positioning depends on.
    #[test]
    fn lyrics_lines_keeps_chords_where_the_author_put_them() {
        let song = song_with(REAL_LYRICS);

        assert_eq!(
            song.lyrics_lines(),
            vec![
                vec![
                    LyricSpan::Text("'Āhani e".to_owned()),
                    LyricSpan::Chord("B".to_owned()),
                    // Non-breaking spaces: the author's spacing, kept, because
                    // the next chord is positioned by it.
                    LyricSpan::Text("\u{a0} \u{a0} \u{a0}".to_owned()),
                    LyricSpan::Chord("F#".to_owned()),
                ],
                vec![
                    LyricSpan::Text("E rāve'a".to_owned()),
                    LyricSpan::Chord("Abm".to_owned()),
                ],
                vec![
                    LyricSpan::Text("Nō te fa".to_owned()),
                    LyricSpan::Chord("E".to_owned()),
                    LyricSpan::Text("'aho'i te ta".to_owned()),
                    LyricSpan::Chord("B".to_owned()),
                    LyricSpan::Text("u i muri".to_owned()),
                    LyricSpan::Chord("F#".to_owned()),
                ],
                vec![
                    LyricSpan::Text("Hina'a".to_owned()),
                    LyricSpan::Chord("Eb".to_owned()),
                    LyricSpan::Text("ro ho'i au".to_owned()),
                ],
            ]
        );
    }

    /// A chord must not split the word it sits inside — the two text spans
    /// either side of it stay separate, so the page can place the chord without
    /// inserting a break into the lyric.
    #[test]
    fn a_chord_inside_a_word_leaves_the_word_in_two_runs() {
        let song = song_with("<div>Hina'a<sup data-nosnippet=\"true\">Eb</sup>ro</div>");

        assert_eq!(
            song.lyrics_lines(),
            vec![vec![
                LyricSpan::Text("Hina'a".to_owned()),
                LyricSpan::Chord("Eb".to_owned()),
                LyricSpan::Text("ro".to_owned()),
            ]]
        );
    }

    /// The verse break is one empty line, however the markup spells it.
    #[test]
    fn lyrics_lines_gives_a_verse_break_one_blank_line() {
        for markup in [
            "<div>one</div><div><br></div><div>two</div>",
            "<div>one</div><div><br></div><div><br></div><div>two</div>",
        ] {
            let song = song_with(markup);
            let lines = song.lyrics_lines();

            assert_eq!(lines.len(), 3, "wrong line count for {markup:?}: {lines:?}");
            assert!(lines[1].is_empty(), "the break is not blank: {markup:?}");
            assert_eq!(lines[0], vec![LyricSpan::Text("one".to_owned())]);
            assert_eq!(lines[2], vec![LyricSpan::Text("two".to_owned())]);
        }
    }

    /// ...and a break at either edge of the song is not a line at all: a sheet
    /// that opened with a gap would look like a missing verse.
    #[test]
    fn lyrics_lines_drops_breaks_at_the_edges() {
        let song = song_with("<div><br></div><div>one</div><div><br></div>");

        assert_eq!(
            song.lyrics_lines(),
            vec![vec![LyricSpan::Text("one".to_owned())]]
        );
    }

    /// Unknown inline markup is transparent: the span survives, the tag does
    /// not. One song in the export wraps a spacer in `<span><b>`, and the
    /// scanner must not drop the spaces inside it.
    #[test]
    fn unknown_inline_tags_are_transparent() {
        let song = song_with("<div>a<span><b>&nbsp;&nbsp;</b></span>b</div>");

        assert_eq!(
            song.lyrics_lines(),
            vec![vec![LyricSpan::Text("a\u{a0}\u{a0}b".to_owned())]]
        );
    }

    /// The one property that matters more than any of the above: the scanner
    /// loses no text.
    ///
    /// Checked with an independently written extractor, over the real markup and
    /// over every fixture, because a scanner that quietly eats a syllable would
    /// still pass every other test in this file.
    #[test]
    fn lyrics_lines_lose_no_text() {
        let mut checked = vec![REAL_LYRICS];
        checked.extend(crate::db::fixtures::SONGS.iter().map(|song| song.lyrics));
        assert!(checked.len() > 1, "the fixtures are not being read");

        for markup in checked {
            let song = song_with(markup);
            let expected = squash(&extract_text(&song.lyrics_html()));
            let actual = squash(
                &song
                    .lyrics_lines()
                    .iter()
                    .flatten()
                    .map(|span| match span {
                        LyricSpan::Text(text) | LyricSpan::Chord(text) => text.as_str(),
                    })
                    .collect::<String>(),
            );

            assert_eq!(
                actual, expected,
                "the scanner changed the text of a song: {markup:.80}…"
            );
        }
    }

    /// Extracts every character of text, decoding entities, with an approach
    /// deliberately unlike [`lyric_lines_of`]'s: it knows nothing about lines or
    /// chords, so it cannot agree with a mistake those two make together.
    fn extract_text(html: &str) -> String {
        let mut out = String::new();
        let mut i = 0;
        while i < html.len() {
            match html.as_bytes()[i] {
                b'<' => match html[i..].find('>') {
                    Some(offset) => i += offset + 1,
                    None => break,
                },
                b'&' => match decode_entity(html, i) {
                    Some((next, ch)) => {
                        out.push(ch);
                        i = next;
                    }
                    None => {
                        out.push('&');
                        i += 1;
                    }
                },
                _ => {
                    let ch = html[i..].chars().next().expect("a char boundary");
                    out.push(ch);
                    i += ch.len_utf8();
                }
            }
        }
        out
    }

    /// Everything but whitespace, so the comparison is about words rather than
    /// about where the scanner chose to break them.
    fn squash(text: &str) -> String {
        text.chars().filter(|c| !c.is_whitespace()).collect()
    }

    // ---- metadata ---------------------------------------------------------
}
