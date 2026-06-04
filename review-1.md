# Review 1: Refactor and UX Recommendations

## Findings (ordered by severity)

### High

1. `spawn` silently ignores await-related flags without `--await`
   - **Where:** `src/main.rs` (`Commands::Spawn` branch, around lines 281-433)
   - **Problem:** `--timeout`, `--poll-interval`, `--output`, `--webhook`, and `--webhook-secret` are accepted for `spawn`, but if `--await` is not set they are ignored with no warning.
   - **Impact:** confusing UX and easy user mistakes (command appears accepted but behavior is different).
   - **Recommendation:** use clap constraints (`requires = "await"` or `requires_if`) for await-only flags on `spawn`, and reject invalid combinations at parse time.

2. Async `spawn` mode does not return or print run id
   - **Where:** `src/main.rs` (`Spawn` branch around lines 361-433), `src/awaiting.rs` (`get_run_id_after_dispatch`)
   - **Problem:** without `--await`, users cannot reliably chain to `gha await` because no run id is surfaced.
   - **Impact:** breaks common async workflow (`spawn` now, `await` later) and scriptability.
   - **Recommendation:** add `--output json|human` support for async mode with at least run id + URL, or add explicit `--print-run-id` flag; fetch run id after dispatch and print to stdout.

3. `get_run_id_after_dispatch()` is race-prone
   - **Where:** `src/awaiting.rs` lines ~291-352
   - **Problem:** picks first queued run for workflow/ref (`status=queued&per_page=1`) and assumes it is the newly spawned run.
   - **Impact:** wrong run tracking when multiple dispatches happen close together.
   - **Recommendation:** match on additional fields (created_at >= dispatch timestamp, actor, head_sha if available), retry for short window, and fail with clear diagnostics if ambiguous.

### Medium

4. `main.rs` command handlers are too large and duplicated
   - **Where:** `src/main.rs` entire file (especially `Spawn` and `Await` branches)
   - **Problem:** large match arms with repeated auth/options/output/error handling reduce maintainability.
   - **Recommendation:** extract helpers:
   - `resolve_repo_and_ref(...)`
   - `build_await_options(...)`
   - `render_final_run_output(...)`
   - `maybe_start_log_stream(...)`
   - `execute_spawn(...)`, `execute_await(...)`

5. Bash dynamic completion helper seems incomplete
   - **Where:** `src/completion.rs` lines ~117-146
   - **Problem:** `_gha_spawn_args_complete` is generated but not clearly wired into the generated bash subcommand completion path.
   - **Impact:** likely mismatch between intended and actual completion behavior for `spawn ARG` values.
   - **Recommendation:** patch the generated bash completion function specifically for `spawn` argument position (similar to zsh override), and add integration tests that execute completion functions.

6. Artifact filter silently degrades on invalid glob
   - **Where:** `src/artifacts.rs` line ~76
   - **Problem:** `glob::Pattern::new(p).ok()` drops parse errors and acts like no filter.
   - **Impact:** surprising downloads when filter typo exists.
   - **Recommendation:** return `ArtifactError::Download` (or dedicated `InvalidFilter`) when glob parsing fails.

7. Log streaming suppresses archive fetch failures
   - **Where:** `src/logs.rs` lines ~45-66
   - **Problem:** `if let Ok(log_files)` ignores errors silently.
   - **Impact:** user sees missing logs without explanation.
   - **Recommendation:** log warnings at least once per failure category (auth/network/rate-limit), optionally backoff.

### Low

8. Minor readability issues and naming drift
   - **Where:** `src/main.rs` line 6 (`use std::fs;mod git_utils;`), comments/messages still use mixed run/await wording in some places.
   - **Recommendation:** run formatting/lint pass and keep terminology consistently as "spawn/await" for CLI and "workflow run" for GitHub entity.

9. `process::exit` deep inside branches reduces testability
   - **Where:** `src/main.rs`
   - **Recommendation:** return `Result<i32>` from command executors and call `process::exit` once in `main`.

## Subcommand UX Structure Review

### Keep

- `spawn` as primary dispatch command
- `await` as dedicated synchronization command
- `artifacts`, `gen-workflow-client`, `workflow-dispatch`, `completion`

### Change

1. Enforce flag intent with clap relationships
   - On `spawn`, make await-specific flags depend on `--await`:
   - `--timeout`, `--poll-interval`, `--output`, `--webhook`, `--webhook-port`, `--webhook-secret`, `--follow-logs`

2. Improve async chaining UX
   - Add async output mode for `spawn` to print run metadata (`id`, `url`, `workflow`, `ref`).

3. Improve `await` ergonomics
   - Consider optional `--repo` with git auto-detect + `--base-dir`, matching `spawn` behavior.

4. Clarify advanced command purpose
   - `workflow-dispatch` overlaps with `spawn`; mark as advanced/legacy in help text and docs.

### Add

- `spawn --dry-run` (print effective request payload without calling API)
- `spawn --open` or `await --open` (open GitHub run URL after completion)
- `await --until <queued|in_progress|completed>` (optional advanced control)

### Consider Removing / Hiding

- If overlap is too high, hide `workflow-dispatch` from primary docs and keep as expert command.

## Suggested Refactor Plan (incremental)

1. **Phase 1 (safe UX fixes):** clap constraints + invalid filter error + explicit log warnings.
2. **Phase 2 (async UX):** expose run id/url from async `spawn`.
3. **Phase 3 (structure):** split `main.rs` handlers into dedicated functions/modules.
4. **Phase 4 (completion hardening):** bash completion wiring tests and behavior fix.

## Open Questions

1. Should async `spawn` default output be human text, JSON, or remain silent unless opted in?
2. Should `await` auto-detect repo by default for parity with `spawn`?
3. Is `workflow-dispatch` intended as public API long-term or backward-compatibility bridge?

