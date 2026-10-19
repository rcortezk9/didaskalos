//! Spaced repetition over whole notes, using the `recall_due`,
//! `recall_interval` and `recall_streak` frontmatter the vault already has.
//!
//! Analogy: each note is a plant on a watering rota. Answer well and the
//! next watering moves further out (1, 3, 7, 16 days ...); struggle and it
//! comes back tomorrow. The rota lives in the note itself, so Obsidian,
//! git and the vault backup all see it.

use anyhow::{anyhow, Result};
use rusqlite::Connection;
use serde::Serialize;

/// Longest gap between reviews, in days.
const MAX_INTERVAL: i64 = 365;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Grade {
    /// Could not recall it.
    Again,
    /// Recalled with real effort or gaps.
    Hard,
    /// Recalled the main points.
    Good,
    /// Recalled it all at once.
    Easy,
}

impl Grade {
    pub fn parse(s: &str) -> Result<Self> {
        match s.trim().to_ascii_lowercase().as_str() {
            "again" | "1" => Ok(Grade::Again),
            "hard" | "2" => Ok(Grade::Hard),
            "good" | "3" => Ok(Grade::Good),
            "easy" | "4" => Ok(Grade::Easy),
            other => Err(anyhow!("grade must be again, hard, good or easy, not \"{other}\"")),
        }
    }
}

/// The next (interval, streak) after a review. `interval` is the gap that
/// just ended; a note never reviewed has none.
pub fn schedule(interval: Option<i64>, streak: Option<i64>, grade: Grade) -> (i64, i64) {
    let prev = interval.unwrap_or(0).max(0);
    let streak = streak.unwrap_or(0).max(0);
    let grow = |p: i64| match p {
        0 => 1,
        1 | 2 => 3,
        p => (p as f64 * 2.2).round() as i64,
    };
    let (next, streak) = match grade {
        Grade::Again => (1, 0),
        Grade::Hard => (((prev as f64) * 1.2).round().max(1.0) as i64, streak),
        Grade::Good => (grow(prev), streak + 1),
        Grade::Easy => ((grow(prev) as f64 * 1.5).round().max(4.0) as i64, streak + 1),
    };
    (next.min(MAX_INTERVAL), streak)
}

/// Today as YYYY-MM-DD in local time. `$DSK_TODAY` overrides it (tests,
/// or catching up on a missed day).
pub fn today() -> String {
    if let Ok(d) = std::env::var("DSK_TODAY") {
        if parse_date(&d).is_some() {
            return d;
        }
    }
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0);
    format_date((secs + local_offset_secs()) / 86_400)
}

/// The local UTC offset, read from `date +%z` so no timezone database is
/// needed. Falls back to UTC.
fn local_offset_secs() -> i64 {
    let out = std::process::Command::new("date").arg("+%z").output();
    let s = match out {
        Ok(o) => String::from_utf8_lossy(&o.stdout).trim().to_string(),
        Err(_) => return 0,
    };
    // "+0200" / "-0500"
    if s.len() != 5 {
        return 0;
    }
    let sign = if s.starts_with('-') { -1 } else { 1 };
    let h: i64 = s[1..3].parse().unwrap_or(0);
    let m: i64 = s[3..5].parse().unwrap_or(0);
    sign * (h * 3600 + m * 60)
}

pub fn add_days(date: &str, days: i64) -> Result<String> {
    let d = parse_date(date).ok_or_else(|| anyhow!("not a YYYY-MM-DD date: {date}"))?;
    Ok(format_date(d + days))
}

/// Days between two dates (b - a).
pub fn days_between(a: &str, b: &str) -> Option<i64> {
    Some(parse_date(b)? - parse_date(a)?)
}

