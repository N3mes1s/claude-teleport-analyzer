use chrono::{DateTime, Utc};
use colored::Colorize;

use crate::types::*;

/// Truncate a string to at most `max_chars` characters, appending "..." if truncated.
fn truncate_str(s: &str, max_chars: usize) -> String {
    if s.chars().count() <= max_chars {
        s.to_string()
    } else {
        let preview: String = s.chars().take(max_chars).collect();
        format!("{preview}...")
    }
}

pub fn format_timestamp(ts: &str) -> String {
    if let Ok(dt) = ts.parse::<DateTime<Utc>>() {
        dt.format("%Y-%m-%d %H:%M:%S UTC").to_string()
    } else {
        ts.to_string()
    }
}

pub fn status_colored(status: &str) -> String {
    match status {
        "running" => status.green().bold().to_string(),
        "idle" => status.yellow().to_string(),
        "completed" => status.blue().to_string(),
        "error" | "failed" => status.red().bold().to_string(),
        _ => status.dimmed().to_string(),
    }
}

pub fn print_session_row(s: &Session) {
    let title = s.title.as_deref().unwrap_or("(untitled)");
    let status = s.session_status.as_deref().unwrap_or("unknown");
    let updated = s
        .updated_at
        .as_deref()
        .map(format_timestamp)
        .unwrap_or_default();
    let repo = s
        .session_context
        .as_ref()
        .and_then(|c| c.sources.as_ref())
        .and_then(|s| s.first())
        .and_then(|s| s.url.as_deref())
        .unwrap_or("");

    println!(
        "  {} {} {}",
        status_colored(status),
        s.id.dimmed(),
        updated.dimmed()
    );
    println!("    {}", title.bold());
    if !repo.is_empty() {
        println!("    {}", repo.dimmed());
    }
    println!();
}

pub fn print_session_detail(session: &Session) {
    println!("\n{}\n", "Session Details".bold());
    println!("  {}: {}", "ID".dimmed(), session.id);
    println!(
        "  {}: {}",
        "Title".dimmed(),
        session.title.as_deref().unwrap_or("(untitled)").bold()
    );
    println!(
        "  {}: {}",
        "Status".dimmed(),
        status_colored(session.session_status.as_deref().unwrap_or("unknown"))
    );
    println!(
        "  {}: {}",
        "Type".dimmed(),
        session.session_type.as_deref().unwrap_or("unknown")
    );
    println!(
        "  {}: {}",
        "Created".dimmed(),
        session
            .created_at
            .as_deref()
            .map(format_timestamp)
            .unwrap_or_default()
    );
    println!(
        "  {}: {}",
        "Updated".dimmed(),
        session
            .updated_at
            .as_deref()
            .map(format_timestamp)
            .unwrap_or_default()
    );

    if let Some(ref ctx) = session.session_context {
        println!(
            "  {}: {}",
            "Model".dimmed(),
            ctx.model.as_deref().unwrap_or("unknown").cyan()
        );

        if let Some(ref sources) = ctx.sources {
            for src in sources {
                println!(
                    "  {}: {} ({})",
                    "Source".dimmed(),
                    src.url.as_deref().unwrap_or(""),
                    src.revision.as_deref().unwrap_or("")
                );
            }
        }

        if let Some(ref outcomes) = ctx.outcomes {
            for out in outcomes {
                if let Some(ref git) = out.git_info {
                    println!(
                        "  {}: {}",
                        "Repo".dimmed(),
                        git.repo.as_deref().unwrap_or("")
                    );
                    if let Some(ref branches) = git.branches {
                        for b in branches {
                            println!("  {}: {}", "Branch".dimmed(), b.green());
                        }
                    }
                }
            }
        }
    }

    println!(
        "\n  {} claude --teleport {}\n",
        "Resume with:".dimmed(),
        session.id.cyan()
    );
}

