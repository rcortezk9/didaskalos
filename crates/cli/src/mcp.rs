//! `dsk mcp`: the vault as tools for Claude Code, over the Model Context
//! Protocol on stdin/stdout.
//!
//! Analogy: Claude Code is the student at the desk; this server is the
//! librarian who only ever hands over your own notes. Claude asks
//! ("search for theosis", "open this note", "what links here?") and
//! writes the answer; every claim it makes can be traced to a note.
//!
//! The protocol is JSON-RPC 2.0, one message per line. Only the parts
//! Claude Code needs are here: `initialize`, `tools/list`, `tools/call`
//! and `ping`. Logs go to stderr, because stdout carries the protocol.

use std::io::{BufRead, Write};
use std::path::{Path, PathBuf};

use anyhow::{anyhow, Context, Result};
use didaskalos_core::search;
use regex::Regex;
use rusqlite::{Connection, OptionalExtension};
use serde_json::{json, Value};

/// Full text kept per matching section in `search` results. Enough to
/// quote from without flooding Claude's context.
const SECTION_CHARS: usize = 1500;
/// Sections returned per note in `search` results.
const SECTIONS_PER_HIT: usize = 2;
/// Most of a note `read_note` returns in one call.
const NOTE_CHARS: usize = 40_000;

pub struct Server {
    conn: Connection,
    vault: PathBuf,
}

impl Server {
    pub fn new(conn: Connection, vault: PathBuf) -> Self {
        Self { conn, vault }
    }

    /// Read requests until stdin closes, answering each on stdout.
    pub fn run(&mut self) -> Result<()> {
        let stdin = std::io::stdin();
        let mut stdout = std::io::stdout().lock();
        for line in stdin.lock().lines() {
            let line = line?;
            if line.trim().is_empty() {
                continue;
            }
            if let Some(reply) = self.handle_line(&line) {
                writeln!(stdout, "{reply}")?;
                stdout.flush()?;
            }
        }
        Ok(())
    }

    /// One incoming line in, at most one outgoing line back.
    /// Notifications (no `id`) get no reply.
    pub fn handle_line(&mut self, line: &str) -> Option<String> {
        let msg: Value = match serde_json::from_str(line) {
            Ok(v) => v,
            Err(e) => return Some(error(Value::Null, -32700, &format!("parse error: {e}")).to_string()),
        };
        let id = msg.get("id").cloned();
        let method = msg.get("method").and_then(Value::as_str).unwrap_or("");
        let params = msg.get("params").cloned().unwrap_or(Value::Null);

        let result = match method {
            "initialize" => Ok(initialize(&params)),
            "ping" => Ok(json!({})),
            "tools/list" => Ok(json!({ "tools": tool_list() })),
            "tools/call" => Ok(self.call(&params)),
            // Notifications such as `notifications/initialized`.
            _ if id.is_none() => return None,
            _ => Err((-32601, format!("method not found: {method}"))),
        };
        let id = id?;
        Some(match result {
            Ok(r) => json!({ "jsonrpc": "2.0", "id": id, "result": r }).to_string(),
            Err((code, m)) => error(id, code, &m).to_string(),
        })
    }

    /// Run a tool. Tool failures are reported inside the result
    /// (`isError`), so Claude sees the message and can try again.
    fn call(&mut self, params: &Value) -> Value {
        let name = params.get("name").and_then(Value::as_str).unwrap_or("");
        let args = params.get("arguments").cloned().unwrap_or(json!({}));
        let out = match name {
            "search" => self.search(&args),
            "read_note" => self.read_note(&args),
            "neighbors" => self.neighbors(&args),
            "lessons" => self.lessons(&args),
            "check_citations" => self.check_citations(&args),
            _ => Err(anyhow!("unknown tool: {name}")),
        };
        match out {
            Ok(text) => json!({ "content": [{ "type": "text", "text": text }], "isError": false }),
            Err(e) => json!({ "content": [{ "type": "text", "text": format!("{e:#}") }], "isError": true }),
        }
    }

