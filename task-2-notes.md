# Task 2 Implementation Notes

## Overview

Implemented synchronous and asynchronous workflow execution: `gha spawn --await` to dispatch and wait, plus a standalone `gha await` command to poll any previous run. Both share a reusable waiting/polling mechanism that includes proper exit codes reflecting workflow conclusion.

## Implementation Summary

### New Module: `src/awaiting.rs`

**Purpose**: Centralized polling and status management for workflow runs, used by both `spawn --await` and standalone `await`.

#### Key Types

**`WorkflowRunStatus` enum:**
- `Queued` — run queued, not started
- `InProgress` — run is executing
- `Completed` — run finished (check `conclusion` for result)

**`WorkflowRunConclusion` enum** with exit code semantics:
```rust
pub enum WorkflowRunConclusion {
    Success => exitcode::OK (0),
    Failure => exitcode::DATAERR (65),
    Canceled => exitcode::TEMPFAIL (75),
    Skipped => exitcode::OK (0),
    Neutral => exitcode::OK (0),
    TimedOut => exitcode::TEMPFAIL (75),
    ActionRequired => exitcode::DATAERR (65),
}
```

Exit code design:
- **0 (OK)** — workflow succeeded or was skipped (no action needed)
- **65 (DATAERR)** — workflow failed or requires manual action (permanent failure)
- **75 (TEMPFAIL)** — workflow was canceled or timed out (might be retryable)

This allows scripts/CI systems to distinguish between transient and permanent failures.

**`WorkflowRun` struct:**
```rust
pub struct WorkflowRun {
    pub id: u64,
    pub html_url: String,        // GitHub UI link
    pub status: WorkflowRunStatus,
    pub conclusion: Option<WorkflowRunConclusion>,
    pub name: String,
    pub created_at: String,
}
```

#### Key Functions

**`await_run(repo, run_id, auth_token) -> WorkflowRun`**
- Polls `GET /repos/{repo}/actions/runs/{run_id}` every 0.5 seconds
- Continues until status is `Completed`
- Max 7200 polls (~1 hour timeout)
- Returns the final run state with conclusion

**`get_run_id_after_dispatch(repo, workflow, ref, auth_token) -> u64`**
- GitHub dispatch API (POST `/dispatches`) returns 202 Accepted with no body
- This function fetches the latest queued run for the workflow/ref combo
- Used immediately after `spawn_workflow()` dispatch to get the ID for polling
- Queries `GET /repos/{repo}/actions/workflows/{workflow}/runs?branch={ref}&status=queued`

### Changes to `src/main.rs`

#### New Commands
- **`Run --await`**: dispatches workflow + polls until completion
- **`Await`**: standalone command to await a workflow execution by ID

#### Spawn Command Signature
```
gha spawn [OPTIONS] <WORKFLOW> [ARG]...
  --await  : Poll and wait for completion instead of exiting immediately
```

When `--await` is specified, after dispatching:
1. Call `get_run_id_after_dispatch()` to fetch the run ID from GitHub's run list
2. Call `await_run()` to poll until completion
3. Log the final conclusion and GitHub UI URL at INFO level
4. Exit with the appropriate code based on `WorkflowRunConclusion`

#### Await Command Signature
```
gha await [OPTIONS] --repo <owner/repo> <RUN_ID>
  --repo <owner/repo> : Required
  --token <TOKEN>     : Optional (resolves normally via --token, GITHUB_TOKEN, ~/.netrc)
```

Waits for an already-running workflow without the dispatch step.

### Logging

**DEBUG level:**
- Polling attempts and status ("Run is in progress...")
- Run ID extracted from dispatch response

**INFO level:**
- Final message: `"Workflow run completed: <CONCLUSION> — <GITHUB_UI_URL>"`
- Example: `"Workflow run completed: ✓ Success — https://github.com/myorg/myrepo/actions/runs/123456789"`

**TRACE level:**
- Individual API response bodies

### Exit Codes in Practice

