//! Apologetics practice: the bank of objections drawn from the vault's
//! response notes, the 8-criterion rubric, and the practice log that
//! schedules each objection's next drill.
//!
//! Analogy: the response notes are your sparring manual; each objection
//! in them becomes a sparring card. A drill is one bout. The log is the
//! coach's notebook, one page per card, and the score decides when that
//! card comes back.

use std::collections::HashMap;
use std::path::Path;

use anyhow::{anyhow, Result};
use rusqlite::Connection;
use serde::Serialize;

use crate::note;
use crate::review;

/// Folder in the vault where drill attempts are written, one note per card.
pub const LOG_DIR: &str = "Practice_Log";

/// The rubric, in the order the response notes use. Each scores 0–3.
pub const CRITERIA: [&str; 8] = [
    "Fair statement",
    "Root error",
    "Scripture",
    "Fathers and councils",
    "Their own premise",
    "Next move",
    "Honesty",
    "Delivery",
];

/// Highest total when a quote, verse or canon was fabricated or wrong.
pub const ACCURACY_CAP: i64 = 12;

#[derive(Debug, Clone, Serialize)]
pub struct Card {
    /// `slug` or `slug#Heading`, the citation for the card's source.
    pub id: String,
    pub slug: String,
    pub path: String,
    /// The note the card comes from.
    pub note_title: String,
    /// The objection in a line or two, from The Claim (or the heading).
    pub objection: String,
    /// Folder the note lives in, e.g. "Catholic_Orthodox_Apologetics".
    pub topic: String,
}

#[derive(Debug, Clone, Default, Serialize)]
pub struct Progress {
    pub attempts: i64,
    pub last_score: Option<i64>,
    pub best_score: Option<i64>,
    pub last_date: Option<String>,
    pub due: Option<String>,
    pub interval: Option<i64>,
    pub streak: Option<i64>,
    /// Vault path of the card's log note.
    pub log_path: String,
}

/// Every objection card in the vault:
/// - each `Reason N` block (a note answering a list of objections),
/// - each `_response` note built on The Claim / Where It Goes Wrong headings,
/// - each section of `elevator_statements`,
/// - each `Q1: "…"` item under a Q&A or objections heading.
pub fn cards(conn: &Connection) -> Result<Vec<Card>> {
    let mut stmt = conn.prepare(
        "SELECT n.slug, n.title, n.path, c.heading, c.heading_path, c.text
         FROM chunks c JOIN notes n ON n.id = c.note_id
         WHERE n.path NOT LIKE 'Practice_Log/%'
           AND (c.heading LIKE 'Reason %' OR c.heading LIKE 'The Claim%'
                OR n.slug = 'elevator_statements' OR c.heading GLOB 'Q[0-9]*')
         ORDER BY n.path, c.ord",
    )?;
    let rows: Vec<(String, String, String, String, String, String)> = stmt
        .query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?, r.get(5)?)))?
        .collect::<Result<_, _>>()?;

    // The Claim text, keyed by (slug, the heading path it sits under).
    let mut claims: HashMap<(String, String), String> = HashMap::new();
    for (slug, _, _, heading, path, text) in &rows {
        if heading.starts_with("The Claim") {
            let parent = path.rsplit_once(" > ").map(|(p, _)| p.to_string()).unwrap_or_default();
            claims.insert((slug.clone(), parent), first_lines(text));
        }
    }

    let mut out = Vec::new();
    let mut seen = std::collections::HashSet::new();
    for (slug, title, path, heading, heading_path, text) in &rows {
        let topic = topic_of(path);
        let mut push = |id: String, objection: String| {
            if seen.insert(id.clone()) {
                out.push(Card {
                    id,
                    slug: slug.clone(),
                    path: path.clone(),
                    note_title: title.clone(),
                    objection,
                    topic: topic.clone(),
                });
            }
        };
        if heading.starts_with("The Claim") {
            // Under a `Reason N` heading the claim is one card of many;
            // otherwise the whole note is one card. (A Reason heading with
            // no text of its own has no chunk, so the claim is the anchor.)
            let parent = heading_path.rsplit(" > ").nth(1).unwrap_or("");
            if is_reason(parent) {
                push(format!("{slug}#{parent}"), first_lines(text));
            } else {
                push(slug.clone(), first_lines(text));
            }
        } else if is_reason(heading) {
            let objection = claims
                .get(&(slug.clone(), heading_path.clone()))
                .cloned()
                .unwrap_or_else(|| strip_reason(heading));
            push(format!("{slug}#{heading}"), objection);
        } else if slug == "elevator_statements" {
            if heading.is_empty() || heading == "Related" || heading_path.matches(" > ").count() != 1 {
                continue;
            }
            push(format!("{slug}#{heading}"), heading.clone());
        } else if heading.starts_with('Q') && heading.contains(':') {
            let low = heading_path.to_lowercase();
            if low.contains("q&a") || low.contains("objection") {
                let q = heading.split_once(':').map(|(_, q)| q.trim()).unwrap_or(heading);
                push(format!("{slug}#{heading}"), q.trim_matches('"').to_string());
            }
        }
    }
    Ok(out)
}

