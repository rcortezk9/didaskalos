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
   Terms of four letters or more match as prefixes ("orthodox" finds
   "orthodoxy"), known abbreviations expand (PSA → penal substitutionary
   atonement), and misspellings are corrected against the vault's own
   words (Anthanasius → Athanasius).
2. **Coverage and phrases.** Sections containing more of the question's
   words rank higher, weighted by how rare each word is in the vault, and
   word pairs that appear side by side ("sola scriptura", "ancestral sin")
   add a bonus. A note whose title names the subject is lifted further.
3. **Vault layer.** Wiki concept pages ×1.5, lessons ×1.3, Wiki sources
   and concept maps ×1.2, books and analyses ×1.0, daily readings ×0.9,
   raw transcripts ×0.7. Indexes and the ingest log ×0.3. Link lists (Cross-References, Related, Source)
   are down-weighted because they match on slugs alone.
4. **Your links.** Notes linked to or from the top five results gain a
   share of their score, and linked notes the words missed are listed
   under "Linked from these results".

5. **Meaning.** `dsk index` also embeds every section with a small local
   model (BGE small English, 384 numbers per section, on the CPU). At
   search time the 300 sections closest in meaning to the question are
   merged with the keyword list by reciprocal rank fusion (k=60), then the
   layer, section, hub, title and link weights above apply. Questions that
   share no words with the right note ("how is Orthodoxy different") can
   now find it. The first `dsk index` downloads the model into
   `~/.cache/didaskalos` and embeds the whole vault (about 25 minutes on
   10 cores); later runs embed only new or changed notes.

   Build with `--no-default-features` to leave this out. Keyword search
   also works alone if the vectors or the model are missing.

## Layout

```
crates/core   parsing (note.rs), schema (db.rs), indexing (index.rs), embeddings (embed.rs), ranking (search.rs)
crates/cli    the `dsk` binary
```

Run the tests with `cargo test`.

## Checking search quality

`eval/questions.txt` holds real study questions. After changing the
ranking, run them all and compare the top results:

```fish
for q in (cat eval/questions.txt); echo "## $q"; dsk search -n 5 $q | grep -E '^ *[0-9]+\. '; end
```
