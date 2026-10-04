//! Chanson du fenua — Tahitian songs with lyrics and chords.
//!
//! v4 is a single-binary rewrite: Topcoat for server-rendered HTML, an embedded
//! SQLite database, and no WASM or client build step.
//!
//! The binary does three things and nothing else: open the database, say what it
//! found, and serve [`chansondufenua::router`]. The shell and the pages are
//! library items so that `tests/` and the layout can name them.

use chansondufenua::db::{self, Db, SongOrder};

#[tokio::main]
async fn main() {
    let db = bootstrap_database().await;
    report(&db).await;

    topcoat::start(chansondufenua::router()).await.unwrap();
}

/// Open (and migrate) the database described by `DATABASE_URL`.
///
/// Failing loudly is right here: a renderer without its database serves 500s,
/// and a process that refuses to start is far easier to notice than one that
/// boots and lies.
async fn bootstrap_database() -> Db {
    let url = std::env::var("DATABASE_URL").unwrap_or_else(|_| db::DEFAULT_URL.to_owned());
    Db::open(&url)
        .await
        .unwrap_or_else(|error| panic!("cannot open database {url}: {error}"))
}

/// Prove the schema is usable at boot. The pages that read it arrive in step 5.
async fn report(db: &Db) {
    let songs = db::songs(db.pool(), SongOrder::Newest, None)
        .await
        .expect("reading songs");
    let artists = db::artists(db.pool()).await.expect("reading artists");
    eprintln!(
        "chansondufenua v{} — {} songs, {} artists",
        env!("CARGO_PKG_VERSION"),
        songs.len(),
        artists.len()
    );
}
