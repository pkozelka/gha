// ! Task 5: Webhook-based waiting — scaffolding
//
// This module will implement event-driven workflow run monitoring
// using GitHub webhooks instead of polling.
//
// TODO: Implement webhook server on configurable port
// TODO: Add --webhook flag to run/wait commands
// TODO: Add HMAC-SHA256 signature verification for GitHub
// TODO: Subscribe to workflow_run events
// TODO: Match incoming events to requested run ID
// TODO: Return completion when event arrives

use anyhow::Result;

/// Start a local webhook server for GitHub events
///
/// Listens for workflow_run events from GitHub.
/// GitHub must be configured to send webhooks to this address.
pub async fn start_webhook_server(_port: u16) -> Result<()> {
    // TODO: Bind to localhost:port
    // TODO: Set up HTTP endpoint for webhook
    // TODO: Implement request signature verification
    unimplemented!("Webhook server not yet implemented")
}

/// Wait for a specific workflow run to complete via webhook
pub async fn wait_for_run_via_webhook(
    _repo: &str,
    _run_id: u64,
    _timeout_secs: u64,
) -> Result<()> {
    // TODO: Start webhook server
    // TODO: Register interest in specific run ID
    // TODO: Wait for matching workflow_run event
    // TODO: Verify event matches our run
    // TODO: Extract conclusion from event
    // TODO: Return result
    unimplemented!("Webhook waiting not yet implemented")
}

