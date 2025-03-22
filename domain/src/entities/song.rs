use super::Artist;

use chrono::{DateTime, Utc};
use eserde::Deserialize;
use serde::Serialize;

type Datetime = DateTime<Utc>;

#[derive(Debug, Default, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
/// Structure representing a song.
pub struct Song {
    id: String,
    title: String,
    lyrics: String,
    view_count: u32,
    artists: Vec<Artist>,
    published: bool,
    #[eserde(compat)]
    created_at: Datetime,
    #[eserde(compat)]
    updated_at: Datetime,
}

impl Song {
    pub fn new(
        id: String,
        title: String,
        lyrics: String,
        view_count: u32,
        artists: Vec<Artist>,
        published: bool,
        created_at: DateTime<Utc>,
        updated_at: DateTime<Utc>,
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

    /// Retrieves all the `artist` of the song.
    pub fn get_artists(&self) -> Vec<Artist> {
        self.artists.to_owned()
    }

    /// Retrieves the last `update` of the song.
    pub fn get_updated_at(&self) -> Datetime {
        self.updated_at.to_owned()
    }

    /// Get a fix timestamp until the last `update`.
    pub fn get_uat_timestamp(&self) -> i64 {
        self.updated_at.timestamp_micros()
    }

    /// Retrieves the `lyrics` of the song.
    /// A text only without Html element.
    pub fn get_clean_lyrics(&self) -> String {
        // todo: you should move that to server side
        let mut lyrics = ammonia::clean(&self.lyrics);

        let patterns = vec![
            (r"<sup>.*?</sup>", ""),
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

    /// Convert the song into schema.org structure data markup.
    pub fn to_jsonld(&self) -> String {
        // todo: you should move that to server side
        use serde_json::json;

        let lyrics = json!({
        "@type": "CreativeWork",
        "text": self.get_clean_lyrics(),
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

        let url = format!("https://www.chansondufenua.pf/himene/{}", self.get_id());

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

        let meta_description = format!(
            "Lyrics of | Paroles de | Parau hīmene nō {} - {}",
            self.title,
            self.get_clean_lyrics(),
        );
        let meta_og_description = format!("Lyrics of {} - {}", self.title, self.get_clean_lyrics());

        let meta_og_url = format!("https://www.chansondufenua.pf/himene/{}", self.id);
        let uat = self.get_uat_timestamp();
        let meta_img_url_og = format!(
            "https://www.chansondufenua.pf/drive/genog/{uat}/himene/{}",
            self.id
        );
        let meta_img_url_tw = format!(
            "https://www.chansondufenua.pf/drive/gentw/{uat}/himene/{}",
            self.id
        );
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
            song_lyrics: self.get_lyrics(),
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

#[cfg(test)]
mod tests {
    #[cfg(test)]
    use crate::entities::Artist;
    use crate::entities::Song;
    use chrono::Utc;

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
        assert_eq!(song.get_artists().len(), 1);
        assert!(song.get_updated_at() <= Utc::now());
    }

    #[test]
    fn clean_lyrics() {
        let song = Song::new(
            "Song ID".to_string(),
            "Song Title".to_string(),
            "<sup>Verse 1</sup> <div>Lyrics</div> &amp; more".to_string(),
            0,
            vec![],
            true,
            Utc::now(),
            Utc::now(),
        );

        let clean_lyrics = song.get_clean_lyrics();
        assert_eq!(clean_lyrics, "Lyrics, more");
    }

    #[test]
    fn song_metadata_without_artist() {
        let song = Song::new(
            "Song ID".to_string(),
            "Song Title".to_string(),
            "Song Lyrics".to_string(),
            100,
            vec![],
            true,
            Utc::now(),
            Utc::now(),
        );

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
    }
}