```bash
# Succeed
$ gha spawn ci.yml --await && echo "Done" || echo "Failed"
# Exit code 0 if workflow succeeded

# Fail with DATAERR
$ gha spawn ci.yml --await
# ... workflow fails ...
# Exit code 65

# Canceled
$ gha spawn ci.yml --await
# ... user cancels via GitHub UI ...
# Exit code 75
```

### Test Coverage

- 4 unit tests in `src/awaiting.rs`:
  - Conclusion parsing from API responses
  - Exit code distinctness
  - Display string generation
  - Conclusion from API string variants
- Existing CLI/integration tests still pass (29 total)

### Design Decisions

1. **Shared Polling Logic**: Both `spawn --await` and `await` command reuse `await_run()` to avoid duplication and ensure consistent behavior.

2. **Run ID Retrieval**: After dispatch returns 202, we fetch the run list filtered by status=queued rather than trying to extract from the response (which is empty). This is reliable because GitHub returns the most recent run first.

3. **Polling Interval**: 0.5 seconds balances responsiveness with API rate limits. A typical workflow takes 30+ seconds, so this doesn't create excess polling.

4. **Timeout**: 7200 polls × 0.5s ≈ 1 hour. Most workflows complete well within this; the timeout prevents hung processes.

5. **Exit Code Mapping**: Used `exitcode` crate constants:
   - `exitcode::OK` (0) — standard success
   - `exitcode::TEMPFAIL` (75) — standard for temporary failure (canceled, timed out)
   - `exitcode::DATAERR` (65) — standard for data/execution error (workflow failed, action required)
   - Avoids custom exit codes and follows BSD sysexits conventions

6. **GitHub UI URL in Final Message**: Essential for user experience — users can click the link to see logs/details without searching for the run manually.

## Files Created

- `src/awaiting.rs` — polling/status types and functions

## Files Modified

- `src/main.rs` — new `await` command, `--await` flag on `spawn`, auth/conclusion handling
- `README.md` — documentation of `--await`, `await` command, exit codes, quick-start update

## Commits

1. Add Task 2: `--await` flag and `gha await` command with shared polling logic
2. Update README with `--await` and `gha await` documentation

## Known Limitations

1. **No Custom Timeout**: The 1-hour timeout is hardcoded. A future enhancement could add `--timeout` flag.

2. **Polling is Synchronous**: The polling blocks the CLI process. For non-blocking dispatch + wait patterns, users should use `gha spawn <workflow>` (exit immediately) then `gha await` later.

3. **No Rate Limiting**: We poll without respecting GitHub API rate limits. For high-frequency polling of many runs, this could hit rate limits. A future enhancement could implement exponential backoff or check rate limit headers.

4. **First Queued Run Assumption**: `get_run_id_after_dispatch()` assumes the first queued run is the one we just dispatched. If multiple runs are queued simultaneously on the same branch, this could be wrong. A future enhancement might store/pass the request timestamp to match more precisely.

## Exit Code Examples

```bash
# Success: exit 0
gha spawn ci.yml --await && echo "Workflow succeeded 🎉"

# Failure: exit 65
gha spawn ci.yml --await || [ $? -eq 65 ] && echo "Workflow failed (permanent)"

# Canceled: exit 75
gha spawn ci.yml --await || [ $? -eq 75 ] && echo "Workflow canceled (retryable)"

# Use in scripts
if gha spawn deploy.yml --await; then
  echo "Deployment successful"
else
  exit_code=$?
  if [ $exit_code -eq 75 ]; then
    echo "Deployment was canceled, retrying..."
    sleep 60
    gha spawn deploy.yml --await
  else
    echo "Deployment failed (exit code: $exit_code)"
    exit $exit_code
  fi
fi
```

## Future Enhancements

1. **`--timeout <SECONDS>`**: Allow customization of max polling time
2. **`--poll-interval <MS>`**: Allow customization of polling frequency
3. **`--output json`**: Return run state as JSON for machine parsing
4. **Webhook-based Waiting**: Use GitHub webhooks instead of polling (more efficient for long-running workflows)
5. **Live Log Streaming**: Include `--follow-logs` to stream workflow logs as they execute
6. **Artifact Download**: Auto-download artifacts when `--await` completes

