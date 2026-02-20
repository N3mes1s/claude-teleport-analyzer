mod client;
mod display;
mod types;

use anyhow::{Context, Result, bail};
use chrono::{DateTime, Utc};
use clap::{Parser, Subcommand};
use colored::Colorize;
use std::collections::HashMap;

use client::{ApiClient, validate_session_id};
use display::*;
use types::*;

// ── CLI ──────────────────────────────────────────────────────────────

#[derive(Parser)]
#[command(name = "claude-teleport-analyzer")]
#[command(about = "Read Claude Code remote sessions without cloning")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// List all remote sessions
    List {
        /// Max number of sessions to show
        #[arg(short, long, default_value = "20")]
        limit: usize,
        /// Filter by status: running, idle, completed
        #[arg(short, long)]
        status: Option<String>,
        /// Only show sessions created after this date (YYYY-MM-DD or ISO8601)
        #[arg(long)]
        after: Option<String>,
        /// Only show sessions created before this date (YYYY-MM-DD or ISO8601)
        #[arg(long)]
        before: Option<String>,
    },
    /// Show session metadata
    Show {
        /// Session ID (e.g. session_01QJaJSUgfY6khmFTzJaMqph)
        session_id: String,
    },
    /// Read the full conversation transcript of a session
    Read {
        /// Session ID
        session_id: String,
        /// Only show user and assistant messages (skip tool_progress, env_manager_log, etc.)
        #[arg(short, long)]
        conversation_only: bool,
        /// Filter by event type (user, assistant, system, tool_use_summary, etc.)
        #[arg(short, long)]
        r#type: Option<String>,
        /// Maximum number of events to fetch (0 = all)
        #[arg(short, long, default_value = "0")]
        max_events: usize,
        /// Search for text in event content (case-insensitive)
        #[arg(short, long)]
        search: Option<String>,
    },
    /// Show a compact summary of a session's conversation
    Summary {
        /// Session ID
        session_id: String,
    },
    /// Show loglines from the session_ingress endpoint
    Loglines {
        /// Session ID
        session_id: String,
    },
    /// Export session events to a JSON file
    Export {
        /// Session ID
        session_id: String,
        /// Output file path
        #[arg(short, long, default_value = "session_export.json")]
        output: String,
    },
}

// ── Helpers ──────────────────────────────────────────────────────────

fn parse_date_filter(s: &str) -> Result<DateTime<Utc>> {
    // Try full ISO8601 first
    if let Ok(dt) = s.parse::<DateTime<Utc>>() {
        return Ok(dt);
    }
    // Try YYYY-MM-DD
    if let Ok(dt) = chrono::NaiveDate::parse_from_str(s, "%Y-%m-%d") {
        return Ok(dt.and_hms_opt(0, 0, 0).unwrap().and_utc());
    }
    bail!("Invalid date format: '{s}'. Use YYYY-MM-DD or ISO8601 (e.g. 2025-01-15T00:00:00Z)")
}

fn event_contains_text(event: &SessionEvent, needle: &str) -> bool {
    let needle_lower = needle.to_lowercase();
    match event {
        SessionEvent::User(e) => e
            .message
            .content
            .as_text()
            .is_some_and(|t| t.to_lowercase().contains(&needle_lower)),
        SessionEvent::Assistant(e) => e.message.content.iter().any(|block| match block {
            ContentBlock::Text(t) => t
                .text
                .as_deref()
                .is_some_and(|s| s.to_lowercase().contains(&needle_lower)),
            ContentBlock::Thinking(t) => t
                .thinking
                .as_deref()
                .is_some_and(|s| s.to_lowercase().contains(&needle_lower)),
            ContentBlock::ToolUse(t) => {
                t.name
                    .as_deref()
                    .is_some_and(|s| s.to_lowercase().contains(&needle_lower))
                    || t.input.as_ref().is_some_and(|v| {
                        serde_json::to_string(v)
                            .unwrap_or_default()
                            .to_lowercase()
                            .contains(&needle_lower)
                    })
            }
            ContentBlock::ToolResult(t) => t.content.as_ref().is_some_and(|v| {
                serde_json::to_string(v)
                    .unwrap_or_default()
                    .to_lowercase()
                    .contains(&needle_lower)
            }),
            ContentBlock::Other => false,
        }),
        SessionEvent::ToolUseSummary(e) => e
            .summary
            .as_deref()
            .is_some_and(|s| s.to_lowercase().contains(&needle_lower)),
        SessionEvent::EnvManagerLog(e) => e
            .data
            .as_ref()
            .and_then(|d| d.content.as_deref())
            .is_some_and(|s| s.to_lowercase().contains(&needle_lower)),
        SessionEvent::System(e) => e
            .subtype
            .as_deref()
            .is_some_and(|s| s.to_lowercase().contains(&needle_lower)),
        _ => false,
    }
}

