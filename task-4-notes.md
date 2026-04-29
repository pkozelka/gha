# Task 4 Implementation Notes

## Decisions

- Artifact filtering uses glob patterns (`glob::Pattern`) so inputs like `test-*` match task expectations.
- Artifact execution returns typed errors (`ArtifactError`) to map command exit codes precisely:
  - `Api` -> `70`
  - `NoArtifacts` / `Download` -> `65`
- Download summary (`ArtifactSummary`) is returned to allow concise reporting in `main.rs`.

## Why

- Pattern semantics are clearer than substring matching.
- Explicit error kinds make exit-code behavior stable and testable.

## Validation

- Command wiring and error mapping implemented in `src/main.rs` and `src/artifacts.rs`.
- Full `cargo test` passes.

