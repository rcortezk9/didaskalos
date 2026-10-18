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
            "lesson_format" => lesson_format(&args),
            "save_lesson" => self.save_lesson(&args),
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

    /// Write a lesson note into the vault, add or update its row in
    /// `lessons_index.md`, and re-index so it is searchable at once.
    /// Any file it replaces is copied to `.didaskalos/backups/` first.
    fn save_lesson(&mut self, args: &Value) -> Result<String> {
        let rel = vault_rel_path(str_arg(args, "path")?)?;
        let content = str_arg(args, "content")?;
        if !content.trim_start().starts_with("---") {
            return Err(anyhow!("content must start with YAML frontmatter (---)"));
        }
        let overwrite = args.get("overwrite").and_then(Value::as_bool).unwrap_or(false);
        let slug = normalize_slug(&rel);

        // Wikilinks are by bare slug, so a second note with the same file
        // name elsewhere would make [[slug]] ambiguous.
        if let Ok((_, _, existing)) = self.find_note(&slug) {
            if existing != rel {
                return Err(anyhow!("a note named \"{slug}\" already exists at {existing}; pick another file name or save to that path"));
            }
        }
        let full = self.vault.join(&rel);
        let mut notes = Vec::new();
        if full.exists() {
            if !overwrite {
                return Err(anyhow!("{rel} already exists; pass overwrite: true to replace it (the old file is backed up)"));
            }
            notes.push(format!("old version backed up to {}", self.backup(&rel)?));
        }
        if let Some(parent) = full.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let mut body = content.trim_end().to_string();
        body.push('\n');
        std::fs::write(&full, body).with_context(|| format!("writing {rel}"))?;

        if let Some(row) = args.get("index_row").filter(|v| v.is_object()) {
            let field = |k: &str| -> Result<String> {
                Ok(str_arg(row, k)?.replace('|', "/").replace('\n', " ").trim().to_string())
            };
            let line = format!(
                "| {} | {} | {} | {} | [[{slug}]] |",
                field("date")?, field("title")?, field("source")?, field("hook")?
            );
            notes.push(self.upsert_lesson_row(&slug, &line)?);
        }

        mcp_reindex(&mut self.conn, &self.vault)?;
        let check = self.check_citations(&json!({ "text": content }))?;
        let mut out = format!("Saved [[{slug}]] at {rel}.\n");
        for n in notes {
            out.push_str(&format!("- {n}\n"));
        }
        if check.contains("MISSING") {
            out.push_str(&format!("\nSome citations do not resolve; fix them and save again with overwrite: true.\n{check}"));
        }
        Ok(out)
    }

    /// Put `line` in the lessons table: replace the row linking to
    /// `[[slug]]` if there is one, otherwise add it after the last row.
    fn upsert_lesson_row(&self, slug: &str, line: &str) -> Result<String> {
        let rel = match self.find_note("lessons_index") {
            Ok((_, _, p)) => p,
            Err(_) => "lessons_index.md".to_string(),
        };
        let full = self.vault.join(&rel);
        let raw = std::fs::read_to_string(&full).unwrap_or_else(|_| {
            "---\ntitle: \"Lessons Index\"\ntags: [index, lessons]\n---\n\n# Lessons Index\n\n\
             | Date | Title | Source | Hook | Link |\n|------|-------|--------|------|------|\n"
                .to_string()
        });
        let link = format!("[[{slug}]]");
        let mut lines: Vec<String> = raw.lines().map(str::to_string).collect();
        let what = if let Some(i) = lines.iter().position(|l| l.contains(&link)) {
            lines[i] = line.to_string();
            "updated its row in"
        } else {
            let last_row = lines.iter().rposition(|l| l.trim_start().starts_with('|'));
            match last_row {
                Some(i) => lines.insert(i + 1, line.to_string()),
                None => lines.push(line.to_string()),
            }
            "added a row to"
        };
        if full.exists() {
            self.backup(&rel)?;
        }
        std::fs::write(&full, lines.join("\n") + "\n")?;
        Ok(format!("{what} {rel}"))
    }

    /// Copy a vault file to `.didaskalos/backups/<unix time>_<name>`.
    /// The dot folder keeps backups out of the index and out of Obsidian.
    fn backup(&self, rel: &str) -> Result<String> {
        let secs = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        let name = rel.replace('/', "__");
        let dest_rel = format!(".didaskalos/backups/{secs}_{name}");
        let dest = self.vault.join(&dest_rel);
        std::fs::create_dir_all(dest.parent().expect("backup path has a parent"))?;
        std::fs::copy(self.vault.join(rel), &dest)?;
        Ok(dest_rel)
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

const CHAPTER_FORMAT: &str = include_str!("../templates/chapter.md");
const SYNTHESIS_FORMAT: &str = include_str!("../templates/synthesis.md");

fn lesson_format(args: &Value) -> Result<String> {
    match args.get("kind").and_then(Value::as_str).unwrap_or("") {
        "chapter" => Ok(CHAPTER_FORMAT.to_string()),
        "synthesis" | "topic" => Ok(SYNTHESIS_FORMAT.to_string()),
        k => Err(anyhow!("unknown lesson kind \"{k}\"; use chapter or synthesis")),
    }
}

/// A safe vault-relative `.md` path: no absolute paths, no `..`, no dot
/// folders, so a lesson can only land inside the vault's own notes.
fn vault_rel_path(p: &str) -> Result<String> {
    let p = p.trim().replace('\\', "/");
    let p = p.trim_start_matches("./");
    if p.starts_with('/') || !p.ends_with(".md") {
        return Err(anyhow!("path must be relative to the vault and end in .md, e.g. Orthodox/Lessons/theosis_synthesis_lesson.md"));
    }
    if p.split('/').any(|c| c.is_empty() || c == ".." || c.starts_with('.')) {
        return Err(anyhow!("path may not contain empty parts, .. or dot folders: {p}"));
    }
    Ok(p.to_string())
}

fn mcp_reindex(conn: &mut Connection, vault: &Path) -> Result<()> {
    didaskalos_core::index::update(conn, vault, false)?;
    Ok(())
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
        },
        {
            "name": "lesson_format",
            "description": "The user's lesson template and rules. kind: chapter (one chapter, video or talk, \
                two-tier Text Analysis) or synthesis (an integrated lesson on a topic or across several notes). \
                Read it before writing a lesson.",
            "inputSchema": {
                "type": "object",
                "properties": { "kind": { "type": "string", "enum": ["chapter", "synthesis"] } },
                "required": ["kind"]
            }
        },
        {
            "name": "save_lesson",
            "description": "Save a finished lesson note into the vault and add (or update) its row in \
                lessons_index.md. Refuses to replace an existing file unless overwrite is true, and backs \
                up anything it replaces. Re-indexes and reports citations that do not resolve.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "path": { "type": "string", "description": "Vault-relative path ending in .md, e.g. Orthodox/Lessons/theosis_synthesis_lesson.md" },
                    "content": { "type": "string", "description": "The whole note, starting with --- frontmatter" },
                    "index_row": {
                        "type": "object",
                        "description": "Row for lessons_index.md; the Link column is filled in from the file name",
                        "properties": {
                            "date": { "type": "string" },
                            "title": { "type": "string" },
                            "source": { "type": "string" },
                            "hook": { "type": "string", "description": "One-sentence thesis" }
                        },
                        "required": ["date", "title", "source", "hook"]
                    },
                    "overwrite": { "type": "boolean", "description": "Replace an existing file (backed up first)" }
                },
                "required": ["path", "content"]
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
    fn lesson_paths_stay_in_vault() {
        assert_eq!(vault_rel_path("./Orthodox/Lessons/x_lesson.md").unwrap(), "Orthodox/Lessons/x_lesson.md");
        assert!(vault_rel_path("/etc/passwd.md").is_err());
        assert!(vault_rel_path("../x.md").is_err());
        assert!(vault_rel_path(".obsidian/x.md").is_err());
        assert!(vault_rel_path("x.txt").is_err());
    }

    #[test]
    fn save_lesson_writes_note_and_index_row() {
        let dir = std::env::temp_dir().join(format!("dsk_test_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("lessons_index.md"),
            "# Lessons Index\n\n| Date | Title | Source | Hook | Link |\n|---|---|---|---|---|\n| 2026-01-01 | Old | S | H | [[old_lesson]] |\n\nAfter.\n").unwrap();
        let mut conn = didaskalos_core::db::open(&dir.join("index.db")).unwrap();
        mcp_reindex(&mut conn, &dir).unwrap();
        let mut s = Server::new(conn, dir.clone());
        let args = json!({
            "path": "Lessons/new_lesson.md",
            "content": "---\ntitle: New\n---\n# New\nSee [[old_lesson]].",
            "index_row": { "date": "2026-10-18", "title": "New | One", "source": "Me", "hook": "Hook" }
        });
        let out = s.save_lesson(&args).unwrap();
        assert!(out.contains("Saved [[new_lesson]]"), "{out}");
        let index = std::fs::read_to_string(dir.join("lessons_index.md")).unwrap();
        assert!(index.contains("| [[old_lesson]] |\n| 2026-10-18 | New / One | Me | Hook | [[new_lesson]] |\n\nAfter."), "{index}");
        // A second save without overwrite is refused; with it, the row is replaced, not duplicated.
        assert!(s.save_lesson(&args).is_err());
        let mut again = args.clone();
        again["overwrite"] = json!(true);
        again["index_row"]["hook"] = json!("Better hook");
        s.save_lesson(&again).unwrap();
        let index = std::fs::read_to_string(dir.join("lessons_index.md")).unwrap();
        assert_eq!(index.matches("[[new_lesson]]").count(), 1);
        assert!(index.contains("Better hook"));
        assert!(dir.join(".didaskalos/backups").read_dir().unwrap().count() >= 2);
        std::fs::remove_dir_all(&dir).unwrap();
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
