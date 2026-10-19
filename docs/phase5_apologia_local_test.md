# Phase 5: set up /apologia and spoken answers on your machine

Run from `~/Projects/didaskalos` with `claude "Do the task in docs/phase5_apologia_local_test.md"`.

1. `git pull`, then `cargo install --path crates/cli`. Fix any compile
   error, run `cargo test`, and commit the fix if one was needed.
2. Look in `~/.claude/commands/` and `~/.claude/skills/` for the user's
   own apologetics commands (for example `apologetics*.md` or
   `apologetics-gap`). Read them. If one overlaps `/apologia`, do not
   delete or change it. Fold into `.claude/commands/apologia.md` anything
   it does that `/apologia` lacks and that fits a drill. Commit and push
   that, and tell the user what you merged. Leave `/apologetics-gap` as it
   is: `/apologia` only offers to add vault gaps.
3. Link the command:
   `ln -sf (pwd)/.claude/commands/apologia.md ~/.claude/commands/apologia.md`
4. Check the drill bank: start `dsk mcp` with a `tools/call` for
   `objections` (see docs/phase2_mcp_local_test.md for the JSON line) and
   report how many cards it finds. The vault copy in the cloud had 27.
5. Spoken answers:
   - Check `arecord --version` (alsa-utils). If it is missing, check
     `parecord --version` instead.
   - Install faster-whisper with `pip install --user faster-whisper`. If
     pip refuses because of an externally managed environment, use
     `pipx` or a venv. The command must work as `python3 -c "import faster_whisper"`
     from the user's normal shell. If it only works inside a venv, tell the
     user to set `DSK_TRANSCRIBE` or activate that venv.
   - Ask the user to say one sentence, run `dsk listen --max 20`, and show
     the transcript. The first run downloads the model (about 150 MB for
     base.en).
6. Tell the user to open a new Claude Code session and try `/apologia`,
   then `/apologia --voice`.
7. Report anything that failed, with the error text.
