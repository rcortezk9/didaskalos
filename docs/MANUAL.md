# didaskalos: the manual

*didaskalos* (Greek: teacher) is a study companion for your Theology vault.
It reads your Obsidian notes and helps you **ask** questions of them,
**build lessons** from them, **review** them before you forget, **drill
apologetics** against them, and **study** an author's argument step by step.
Every answer points back to the note and heading it came from.

- [How it fits together](#how-it-fits-together)
- [Set it up (once)](#set-it-up-once)
- [A typical week](#a-typical-week)
- [The five study modes](#the-five-study-modes)
  - [/ask: answer a question from your notes](#ask-answer-a-question-from-your-notes)
  - [/lesson: build a full lesson](#lesson-build-a-full-lesson)
  - [/review: spaced-repetition quiz](#review-spaced-repetition-quiz)
  - [/apologia: apologetics drill](#apologia-apologetics-drill)
  - [/study: walk through an argument](#study-walk-through-an-argument)
- [Terminal commands (no Claude needed)](#terminal-commands-no-claude-needed)
- [What it writes into your vault](#what-it-writes-into-your-vault)
- [Settings](#settings)
- [Updating](#updating)
- [Troubleshooting](#troubleshooting)

---

## How it fits together

Think of it as a **library with a librarian and a tutor**.

- **The library is your vault.** It stays the single source of truth.
  didaskalos never moves or rewrites your notes. It only adds new notes
  (lessons, practice logs) and three review-date lines in a note's
  frontmatter.
- **The card catalogue is the index**
  (`~/.local/share/didaskalos/index.db`). It is a SQLite file holding every
  note split at its headings, a keyword index, a "meaning" index
  (embeddings, so "one true church" finds notes that never use those
  words), and your wikilinks. It is a cache: delete it and `dsk index`
  rebuilds it.
- **The librarian is `dsk`,** a small Rust program. In the terminal it
  searches and lists what is due. Inside Claude Code it runs as an MCP
  server (`dsk mcp`), handing Claude your notes through a few tools:
  search, read a note, follow links, save a lesson, record a review, log
  a drill.
- **The tutor is Claude Code.** The slash commands (`/ask`, `/lesson`,
  `/review`, `/apologia`, `/study`) are instructions telling Claude how to
  teach from what the librarian hands it, and always to cite the shelf
  it came from.

```
  Obsidian vault  ──dsk index──▶  index.db  ◀──dsk mcp──  Claude Code
  (your notes)                    (cache)                  /ask /lesson /review /apologia /study
       ▲                                                        │
       └──────── new lessons, Practice_Log, recall dates ───────┘
```

Everything runs on your machine. Claude Code uses your normal Claude
subscription. Spoken answers are transcribed locally, and no audio leaves
the computer.

---

## Set it up (once)

From the repo, in fish:

```fish
cd ~/Projects/didaskalos
git pull
cargo install --path crates/cli                  # installs `dsk` into ~/.cargo/bin
set -Ux DSK_VAULT ~/Documents/Notes/Theology     # so dsk always knows where the vault is
dsk completions fish > ~/.config/fish/completions/dsk.fish
dsk index                                        # first run: reads every note, then builds the meaning index (~25 min)
```

Connect it to Claude Code, and link the slash commands so they work from
any folder:

```fish
claude mcp add --scope user didaskalos -- dsk mcp --vault ~/Documents/Notes/Theology
mkdir -p ~/.claude/commands
for c in ask lesson review apologia study
    ln -sf (pwd)/.claude/commands/$c.md ~/.claude/commands/$c.md
end
claude mcp list                                  # should show didaskalos as connected
```

> **If you already have your own `/lesson` command** (a regular file at
> `~/.claude/commands/lesson.md`, not a link), move it aside first:
> `mv ~/.claude/commands/lesson.md ~/.claude/commands/lesson_original.md`.
> It stays available as `/lesson_original`. The phase 3 task
> (`docs/phase3_lessons_local_test.md`) merges what it does into the new
> one.

For spoken answers in `/apologia --voice` (optional):

```fish
pip install --user faster-whisper    # if pip refuses, use pipx or a venv (see Troubleshooting)
dsk listen --max 20                  # say a sentence; the first run downloads the model (~150 MB)
```

Open a **new** Claude Code session after setup. Servers and commands
load when a session starts.

> **Shortcut:** `claude "Set up didaskalos as docs/MANUAL.md 'Set it up (once)' describes, and report anything that fails."`

---

## A typical week

| When | What | Command |
|---|---|---|
| Morning, 10 minutes | See what is due, then quiz yourself on it | `dsk due`, then `/review` in Claude Code |
| A question comes up | Ask your notes, not the internet | `/ask what is the difference between latreia and dulia` |
| After reading a chapter or watching a talk | Turn it into a lesson in your format | `/lesson Raw/some_chapter.txt` |
| A topic feels scattered across notes | Pull it into one integrated lesson | `/lesson theosis` |
| Before a conversation or a comment thread | Drill the objection you expect | `/apologia papal`, or `/apologia --voice` |
| Reading a Father section by section | Rebuild the argument yourself | `/study On the Incarnation 33-40` |
| After writing new notes | Refresh the index | `dsk index` (seconds; only changed notes) |

Everything you review or drill gets a next date, so `dsk due` and
`/review` always know what to bring back.

---

## The five study modes

All five run inside Claude Code: type `claude` in any folder, then the
command. Each one reads your notes through `dsk mcp` and cites them as
`[[note#Heading]]`, which Obsidian opens directly.

### /ask: answer a question from your notes

```
/ask Why does the Orthodox Church not hold to sola scriptura?
/ask Explain theosis and how to achieve it
```

**What you get:**

1. **The short answer:** two or three sentences.
2. **From your notes:** the explanation, built only from your notes. Each
   source is cited once per run of sentences it supports, and again after
   any direct quote. Quotes are word for word; everything else is faithful
   paraphrase.
3. **Beyond your notes:** this appears only when your notes leave a real
   gap, and anything in it is labelled as general knowledge.
4. **Read next:** two to four notes worth opening.

Before answering, Claude checks every citation against the vault, so no
link points at a note or heading that doesn't exist.

**Tips.** Ask the way you would ask a teacher. Spelling slips and
abbreviations are fixed for you: "Anthanasius" finds Athanasius, and
PSA, HRM, LXX, OSB, TULIP, OT and NT are expanded. Questions with two
parts, like "what is X and why do we reject it", get searched both ways.

### /lesson: build a full lesson

```
/lesson theosis                                  # a topic → synthesis lesson
/lesson Raw/thinking_orthodox_ch07.txt           # one source → chapter lesson
/lesson Fr. Stephen talk on the canon            # a talk you have a transcript or notes for
```

It picks one of two formats, both copied from your existing lessons:

- **Synthesis lesson** (like *The Whole Counsel of God: An Integrated
  Lesson*). For a topic: Introduction, Parts I…N, Conclusion, Key
  Scripture Index, Key Figures Index, Reflection Questions and Source
  Analyses. Every claim cites your notes. Anything your notes don't cover
  is marked ⚠ and listed under *Not Sure / Check These*.
- **Chapter lesson** (like the *Thinking Orthodox* ones, "Text
  Analysis"). For one chapter, talk or video: Curated Takeaways (Core
  Claims, Supporting Evidence, Key Terminology, Synthesis & Application),
  Full Chapter Facts section by section, and Chapter Synthesis. Quotes
  are word for word, and paraphrase is marked.

**Before writing,** it checks lessons_index for a lesson on the same thing.
If it finds one, it asks whether to improve that lesson or write a new one.

**When it's done,** the note is saved into your vault and a row (date,
title, source, one-line hook) is added to `lessons_index.md`. The lesson
is searchable straight away.

**Safety:** it never replaces an existing file unless you ask. If you do,
the old version is copied to `.didaskalos/backups/` in the vault first.

### /review: spaced-repetition quiz

```
/review              # whatever is due today
/review theosis      # one note right now, due or not
```

1. It lists what is due (most overdue first). If nothing is due, it
   offers lessons you have never reviewed.
2. For each note it asks **4 to 6 questions, one at a time**, built from
   the note's own Review Questions with their Recall Key, Reflection
   Questions, Core Claims, terms and scripture. One is always "how would
   you answer someone who says…".
3. After each answer: what you got right, what you missed, and the note's
   own wording with its link. Say **skip** or **I don't know** to see the
   answer.
4. At the end of a note it proposes a grade. You can accept it or give
   your own.

| Grade | Meaning | Next review |
|---|---|---|
| again | missed most of it | tomorrow |
| hard | the gist, with real gaps | a little longer than last time (×1.2) |
| good | the main points | the gap grows: 1 → 3 → 7 → 15 → 33 days … |
| easy | all of it, at once | about 1.5 × the "good" gap |

The schedule is written into the note's own frontmatter (`recall_due`,
`recall_interval`, `recall_streak`), the fields your vault already used.
Nothing else in the note changes.

### /apologia: apologetics drill

```
/apologia                 # the most useful card: due first, then never tried
/apologia papal           # a card about the papacy
/apologia Isaiah 53       # find a specific objection
/apologia --voice         # answer out loud
```

**The cards** come from your response notes:

- each `## Reason N` block, as in *Ten Reasons Not to Convert*;
- each `_response` note built on The Claim / Where It Goes Wrong / … headings;
- each section of `elevator_statements`;
- each `Q1: "…"` item under a Q&A or objections heading.

**A round:**

1. **The objection.** A realistic objector, such as a Reformed Baptist, a
   Catholic apologist, a Hebrew Roots teacher or a skeptic, states it in
   their own voice with their best verse. You don't see your note.
2. **You answer,** typed or spoken.
3. **Pushback.** One or two pushbacks in character, taken from the note's
   *What They'll Say Next*.
4. **Grade.** Your first answer is scored out of 24: eight criteria, 0 to 3
   each, each checked against your note's matching heading.

| # | Criterion | Checked against |
|---|---|---|
| 1 | Fair statement of the objection | The Claim, Charity Check |
| 2 | Root error named | Where It Goes Wrong |
| 3 | Scripture, in context (LXX/OSB where it matters) | Scriptural Ground |
| 4 | Fathers and councils, accurately | Patristic Witness |
| 5 | Turning their own premise | Their Inconsistency |
| 6 | Ready for the next move | What They'll Say Next |
| 7 | Honesty: concede what's true, opinion ≠ dogma | Charity Check, Vault Gaps |
| 8 | Delivery: plain, short, no point-scoring | Draft Reply |

   - **Accuracy gate.** A made-up quote, a wrong verse or a misstated
     canon caps the score at 12.
   - **Critique.** You get at most four critique points, each citing your
     note.
   - **Stronger reply.** You get a stronger response adapted from your
     Draft Reply.
   - **Vault gaps.** If your notes have nothing on a point, it says "vault
     gap" instead of inventing a source. It offers to add the gap to that
     note's Vault Gaps list, and changes the note only if you say yes.

**Bands and scheduling:**

| Score | Band | The card comes back |
|---|---|---|
| 20–24 | ready to use live | double the last gap (at least 2 weeks) |
| 14–19 | sound, needs one fix | in a week |
| 8–13 | re-read the source note | in 2 days |
| under 8 | build a full lesson on this (it suggests `/lesson`) | tomorrow |

Every attempt is saved to `Practice_Log/` in your vault, **one note per
objection**, each attempt appended with your answer, the score table, the
critique and the stronger reply. Open it in Obsidian to see your progress
on that objection over time.

**Answering out loud (`--voice`).** Claude runs `dsk listen`, which
records until you **pause for 3 seconds**. Your speech is transcribed on
your machine, with theology terms primed (Theotokos, latreia,
hilastērion, theologoumenon …). You see the transcript and can fix
misheard words before grading. Delivery also weighs the spoken checks:
whether your first sentence carries the point, filler words, and length
(60–90 seconds is the target for one turn). The recording is deleted
afterwards.

### /study: walk through an argument

```
/study On the Incarnation 20-32
/study                                  # the next due study note, or the next block in a series
/study Raw/on_the_incarnation_41_57.txt # no study note yet: it builds one first
```

The point is to **rebuild the author's argument yourself**, not to read
a summary.

1. **The Core Problem.** What the last block left open, and the question
   this one answers. Then: what do you expect the first move to be?
2. **The chain, one step at a time.** It asks each step's connector
   question (*"Then why not die privately and reappear later?"*), waits for
   your answer, then shows the step with its § and quotes, and says
   whether you got it, partly got it, or missed it. Say **skip** to see a
   step, or **faster** for two steps at a time. At the end you say the whole
   chain back before it shows the one-line version.
3. **Does it hold up?** It asks for the strongest step, the weakest (in
   Adler's terms: uninformed, misinformed, illogical or incomplete) and
   your verdict. It compares them with your note. Disagreeing is fine if
   you argue for it.
4. **Review Questions.** The note's own questions, checked against its
   Recall Key. Anything marked ⚠ or listed under *Not Sure / Check These*
   is flagged so you don't learn it as certain.
5. **Schedule.** A grade goes into the note's recall dates, so study notes
   also come back in `/review` and `dsk due`. It ends with the steps to
   reread and the link to the next block.

If no study note exists for the text, it builds one first. It uses your
`section-study` skill if you have it, otherwise the same format from
`crates/cli/templates/section_study.md`. Study notes are not added to
lessons_index, as before.

---

## Terminal commands (no Claude needed)

| Command | What it does |
|---|---|
| `dsk index` | Bring the index up to date. Only changed notes are re-read, so it takes seconds after the first run. New sections get meaning vectors. |
| `dsk index --full` | Rebuild everything from scratch (rarely needed). |
| `dsk search is Adam's guilt inherited` | Ranked notes with the sections that matched and their `[[note#Heading]]` links. `-n 3` for fewer, `--json` for scripts. |
| `dsk due` | What is due for review today (with days overdue), and how many lessons you have never reviewed. |
| `dsk listen` | Record a spoken answer and print the transcript plus a delivery line. `--max 120` sets the longest recording, `--silence 4` the pause that ends it, `--keep` keeps the audio, and `--file x.wav` transcribes an existing file. |
| `dsk stats` | How many notes, sections and links the index holds, by layer. |
| `dsk mcp` | The server Claude Code starts by itself. You never run this by hand. |
| `dsk completions fish` | Shell completions. |

**How search ranks**, in short. It finds keyword matches (with spelling
fixes) and meaning matches, merges the two lists, then favours your Wiki
concept pages and lessons over raw transcripts and boosts notes your own
links tie to the top results. Practice logs rank lowest, with the raw
material, because they hold your own answers, right or wrong.

---

## What it writes into your vault

| What | Where | When |
|---|---|---|
| New lesson notes | Where the lesson template says, e.g. beside the book's notes or `Orthodox/Lessons/` | `/lesson`, and `/study` when it builds a study note |
| One row per lesson | `lessons_index.md` | `/lesson` (not for study notes) |
| `recall_due`, `recall_interval`, `recall_streak` | The reviewed note's frontmatter (only those three lines) | `/review`, `/study` |
| One note per objection, attempts appended | `Practice_Log/` | `/apologia` |
| A line in a note's Vault Gaps | That response note | `/apologia`, only when you say yes |
| Backups of anything replaced | `.didaskalos/backups/` (hidden from Obsidian and the index) | `/lesson` with overwrite, and lessons_index edits |

It never deletes notes. The only existing files it changes are
`lessons_index.md` (one row per lesson), the three recall lines in a
reviewed note, a lesson you asked it to replace, and a Vault Gaps line
you approved. Because the vault is plain files, git or your usual backup
covers all of it.

---

## Settings

| Variable | Default | Use |
|---|---|---|
| `DSK_VAULT` | `~/Documents/Notes/Theology` | The vault folder (or pass `--vault`). |
| `DSK_DB` | `~/.local/share/didaskalos/index.db` | Where the index lives (or pass `--db`). |
| `DSK_WHISPER_MODEL` | `base.en` | Transcription model. `small.en` is more accurate and slower. |
| `DSK_TRANSCRIBE` | (unset) | Use a different transcriber: any command that takes the WAV path last and prints text, e.g. a whisper.cpp `whisper-cli` wrapper. |
| `DSK_TODAY` | today | Pretend it is another date (YYYY-MM-DD). Useful for catching up after a missed day, or for testing. |

The embedding model is cached in `~/.cache/didaskalos/`.

---

## Updating

When new work is pushed:

```fish
cd ~/Projects/didaskalos
git pull
cargo install --path crates/cli     # only needed when Rust code changed
```

Changes to slash commands take effect in your next Claude Code session
with no reinstall, because the commands are links into the repo. Each
release also ships a short task file in `docs/` (for example
`docs/phase6_study_local_test.md`). Running
`claude "Do the task in docs/<file>"` does that release's setup for you.

---

## Troubleshooting

| Problem | Fix |
|---|---|
| `/ask` etc. don't appear in Claude Code | Open a new session. Check `ls -l ~/.claude/commands/` shows links into the repo. |
| Claude says the didaskalos tools aren't available | Run `claude mcp list`. If it isn't there or isn't connected, re-run the `claude mcp add …` line from setup, and check `dsk` is on your PATH (`which dsk`). |
| A new note isn't found | `dsk mcp` refreshes keywords when a session starts. Run `dsk index` for meaning search on new notes too. |
| Search feels keyword-only, or prints "meaning search off" | The embedding model didn't load. Run `dsk index`, which downloads it if missing and fills in missing vectors. |
| `fatal: not a git repository` | Check the folder name: `~/Projects/didaskalos`. |
| `dsk listen`: "no recorder found" | `sudo apt install alsa-utils` (for `arecord`). |
| `dsk listen`: "no speech heard in 20 seconds" | Check the input device in COSMIC Settings → Sound, then test with `arecord -d 3 /tmp/t.wav && aplay /tmp/t.wav`. |
| `dsk listen`: "faster-whisper is not installed" | `pip install --user faster-whisper`. If pip refuses (externally managed environment), make a venv (`python3 -m venv ~/.venvs/whisper; ~/.venvs/whisper/bin/pip install faster-whisper`) and point `DSK_TRANSCRIBE` at a small script that runs it, or ask Claude to set that up. |
| Transcripts misspell a term | Fix it when the transcript is shown. For better accuracy overall, `set -Ux DSK_WHISPER_MODEL small.en`. |
| Something looks wrong in the index | Delete `~/.local/share/didaskalos/index.db` and run `dsk index`. It is only a cache. |
| A lesson was replaced by mistake | The old file is in `.didaskalos/backups/` in the vault, named with a timestamp. |
