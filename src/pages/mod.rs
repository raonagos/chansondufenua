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

pub mod home;
pub mod songs;
