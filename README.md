# didaskalos

A study companion for an Obsidian theology vault. Phase 1 is the
retrieval core: it indexes the vault into one SQLite file and answers
searches with the notes and sections that matter, cited as
`[[note#Heading]]` links Obsidian opens.

Later phases put an MCP server on top so Claude Code can answer
questions, build lessons, quiz you and coach apologetics from your notes.
The design doc lives in the project.

## Install

```fish
cargo install --path crates/cli        # installs `dsk` into ~/.cargo/bin
dsk completions fish > ~/.config/fish/completions/dsk.fish
set -Ux DSK_VAULT ~/Documents/Notes/Theology   # or pass --vault
```

## Use

```fish
dsk index                     # first run reads everything; later runs only changed notes
dsk search is Adam\'s guilt inherited
dsk search -n 3 --json uncreated energies   # JSON for scripts and the MCP server
dsk stats
```

The index lives at `~/.local/share/didaskalos/index.db` (override with
`--db` or `$DSK_DB`). It is a cache: delete it any time and run
`dsk index` again.

## How search ranks

1. **Keywords.** SQLite FTS5 with Porter stemming over every heading
   section ("chunk"), weighting the note title and heading above the body.
2. **Coverage.** Sections containing more of the question's words rank higher.
3. **Vault layer.** Wiki concept pages ×1.5, lessons ×1.3, Wiki sources
   and concept maps ×1.2, books and analyses ×1.0, daily readings ×0.9,
   raw transcripts ×0.7. Link lists (Cross-References, Related, Source)
   are down-weighted because they match on slugs alone.
4. **Your links.** Notes linked to or from the top five results gain a
   share of their score, and linked notes the words missed are listed
   under "Linked from these results".

Meaning-based (embedding) search is the next step; see the design doc.

## Layout

```
crates/core   parsing (note.rs), schema (db.rs), indexing (index.rs), ranking (search.rs)
crates/cli    the `dsk` binary
```

Run the tests with `cargo test`.
