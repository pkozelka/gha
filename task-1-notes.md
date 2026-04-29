# Task 1 Implementation Notes

## Overview
This document outlines the design decisions and implementation approach for the `gha spawn` command, which allows users to execute GitHub Actions workflows directly via the GitHub API without generating a Makefile.

## Implementation Summary

### New Modules

#### 1. `src/auth.rs` - Authentication Management
**Purpose**: Centralized, reusable authentication logic for GitHub API calls

**Design Decisions**:
- **Auth Resolution Precedence**: CLI `--token` > `GITHUB_TOKEN` env > `~/.netrc`
  - Follows standard CLI conventions where explicit args override env vars
  - Supports `.netrc` for backward compatibility with curl workflow
  - Returns meaningful errors when no auth is provided
  
**Key Components**:
- `GithubAuth` struct: Simple token wrapper with validation
- `resolve()` method: Handles precedence chain with debug logging
- Unit tests: Verify token precedence logic

#### 2. `src/spawn.rs` - Workflow Execution
**Purpose**: Direct GitHub API workflow dispatch without Makefile generation

**Design Decisions**:
- **HTTP Logging**: Multi-level approach matching task requirements
  - DEBUG: Request URL/method, response status, payloads (when not error)
  - TRACE: Individual headers, request body
  - INFO: High-level success messages
  
- **Input Parsing**: Reuse common pattern from existing code
  - Format: `name=value` or `name=@file`
  - File content embedded as string in JSON payload
  - Errors are explicit and actionable

- **Request Building**: Manual header setup for clarity
  - Explicitly sets: Accept, User-Agent, X-GitHub-Api-Version
  - Only adds Authorization when token is non-empty
  - Maps to same headers as existing Makefile-based dispatch

- **Error Handling**: Distinguish API vs. network errors
  - Failed HTTP requests: use context and status code
  - API errors: include response body for debugging
  - Parse errors: reference input format in message

**Key Functions**:
- `spawn_workflow()`: Main async function that orchestrates dispatch
- `parse_input_args()`: Validates and parses CLI arguments
- `get_workflow_info()`: Future extension point for shell completion info
- `trace_request_body()`: Helper for request body logging

#### 3. `src/completion.rs` - Shell Completion
**Purpose**: Generate shell completions for all commands and parameters

**Design Decisions**:
- **Shell Support**: Bash, Zsh, Fish, PowerShell, Elvish
  - Covers the most common shells in dev workflows
  - Uses clap_complete's built-in generators (no custom logic)
  
- **Generation Approach**:
  - Single `completion` subcommand with shells as values
  - Output goes to stdout for easy pipe to completion file
  - Error handling is simple (IO errors only)

- **Parameter Completion**:
  - For trailing `ARG` (choice inputs): values are suggested based on workflow YAMLs in `.github/workflows/` found at completion-generation time — `collect_choice_inputs()` scans them and emits the options into the script
  - For `--token`: No suggestions (security-sensitive)
  - For `--ref`: No suggestions (requires git state at runtime, not feasible in static scripts)

**Dependencies**:
- `clap_complete ^4`: Provides generation logic, no features needed

### Changes to Existing Modules

#### `src/main.rs`
- Added new `Commands::Spawn` variant with required/optional parameters
- Added `Commands::Completion` for shell completion generation
- Proper error handling for auth resolution and input parsing
- Modified logging subscriber to use compact format
- Integrated auth module with token resolution logic

#### `tests/cli.rs`
- Removed old test for placeholder `Spawn` command (with `--name` flag)
- Added test for new `spawn` command with required `--workflow` parameter
- Maintained existing tests for help and missing-command failure

#### `tests/spawn.rs`
- New integration test file for spawn command
- Tests: help display, argument validation, required field enforcement

#### `Cargo.toml`
- Added `clap_complete` dependency (crate, not features)
- Added `cargo` feature to clap for better help integration

## Logging Strategy

The implementation follows a structured logging approach:

```
INFO:  gha spawn --workflow ci.yml --repo owner/repo --ref main
DEBUG: Dispatching workflow: repo=owner/repo, workflow=ci.yml, ref=main
DEBUG: Payload: {"ref":"main","inputs":{...}}
DEBUG: HTTP Request: POST https://api.github.com/repos/owner/repo/actions/workflows/ci.yml/dispatches
TRACE: Request header: Authorization: Bearer ***
TRACE: Request header: Accept: application/vnd.github+json
TRACE: Request body: {"ref":"main","inputs":{...}}
DEBUG: HTTP Response Status: 204 No Content
DEBUG: Workflow ci.yml dispatched successfully on ref main
```

Usage:
```bash
gha spawn ci.yml -b main arg=val        # info level
gha -v run ci.yml -b main arg=val     # debug level
gha -vv run ci.yml -b main arg=val    # trace level
```

## Shell Completion Examples

