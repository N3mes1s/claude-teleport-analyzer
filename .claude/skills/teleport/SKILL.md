---
name: teleport
description: Read and analyze Claude Code remote sessions. List sessions, read transcripts, export events, and generate flamegraph visualizations of any remote session.
argument-hint: [command] [session_id] [options]
allowed-tools: Bash Read Grep Glob
---

# Claude Teleport Analyzer

Inspect remote Claude Code sessions: $ARGUMENTS

## Available Commands

### Session Discovery
- `/teleport list` — List remote sessions (most recent first)
- `/teleport list --status running` — Filter by status (running, idle, completed)
- `/teleport list --after 2025-01-01` — Filter by date

### Session Inspection
- `/teleport show <session_id>` — Show session metadata (model, status, sources, outcomes)
- `/teleport read <session_id>` — Read full conversation transcript
- `/teleport read <session_id> -c` — Conversation only (skip tool_progress, env_manager_log)
- `/teleport read <session_id> -s "search term"` — Search within events
- `/teleport summary <session_id>` — Compact summary with event counts and user messages
- `/teleport loglines <session_id>` — Show session_ingress loglines

### Export & Visualization
- `/teleport export <session_id>` — Export session + events to JSON
- `/teleport flamegraph <session_id>` — Export then visualize with token flamegraph toolkit

## Implementation

The binary is built from this repo:

```bash
cd /home/user/claude-teleport-analyzer
```

If not built yet, run `cargo build` first.

Route based on first argument:

| Argument | Command |
|----------|---------|
| `list` | `cargo run -- list $REST_ARGS` |
| `show` | `cargo run -- show $SESSION_ID` |
| `read` | `cargo run -- read $SESSION_ID $REST_ARGS` |
| `summary` | `cargo run -- summary $SESSION_ID` |
| `loglines` | `cargo run -- loglines $SESSION_ID` |
| `export` | `cargo run -- export $SESSION_ID $REST_ARGS` |
| `flamegraph` | `cargo run -- export $SESSION_ID -o /tmp/teleport_export.json && cd /home/user/Playground/token-flamegraph && python3 cli.py /tmp/teleport_export.json` |

### Authentication

The tool tries auth sources in order:
1. `ANTHROPIC_API_KEY` env var (API key via x-api-key header)
2. OAuth token from remote session files (CLAUDE_SESSION_INGRESS_TOKEN_FILE)
3. `~/.claude/.credentials.json` or macOS Keychain

### Notes
- Session IDs look like `session_01QJaJSUgfY6khmFTzJaMqph`
- The flamegraph subcommand requires the token-flamegraph toolkit in `/home/user/Playground/token-flamegraph/`
- Export format is `{session, events, exported_at, total_events}` JSON
