//! `dsk`: the terminal front end for didaskalos.
//!
//!   dsk index            bring the index up to date (only changed notes)
//!   dsk search "query"   ranked notes with the sections that matched
//!   dsk stats            what the index holds
//!   dsk completions fish > ~/.config/fish/completions/dsk.fish

use std::io::Write;
use std::path::PathBuf;
use std::time::Instant;

use anyhow::{bail, Result};
use clap::{CommandFactory, Parser, Subcommand};
use didaskalos_core::{db, index, search};

#[derive(Parser)]
#[command(name = "dsk", version, about = "Study your theology vault from the terminal")]
struct Cli {
    /// Vault folder. Defaults to $DSK_VAULT, then ~/Documents/Notes/Theology.
    #[arg(long, global = true)]
    vault: Option<PathBuf>,

    /// Index file. Defaults to $DSK_DB, then ~/.local/share/didaskalos/index.db.
    #[arg(long, global = true)]
    db: Option<PathBuf>,

    #[command(subcommand)]
    cmd: Cmd,
}

#[derive(Subcommand)]
enum Cmd {
    /// Bring the index up to date with the vault.
    Index {
        /// Re-read every note instead of only changed ones.
        #[arg(long)]
        full: bool,
    },
    /// Search the vault and list the best notes with matching sections.
    Search {
        /// What to look for, e.g. "is Adam's guilt inherited".
        query: Vec<String>,
        /// How many notes to show.
        #[arg(short = 'n', long, default_value_t = 8)]
        limit: usize,
        /// Print JSON (for scripts and the MCP server).
        #[arg(long)]
        json: bool,
    },
    /// Show what the index holds.
    Stats,
    /// Print shell completions, e.g. `dsk completions fish`.
    Completions { shell: clap_complete::Shell },
}

fn vault_path(cli: &Cli) -> PathBuf {
    cli.vault
        .clone()
        .or_else(|| std::env::var_os("DSK_VAULT").map(PathBuf::from))
        .unwrap_or_else(|| {
            let home = std::env::var("HOME").unwrap_or_else(|_| ".".into());
            PathBuf::from(home).join("Documents/Notes/Theology")
        })
}

fn db_path(cli: &Cli) -> PathBuf {
    cli.db
        .clone()
        .or_else(|| std::env::var_os("DSK_DB").map(PathBuf::from))
        .unwrap_or_else(db::default_db_path)
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    if let Cmd::Completions { shell } = &cli.cmd {
        clap_complete::generate(*shell, &mut Cli::command(), "dsk", &mut std::io::stdout());
        return Ok(());
    }

    let mut conn = db::open(&db_path(&cli))?;

    match &cli.cmd {
        Cmd::Index { full } => {
            let vault = vault_path(&cli);
            if !vault.is_dir() {
                bail!("vault folder not found: {} (set --vault or $DSK_VAULT)", vault.display());
            }
            let t = Instant::now();
            let r = index::update(&mut conn, &vault, *full)?;
            println!(
                "{} files scanned: {} added, {} updated, {} removed, {} unchanged ({} new chunks) in {:.1}s",
                r.scanned, r.added, r.updated, r.removed, r.unchanged, r.chunks,
                t.elapsed().as_secs_f64()
            );
            embed_vectors(&mut conn)?;
        }
        Cmd::Search { query, limit, json } => {
            let q = query.join(" ");
            if q.trim().is_empty() {
                bail!("give a query, e.g. dsk search \"uncreated energies\"");
            }
            let res = search::search(&conn, &q, *limit)?;
            if *json {
                println!("{}", serde_json::to_string_pretty(&res)?);
            } else {
                print_results(&res)?;
            }
        }
        Cmd::Stats => {
            let s = index::stats(&conn)?;
            println!("{} notes, {} chunks, {} links ({} point at notes not in the vault)",
                s.notes, s.chunks, s.links, s.dangling_links);
            for (kind, n) in s.by_kind {
                println!("  {kind:14} {n}");
            }
        }
        Cmd::Completions { .. } => unreachable!(),
    }
    Ok(())
}

#[cfg(feature = "embed")]
fn embed_vectors(conn: &mut rusqlite::Connection) -> Result<()> {
    let t = Instant::now();
    let n = index::embed_missing(conn, |done, total| {
        eprint!("\rembedding chunks: {done}/{total}");
    })?;
    if n > 0 {
        eprintln!("\rembedded {n} chunks in {:.0}s{:20}", t.elapsed().as_secs_f64(), "");
    }
    Ok(())
}

#[cfg(not(feature = "embed"))]
fn embed_vectors(_conn: &mut rusqlite::Connection) -> Result<()> {
    Ok(())
}

fn print_results(res: &search::Results) -> Result<()> {
    let mut out = std::io::stdout().lock();
    if !res.corrections.is_empty() {
        writeln!(out, "(searched with {})\n", res.corrections.join(", "))?;
    }
    if res.hits.is_empty() {
        writeln!(out, "No matches for \"{}\".", res.query)?;
        return Ok(());
    }
    for (i, hit) in res.hits.iter().enumerate() {
        writeln!(out, "{:>2}. {}  ({}, {:.1})", i + 1, hit.title, hit.kind, hit.score)?;
        writeln!(out, "    {}", hit.path)?;
        if let Some(via) = &hit.linked_with {
            writeln!(out, "    linked with: {via}")?;
        }
        for c in &hit.chunks {
            writeln!(out, "    {}  {}", hit.cite(c), c.snippet)?;
        }
        writeln!(out)?;
    }
    if !res.related.is_empty() {
        writeln!(out, "Linked from these results:")?;
        for r in &res.related {
            writeln!(out, "  [[{}]]  {}", r.slug, r.title)?;
        }
    }
    Ok(())
}