// ── Commands ─────────────────────────────────────────────────────────

async fn cmd_list(
    limit: usize,
    status_filter: Option<String>,
    after: Option<String>,
    before: Option<String>,
) -> Result<()> {
    let after_dt = after.as_deref().map(parse_date_filter).transpose()?;
    let before_dt = before.as_deref().map(parse_date_filter).transpose()?;

    let api = ApiClient::new().await?;
    let sessions = api.list_sessions().await?;

    let filtered: Vec<&Session> = sessions
        .iter()
        .filter(|s| {
            if let Some(ref f) = status_filter
                && s.session_status.as_deref() != Some(f.as_str())
            {
                return false;
            }
            let created_dt = s
                .created_at
                .as_deref()
                .and_then(|c| c.parse::<DateTime<Utc>>().ok());
            if let (Some(after), Some(dt)) = (&after_dt, &created_dt)
                && dt < after
            {
                return false;
            }
            if let (Some(before), Some(dt)) = (&before_dt, &created_dt)
                && dt > before
            {
                return false;
            }
            true
        })
        .take(limit)
        .collect();

    println!(
        "\n{} ({} total, showing {})\n",
        "Remote Sessions".bold(),
        sessions.len(),
        filtered.len()
    );

    for s in &filtered {
        print_session_row(s);
    }

    Ok(())
}

async fn cmd_show(session_id: &str) -> Result<()> {
    validate_session_id(session_id)?;
    let api = ApiClient::new().await?;
    let session = api.get_session(session_id).await?;
    print_session_detail(&session);
    Ok(())
}

async fn cmd_read(
    session_id: &str,
    conversation_only: bool,
    type_filter: Option<String>,
    max_events: usize,
    search: Option<String>,
) -> Result<()> {
    validate_session_id(session_id)?;
    let api = ApiClient::new().await?;

    eprintln!("Fetching session events...");
    let events = api.get_events(session_id, max_events).await?;

    let filtered: Vec<&SessionEvent> = events
        .iter()
        .filter(|e| {
            if let Some(ref tf) = type_filter
                && e.event_type() != tf.as_str()
            {
                return false;
            }
            if conversation_only && !e.is_conversation() {
                return false;
            }
            if let Some(ref needle) = search
                && !event_contains_text(e, needle)
            {
                return false;
            }
            true
        })
        .collect();

    let mut label_parts = vec![format!("{} events", filtered.len())];
    if conversation_only {
        label_parts.push("conversation only".to_string());
    }
    if let Some(ref s) = search {
        label_parts.push(format!("search: \"{s}\""));
    }

    println!(
        "\n{} ({})\n",
        "Session Transcript".bold(),
        label_parts.join(" - ").cyan()
    );

    for event in &filtered {
        print_event(event);
    }

    Ok(())
}

