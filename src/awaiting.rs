use anyhow::Result;
use serde_json::{json, Value};
use std::time::Duration;
use tracing::debug;

/// Options for awaiting a workflow run
#[derive(Debug, Clone)]
pub struct AwaitOptions {
    /// Maximum time to await in seconds (default: 3600 = 1 hour)
    pub timeout_secs: u64,
    /// Polling interval in milliseconds (default: 500)
    pub poll_interval_ms: u64,
    /// Output format: "human" or "json"
    pub output_format: OutputFormat,
    /// Use webhook listener instead of polling GitHub API
    pub use_webhook: bool,
    /// Local port to bind webhook listener on when webhook mode is enabled
    pub webhook_port: u16,
    /// Webhook secret used for HMAC signature verification
    pub webhook_secret: Option<String>,
    /// Stream run logs while awaiting
    pub follow_logs: bool,
}

impl Default for AwaitOptions {
    fn default() -> Self {
        Self {
            timeout_secs: 3600,
            poll_interval_ms: 500,
            output_format: OutputFormat::Human,
            use_webhook: false,
            webhook_port: 3456,
            webhook_secret: None,
            follow_logs: false,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutputFormat {
    Human,
    Json,
}

/// ...existing code...
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WorkflowRunStatus {
    Queued,
    InProgress,
    Completed,
}

/// Workflow run conclusion (final status when completed)
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WorkflowRunConclusion {
    Success,
    Failure,
    Canceled,
    Skipped,
    Neutral,
    TimedOut,
    ActionRequired,
}

impl WorkflowRunConclusion {
    /// Convert GitHub API conclusion string to enum
    pub fn from_api(s: &str) -> Option<Self> {
        match s {
            "success" => Some(Self::Success),
            "failure" => Some(Self::Failure),
            "canceled" => Some(Self::Canceled),
            "skipped" => Some(Self::Skipped),
            "neutral" => Some(Self::Neutral),
            "timed_out" => Some(Self::TimedOut),
            "action_required" => Some(Self::ActionRequired),
            _ => None,
        }
    }

    /// Human-readable status string
    pub fn display(&self) -> &'static str {
        match self {
            Self::Success => "✓ Success",
            Self::Failure => "✗ Failure",
            Self::Canceled => "⊙ Canceled",
            Self::Skipped => "⊗ Skipped",
            Self::Neutral => "○ Neutral",
            Self::TimedOut => "⏱ Timed Out",
            Self::ActionRequired => "⚠ Action Required",
        }
    }

    /// Exit code for workflow conclusion
    pub fn exit_code(&self) -> i32 {
        match self {
            Self::Success => exitcode::OK,
            Self::Failure => exitcode::DATAERR, // 65 — data format error / execution failure
            Self::Canceled => exitcode::TEMPFAIL, // 75 — temporary failure (user canceled)
            Self::Skipped => exitcode::OK,
            Self::Neutral => exitcode::OK,
            Self::TimedOut => exitcode::TEMPFAIL,
            Self::ActionRequired => exitcode::DATAERR,
        }
    }
}

/// Workflow run information fetched from GitHub API
#[derive(Debug, Clone)]
pub struct WorkflowRun {
    pub id: u64,
    pub html_url: String,
    pub status: WorkflowRunStatus,
    pub conclusion: Option<WorkflowRunConclusion>,
    pub name: String,
    pub created_at: String,
}

impl WorkflowRun {
    /// Parse a workflow run from GitHub API response
    pub fn from_api_response(json: &Value) -> Result<Self> {
        let id = json["id"]
            .as_u64()
            .ok_or_else(|| anyhow::anyhow!("missing 'id' field in run response"))?;
        let html_url = json["html_url"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("missing 'html_url' field"))?
            .to_string();
        let status_str = json["status"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("missing 'status' field"))?;
        let status = match status_str {
            "queued" => WorkflowRunStatus::Queued,
            "in_progress" => WorkflowRunStatus::InProgress,
            "completed" => WorkflowRunStatus::Completed,
            _ => anyhow::bail!("unknown status: {}", status_str),
        };
        let conclusion = json["conclusion"].as_str().and_then(|s| {
            if s == "null" || s.is_empty() {
                None
            } else {
                WorkflowRunConclusion::from_api(s)
            }
        });
        let name = json["name"]
            .as_str()
            .unwrap_or("unknown")
            .to_string();
        let created_at = json["created_at"]
            .as_str()
            .unwrap_or("unknown")
            .to_string();

        Ok(WorkflowRun {
            id,
            html_url,
            status,
            conclusion,
            name,
            created_at,
        })
    }

    /// Convert WorkflowRun to JSON output
    pub fn to_json(&self) -> Result<Value> {
        Ok(json!({
            "id": self.id,
            "name": self.name,
            "status": match self.status {
                WorkflowRunStatus::Queued => "queued",
                WorkflowRunStatus::InProgress => "in_progress",
                WorkflowRunStatus::Completed => "completed",
            },
            "conclusion": self.conclusion.as_ref().map(|c| match c {
                WorkflowRunConclusion::Success => "success",
                WorkflowRunConclusion::Failure => "failure",
                WorkflowRunConclusion::Canceled => "canceled",
                WorkflowRunConclusion::Skipped => "skipped",
                WorkflowRunConclusion::Neutral => "neutral",
                WorkflowRunConclusion::TimedOut => "timed_out",
                WorkflowRunConclusion::ActionRequired => "action_required",
            }),
            "url": self.html_url,
            "created_at": self.created_at,
        }))
    }
}

/// Await a workflow run to complete, polling the GitHub API.
///
/// Polls `GET /repos/{repo}/actions/runs/{run_id}` until the run is completed.
/// Returns the final run state and conclusion.
pub async fn await_run(
    repo: &str,
    run_id: u64,
    auth_token: &str,
    options: &AwaitOptions,
) -> Result<WorkflowRun> {
    if options.use_webhook {
        let secret = options
            .webhook_secret
            .as_deref()
            .ok_or_else(|| anyhow::anyhow!("webhook mode requires a webhook secret"))?;
        return crate::webhook::await_run_via_webhook(
            repo,
            run_id,
            options.timeout_secs,
            options.webhook_port,
            secret,
        )
        .await;
    }

    let url = format!("https://api.github.com/repos/{}/actions/runs/{}", repo, run_id);
    let client = reqwest::Client::new();

    let mut poll_count = 0;
    let poll_interval_ms = options.poll_interval_ms.max(1);
    let max_polls = ((options.timeout_secs * 1000) / poll_interval_ms).max(1);

    loop {
        poll_count += 1;
        if poll_count > max_polls {
            anyhow::bail!(
                "workflow run took too long (>{} seconds) to complete",
                options.timeout_secs
            );
        }

        debug!("Polling run status (attempt {})", poll_count);

        let mut builder = client.get(&url);
        builder = builder
            .header("Accept", "application/vnd.github+json")
            .header("User-Agent", "gha")
            .header("X-GitHub-Api-Version", "2022-11-28");

        if !auth_token.is_empty() {
            builder = builder.header("Authorization", format!("Bearer {}", auth_token));
        }

        let response = builder
            .send()
            .await
            .map_err(|e| anyhow::anyhow!("Failed to fetch run status: {}", e))?;

        let status = response.status();
        if !status.is_success() {
            let text = response
                .text()
                .await
                .unwrap_or_else(|_| "(no response body)".to_string());
            anyhow::bail!("GitHub API error: {} - {}", status, text);
        }

        let body = response
            .text()
            .await
            .map_err(|e| anyhow::anyhow!("Failed to read response body: {}", e))?;
        debug!("Run status response: {}", body);

        let json: Value = serde_json::from_str(&body)
            .map_err(|e| anyhow::anyhow!("Failed to parse run response: {}", e))?;

        let run = WorkflowRun::from_api_response(&json)?;

        match run.status {
            WorkflowRunStatus::Completed => {
                debug!(
                    "Workflow run completed with conclusion: {:?}",
                    run.conclusion
                );
                return Ok(run);
            }
            WorkflowRunStatus::Queued => {
                debug!("Run is queued, awaiting...");
                tokio::time::sleep(Duration::from_millis(poll_interval_ms)).await;
            }
            WorkflowRunStatus::InProgress => {
                debug!("Run is in progress, awaiting...");
                tokio::time::sleep(Duration::from_millis(poll_interval_ms)).await;
            }
        }
    }
}

/// Extract run ID from dispatch API response (202 Accepted, no body)
///
/// The GitHub API returns a 202 Accepted when a workflow is successfully dispatched,
/// but doesn't include the run ID in the response. We must fetch the latest run
/// for that workflow to get the ID.
pub async fn get_run_id_after_dispatch(
    repo: &str,
    workflow: &str,
    r#ref: &str,
    auth_token: &str,
) -> Result<u64> {
    let client = reqwest::Client::new();
    let url = format!(
        "https://api.github.com/repos/{}/actions/workflows/{}/runs?branch={}&per_page=20&event=workflow_dispatch",
        repo, workflow, r#ref
    );

    for attempt in 1..=20 {
        let mut builder = client.get(&url);
        builder = builder
            .header("Accept", "application/vnd.github+json")
            .header("User-Agent", "gha")
            .header("X-GitHub-Api-Version", "2022-11-28");

        if !auth_token.is_empty() {
            builder = builder.header("Authorization", format!("Bearer {}", auth_token));
        }

        let response = builder
            .send()
            .await
            .map_err(|e| anyhow::anyhow!("Failed to get run list: {}", e))?;

        let status = response.status();
        if !status.is_success() {
            let text = response
                .text()
                .await
                .unwrap_or_else(|_| "(no response body)".to_string());
            anyhow::bail!(
                "Failed to fetch run ID from workflow list: {} - {}",
                status,
                text
            );
        }

        let body = response
            .text()
            .await
            .map_err(|e| anyhow::anyhow!("Failed to read response body: {}", e))?;

        let json: Value = serde_json::from_str(&body)
            .map_err(|e| anyhow::anyhow!("Failed to parse runs response: {}", e))?;

        let runs = json["workflow_runs"]
            .as_array()
            .ok_or_else(|| anyhow::anyhow!("No 'workflow_runs' array in response"))?;

        if let Some(run_id) = runs
            .iter()
            .find(|r| {
                matches!(
                    r["status"].as_str(),
                    Some("queued") | Some("in_progress") | Some("completed")
                )
            })
            .and_then(|r| r["id"].as_u64())
        {
            debug!("Found run ID on attempt {}: {}", attempt, run_id);
            return Ok(run_id);
        }

        debug!("Run ID not available yet (attempt {}), retrying...", attempt);
        tokio::time::sleep(Duration::from_millis(500)).await;
    }

    anyhow::bail!(
        "Unable to determine run ID for workflow {} on ref {} after retries",
        workflow,
        r#ref
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn conclusion_from_api_parses_all_types() {
        assert_eq!(WorkflowRunConclusion::from_api("success"), Some(WorkflowRunConclusion::Success));
        assert_eq!(WorkflowRunConclusion::from_api("failure"), Some(WorkflowRunConclusion::Failure));
        assert_eq!(WorkflowRunConclusion::from_api("canceled"), Some(WorkflowRunConclusion::Canceled));
    }

    #[test]
    fn exit_codes_are_distinct() {
        assert_eq!(WorkflowRunConclusion::Success.exit_code(), exitcode::OK);
        assert!(WorkflowRunConclusion::Failure.exit_code() != exitcode::OK);
        assert!(WorkflowRunConclusion::Canceled.exit_code() != exitcode::OK);
    }

    #[test]
    fn conclusion_display_is_readable() {
        assert!(!WorkflowRunConclusion::Success.display().is_empty());
        assert!(WorkflowRunConclusion::Success.display().contains("Success"));
    }
}
