#[cfg(feature = "ssr")]
use surrealdb::Error as SurrealError;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum AppError {
    #[cfg(feature = "ssr")]
    #[error("{0}")]
    Database(#[from] SurrealError),
    #[error("Something wrong !")]
    // todo: make this more understandable
    Unknown,
}
