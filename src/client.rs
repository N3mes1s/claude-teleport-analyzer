use anyhow::{Context, Result, bail};
use reqwest::header::{AUTHORIZATION, CONTENT_TYPE, HeaderMap, HeaderValue};
use std::path::PathBuf;
use std::time::Duration;

use crate::types::*;

const BASE_API_URL: &str = "https://api.anthropic.com";
const ANTHROPIC_VERSION: &str = "2023-06-01";
const ANTHROPIC_BETA: &str = "ccr-byoc-2025-07-29";
const REQUEST_TIMEOUT: Duration = Duration::from_secs(30);
const CONNECT_TIMEOUT: Duration = Duration::from_secs(10);

pub fn validate_session_id(id: &str) -> Result<()> {
    if !id.starts_with("session_") || id.len() < 16 {
        bail!(
            "Invalid session ID format: '{id}'. \
             Expected format: session_01... (e.g. session_01QJaJSUgfY6khmFTzJaMqph)"
        );
    }
    Ok(())
}

pub struct ApiClient {
    client: reqwest::Client,
    access_token: String,
    org_uuid: String,
}

impl ApiClient {
    pub async fn new() -> Result<Self> {
        let creds = load_credentials()?;
        let access_token = creds.claude_ai_oauth.access_token;

        let client = reqwest::Client::builder()
            .timeout(REQUEST_TIMEOUT)
            .connect_timeout(CONNECT_TIMEOUT)
            .build()
            .context("Failed to build HTTP client")?;

        let org_uuid = fetch_org_uuid(&client, &access_token).await?;

        Ok(Self {
            client,
            access_token,
            org_uuid,
        })
    }

    fn headers(&self) -> Result<HeaderMap> {
        let mut headers = HeaderMap::new();
        headers.insert(
            AUTHORIZATION,
            HeaderValue::from_str(&format!("Bearer {}", self.access_token))?,
        );
        headers.insert(
            "x-organization-uuid",
            HeaderValue::from_str(&self.org_uuid)?,
        );
        headers.insert("anthropic-beta", HeaderValue::from_static(ANTHROPIC_BETA));
        headers.insert(
            "anthropic-version",
            HeaderValue::from_static(ANTHROPIC_VERSION),
        );
        headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
        Ok(headers)
    }

    pub async fn list_sessions(&self) -> Result<Vec<Session>> {
        let url = format!("{BASE_API_URL}/v1/sessions");
        let resp = self
            .client
            .get(&url)
            .headers(self.headers()?)
            .send()
            .await
            .context("Failed to connect to Anthropic API")?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            bail!("Failed to list sessions: {status} - {body}");
        }

        let data: SessionsListResponse = resp
            .json()
            .await
            .context("Failed to parse sessions list response")?;
        Ok(data.data)
    }

    pub async fn get_session(&self, session_id: &str) -> Result<Session> {
        let url = format!("{BASE_API_URL}/v1/sessions/{session_id}");
        let resp = self
            .client
            .get(&url)
            .headers(self.headers()?)
            .send()
            .await
            .with_context(|| format!("Failed to fetch session {session_id}"))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            bail!("Session {session_id} not found: {status} - {body}");
        }

        resp.json()
            .await
            .with_context(|| format!("Failed to parse session {session_id} response"))
    }

    pub async fn get_events(
        &self,
        session_id: &str,
        max_events: usize,
    ) -> Result<Vec<SessionEvent>> {
        let mut all_events = Vec::new();
        let mut after_id: Option<String> = None;

        loop {
            let mut url =
                reqwest::Url::parse(&format!("{BASE_API_URL}/v1/sessions/{session_id}/events"))
                    .context("Failed to build events URL")?;

            if let Some(ref aid) = after_id {
                url.query_pairs_mut().append_pair("after_id", aid);
            }

            let resp = self
                .client
                .get(url)
                .headers(self.headers()?)
                .send()
                .await
                .with_context(|| {
                    format!(
                        "Failed to fetch events for session {session_id} (page {})",
                        all_events.len() / 1000 + 1
                    )
                })?;

            if !resp.status().is_success() {
                let status = resp.status();
                let body = resp.text().await.unwrap_or_default();
                bail!("Failed to fetch events for session {session_id}: {status} - {body}");
            }

            let page: EventsResponse = resp.json().await.with_context(|| {
                format!("Failed to parse events response for session {session_id}")
            })?;
            all_events.extend(page.data);

            eprint!("\r  Fetched {} events...", all_events.len());

            if max_events > 0 && all_events.len() >= max_events {
                all_events.truncate(max_events);
                break;
            }

            if page.has_more != Some(true) {
                break;
            }

            after_id = page.last_id;
        }
        eprintln!();

        Ok(all_events)
    }

    pub async fn get_loglines(&self, session_id: &str) -> Result<Vec<Logline>> {
        let url = format!("{BASE_API_URL}/v1/session_ingress/session/{session_id}");
        let resp = self
            .client
            .get(&url)
            .headers(self.headers()?)
            .send()
            .await
            .with_context(|| format!("Failed to fetch loglines for session {session_id}"))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            bail!("Failed to fetch loglines for session {session_id}: {status} - {body}");
        }

        let data: IngressResponse = resp
            .json()
            .await
            .with_context(|| format!("Failed to parse loglines for session {session_id}"))?;
        Ok(data.loglines)
    }
}

