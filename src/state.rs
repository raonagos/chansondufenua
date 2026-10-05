//! The application's shared state, and the one place that names it.
//!
//! Topcoat's *app context* is a type-keyed bag built once at startup and
//! borrowed by every request. A handler reads an entry with
//! [`app_context::<T>`](topcoat::context::app_context), which **panics** when
//! nothing of type `T` was registered — so the key type is a promise the router
//! makes at boot and every caller bets on.
//!
//! Naming `Db` at each call site would spread that bet across the application.
//! It is named here exactly once, and everything else asks for [`db`] — the
//! shape Topcoat's own guide recommends ("wrap them in small application-specific
//! functions like `db(cx)`").
//!
//! This module also keeps [`crate::db`] free of Topcoat. `src/db/mod.rs` says it
//! is the one module allowed to know about SQLite; it should not also have to
//! know about the web framework. This is the seam between the two.

use topcoat::context::{Cx, app_context};

use crate::db::Db;

/// The database handle registered on the router by [`crate::router`].
///
/// Borrowed rather than cloned: [`Db`] is a pool handle, and the pool outlives
/// every request.
///
/// # Panics
///
/// Panics if the router was built without a database — i.e. if a handler is
/// driven by anything other than [`crate::router`]. That is a wiring mistake,
/// not a runtime condition, and the panic message names the missing type.
pub fn db(cx: &Cx) -> &Db {
    app_context::<Db>(cx)
}
