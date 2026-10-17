# Phase 1b: meaning search (embeddings)

A brief for a Claude Code session run on Demetrios's machine, where the
embedding model can be downloaded. Run from the repo root:

```fish
claude "Do the task in docs/phase1b_embeddings.md"
```

## Task

1. Read README.md and crates/core/src/{search.rs,query.rs,index.rs,db.rs}.
2. Add local embeddings behind a cargo feature `embed` (default on) using the
   `fastembed` crate (BGESmallENV15, 384 dims, CPU). Store one f32 vector per
   chunk in a new `chunk_vec(chunk_id INTEGER PRIMARY KEY, vec BLOB)` table
   (bump SCHEMA_VERSION). Embed `heading_path + "\n" + text`, truncated to
   the model limit. Brute-force cosine in Rust is fine (~9.4k chunks).
   Embed only new or changed notes during `dsk index`, in batches, with
   progress output. Cache the model under ~/.cache/didaskalos.
3. Make search hybrid: take the top ~300 chunks by cosine, merge with the
   keyword candidates by reciprocal rank fusion (k=60), then keep the
   existing layer boost, section and hub weights, title boost and link
   boost. Keyword-only search must still work when the feature is off or
   vectors are missing.
4. Test on the real vault: `dsk index --vault ~/Documents/Notes/Theology`,
   then run every question in eval/questions.txt with `dsk search -n 5`.
   Keyword-only currently misses:
   - #4 (how Orthodoxy differs). Expected: thinking_orthodox_ch01_lesson
     ("Distinctiveness of Orthodox Christianity"), source_7_orthodox_beliefs_shock.
   - #7 (one true church). Expected: source_orthodoxy_true_church_history
     ("Orthodoxy IS the TRUE CHURCH"), ch01_holy_church,
     concept_church_history_and_apostolicity.
   Do not regress the other 8. Record before/after top-5 for all 10 in
   eval/results.md.
5. `cargo test` must pass; add a unit test for the RRF merge. Commit and
   push to main.
6. `cargo install --path crates/cli`, and write Fish completions with
   `dsk completions fish > ~/.config/fish/completions/dsk.fish`.