    /// Ranked notes with the full text of their best sections, each
    /// headed by the citation to use for it.
    fn search(&self, args: &Value) -> Result<String> {
        let query = str_arg(args, "query")?;
        let limit = args.get("limit").and_then(Value::as_u64).unwrap_or(6).clamp(1, 20) as usize;
        let res = search::search(&self.conn, query, limit)?;

        let mut out = String::new();
        if !res.corrections.is_empty() {
            out.push_str(&format!("(searched with {})\n\n", res.corrections.join(", ")));
        }
        if res.hits.is_empty() {
            out.push_str(&format!("No notes match \"{query}\". Try other words, or say the vault has nothing on it."));
            return Ok(out);
        }
        let mut stmt = self.conn.prepare_cached(
            "SELECT c.text FROM chunks c JOIN notes n ON n.id = c.note_id
             WHERE n.slug = ?1 AND c.heading_path = ?2 ORDER BY c.ord LIMIT 1",
        )?;
        for (i, hit) in res.hits.iter().enumerate() {
            out.push_str(&format!("## {}. {} ({}, score {:.1})\nnote: [[{}]]  file: {}\n",
                i + 1, hit.title, hit.kind, hit.score, hit.slug, hit.path));
            if let Some(via) = &hit.linked_with {
                out.push_str(&format!("linked with: {via}\n"));
            }
            for c in hit.chunks.iter().take(SECTIONS_PER_HIT) {
                let text: Option<String> =
                    stmt.query_row([&hit.slug, &c.heading_path], |r| r.get(0)).optional()?;
                let text = text.unwrap_or_else(|| c.snippet.clone());
                out.push_str(&format!("\n### {}\n{}\n", hit.cite(c), clip(&text, SECTION_CHARS)));
            }
            out.push('\n');
        }
        if !res.related.is_empty() {
            out.push_str("Linked from these results (not matched by the words; open with read_note if useful):\n");
            for r in &res.related {
                out.push_str(&format!("- [[{}]] {}\n", r.slug, r.title));
            }
        }
        Ok(out)
    }

    /// A whole note, or one section of it by heading.
    fn read_note(&self, args: &Value) -> Result<String> {
        let slug = normalize_slug(str_arg(args, "slug")?);
        let (slug, title, path) = self.find_note(&slug)?;
        let raw = std::fs::read_to_string(self.vault.join(&path))
            .with_context(|| format!("reading {path} in the vault"))?;

        let body = match args.get("heading").and_then(Value::as_str).filter(|h| !h.trim().is_empty()) {
            Some(h) => section(&raw, h).ok_or_else(|| {
                anyhow!("[[{slug}]] has no heading \"{h}\". Its headings: {}", headings(&raw).join(" | "))
            })?,
            None => raw,
        };
        let total = body.chars().count();
        let mut out = format!("# {title}\nnote: [[{slug}]]  file: {path}\n\n{}", clip(&body, NOTE_CHARS));
        if total > NOTE_CHARS {
            out.push_str(&format!(
                "\n\n(cut at {NOTE_CHARS} of {total} characters; ask for a heading to read the rest: {})",
                headings(&body).join(" | ")
            ));
        }
        Ok(out)
    }