fn is_reason(heading: &str) -> bool {
    heading
        .strip_prefix("Reason ")
        .and_then(|r| r.chars().next())
        .map_or(false, |c| c.is_ascii_digit())
}

/// "Reason 1 — Praying to saints" -> "Praying to saints".
fn strip_reason(heading: &str) -> String {
    heading
        .split_once(['—', '-', ':'])
        .map(|(_, rest)| rest.trim().to_string())
        .unwrap_or_else(|| heading.to_string())
}

fn first_lines(text: &str) -> String {
    let t: String = text
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty() && !l.starts_with('#'))
        .collect::<Vec<_>>()
        .join(" ");
    if t.chars().count() > 280 {
        format!("{} …", t.chars().take(280).collect::<String>().trim_end())
    } else {
        t
    }
}

fn topic_of(path: &str) -> String {
    let parts: Vec<&str> = path.split('/').collect();
    match parts.len() {
        0 | 1 => "Vault root".into(),
        _ => parts[parts.len() - 2].to_string(),
    }
}

/// Log file for a card: `Practice_Log/<slug>__<heading>.md`.
pub fn log_path(card_id: &str) -> String {
    // Lowercase, every run of other characters becomes one underscore.
    let tidy = |s: &str| -> String {
        let mut out = String::new();
        for c in s.chars() {
            if c.is_alphanumeric() || c == '-' {
                out.extend(c.to_lowercase());
            } else if !out.ends_with('_') {
                out.push('_');
            }
        }
        out.trim_matches('_').to_string()
    };
    let mut name = match card_id.split_once('#') {
        Some((slug, heading)) => format!("{}__{}", tidy(slug), tidy(heading)),
        None => tidy(card_id),
    };
    name = name.chars().take(120).collect();
    format!("{LOG_DIR}/{name}.md")
}

/// Progress for every card that has a log, keyed by card id.
pub fn progress(vault: &Path) -> Result<HashMap<String, Progress>> {
    let mut out = HashMap::new();
    let dir = vault.join(LOG_DIR);
    let Ok(entries) = std::fs::read_dir(&dir) else { return Ok(out) };
    for e in entries.flatten() {
        let path = e.path();
        if path.extension().and_then(|x| x.to_str()) != Some("md") {
            continue;
        }
        let raw = std::fs::read_to_string(&path).unwrap_or_default();
        let rel = format!("{LOG_DIR}/{}", e.file_name().to_string_lossy());
        if let Some((card, p)) = read_log(&raw, &rel) {
            out.insert(card, p);
        }
    }
    Ok(out)
}

