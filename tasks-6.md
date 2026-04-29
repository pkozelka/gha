# Task 6 - Live Log Streaming (Branch: feature/log-streaming)

Stream workflow logs in real-time as the job executes.

## Overview

Instead of waiting silently for a workflow to complete, stream the logs to stdout as they're generated. This provides real-time feedback to the user running `gha spawn --await --follow-logs`.

## Requirements

1. **New `--follow-logs` flag**
   - `gha spawn ci.yml --await --follow-logs`
   - `gha await --repo o/r 123456 --follow-logs`
   - Only applicable with `--await` (synchronous mode)
   - Requires `--timeout` to be set appropriately

2. **Log fetching**
   - Once run starts, poll for job/step logs
   - GitHub API: `GET /repos/{repo}/actions/runs/{run_id}/jobs`
   - Then fetch logs for each job step as they execute
   - Stream to stdout in real-time

3. **Live output**
   - Print logs as they arrive
   - Timestamp each line
   - Color-code by job/step for readability
   - Show progress: `[step 1/5]`, etc.

4. **Integration with wait**
   - `await_run()` continues polling for completion
   - Parallel task streams logs
   - Both complete simultaneously
   - Exit code determined by final conclusion (as usual)

## Implementation (Branch feature/log-streaming)

New module `src/logs.rs`:
- `struct LogStream { run_id, jobs }`
- `async fn stream_job_logs(repo, run_id, auth, format) -> Result<()>`
- Fetch jobs from run, poll for log updates
- Pretty-print logs to stdout
- Return when all steps complete

Update `src/main.rs`:
- Add `--follow-logs` flag to Spawn and Await
- Spawn tokio task for log streaming if flag set
- Continue polling for completion in main task
- Join tasks before exit

## Notes

- Logs are streamed from GitHub Actions API on-demand
- GitHub API rate limits apply (but less restrictive than polling)
- Better UX for interactive use (see progress in real-time)
- Batch/script mode may not need this
- Consider using ANSI color codes for job names/steps

## Branch Management

- Create feature branch: `git checkout -b feature/log-streaming`
- Implement log streaming in this branch
- Can merge when ready for release
- Disabled by default (requires `--follow-logs` flag)