pub fn print_event(event: &SessionEvent) {
    let created = event.created_at().map(format_timestamp).unwrap_or_default();

    match event {
        SessionEvent::System(e) => {
            let subtype = e.subtype.as_deref().unwrap_or("");
            let model = e.model.as_deref().unwrap_or("");
            let cwd = e.cwd.as_deref().unwrap_or("");
            println!(
                "{} {} [{}] model={} cwd={}",
                created.dimmed(),
                "SYSTEM".magenta().bold(),
                subtype,
                model.cyan(),
                cwd
            );
        }

        SessionEvent::User(e) => {
            let content = e.message.content.as_text().unwrap_or("");
            println!("{} {}", created.dimmed(), "USER".green().bold());
            for line in content.lines() {
                println!("  {line}");
            }
            println!();
        }

        SessionEvent::Assistant(e) => {
            println!("{} {}", created.dimmed(), "ASSISTANT".blue().bold());
            for block in &e.message.content {
                print_content_block(block);
            }
            println!();
        }

        SessionEvent::ToolUseSummary(e) => {
            let summary = e.summary.as_deref().unwrap_or("");
            println!("{} {} {}", created.dimmed(), "SUMMARY".yellow(), summary);
        }

        SessionEvent::ToolProgress(e) => {
            let tool = e.tool_name.as_deref().unwrap_or("");
            let elapsed = e.elapsed_time_seconds.unwrap_or(0);
            println!(
                "{} {} {} ({}s)",
                created.dimmed(),
                "PROGRESS".dimmed(),
                tool.dimmed(),
                elapsed,
            );
        }

        SessionEvent::Result(e) => {
            let duration_s = e.duration_ms.unwrap_or(0) / 1000;
            println!(
                "{} {} duration={}s",
                created.dimmed(),
                "RESULT".cyan().bold(),
                duration_s,
            );
        }

        SessionEvent::ControlResponse(e) => {
            let subtype = e
                .response
                .as_ref()
                .and_then(|r| r.subtype.as_deref())
                .unwrap_or("");
            println!(
                "{} {} [{}]",
                created.dimmed(),
                "CONTROL".dimmed(),
                subtype.dimmed()
            );
        }

        SessionEvent::EnvManagerLog(e) => {
            let (content, level) = match &e.data {
                Some(d) => (
                    d.content.as_deref().unwrap_or(""),
                    d.level.as_deref().unwrap_or("info"),
                ),
                None => ("", "info"),
            };
            let level_colored = match level {
                "error" => level.red().to_string(),
                "warn" => level.yellow().to_string(),
                "debug" => level.dimmed().to_string(),
                _ => level.to_string(),
            };
            println!(
                "{} {} [{}] {}",
                created.dimmed(),
                "ENV".dimmed(),
                level_colored,
                content
            );
        }

        SessionEvent::Unknown => {
            println!("{} {}", created.dimmed(), "UNKNOWN".dimmed());
        }
    }
}

fn print_content_block(block: &ContentBlock) {
    match block {
        ContentBlock::Thinking(b) => {
            if let Some(ref text) = b.thinking
                && !text.is_empty()
            {
                let preview = truncate_str(text, 200);
                println!("  {} {}", "thinking:".dimmed(), preview.dimmed());
            }
        }
        ContentBlock::Text(b) => {
            let text = b.text.as_deref().unwrap_or("");
            for line in text.lines() {
                println!("  {line}");
            }
        }
        ContentBlock::ToolUse(b) => {
            let tool = b.name.as_deref().unwrap_or("unknown");
            let input_preview = b
                .input
                .as_ref()
                .map(|v| {
                    let s = serde_json::to_string(v).unwrap_or_default();
                    truncate_str(&s, 120)
                })
                .unwrap_or_default();
            println!(
                "  {} {} {}",
                "tool_use:".yellow(),
                tool.cyan().bold(),
                input_preview.dimmed()
            );
        }
        ContentBlock::ToolResult(b) => {
            let preview = b
                .content
                .as_ref()
                .map(|v| {
                    let s = serde_json::to_string(v).unwrap_or_default();
                    truncate_str(&s, 200)
                })
                .unwrap_or_default();
            println!("  {} {}", "tool_result:".yellow(), preview.dimmed());
        }
        ContentBlock::Other => {}
    }
}

