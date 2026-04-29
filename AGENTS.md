# AGENTS.md

## Project overview
- `gha` is a small Rust CLI for GitHub Actions work: it can dispatch a workflow directly and generate a Makefile client for `workflow_dispatch` workflows.
- The CLI entry point is `src/main.rs`; most nontrivial behavior fans out into `src/git_utils.rs`, `src/github_utils.rs`, and `src/gen_client.rs`.
- There were no existing repo-local AI instruction files or `README.md` files found in the requested search paths.

## Core flows to understand first
- `workflow-dispatch` / `wd` in `src/main.rs` resolves missing defaults from the local checkout before hitting GitHub:
  - repo from `git remote.origin.url` via `git_utils::default_repo_from_git`
  - ref from current branch, then fallback SHA, via `git_utils::default_ref_from_git`
  - workflow filename only when exactly one YAML exists under `.github/workflows` via `github_utils::default_workflow_from_dir`
- `gen-workflow-client` / `gen` in `src/main.rs` calls `gen_client::generate_makefile`, which:
  - scans a workflows directory for `.yml` / `.yaml`
  - parses only `on.workflow_dispatch` entries in `parse_workflow`
  - ignores `repository_dispatch` workflows with a warning
  - renders `src/template.Makefile` with Handlebars (no escaping) into a runnable Makefile

## Repo-specific conventions
- `.env` loading is automatic and happens before CLI parsing: `load_env_file()` walks upward from the current directory until `$HOME` / filesystem root and loads the first `.env` it finds.
- Logging goes to stderr via `tracing_subscriber`; `-v` enables debug and `-vv` enables trace. Keep stdout clean for commands that intentionally print machine-usable output (`--mode curl`, `--mode make`, `spawn`).
- `main()` uses `anyhow` for errors but converts outcomes into explicit `exitcode::*` values with `process::exit`; follow that pattern instead of letting user-facing command failures panic.
- Workflow inputs are passed as repeated `--arg` values in `name=value` or `name=@file` form; `@file` embeds file contents as a string in the JSON payload.
- In `gen_client.rs`, an input counts as required only when YAML says `required: true` **and** there is no default (`is_required = required && default.is_none()`).
- Generator special case: if the **first** workflow input is `type: choice`, the generator emits one Make target per option (see `build_render_model`).

## Key files to use as references
- `src/main.rs`: clap subcommands, `.env` behavior, dispatch modes (`curl`, `make`, `call`)
- `src/gen_client.rs`: workflow parsing, render model, Handlebars integration
- `src/template.Makefile`: source-of-truth template for generated clients
- `workflow_dispatch.Makefile`: checked-in example of generated client structure and runtime flow
- `tests/cli.rs`: expected CLI surface (`--help`, `spawn`, missing-command failure)
- `tests/reproduce_issue.rs`: YAML parsing edge cases the project explicitly cares about
- `tests/empty.yml`: minimal `workflow_dispatch` fixture used by parser tests

## Developer workflows
- Build/test: `cargo build`, `cargo test`
- Quick generator smoke test: `cargo run -- -v gen -d tests -o /tmp/gha-generated.mk`
- Local install shortcuts:
  - `make install-debug` symlinks `target/debug/gha` into `~/.cargo/bin/gha`
  - `make install` runs `cargo install --path .`
- `GNUmakefile` is a local convenience layer that includes generated Makefiles from `target/*.workflow_dispatch.Makefile`; its `make gen` target references a user-specific path, so treat it as an example, not a portable workflow.

## External dependencies and integration points
- Direct dispatch (`--mode call`) uses `reqwest` against `POST /repos/{repo}/actions/workflows/{workflow}/dispatches`.
- Default detection depends on the `git` CLI being available and run from inside a checkout.
- Generated Makefiles assume shell tools such as `curl`, `jq`, `unzip`, and OS-specific date handling (`date` on Linux, `date -j` on macOS via `$(OS)`).
- Authentication is expected via `GITHUB_TOKEN`; generated Makefiles also support `~/.netrc` when the token env var is absent.

## When changing behavior
- If you change workflow parsing or generated target structure, update both `src/gen_client.rs` and `src/template.Makefile`, then run `cargo test` and regenerate a sample Makefile to inspect the output.
- Prefer adding tests near the behavior style already used here: parser/unit coverage in `src/gen_client.rs` and CLI/integration coverage under `tests/`.

