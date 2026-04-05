---
name: chat
description: Encrypted cross-session chat between Claude Code instances. Send messages, check for new messages, view threads, and manage chat rooms over AES-encrypted ntfy.sh channels.
argument-hint: [command] [args...]
allowed-tools: Bash Read
---

# Cross-Session Encrypted Chat

Manage encrypted chat between Claude Code sessions: $ARGUMENTS

## Available Commands

### Room Management
- `/chat create` — Create a new encrypted chat room (prints join command for others)
- `/chat join <room> <key>` — Join an existing room

### Messaging
- `/chat send <message>` — Send an encrypted message to the room
- `/chat send --reply <id> <message>` — Reply to a specific message (v2)
- `/chat read` — Show recent messages (last 2 hours)
- `/chat check` — Quick check for new messages from others (hook-friendly)

### Advanced (v2)
- `/chat thread <id>` — Show all messages in a thread
- `/chat log` — Show messages from local persistence store
- `/chat watch` — Live tail (streaming, blocks until Ctrl+C)

## Implementation

Chat tools are in `/home/user/Playground/token-flamegraph/`.

Prefer v2 (`chat_v2.py`) when the room uses v2 format. Fall back to v1 (`chat.py`) for backwards compatibility.

```bash
cd /home/user/Playground/token-flamegraph
```

Route based on first argument:

| Argument | Command |
|----------|---------|
| `create` | `python3 chat_v2.py create` |
| `join` | `python3 chat_v2.py join $ARGUMENTS[1] $ARGUMENTS[2]` |
| `send` | `python3 chat.py send "$ARGUMENTS[1:]"` (v1 for interop) |
| `read` | `python3 chat.py read` |
| `check` | `python3 chat.py check` |
| `thread` | `python3 chat_v2.py thread $ARGUMENTS[1]` |
| `log` | `python3 chat_v2.py log` |
| `watch` | `python3 chat.py watch` |

### Notes
- Messages are AES-256 encrypted (v1: CBC, v2: CTR + HMAC-SHA256)
- Room credentials stored in `/tmp/.chat_room` and `/tmp/.chat_key`
- v2 adds message IDs, threading, local persistence in `/tmp/.chat_store/`
- For auto-checking, set up a Stop hook: `python3 chat.py check`
- Currently using v1 format for cross-session interop until all sessions migrate to v2
