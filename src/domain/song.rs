//! `Song`: the entity and every rule that applies to it.
//!
//! Three renderings of the same `lyrics` value live here, and they are *not*
//! interchangeable:
//!
//! * [`Song::lyrics_html`] — sanitised HTML for the page. v3 rendered the raw
//!   column straight into the template, which is an XSS hole on a free-form
//!   "add the lyrics" form.
//! * [`Song::clean_lyrics`] — chord-free plain text, byte-for-byte what v3 put
//!   in `og:description` and JSON-LD (`Lyrics of <title> - <clean>`).
//! * [`Song::lyrics_markdown`] — Markdown with chords kept inline, for the
//!   `Accept: text/markdown` negotiation in step 10.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use super::artist::Artist;
use super::error::{AppError, AppResult};

type Datetime = DateTime<Utc>;

/// Canonical host. Every absolute URL the site emits is built from this.
pub const SITE_URL: &str = "https://www.chansondufenua.pf";

/// Schema bounds, transcribed verbatim from v3 (`legacy/surrealdb.surql`) and
/// mirrored by the `CHECK` constraints in `migrations/0001_init.sql`.
pub const TITLE_MIN: usize = 4;
pub const TITLE_MAX: usize = 100;
pub const LYRICS_MIN: usize = 100;
pub const LYRICS_MAX: usize = 6000;
/// `ASSERT array::len($value) <= 75` in v3.
pub const ARTISTS_MAX: usize = 75;

#[derive(Debug, Default, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
/// Structure representing a song.
pub struct Song {
    id: String,
    title: String,
    lyrics: String,
    view_count: u32,
    artists: Vec<Artist>,
    published: bool,
    created_at: Datetime,
    updated_at: Datetime,
}

impl Song {
    /// Eight arguments is a lot, but this is v3's constructor signature kept
    /// intact so the port is auditable. It disappears once the repository builds
    /// songs from rows (step 3a).
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        id: String,
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
    pub fn get_id(&self) -> String {
        self.id.to_owned()
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

    /// The song's canonical URL.
    pub fn get_url(&self) -> String {
        format!("{SITE_URL}/himene/{}", self.id)
    }

    /// Rejects a song the database would reject, before it gets there.
    ///
    /// This is the same set of bounds v3 expressed as SurrealDB `ASSERT` clauses
    /// (`legacy/surrealdb.surql`) — `title` 4..=100, `lyrics` 100..=6000,
    /// `view_count > 0`, at most 75 artists, each with a valid `fullname`.
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
        let mut builder = ammonia::Builder::default();
        let mut generic_attributes = std::collections::HashSet::new();
        generic_attributes.insert("data-nosnippet");
        builder.generic_attributes(generic_attributes);
        builder.clean(&self.lyrics).to_string()
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
        html_to_markdown(&self.lyrics_html())
    }

