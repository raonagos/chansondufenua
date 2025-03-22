use super::Result;

use domain::entities::Song;
use domain::forms::SongBodyCreation;

use leptos::prelude::*;
use leptos::server_fn::codec::PostUrl;

/// Creates a new song in the database.
///
/// ## Arguments
///
/// * `data` - A `SongBodyCreation` object containing the data for the new song.
///
/// ## Returns
///
/// Returns a `Result` containing the created `Song` object.
/// If the song creation fails, returns a [`ServerFnError`] with a status code of `BAD_REQUEST` (400).
///
/// ## Example
///
/// ```rust
/// match create_song(song_data).await {
///     Ok(song) => println!("Song created: {:?}", song),
///     Err(e) => eprintln!("Error creating song: {:?}", e),
/// }
/// ```
///
/// [`ServerFnError`]: https://docs.rs/leptos/latest/leptos/prelude/enum.ServerFnError.html
#[server(input = PostUrl)]
pub async fn create_song(data: SongBodyCreation) -> Result<Song> {
    use crate_core::state::AppState;

    let AppState { pool: db, .. } = expect_context::<AppState>();
    if let Ok(song) = db.create_song(data).await {
        let path = format!("/himene/{}", song.get_id());
        // code 302, not 201
        leptos_axum::redirect(&path);
        return Ok(song);
    }

    let response = expect_context::<leptos_axum::ResponseOptions>();
    response.set_status(axum::http::StatusCode::BAD_REQUEST);

    return Err(ServerFnError::new("could not create song !"));
}

/// Fetches a song and its associated artist from the database.
///
/// ## Arguments
///
/// * `id` - A `String` representing the ID of the song to retrieve.
///
/// ## Returns
///
/// Returns a `Result` containing the `Song` object.
/// If the song is not found, returns a [`ServerFnError`].
///
/// ## Example
///
/// ```rust
/// match get_song_fetch_artist(song_id).await {
///     Ok(song) => println!("Song retrieved: {:?}", song),
///     Err(e) => eprintln!("Error retrieving song: {:?}", e),
/// }
/// ```
///
/// [`ServerFnError`] : https://docs.rs/leptos/latest/leptos/prelude/enum.ServerFnError.html
#[server]
pub async fn get_song_fetch_artist(id: String) -> Result<Song> {
    use crate_core::state::AppState;
    use domain::errors::AppError;

    let AppState { pool: db, .. } = expect_context::<AppState>();

    let song = db
        .get_one_song(id)
        .await?
        .ok_or_else(|| AppError::Unknown)?;
    Ok(song)
}

/// Retrieves a list of songs from the database with pagination.
///
/// ## Arguments
///
/// * `page` - A `u8` representing the page number for pagination.
/// * `limit` - An optional `u8` representing the number of songs per page.
///
/// ## Returns
///
/// Returns a `Result` containing a vector of `Song` objects.
/// If an error occurs, returns a [`ServerFnError`].
///
/// ## Example
///
/// ```rust
/// match get_songs(1, Some(10)).await {
///     Ok(songs) => println!("Songs retrieved: {:?}", songs),
///     Err(e) => eprintln!("Error retrieving songs: {:?}", e),
/// }
/// ```
///
/// [`ServerFnError`] : https://docs.rs/leptos/latest/leptos/prelude/enum.ServerFnError.html
#[server]
pub async fn get_songs(page: u8, limit: Option<u8>) -> Result<Vec<Song>> {
    use crate_core::state::AppState;

    let AppState { pool: db, .. } = expect_context::<AppState>();
    let songs = db.get_songs(page, limit).await?;
    Ok(songs)
}

/// Retrieves a list of songs and their associated artists from the database with pagination.
///
/// ## Arguments
///
/// * `page` - A `u8` representing the page number for pagination.
/// * `limit` - An optional `u8` representing the number of songs per page.
///
/// ## Returns
///
/// Returns a `Result` containing a vector of `Song` objects.
/// If an error occurs, returns a [`ServerFnError`].
///
/// ## Example
///
/// ```rust
/// match get_songs_fetch_artist(1, Some(10)).await {
///     Ok(songs) => println!("Songs with artists retrieved: {:?}", songs),
///     Err(e) => eprintln!("Error retrieving songs with artists: {:?}", e),
/// }
/// ```
///
/// [`ServerFnError`] : https://docs.rs/leptos/latest/leptos/prelude/enum.ServerFnError.html
#[server]
pub async fn get_songs_fetch_artist(page: u8, limit: Option<u8>) -> Result<Vec<Song>> {
    use crate_core::state::AppState;

    let AppState { pool: db, .. } = expect_context::<AppState>();
    let songs = db.get_songs(page, limit).await?;
    Ok(songs)
}

/// Retrieves a list of songs and their associated artists from the database with pagination, ordered by view count.
///
/// ## Arguments
///
/// * `page` - A `u8` representing the page number for pagination.
/// * `limit` - An optional `u8` representing the number of songs per page.
///
/// ## Returns
///
/// Returns a `Result` containing a vector of `Song` objects.
/// If an error occurs, returns a [`ServerFnError`].
///
/// ## Example
///
/// ```rust
/// match get_songs_fetch_artist_by_view(1, Some(10)).await {
///     Ok(songs) => println!("Songs with artists by view retrieved: {:?}", songs),
///     Err(e) => eprintln!("Error retrieving songs with artists by view: {:?}", e),
/// }
/// ```
///
/// [`ServerFnError`] : https://docs.rs/leptos/latest/leptos/prelude/enum.ServerFnError.html
#[server]
pub async fn get_songs_fetch_artist_by_view(page: u8, limit: Option<u8>) -> Result<Vec<Song>> {
    use crate_core::state::AppState;

    let AppState { pool: db, .. } = expect_context::<AppState>();
    let songs = db.get_songs_by_view(page, limit).await?;
    Ok(songs)
}
