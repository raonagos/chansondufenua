use chrono::{DateTime, Utc};
use eserde::Deserialize;
use serde::Serialize;

type Datetime = DateTime<Utc>;

#[derive(Debug, Default, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
/// Structure representing an artist.
pub struct Artist {
    id: String,
    fullname: String,
    #[eserde(compat)]
    created_at: Datetime,
    #[eserde(compat)]
    updated_at: Datetime,
}

impl Artist {
    pub fn new(
        id: String,
        fullname: String,
        created_at: DateTime<Utc>,
        updated_at: DateTime<Utc>,
    ) -> Self {
        Self {
            id,
            fullname,
            created_at,
            updated_at,
        }
    }

    pub fn with_id(id: String) -> Self {
        Self {
            id,
            ..Default::default()
        }
    }

    /// Retrives the `id` of the artist.
    pub fn get_id(&self) -> String {
        self.id.to_owned()
    }

    /// Retrieves the `fullname` of the artist.
    pub fn get_fullname(&self) -> String {
        self.fullname.to_owned()
    }
}

#[cfg(test)]
mod tests {
    use crate::entities::Artist;
    use chrono::Utc;

    #[test]
    fn artist_creation() {
        let created_at = Utc::now();
        let updated_at = Utc::now();
        let artist = Artist::new(
            "Artist ID".to_string(),
            "Artist Name".to_string(),
            created_at,
            updated_at,
        );

        assert_eq!(artist.get_id(), "Artist ID");
        assert_eq!(artist.get_fullname(), "Artist Name");
        //todo assert_eq!(artist.created_at, created_at);
        //todo assert_eq!(artist.updated_at, updated_at);
    }

    #[test]
    fn artist_with_id() {
        let artist = Artist::with_id("Artist ID".to_string());

        assert_eq!(artist.get_id(), "Artist ID");
        assert_eq!(artist.get_fullname(), ""); // ! empty string
                                               //todo assert!(artist.created_at <= Utc::now());
                                               //todo assert!(artist.updated_at <= Utc::now());
    }
}