#[cfg(test)]
fn format_content_block(block: &ContentBlock) -> String {
    let mut lines = Vec::new();
    match block {
        ContentBlock::Thinking(b) => {
            if let Some(ref text) = b.thinking
                && !text.is_empty()
            {
                let preview = truncate_str(text, 200);
                lines.push(format!("  thinking: {preview}"));
            }
        }
        ContentBlock::Text(b) => {
            let text = b.text.as_deref().unwrap_or("");
            for line in text.lines() {
                lines.push(format!("  {line}"));
            }
        }
        ContentBlock::ToolUse(b) => {
            let tool = b.name.as_deref().unwrap_or("unknown");
            let input_preview = b
                .input
                .as_ref()
                .map(|v| {
                    let s = serde_json::to_string(v).unwrap_or_default();
                    truncate_str(&s, 120)
                })
                .unwrap_or_default();
            lines.push(format!("  tool_use: {tool} {input_preview}"));
        }
        ContentBlock::ToolResult(b) => {
            let preview = b
                .content
                .as_ref()
                .map(|v| {
                    let s = serde_json::to_string(v).unwrap_or_default();
                    truncate_str(&s, 200)
                })
                .unwrap_or_default();
            lines.push(format!("  tool_result: {preview}"));
        }
        ContentBlock::Other => {}
    }
    lines.join("\n")
}

