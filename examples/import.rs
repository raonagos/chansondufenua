//! One-shot importer: a `surreal export` dump → the embedded SQLite database.
//!
//! ```text
//! cargo run --example import -- <dump.surrealql> [database-url]
//! ```
//!
//! The database URL defaults to `$DATABASE_URL`, then to `db::DEFAULT_URL`.
//! The import is idempotent, so a corrected dump can simply be re-imported.
//!
//! This is a *development* tool and is not part of the deployed artifact: the
//! migration happens once, on the maintainer's side.
//!
//! It lives in `examples/` rather than `src/bin/` on purpose. `topcoat asset
//! bundle` scans `target/debug/` for the binary to read assets out of, and a
//! second binary there makes it refuse to guess:
//!
//! ```text
//! cargo produced multiple targets; pass --bin or --package to choose one
//! ```
//!
//! `default-run` does not settle it — topcoat-cli 0.10.0 ignores it. An example
//! is built into `target/debug/examples/`, so the deploy path stays
//! `cargo build && topcoat asset bundle && ./target/debug/chansondufenua`, with
//! nothing extra to remember.

use std::process::ExitCode;

use chansondufenua::db::{self, Db};

const USAGE: &str = "\
import a SurrealDB export into the embedded SQLite database

usage: import <dump.surrealql> [database-url]

  database-url   defaults to $DATABASE_URL, then to the application default
";

#[tokio::main]
async fn main() -> ExitCode {
    let mut args = std::env::args().skip(1);
    let Some(path) = args.next() else {
        eprint!("{USAGE}");
        return ExitCode::FAILURE;
    };
    if path == "-h" || path == "--help" {
        print!("{USAGE}");
        return ExitCode::SUCCESS;
    }

    let url = args.next().unwrap_or_else(|| {
        std::env::var("DATABASE_URL").unwrap_or_else(|_| db::DEFAULT_URL.to_owned())
    });

    match run(&path, &url).await {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("import failed: {error}");
            ExitCode::FAILURE
        }
    }
}

async fn run(path: &str, url: &str) -> Result<(), Box<dyn std::error::Error>> {
    let source =
        std::fs::read_to_string(path).map_err(|error| format!("cannot read {path}: {error}"))?;

    // Parse before opening the database: a malformed dump should not touch —
    // or create — the real file.
    let dump = db::Dump::parse(&source)?;
    println!(
        "parsed {path}: {} artists, {} songs, {} user rows (not imported)",
        dump.artists.len(),
        dump.songs.len(),
        dump.user_rows
    );

    let db = Db::open(url).await?;
    let report = db::import::load(db.pool(), &dump).await?;

    println!(
        "wrote {url}: {} artists, {} songs, {} credits ({} user rows skipped)",
        report.artists, report.songs, report.credits, report.user_rows_skipped
    );

    let songs = db::songs(db.pool(), db::SongOrder::Newest, None).await?;
    let artists = db::artists(db.pool()).await?;
    println!(
        "read back: {} published songs, {} artists",
        songs.len(),
        artists.len()
    );
    if let Some(newest) = songs.first() {
        println!(
            "newest: {} ({})",
            newest.get_title(),
            newest.get_created_at().to_rfc3339()
        );
    }

    Ok(())
}
