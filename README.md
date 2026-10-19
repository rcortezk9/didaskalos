# didaskalos

A study companion for an Obsidian theology vault. It indexes the vault
into one SQLite file, answers searches with the notes and sections that
matter, cited as `[[note#Heading]]` links Obsidian opens, and serves
those notes to Claude Code so it can answer questions from them (`/ask`).

Later phases add lessons, spaced-repetition review and apologetics
coaching on the same server. The design doc lives in the project.

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
dsk due                       # notes due for review today
dsk listen                    # record a spoken answer and print the transcript
```

The index lives at `~/.local/share/didaskalos/index.db` (override with
`--db` or `$DSK_DB`). It is a cache: delete it any time and run
`dsk index` again.

## Use it from Claude Code

`dsk mcp` serves the vault to Claude Code as tools (MCP over stdio).
Register it once for every project, and link the slash commands into
your user commands so they work from any folder:

```fish
claude mcp add --scope user didaskalos -- dsk mcp --vault ~/Documents/Notes/Theology
mkdir -p ~/.claude/commands
ln -sf (pwd)/.claude/commands/ask.md ~/.claude/commands/ask.md
ln -sf (pwd)/.claude/commands/lesson.md ~/.claude/commands/lesson.md
ln -sf (pwd)/.claude/commands/review.md ~/.claude/commands/review.md
ln -sf (pwd)/.claude/commands/apologia.md ~/.claude/commands/apologia.md
```

Then in Claude Code: `/ask why don't the Orthodox hold to sola scriptura`.
The answer cites sections as `[[slug#Heading]]`, separates what your
notes say from anything added under "Beyond your notes", and is checked
so every citation points at a real note and heading.

`/lesson theosis` writes a synthesis lesson from your notes;
`/lesson Raw/some_chapter.txt` writes a two-tier chapter lesson from a
source. Either way the note is saved into the vault and its row added to
`lessons_index.md`. The templates are in `crates/cli/templates/`.

`/review` quizzes you on the notes due today, one question at a time,
then grades each note (again, hard, good, easy) and writes the next
review date into its `recall_due` / `recall_interval` / `recall_streak`
frontmatter. Gaps grow 1, 3, 7, 15, 33 days with good answers and drop
back to 1 day after a miss. `/review theosis` reviews one note now.
`dsk due` lists what is due from the terminal.

`/apologia` runs an apologetics drill. A realistic objector raises an
objection from your response notes, you answer, it pushes back once or
twice, and then it grades your first answer on eight criteria out of 24
against the note's own headings (The Claim, Where It Goes Wrong,
Scriptural Ground …), with a critique and a stronger reply. Each attempt
is logged to `Practice_Log/` in the vault, one note per objection, and
the score sets when that objection comes back. `/apologia papal --voice`
lets you answer aloud.

Spoken answers use `dsk listen`. It records from the microphone until you
pause (or press Enter in a terminal), transcribes on your machine with
faster-whisper, and prints the transcript with length, pace and
filler-word checks. The audio is deleted unless you pass `--keep`.

```fish
pip install --user faster-whisper     # once; the model downloads on first use
dsk listen                            # try it; set DSK_WHISPER_MODEL=small.en for better accuracy
```

The server's tools:

| Tool | What it gives Claude |
|------|----------------------|
| `search` | Best notes with the full text of their best sections, each headed by its citation |
| `read_note` | A whole note, or one section by heading |
| `neighbors` | What a note links to and what links to it |
| `lessons` | Rows of `lessons_index.md`, filterable by words |
| `check_citations` | Which `[[...]]` links in a draft point at missing notes or headings |
| `lesson_format` | The chapter or synthesis lesson template and its rules |
| `due` | Notes due for review today, most overdue first, and lessons never reviewed |
| `record_review` | Grades a review and writes the next due date into the note |
| `objections` | The objection bank built from your response notes, with each card's practice history and what is due |
| `log_practice` | Writes a graded drill to the card's `Practice_Log/` note and schedules the card from the score |
| `save_lesson` | Writes a lesson into the vault and its row into `lessons_index.md`; never replaces a file without `overwrite`, and backs up whatever it replaces to `.didaskalos/backups/` |

On start it brings the keyword index up to date, so notes written since
the last `dsk index` are searchable at once; run `dsk index` for their
vectors.

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
crates/cli    the `dsk` binary and the MCP server (mcp.rs)
crates/cli/templates   lesson templates served by the lesson_format tool
.claude/commands   slash commands for Claude Code (/ask, /lesson)
```

Run the tests with `cargo test`.

## Checking search quality

`eval/questions.txt` holds real study questions. After changing the
ranking, run them all and compare the top results:

```fish
for q in (cat eval/questions.txt); echo "## $q"; dsk search -n 5 $q | grep -E '^ *[0-9]+\. '; end
```
