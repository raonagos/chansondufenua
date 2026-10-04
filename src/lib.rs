//! Chanson du fenua — Tahitian songs with lyrics and chords.
//!
//! The crate is both a library and a binary, and that is deliberate:
//!
//! * the library lets `tests/` drive the database layer against a real
//!   file-backed database (migrations included), and
//! * it lets the one-shot importer in `src/bin/import.rs` reuse the same code
//!   path the application uses.
//!
//! It is still **one crate**, not a workspace. v3's seven-crate hexagon was
//! collapsed in step 1 — see `PLAN.md` §2.1 for why.

pub mod db;
pub mod domain;
