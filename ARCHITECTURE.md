# Architecture

## Overview

claude-teleport-analyzer is a Rust CLI that reads Claude Code remote session data from the Anthropic API using OAuth credentials.

```mermaid
graph TD
    User[User CLI Input] --> Main[main.rs<br/>CLI parsing · command dispatch<br/>filtering · search · date logic]
    Main --> Client[client.rs<br/>API calls · auth flow]
    Main --> Display[display.rs<br/>colored terminal formatting]
    Client --> Types[types.rs<br/>serde structs for all API data]
    Client --> API[Anthropic API<br/>HTTPS + OAuth]
    Types -.-> Client
    Types -.-> Display
```

## Authentication (Cross-Platform)

```mermaid
flowchart TD
    Start[load_credentials] --> IsMac{macOS?}
    IsMac -->|Yes| Keychain[Keychain<br/>security find-generic-password<br/>-s 'Claude Code-credentials' -w]
    Keychain -->|Success| Parse1[Parse JSON → OAuthCredentials]
    Keychain -->|Fail| File
    IsMac -->|No| File[Credentials File]
    File --> EnvCheck{CLAUDE_CONFIG_DIR<br/>set?}
    EnvCheck -->|Yes| Custom["$CLAUDE_CONFIG_DIR/.credentials.json"]
    EnvCheck -->|No| Default["~/.claude/.credentials.json"]
    Custom --> Parse2[Parse JSON → OAuthCredentials]
    Default --> Parse2
    Parse1 --> FetchOrg[GET /api/oauth/profile<br/>→ organization.uuid]
    Parse2 --> FetchOrg
    FetchOrg --> ApiClient[ApiClient<br/>access_token + org_uuid]
    ApiClient --> Headers[All requests include:<br/>Authorization · x-organization-uuid<br/>anthropic-beta · anthropic-version]
```

| Platform | Primary | Fallback |
|----------|---------|----------|
| macOS | Keychain via `security` CLI | `~/.claude/.credentials.json` |
| Linux | `~/.claude/.credentials.json` | -- |
| Windows | `%USERPROFILE%\.claude\.credentials.json` | -- |

## Command Data Flow

```mermaid
flowchart LR
    Input[CLI args] --> Parse[clap parsing]
    Parse --> Validate[validate_session_id]
    Validate --> Auth[ApiClient::new<br/>auth + org UUID]
    Auth --> Fetch[API call<br/>sessions / events / loglines]
    Fetch --> Filter[Filter & transform<br/>date · status · text search]
    Filter --> Output[Terminal output<br/>colored formatting]
```

Six subcommands: `list`, `show`, `read`, `summary`, `loglines`, `export`.

## API Endpoints

| Method | Endpoint | Pagination | Notes |
|--------|----------|------------|-------|
| `list_sessions()` | `GET /v1/sessions` | No | Returns all sessions |
| `get_session(id)` | `GET /v1/sessions/{id}` | No | Single session metadata |
| `get_events(id, max)` | `GET /v1/sessions/{id}/events` | Yes (cursor-based) | 1000 events/page, `?after_id=<last_id>` |
| `get_loglines(id)` | `GET /v1/session_ingress/session/{id}` | No | Compact transcript |

## Event Pagination

```mermaid
flowchart TD
    Start[get_events] --> Fetch[GET /v1/sessions/{id}/events<br/>?after_id=cursor]
    Fetch --> Append[Append page to Vec]
    Append --> CheckMax{max_events<br/>reached?}
    CheckMax -->|Yes| Truncate[Truncate & return]
    CheckMax -->|No| CheckMore{has_more?}
    CheckMore -->|Yes| UpdateCursor[cursor = last_id] --> Fetch
    CheckMore -->|No| Return[Return all events]
```

Sessions can have 10,000+ events (1000 per page).

## Event Type Hierarchy

```mermaid
classDiagram
    class SessionEvent {
        <<enum>>
        System
        User
        Assistant
        ToolUseSummary
        ToolProgress
        Result
        ControlResponse
        EnvManagerLog
        Unknown
    }

    class UserContent {
        <<enum>>
        Text: String
        Blocks: Vec~Value~
    }

    class ContentBlock {
        <<enum>>
        Thinking
        Text
        ToolUse
        ToolResult
        Other
    }

    SessionEvent --> UserEvent : User
    SessionEvent --> AssistantEvent : Assistant
    UserEvent --> UserMessage
    UserMessage --> UserContent
    AssistantEvent --> AssistantMessage
    AssistantMessage --> ContentBlock : Vec
```

`SessionEvent` uses `#[serde(tag = "type")]` for internally-tagged deserialization. `Unknown` with `#[serde(other)]` catches any future event types. `UserContent` uses `#[serde(untagged)]` since content can be a plain string or block array with no discriminator.

## Key Design Decisions

1. **Strongly-typed events over `serde_json::Value`** -- catches deserialization mismatches at compile/test time.
2. **`#[serde(other)]` for forward compatibility** -- new event types deserialize as `Unknown` instead of crashing.
3. **Cursor-based auto-pagination** -- the client transparently fetches all pages with a progress indicator on stderr.
4. **Cross-platform auth with macOS Keychain priority** -- Keychain first on macOS, file-based fallback everywhere.
5. **Client-side filtering** -- the API doesn't offer server-side filtering, so we fetch then filter locally.