async fn cmd_summary(session_id: &str) -> Result<()> {
    validate_session_id(session_id)?;
    let api = ApiClient::new().await?;
    let session = api.get_session(session_id).await?;

    println!("\n{}\n", "Session Summary".bold());
    println!(
        "  {} ({})",
        session.title.as_deref().unwrap_or("(untitled)").bold(),
        status_colored(session.session_status.as_deref().unwrap_or("unknown"))
    );
    println!();

    eprintln!("Fetching events...");
    let events = api.get_events(session_id, 0).await?;

    let mut type_counts: HashMap<&str, usize> = HashMap::new();
    for e in &events {
        *type_counts.entry(e.event_type()).or_default() += 1;
    }

    println!("  {}: {}", "Total events".dimmed(), events.len());
    let mut sorted_types: Vec<_> = type_counts.iter().collect();
    sorted_types.sort_by(|a, b| b.1.cmp(a.1));
    for (t, c) in &sorted_types {
        println!("    {}: {c}", t.dimmed());
    }
    println!();

    // Tool-use summaries
    let summaries: Vec<&str> = events
        .iter()
        .filter_map(|e| match e {
            SessionEvent::ToolUseSummary(s) => s.summary.as_deref(),
            _ => None,
        })
        .collect();

    if !summaries.is_empty() {
        println!("  {} ({}):", "Tool Use Summaries".bold(), summaries.len());
        for (i, s) in summaries.iter().enumerate() {
            let prefix = if i == summaries.len() - 1 {
                "  \u{2514}\u{2500}"
            } else {
                "  \u{251c}\u{2500}"
            };
            println!("  {prefix} {s}");
        }
    }

    // User messages
    let user_messages: Vec<&str> = events
        .iter()
        .filter_map(|e| match e {
            SessionEvent::User(u) => u.message.content.as_text(),
            _ => None,
        })
        .collect();

    if !user_messages.is_empty() {
        println!("\n  {} ({}):", "User Messages".bold(), user_messages.len());
        for (i, msg) in user_messages.iter().enumerate() {
            let preview: String = msg.chars().take(120).collect();
            let suffix = if msg.len() > 120 { "..." } else { "" };
            let prefix = if i == user_messages.len() - 1 {
                "  \u{2514}\u{2500}"
            } else {
                "  \u{251c}\u{2500}"
            };
            println!("  {prefix} {preview}{suffix}");
        }
    }

    println!();
    Ok(())
}

async fn cmd_loglines(session_id: &str) -> Result<()> {
    validate_session_id(session_id)?;
    let api = ApiClient::new().await?;

    eprintln!("Fetching session loglines...");
    let loglines = api.get_loglines(session_id).await?;

    println!(
        "\n{} ({} loglines)\n",
        "Session Loglines".bold(),
        loglines.len()
    );

    for log in &loglines {
        print_logline(log);
    }

    Ok(())
}

async fn cmd_export(session_id: &str, output: &str) -> Result<()> {
    validate_session_id(session_id)?;

    // Validate output path
    let path = std::path::Path::new(output);
    if let Some(parent) = path.parent()
        && !parent.as_os_str().is_empty()
        && !parent.exists()
    {
        bail!("Output directory does not exist: {}", parent.display());
    }

    let api = ApiClient::new().await?;

    eprintln!("Fetching session metadata...");
    let session = api.get_session(session_id).await?;

    eprintln!("Fetching all events...");
    let events = api.get_events(session_id, 0).await?;

    let export = serde_json::json!({
        "session": session,
        "events": events,
        "exported_at": Utc::now().to_rfc3339(),
        "total_events": events.len(),
    });

    std::fs::write(output, serde_json::to_string_pretty(&export)?)
        .with_context(|| format!("Failed to write export to {output}"))?;
    println!(
        "\nExported {} events to {}\n",
        events.len().to_string().cyan(),
        output.green()
    );

    Ok(())
}