/// Returns the path to `.credentials.json`, respecting `CLAUDE_CONFIG_DIR`.
pub fn credentials_file_path() -> PathBuf {
    if let Ok(dir) = std::env::var("CLAUDE_CONFIG_DIR") {
        return PathBuf::from(dir).join(".credentials.json");
    }
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".claude")
        .join(".credentials.json")
}

fn load_credentials_from_file(path: &std::path::Path) -> Result<OAuthCredentials> {
    let json_str = std::fs::read_to_string(path)
        .with_context(|| format!("Failed to read credentials from {}", path.display()))?;
    serde_json::from_str(json_str.trim())
        .with_context(|| format!("Failed to parse credentials JSON from {}", path.display()))
}

#[cfg(target_os = "macos")]
fn load_credentials_from_keychain() -> Result<OAuthCredentials> {
    let output = std::process::Command::new("security")
        .args([
            "find-generic-password",
            "-s",
            "Claude Code-credentials",
            "-w",
        ])
        .output()
        .context(
            "Failed to run 'security' command. \
             This tool requires macOS Keychain access.",
        )?;

    if !output.status.success() {
        bail!("No Claude Code credentials found in macOS Keychain.");
    }

    let json_str =
        String::from_utf8(output.stdout).context("Credentials output is not valid UTF-8")?;
    serde_json::from_str(json_str.trim()).context("Failed to parse credentials JSON from Keychain")
}

fn load_credentials() -> Result<OAuthCredentials> {
    // On macOS, try Keychain first, then fall back to file.
    #[cfg(target_os = "macos")]
    {
        if let Ok(creds) = load_credentials_from_keychain() {
            return Ok(creds);
        }
    }

    // All platforms: try the credentials file.
    let path = credentials_file_path();
    if path.exists() {
        return load_credentials_from_file(&path);
    }

    #[cfg(target_os = "macos")]
    bail!(
        "No Claude Code credentials found. \
         Checked macOS Keychain and {}. \
         Make sure you're logged in with 'claude' first.",
        path.display()
    );

    #[cfg(not(target_os = "macos"))]
    bail!(
        "No Claude Code credentials found at {}. \
         Make sure you're logged in with 'claude' first.",
        path.display()
    );
}

