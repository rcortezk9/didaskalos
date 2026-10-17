//! The SQLite index: one file under `~/.local/share/didaskalos/`.
//! It is a cache of the vault, so deleting it is always safe.

use std::path::{Path, PathBuf};

use anyhow::Result;
use rusqlite::Connection;

/// Bump when the schema changes; `open` then rebuilds from scratch.
const SCHEMA_VERSION: i64 = 3;

pub fn default_db_path() -> PathBuf {
    let data_home = std::env::var("XDG_DATA_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|_| {
            let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
            PathBuf::from(home).join(".local/share")
        });
    data_home.join("didaskalos").join("index.db")
}

pub fn open(path: &Path) -> Result<Connection> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let conn = Connection::open(path)?;
    conn.execute_batch("PRAGMA journal_mode = WAL; PRAGMA foreign_keys = ON;")?;

    let version: i64 = conn.query_row("PRAGMA user_version", [], |r| r.get(0))?;
    if version != SCHEMA_VERSION {
        reset(&conn)?;
    }
    Ok(conn)
}

/// Drop everything and create the current schema.
fn reset(conn: &Connection) -> Result<()> {
    conn.execute_batch(&format!(
        "DROP TABLE IF EXISTS chunks_vocab;
         DROP TABLE IF EXISTS chunk_vec;
         DROP TABLE IF EXISTS chunks_fts;
         DROP TABLE IF EXISTS links;
         DROP TABLE IF EXISTS chunks;
         DROP TABLE IF EXISTS notes;

         CREATE TABLE notes (
             id              INTEGER PRIMARY KEY,
             slug            TEXT NOT NULL UNIQUE,
             path            TEXT NOT NULL,
             title           TEXT NOT NULL,
             kind            TEXT NOT NULL,
             tags            TEXT NOT NULL,     -- space separated
             hash            TEXT NOT NULL,     -- blake3 of the file, for incremental updates
             recall_due      TEXT,
             recall_interval INTEGER,
             recall_streak   INTEGER
         );

         CREATE TABLE chunks (
             id           INTEGER PRIMARY KEY,
             note_id      INTEGER NOT NULL REFERENCES notes(id) ON DELETE CASCADE,
             ord          INTEGER NOT NULL,
             heading      TEXT NOT NULL,
             heading_path TEXT NOT NULL,
             text         TEXT NOT NULL
         );
         CREATE INDEX chunks_note ON chunks(note_id);

         -- One unit-length f32 vector (384 little-endian floats) per chunk,
         -- for meaning search. Missing rows just mean keyword-only for that chunk.
         CREATE TABLE chunk_vec (
             chunk_id INTEGER PRIMARY KEY REFERENCES chunks(id) ON DELETE CASCADE,
             vec      BLOB NOT NULL
         );

         -- Keyword index. rowid = chunks.id. Porter stemming so
         -- 'inherited' matches 'inheritance'; diacritics folded for Greek.
         CREATE VIRTUAL TABLE chunks_fts USING fts5(
             title, heading, body,
             tokenize = 'porter unicode61 remove_diacritics 2'
         );

         -- Every indexed word with its document count, for spelling fixes.
         CREATE VIRTUAL TABLE chunks_vocab USING fts5vocab(chunks_fts, 'row');

         CREATE TABLE links (
             src_id  INTEGER NOT NULL REFERENCES notes(id) ON DELETE CASCADE,
             target  TEXT NOT NULL,   -- slug; may point at a note that does not exist yet
             heading TEXT,
             kind    TEXT NOT NULL
         );
         CREATE INDEX links_src ON links(src_id);
         CREATE INDEX links_target ON links(target);

         PRAGMA user_version = {SCHEMA_VERSION};"
    ))?;
    Ok(())
}
