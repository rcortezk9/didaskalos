//! Parsing one vault file into the pieces the index stores:
//! frontmatter fields, heading-sized chunks, and outbound links.
//!
//! Nothing here touches the database, so it is easy to unit test.

use std::path::Path;
use std::sync::OnceLock;

use regex::Regex;
use serde_yaml::Value;

/// Sections longer than this (in characters, roughly 750 tokens) are split
/// again at paragraph breaks so one chunk never swamps the context window.
const MAX_CHUNK_CHARS: usize = 3000;

/// Which layer of the vault a note belongs to. Retrieval weights the
/// distilled layers (Wiki, lessons) above raw transcripts.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    WikiConcept,
    WikiSource,
    Lesson,
    ConceptMap,
    Book,
    Analysis,
    Daily,
    Raw,
}

impl Kind {
    pub fn as_str(self) -> &'static str {
        match self {
            Kind::WikiConcept => "wiki-concept",
            Kind::WikiSource => "wiki-source",
            Kind::Lesson => "lesson",
            Kind::ConceptMap => "concept-map",
            Kind::Book => "book",
            Kind::Analysis => "analysis",
            Kind::Daily => "daily",
            Kind::Raw => "raw",
        }
    }

    pub fn from_str(s: &str) -> Kind {
        match s {
            "wiki-concept" => Kind::WikiConcept,
            "wiki-source" => Kind::WikiSource,
            "lesson" => Kind::Lesson,
            "concept-map" => Kind::ConceptMap,
            "book" => Kind::Book,
            "daily" => Kind::Daily,
            "raw" => Kind::Raw,
            _ => Kind::Analysis,
        }
    }

    /// Multiplier applied to a chunk's search score.
    pub fn boost(self) -> f64 {
        match self {
            Kind::WikiConcept => 1.5,
            Kind::Lesson => 1.3,
            Kind::WikiSource | Kind::ConceptMap => 1.2,
            Kind::Book | Kind::Analysis => 1.0,
            Kind::Daily => 0.9,
            Kind::Raw => 0.7,
        }
    }
}

/// One heading section (or a slice of a long one).
#[derive(Debug, Clone, PartialEq)]
pub struct Chunk {
    /// The nearest heading, or "" for text before the first heading.
    pub heading: String,
    /// "Title > Section > Subsection", prepended to the text for search
    /// and embeddings so a bare bullet list keeps its context.
    pub heading_path: String,
    pub text: String,
}

/// A `[[target#heading|alias]]` wikilink or a concept-map `click` edge.
#[derive(Debug, Clone, PartialEq)]
pub struct Link {
    pub target: String,
    pub heading: Option<String>,
    pub kind: &'static str,
}

#[derive(Debug, Clone, Default)]
pub struct Recall {
    pub due: Option<String>,
    pub interval: Option<i64>,
    pub streak: Option<i64>,
}

#[derive(Debug, Clone)]
pub struct Note {
    /// File stem. Wikilinks in this vault are by bare slug, so it is unique.
    pub slug: String,
    /// Path relative to the vault root, with `/` separators.
    pub rel_path: String,
    pub title: String,
    pub kind: Kind,
    pub tags: Vec<String>,
    pub recall: Recall,
    pub chunks: Vec<Chunk>,
    pub links: Vec<Link>,
}

/// Parse a file's text. `rel_path` decides the note's kind and slug.
pub fn parse(rel_path: &str, raw: &str) -> Note {
    let slug = Path::new(rel_path)
        .file_stem()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_default();
    let is_txt = rel_path.ends_with(".txt");

    let (front, body) = if is_txt { (None, raw) } else { split_frontmatter(raw) };
    let front = front.and_then(|f| serde_yaml::from_str::<Value>(f).ok());

    let fm_str = |key: &str| -> Option<String> {
        front.as_ref()?.get(key).and_then(|v| match v {
            Value::String(s) => Some(s.clone()),
            Value::Number(n) => Some(n.to_string()),
            _ => None,
        })
    };
    let fm_int = |key: &str| front.as_ref()?.get(key)?.as_i64();

    let title = fm_str("title")
        .or_else(|| first_h1(body))
        .unwrap_or_else(|| slug.replace('_', " "));
    let tags = front.as_ref().map(read_tags).unwrap_or_default();
    let fm_type = fm_str("type").unwrap_or_default();

    let recall = Recall {
        due: fm_str("recall_due"),
        interval: fm_int("recall_interval"),
        streak: fm_int("recall_streak"),
    };

    let chunks = if is_txt {
        split_long(&title, "", &title, body)
    } else {
        chunk_markdown(&title, body)
    };

    Note {
        kind: classify(rel_path, &slug, &fm_type),
        links: extract_links(body),
        slug,
        rel_path: rel_path.to_string(),
        title,
        tags,
        recall,
        chunks,
    }
}

