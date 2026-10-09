//! Chanson du fenua — Tahitian songs with lyrics and chords.
//!
//! v4 is a single-binary rewrite: Topcoat for server-rendered HTML, an embedded
//! SQLite database, and no WASM or client build step.
//!
//! The binary opens the database, says what it found and where it is listening,
//! and serves [`chansondufenua::router`]. The shell and the pages are library
//! items so that `tests/` and the layout can name them.
//!
//! Every line below goes through [`chansondufenua::log`], so the boot sequence
//! and the per-request access log share one format and one `RUST_LOG` filter.
//! Nothing is written to stderr except a panic: a healthy process has one story,
//! and it is on stdout, where `journalctl` finds it with no environment set.

use chansondufenua::db::{self, Db, SongOrder};
use chansondufenua::{i18n, log};

#[tokio::main]
async fn main() {
    log::init();
    chrome();

    let url = database_url();
    let db = bootstrap_database(&url).await;
    let (songs, artists) = counts(&db).await;

    log::info(format_args!(
        "boot version={} database={} migrations={} songs={} artists={}",
        env!("CARGO_PKG_VERSION"),
        url,
        db::migration_count(),
        songs,
        artists,
    ));

    let listener = listen().await;
    log::info(format_args!(
        "listening addr={}",
        listener
            .local_addr()
            .expect("a bound listener has a local address")
    ));

    topcoat::serve(listener, chansondufenua::router(db))
        .await
        .expect("serving");
}

/// Read the chrome's catalog, and say where its words came from.
///
/// First thing after the log, and loud. The words are files now, not `match`
/// arms the compiler could check, so the boot is the one place left that can
/// refuse a catalog with a sentence missing or a line no key owns — and it has
/// to be here, before a page can render, because the alternative is a server
/// that serves key names to readers and looks healthy while it does it.
fn chrome() {
    let report = i18n::init();

    log::info(format_args!(
        "catalog dir={} keys={} {}",
        report.dir.display(),
        report.keys,
        report
            .words
            .iter()
            .map(|(lang, words)| format!("{}={words}", lang.code()))
            .collect::<Vec<_>>()
            .join(" "),
    ));
}

/// The database to open: `DATABASE_URL`, or the application's own default.
fn database_url() -> String {
    std::env::var("DATABASE_URL").unwrap_or_else(|_| db::DEFAULT_URL.to_owned())
}

/// Open (and migrate) the database.
///
/// Failing loudly is right here: a renderer without its database serves 500s,
/// and a process that refuses to start is far easier to notice than one that
/// boots and lies.
async fn bootstrap_database(url: &str) -> Db {
    Db::open(url)
        .await
        .unwrap_or_else(|error| panic!("cannot open database {url}: {error}"))
}

/// What the database holds, for the boot event.
///
/// Two counts, so a fresh clone that was never imported says `0 songs, 0
/// artists` on the first line it ever prints — much quicker to notice than an
/// empty page, and it survives the deployment where nobody watches the site.
async fn counts(db: &Db) -> (usize, usize) {
    let songs = db::songs(db.pool(), SongOrder::Newest, None)
        .await
        .expect("reading songs");
    let artists = db::artists(db.pool()).await.expect("reading artists");

    (songs.len(), artists.len())
}

/// Bind the socket, so the log can name the address the OS actually gave us.
///
/// `topcoat::start` would read `HOST`/`PORT` and bind for us, but it reports no
/// address — and with `PORT=0` the address it binds is not the one that was
/// asked for. Binding here keeps the framework's two variables and its defaults
/// (`127.0.0.1:3000`) and lets the `listening` line be evidence rather than a
/// restatement of the request: it is printed only once the socket is real, so
/// its absence is a failure and not a missing log line.
async fn listen() -> tokio::net::TcpListener {
    let host = std::env::var("HOST").unwrap_or_else(|_| "127.0.0.1".to_owned());
    let port = std::env::var("PORT").unwrap_or_else(|_| "3000".to_owned());
    let port: u16 = port
        .parse()
        .unwrap_or_else(|error| panic!("PORT must be a port number, not {port:?}: {error}"));

    tokio::net::TcpListener::bind((host.as_str(), port))
        .await
        .unwrap_or_else(|error| panic!("cannot bind {host}:{port}: {error}"))
}
