# Phase 4: set up /review on your machine

Run from `~/Projects/didaskalos` with `claude "Do the task in docs/phase4_review_local_test.md"`.

1. `git pull`, then `cargo install --path crates/cli`. Fix any compile
   error, run `cargo test`, and commit the fix if one was needed.
2. Link the command (it is new, so nothing to merge):
   `ln -sf (pwd)/.claude/commands/review.md ~/.claude/commands/review.md`
3. Run `dsk due` and show the user its output.
4. Tell the user to open a new Claude Code session and type `/review`.
   Grades write `recall_due`, `recall_interval` and `recall_streak` into
   each reviewed note's frontmatter; nothing else in the note changes.
5. Report anything that failed, with the error text.