/// Days since 1970-01-01 (Howard Hinnant's civil-date algorithm).
fn parse_date(s: &str) -> Option<i64> {
    let s = s.trim().trim_matches('"').trim_matches('\'');
    let mut it = s.get(..10)?.split('-');
    let y: i64 = it.next()?.parse().ok()?;
    let m: i64 = it.next()?.parse().ok()?;
    let d: i64 = it.next()?.parse().ok()?;
    if !(1..=12).contains(&m) || !(1..=31).contains(&d) {
        return None;
    }
    let y = if m <= 2 { y - 1 } else { y };
    let era = y.div_euclid(400);
    let yoe = y - era * 400;
    let mp = (m + 9) % 12;
    let doy = (153 * mp + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    Some(era * 146_097 + doe - 719_468)
}

fn format_date(days: i64) -> String {
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = yoe + era * 400 + i64::from(m <= 2);
    format!("{y:04}-{m:02}-{d:02}")
}

/// Write the three recall fields into a note's frontmatter, replacing
/// them if present and adding them before the closing `---` if not.
/// Everything else in the file is left byte for byte.
pub fn set_recall(raw: &str, due: &str, interval: i64, streak: i64) -> Result<String> {
    let fields = [
        ("recall_due", due.to_string()),
        ("recall_interval", interval.to_string()),
        ("recall_streak", streak.to_string()),
    ];
    let lines: Vec<&str> = raw.split_inclusive('\n').collect();
    if lines.first().map(|l| l.trim_end()) != Some("---") {
        // No frontmatter: start one.
        let mut out = String::from("---\n");
        for (k, v) in &fields {
            out.push_str(&format!("{k}: {v}\n"));
        }
        out.push_str("---\n");
        out.push_str(raw);
        return Ok(out);
    }
    let close = lines
        .iter()
        .skip(1)
        .position(|l| l.trim_end() == "---")
        .map(|i| i + 1)
        .ok_or_else(|| anyhow!("frontmatter has no closing ---"))?;

    let mut out = String::new();
    let mut written = [false; 3];
    out.push_str(lines[0]);
    for line in &lines[1..close] {
        let key = line.split(':').next().unwrap_or("").trim();
        match fields.iter().position(|(k, _)| *k == key) {
            Some(i) => {
                out.push_str(&format!("{}: {}\n", fields[i].0, fields[i].1));
                written[i] = true;
            }
            None => out.push_str(line),
        }
    }
    for (i, (k, v)) in fields.iter().enumerate() {
        if !written[i] {
            out.push_str(&format!("{k}: {v}\n"));
        }
    }
    for line in &lines[close..] {
        out.push_str(line);
    }
    Ok(out)
}

#[derive(Debug, Clone, Serialize)]
pub struct DueNote {
    pub slug: String,
    pub title: String,
    pub kind: String,
    pub path: String,
    /// None for a note never reviewed.
    pub due: Option<String>,
    pub interval: Option<i64>,
    pub streak: Option<i64>,
    /// Days past due (0 = due today). None for a new note.
    pub overdue: Option<i64>,
}

/// Notes due on or before `today`, most overdue first.
pub fn due(conn: &Connection, today: &str, limit: usize) -> Result<Vec<DueNote>> {
    let mut stmt = conn.prepare(
        "SELECT slug, title, kind, path, recall_due, recall_interval, recall_streak FROM notes
         WHERE recall_due IS NOT NULL AND recall_due <= ?1
         ORDER BY recall_due, slug LIMIT ?2",
    )?;
    let rows = stmt.query_map(rusqlite::params![today, limit as i64], row_to_due)?;
    let mut out: Vec<DueNote> = rows.collect::<Result<_, _>>()?;
    for n in &mut out {
        n.overdue = n.due.as_deref().and_then(|d| days_between(d, today));
    }
    Ok(out)
}

/// Lessons and section studies never reviewed, oldest file path order,
/// as candidates to start reviewing.
pub fn new_notes(conn: &Connection, limit: usize) -> Result<Vec<DueNote>> {
    let mut stmt = conn.prepare(
        "SELECT slug, title, kind, path, recall_due, recall_interval, recall_streak FROM notes
         WHERE recall_due IS NULL AND slug <> 'lessons_index'
           AND (kind = 'lesson' OR slug LIKE '%\\_study' ESCAPE '\\')
         ORDER BY path LIMIT ?1",
    )?;
    let rows = stmt.query_map([limit as i64], row_to_due)?;
    Ok(rows.collect::<Result<_, _>>()?)
}

/// The next due date after `today`, if any note is scheduled.
pub fn next_due(conn: &Connection, today: &str) -> Result<Option<(String, i64)>> {
    let row = conn.query_row(
        "SELECT MIN(recall_due), COUNT(*) FROM notes WHERE recall_due > ?1",
        [today],
        |r| Ok((r.get::<_, Option<String>>(0)?, r.get::<_, i64>(1)?)),
    )?;
    Ok(row.0.map(|d| (d, row.1)))
}

fn row_to_due(r: &rusqlite::Row) -> rusqlite::Result<DueNote> {
    Ok(DueNote {
        slug: r.get(0)?,
        title: r.get(1)?,
        kind: r.get(2)?,
        path: r.get(3)?,
        due: r.get(4)?,
        interval: r.get(5)?,
        streak: r.get(6)?,
        overdue: None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dates_round_trip() {
        assert_eq!(add_days("2026-10-19", 1).unwrap(), "2026-10-20");
        assert_eq!(add_days("2026-12-31", 1).unwrap(), "2027-01-01");
        assert_eq!(add_days("2028-02-28", 1).unwrap(), "2028-02-29");
        assert_eq!(days_between("2026-10-01", "2026-10-19"), Some(18));
        assert_eq!(format_date(0), "1970-01-01");
    }

    #[test]
    fn intervals_grow_and_reset() {
        assert_eq!(schedule(None, None, Grade::Good), (1, 1));
        assert_eq!(schedule(Some(1), Some(1), Grade::Good), (3, 2));
        assert_eq!(schedule(Some(3), Some(2), Grade::Good), (7, 3));
        assert_eq!(schedule(Some(7), Some(3), Grade::Good), (15, 4));
        assert_eq!(schedule(Some(15), Some(4), Grade::Again), (1, 0));
        assert_eq!(schedule(Some(10), Some(2), Grade::Hard), (12, 2));
        assert_eq!(schedule(Some(3), Some(2), Grade::Easy), (11, 3));
        assert_eq!(schedule(Some(300), Some(9), Grade::Good), (365, 10));
    }

    #[test]
    fn recall_fields_replace_or_add() {
        let raw = "---\ntitle: X\nrecall_due: 2026-10-01\nrecall_interval: 1\ntags: [a]\n---\n\n# X\n";
        let out = set_recall(raw, "2026-10-22", 3, 1).unwrap();
        assert_eq!(out, "---\ntitle: X\nrecall_due: 2026-10-22\nrecall_interval: 3\ntags: [a]\nrecall_streak: 1\n---\n\n# X\n");
        let bare = set_recall("# Y\n", "2026-10-20", 1, 1).unwrap();
        assert!(bare.starts_with("---\nrecall_due: 2026-10-20\n"));
        assert!(bare.ends_with("---\n# Y\n"));
    }
}
