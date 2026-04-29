# Task 3 - Configurable Wait Options

Implement `--timeout`, `--poll-interval`, and `--output json/human` options for both `gha spawn --await` and `gha await` commands.

## Requirements

1. **`--timeout <SECONDS>`**
   - Specify maximum wait time before aborting
   - Default: 3600 (1 hour)
   - Example: `gha spawn ci.yml --await --timeout 600` (10 minutes)
   - Applied to both `gha spawn --await` and `gha await`

2. **`--poll-interval <MS>`**
   - Control how often we query GitHub API
   - Default: 500 milliseconds
   - Useful for fast feedback loops or rate limiting
   - Example: `gha await --repo o/r 12345 --poll-interval 1000`

3. **`--output json/human`**
   - Output format for final result (when complete)
   - `human` (default): "Workflow run completed: ✓ Success — https://..."
   - `json`: Structured output with all run details
   - JSON format should include: id, name, status, conclusion, url, created_at
   - Example: `gha await --repo o/r 12345 --output json | jq '.conclusion'`

## Implementation

### Changes to src/awaiting.rs
- [x] Define `AwaitOptions` struct with fields: `timeout_secs`, `poll_interval_ms`, `output_format`
- [x] Define `OutputFormat` enum with variants: `Human`, `Json`
- [x] Update `await_run()` signature to accept `&AwaitOptions`
- [x] Update polling logic to use configurable timeout and interval
- [x] Add `WorkflowRun::to_json()` method

### Changes to src/main.rs
- [x] Add `--timeout <SECS>` flag to both Run and Wait commands
- [x] Add `--poll-interval <MS>` flag to both Run and Wait commands  
- [x] Add `--output <FORMAT>` flag to both Run and Wait commands
- [x] Parse these flags and build `AwaitOptions` struct
- [x] Pass options to `await_run()` calls
- [x] Output JSON before exit if `output_format == Json`

### Test Coverage
- [ ] Test timeout (mock slow API responses)
- [ ] Test custom poll interval
- [x] Test JSON output format parsing
- [x] Test output format validation

## Exit Codes
Exit codes remain unchanged - derived from `WorkflowRunConclusion` enum, not affected by output format.

## Example Commands

```bash
# Wait with 10 minute timeout
gha spawn ci.yml --await --timeout 600 env=prod

# Fast polling every 100ms
gha await --repo myorg/repo 123456 --poll-interval 100

# JSON output for parsing
gha await --repo myorg/repo 123456 --output json

# Custom timeout AND poll interval
gha spawn deploy.yml --await --timeout 1800 --poll-interval 250
```

## Notes for Implementation

- Token constraints mean we combine this into minimal CLI changes
- AwaitOptions struct is already defined with defaults in src/awaiting.rs
- Use `clap` value_enum or simple enum-to-string parsing for `--output`
- Make `--timeout` and `--poll-interval` u64 args parsed directly
- JSON serialization already added to WorkflowRun (to_json method)