// ── Main ─────────────────────────────────────────────────────────────

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::List {
            limit,
            status,
            after,
            before,
        } => cmd_list(limit, status, after, before).await,
        Commands::Show { session_id } => cmd_show(&session_id).await,
        Commands::Read {
            session_id,
            conversation_only,
            r#type,
            max_events,
            search,
        } => cmd_read(&session_id, conversation_only, r#type, max_events, search).await,
        Commands::Summary { session_id } => cmd_summary(&session_id).await,
        Commands::Loglines { session_id } => cmd_loglines(&session_id).await,
        Commands::Export { session_id, output } => cmd_export(&session_id, &output).await,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── parse_date_filter ───────────────────────────────────────────

    #[test]
    fn parse_date_filter_iso8601() {
        let dt = parse_date_filter("2025-06-15T14:30:00Z").unwrap();
        assert_eq!(dt.format("%Y-%m-%d").to_string(), "2025-06-15");
    }

    #[test]
    fn parse_date_filter_date_only() {
        let dt = parse_date_filter("2025-06-15").unwrap();
        assert_eq!(
            dt.format("%Y-%m-%d %H:%M:%S").to_string(),
            "2025-06-15 00:00:00"
        );
    }

    #[test]
    fn parse_date_filter_invalid() {
        let err = parse_date_filter("not-a-date").unwrap_err();
        assert!(err.to_string().contains("Invalid date format"));
    }

    // ── event_contains_text ─────────────────────────────────────────

    fn make_user_event(text: &str) -> SessionEvent {
        SessionEvent::User(UserEvent {
            created_at: None,
            uuid: None,
            session_id: None,
            message: UserMessage {
                role: None,
                content: UserContent::Text(text.to_string()),
            },
            parent_tool_use_id: None,
            is_replay: None,
        })
    }

    fn make_assistant_event(text: &str) -> SessionEvent {
        SessionEvent::Assistant(AssistantEvent {
            created_at: None,
            uuid: None,
            session_id: None,
            message: AssistantMessage {
                role: None,
                content: vec![ContentBlock::Text(TextBlock {
                    text: Some(text.to_string()),
                })],
            },
        })
    }

    fn make_summary_event(summary: &str) -> SessionEvent {
        SessionEvent::ToolUseSummary(ToolUseSummaryEvent {
            created_at: None,
            uuid: None,
            session_id: None,
            summary: Some(summary.to_string()),
            preceding_tool_use_ids: None,
        })
    }

    #[test]
    fn search_user_event_matches() {
        let event = make_user_event("Hello World");
        assert!(event_contains_text(&event, "hello"));
        assert!(event_contains_text(&event, "WORLD"));
        assert!(!event_contains_text(&event, "missing"));
    }

    #[test]
    fn search_assistant_event_matches() {
        let event = make_assistant_event("Here is the answer");
        assert!(event_contains_text(&event, "answer"));
        assert!(!event_contains_text(&event, "question"));
    }

    #[test]
    fn search_tool_summary_matches() {
        let event = make_summary_event("Read 3 files and edited main.rs");
        assert!(event_contains_text(&event, "main.rs"));
        assert!(!event_contains_text(&event, "cargo.toml"));
    }

    #[test]
    fn search_case_insensitive() {
        let event = make_user_event("CamelCase mixed TEXT");
        assert!(event_contains_text(&event, "camelcase"));
        assert!(event_contains_text(&event, "MIXED"));
        assert!(event_contains_text(&event, "text"));
    }

    #[test]
    fn search_unknown_event_returns_false() {
        assert!(!event_contains_text(&SessionEvent::Unknown, "anything"));
    }

    #[test]
    fn search_tool_use_by_name() {
        let event = SessionEvent::Assistant(AssistantEvent {
            created_at: None,
            uuid: None,
            session_id: None,
            message: AssistantMessage {
                role: None,
                content: vec![ContentBlock::ToolUse(ToolUseBlock {
                    id: None,
                    name: Some("Bash".to_string()),
                    input: Some(serde_json::json!({"command": "cargo test"})),
                })],
            },
        });
        assert!(event_contains_text(&event, "Bash"));
        assert!(event_contains_text(&event, "cargo test"));
        assert!(!event_contains_text(&event, "npm"));
    }

    #[test]
    fn search_env_manager_log() {
        let event = SessionEvent::EnvManagerLog(EnvManagerLogEvent {
            created_at: None,
            uuid: None,
            data: Some(EnvManagerLogData {
                category: None,
                content: Some("Installing dependencies...".to_string()),
                level: None,
                timestamp: None,
                extra: None,
            }),
        });
        assert!(event_contains_text(&event, "dependencies"));
        assert!(!event_contains_text(&event, "compiling"));
    }
}
