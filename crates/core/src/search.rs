//! Retrieval: keyword search, layer boost, then a boost for notes your
//! own links tie to the top results.
//!
//! Analogy: FTS5 is the card catalogue (exact words), the layer boost is
//! shelving the encyclopedia above the raw transcripts, and the link boost
//! is following the "see also" notes you wrote in the margins.

use std::collections::{HashMap, HashSet};

use anyhow::Result;
use rusqlite::{params_from_iter, Connection};
use serde::Serialize;

use crate::note::Kind;

/// How many chunks the keyword pass pulls before ranking notes.
const CANDIDATE_CHUNKS: usize = 300;
/// How many top notes seed the link boost.
const LINK_SEEDS: usize = 5;
/// Share of a seed note's score a linked note gains.
const LINK_BOOST: f64 = 0.2;

const STOPWORDS: &[&str] = &[
    "a", "an", "and", "are", "as", "at", "be", "but", "by", "can", "did", "do", "does", "for", "from",
    "how", "i", "if", "in", "into", "is", "it", "its", "me", "my", "not", "of", "on", "or", "so",
    "that", "the", "their", "them", "there", "these", "they", "this", "to", "was", "we", "were",
    "what", "when", "where", "which", "who", "whom", "why", "will", "with", "would", "you", "your",
];

#[derive(Debug, Clone, Serialize)]
pub struct HitChunk {
    pub heading: String,
    pub heading_path: String,
    pub snippet: String,
    pub score: f64,
}

#[derive(Debug, Clone, Serialize)]
pub struct Hit {
    pub slug: String,
    pub title: String,
    pub kind: String,
    pub path: String,
    pub score: f64,
    /// Title of the top result that links to (or from) this note, when
    /// the link boost contributed.
    pub linked_with: Option<String>,
    pub chunks: Vec<HitChunk>,
}