    /// Convert the song into schema.org structure data markup.
    pub fn to_jsonld(&self) -> String {
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

        let url = self.get_url();

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
    pub fn get_meta_data(&self) -> MetaSongData {
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

        // Left exactly as v3 wrote it, mixing French and Tahitian in one string.
        // Step 8 (i18n) is where this becomes a translated message.
        let meta_description = format!(
            "Lyrics of | Paroles de | Parau hīmene nō {} - {}",
            self.title,
            self.clean_lyrics(),
        );
        let meta_og_description = format!("Lyrics of {} - {}", self.title, self.clean_lyrics());

        let meta_og_url = self.get_url();
        let uat = self.get_uat_timestamp();
        let meta_img_url_og = format!("{SITE_URL}/drive/genog/{uat}/himene/{}", self.id);
        let meta_img_url_tw = format!("{SITE_URL}/drive/gentw/{uat}/himene/{}", self.id);
        let meta_og_img_alt = format!("Lyrics for {}", page_title);
        let meta_jsonld = self.to_jsonld();

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

/// Markdown renderer for the chord markup.
///
/// The markup is tiny and regular — `<div>` per line, `<br>` for a blank line,
/// `<sup>` for a chord — so this is a hand-rolled scanner rather than a regex
/// pile. Anything unrecognised is dropped rather than escaped: the input has
/// already been through [`ammonia`].
fn html_to_markdown(html: &str) -> String {
    let mut lines: Vec<String> = Vec::new();
    let mut current = String::new();
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
                        ("sup", false) => current.push('['),
                        ("sup", true) => current.push(']'),
                        // A `<div>` opens a line only if there is one to close —
                        // otherwise `<div>a</div><div>b</div>` would leave a
                        // spurious blank between a and b.
                        ("div" | "p", false) => flush_soft(&mut lines, &mut current),
                        ("div" | "p", true) | ("br", _) => flush_hard(&mut lines, &mut current),
                        _ => {}
                    }
                    i += offset + 1;
                }
                // Unterminated tag: stop rather than lose the tail.
                None => {
                    current.push(' ');
                    break;
                }
            },
            b'&' => match decode_entity(html, i) {
                Some((next, ch)) => {
                    current.push(ch);
                    i = next;
                }
                None => {
                    current.push('&');
                    i += 1;
                }
            },
            _ => {
                let ch = html[i..].chars().next().unwrap();
                current.push(ch);
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
        "nbsp" => ' ',
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
        // The oracle: the prefix of the real `og:description` served for
        // /himene/7114wvk91gffr2bj6wza, up to the fourth line.
        let song = song_with(REAL_LYRICS);
        assert_eq!(
            song.clean_lyrics(),
            "'Āhani e , E rāve'a, Nō te fa'aho'i te tau i muri, Hina'aro ho'i au"
        );
        assert_eq!(
            song.get_meta_data().meta_description,
            "Lyrics of | Paroles de | Parau hīmene nō Song Title - 'Āhani e , E rāve'a, Nō te fa'aho'i te tau i muri, Hina'aro ho'i au"
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

        let meta_data = song.get_meta_data();
        assert_eq!(meta_data.page_title, "Song Title | Chanson du fenua");
        assert!(meta_data
            .meta_description
            .contains("Lyrics of | Paroles de | Parau hīmene nō Song Title"));
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
            "Song Title".to_string(),
            "Song Lyrics".to_string(),
            100,
            vec![artist],
            true,
            Utc::now(),
            Utc::now(),
        );

        let meta_data = song.get_meta_data();
        assert_eq!(
            meta_data.page_title,
            "Song Title - Artist Name | Chanson du fenua"
        );
        assert!(meta_data
            .meta_description
            .contains("Lyrics of | Paroles de | Parau hīmene nō Song Title"));
        assert_eq!(
            meta_data.meta_og_url,
            "https://www.chansondufenua.pf/himene/Song ID"
        );
        assert!(meta_data
            .meta_img_url_og
            .starts_with("https://www.chansondufenua.pf/drive/genog/"));
        assert!(meta_data
            .meta_img_url_tw
            .starts_with("https://www.chansondufenua.pf/drive/gentw/"));
    }

    #[test]
    fn jsonld_is_a_musiccomposition_with_the_clean_text() {
        let song = song_with("<div>Line and more</div>");
        let jsonld: serde_json::Value = serde_json::from_str(&song.to_jsonld()).unwrap();

        assert_eq!(jsonld["@type"], "MusicComposition");
        assert_eq!(jsonld["name"], "Song Title");
        assert_eq!(jsonld["lyrics"]["@type"], "CreativeWork");
        assert_eq!(jsonld["lyrics"]["text"], "Line and more");
        assert_eq!(
            jsonld["url"],
            "https://www.chansondufenua.pf/himene/8nntgjk4rl5dbp67c6en"
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

        let jsonld: serde_json::Value = serde_json::from_str(&song.to_jsonld()).unwrap();
        assert_eq!(jsonld["composer"][0]["name"], "2B Brothers Tahiti");
    }

    // ---- validation -------------------------------------------------------

    #[test]
    fn validate_accepts_a_real_song() {
        let song = Song::new(
            "8nntgjk4rl5dbp67c6en".to_string(),
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
        assert!(matches!(song.validate(), Err(AppError::Invalid { field: "title", .. })));

        song.title = "Song Title".to_string();
        song.lyrics = "short".to_string();
        assert!(matches!(song.validate(), Err(AppError::Invalid { field: "lyrics", .. })));

        song.lyrics = "x".repeat(LYRICS_MIN);
        song.view_count = 0;
        assert!(matches!(song.validate(), Err(AppError::Invalid { field: "view_count", .. })));
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

        assert!(matches!(song.validate(), Err(AppError::Invalid { field: "artists", .. })));
    }

    #[test]
    fn validate_propagates_the_artist_rule() {
        let mut song = song_with(&"x".repeat(LYRICS_MIN));
        song.artists.push(Artist::new(
            "id".to_string(),
            "Joe".to_string(),
            Utc::now(),
            Utc::now(),
        ));

        assert!(matches!(song.validate(), Err(AppError::Invalid { field: "fullname", .. })));
    }
}
