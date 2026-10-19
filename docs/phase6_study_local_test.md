# Phase 6: set up /study on your machine

Run from `~/Projects/didaskalos` with `claude "Do the task in docs/phase6_study_local_test.md"`.

1. `git pull`, then `cargo install --path crates/cli`. Fix any compile
   error, run `cargo test`, and commit the fix if one was needed.
2. Link the command:
   `ln -sf (pwd)/.claude/commands/study.md ~/.claude/commands/study.md`
3. If `~/.claude/skills/section-study` exists, read its SKILL.md and
   compare it with `crates/cli/templates/section_study.md`. Make the
   template match the skill's current format: headings, order,
   frontmatter and rules. Do not change the skill. Commit and push that,
   and tell the user what changed.
4. Tell the user to open a new Claude Code session and try
   `/study On the Incarnation 20-32`.
5. Report anything that failed, with the error text.
