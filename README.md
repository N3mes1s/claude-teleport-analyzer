# claude-teleport-analyzer

A Rust CLI tool to read and analyze Claude Code remote sessions (teleport/tengu) directly from the Anthropic API, without cloning repositories.

## What is this?

When you run `claude --teleport <session_id>`, Claude Code creates a remote session that runs in the cloud. This tool lets you inspect those sessions -- read transcripts, view metadata, search through events, and export data -- all from your terminal.

## Prerequisites

- **macOS, Linux, or Windows**
- **Rust 1.85+** (edition 2024)
- **Claude Code** logged in (`claude` CLI must have been authenticated at least once)

The tool reads your OAuth credentials from the same location Claude Code stores them (see [Authentication](#authentication) below).

## Installation

```bash
git clone <repo-url>
cd claude-teleport-analyzer
make setup    # installs git hooks + builds
cargo build --release
```

The binary will be at `target/release/claude-teleport-analyzer`.

## Usage

### List sessions

```bash
# List the 20 most recent sessions
claude-teleport-analyzer list

# Show only running sessions
claude-teleport-analyzer list -s running

# List sessions from 2025 onwards, limit to 5
claude-teleport-analyzer list --after 2025-01-01 -l 5

# Sessions in a date range
claude-teleport-analyzer list --after 2025-06-01 --before 2025-07-01
```

### Show session details

```bash
claude-teleport-analyzer show session_01QJaJSUgfY6khmFTzJaMqph
```

Displays: ID, title, status, type, model, source repo, branches, created/updated timestamps, and a resume command.

### Read session transcript

```bash
# Full transcript (all event types)
claude-teleport-analyzer read session_01QJaJSUgfY6khmFTzJaMqph

# Only user/assistant/system messages (skip tool progress, env logs, etc.)
claude-teleport-analyzer read session_01QJaJSUgfY6khmFTzJaMqph -c

# Filter by event type
claude-teleport-analyzer read session_01QJaJSUgfY6khmFTzJaMqph -t user
claude-teleport-analyzer read session_01QJaJSUgfY6khmFTzJaMqph -t assistant

# Search for text across all events (case-insensitive)
claude-teleport-analyzer read session_01QJaJSUgfY6khmFTzJaMqph -s "cargo test"
claude-teleport-analyzer read session_01QJaJSUgfY6khmFTzJaMqph -c -s "authentication"

# Limit number of events fetched
claude-teleport-analyzer read session_01QJaJSUgfY6khmFTzJaMqph -m 100
```

### Session summary

```bash
claude-teleport-analyzer summary session_01QJaJSUgfY6khmFTzJaMqph
```

Shows: title, status, event type breakdown, all tool use summaries, and user message previews.

### Loglines

```bash
claude-teleport-analyzer loglines session_01QJaJSUgfY6khmFTzJaMqph
```

Shows compact loglines from the session ingress endpoint (a lighter alternative to full events).

### Export to JSON

```bash
# Default output: session_export.json
claude-teleport-analyzer export session_01QJaJSUgfY6khmFTzJaMqph

# Custom output path
claude-teleport-analyzer export session_01QJaJSUgfY6khmFTzJaMqph -o my_session.json
```

Exports the full session metadata and all events as pretty-printed JSON.

## Event Types

The tool handles these event types from the sessions API:

| Type | Description | Shown with `-c` |
|------|-------------|-----------------|
| `system` | Session initialization (model, cwd, tools, version) | Yes |
| `user` | User messages | Yes |
| `assistant` | Assistant responses (text, thinking, tool use, tool results) | Yes |
| `tool_use_summary` | Compact summary of preceding tool calls | No |
| `tool_progress` | Real-time tool execution progress | No |
| `result` | Turn completion with duration metrics | Yes |
| `control_response` | Control plane responses (resume, ack) | No |
| `env_manager_log` | Environment setup logs (deps, builds) | No |

Unknown event types are deserialized as `Unknown` to ensure forward compatibility.

## Authentication

This tool uses the same OAuth credentials as Claude Code. It does **not** support API keys -- the sessions API requires Claude.ai OAuth authentication.

### Credential locations

| Platform | Primary | Fallback |
|----------|---------|----------|
| **macOS** | Keychain (`security find-generic-password -s 'Claude Code-credentials' -w`) | `~/.claude/.credentials.json` |
| **Linux** | `~/.claude/.credentials.json` | -- |
| **Windows** | `%USERPROFILE%\.claude\.credentials.json` | -- |

Set the `CLAUDE_CONFIG_DIR` environment variable to override the default `~/.claude/` directory on any platform.

### Auth flow

1. Load OAuth token from the credential store (see table above)
2. Fetch organization UUID from `GET /api/oauth/profile`
3. Use both for all subsequent API calls with headers: `Authorization`, `x-organization-uuid`, `anthropic-beta`, `anthropic-version`

## Development

```bash
make setup    # install git hooks + build
make check    # run fmt + clippy + tests
make test     # run tests only
make release  # build release binary
```

### Pre-commit hook

The project includes a pre-commit hook (`scripts/pre-commit`) that runs:
1. `cargo fmt --check`
2. `cargo clippy -- -D warnings`
3. `cargo test`

Install with `make setup` or manually: `cp scripts/pre-commit .git/hooks/pre-commit && chmod +x .git/hooks/pre-commit`

### Running tests

```bash
cargo test
```

Tests cover:
- Serde deserialization of all API types and event variants
- `SessionEvent` helper methods (`event_type()`, `created_at()`, `is_conversation()`)
- Content block formatting and truncation
- Date filter parsing
- Case-insensitive search across all event types
- Session ID validation
- Display functions (no-panic smoke tests)
- Serialization round-trips

## API Endpoints

The tool interacts with these Anthropic API endpoints:

| Endpoint | Method | Description |
|----------|--------|-------------|
| `/api/oauth/profile` | GET | Get organization UUID from OAuth token |
| `/v1/sessions` | GET | List all remote sessions |
| `/v1/sessions/{id}` | GET | Get session metadata |
| `/v1/sessions/{id}/events` | GET | Get paginated session events (1000/page, cursor: `?after_id=`) |
| `/v1/session_ingress/session/{id}` | GET | Get session loglines |

Required headers: `Authorization: Bearer <token>`, `x-organization-uuid: <uuid>`, `anthropic-beta: ccr-byoc-2025-07-29`, `anthropic-version: 2023-06-01`

## License

Apache 2.0 -- see [LICENSE](LICENSE).