    /// What a note links to, and what links to it.
    fn neighbors(&self, args: &Value) -> Result<String> {
        let slug = normalize_slug(str_arg(args, "slug")?);
        let (slug, title, _) = self.find_note(&slug)?;

        let mut out = format!("# {title} [[{slug}]]\n\nLinks out:\n");
        let mut stmt = self.conn.prepare_cached(
            "SELECT DISTINCT l.target, t.title FROM links l
             JOIN notes s ON s.id = l.src_id
             LEFT JOIN notes t ON t.slug = l.target COLLATE NOCASE
             WHERE s.slug = ?1 ORDER BY l.target",
        )?;
        let rows: Vec<(String, Option<String>)> =
            stmt.query_map([&slug], |r| Ok((r.get(0)?, r.get(1)?)))?.collect::<Result<_, _>>()?;
        if rows.is_empty() {
            out.push_str("- (none)\n");
        }
        for (target, t) in rows {
            match t {
                Some(t) => out.push_str(&format!("- [[{target}]] {t}\n")),
                None => out.push_str(&format!("- [[{target}]] (no such note yet)\n")),
            }
        }

        out.push_str("\nLinked from:\n");
        let mut stmt = self.conn.prepare_cached(
            "SELECT DISTINCT s.slug, s.title, s.kind FROM links l JOIN notes s ON s.id = l.src_id
             WHERE l.target = ?1 COLLATE NOCASE ORDER BY s.kind, s.slug",
        )?;
        let rows: Vec<(String, String, String)> =
            stmt.query_map([&slug], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))?.collect::<Result<_, _>>()?;
        if rows.is_empty() {
            out.push_str("- (none)\n");
        }
        for (s, t, k) in rows {
            out.push_str(&format!("- [[{s}]] {t} ({k})\n"));
        }
        Ok(out)
    }

    /// Rows of `lessons_index.md`, optionally filtered by words.
    fn lessons(&self, args: &Value) -> Result<String> {
        let (_, _, path) = self.find_note("lessons_index")?;
        let raw = std::fs::read_to_string(self.vault.join(&path))?;
        let filter: Vec<String> = args
            .get("filter")
            .and_then(Value::as_str)
            .map(|f| f.split_whitespace().map(str::to_lowercase).collect())
            .unwrap_or_default();

        let rows: Vec<&str> = raw
            .lines()
            .filter(|l| l.trim_start().starts_with('|'))
            // Skip the header row and the |---| rule under it.
            .filter(|l| !l.contains("---") && !l.to_lowercase().contains("| date |"))
            .filter(|l| {
                let low = l.to_lowercase();
                filter.iter().all(|w| low.contains(w.as_str()))
            })
            .collect();
        if rows.is_empty() {
            return Ok(format!("No lessons in {path} match. Lessons listed there are written with /lesson."));
        }
        let mut out = format!("{} lessons from {path} (Date | Title | Source | Hook | Link):\n", rows.len());
        for r in rows {
            out.push_str(r.trim());
            out.push('\n');
        }
        Ok(out)
    }

    /// Check every `[[slug]]` and `[[slug#Heading]]` in a draft answer
    /// against the vault, so no citation points at nothing.
    fn check_citations(&self, args: &Value) -> Result<String> {
        let text = str_arg(args, "text")?;
        let re = Regex::new(r"\[\[([^\]|#]+)(?:#([^\]|]+))?(?:\|[^\]]*)?\]\]").expect("valid regex");

        let mut seen = std::collections::HashSet::new();
        let (mut ok, mut bad) = (Vec::new(), Vec::new());
        for cap in re.captures_iter(text) {
            let whole = cap[0].to_string();
            if !seen.insert(whole.clone()) {
                continue;
            }
            let slug = normalize_slug(&cap[1]);
            let heading = cap.get(2).map(|m| m.as_str().trim().to_string());
            let note = match self.find_note(&slug) {
                Ok(n) => n,
                Err(_) => {
                    bad.push(format!("{whole}: no note named \"{slug}\""));
                    continue;
                }
            };
            match heading {
                None => ok.push(whole),
                Some(h) => {
                    let raw = std::fs::read_to_string(self.vault.join(&note.2)).unwrap_or_default();
                    if section(&raw, &h).is_some() {
                        ok.push(whole);
                    } else {
                        bad.push(format!(
                            "{whole}: [[{}]] has no heading \"{h}\". Its headings: {}",
                            note.0,
                            headings(&raw).join(" | ")
                        ));
                    }
                }
            }
        }
        if ok.is_empty() && bad.is_empty() {
            return Ok("No [[citations]] found in the text.".into());
        }
        let mut out = format!("{} citations resolve, {} do not.\n", ok.len(), bad.len());
        for b in &bad {
            out.push_str(&format!("MISSING {b}\n"));
        }
        for o in &ok {
            out.push_str(&format!("ok {o}\n"));
        }
        Ok(out)
    }

    /// (slug, title, path) for a slug, ignoring case like Obsidian does.
    fn find_note(&self, slug: &str) -> Result<(String, String, String)> {
        self.conn
            .query_row(
                "SELECT slug, title, path FROM notes WHERE slug = ?1 COLLATE NOCASE
                 ORDER BY slug = ?1 DESC LIMIT 1",
                [slug],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
            )
            .optional()?
            .ok_or_else(|| anyhow!("no note named \"{slug}\" (search first to find its slug)"))
    }
}