fn read_log(raw: &str, rel: &str) -> Option<(String, Progress)> {
    let (front, _) = note::split_frontmatter(raw);
    let fm: serde_yaml::Value = serde_yaml::from_str(front?).ok()?;
    let s = |k: &str| fm.get(k).and_then(|v| v.as_str().map(str::to_string).or_else(|| v.as_i64().map(|n| n.to_string())));
    let i = |k: &str| fm.get(k).and_then(|v| v.as_i64());
    Some((
        s("card")?,
        Progress {
            attempts: i("attempts").unwrap_or(0),
            last_score: i("last_score"),
            best_score: i("best_score"),
            last_date: s("last_date"),
            due: s("recall_due"),
            interval: i("recall_interval"),
            streak: i("recall_streak"),
            log_path: rel.to_string(),
        },
    ))
}

/// The band the design doc names for a total out of 24.
pub fn band(total: i64) -> &'static str {
    match total {
        20.. => "ready to use live",
        14..=19 => "sound, needs one fix",
        8..=13 => "re-read the source note",
        _ => "build a full lesson on this",
    }
}

/// Days until the card comes back: under 8 tomorrow, 8–13 in two days,
/// 14–19 in a week, 20+ doubles the last gap (at least two weeks).
pub fn next_interval(total: i64, prev: Option<i64>) -> i64 {
    match total {
        20.. => (prev.unwrap_or(7) * 2).max(14).min(365),
        14..=19 => 7,
        8..=13 => 2,
        _ => 1,
    }
}

/// One drill attempt, as `/apologia` reports it.
#[derive(Debug, Clone)]
pub struct Attempt {
    pub card: String,
    pub objection: String,
    pub objector: String,
    /// "typed" or "spoken".
    pub mode: String,
    pub answer: String,
    /// One score 0–3 per criterion, in `CRITERIA` order, each with a reason.
    pub scores: Vec<(i64, String)>,
    /// A fabricated or wrong quote, verse or canon: total capped at 12.
    pub accuracy_problem: Option<String>,
    pub critique: String,
    pub stronger: String,
    pub pushback: String,
    pub vault_gaps: String,
    pub delivery: String,
}

pub struct Logged {
    pub path: String,
    pub total: i64,
    pub band: &'static str,
    pub next_due: String,
    pub interval: i64,
    pub attempts: i64,
}

/// Append an attempt to the card's log note (creating it on the first
/// attempt) and reschedule the card from the score.
pub fn log_attempt(vault: &Path, a: &Attempt, today: &str) -> Result<Logged> {
    if a.scores.len() != CRITERIA.len() {
        return Err(anyhow!("give exactly {} scores, one per criterion: {}", CRITERIA.len(), CRITERIA.join(", ")));
    }
    if let Some((s, _)) = a.scores.iter().find(|(s, _)| !(0..=3).contains(s)) {
        return Err(anyhow!("scores run 0 to 3, not {s}"));
    }
    let raw_total: i64 = a.scores.iter().map(|(s, _)| s).sum();
    let total = if a.accuracy_problem.is_some() { raw_total.min(ACCURACY_CAP) } else { raw_total };

    let rel = log_path(&a.card);
    let full = vault.join(&rel);
    let existing = std::fs::read_to_string(&full).ok();
    let prev = existing.as_deref().and_then(|r| read_log(r, &rel)).map(|(_, p)| p).unwrap_or_default();

    let interval = next_interval(total, prev.interval);
    let streak = if total >= 14 { prev.streak.unwrap_or(0) + 1 } else { 0 };
    let next_due = review::add_days(today, interval)?;
    let attempts = prev.attempts + 1;
    let best = prev.best_score.map_or(total, |b| b.max(total));

    let short: String = a.objection.chars().take(70).collect();
    let front = format!(
        "---\ntitle: {}\ncard: {}\nsource: \"[[{}]]\"\ntags: [practice, apologetics]\nattempts: {attempts}\nlast_score: {total}\nbest_score: {best}\nlast_date: {today}\nrecall_due: {next_due}\nrecall_interval: {interval}\nrecall_streak: {streak}\n---\n",
        yaml_str(&format!("Practice: {short}")),
        yaml_str(&a.card),
        a.card,
    );
    let body = match &existing {
        Some(r) => note::split_frontmatter(r).1.trim_end().to_string(),
        None => format!(
            "# Practice: {}\n\n> Card: [[{}]]. Each attempt below is one drill from `/apologia`, newest last.\n\n## Attempts",
            a.objection.trim(),
            a.card
        ),
    };

    let mut entry = format!("\n\n### {today} — {total}/24 ({})\n\n", band(total));
    entry.push_str(&format!("**Objection ({}):** {}\n\n", a.objector.trim(), a.objection.trim()));
    entry.push_str(&format!("**Your answer ({}):** {}\n\n", a.mode.trim(), a.answer.trim()));
    entry.push_str("| Criterion | Score | Why |\n|---|---|---|\n");
    for (name, (score, why)) in CRITERIA.iter().zip(&a.scores) {
        entry.push_str(&format!("| {name} | {score} | {} |\n", why.replace('|', "/").replace('\n', " ")));
    }
    entry.push_str(&format!("| **Total** | **{total}** | {} |\n", band(total)));
    if let Some(p) = &a.accuracy_problem {
        entry.push_str(&format!("\n**Accuracy gate:** {} (raw total {raw_total}, capped at {ACCURACY_CAP}).\n", p.trim()));
    }
    for (label, text) in [
        ("Delivery", &a.delivery),
        ("Critique", &a.critique),
        ("Stronger response", &a.stronger),
        ("Pushback", &a.pushback),
        ("Vault gaps", &a.vault_gaps),
    ] {
        if !text.trim().is_empty() {
            entry.push_str(&format!("\n**{label}:**\n\n{}\n", text.trim()));
        }
    }

    std::fs::create_dir_all(full.parent().expect("log path has a folder"))?;
    std::fs::write(&full, format!("{front}{body}{entry}"))?;
    Ok(Logged { path: rel, total, band: band(total), next_due, interval, attempts })
}

