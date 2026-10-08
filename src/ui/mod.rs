//! The application shell: layout, chrome, the reader's own controls, and the
//! design tokens they compose.
//!
//! v3 built the same thing out of Leptos components (`app/src/components/`) plus
//! two SCSS files. Here it is one module of Rust and one palette declaration —
//! no stylesheet of rules, no `@apply`, no `.css` file anywhere in the tree, and
//! no build step beyond Tailwind's own.

pub mod assets;
pub mod autoscroll;
pub mod flags;
pub mod fonts;
pub mod icons;
pub mod layout;
pub mod palette;
pub mod share;
pub mod theme;

pub use layout::root_layout;
