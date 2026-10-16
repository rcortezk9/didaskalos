//! Walk the vault and bring the index up to date. Only files whose
//! content hash changed are re-parsed, so a routine run takes seconds.

use std::collections::HashMap;
use std::path::Path;

use anyhow::{Context, Result};
use rusqlite::{params, Connection};
use walkdir::WalkDir;

use crate::note::{self, Note};

#[derive(Debug, Default)]
pub struct Report {
    pub scanned: usize,
    pub added: usize,
    pub updated: usize,
    pub removed: usize,
    pub unchanged: usize,
    pub chunks: usize,
}

/// Files the index reads. PDFs are skipped; their text lives in `.txt`
/// extractions or in the notes written from them.
fn is_indexable(path: &Path) -> bool {
    matches!(path.extension().and_then(|e| e.to_str()), Some("md") | Some("txt"))
}

/// Skip Obsidian config, git internals, trash and other dot folders.
fn is_hidden(entry: &walkdir::DirEntry) -> bool {
    entry.depth() > 0 && entry.file_name().to_string_lossy().starts_with('.')
}

pub fn update(conn: &mut Connection, vault: &Path, full: bool) -> Result<Report> {
    let mut report = Report::default();
    if full {
        conn.execute_batch("DELETE FROM chunks_fts; DELETE FROM links; DELETE FROM chunks; DELETE FROM notes;")?;
    }

    // path -> (id, hash) for what is already indexed.
    let mut known: HashMap<String, (i64, String)> = {
        let mut stmt = conn.prepare("SELECT path, id, hash FROM notes")?;
        let rows = stmt.query_map([], |r| Ok((r.get::<_, String>(0)?, (r.get(1)?, r.get(2)?))))?;
        rows.collect::<Result<_, _>>()?
    };

    let tx = conn.transaction()?;
    for entry in WalkDir::new(vault).into_iter().filter_entry(|e| !is_hidden(e)) {
        let entry = entry?;
        if !entry.file_type().is_file() || !is_indexable(entry.path()) {
            continue;
        }
        report.scanned += 1;

        let bytes = std::fs::read(entry.path())
            .with_context(|| format!("reading {}", entry.path().display()))?;
        let hash = blake3::hash(&bytes).to_hex().to_string();
        let rel = entry
            .path()
            .strip_prefix(vault)
            .unwrap_or(entry.path())
            .to_string_lossy()
            .replace('\\', "/");

        match known.remove(&rel) {
            Some((_, old_hash)) if old_hash == hash => {
                report.unchanged += 1;
                continue;
            }
            Some((id, _)) => {
                delete_note(&tx, id)?;
                report.updated += 1;
            }
            None => report.added += 1,
        }

        let text = String::from_utf8_lossy(&bytes);
        let mut parsed = note::parse(&rel, &text);
        // Two files can share a stem (`index.md` and `Wiki/index.md`).
        // The second one is keyed by its relative path.
        let taken: bool = tx.query_row(
            "SELECT EXISTS(SELECT 1 FROM notes WHERE slug = ?1)",
            [&parsed.slug],
            |r| r.get(0),
        )?;
        if taken {
            // `Raw/matins.txt` beside `Raw/matins.md` would still clash
            // without the extension, so keep the whole relative path.
            parsed.slug = rel.clone();
        }
        report.chunks += parsed.chunks.len();
        insert_note(&tx, &parsed, &hash)?;
    }

    // Anything left in `known` is no longer on disk.
    for (_, (id, _)) in known {
        delete_note(&tx, id)?;
        report.removed += 1;
    }
    tx.commit()?;
    Ok(report)
}

fn delete_note(conn: &Connection, id: i64) -> Result<()> {
    // FTS rows are not covered by the foreign-key cascade.
    conn.execute(
        "DELETE FROM chunks_fts WHERE rowid IN (SELECT id FROM chunks WHERE note_id = ?1)",
        [id],
    )?;
    conn.execute("DELETE FROM notes WHERE id = ?1", [id])?;
    Ok(())
}

fn insert_note(conn: &Connection, n: &Note, hash: &str) -> Result<()> {
    conn.execute(
        "INSERT INTO notes (slug, path, title, kind, tags, hash, recall_due, recall_interval, recall_streak)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
        params![
            n.slug,
            n.rel_path,
            n.title,
            n.kind.as_str(),
            n.tags.join(" "),
            hash,
            n.recall.due,
            n.recall.interval,
            n.recall.streak,
        ],
    )?;
    let note_id = conn.last_insert_rowid();

    let mut chunk_stmt = conn.prepare_cached(
        "INSERT INTO chunks (note_id, ord, heading, heading_path, text) VALUES (?1, ?2, ?3, ?4, ?5)",
    )?;
    let mut fts_stmt =
        conn.prepare_cached("INSERT INTO chunks_fts (rowid, title, heading, body) VALUES (?1, ?2, ?3, ?4)")?;
    for (ord, c) in n.chunks.iter().enumerate() {
        chunk_stmt.execute(params![note_id, ord as i64, c.heading, c.heading_path, c.text])?;
        let chunk_id = conn.last_insert_rowid();
        // Tags ride along in the title column so `book/thinking_orthodox`
        // or `soteriology` find the note by keyword.
        let title_field = format!("{} {}", n.title, n.tags.join(" ").replace('/', " "));
        fts_stmt.execute(params![chunk_id, title_field, c.heading_path, c.text])?;
    }

    let mut link_stmt =
        conn.prepare_cached("INSERT INTO links (src_id, target, heading, kind) VALUES (?1, ?2, ?3, ?4)")?;
    for l in &n.links {
        link_stmt.execute(params![note_id, l.target, l.heading, l.kind])?;
    }
    Ok(())
}

#[derive(Debug)]
pub struct Stats {
    pub notes: i64,
    pub chunks: i64,
    pub links: i64,
    pub dangling_links: i64,
    pub by_kind: Vec<(String, i64)>,
}

pub fn stats(conn: &Connection) -> Result<Stats> {
    let count = |sql: &str| conn.query_row(sql, [], |r| r.get::<_, i64>(0));
    let mut stmt = conn.prepare("SELECT kind, COUNT(*) FROM notes GROUP BY kind ORDER BY 2 DESC")?;
    let by_kind = stmt
        .query_map([], |r| Ok((r.get(0)?, r.get(1)?)))?
        .collect::<Result<_, _>>()?;
    Ok(Stats {
        notes: count("SELECT COUNT(*) FROM notes")?,
        chunks: count("SELECT COUNT(*) FROM chunks")?,
        links: count("SELECT COUNT(*) FROM links")?,
        dangling_links: count(
            "SELECT COUNT(*) FROM links WHERE target NOT IN (SELECT slug FROM notes)",
        )?,
        by_kind,
    })
}
