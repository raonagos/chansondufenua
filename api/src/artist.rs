use super::Result;

use domain::entities::Artist;

use leptos::prelude::*;

/// Retrieves all artists from the database.
///
/// ## Returns
///
/// Returns a `Result` containing a vector of artists.
/// If an error occurs (e.g., a database connection issue),
/// it is returned as [`ServerFnError`].
///
/// ## Example
///
/// ```rust
/// match get_artists().await {
///     Ok(artists) => println!("Artists retrieved: {:?}", artists),
///     Err(e) => eprintln!("Error retrieving artists: {:?}", e),
/// }
/// ```
#[server]
pub async fn get_artists() -> Result<Vec<Artist>> {
    use crate_core::state::AppState;

    let AppState { pool: db, .. } = expect_context::<AppState>();
    let artists = db.get_artists().await?;
    Ok(artists)
}
