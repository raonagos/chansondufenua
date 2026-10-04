//! Chanson du fenua — Tahitian songs with lyrics and chords.
//!
//! v4 is a single-binary rewrite: Topcoat for server-rendered HTML, an
//! embedded SQLite database, and no WASM or client build step.
//!
//! Step 1 of `PLAN.md`: the smallest thing that boots. The real app shell and
//! design tokens arrive in step 4.

use chansondufenua::db::{self, Db, SongOrder};
use topcoat::{
    Result,
    asset::{AssetBundle, RouterBuilderAssetExt},
    router::{Router, RouterBuilderDiscoverExt, module_router, page},
    tailwind,
    view::{View, class, view},
};

/// Where the embedded database lives when `DATABASE_URL` is not set.
const DEFAULT_DATABASE_URL: &str = "sqlite://data/chansondufenua.db";

#[tokio::main]
async fn main() {
    let db = bootstrap_database().await;
    report(&db).await;

    topcoat::start(router()).await.unwrap();
}

/// Open (and migrate) the database described by `DATABASE_URL`.
///
/// Failing loudly is right here: a renderer without its database serves 500s,
/// and a process that refuses to start is far easier to notice than one that
/// boots and lies.
async fn bootstrap_database() -> Db {
    let url = std::env::var("DATABASE_URL").unwrap_or_else(|_| DEFAULT_DATABASE_URL.to_owned());
    Db::open(&url)
        .await
        .unwrap_or_else(|error| panic!("cannot open database {url}: {error}"))
}

/// Prove the schema is usable at boot. Replaced by the real pages in step 5.
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

/// The router is built from this module, so `#[page]` items declared here (and
/// in child modules) map onto `/`. Assets are loaded from `assets/` next to the
/// binary — run `topcoat asset bundle` after building.
fn router() -> Router {
    module_router!()
        .discover()
        .assets(AssetBundle::load().unwrap())
        .build()
}

#[page]
async fn home() -> Result<impl View> {
    Ok(view! {
        <!DOCTYPE html>
        <html lang="fr">
            <head>
                <meta charset="utf-8"/>
                <meta name="viewport" content="width=device-width, initial-scale=1"/>
                <title>"Chanson du fenua"</title>
                <link rel="stylesheet" href=(tailwind::stylesheet!())/>
            </head>
            <body class=(class!("bg-white", "text-slate-900"))>
                <main class=(class!("mx-auto", "max-w-3xl", "p-8"))>
                    <h1 class=(class!("text-3xl", "font-bold"))>"Chanson du fenua"</h1>
                    <p class=(class!("mt-4"))>"Tahitian songs with lyrics and chords."</p>
                </main>
            </body>
        </html>
    })
}