impl Hit {
    /// `[[slug#Heading]]`, the citation form Obsidian opens.
    pub fn cite(&self, chunk: &HitChunk) -> String {
        if chunk.heading.is_empty() {
            format!("[[{}]]", self.slug)
        } else {
            format!("[[{}#{}]]", self.slug, chunk.heading)
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct Related {
    pub slug: String,
    pub title: String,
    /// How many of the top results link to or from it.
    pub links: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct Results {
    pub query: String,
    pub hits: Vec<Hit>,
    /// Notes your links tie to the top results that the words alone
    /// did not find. Useful reading for a full lesson.
    pub related: Vec<Related>,
}

/// Lowercase word terms with stopwords removed.
pub fn terms(query: &str) -> Vec<String> {
    let mut seen = HashSet::new();
    query
        .split(|c: char| !c.is_alphanumeric())
        .map(|t| t.to_lowercase())
        .filter(|t| t.chars().count() >= 2 && !STOPWORDS.contains(&t.as_str()))
        .filter(|t| seen.insert(t.clone()))
        .collect()
}

/// `"theosis" OR "energies"`. Quoting each term keeps FTS5 syntax
/// characters in the user's text from being parsed as operators.
fn fts_query(terms: &[String]) -> String {
    terms.iter().map(|t| format!("\"{t}\"")).collect::<Vec<_>>().join(" OR ")
}

/// Fraction of query terms that appear in the chunk. A rough stem match
/// (first 5 letters) so "inherited" counts "inheritance".
fn coverage(terms: &[String], text: &str) -> f64 {
    if terms.is_empty() {
        return 1.0;
    }
    let lower = text.to_lowercase();
    let hits = terms
        .iter()
        .filter(|t| {
            let stem: String = t.chars().take(5).collect();
            lower.contains(&stem)
        })
        .count();
    hits as f64 / terms.len() as f64
}

struct Candidate {
    note_id: i64,
    slug: String,
    title: String,
    kind: String,
    path: String,
    chunks: Vec<HitChunk>,
}

pub fn search(conn: &Connection, query: &str, limit: usize) -> Result<Results> {
    let terms = terms(query);
    let empty = Results { query: query.to_string(), hits: vec![], related: vec![] };
    if terms.is_empty() {
        return Ok(empty);
    }

    // 1. Keyword pass over chunks. bm25 weights: title 4, heading 2, body 1.
    let mut stmt = conn.prepare_cached(
        "SELECT n.id, n.slug, n.title, n.kind, n.path, c.heading, c.heading_path, c.text,
                bm25(chunks_fts, 4.0, 2.0, 1.0) AS rank,
                snippet(chunks_fts, 2, '«', '»', ' … ', 28)
         FROM chunks_fts
         JOIN chunks c ON c.id = chunks_fts.rowid
         JOIN notes n ON n.id = c.note_id
         WHERE chunks_fts MATCH ?1
         ORDER BY rank
         LIMIT ?2",
    )?;
    let mut by_note: HashMap<i64, Candidate> = HashMap::new();
    let rows = stmt.query_map(rusqlite::params![fts_query(&terms), CANDIDATE_CHUNKS as i64], |r| {
        Ok((
            r.get::<_, i64>(0)?,
            r.get::<_, String>(1)?,
            r.get::<_, String>(2)?,
            r.get::<_, String>(3)?,
            r.get::<_, String>(4)?,
            r.get::<_, String>(5)?,
            r.get::<_, String>(6)?,
            r.get::<_, String>(7)?,
            r.get::<_, f64>(8)?,
            r.get::<_, String>(9)?,
        ))
    })?;
    for row in rows {
        let (note_id, slug, title, kind, path, heading, heading_path, text, rank, snippet) = row?;
        // bm25 is negative-is-better; flip it, then weight by vault layer
        // and by how many of the question's words the chunk contains.
        let cov = coverage(&terms, &format!("{title} {heading_path} {text}"));
        let score = -rank
            * Kind::from_str(&kind).boost()
            * section_weight(&heading)
            * (0.25 + 0.75 * cov * cov);
        let cand = by_note.entry(note_id).or_insert_with(|| Candidate {
            note_id,
            slug,
            title,
            kind,
            path,
            chunks: Vec::new(),
        });
        cand.chunks.push(HitChunk { heading, heading_path, snippet: tidy(&snippet), score });
    }

    // 2. Note score: best chunk, plus a little for a second and third
    //    matching section (a note that covers the topic in depth).
    let mut hits: Vec<(Candidate, f64)> = by_note
        .into_values()
        .map(|mut c| {
            c.chunks.sort_by(|a, b| b.score.total_cmp(&a.score));
            let s: Vec<f64> = c.chunks.iter().map(|h| h.score).collect();
            let score = s.first().copied().unwrap_or(0.0)
                + 0.25 * s.get(1).copied().unwrap_or(0.0)
                + 0.1 * s.get(2).copied().unwrap_or(0.0);
            (c, score)
        })
        .collect();
    hits.sort_by(|a, b| b.1.total_cmp(&a.1));

    // 3. Link boost: notes linked to or from the top few gain a share of
    //    the linking note's score.
    let seeds: Vec<(i64, String, String, f64)> = hits
        .iter()
        .take(LINK_SEEDS)
        .map(|(c, s)| (c.note_id, c.slug.clone(), c.title.clone(), *s))
        .collect();
    let neighbours = neighbours_of(conn, &seeds)?;

    let mut linked_with: HashMap<i64, String> = HashMap::new();
    for (cand, score) in hits.iter_mut() {
        if let Some(seed_hits) = neighbours.by_id.get(&cand.note_id) {
            let best = seed_hits
                .iter()
                .filter(|(seed_id, _, _)| *seed_id != cand.note_id)
                .max_by(|a, b| a.2.total_cmp(&b.2));
            if let Some((_, seed_title, seed_score)) = best {
                *score += LINK_BOOST * seed_score;
                linked_with.insert(cand.note_id, seed_title.clone());
            }
        }
    }
    hits.sort_by(|a, b| b.1.total_cmp(&a.1));

    let matched: HashSet<i64> = hits.iter().map(|(c, _)| c.note_id).collect();
    let hits: Vec<Hit> = hits
        .into_iter()
        .take(limit)
        .map(|(c, score)| Hit {
            linked_with: linked_with.remove(&c.note_id),
            slug: c.slug,
            title: c.title,
            kind: c.kind,
            path: c.path,
            score: (score * 100.0).round() / 100.0,
            chunks: c.chunks.into_iter().take(2).collect(),
        })
        .collect();

    let mut related: Vec<Related> = neighbours
        .by_id
        .iter()
        .filter(|(id, _)| !matched.contains(id))
        .map(|(id, seeds)| {
            let (slug, title) = neighbours.names[id].clone();
            Related { slug, title, links: seeds.len() }
        })
        .collect();
    related.sort_by(|a, b| b.links.cmp(&a.links).then(a.title.cmp(&b.title)));
    related.truncate(8);

    Ok(Results { query: query.to_string(), hits, related })
}

struct Neighbours {
    /// note id -> the seeds it is linked with: (seed id, seed title, seed score)
    by_id: HashMap<i64, Vec<(i64, String, f64)>>,
    names: HashMap<i64, (String, String)>,
}

/// Every note linked to or from one of the seed notes, in either direction.
fn neighbours_of(conn: &Connection, seeds: &[(i64, String, String, f64)]) -> Result<Neighbours> {
    let mut out = Neighbours { by_id: HashMap::new(), names: HashMap::new() };
    for (seed_id, seed_slug, seed_title, seed_score) in seeds {
        let mut stmt = conn.prepare_cached(
            "SELECT DISTINCT n.id, n.slug, n.title FROM links l JOIN notes n ON n.slug = l.target
               WHERE l.src_id = ?1
             UNION
             SELECT DISTINCT n.id, n.slug, n.title FROM links l JOIN notes n ON n.id = l.src_id
               WHERE l.target = ?2",
        )?;
        let rows = stmt.query_map(params_from_iter([seed_id.to_string(), seed_slug.clone()]), |r| {
            Ok((r.get::<_, i64>(0)?, r.get::<_, String>(1)?, r.get::<_, String>(2)?))
        })?;
        for row in rows {
            let (id, slug, title) = row?;
            // Hub pages (the Wiki index, the lessons index) link to
            // everything, so they carry no signal.
            if id == *seed_id || slug == "index" || slug.ends_with("/index") || slug == "lessons_index" || slug == "log" {
                continue;
            }
            out.by_id.entry(id).or_default().push((*seed_id, seed_title.clone(), *seed_score));
            out.names.insert(id, (slug, title));
        }
    }
    Ok(out)
}

/// Link lists ("Cross-References", "Related") repeat other notes' slugs,
/// so they match on words without saying anything about them.
fn section_weight(heading: &str) -> f64 {
    let h = heading.to_lowercase();
    let link_list = ["cross-references", "cross references", "related", "source", "sources", "source analyses", "see also"];
    if link_list.iter().any(|l| h == *l) { 0.35 } else { 1.0 }
}

/// Collapse whitespace so a snippet prints on one line.
fn tidy(s: &str) -> String {
    s.split_whitespace().collect::<Vec<_>>().join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn terms_drop_stopwords_and_punctuation() {
        assert_eq!(terms("Is Adam's guilt inherited?"), vec!["adam", "guilt", "inherited"]);
    }

    #[test]
    fn fts_query_quotes_terms() {
        assert_eq!(fts_query(&["sola".into(), "scriptura".into()]), "\"sola\" OR \"scriptura\"");
    }

    #[test]
    fn coverage_counts_stems() {
        let t = terms("inherited guilt");
        assert_eq!(coverage(&t, "the inheritance of guilt"), 1.0);
        assert_eq!(coverage(&t, "guilt alone"), 0.5);
    }
}
