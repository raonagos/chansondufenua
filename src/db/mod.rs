//! SQLite persistence layer (step 3a of `PLAN.md`).
//!
//! One embedded database, no server, no credentials. The engine is compiled into
//! the binary by `sqlx`'s bundled `libsqlite3-sys`, so deploying the site is
//! copying one file.
//!
//! This module is the *outside* of the former hexagon: it is allowed to know
//! about SQLite, and it is the only place that is. `src/domain` stays ignorant
//! of it, which is the one rule worth keeping from v3's ports-and-adapters
//! layout (see `PLAN.md` §2.1).

pub mod fixtures;
pub mod queries;

pub use queries::{
    SongOrder, artists, create_song, increment_view_count, search_artists, song, songs,
};

use std::str::FromStr;
use std::time::Duration;

use sqlx::SqlitePool;
use sqlx::migrate::MigrateError;
use sqlx::sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions, SqliteSynchronous};
use thiserror::Error;

use crate::domain::AppError;

/// Everything that can go wrong below the domain layer.
///
/// Deliberately small. `Domain` is carried through rather than flattened so that
/// an HTTP handler can tell "the caller sent nonsense" (422) from "the database
/// broke" (500) without string-matching a message.
#[derive(Debug, Error)]
pub enum DbError {
    #[error(transparent)]
    Sqlx(#[from] sqlx::Error),

    #[error(transparent)]
    Migrate(#[from] MigrateError),

    #[error(transparent)]
    Domain(#[from] AppError),

    /// Creating the directory for a file-backed database failed.
    #[error(transparent)]
    Io(#[from] std::io::Error),

    #[error("no {entity} with id {id}")]
    NotFound { entity: &'static str, id: String },

    /// A stored timestamp was not RFC 3339. Only reachable if something wrote to
    /// the file outside this module.
    #[error("stored timestamp {0:?} is not RFC 3339")]
    Timestamp(String),
}

pub type DbResult<T> = Result<T, DbError>;

/// The migrations, embedded in the binary at compile time.
///
/// Embedding rather than reading `migrations/` at runtime is what makes the
/// binary self-contained: a deployment is one executable plus the SQLite file.
static MIGRATIONS: sqlx::migrate::Migrator = sqlx::migrate!("./migrations");

/// A handle to the database.
///
/// `Clone` because it is just a pool handle — cloning is how it gets into
/// Topcoat's app context in step 4.
#[derive(Clone)]
pub struct Db {
    pool: SqlitePool,
}

impl Db {
    /// Open (creating if absent) the database at `url` and bring the schema up
    /// to date.
    ///
    /// `url` is a sqlx SQLite URL: `sqlite://data/chansondufenua.db` on disk, or
    /// `sqlite::memory:` for tests.
    pub async fn open(url: &str) -> DbResult<Self> {
        let memory = is_memory(url);
        if !memory {
            // `create_if_missing` creates the *file*, not the directory holding
            // it, so a fresh clone would fail on the first run without this.
            ensure_parent_dir(url)?;
        }
        let pool = SqlitePoolOptions::new()
            // An in-memory database lives inside its connection, so a second
            // connection would silently get a *different*, empty database.
            // Pin the pool to one connection so the schema survives the pool.
            .max_connections(if memory { 1 } else { 8 })
            .min_connections(1)
            .idle_timeout(if memory {
                None
            } else {
                Some(Duration::from_secs(600))
            })
            .connect_with(options(url)?)
            .await?;

        MIGRATIONS.run(&pool).await?;

        Ok(Self { pool })
    }

    /// A throwaway in-memory database with the schema applied. For tests.
    pub async fn open_in_memory() -> DbResult<Self> {
        Self::open("sqlite::memory:").await
    }

    pub fn pool(&self) -> &SqlitePool {
        &self.pool
    }
}

fn is_memory(url: &str) -> bool {
    url.contains(":memory:")
}

/// Create the directory that will hold a file-backed database.
///
/// Only understands the URL shapes this application uses
/// (`sqlite://path`, `sqlite:path`, optional `?query`). Anything it cannot
/// parse is left alone rather than guessed at.
fn ensure_parent_dir(url: &str) -> std::io::Result<()> {
    let path = url
        .strip_prefix("sqlite://")
        .or_else(|| url.strip_prefix("sqlite:"))
        .unwrap_or(url);
    let path = path.split('?').next().unwrap_or(path);

    // Note: leading slashes are significant. sqlx reads everything after
    // `sqlite://` as the filename, so `sqlite:///var/lib/x.db` is absolute
    // (`/var/lib/x.db`) while `sqlite://data/x.db` is relative to the cwd.
    // Trimming the slash here would silently redefine one as the other.
    if path.is_empty() {
        return Ok(());
    }
    match std::path::Path::new(path).parent() {
        Some(parent) if !parent.as_os_str().is_empty() => std::fs::create_dir_all(parent),
        _ => Ok(()),
    }
}

fn options(url: &str) -> DbResult<SqliteConnectOptions> {
    let memory = is_memory(url);
    let base = SqliteConnectOptions::from_str(url)?
        .create_if_missing(true)
        // Off by default in SQLite, and `song_artist` relies on it to stop a
        // deleted song from leaving dangling credits behind.
        .foreign_keys(true)
        .busy_timeout(Duration::from_secs(5));

    Ok(if memory {
        base.journal_mode(SqliteJournalMode::Memory)
    } else {
        // WAL because every song page fires an `UPDATE ... view_count`. In
        // rollback-journal mode that write would block concurrent readers.
        base.journal_mode(SqliteJournalMode::Wal)
            .synchronous(SqliteSynchronous::Normal)
    })
}
