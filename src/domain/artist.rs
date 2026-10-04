//! `Artist`: a person or group credited on a song.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use super::error::{AppError, AppResult};

type Datetime = DateTime<Utc>;

/// Schema bounds, transcribed verbatim from v3 (`legacy/surrealdb.surql`) and
/// mirrored by the `CHECK` constraints in `migrations/0001_init.sql`.
pub const FULLNAME_MIN: usize = 4;
pub const FULLNAME_MAX: usize = 50;

#[derive(Debug, Default, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
/// Structure representing an artist.
pub struct Artist {
    id: String,
    fullname: String,
    created_at: Datetime,
    updated_at: Datetime,
}

impl Artist {
    pub fn new(
        id: String,
        fullname: String,
        created_at: Datetime,
        updated_at: Datetime,
    ) -> Self {
        Self {
            id,
            fullname,
            created_at,
            updated_at,
        }
    }

    /// An artist that is only ever referenced by id, never read. v3 used this
    /// when building the artist list of a song from ids alone.
    pub fn with_id(id: String) -> Self {
        Self {
            id,
            ..Default::default()
        }
    }

    /// Retrieves the `id` of the artist.
    pub fn get_id(&self) -> String {
        self.id.to_owned()
    }

    /// Retrieves the `fullname` of the artist.
    pub fn get_fullname(&self) -> String {
        self.fullname.to_owned()
    }

    /// Retrieves the creation date of the artist.
    pub fn get_created_at(&self) -> Datetime {
        self.created_at
    }

    /// Retrieves the last update of the artist.
    pub fn get_updated_at(&self) -> Datetime {
        self.updated_at
    }

    /// Rejects a `fullname` the database would reject.
    pub fn validate_fullname(fullname: &str) -> AppResult<()> {
        let trimmed = fullname.trim();
        let len = trimmed.chars().count();
        if !(FULLNAME_MIN..=FULLNAME_MAX).contains(&len) {
            return Err(AppError::invalid(
                "fullname",
                format!("expected {FULLNAME_MIN}..={FULLNAME_MAX} characters, got {len}"),
            ));
        }
        Ok(())
    }

    /// Splits the editor's comma-separated artist field into individual names,
    /// dropping blanks. This mirrors `fn::get_unknows_artists` in v3, which
    /// split on `,` and skipped empty entries.
    pub fn split_fullnames(fullnames: &str) -> Vec<&str> {
        fullnames
            .split(',')
            .map(str::trim)
            .filter(|name| !name.is_empty())
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
        // v3 left these as `todo!`; the `eserde` dependency that made them
        // awkward is gone, so they are real assertions now.
        assert_eq!(artist.get_created_at(), created_at);
        assert_eq!(artist.get_updated_at(), updated_at);
    }

    #[test]
    fn artist_with_id() {
        let artist = Artist::with_id("Artist ID".to_string());

        assert_eq!(artist.get_id(), "Artist ID");
        assert_eq!(artist.get_fullname(), ""); // ! empty string
        assert!(artist.get_created_at() <= Utc::now());
        assert!(artist.get_updated_at() <= Utc::now());
    }

    #[test]
    fn validate_fullname_bounds() {
        assert!(Artist::validate_fullname("Joe").is_err());
        assert!(Artist::validate_fullname("Jose").is_ok());
        assert!(Artist::validate_fullname(&"a".repeat(FULLNAME_MAX)).is_ok());
        assert!(Artist::validate_fullname(&"a".repeat(FULLNAME_MAX + 1)).is_err());
        // Counts characters, not bytes: real names here include ā, ', ē.
        assert!(Artist::validate_fullname("'Āhani e").is_ok());
    }

    #[test]
    fn split_fullnames_drops_blanks() {
        assert_eq!(
            Artist::split_fullnames("2B Brothers Tahiti,T'Angelo"),
            vec!["2B Brothers Tahiti", "T'Angelo"]
        );
        assert_eq!(Artist::split_fullnames(" , ,  "), Vec::<&str>::new());
    }
}