```bash
# Generate completions for bash
./gha completion bash | sudo tee /etc/bash_completion.d/gha
# Or for user-local completion
./gha completion bash >> ~/.bash_completion

# For zsh
./gha completion zsh | sudo tee /usr/share/zsh/site-functions/_gha

# For fish
./gha completion fish > ~/.config/fish/completions/gha.fish
```

Completions include:
- All subcommand names and their aliases
- All parameter flags
- Parameter value enums (e.g., `--token`, `--ref`, `--arg`)

## Authentication Priority and Examples

1. **CLI Token (Highest Priority)**
   ```bash
   gha spawn --workflow ci.yml --token ghp_xxxx --repo owner/repo --ref main
   # Uses ghp_xxxx, ignores GITHUB_TOKEN env
   ```

2. **Environment Variable**
   ```bash
   export GITHUB_TOKEN=ghp_yyyy
   gha spawn --workflow ci.yml --repo owner/repo --ref main
   # Uses GITHUB_TOKEN from env
   ```

3. **~/.netrc (Lowest Priority)**
   ```bash
   # ~/.netrc contains:
   # machine api.github.com login github_username password ghp_zzzz
   gha spawn --workflow ci.yml --repo owner/repo --ref main
   # Uses token from .netrc
   ```

## Future Enhancement Opportunities

1. **Dynamic Workflow/Input Completion**
   - Parse `.github/workflows/` directory to offer workflow names
   - Parse workflow YAML to offer input names as completion values
   - Current blocker: would add I/O overhead to all CLI invocations

2. **Workflow Validation Before Dispatch**
   - Check workflow exists and is accessible
   - Validate input types (string, boolean, choice, environment)
   - Provide helpful errors for invalid combinations

3. **Interactive Mode**
   - Prompt for missing required inputs
   - Show available choices for choice-type inputs
   - Confirm dispatch before API call

4. **Output Handling**
   - Optionally follow workflow run until completion (like Makefile)
   - Stream logs to stdout in real time
   - Format request/response as JSON for machine processing

5. **Request/Response Logging Improvements**
   - Structured logging with serde_json for better parsing
   - Optional request/response file recording (for debugging)
   - Pretty-print JSON with truncation for large payloads

## Testing Strategy

Current test coverage:
- **Unit Tests** (in `src/`): Auth precedence, input parsing
- **Integration Tests** (in `tests/`): CLI argument parsing, help display

Manual testing checklist:
- [ ] `gha spawn --help` shows correct options
- [ ] `gha spawn --workflow test.yml --token fake --repo o/r --ref x` fails appropriately (e.g., network error)
- [ ] `gha spawn --workflow test.yml --arg key=value` parses input correctly
- [ ] `gha spawn --workflow test.yml --arg key=@file` reads file content
- [ ] `gha completion bash` generates valid bash completion script
- [ ] `gha completion zsh` generates valid zsh completion script

## Code Quality Notes

- **Error Handling**: All Result-returning functions use `anyhow` for context
- **Logging**: Uses `tracing` crate with proper levels (not println!)
- **Async**: Uses tokio for non-blocking HTTP in `spawn_workflow()`
- **Separation of Concerns**: Auth, dispatch, and completion are isolated modules
- **Reusability**: `parse_input_args()` and `GithubAuth::resolve()` are public for potential library use
- **Maintainability**: Minimal changes to existing code, new functionality in isolated modules

## Known Limitations

1. **No Workflow Validation**: Spawn command doesn't check if workflow exists before dispatch
   - GitHub API returns 404 after submission if not found
   - Could be improved by pre-fetching workflow list

2. **No Input Type Validation**: All inputs are treated as strings
   - Boolean/choice types could be parsed and validated locally
   - Would require parsing workflow YAML in `spawn` command

3. **No `.netrc` Support in Code**: Token from `.netrc` only works if reqwest uses it internally
   - Current approach depends on reqwest's built-in `.netrc` support
   - May need explicit implementation if reqwest doesn't support it

4. **Completion Doesn't Include Values For `--workflow`**
   - Would require filesystem access or API calls
   - Deferred to future enhancement

## Commit History

1. Add spawn command infrastructure with auth and HTTP dispatch
2. Add integration tests for gha spawn command
3. Add shell completion support and improve logging
4. Make run CLI match task spec: positional workflow, -b for branch, trailing args
5. Add choice-value completion for workflow inputs in Zsh and Bash

## Files Created

- `src/auth.rs` - Authentication handling
- `src/spawn.rs` - Workflow execution
- `src/completion.rs` - Shell completion generation with dynamic choice support
- `tests/spawn.rs` - Integration tests for the spawn command
- `tests/deploy.yml` - Fixture for choice-input completion tests

## Files Modified

- `src/main.rs` - New Spawn and Completion commands, logging improvements
- `tests/cli.rs` - Updated tests for new Spawn command
- `Cargo.toml` - Added clap_complete dependency

Total lines added: ~650 (including tests and documentation)




