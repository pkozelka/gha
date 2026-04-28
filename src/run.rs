use reqwest::Client;
use serde_json::{json, Map, Value};
use std::fs;
use tracing::{debug, info, trace};
use anyhow::{Context, Result};
use crate::auth::GithubAuth;
use crate::gen_client::{parse_workflow, WorkflowInfo};

/// Dispatch a workflow directly via GitHub API
pub async fn run_workflow(
    repo: &str,
    workflow: &str,
    r#ref: &str,
    auth: &GithubAuth,
    inputs: &[(String, String)],
) -> Result<()> {
    let url = format!(
        "https://api.github.com/repos/{}/actions/workflows/{}/dispatches",
        repo, workflow
    );

    // Build inputs map
    let mut input_map = Map::new();
    for (key, value) in inputs {
        input_map.insert(key.clone(), Value::String(value.clone()));
    }

    let payload = json!({
        "ref": r#ref,
        "inputs": input_map,
    });

    debug!(
        "Dispatching workflow: repo={}, workflow={}, ref={}",
        repo, workflow, r#ref
    );
    debug!("Payload: {}", serde_json::to_string_pretty(&payload)?);

    let client = Client::new();

    // Build request
    let mut builder = client.post(&url);

    // Set up headers
    builder = builder
        .header("Accept", "application/vnd.github+json")
        .header("User-Agent", "gha")
        .header("X-GitHub-Api-Version", "2022-11-28");

    if auth.is_valid() {
        builder = builder.header("Authorization", format!("Bearer {}", auth.token));
        debug!("Using Bearer token authentication");
    } else {
        debug!("No token provided; relying on .netrc or other auth");
    }

    let request = builder.json(&payload).build()?;

    debug!(
        "HTTP Request: {} {}",
        request.method(),
        request.url()
    );

    for (name, value) in request.headers() {
        trace!("Request header: {}: {:?}", name, value);
    }

    trace_request_body(&request)?;

    let response = client
        .execute(request)
        .await
        .context("Failed to send workflow dispatch request")?;

    let status = response.status();
    debug!("HTTP Response Status: {}", status);

    let headers = response.headers().clone();
    for (name, value) in &headers {
        trace!("Response header: {}: {:?}", name, value);
    }

    let body = response
        .text()
        .await
        .context("Failed to read response body")?;

    if !status.is_success() {
        debug!("Response body (error): {}", body);
        anyhow::bail!("GitHub API error: {} - {}", status, body);
    }

    if !body.is_empty() {
        debug!("Response body (success): {}", body);
    }

    info!(
        "Workflow {} dispatched successfully on ref {}",
        workflow, r#ref
    );

    Ok(())
}

/// Parse command-line input arguments in name=value or name=@file format
pub fn parse_input_args(args: &[String]) -> Result<Vec<(String, String)>> {
    let mut inputs = Vec::new();

    for arg in args {
        if let Some((key, value)) = arg.split_once('=') {
            let val = if value.starts_with('@') {
                let file_path = &value[1..];
                fs::read_to_string(file_path)
                    .with_context(|| format!("Failed to read input file: {}", file_path))?
            } else {
                value.to_string()
            };
            inputs.push((key.to_string(), val));
        } else {
            anyhow::bail!("Invalid input format: '{}'. Use name=value or name=@file", arg);
        }
    }

    Ok(inputs)
}

/// Get workflow information for shell completion
#[allow(dead_code)]
pub fn get_workflow_info(workflow_path: &std::path::Path) -> Result<Option<WorkflowInfo>> {
    parse_workflow(workflow_path)
}

fn trace_request_body(request: &reqwest::Request) -> Result<()> {
    if let Some(body) = request.body() {
        if let Some(bytes) = body.as_bytes() {
            if let Ok(text) = std::str::from_utf8(bytes) {
                trace!("Request body: {}", text);
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_simple_input() {
        let args = vec!["key=value".to_string()];
        let inputs = parse_input_args(&args).unwrap();
        assert_eq!(inputs.len(), 1);
        assert_eq!(inputs[0].0, "key");
        assert_eq!(inputs[0].1, "value");
    }

    #[test]
    fn parse_multiple_inputs() {
        let args = vec![
            "key1=value1".to_string(),
            "key2=value2".to_string(),
        ];
        let inputs = parse_input_args(&args).unwrap();
        assert_eq!(inputs.len(), 2);
    }

    #[test]
    fn parse_input_with_equals_in_value() {
        let args = vec!["key=foo=bar".to_string()];
        let inputs = parse_input_args(&args).unwrap();
        assert_eq!(inputs[0].0, "key");
        assert_eq!(inputs[0].1, "foo=bar");
    }

    #[test]
    fn reject_malformed_input() {
        let args = vec!["no_equals_here".to_string()];
        let result = parse_input_args(&args);
        assert!(result.is_err());
    }
}






