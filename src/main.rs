//! Chanson du fenua — Tahitian songs with lyrics and chords.
//!
//! v4 is a single-binary rewrite: Topcoat for server-rendered HTML, an
//! embedded SQLite database, and no WASM or client build step.
//!
//! Step 1 of `PLAN.md`: the smallest thing that boots. The real app shell and
//! design tokens arrive in step 4.

use topcoat::{
    Result,
    asset::{AssetBundle, RouterBuilderAssetExt},
    router::{Router, RouterBuilderDiscoverExt, module_router, page},
    tailwind,
    view::{View, class, view},
};

#[tokio::main]
async fn main() {
    topcoat::start(router()).await.unwrap();
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
