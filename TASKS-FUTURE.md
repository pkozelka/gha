# Future Tasks Summary (Tasks 3-6)

This document outlines the four remaining enhancement tasks that were identified during the Task 2 Future Enhancements brainstorm.

## Overview

All four tasks have been fully planned and documented. The infrastructure foundation is in place for Tasks 3 and 4 (which will be implemented on the `master` branch). Tasks 5 and 6 are designed as separate feature branches due to their complexity and optional nature.

---

## Task 3: Configurable Wait Options ✓ INFRASTRUCTURE IN PLACE

**Documentation**: `tasks-3.md`  
**Status**: Foundation complete, awaiting CLI flag additions

### What's already done
- `WaitOptions` struct with `timeout_secs`, `poll_interval_ms`, `output_format` fields
- `OutputFormat` enum (Human, Json)
- `wait_for_run()` updated to accept `&WaitOptions`
- `WorkflowRun::to_json()` serialization method
- Default implementations and validation

### What remains
- Add `--timeout <SECS>` U64 arg to Run and Wait commands
- Add `--poll-interval <MS>` U64 arg to Run and Wait commands
- Add `--output <FORMAT>` flag to Run and Wait commands
- Update command handlers to build `WaitOptions` struct from CLI args
- Output JSON before exit if `output_format == Json`
- Tests for timeout behavior, intervals, and JSON output

### Example: After Task 3 completion
```bash
gha run ci.yml --wait --timeout 600 --output json | jq '.conclusion'
gha wait --repo myorg/repo 12345 --poll-interval 100 --output json
```

---

## Task 4: Artifact Download ✓ FULLY PLANNED

**Documentation**: `tasks-4.md`  
**Status**: Ready for implementation

### New command
```bash
gha artifacts --repo owner/repo RUN_ID [--output-dir DIR] [--filter PATTERN]
```

### Scope
- Single new module `src/artifacts.rs`
- Fetch artifact list from GitHub API
- Download each artifact ZIP
- Unzip to output directory
- Return exit code based on success

### Why separate from wait?
- Can be used independently (don't need to wait for completion first)
- Artifact download is a distinct operation
- Users might wait for run via `gha wait`, then later download artifacts

---

## Task 5: Webhook-Based Waiting ✓ FULLY PLANNED

**Documentation**: `tasks-5.md`  
**Branch**: `feature/webhook-wait`  
**Status**: Complete specification, no implementation yet

### Key insight
Polling every 500ms hits rate limits on large-scale deployments. Webhooks are event-driven and much more efficient for long-running workflows.

### Scope
- New module `src/webhook.rs` with server  
- `--webhook` flag (when GitHub webhook is configured)
- User configures GitHub webhook → points to `https://your-machine:3456/webhook/github`
- Workflow events trigger our server directly
- No polling needed

### Why a separate branch?
- Advanced feature, optional for users
- Testing requires real GitHub webhooks (not easy to mock)
- Can be merged when stable
- Polling remains the reliable default

### Example: After Task 5 completion
```bash
gha run ci.yml --wait --webhook    # Uses event-driven waiting instead of polling
```

---

## Task 6: Live Log Streaming ✓ FULLY PLANNED

**Documentation**: `tasks-6.md`  
**Branch**: `feature/log-streaming`  
**Status**: Complete specification, no implementation yet

### Key insight
Users running `gha run --wait` sit in silence. Streaming logs provides real-time feedback and better UX for long-running workflows.

### Scope
- New module `src/logs.rs`
- `--follow-logs` flag (requires `--wait`)
- Fetch job/step logs from GitHub API as they execute
- Color-code and timestamp output
- Show progress indicators

### Why a separate branch?
- Adds complexity with async log streaming
- Requires careful handling of log buffering/flushing
- Can be merged independently when ready
- Log streaming is "nice to have", not essential

### Example: After Task 6 completion
```bash
gha run deploy.yml --wait --follow-logs    # See logs in real-time
```

---

## Implementation Order Recommendation

1. **Task 3** (master branch, ~2-3 commits):
   - Highest impact with minimal complexity
   - All infrastructure is in place
   - Direct benefit to users (customizable waiting behavior)

2. **Task 4** (master branch, ~1-2 commits):
   - Useful utility command
   - Can be completely independent
   - Good for post-workflow operations

3. **Task 5** (feature/webhook-wait branch):
   - Advanced, optional optimization
   - Can be worked on after Tasks 3-4 stabilize
   - Only needed for scale/efficiency

4. **Task 6** (feature/log-streaming branch):
   - Enhances UX significantly
   - Higher complexity than Tasks 3-4
   - Can be worked on in parallel with Task 5

---

## Test Plan

### Task 3
- Unit tests for `WaitOptions` parsing
- Integration tests for timeout behavior
- JSON format validation tests

### Task 4
- Mock artifact download tests
- Verification of extraction logic

### Tasks 5 & 6
- These are advanced features
- Testing with real GitHub webhooks is manual
- Can be tested interactively as features mature

---

## Notes

- All four tasks are **non-breaking** additions
- Existing CLI behavior remains unchanged
- Defaults are sensible (human output, 1hr timeout, 500ms polling)
- Tasks 3 & 4 share master branch, Tasks 5 & 6 are feature branches
- Full backward compatibility maintained

---

## Files Organized By Task

### Task 3
- `tasks-3.md` (requirements and implementation guide)
- Modified `src/wait.rs` (already done ✓)
- Will modify: `src/main.rs` (add flags)

### Task 4
- `tasks-4.md` (requirements and implementation guide)
- Will create: `src/artifacts.rs` (new module)
- Will modify: `src/main.rs` (add command)

### Task 5
- `tasks-5.md` (requirements and implementation guide)
- Will create (on feature/webhook-wait): `src/webhook.rs`
- Will modify: `src/wait.rs` (webhook option in WaitOptions)

### Task 6
- `tasks-6.md` (requirements and implementation guide)
- Will create (on feature/log-streaming): `src/logs.rs`
- Will modify: `src/main.rs` (--follow-logs flag)

