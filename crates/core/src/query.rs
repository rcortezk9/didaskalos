//! Turning a typed question into search terms: drop filler words,
//! expand abbreviations the vault spells out (PSA), and fix misspellings
//! against the words the vault actually contains (Anthanasius).

use std::collections::HashSet;

use anyhow::Result;
use rusqlite::Connection;
use serde::Serialize;

const STOPWORDS: &[&str] = &[
    "a", "about", "an", "and", "are", "as", "at", "be", "but", "by", "can", "did", "do", "does",
    "don", "explain", "find", "for", "from", "how", "i", "if", "in", "into", "is", "it", "its",
    "lay", "me", "makes", "my", "not", "of", "on", "or", "out", "so", "should", "that", "the",
    "their", "them", "there", "these", "they", "this", "to", "was", "we", "were", "what", "when",
    "where", "which", "who", "whom", "why", "will", "with", "would", "you", "your",
];

/// Abbreviations the vault writes out in full. Each expands to the words
/// added to the search; the abbreviation itself is kept too.
const ABBREVIATIONS: &[(&str, &str)] = &[
    ("psa", "penal substitutionary atonement"),
    ("hrm", "hebrew roots movement"),
    ("lxx", "septuagint"),
    ("osb", "orthodox study bible"),
    ("tulip", "total depravity unconditional election limited atonement"),
    ("ot", "old testament"),
    ("nt", "new testament"),
];

#[derive(Debug, Clone, Serialize)]
pub struct Query {
    /// Terms in the order typed, after expansion and correction.
    pub terms: Vec<String>,
    /// Rarity of each term (inverse document frequency over chunks),
    /// aligned with `terms`. Common words like "orthodox" score low.
    pub weights: Vec<f64>,
    /// Human-readable notes on what changed, e.g. "anthanasius → athanasius".
    pub notes: Vec<String>,
}

/// Lowercase word terms with filler words removed, duplicates dropped.
pub fn words(text: &str) -> Vec<String> {
    let mut seen = HashSet::new();
    text.split(|c: char| !c.is_alphanumeric())
        .map(|t| t.to_lowercase())
        .filter(|t| t.chars().count() >= 2 && !STOPWORDS.contains(&t.as_str()))
        .filter(|t| seen.insert(t.clone()))
        .collect()
}

pub fn parse(conn: &Connection, text: &str) -> Result<Query> {
    let mut terms = Vec::new();
    let mut notes = Vec::new();
    for w in words(text) {
        if let Some((_, full)) = ABBREVIATIONS.iter().find(|(abbr, _)| *abbr == w) {
            notes.push(format!("{w} → {full}"));
            terms.push(w);
            terms.extend(full.split(' ').map(str::to_string));
            continue;
        }
        match correct(conn, &w)? {
            Some(fixed) if fixed != w => {
                notes.push(format!("{w} → {fixed}"));
                terms.push(fixed);
            }
            _ => terms.push(w),
        }
    }
    let mut seen = HashSet::new();
    terms.retain(|t| seen.insert(t.clone()));
    let weights = terms.iter().map(|t| idf(conn, t)).collect::<Result<_>>()?;
    Ok(Query { terms, weights, notes })
}

/// If no indexed word starts like `word`, return the closest indexed word
/// within two edits, preferring common ones. Vocabulary terms are Porter
/// stems ("athanasiu"), so the match is made against the stem and the
/// typed word's own ending is kept where it fits.
fn correct(conn: &Connection, word: &str) -> Result<Option<String>> {
    if word.chars().count() < 5 || word.chars().any(|c| c.is_ascii_digit()) {
        return Ok(None);
    }
    // Known if some indexed stem is a prefix of the word: "differ" for
    // "difference", "inherit" for "inherited".
    let chars: Vec<char> = word.chars().collect();
    let min = chars.len().saturating_sub(4).max(3);
    let prefixes: Vec<String> = (min..=chars.len()).map(|n| chars[..n].iter().collect()).collect();
    let placeholders = vec!["?"; prefixes.len()].join(",");
    let known: bool = conn.query_row(
        &format!("SELECT EXISTS(SELECT 1 FROM chunks_vocab WHERE term IN ({placeholders}))"),
        rusqlite::params_from_iter(prefixes.iter()),
        |r| r.get(0),
    )?;
    if known {
        return Ok(None);
    }

    let first: String = word.chars().take(1).collect();
    let len = word.chars().count() as i64;
    let mut stmt = conn.prepare_cached(
        "SELECT term, doc FROM chunks_vocab
         WHERE term >= ?1 AND term < ?1 || '~' AND length(term) BETWEEN ?2 AND ?3",
    )?;
    let mut best: Option<(usize, i64, String)> = None;
    let rows = stmt.query_map(rusqlite::params![first, len - 3, len + 2], |r| {
        Ok((r.get::<_, String>(0)?, r.get::<_, i64>(1)?))
    })?;
    for row in rows {
        let (term, docs) = row?;
        // Compare against the word with and without a trailing "s", since
        // the stemmer usually drops it.
        let d = levenshtein(word, &term).min(levenshtein(word.trim_end_matches('s'), &term));
        if d <= 2 && best.as_ref().map_or(true, |(bd, bdocs, _)| (d, -docs) < (*bd, -*bdocs)) {
            best = Some((d, docs, term));
        }
    }
    // Return the stem; prefix search below matches every ending of it.
    Ok(best.map(|(_, _, t)| t))
}

/// ln(chunks / chunks containing a word starting like `term`), floored
/// at 0.1 so no term counts for nothing.
fn idf(conn: &Connection, term: &str) -> Result<f64> {
    let total: f64 = conn.query_row("SELECT COUNT(*) FROM chunks", [], |r| r.get::<_, i64>(0))? as f64;
    let stem: String = term.chars().take(5).collect();
    // `doc` is per vocabulary word, so taking the most common form under the
    // prefix approximates how many chunks hold the word at all.
    let docs: f64 = conn.query_row(
        "SELECT COALESCE(MAX(doc), 0) FROM chunks_vocab WHERE term >= ?1 AND term < ?1 || '~'",
        [&stem],
        |r| r.get::<_, i64>(0),
    )? as f64;
    Ok(((total + 1.0) / (docs + 1.0)).ln().max(0.1))
}

fn levenshtein(a: &str, b: &str) -> usize {
    let a: Vec<char> = a.chars().collect();
    let b: Vec<char> = b.chars().collect();
    let mut prev: Vec<usize> = (0..=b.len()).collect();
    for i in 1..=a.len() {
        let mut cur = vec![i; b.len() + 1];
        for j in 1..=b.len() {
            let cost = usize::from(a[i - 1] != b[j - 1]);
            cur[j] = (prev[j] + 1).min(cur[j - 1] + 1).min(prev[j - 1] + cost);
        }
        prev = cur;
    }
    prev[b.len()]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn words_drop_filler() {
        assert_eq!(words("Is Adam's guilt inherited?"), vec!["adam", "guilt", "inherited"]);
        assert_eq!(words("Explain Theosis and how to achieve it?"), vec!["theosis", "achieve"]);
    }

    #[test]
    fn edit_distance() {
        assert_eq!(levenshtein("anthanasius", "athanasius"), 1);
        assert_eq!(levenshtein("anthanasiu", "athanasiu"), 1);
    }
}