fn initialize(params: &Value) -> Value {
    // Speak whichever protocol version the client asked for; the subset
    // used here is the same across them.
    let version = params.get("protocolVersion").and_then(Value::as_str).unwrap_or("2025-06-18");
    json!({
        "protocolVersion": version,
        "capabilities": { "tools": {} },
        "serverInfo": { "name": "didaskalos", "version": env!("CARGO_PKG_VERSION") },
        "instructions": "Tools over the user's Orthodox theology Obsidian vault. Search before answering, \
            cite sections as [[slug#Heading]] exactly as search shows them, and run check_citations on a draft."
    })
}

fn tool_list() -> Value {
    json!([
        {
            "name": "search",
            "description": "Search the theology vault. Returns the best notes, each with the full text of its \
                best-matching sections headed by the citation to use ([[slug#Heading]]), plus notes linked to \
                the results. Spelling and abbreviations (PSA, LXX, HRM) are fixed automatically. Run several \
                searches with different wording for a broad question.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "query": { "type": "string", "description": "A question or keywords, e.g. \"ancestral sin vs inherited guilt\"" },
                    "limit": { "type": "integer", "description": "Notes to return (default 6, max 20)" }
                },
                "required": ["query"]
            }
        },
        {
            "name": "read_note",
            "description": "Read a vault note in full, or one section of it by heading. Use when a search \
                section is cut off or you need the surrounding argument.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "slug": { "type": "string", "description": "Note slug as in [[slug]], e.g. concept_theosis" },
                    "heading": { "type": "string", "description": "Optional heading text, as in [[slug#Heading]]" }
                },
                "required": ["slug"]
            }
        },
        {
            "name": "neighbors",
            "description": "List the notes a note links to and the notes that link to it. Use to follow \
                the user's own connections between topics.",
            "inputSchema": {
                "type": "object",
                "properties": { "slug": { "type": "string" } },
                "required": ["slug"]
            }
        },
        {
            "name": "lessons",
            "description": "List lessons from lessons_index.md (date, title, source, hook, link), optionally \
                filtered by words. Use to find a lesson the user remembers, or to avoid duplicating one.",
            "inputSchema": {
                "type": "object",
                "properties": { "filter": { "type": "string", "description": "Words that must all appear in the row" } }
            }
        },
        {
            "name": "check_citations",
            "description": "Check every [[slug]] and [[slug#Heading]] in a draft answer against the vault. \
                Reports any that point at a missing note or heading, with the note's real headings.",
            "inputSchema": {
                "type": "object",
                "properties": { "text": { "type": "string", "description": "The draft answer" } },
                "required": ["text"]
            }
        }
    ])
}

fn error(id: Value, code: i64, message: &str) -> Value {
    json!({ "jsonrpc": "2.0", "id": id, "error": { "code": code, "message": message } })
}

fn str_arg<'a>(args: &'a Value, key: &str) -> Result<&'a str> {
    args.get(key)
        .and_then(Value::as_str)
        .filter(|s| !s.trim().is_empty())
        .ok_or_else(|| anyhow!("missing \"{key}\""))
}

