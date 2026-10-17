# Phase 2: try the MCP server and /ask on your machine

Run this from `~/Projects/didaskalos` with `claude "Do the task in docs/phase2_mcp_local_test.md"`.

1. `git pull`, then `cargo install --path crates/cli` (with the default
   `embed` feature, so search uses the vectors already built). The
   model cache in `vector_candidates` (crates/core/src/search.rs) could
   only be built without `embed` in the cloud; if it fails to compile,
   fix it there, run `cargo test`, and commit the fix.
2. Register the server and the command:
   ```fish
   claude mcp add --scope user didaskalos -- dsk mcp --vault /home/recocortez/Documents/Notes/Theology
   mkdir -p ~/.claude/commands
   ln -sf (pwd)/.claude/commands/ask.md ~/.claude/commands/ask.md
   ```
   If `didaskalos` is already registered, `claude mcp remove --scope user didaskalos` first.
3. Check the server answers: `claude mcp list` should show `didaskalos` as connected.
   Also pipe a request straight in:
   ```fish
   printf '%s\n' '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}' \
     '{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"search","arguments":{"query":"one true church","limit":3}}}' \
     | dsk mcp --vault /home/recocortez/Documents/Notes/Theology
   ```
   The second reply should list Church notes, not True Israel ones, and
   searching twice in one session should not reload the model each time.
4. Tell the user it is set up, and that they can open a new Claude Code
   session anywhere and try `/ask Explain Theosis and how to achieve it?`.
   (A new session is needed: MCP servers and commands load at start.)
5. Report anything that failed, with the error text.
