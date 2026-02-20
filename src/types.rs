use serde::{Deserialize, Serialize};

// ── OAuth / Auth ─────────────────────────────────────────────────────

#[derive(Deserialize)]
pub struct OAuthCredentials {
    #[serde(rename = "claudeAiOauth")]
    pub claude_ai_oauth: OAuthToken,
}

impl std::fmt::Debug for OAuthCredentials {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("OAuthCredentials")
            .field("claude_ai_oauth", &self.claude_ai_oauth)
            .finish()
    }
}

#[derive(Deserialize)]
pub struct OAuthToken {
    #[serde(rename = "accessToken")]
    pub access_token: String,
    #[serde(rename = "expiresAt")]
    pub expires_at: u64,
    pub scopes: Vec<String>,
}

impl std::fmt::Debug for OAuthToken {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("OAuthToken")
            .field("access_token", &"[REDACTED]")
            .field("expires_at", &self.expires_at)
            .field("scopes", &self.scopes)
            .finish()
    }
}

#[derive(Debug, Deserialize)]
pub struct ProfileResponse {
    pub organization: OrgInfo,
}

#[derive(Debug, Deserialize)]
pub struct OrgInfo {
    pub uuid: String,
}

// ── Session ──────────────────────────────────────────────────────────

#[derive(Debug, Deserialize, Serialize)]
pub struct SessionsListResponse {
    pub data: Vec<Session>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Session {
    pub id: String,
    pub title: Option<String>,
    pub session_status: Option<String>,
    #[serde(rename = "type")]
    pub session_type: Option<String>,
    pub created_at: Option<String>,
    pub updated_at: Option<String>,
    pub environment_id: Option<String>,
    pub session_context: Option<SessionContext>,
    pub metadata: Option<serde_json::Value>,
    pub active_mount_paths: Option<Vec<String>>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct SessionContext {
    pub model: Option<String>,
    pub cwd: Option<String>,
    pub sources: Option<Vec<SessionSource>>,
    pub outcomes: Option<Vec<SessionOutcome>>,
    pub allowed_tools: Option<Vec<String>>,
    pub disallowed_tools: Option<Vec<String>>,
    pub knowledge_base_ids: Option<Vec<String>>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct SessionSource {
    #[serde(rename = "type")]
    pub source_type: Option<String>,
    pub url: Option<String>,
    pub revision: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct SessionOutcome {
    #[serde(rename = "type")]
    pub outcome_type: Option<String>,
    pub git_info: Option<GitInfo>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct GitInfo {
    #[serde(rename = "type")]
    pub git_type: Option<String>,
    pub repo: Option<String>,
    pub branches: Option<Vec<String>>,
}

// ── Events ───────────────────────────────────────────────────────────

#[derive(Debug, Deserialize, Serialize)]
pub struct EventsResponse {
    pub data: Vec<SessionEvent>,
    pub first_id: Option<String>,
    pub last_id: Option<String>,
    pub has_more: Option<bool>,
}

/// A tagged union over every event type the sessions API can return.
/// Uses `#[serde(tag = "type")]` for internally-tagged deserialization.
#[derive(Debug, Deserialize, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum SessionEvent {
    System(SystemEvent),
    User(UserEvent),
    Assistant(AssistantEvent),
    ToolUseSummary(ToolUseSummaryEvent),
    ToolProgress(ToolProgressEvent),
    Result(ResultEvent),
    ControlResponse(ControlResponseEvent),
    EnvManagerLog(EnvManagerLogEvent),
    /// Catch-all for unknown event types to avoid deserialization failures.
    #[serde(other)]
    Unknown,
}

impl SessionEvent {
    pub fn event_type(&self) -> &str {
        match self {
            Self::System(_) => "system",
            Self::User(_) => "user",
            Self::Assistant(_) => "assistant",
            Self::ToolUseSummary(_) => "tool_use_summary",
            Self::ToolProgress(_) => "tool_progress",
            Self::Result(_) => "result",
            Self::ControlResponse(_) => "control_response",
            Self::EnvManagerLog(_) => "env_manager_log",
            Self::Unknown => "unknown",
        }
    }

    pub fn created_at(&self) -> Option<&str> {
        match self {
            Self::System(e) => e.created_at.as_deref(),
            Self::User(e) => e.created_at.as_deref(),
            Self::Assistant(e) => e.created_at.as_deref(),
            Self::ToolUseSummary(e) => e.created_at.as_deref(),
            Self::ToolProgress(e) => e.created_at.as_deref(),
            Self::Result(e) => e.created_at.as_deref(),
            Self::ControlResponse(e) => e.created_at.as_deref(),
            Self::EnvManagerLog(e) => e.created_at.as_deref(),
            Self::Unknown => None,
        }
    }

    pub fn is_conversation(&self) -> bool {
        matches!(
            self,
            Self::System(_) | Self::User(_) | Self::Assistant(_) | Self::Result(_)
        )
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub struct SystemEvent {
    pub created_at: Option<String>,
    pub uuid: Option<String>,
    pub subtype: Option<String>,
    pub session_id: Option<String>,
    pub model: Option<String>,
    pub cwd: Option<String>,
    pub claude_code_version: Option<String>,
    pub tools: Option<Vec<String>>,
    pub agents: Option<Vec<String>>,
    pub skills: Option<Vec<String>>,
    pub slash_commands: Option<Vec<String>>,
    pub mcp_servers: Option<Vec<serde_json::Value>>,
    #[serde(rename = "permissionMode")]
    pub permission_mode: Option<String>,
    pub fast_mode_state: Option<String>,
    pub output_style: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct UserEvent {
    pub created_at: Option<String>,
    pub uuid: Option<String>,
    pub session_id: Option<String>,
    pub message: UserMessage,
    pub parent_tool_use_id: Option<String>,
    #[serde(rename = "isReplay")]
    pub is_replay: Option<bool>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct UserMessage {
    pub role: Option<String>,
    pub content: UserContent,
}

/// User content can be a plain string or a list of content blocks.
#[derive(Debug, Deserialize, Serialize)]
#[serde(untagged)]
pub enum UserContent {
    Text(String),
    Blocks(Vec<serde_json::Value>),
}

impl UserContent {
    pub fn as_text(&self) -> Option<&str> {
        match self {
            Self::Text(s) => Some(s),
            Self::Blocks(_) => None,
        }
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub struct AssistantEvent {
    pub created_at: Option<String>,
    pub uuid: Option<String>,
    pub session_id: Option<String>,
    pub message: AssistantMessage,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct AssistantMessage {
    pub role: Option<String>,
    pub content: Vec<ContentBlock>,
}

/// A content block in an assistant message.
#[derive(Debug, Deserialize, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ContentBlock {
    Thinking(ThinkingBlock),
    Text(TextBlock),
    ToolUse(ToolUseBlock),
    ToolResult(ToolResultBlock),
    /// Catch-all for signatures, redacted thinking, etc.
    #[serde(other)]
    Other,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ThinkingBlock {
    pub thinking: Option<String>,
    pub signature: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct TextBlock {
    pub text: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ToolUseBlock {
    pub id: Option<String>,
    pub name: Option<String>,
    pub input: Option<serde_json::Value>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ToolResultBlock {
    pub tool_use_id: Option<String>,
    pub content: Option<serde_json::Value>,
    pub is_error: Option<bool>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ToolUseSummaryEvent {
    pub created_at: Option<String>,
    pub uuid: Option<String>,
    pub session_id: Option<String>,
    pub summary: Option<String>,
    pub preceding_tool_use_ids: Option<Vec<String>>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ToolProgressEvent {
    pub created_at: Option<String>,
    pub uuid: Option<String>,
    pub session_id: Option<String>,
    pub tool_name: Option<String>,
    pub tool_use_id: Option<String>,
    pub parent_tool_use_id: Option<String>,
    pub elapsed_time_seconds: Option<u64>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ResultEvent {
    pub created_at: Option<String>,
    pub duration_ms: Option<u64>,
    pub duration_api_ms: Option<u64>,
    pub errors: Option<Vec<String>>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ControlResponseEvent {
    pub created_at: Option<String>,
    pub response: Option<ControlResponseData>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ControlResponseData {
    pub subtype: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct EnvManagerLogEvent {
    pub created_at: Option<String>,
    pub uuid: Option<String>,
    pub data: Option<EnvManagerLogData>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct EnvManagerLogData {
    pub category: Option<String>,
    pub content: Option<String>,
    pub level: Option<String>,
    pub timestamp: Option<String>,
    pub extra: Option<serde_json::Value>,
}

// ── Loglines (session_ingress) ───────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct IngressResponse {
    pub loglines: Vec<Logline>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Logline {
    #[serde(rename = "type")]
    pub log_type: Option<String>,
    pub subtype: Option<String>,
    pub content: Option<String>,
    pub timestamp: Option<String>,
    #[serde(rename = "gitBranch")]
    pub git_branch: Option<String>,
    #[serde(rename = "sessionId")]
    pub session_id: Option<String>,
    pub cwd: Option<String>,
    pub level: Option<String>,
    #[serde(rename = "isMeta")]
    pub is_meta: Option<bool>,
    #[serde(rename = "isSidechain")]
    pub is_sidechain: Option<bool>,
    pub slug: Option<String>,
    #[serde(rename = "compactMetadata")]
    pub compact_metadata: Option<serde_json::Value>,
    /// Catch any additional fields we haven't mapped yet.
    #[serde(flatten)]
    pub extra: serde_json::Map<String, serde_json::Value>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    // ── OAuthCredentials ────────────────────────────────────────────

    #[test]
    fn deserialize_oauth_credentials() {
        let json = json!({
            "claudeAiOauth": {
                "accessToken": "tok_abc",
                "refreshToken": "ref_xyz",
                "expiresAt": 1700000000,
                "scopes": ["read", "write"],
                "subscriptionType": "pro",
                "rateLimitTier": "tier1"
            }
        });
        let creds: OAuthCredentials = serde_json::from_value(json).unwrap();
        assert_eq!(creds.claude_ai_oauth.access_token, "tok_abc");
        assert_eq!(creds.claude_ai_oauth.expires_at, 1700000000);
        assert_eq!(creds.claude_ai_oauth.scopes, vec!["read", "write"]);
    }

    #[test]
    fn deserialize_oauth_credentials_ignores_extra_fields() {
        let json = json!({
            "claudeAiOauth": {
                "accessToken": "tok",
                "expiresAt": 0,
                "scopes": [],
                "refreshToken": "ignored",
                "subscriptionType": "pro",
                "rateLimitTier": "tier1"
            }
        });
        let creds: OAuthCredentials = serde_json::from_value(json).unwrap();
        assert_eq!(creds.claude_ai_oauth.access_token, "tok");
    }

    #[test]
    fn oauth_token_debug_redacts_secrets() {
        let json = json!({
            "claudeAiOauth": {
                "accessToken": "super_secret_token",
                "expiresAt": 9999999999u64,
                "scopes": ["read"]
            }
        });
        let creds: OAuthCredentials = serde_json::from_value(json).unwrap();
        let debug_output = format!("{:?}", creds);
        assert!(!debug_output.contains("super_secret_token"));
        assert!(debug_output.contains("[REDACTED]"));
    }

    // ── ProfileResponse ─────────────────────────────────────────────

    #[test]
    fn deserialize_profile_response() {
        let json = json!({
            "organization": { "uuid": "org-123" },
            "account": { "uuid": "acc-456", "display_name": "Test" }
        });
        let profile: ProfileResponse = serde_json::from_value(json).unwrap();
        assert_eq!(profile.organization.uuid, "org-123");
    }

    #[test]
    fn profile_response_ignores_extra_fields() {
        let json = json!({
            "organization": { "uuid": "org-1", "name": "Acme" },
            "account": { "uuid": "a" },
            "some_future_field": true
        });
        let profile: ProfileResponse = serde_json::from_value(json).unwrap();
        assert_eq!(profile.organization.uuid, "org-1");
    }

    // ── Session ─────────────────────────────────────────────────────

    #[test]
    fn deserialize_session_minimal() {
        let json = json!({ "id": "session_01abc" });
        let session: Session = serde_json::from_value(json).unwrap();
        assert_eq!(session.id, "session_01abc");
        assert!(session.title.is_none());
        assert!(session.session_status.is_none());
        assert!(session.session_context.is_none());
    }

    #[test]
    fn deserialize_session_full() {
        let json = json!({
            "id": "session_01xyz",
            "title": "My session",
            "session_status": "running",
            "type": "remote",
            "created_at": "2025-01-01T00:00:00Z",
            "updated_at": "2025-01-01T01:00:00Z",
            "environment_id": "env_01",
            "session_context": {
                "model": "claude-sonnet-4-20250514",
                "cwd": "/home/user",
                "sources": [{
                    "type": "git",
                    "url": "https://github.com/user/repo",
                    "revision": "abc123"
                }],
                "outcomes": [{
                    "type": "git",
                    "git_info": {
                        "type": "push",
                        "repo": "user/repo",
                        "branches": ["main", "feature"]
                    }
                }]
            }
        });
        let session: Session = serde_json::from_value(json).unwrap();
        assert_eq!(session.title.as_deref(), Some("My session"));
        assert_eq!(session.session_status.as_deref(), Some("running"));
        assert_eq!(session.session_type.as_deref(), Some("remote"));

        let ctx = session.session_context.unwrap();
        assert_eq!(ctx.model.as_deref(), Some("claude-sonnet-4-20250514"));
        assert_eq!(ctx.sources.as_ref().unwrap().len(), 1);
        assert_eq!(
            ctx.sources.as_ref().unwrap()[0].url.as_deref(),
            Some("https://github.com/user/repo")
        );

        let git = ctx.outcomes.as_ref().unwrap()[0].git_info.as_ref().unwrap();
        assert_eq!(git.branches.as_ref().unwrap(), &["main", "feature"]);
    }

    #[test]
    fn deserialize_sessions_list_response() {
        let json = json!({
            "data": [
                { "id": "s1" },
                { "id": "s2", "title": "Second" }
            ]
        });
        let resp: SessionsListResponse = serde_json::from_value(json).unwrap();
        assert_eq!(resp.data.len(), 2);
        assert_eq!(resp.data[0].id, "s1");
        assert_eq!(resp.data[1].title.as_deref(), Some("Second"));
    }

    // ── SessionEvent deserialization ────────────────────────────────

    #[test]
    fn deserialize_system_event() {
        let json = json!({
            "type": "system",
            "created_at": "2025-01-01T00:00:00Z",
            "subtype": "init",
            "model": "claude-sonnet-4-20250514",
            "cwd": "/tmp"
        });
        let event: SessionEvent = serde_json::from_value(json).unwrap();
        assert_eq!(event.event_type(), "system");
        assert_eq!(event.created_at(), Some("2025-01-01T00:00:00Z"));
        assert!(event.is_conversation());

        if let SessionEvent::System(e) = &event {
            assert_eq!(e.subtype.as_deref(), Some("init"));
            assert_eq!(e.model.as_deref(), Some("claude-sonnet-4-20250514"));
            assert_eq!(e.cwd.as_deref(), Some("/tmp"));
        } else {
            panic!("Expected System variant");
        }
    }

    #[test]
    fn deserialize_user_event_text_content() {
        let json = json!({
            "type": "user",
            "created_at": "2025-01-01T00:00:00Z",
            "message": {
                "role": "user",
                "content": "Hello, world!"
            }
        });
        let event: SessionEvent = serde_json::from_value(json).unwrap();
        assert_eq!(event.event_type(), "user");
        assert!(event.is_conversation());

        if let SessionEvent::User(e) = &event {
            assert_eq!(e.message.content.as_text(), Some("Hello, world!"));
        } else {
            panic!("Expected User variant");
        }
    }

    #[test]
    fn deserialize_user_event_blocks_content() {
        let json = json!({
            "type": "user",
            "message": {
                "role": "user",
                "content": [
                    { "type": "text", "text": "block content" }
                ]
            }
        });
        let event: SessionEvent = serde_json::from_value(json).unwrap();
        if let SessionEvent::User(e) = &event {
            assert!(e.message.content.as_text().is_none());
            if let UserContent::Blocks(blocks) = &e.message.content {
                assert_eq!(blocks.len(), 1);
            } else {
                panic!("Expected Blocks variant");
            }
        } else {
            panic!("Expected User variant");
        }
    }

    #[test]
    fn deserialize_assistant_event_with_content_blocks() {
        let json = json!({
            "type": "assistant",
            "created_at": "2025-01-01T00:00:00Z",
            "message": {
                "role": "assistant",
                "content": [
                    { "type": "thinking", "thinking": "Let me think..." },
                    { "type": "text", "text": "Here is my answer." },
                    {
                        "type": "tool_use",
                        "id": "tu_1",
                        "name": "Bash",
                        "input": { "command": "ls" }
                    },
                    {
                        "type": "tool_result",
                        "tool_use_id": "tu_1",
                        "content": "file.txt",
                        "is_error": false
                    },
                    { "type": "redacted_thinking", "data": "xyz" }
                ]
            }
        });
        let event: SessionEvent = serde_json::from_value(json).unwrap();
        assert_eq!(event.event_type(), "assistant");

        if let SessionEvent::Assistant(e) = &event {
            assert_eq!(e.message.content.len(), 5);
            assert!(
                matches!(&e.message.content[0], ContentBlock::Thinking(t) if t.thinking.as_deref() == Some("Let me think..."))
            );
            assert!(
                matches!(&e.message.content[1], ContentBlock::Text(t) if t.text.as_deref() == Some("Here is my answer."))
            );
            assert!(
                matches!(&e.message.content[2], ContentBlock::ToolUse(t) if t.name.as_deref() == Some("Bash"))
            );
            assert!(
                matches!(&e.message.content[3], ContentBlock::ToolResult(t) if t.is_error == Some(false))
            );
            assert!(matches!(&e.message.content[4], ContentBlock::Other));
        } else {
            panic!("Expected Assistant variant");
        }
    }

    #[test]
    fn deserialize_tool_use_summary_event() {
        let json = json!({
            "type": "tool_use_summary",
            "created_at": "2025-01-01T00:00:00Z",
            "summary": "Read 3 files and edited 1",
            "preceding_tool_use_ids": ["tu_1", "tu_2"]
        });
        let event: SessionEvent = serde_json::from_value(json).unwrap();
        assert_eq!(event.event_type(), "tool_use_summary");
        assert!(!event.is_conversation());

        if let SessionEvent::ToolUseSummary(e) = &event {
            assert_eq!(e.summary.as_deref(), Some("Read 3 files and edited 1"));
            assert_eq!(
                e.preceding_tool_use_ids.as_ref().unwrap(),
                &["tu_1", "tu_2"]
            );
        } else {
            panic!("Expected ToolUseSummary variant");
        }
    }

    #[test]
    fn deserialize_tool_progress_event() {
        let json = json!({
            "type": "tool_progress",
            "tool_name": "Bash",
            "elapsed_time_seconds": 5
        });
        let event: SessionEvent = serde_json::from_value(json).unwrap();
        assert_eq!(event.event_type(), "tool_progress");
        assert!(!event.is_conversation());

        if let SessionEvent::ToolProgress(e) = &event {
            assert_eq!(e.tool_name.as_deref(), Some("Bash"));
            assert_eq!(e.elapsed_time_seconds, Some(5));
        } else {
            panic!("Expected ToolProgress variant");
        }
    }

    #[test]
    fn deserialize_result_event() {
        let json = json!({
            "type": "result",
            "created_at": "2025-01-01T00:00:00Z",
            "duration_ms": 15000,
            "duration_api_ms": 12000
        });
        let event: SessionEvent = serde_json::from_value(json).unwrap();
        assert_eq!(event.event_type(), "result");
        assert!(event.is_conversation());

        if let SessionEvent::Result(e) = &event {
            assert_eq!(e.duration_ms, Some(15000));
            assert_eq!(e.duration_api_ms, Some(12000));
        } else {
            panic!("Expected Result variant");
        }
    }

    #[test]
    fn deserialize_control_response_event() {
        let json = json!({
            "type": "control_response",
            "created_at": "2025-01-01T00:00:00Z",
            "response": { "subtype": "resume" }
        });
        let event: SessionEvent = serde_json::from_value(json).unwrap();
        assert_eq!(event.event_type(), "control_response");

        if let SessionEvent::ControlResponse(e) = &event {
            assert_eq!(
                e.response.as_ref().unwrap().subtype.as_deref(),
                Some("resume")
            );
        } else {
            panic!("Expected ControlResponse variant");
        }
    }

    #[test]
    fn deserialize_env_manager_log_event() {
        let json = json!({
            "type": "env_manager_log",
            "created_at": "2025-01-01T00:00:00Z",
            "data": {
                "category": "setup",
                "content": "Installing deps...",
                "level": "info"
            }
        });
        let event: SessionEvent = serde_json::from_value(json).unwrap();
        assert_eq!(event.event_type(), "env_manager_log");

        if let SessionEvent::EnvManagerLog(e) = &event {
            let d = e.data.as_ref().unwrap();
            assert_eq!(d.content.as_deref(), Some("Installing deps..."));
            assert_eq!(d.level.as_deref(), Some("info"));
            assert_eq!(d.category.as_deref(), Some("setup"));
        } else {
            panic!("Expected EnvManagerLog variant");
        }
    }

    #[test]
    fn deserialize_unknown_event_type() {
        let json = json!({
            "type": "future_event_type",
            "some_field": "value"
        });
        let event: SessionEvent = serde_json::from_value(json).unwrap();
        assert_eq!(event.event_type(), "unknown");
        assert!(event.created_at().is_none());
        assert!(!event.is_conversation());
    }

    // ── EventsResponse ──────────────────────────────────────────────

    #[test]
    fn deserialize_events_response_with_pagination() {
        let json = json!({
            "data": [
                { "type": "system", "subtype": "init" },
                { "type": "user", "message": { "content": "hi" } }
            ],
            "first_id": "evt_first",
            "last_id": "evt_last",
            "has_more": true
        });
        let resp: EventsResponse = serde_json::from_value(json).unwrap();
        assert_eq!(resp.data.len(), 2);
        assert_eq!(resp.first_id.as_deref(), Some("evt_first"));
        assert_eq!(resp.last_id.as_deref(), Some("evt_last"));
        assert_eq!(resp.has_more, Some(true));
    }

    #[test]
    fn deserialize_events_response_no_more() {
        let json = json!({
            "data": [],
            "has_more": false
        });
        let resp: EventsResponse = serde_json::from_value(json).unwrap();
        assert!(resp.data.is_empty());
        assert_eq!(resp.has_more, Some(false));
        assert!(resp.last_id.is_none());
    }

    // ── ContentBlock ────────────────────────────────────────────────

    #[test]
    fn content_block_thinking_empty() {
        let json = json!({ "type": "thinking", "thinking": "" });
        let block: ContentBlock = serde_json::from_value(json).unwrap();
        if let ContentBlock::Thinking(t) = &block {
            assert_eq!(t.thinking.as_deref(), Some(""));
        } else {
            panic!("Expected Thinking variant");
        }
    }

    #[test]
    fn content_block_tool_use_with_input() {
        let json = json!({
            "type": "tool_use",
            "id": "tu_42",
            "name": "Read",
            "input": { "file_path": "/tmp/test.rs" }
        });
        let block: ContentBlock = serde_json::from_value(json).unwrap();
        if let ContentBlock::ToolUse(t) = &block {
            assert_eq!(t.id.as_deref(), Some("tu_42"));
            assert_eq!(t.name.as_deref(), Some("Read"));
            assert!(t.input.is_some());
        } else {
            panic!("Expected ToolUse variant");
        }
    }

    // ── UserContent ─────────────────────────────────────────────────

    #[test]
    fn user_content_text_variant() {
        let json = json!("plain text message");
        let content: UserContent = serde_json::from_value(json).unwrap();
        assert_eq!(content.as_text(), Some("plain text message"));
    }

    #[test]
    fn user_content_blocks_variant() {
        let json = json!([{ "type": "text", "text": "hi" }]);
        let content: UserContent = serde_json::from_value(json).unwrap();
        assert!(content.as_text().is_none());
    }

    // ── Logline ─────────────────────────────────────────────────────

    #[test]
    fn deserialize_logline() {
        let json = json!({
            "type": "user",
            "subtype": "message",
            "content": "hello",
            "timestamp": "2025-01-01T00:00:00Z",
            "gitBranch": "main",
            "sessionId": "session_01abc",
            "cwd": "/home/user",
            "isMeta": false,
            "isSidechain": false,
            "slug": "test-slug",
            "unknownField": "should be captured"
        });
        let log: Logline = serde_json::from_value(json).unwrap();
        assert_eq!(log.log_type.as_deref(), Some("user"));
        assert_eq!(log.subtype.as_deref(), Some("message"));
        assert_eq!(log.content.as_deref(), Some("hello"));
        assert_eq!(log.git_branch.as_deref(), Some("main"));
        assert_eq!(log.is_meta, Some(false));
        assert!(log.extra.contains_key("unknownField"));
    }

    #[test]
    fn deserialize_logline_minimal() {
        let json = json!({});
        let log: Logline = serde_json::from_value(json).unwrap();
        assert!(log.log_type.is_none());
        assert!(log.content.is_none());
        assert!(log.extra.is_empty());
    }

    // ── IngressResponse ─────────────────────────────────────────────

    #[test]
    fn deserialize_ingress_response() {
        let json = json!({
            "loglines": [
                { "type": "system", "content": "init" },
                { "type": "user", "content": "hi" }
            ]
        });
        let resp: IngressResponse = serde_json::from_value(json).unwrap();
        assert_eq!(resp.loglines.len(), 2);
    }

    // ── Serialization round-trip ────────────────────────────────────

    #[test]
    fn session_event_roundtrip() {
        let json = json!({
            "type": "user",
            "created_at": "2025-01-01T00:00:00Z",
            "uuid": "evt_123",
            "message": {
                "role": "user",
                "content": "test roundtrip"
            }
        });
        let event: SessionEvent = serde_json::from_value(json).unwrap();
        let serialized = serde_json::to_string(&event).unwrap();
        let deserialized: SessionEvent = serde_json::from_str(&serialized).unwrap();
        assert_eq!(deserialized.event_type(), "user");
        assert_eq!(deserialized.created_at(), Some("2025-01-01T00:00:00Z"));
    }

    #[test]
    fn session_roundtrip() {
        let session = Session {
            id: "session_01test".to_string(),
            title: Some("Test Session".to_string()),
            session_status: Some("completed".to_string()),
            session_type: Some("remote".to_string()),
            created_at: Some("2025-01-01T00:00:00Z".to_string()),
            updated_at: None,
            environment_id: None,
            session_context: None,
            metadata: None,
            active_mount_paths: None,
        };
        let serialized = serde_json::to_string(&session).unwrap();
        let deserialized: Session = serde_json::from_str(&serialized).unwrap();
        assert_eq!(deserialized.id, "session_01test");
        assert_eq!(deserialized.title.as_deref(), Some("Test Session"));
    }

    // ── Mixed event list (like real API responses) ──────────────────

    #[test]
    fn deserialize_mixed_event_list() {
        let json = json!({
            "data": [
                { "type": "system", "subtype": "init", "model": "opus" },
                { "type": "user", "message": { "content": "hi" } },
                { "type": "assistant", "message": { "content": [{ "type": "text", "text": "hello" }] } },
                { "type": "tool_use_summary", "summary": "Read file" },
                { "type": "tool_progress", "tool_name": "Bash", "elapsed_time_seconds": 2 },
                { "type": "result", "duration_ms": 5000 },
                { "type": "control_response", "response": { "subtype": "ack" } },
                { "type": "env_manager_log", "data": { "level": "warn", "content": "slow" } },
                { "type": "never_seen_before" }
            ],
            "has_more": false
        });
        let resp: EventsResponse = serde_json::from_value(json).unwrap();
        assert_eq!(resp.data.len(), 9);

        let types: Vec<&str> = resp.data.iter().map(|e| e.event_type()).collect();
        assert_eq!(
            types,
            vec![
                "system",
                "user",
                "assistant",
                "tool_use_summary",
                "tool_progress",
                "result",
                "control_response",
                "env_manager_log",
                "unknown"
            ]
        );
    }
}