/// Split `---\n...\n---` frontmatter off the top of a Markdown file.
pub fn split_frontmatter(raw: &str) -> (Option<&str>, &str) {
    let rest = match raw.strip_prefix("---\n").or_else(|| raw.strip_prefix("---\r\n")) {
        Some(r) => r,
        None => return (None, raw),
    };
    // The closing fence is a line that is exactly `---`.
    let mut offset = 0;
    for line in rest.split_inclusive('\n') {
        if line.trim_end() == "---" {
            return (Some(&rest[..offset]), &rest[offset + line.len()..]);
        }
        offset += line.len();
    }
    (None, raw)
}

fn read_tags(front: &Value) -> Vec<String> {
    match front.get("tags") {
        Some(Value::Sequence(items)) => items
            .iter()
            .filter_map(|v| v.as_str().map(str::to_string))
            .collect(),
        Some(Value::String(s)) => s
            .split(|c: char| c == ',' || c.is_whitespace())
            .filter(|t| !t.is_empty())
            .map(str::to_string)
            .collect(),
        _ => Vec::new(),
    }
}

fn first_h1(body: &str) -> Option<String> {
    body.lines()
        .find_map(|l| l.strip_prefix("# "))
        .map(|t| t.trim().to_string())
}

/// Decide the vault layer from the folder, the file name and `type:`.
fn classify(rel_path: &str, slug: &str, fm_type: &str) -> Kind {
    let p = rel_path.to_ascii_lowercase();
    // Practice_Log holds your own drill answers, right or wrong, so it
    // ranks with the raw material rather than with the teaching notes.
    if p.starts_with("raw/") || p.contains("/raw/") || p.ends_with(".txt") || p.starts_with("practice_log/") {
        Kind::Raw
    } else if p.starts_with("wiki/") {
        match fm_type {
            "source" | "playlist" => Kind::WikiSource,
            _ if slug.starts_with("source_") => Kind::WikiSource,
            _ => Kind::WikiConcept,
        }
    } else if slug.ends_with("_lesson") || slug.ends_with("_wisdom") || slug == "lessons_index" {
        Kind::Lesson
    } else if p.starts_with("concept_maps/") {
        Kind::ConceptMap
    } else if p.contains("daily_readings/") || p.contains("/psalms/") {
        Kind::Daily
    } else if p.contains("book_readings/") {
        Kind::Book
    } else {
        Kind::Analysis
    }
}

/// Split Markdown at `#`, `##` and `###` headings, ignoring `#` lines
/// inside fenced code blocks (Mermaid diagrams, shell snippets).
pub fn chunk_markdown(title: &str, body: &str) -> Vec<Chunk> {
    // stack[i] holds the current heading at level i+1.
    let mut stack: Vec<String> = Vec::new();
    let mut heading = String::new();
    let mut buf = String::new();
    let mut in_fence = false;
    let mut out = Vec::new();

    let flush = |heading: &str, stack: &[String], buf: &str, out: &mut Vec<Chunk>| {
        let mut path = vec![title.to_string()];
        // Skip an H1 that just repeats the note title.
        path.extend(stack.iter().filter(|h| !h.is_empty() && *h != title).cloned());
        out.extend(split_long(title, heading, &path.join(" > "), buf));
    };

    for line in body.lines() {
        let trimmed = line.trim_start();
        if trimmed.starts_with("```") || trimmed.starts_with("~~~") {
            in_fence = !in_fence;
        }
        let level = if in_fence { 0 } else { heading_level(line) };
        if (1..=3).contains(&level) {
            flush(&heading, &stack, &buf, &mut out);
            buf.clear();
            heading = line[level..].trim().trim_matches('#').trim().to_string();
            stack.truncate(level - 1);
            while stack.len() < level - 1 {
                stack.push(String::new());
            }
            stack.push(heading.clone());
        } else {
            buf.push_str(line);
            buf.push('\n');
        }
    }
    flush(&heading, &stack, &buf, &mut out);
    out
}

/// `## Foo` -> 2. Returns 0 for anything that is not an ATX heading.
fn heading_level(line: &str) -> usize {
    let hashes = line.bytes().take_while(|&b| b == b'#').count();
    if (1..=6).contains(&hashes) && line[hashes..].starts_with(' ') {
        hashes
    } else {
        0
    }
}

/// Turn one section into one or more chunks, splitting at blank lines
/// when it is longer than `MAX_CHUNK_CHARS`. Empty sections are dropped.
fn split_long(_title: &str, heading: &str, heading_path: &str, text: &str) -> Vec<Chunk> {
    let text = text.trim();
    if text.chars().filter(|c| c.is_alphanumeric()).count() < 20 {
        return Vec::new();
    }
    let make = |t: &str| Chunk {
        heading: heading.to_string(),
        heading_path: heading_path.to_string(),
        text: t.trim().to_string(),
    };
    if text.len() <= MAX_CHUNK_CHARS {
        return vec![make(text)];
    }

    let mut out = Vec::new();
    let mut cur = String::new();
    for para in text.split("\n\n") {
        if !cur.is_empty() && cur.len() + para.len() > MAX_CHUNK_CHARS {
            out.push(make(&cur));
            cur.clear();
        }
        // A single huge paragraph (a transcript with no breaks) is cut at
        // sentence ends instead.
        if para.len() > MAX_CHUNK_CHARS {
            for piece in split_sentences(para, MAX_CHUNK_CHARS) {
                out.push(make(&piece));
            }
            continue;
        }
        cur.push_str(para);
        cur.push_str("\n\n");
    }
    if !cur.trim().is_empty() {
        out.push(make(&cur));
    }
    out
}

