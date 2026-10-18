# Phase 3: set up /lesson on your machine

Run from `~/Projects/didaskalos` with `claude "Do the task in docs/phase3_lessons_local_test.md"`.

1. `git pull`, then `cargo install --path crates/cli`. Fix any compile
   error, run `cargo test`, and commit the fix if one was needed.
2. Check `~/.claude/commands/lesson.md` before linking anything:
   - If it does not exist, or is already a symlink into this repo, run
     `ln -sf (pwd)/.claude/commands/lesson.md ~/.claude/commands/lesson.md`.
   - If it is the user's own `/lesson` command (a regular file), do not
     replace it yet. Move it to `~/.claude/commands/lesson_original.md`,
     read it, and compare it with `.claude/commands/lesson.md` and
     `crates/cli/templates/`. Fold into the repo's versions anything the
     original does that they lack (for example how it reads PDF scans or
     photos, scan notes, file naming, folders, tags). Commit and push that,
     then make the symlink. Tell the user the original is kept as
     `/lesson_original`.
3. Do the same check for `~/.claude/skills/section-study` only to note
   that it exists; leave it alone (it is the next phase).
4. Tell the user to open a new Claude Code session and try
   `/lesson theosis`. The lesson is saved into the vault and its row added
   to `lessons_index.md`; any file replaced is backed up in the vault's
   `.didaskalos/backups/`.
5. Report what was merged from the original command, and anything that failed.
