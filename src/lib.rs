//! Chanson du fenua — Tahitian songs with lyrics and chords.
//!
//! The crate is both a library and a binary, and that is deliberate:
//!
//! * the library lets `tests/` drive the database layer against a real
//!   file-backed database (migrations included), and
//! * it lets the one-shot importer in `examples/import.rs` reuse the same code
//!   path the application uses.
//!
//! It is still **one crate**, not a workspace. v3's seven-crate hexagon was
//! collapsed in step 1 — see `PLAN.md` §2.1 for why.
//!
//! The route tree lives here rather than in `main.rs` because the layout in
//! [`ui`] and the pages in [`pages`] are library items: a `href!` from the shell
//! to a page has to resolve to something both halves can name.

pub mod db;
pub mod domain;
pub mod i18n;
pub mod pages;
pub mod state;
pub mod ui;

use topcoat::{
    asset::{AssetBundle, RouterBuilderAssetExt},
    cookie::RouterBuilderCookieExt,
    router::{Router, RouterBuilderDiscoverExt},
};

use crate::db::Db;

/// Build the application router.
///
/// Every handler declares its own path (`#[page("/himene")]`, `#[layout("/")]`)
/// and `discover()` collects them at link time, so registering a page is adding
/// a function — there is no route table to keep in step.
///
/// The database is registered as the router's *app context* rather than handed
/// to each page: a handler reaches it through [`state::db`], so no page
/// signature grows a parameter for it and no call site has to thread one.
///
/// `AssetBundle::load()` reads the `assets/` directory that
/// `topcoat asset bundle` writes next to the binary, so the build is
/// `cargo build && topcoat asset bundle && ./target/debug/chansondufenua`.
/// `cargo run` alone panics here.
///
/// `.cookies()` registers the request-scoped cookie jar. It is what
/// [`i18n::resolve`] reads a remembered language choice from, and what writes
/// one back when a visitor asks for a language by name — without the layer,
/// `cookies(cx)` panics rather than quietly returning nothing.
pub fn router(db: Db) -> Router {
    Router::builder()
        .discover()
        .cookies()
        .assets(AssetBundle::load().unwrap())
        .app_context(db)
        .build()
}