fn split_sentences(text: &str, max: usize) -> Vec<String> {
    let mut out = Vec::new();
    let mut cur = String::new();
    for sentence in text.split_inclusive(". ") {
        if cur.len() + sentence.len() > max && !cur.is_empty() {
            out.push(std::mem::take(&mut cur));
        }
        cur.push_str(sentence);
    }
    if !cur.is_empty() {
        out.push(cur);
    }
    out
}

fn wikilink_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    // [[target]], [[target#Heading]], [[folder/target|alias]]
    RE.get_or_init(|| Regex::new(r"\[\[([^\]\|#]+)(?:#([^\]\|]*))?(?:\|[^\]]*)?\]\]").unwrap())
}

fn click_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    // Mermaid concept-map edges: `click A "slug"`
    RE.get_or_init(|| Regex::new(r#"(?m)^\s*click\s+\w+\s+"([^"]+)""#).unwrap())
}

pub fn extract_links(body: &str) -> Vec<Link> {
    let mut links = Vec::new();
    for cap in wikilink_re().captures_iter(body) {
        // Folder-qualified links (`Wiki/index`) resolve by their last part.
        let target = cap[1].trim().rsplit('/').next().unwrap_or("").trim();
        if target.is_empty() {
            continue;
        }
        links.push(Link {
            target: target.to_string(),
            heading: cap.get(2).map(|h| h.as_str().trim().to_string()).filter(|h| !h.is_empty()),
            kind: "wikilink",
        });
    }
    for cap in click_re().captures_iter(body) {
        links.push(Link { target: cap[1].trim().to_string(), heading: None, kind: "concept-map" });
    }
    links
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = "---\ntitle: Ancestral Sin\ntype: concept\ntags: [theology, orthodox]\nrecall_due: 2026-10-01\nrecall_interval: 2\n---\n\n## Summary\n\nWhat is inherited is mortality and corruption, not guilt. See [[concept_theosis]].\n\n## The Sequence\n\n```mermaid\n# not a heading\nflowchart TD\n```\n\n### Detail\n\nMore text about the Fall here, linking [[Wiki/index|Index]] and [[concept_prelest#Forms]].\n";

    #[test]
    fn parses_frontmatter() {
        let n = parse("Wiki/concept_ancestral_sin.md", SAMPLE);
        assert_eq!(n.slug, "concept_ancestral_sin");
        assert_eq!(n.title, "Ancestral Sin");
        assert_eq!(n.kind, Kind::WikiConcept);
        assert_eq!(n.tags, vec!["theology", "orthodox"]);
        assert_eq!(n.recall.due.as_deref(), Some("2026-10-01"));
        assert_eq!(n.recall.interval, Some(2));
    }

    #[test]
    fn chunks_at_headings_and_ignores_fenced_hashes() {
        let n = parse("Wiki/concept_ancestral_sin.md", SAMPLE);
        let headings: Vec<_> = n.chunks.iter().map(|c| c.heading.as_str()).collect();
        assert_eq!(headings, vec!["Summary", "The Sequence", "Detail"]);
        assert_eq!(n.chunks[2].heading_path, "Ancestral Sin > The Sequence > Detail");
    }

    #[test]
    fn extracts_links() {
        let n = parse("Wiki/concept_ancestral_sin.md", SAMPLE);
        let targets: Vec<_> = n.links.iter().map(|l| l.target.as_str()).collect();
        assert_eq!(targets, vec!["concept_theosis", "index", "concept_prelest"]);
        assert_eq!(n.links[2].heading.as_deref(), Some("Forms"));
    }

    #[test]
    fn concept_map_clicks_are_links() {
        let links = extract_links("flowchart TD\n    click A \"theosis_purpose_human_life\"\n");
        assert_eq!(links[0].target, "theosis_purpose_human_life");
        assert_eq!(links[0].kind, "concept-map");
    }

    #[test]
    fn classifies_layers() {
        assert_eq!(classify("Raw/when_you_are_angry.txt", "when_you_are_angry", ""), Kind::Raw);
        assert_eq!(classify("Wiki/source_uoc_06.md", "source_uoc_06", "source"), Kind::WikiSource);
        assert_eq!(
            classify("Orthodox/Book_Readings/x/thinking_orthodox_ch01_lesson.md", "thinking_orthodox_ch01_lesson", ""),
            Kind::Lesson
        );
        assert_eq!(classify("Orthodox/Book_Readings/x/ch01.md", "ch01", ""), Kind::Book);
    }

    #[test]
    fn long_sections_split() {
        let para = "word ".repeat(400); // ~2000 chars
        let body = format!("## Big\n\n{para}\n\n{para}\n\n{para}\n");
        let chunks = chunk_markdown("T", &body);
        assert!(chunks.len() >= 2);
        assert!(chunks.iter().all(|c| c.text.len() <= MAX_CHUNK_CHARS + 10));
    }
}
