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
//! collapsed in step 1: ports-and-adapters pays off when several
//! interchangeable adapters sit behind one core, and this app has one of each.
//!
//! The route tree lives here rather than in `main.rs` because the layout in
//! [`ui`] and the pages in [`pages`] are library items: a `href!` from the shell
//! to a page has to resolve to something both halves can name.

pub mod db;
pub mod domain;
pub mod i18n;
pub mod log;
pub mod pages;
pub mod routes;
pub mod state;
pub mod ui;

use topcoat::{
    asset::{AssetBundle, RouterBuilderAssetExt},
    router::{OriginPolicy, Router, RouterBuilderDiscoverExt},
};

use crate::db::Db;
use crate::routes::{language, mcp, negotiation};

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
/// The cookie jar is deliberately **not** registered. The language is read from
/// the request's own `Cookie` header by [`crate::i18n::resolve`], and the one
/// request that *writes* a cookie — the language switcher's
/// `/reo/{code}` ([`language`]) — sets its own `Set-Cookie`. Registering a
/// jar nothing reads would be wiring kept for a decision that moved.
///
/// `.layer(...)` registers the three layers the site has, by hand rather than
/// discovered. `#[layer]` always carries a path; all of these are *pathless* on
/// purpose, and none is a route:
///
/// * [`negotiation::Negotiation`] answers `Accept: text/markdown` with a Markdown
///   document instead of the page, and puts the `Link` headers on the HTML
///   responses it passes through. Registered because a site that advertises a
///   Markdown form has to serve one.
/// * [`language::LanguageLayer`] retires the two spellings that used to name a
///   language — the `/fr`/`/ty`/`/en` prefixes and `?lang=` — with a `301` to the
///   address each page has now, and puts `Vary: Cookie, Accept-Language` on every
///   page, because the same URL answers different readers in different languages.
///   It runs *outside* the negotiator, so a Markdown response carries the `Vary`
///   too, and outside every route, so a prefixed path is answered before the
///   router has to have an opinion about it.
/// * [`log::AccessLog`] writes one access line per request. Registered *after*
///   the other two, and that ordering is the point — among layers sharing a path
///   the later one runs first, so the log sits outside the language rewrite and
///   reports the URL the reader asked for, once, with the final status.
///
/// `.origin_policy(...)` keeps the default (state-changing browser requests from
/// other origins are refused) and exempts [`mcp::PATH`]. A browser-based MCP
/// client sends an `Origin`, and the default would answer its `POST` with a 403;
/// the exemption is safe because `/mcp` is read-only — there is no state for a
/// forged request to change. The create-song form's `POST` stays protected.
pub fn router(db: Db) -> Router {
    Router::builder()
        .discover()
        .assets(AssetBundle::load().unwrap())
        .app_context(db)
        .origin_policy(OriginPolicy::new().exempt_paths([mcp::PATH]))
        .layer(negotiation::Negotiation)
        .layer(language::LanguageLayer)
        .layer(log::AccessLog)
        .build()
}
