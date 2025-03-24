use domain::entities::{Artist, Song};
use domain::result::AppResult;
use domain::forms::SongBodyCreation;

/// Trait defining the database operations for managing songs and artists.
///
/// This trait is used to abstract database interactions related to songs and artists.
/// It is intended to be implemented by a database access layer.
#[async_trait::async_trait]
pub trait Database: std::fmt::Debug + Send + Sync {
    /// Creates a new song in the database.
    ///
    /// This function takes song data as input and attempts to create a new song entry in the database.
    ///
    /// ## Arguments
    ///
    /// * `data` - A `SongBodyCreation` object containing the data for the new song.
    ///
    /// ## Returns
    ///
    /// Returns an `AppResult` containing the created `Song` object.
    /// If the song creation fails, returns an error.
    async fn create_song(&self, data: SongBodyCreation) -> AppResult<Song>;

    /// Retrieves a single song by its ID from the database.
    ///
    /// This function fetches a song based on the provided ID.
    ///
    /// ## Arguments
    ///
    /// * `id` - A `String` representing the ID of the song to retrieve.
    ///
    /// ## Returns
    ///
    /// Returns an `AppResult` containing the `Song`.
    /// If the song is not found, returns none.
    /// If an error occurs, returns the error.
    async fn get_one_song(&self, id: String) -> AppResult<Option<Song>>;

    /// Retrieves a list of songs from the database with pagination.
    ///
    /// This function fetches a list of songs based on the specified page and limit.
    ///
    /// ## Arguments
    ///
    /// * `page` - A `u8` representing the page number for pagination.
    /// * `limit` - An optional `u8` representing the number of songs per page.
    ///
    /// ## Returns
    ///
    /// Returns an `AppResult` containing a vector of `Song` objects or an empty array.
    /// If an error occurs, returns the error.
    async fn get_songs(&self, page: u8, limit: Option<u8>) -> AppResult<Vec<Song>>;

    /// Retrieves a list of songs, ordering by view (descending order), from the database with pagination.
    ///
    /// This function fetches a list of songs based on the specified page and limit.
    ///
    /// ## Arguments
    ///
    /// * `page` - A `u8` representing the page number for pagination.
    /// * `limit` - An optional `u8` representing the number of songs per page.
    ///
    /// ## Returns
    ///
    /// Returns an `AppResult` containing a vector of `Song` objects or an empty array.
    /// If an error occurs, returns the error.
    async fn get_songs_by_view(&self, page: u8, limit: Option<u8>) -> AppResult<Vec<Song>>;

    /// Retrieves all artists from the database.
    ///
    /// This function fetches a list of all artists from the database.
    ///
    /// ## Returns
    ///
    /// Returns an `AppResult` containing a vector of `Artist` objects or an empty array.
    /// If an error occurs, returns the error.
    async fn get_artists(&self) -> AppResult<Vec<Artist>>;
}
