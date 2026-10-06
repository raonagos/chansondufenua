//! Pages.
//!
//! Every route carries its path in the attribute rather than deriving it from
//! its module path. That is not the Topcoat default, and it is deliberate: the
//! site's URLs do not mirror a sensible module tree. `home.rs` serves two paths
//! (`/` and `/aepa`), and later steps put `song.rs` and `editor.rs` under
//! `/himene` alongside `songs.rs`. Explicit paths keep the URLs legible in one
//! place instead of bending the module tree to fit them.
//!
//! The modules are *not* re-exported from here on purpose. A page is reached
//! through `href!`, which resolves a path such as `home::home` to the path the
//! handler declared; re-exporting the function would shadow its module and make
//! that path ambiguous.

pub mod artiste;
pub mod editor;
pub mod home;
pub mod pluriel;
pub mod recherche;
pub mod song;
pub mod songs;
pub mod support;

// Every URL that matches nothing else.
//
// Without this, a request for an unregistered path is answered by the *router*,
// and nothing else runs: not the layout, not the pages, not the `error_boundary`
// the layout installs. The response is Topcoat's bare `not found` — nine bytes
// of text with no chrome, and worse than v3, which at least rendered
// "La page n'existe pas." inside the site.
//
// `not_found!("/")` expands to a catch-all page (`GET /{*rest}`) that does
// nothing but fail with a [`NotFoundError`](topcoat::router::error::NotFoundError).
// Failing is the point: an error raised *by a handler* travels back through the
// layout, so the boundary catches it and the 404 gets the same chrome as every
// other page — while the status stays 404.
//
// This is the one place a route is written as a *pattern* rather than with
// `#[page("/…")]`, and it has to be: the macro exists precisely because a
// catch-all cannot be spelled as an ordinary page path without also catching
// the paths that do have pages.
topcoat::router::not_found!("/");
