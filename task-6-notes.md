# Task 6 Implementation Notes

## Decisions

- Log streaming is implemented as a background task started by `run --wait --follow-logs` and `wait --follow-logs`.
- Stream source is the GitHub run logs archive endpoint (`/actions/runs/{run_id}/logs`), diffed incrementally by file/line offsets.
- Output includes timestamps and ANSI color tags per log file for readability.
- `--follow-logs` is rejected without `--wait` for `run`.

## Why

- Archive-diff streaming gives practical near-live output without requiring external tooling.
- Background task keeps wait conclusion logic unchanged and reusable.

## Validation

- Integration test added for `--follow-logs` usage guard (`tests/run.rs`).
- Full `cargo test` passes.

