---
name: flamegraph
description: Analyze token usage for Claude Code sessions. Generates flamegraph visualizations, efficiency scores, optimization rules, and session comparisons. Use when analyzing costs, finding waste, or optimizing CLAUDE.md rules.
argument-hint: [command] [args...]
allowed-tools: Bash Read Grep Glob
---

# Token Flamegraph Toolkit

Analyze Claude Code session token usage: $ARGUMENTS

## Available Commands

### Visualization & Analysis
- `/flamegraph` or `/flamegraph dashboard` — Show the full terminal dashboard (cli.py --self)
- `/flamegraph demo` — Show a demo flamegraph with sample data (cli.py --demo)
- `/flamegraph html [output]` — Generate interactive HTML flamegraph
- `/flamegraph score` — Show efficiency score for current session

### Optimization
- `/flamegraph optimize` — Run optimizer.py to generate CLAUDE.md rules from session waste patterns
- `/flamegraph rules` — Show existing optimization rules from CLAUDE.md

### Comparison
- `/flamegraph snapshot` — Save current session metrics as baseline
- `/flamegraph diff` — Compare current metrics against last snapshot
- `/flamegraph compare` — Run snapshot + diff in one shot

### Teleport Integration
- `/flamegraph teleport` — Analyze a remote session via claude-teleport-analyzer

## Implementation

All tools live in `/home/user/Playground/token-flamegraph/`.

```bash
cd /home/user/Playground/token-flamegraph
```

Route based on the first argument:

| Argument | Command |
|----------|---------|
| (none), `dashboard` | `python3 cli.py --self` |
| `demo` | `python3 cli.py --demo` |
| `html` | `python3 cli.py --html $ARGUMENTS[1]` |
| `optimize` | `python3 optimizer.py` |
| `snapshot` | `python3 compare.py snapshot` |
| `diff` | `python3 compare.py diff` |
| `compare` | `python3 compare.py snapshot && python3 compare.py diff` |
| `teleport` | `python3 cli.py --teleport` |
| `score` | `python3 -c "from cli import *; show_score()"` |

After running, summarize the key findings:
- Total cost and token breakdown
- Worst efficiency turns
- Top optimization opportunities
- Comparison deltas (if diffing)