/// A YAML double-quoted string.
fn yaml_str(s: &str) -> String {
    format!("\"{}\"", s.replace('\\', "\\\\").replace('"', "\\\""))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn schedule_by_score() {
        assert_eq!(next_interval(5, None), 1);
        assert_eq!(next_interval(13, Some(30)), 2);
        assert_eq!(next_interval(17, None), 7);
        assert_eq!(next_interval(21, None), 14);
        assert_eq!(next_interval(22, Some(14)), 28);
    }

    #[test]
    fn log_paths_are_tidy() {
        assert_eq!(
            log_path("ten_reasons_not_to_convert_response#Reason 1 — Praying to saints"),
            "Practice_Log/ten_reasons_not_to_convert_response__reason_1_praying_to_saints.md"
        );
    }

    #[test]
    fn attempts_append_and_reschedule() {
        let dir = std::env::temp_dir().join(format!("dsk_practice_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let mut a = Attempt {
            card: "canon_no_agreement_response".into(),
            objection: "Orthodoxy has no agreed canon.".into(),
            objector: "Catholic apologist".into(),
            mode: "typed".into(),
            answer: "We do, liturgically.".into(),
            scores: vec![(3, "fair".into()); 8],
            accuracy_problem: Some("misquoted Trullo canon 2".into()),
            critique: "Name the canon.".into(),
            stronger: "Better.".into(),
            pushback: String::new(),
            vault_gaps: String::new(),
            delivery: String::new(),
        };
        let first = log_attempt(&dir, &a, "2026-10-19").unwrap();
        assert_eq!((first.total, first.interval, first.next_due.as_str()), (12, 2, "2026-10-21"));
        a.accuracy_problem = None;
        let second = log_attempt(&dir, &a, "2026-10-21").unwrap();
        assert_eq!((second.total, second.interval, second.attempts), (24, 14, 2));
        let raw = std::fs::read_to_string(dir.join(&second.path)).unwrap();
        assert_eq!(raw.matches("### 2026-").count(), 2);
        assert!(raw.contains("best_score: 24") && raw.contains("recall_due: 2026-11-04"));
        let p = progress(&dir).unwrap();
        assert_eq!(p["canon_no_agreement_response"].attempts, 2);
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