async fn fetch_org_uuid(client: &reqwest::Client, token: &str) -> Result<String> {
    let url = format!("{BASE_API_URL}/api/oauth/profile");
    let resp = client
        .get(&url)
        .header(AUTHORIZATION, format!("Bearer {token}"))
        .header(CONTENT_TYPE, "application/json")
        .send()
        .await
        .context("Failed to fetch profile")?;

    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        bail!("Failed to fetch profile (token may be expired): {status} - {body}");
    }

    let profile: ProfileResponse = resp
        .json()
        .await
        .context("Failed to parse profile response")?;
    Ok(profile.organization.uuid)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validate_session_id_valid() {
        assert!(validate_session_id("session_01QJaJSUgfY6khmFTzJaMqph").is_ok());
        assert!(validate_session_id("session_01NVvyq9UPuvtLMPpjdd3dNQ").is_ok());
    }

    #[test]
    fn validate_session_id_invalid_prefix() {
        let err = validate_session_id("ses_01QJaJSUgfY6khmFTzJaMqph").unwrap_err();
        assert!(err.to_string().contains("Invalid session ID format"));
    }

    #[test]
    fn validate_session_id_too_short() {
        let err = validate_session_id("session_01").unwrap_err();
        assert!(err.to_string().contains("Invalid session ID format"));
    }

    #[test]
    fn validate_session_id_empty() {
        let err = validate_session_id("").unwrap_err();
        assert!(err.to_string().contains("Invalid session ID format"));
    }

    #[test]
    fn validate_session_id_no_prefix() {
        let err = validate_session_id("01QJaJSUgfY6khmFTzJaMqph").unwrap_err();
        assert!(err.to_string().contains("Invalid session ID format"));
    }

    // ── Credential path resolution ─────────────────────────────────

    #[test]
    fn credentials_file_path_default() {
        // Temporarily unset CLAUDE_CONFIG_DIR to test default behavior
        let prev = std::env::var("CLAUDE_CONFIG_DIR").ok();
        // SAFETY: Only used in tests, acceptable for single-threaded test context.
        unsafe { std::env::remove_var("CLAUDE_CONFIG_DIR") };

        let path = credentials_file_path();
        assert!(path.ends_with(".claude/.credentials.json"));

        if let Some(v) = prev {
            unsafe { std::env::set_var("CLAUDE_CONFIG_DIR", v) };
        }
    }

    #[test]
    fn credentials_file_path_with_env_override() {
        let prev = std::env::var("CLAUDE_CONFIG_DIR").ok();
        // SAFETY: Only used in tests, acceptable for single-threaded test context.
        unsafe { std::env::set_var("CLAUDE_CONFIG_DIR", "/tmp/custom-claude-config") };

        let path = credentials_file_path();
        assert_eq!(
            path,
            PathBuf::from("/tmp/custom-claude-config/.credentials.json")
        );

        match prev {
            Some(v) => unsafe { std::env::set_var("CLAUDE_CONFIG_DIR", v) },
            None => unsafe { std::env::remove_var("CLAUDE_CONFIG_DIR") },
        }
    }

    #[test]
    fn load_credentials_from_file_valid() {
        let dir = std::env::temp_dir().join("cta-test-creds");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join(".credentials.json");
        std::fs::write(
            &path,
            r#"{
                "claudeAiOauth": {
                    "accessToken": "test_token",
                    "refreshToken": "test_refresh",
                    "expiresAt": 9999999999,
                    "scopes": ["user:inference"]
                }
            }"#,
        )
        .unwrap();

        let creds = load_credentials_from_file(&path).unwrap();
        assert_eq!(creds.claude_ai_oauth.access_token, "test_token");
        assert_eq!(creds.claude_ai_oauth.refresh_token, "test_refresh");

        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn load_credentials_from_file_missing() {
        let path = PathBuf::from("/tmp/nonexistent-cta-creds/.credentials.json");
        let err = load_credentials_from_file(&path).unwrap_err();
        assert!(err.to_string().contains("Failed to read credentials"));
    }

    #[test]
    fn load_credentials_from_file_invalid_json() {
        let dir = std::env::temp_dir().join("cta-test-bad-creds");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join(".credentials.json");
        std::fs::write(&path, "not valid json").unwrap();

        let err = load_credentials_from_file(&path).unwrap_err();
        assert!(err.to_string().contains("Failed to parse credentials"));

        std::fs::remove_dir_all(&dir).unwrap();
    }
}
