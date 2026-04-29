# Task 3 Implementation Notes

## Decisions

- CLI parsing for `--output` is implemented with a typed clap `ValueEnum` (`human`/`json`) instead of manual string parsing.
- Wait configuration is centralized in `wait::WaitOptions` and shared by both `gha run --wait` and `gha wait`.
- Output handling is applied only to final run result:
  - `human`: logs a concise status line with run URL
  - `json`: prints structured JSON to stdout

## Why

- Typed enums improve completion quality and remove ad-hoc validation paths.
- Shared options avoid behavioral drift between `run --wait` and `wait`.
- Keeping JSON output on stdout preserves scriptability.

## Validation

- Integration tests validate help surface and output value validation (`tests/wait.rs`, `tests/run.rs`).
- Full `cargo test` passes.

