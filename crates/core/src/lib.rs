//! didaskalos-core: indexing and retrieval over an Obsidian theology vault.
//!
//! The vault stays the source of truth. Everything here reads it into a
//! disposable SQLite index (`db`), keeps that index current (`index`),
//! and answers searches against it (`search`).

pub mod db;
pub mod index;
pub mod note;
pub mod query;
pub mod search;