/// `Wiki/concept_theosis.md` and `[[concept_theosis]]` both mean `concept_theosis`.
fn normalize_slug(s: &str) -> String {
    let s = s.trim().trim_start_matches("[[").trim_end_matches("]]");
    let s = s.split('|').next().unwrap_or(s).split('#').next().unwrap_or(s);
    let s = s.rsplit('/').next().unwrap_or(s);
    s.strip_suffix(".md").unwrap_or(s).trim().to_string()
}

/// Markdown heading lines outside code fences, as (level, text).
fn heading_lines(raw: &str) -> Vec<(usize, usize, String)> {
    let mut out = Vec::new();
    let mut fenced = false;
    for (i, line) in raw.lines().enumerate() {
        if line.trim_start().starts_with("```") {
            fenced = !fenced;
            continue;
        }
        if fenced {
            continue;
        }
        let level = line.chars().take_while(|&c| c == '#').count();
        if (1..=6).contains(&level) && line[level..].starts_with(' ') {
            out.push((i, level, line[level..].trim().to_string()));
        }
    }
    out
}

fn headings(raw: &str) -> Vec<String> {
    heading_lines(raw).into_iter().map(|(_, _, h)| h).collect()
}

/// The text under a heading, up to the next heading of the same or a
/// higher level. Matches the way Obsidian resolves `#Heading` links:
/// case-insensitive, ignoring surrounding spaces.
fn section(raw: &str, heading: &str) -> Option<String> {
    let want = heading.trim().to_lowercase();
    let hs = heading_lines(raw);
    let pos = hs.iter().position(|(_, _, h)| h.to_lowercase() == want)?;
    let (start, level, _) = hs[pos];
    let end = hs[pos + 1..]
        .iter()
        .find(|(_, l, _)| *l <= level)
        .map(|(i, _, _)| *i)
        .unwrap_or(usize::MAX);
    let lines: Vec<&str> = raw.lines().skip(start).take(end.saturating_sub(start)).collect();
    Some(lines.join("\n"))
}

fn clip(text: &str, max: usize) -> String {
    if text.chars().count() <= max {
        text.to_string()
    } else {
        format!("{} …", text.chars().take(max).collect::<String>().trim_end())
    }
}

/// Bring the keyword index up to date before serving, so notes written
/// since the last `dsk index` are searchable. Vectors for them come with
/// the next `dsk index`.
pub fn refresh(conn: &mut Connection, vault: &Path) -> Result<()> {
    let r = didaskalos_core::index::update(conn, vault, false)?;
    if r.added + r.updated + r.removed > 0 {
        eprintln!("dsk mcp: index refreshed ({} added, {} updated, {} removed)", r.added, r.updated, r.removed);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    const NOTE: &str = "# Theosis\nintro\n## How\nprayer\n### Detail\nmore\n## Why\nlove\n";

    #[test]
    fn sections_stop_at_same_level() {
        assert_eq!(section(NOTE, "how").unwrap(), "## How\nprayer\n### Detail\nmore");
        assert_eq!(section(NOTE, " Why ").unwrap(), "## Why\nlove");
        assert!(section(NOTE, "nope").is_none());
    }

    #[test]
    fn slugs_normalize() {
        assert_eq!(normalize_slug("Wiki/concept_theosis.md"), "concept_theosis");
        assert_eq!(normalize_slug("[[concept_theosis#How|theosis]]"), "concept_theosis");
    }

    #[test]
    fn protocol_basics() {
        let conn = Connection::open_in_memory().unwrap();
        let mut s = Server::new(conn, PathBuf::from("."));
        let init = s.handle_line(r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-03-26"}}"#).unwrap();
        assert!(init.contains("2025-03-26") && init.contains("didaskalos"));
        assert!(s.handle_line(r#"{"jsonrpc":"2.0","method":"notifications/initialized"}"#).is_none());
        let list = s.handle_line(r#"{"jsonrpc":"2.0","id":2,"method":"tools/list"}"#).unwrap();
        assert!(list.contains("check_citations"));
        let missing = s.handle_line(r#"{"jsonrpc":"2.0","id":3,"method":"nope"}"#).unwrap();
        assert!(missing.contains("-32601"));
    }
}
