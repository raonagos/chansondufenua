//! Domain layer: the entities and the rules that govern them.
//!
//! This is the *inside* of what used to be the hexagonal centre in v3. It knows
//! nothing about SQLite, Topcoat or HTTP. The rule is the
//! only thing kept from the hexagon; the crate ceremony around it is gone.

pub mod artist;
pub mod error;
pub mod slug;
pub mod song;

pub use artist::Artist;
pub use error::{AppError, AppResult};
pub use song::{MetaSongData, Song, sanitise_lyrics};
