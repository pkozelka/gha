# Task 5 - Webhook-Based Waiting (Branch: feature/webhook-wait)

Use GitHub webhooks instead of polling for more efficient long-running workflow monitoring.

## Overview

Instead of polling the GitHub API every 0.5 seconds (which hits rate limits on large-scale deployments), implement a webhook-based listener that receives push events when workflow runs complete.

## Requirements

1. **Start a local webhook server**
   - Bind to `127.0.0.1:3456` (or configured port)
   - Expose `/webhook/github` endpoint
   - Verify GitHub's HMAC-SHA256 signature on incoming requests

2. **Workflow run events**
   - Listen for `workflow_run` events (GitHub Actions events)
   - Webhook fires when run reaches terminal status (completed, canceled, etc.)
   - No polling needed - event-driven

3. **New command or flag**
   - `gha wait --webhook` — Use webhook instead of polling
   - `gha run --wait --webhook` — Same for run command
   - Still requires GitHub to be able to reach your local machine
   - Better for CI/CD environments or long-running workflows

4. **Setup**
   - User must configure webhook in GitHub repo settings
   - Webhook URL: `https://your-machine:3456/webhook/github`
   - Events: `workflow_runs`
   - No need for this to be public (can be private network)

## Implementation (Branch feature/webhook-wait)

New module `src/webhook.rs`:
- `struct WebhookServer { listener, secret_key }`
- `async fn start_webhook_server(port) -> WebhookServer`
- `async fn wait_for_workflow_event(server, run_id, timeout) -> WorkflowRun`
- Verify GitHub signature on requests
- Parse workflow_run event JSON
- Return when correct run_id completes

Update `src/main.rs`:
- Add `--webhook` flag to Run and Wait commands
- Pass to `wait_for_run()` as part of `WaitOptions`
- Choose between polling vs. webhook based on flag

## Notes

- This is advanced and optional - polling works fine for most users
- Webhooks are more complex but much more efficient
- Requires GitHub webhook setup (out-of-app requirement)
- Good for production CI/CD pipelines with long-running workflows

## Branch Management

- Create feature branch: `git checkout -b feature/webhook-wait`
- Implement webhook logic in this branch
- Can merge later when fully tested
- Polling remains the default