pub fn print_logline(log: &Logline) {
    let log_type = log.log_type.as_deref().unwrap_or("unknown");
    let subtype = log.subtype.as_deref().unwrap_or("");
    let content = log.content.as_deref().unwrap_or("");
    let timestamp = log
        .timestamp
        .as_deref()
        .map(format_timestamp)
        .unwrap_or_default();
    let branch = log.git_branch.as_deref().unwrap_or("");

    let type_display = if subtype.is_empty() {
        log_type.to_string()
    } else {
        format!("{log_type}/{subtype}")
    };

    let type_colored = match log_type {
        "system" => type_display.magenta().to_string(),
        "user" => type_display.green().to_string(),
        "assistant" => type_display.blue().to_string(),
        _ => type_display.dimmed().to_string(),
    };

    println!(
        "{} {} {}",
        timestamp.dimmed(),
        type_colored,
        branch.dimmed()
    );
    if !content.is_empty() {
        let preview: String = content.chars().take(200).collect();
        println!("  {preview}");
    }
    println!();
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    // ── truncate_str ────────────────────────────────────────────────

    #[test]
    fn truncate_str_ascii_no_truncation() {
        assert_eq!(truncate_str("hello", 10), "hello");
    }

    #[test]
    fn truncate_str_ascii_exact_boundary() {
        assert_eq!(truncate_str("hello", 5), "hello");
    }

    #[test]
    fn truncate_str_ascii_truncates() {
        assert_eq!(truncate_str("hello world", 5), "hello...");
    }

    #[test]
    fn truncate_str_unicode_box_drawing_no_panic() {
        // Regression test for issue #1: box-drawing char ─ (U+2500) is 3 bytes in UTF-8.
        // A naive &s[..120] would panic if index 120 lands inside this char.
        let s = format!(
            "{}{}",
            "a".repeat(119),
            "─────" // each ─ is 3 bytes (0xE2 0x94 0x80)
        );
        // Byte length: 119 + 15 = 134, char count: 119 + 5 = 124
        let result = truncate_str(&s, 120);
        assert!(result.ends_with("..."));
        // Should contain exactly 120 chars + "..."
        assert_eq!(result.chars().count(), 123); // 120 + 3 for "..."
    }

    #[test]
    fn truncate_str_multibyte_emoji() {
        // Emoji like 🦀 is 4 bytes in UTF-8
        let s = format!("{}{}", "x".repeat(118), "🦀🦀🦀");
        let result = truncate_str(&s, 120);
        assert!(result.ends_with("..."));
        assert_eq!(result.chars().count(), 123);
    }

    #[test]
    fn truncate_str_empty() {
        assert_eq!(truncate_str("", 10), "");
    }

    // ── format_timestamp ────────────────────────────────────────────

    #[test]
    fn format_timestamp_valid_utc() {
        let result = format_timestamp("2025-01-15T14:30:00Z");
        assert_eq!(result, "2025-01-15 14:30:00 UTC");
    }

    #[test]
    fn format_timestamp_with_millis() {
        let result = format_timestamp("2025-06-01T08:00:00.123Z");
        assert_eq!(result, "2025-06-01 08:00:00 UTC");
    }

    #[test]
    fn format_timestamp_invalid_returns_original() {
        let result = format_timestamp("not-a-timestamp");
        assert_eq!(result, "not-a-timestamp");
    }

    #[test]
    fn format_timestamp_empty() {
        let result = format_timestamp("");
        assert_eq!(result, "");
    }

    // ── status_colored ──────────────────────────────────────────────
    // We test that the function doesn't panic and returns non-empty strings.
    // Exact ANSI codes depend on terminal, so we just check content.

    #[test]
    fn status_colored_running() {
        let result = status_colored("running");
        assert!(result.contains("running"));
    }

    #[test]
    fn status_colored_idle() {
        let result = status_colored("idle");
        assert!(result.contains("idle"));
    }

    #[test]
    fn status_colored_completed() {
        let result = status_colored("completed");
        assert!(result.contains("completed"));
    }

    #[test]
    fn status_colored_error() {
        let result = status_colored("error");
        assert!(result.contains("error"));
    }

    #[test]
    fn status_colored_failed() {
        let result = status_colored("failed");
        assert!(result.contains("failed"));
    }

    #[test]
    fn status_colored_unknown() {
        let result = status_colored("something_else");
        assert!(result.contains("something_else"));
    }

    // ── format_content_block ────────────────────────────────────────

    #[test]
    fn format_thinking_block() {
        let block = ContentBlock::Thinking(ThinkingBlock {
            thinking: Some("I need to think about this...".to_string()),
            signature: None,
        });
        let output = format_content_block(&block);
        assert!(output.contains("thinking:"));
        assert!(output.contains("I need to think about this..."));
    }

    #[test]
    fn format_thinking_block_empty() {
        let block = ContentBlock::Thinking(ThinkingBlock {
            thinking: Some("".to_string()),
            signature: None,
        });
        let output = format_content_block(&block);
        assert!(output.is_empty());
    }

    #[test]
    fn format_thinking_block_none() {
        let block = ContentBlock::Thinking(ThinkingBlock {
            thinking: None,
            signature: None,
        });
        let output = format_content_block(&block);
        assert!(output.is_empty());
    }

    #[test]
    fn format_thinking_block_long_truncates() {
        let long_text = "a".repeat(300);
        let block = ContentBlock::Thinking(ThinkingBlock {
            thinking: Some(long_text),
            signature: None,
        });
        let output = format_content_block(&block);
        assert!(output.contains("..."));
        // The preview is 200 chars + prefix + suffix
        assert!(output.len() < 300);
    }

    #[test]
    fn format_text_block() {
        let block = ContentBlock::Text(TextBlock {
            text: Some("Hello\nWorld".to_string()),
        });
        let output = format_content_block(&block);
        assert!(output.contains("Hello"));
        assert!(output.contains("World"));
    }

    #[test]
    fn format_text_block_empty() {
        let block = ContentBlock::Text(TextBlock { text: None });
        let output = format_content_block(&block);
        // Empty text still gets an empty line
        assert!(output.is_empty());
    }

    #[test]
    fn format_tool_use_block() {
        let block = ContentBlock::ToolUse(ToolUseBlock {
            id: Some("tu_1".to_string()),
            name: Some("Bash".to_string()),
            input: Some(json!({"command": "ls -la"})),
        });
        let output = format_content_block(&block);
        assert!(output.contains("tool_use:"));
        assert!(output.contains("Bash"));
        assert!(output.contains("ls -la"));
    }

    #[test]
    fn format_tool_use_block_long_input_truncates() {
        let long_input = "x".repeat(200);
        let block = ContentBlock::ToolUse(ToolUseBlock {
            id: None,
            name: Some("Write".to_string()),
            input: Some(json!({"content": long_input})),
        });
        let output = format_content_block(&block);
        assert!(output.contains("..."));
    }

    /// Regression test for issue #1: Unicode box-drawing characters in tool_use input
    /// must not cause a panic during truncation.
    #[test]
    fn format_tool_use_block_unicode_box_drawing_no_panic() {
        let content = format!(
            "    // ── ALLOWLIST DEFAULT: reject unknown/unclassified syscalls {}\n    //\n    // SECURITY: This is the critical allow",
            "─".repeat(20)
        );
        let block = ContentBlock::ToolUse(ToolUseBlock {
            id: Some("tu_1".to_string()),
            name: Some("Edit".to_string()),
            input: Some(json!({"file_path": "/test.rs", "new_string": content})),
        });
        // Must not panic
        let output = format_content_block(&block);
        assert!(output.contains("tool_use:"));
        assert!(output.contains("Edit"));
    }

    /// Ensure print_content_block (the actual println path) also doesn't panic with Unicode.
    #[test]
    fn print_tool_use_unicode_no_panic() {
        let content = format!("// {}", "─".repeat(100));
        let block = ContentBlock::ToolUse(ToolUseBlock {
            id: None,
            name: Some("Write".to_string()),
            input: Some(json!({"file_path": "/test.rs", "content": content})),
        });
        print_content_block(&block);
    }

    #[test]
    fn format_tool_result_block() {
        let block = ContentBlock::ToolResult(ToolResultBlock {
            tool_use_id: Some("tu_1".to_string()),
            content: Some(json!("result text")),
            is_error: Some(false),
        });
        let output = format_content_block(&block);
        assert!(output.contains("tool_result:"));
        assert!(output.contains("result text"));
    }

    #[test]
    fn format_tool_result_block_long_truncates() {
        let long_result = "y".repeat(300);
        let block = ContentBlock::ToolResult(ToolResultBlock {
            tool_use_id: None,
            content: Some(json!(long_result)),
            is_error: None,
        });
        let output = format_content_block(&block);
        assert!(output.contains("..."));
    }

    #[test]
    fn format_other_block() {
        let block = ContentBlock::Other;
        let output = format_content_block(&block);
        assert!(output.is_empty());
    }

    // ── print functions don't panic ─────────────────────────────────

    #[test]
    fn print_session_row_doesnt_panic() {
        let session = Session {
            id: "session_01test".to_string(),
            title: Some("Test".to_string()),
            session_status: Some("running".to_string()),
            session_type: None,
            created_at: None,
            updated_at: Some("2025-01-01T00:00:00Z".to_string()),
            environment_id: None,
            session_context: None,
            metadata: None,
            active_mount_paths: None,
        };
        print_session_row(&session);
    }

    #[test]
    fn print_session_row_minimal_doesnt_panic() {
        let session = Session {
            id: "s1".to_string(),
            title: None,
            session_status: None,
            session_type: None,
            created_at: None,
            updated_at: None,
            environment_id: None,
            session_context: None,
            metadata: None,
            active_mount_paths: None,
        };
        print_session_row(&session);
    }

    #[test]
    fn print_session_detail_doesnt_panic() {
        let session = Session {
            id: "session_01full".to_string(),
            title: Some("Full Session".to_string()),
            session_status: Some("completed".to_string()),
            session_type: Some("remote".to_string()),
            created_at: Some("2025-01-01T00:00:00Z".to_string()),
            updated_at: Some("2025-01-01T01:00:00Z".to_string()),
            environment_id: None,
            session_context: Some(SessionContext {
                model: Some("claude-sonnet-4-20250514".to_string()),
                cwd: Some("/tmp".to_string()),
                sources: Some(vec![SessionSource {
                    source_type: Some("git".to_string()),
                    url: Some("https://github.com/test/repo".to_string()),
                    revision: Some("abc123".to_string()),
                }]),
                outcomes: Some(vec![SessionOutcome {
                    outcome_type: Some("git".to_string()),
                    git_info: Some(GitInfo {
                        git_type: Some("push".to_string()),
                        repo: Some("test/repo".to_string()),
                        branches: Some(vec!["main".to_string()]),
                    }),
                }]),
                allowed_tools: None,
                disallowed_tools: None,
                knowledge_base_ids: None,
            }),
            metadata: None,
            active_mount_paths: None,
        };
        print_session_detail(&session);
    }

    #[test]
    fn print_event_all_variants_dont_panic() {
        let events: Vec<SessionEvent> = vec![
            SessionEvent::System(SystemEvent {
                created_at: Some("2025-01-01T00:00:00Z".to_string()),
                uuid: None,
                subtype: Some("init".to_string()),
                session_id: None,
                model: Some("opus".to_string()),
                cwd: Some("/tmp".to_string()),
                claude_code_version: None,
                tools: None,
                agents: None,
                skills: None,
                slash_commands: None,
                mcp_servers: None,
                permission_mode: None,
                fast_mode_state: None,
                output_style: None,
            }),
            SessionEvent::User(UserEvent {
                created_at: None,
                uuid: None,
                session_id: None,
                message: UserMessage {
                    role: Some("user".to_string()),
                    content: UserContent::Text("hello".to_string()),
                },
                parent_tool_use_id: None,
                is_replay: None,
            }),
            SessionEvent::Assistant(AssistantEvent {
                created_at: None,
                uuid: None,
                session_id: None,
                message: AssistantMessage {
                    role: Some("assistant".to_string()),
                    content: vec![ContentBlock::Text(TextBlock {
                        text: Some("response".to_string()),
                    })],
                },
            }),
            SessionEvent::ToolUseSummary(ToolUseSummaryEvent {
                created_at: None,
                uuid: None,
                session_id: None,
                summary: Some("did stuff".to_string()),
                preceding_tool_use_ids: None,
            }),
            SessionEvent::ToolProgress(ToolProgressEvent {
                created_at: None,
                uuid: None,
                session_id: None,
                tool_name: Some("Bash".to_string()),
                tool_use_id: None,
                parent_tool_use_id: None,
                elapsed_time_seconds: Some(3),
            }),
            SessionEvent::Result(ResultEvent {
                created_at: None,
                duration_ms: Some(10000),
                duration_api_ms: Some(8000),
                errors: None,
            }),
            SessionEvent::ControlResponse(ControlResponseEvent {
                created_at: None,
                response: Some(ControlResponseData {
                    subtype: Some("ack".to_string()),
                }),
            }),
            SessionEvent::EnvManagerLog(EnvManagerLogEvent {
                created_at: None,
                uuid: None,
                data: Some(EnvManagerLogData {
                    category: None,
                    content: Some("setup done".to_string()),
                    level: Some("error".to_string()),
                    timestamp: None,
                    extra: None,
                }),
            }),
            SessionEvent::Unknown,
        ];

        for event in &events {
            print_event(event);
        }
    }

    #[test]
    fn print_logline_doesnt_panic() {
        let log = Logline {
            log_type: Some("user".to_string()),
            subtype: Some("message".to_string()),
            content: Some("hello world".to_string()),
            timestamp: Some("2025-01-01T00:00:00Z".to_string()),
            git_branch: Some("main".to_string()),
            session_id: None,
            cwd: None,
            level: None,
            is_meta: None,
            is_sidechain: None,
            slug: None,
            compact_metadata: None,
            extra: serde_json::Map::new(),
        };
        print_logline(&log);
    }

    #[test]
    fn print_logline_minimal_doesnt_panic() {
        let log = Logline {
            log_type: None,
            subtype: None,
            content: None,
            timestamp: None,
            git_branch: None,
            session_id: None,
            cwd: None,
            level: None,
            is_meta: None,
            is_sidechain: None,
            slug: None,
            compact_metadata: None,
            extra: serde_json::Map::new(),
        };
        print_logline(&log);
    }
}
