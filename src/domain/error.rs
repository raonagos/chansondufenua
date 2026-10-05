//! Domain error type.
//!
//! Deliberately small: the domain says *what* is invalid, and the adapters
//! decide how to surface it (HTTP status, log line, ...). v3's variant here was
//! `#[from] surrealdb::Error`, which is exactly the leak we removed by dropping
//! the hexagon — the domain no longer names a database.

use thiserror::Error;

#[derive(Debug, Error)]
pub enum AppError {
    /// A rule in this layer rejected the input.
    #[error("invalid {field}: {reason}")]
    Invalid { field: &'static str, reason: String },

    /// Catch-all, kept from v3 (`domain/src/error.rs`).
    #[error("Something wrong !")]
    Unknown,
}

impl AppError {
    pub fn invalid(field: &'static str, reason: impl Into<String>) -> Self {
        Self::Invalid {
            field,
            reason: reason.into(),
        }
    }
}

/// Convenience alias, as in v3 (`domain/src/result.rs`).
pub type AppResult<T> = Result<T, AppError>;
