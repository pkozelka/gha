# gha — GitHub Actions CLI

`gha` is a small Rust CLI for working with GitHub Actions workflows. It can:

- **Run** a `workflow_dispatch` workflow directly from the command line.
- **Generate** a self-contained Makefile client for any set of `workflow_dispatch` workflows — useful when you want a repeatable, shell-transparent way to dispatch and follow workflow runs.
- **Print** dispatch commands in `curl` or Makefile syntax (for copy-paste or scripting).

---

## Contents

- [Installation](#installation)
- [Authentication](#authentication)
- [Shell Completions](#shell-completions)
- [Commands](#commands)
  - [run](#run)
  - [wait](#wait)
  - [gen-workflow-client](#gen-workflow-client)
  - [workflow-dispatch](#workflow-dispatch)
  - [completion](#completion)
- [Verbose Logging](#verbose-logging)
- [Exit Codes](#exit-codes)
- [Environment & `.env` Files](#environment--env-files)

---

## Installation

### From source (recommended)

```bash
git clone https://github.com/<owner>/gha.git
cd gha
cargo install --path .
```

This places `gha` in `~/.cargo/bin/`. Make sure that directory is on your `PATH`.

### Local debug build (for development)

```bash
cargo build
# Symlink debug binary into ~/.cargo/bin:
make install-debug
```

### Requirements

- Rust 1.80 or later (`rustup update stable`)
- `git` must be on `PATH` (used to auto-detect `--repo` and `--ref` from the current checkout)
- For generated Makefiles: `curl`, `jq`, `unzip`

---

## Authentication

All commands that reach the GitHub API need a token with `actions:write` (and `repo`) scope.

Authentication is resolved in this priority order:

| Priority | Source | How to set |
|----------|--------|------------|
| 1 (highest) | `--token <TOKEN>` CLI flag | Pass directly on the command line |
| 2 | `GITHUB_TOKEN` environment variable | `export GITHUB_TOKEN=ghp_…` |
| 3 (lowest) | `~/.netrc` | See example below |

**`~/.netrc` example:**
```
machine api.github.com
  login anyone
  password ghp_XXXXX
```

> When `--token` is supplied it always wins, even if `GITHUB_TOKEN` is also set.

---

## Shell Completions

`gha` can generate completion scripts for Bash, Zsh, Fish, PowerShell, and Elvish.

For workflows that have `choice`-typed inputs, the generated Zsh and Bash scripts also include completion of the valid option *values* (e.g. `environment=` → `dev`, `staging`, `prod`). The options are baked into the script from the `.github/workflows/` directory at generation time, so run `gha completion` again whenever you add or change workflow inputs.

### Zsh

```zsh
# One-time setup (add this line to ~/.zshrc for the shortcut):
gha completion zsh > "${fpath[1]}/_gha"

# Reload completions without restarting the shell:
autoload -Uz compinit && compinit
```

Or to a user-local location:
```zsh
mkdir -p ~/.zsh/completions
gha completion zsh > ~/.zsh/completions/_gha
# Make sure ~/.zsh/completions is on fpath (add to ~/.zshrc):
fpath=(~/.zsh/completions $fpath)
autoload -Uz compinit && compinit
```

### Bash

```bash
gha completion bash | sudo tee /etc/bash_completion.d/gha > /dev/null
# Or for a single user:
gha completion bash >> ~/.bash_completion
source ~/.bash_completion
```

### Fish

```fish
gha completion fish > ~/.config/fish/completions/gha.fish
```

### PowerShell

```powershell
gha completion powershell | Out-String | Invoke-Expression
# To persist, add the above line to your $PROFILE.
```

### Elvish

```elvish
gha completion elvish >> ~/.elvish/rc.elv
```

---

## Commands

### `run`

Alias: `r`

Dispatches a `workflow_dispatch` workflow directly via the GitHub API and exits. This is the simplest way to trigger a workflow without generating any files.

```
gha run [OPTIONS] <WORKFLOW> [ARG]...
```

| Argument / Option | Description |
|-------------------|-------------|
| `<WORKFLOW>` | Workflow filename or ID, e.g. `ci.yml` (**required, positional**) |
| `[ARG]...` | Input arguments, one per input, in `name=value` or `name=@file` form |
| `--repo <owner/repo>` | GitHub repository. Auto-detected from `git remote.origin.url` if omitted. |
| `-b`, `--ref <REF>` | Branch, tag, or SHA. Auto-detected from current branch / HEAD if omitted. |
| `--token <TOKEN>` | GitHub token (overrides `GITHUB_TOKEN` env and `~/.netrc`) |
| `--base-dir <DIR>` | Directory used for git-based auto-detection (default: `.`) |

**Examples:**

```bash
# Minimal — auto-detect repo and ref from git
gha run deploy.yml environment=prod

# Specify everything explicitly
gha run deploy.yml \
  --repo myorg/myrepo \
  -b main \
  --token ghp_xxxx \
  environment=staging version=1.2.3

# Read an input value from a file
gha run deploy.yml config=@./config.json

# With GITHUB_TOKEN already set in the environment
export GITHUB_TOKEN=ghp_xxxx
gha run ci.yml

# Wait for the workflow to complete
gha run deploy.yml --wait environment=prod

# Wait with custom polling (will exit with non-zero if workflow fails)
gha run deploy.yml --wait --repo myorg/myrepo -b main
```

**Input argument format:**

- `name=value` — passes `value` as a string.
- `name=@path/to/file` — reads the file and passes its content as the value string.

**`--wait` mode:**

When `--wait` is specified, `gha run` will:
1. Dispatch the workflow
2. Poll the GitHub API until the run completes
3. Print the final conclusion (Success, Failure, Canceled, etc.) with the GitHub UI URL
4. Exit with a status code reflecting the conclusion:
   - `0` if the workflow succeeded or was skipped
   - `65` (dataerr) if the workflow failed or requires action
   - `75` (tempfail) if the workflow was canceled or timed out

---

### `wait`

Alias: `w`

Waits for an already-dispatched workflow run to complete. Useful when you want to dispatch a workflow asynchronously and then wait for it later.

```
gha wait [OPTIONS] --repo <owner/repo> <RUN_ID>
```

| Argument / Option | Description |
|-------------------|-------------|
| `<RUN_ID>` | Workflow run ID (required, positional) |
| `--repo <owner/repo>` | GitHub repository (required) |
| `--token <TOKEN>` | GitHub token (overrides `GITHUB_TOKEN` env and `~/.netrc`) |

**Examples:**

```bash
# Wait for a specific run (assuming you know the run ID from a previous dispatch)
gha wait --repo myorg/myrepo 12345678

# With explicit authentication
gha wait --repo myorg/myrepo 12345678 --token ghp_xxxx

# Common exit codes:
#   0  -> success
#  65  -> failure
#  75  -> canceled
```

Exit codes:
- `0` — Success or Skipped
- `65` — Failure or Action Required
- `75` — Canceled or Timed Out

---

### `gen-workflow-client`

Alias: `gen`

Scans a directory for `workflow_dispatch` workflow YAML files and generates a standalone Makefile client. The Makefile lets you dispatch workflows, wait for completion, download logs and artifacts, and check the final conclusion — all via `curl` and `jq`, without needing `gha` at runtime.

```
gha gen-workflow-client [OPTIONS]
```

| Option | Description | Default |
|--------|-------------|---------|
| `-d`, `--workflows-dir` | Directory containing workflow YAML files | `.github/workflows` |
| `-o`, `--output-file` | Path to write the generated Makefile | `workflow_dispatch.Makefile` |

**Examples:**

```bash
# Generate from the standard location
gha gen

# Custom directories
gha gen -d path/to/workflows -o target/dispatch.Makefile

# Using the generated Makefile (example targets depend on workflow names):
make -f workflow_dispatch.Makefile deploy          # dispatch + wait
make -f workflow_dispatch.Makefile async-deploy    # dispatch only (don't wait)
make -f workflow_dispatch.Makefile await           # wait for last dispatched run
make -f workflow_dispatch.Makefile await-all       # wait for all recent runs
```

**How generated inputs work:**

- Required inputs (YAML `required: true` with no default) produce a `test -n` guard.
- Optional inputs are passed to the API only when the corresponding Make variable is non-empty.
- `choice`-typed inputs where the first input drives per-option targets get one `make` target per option value.
- Override an input at dispatch time: `make deploy VERSION=1.2.3`.

**Notes on workflow types:**
- `workflow_dispatch` workflows are fully supported.
- `repository_dispatch` workflows are skipped with a warning.

---

### `workflow-dispatch`

Alias: `wd`

Lower-level dispatch command. Mainly useful for printing dispatch commands in `curl` or Makefile syntax, or for scripting. For interactive use, prefer [`run`](#run).

```
gha workflow-dispatch [OPTIONS] --token <TOKEN>
```

| Option | Description | Default |
|--------|-------------|---------|
| `--repo <owner/repo>` | GitHub repository | Auto-detected |
| `--workflow <FILE>` | Workflow filename | Auto-detected if exactly one exists |
| `--ref <REF>` | Branch or tag | Auto-detected |
| `--token <TOKEN>` | GitHub token — **required** (also via `GITHUB_TOKEN` env) | — |
| `--arg <name=value>` | Input argument (repeatable) | — |
| `--mode <MODE>` | Output mode: `curl`, `make`, or `call` | `curl` |
| `--base-dir <DIR>` | Base dir for git auto-detection | `.` |

**Modes:**

| Mode | Behaviour |
|------|-----------|
| `curl` | Prints a ready-to-run `curl` command to stdout |
| `make` | Prints the dispatch recipe in Makefile tab-indented syntax |
| `call` | Executes the dispatch directly (same as `gha run --mode call`) |

**Examples:**

```bash
# Print a curl command to stdout
gha wd --workflow deploy.yml --repo myorg/myrepo --ref main \
  --token ghp_xxxx --arg environment=prod

# Execute directly
gha wd --workflow deploy.yml --mode call

# Print Makefile recipe
gha wd --workflow deploy.yml --mode make
```

---

### `completion`

Alias: `comp`

Generates a shell completion script for `gha` and writes it to stdout.

```
gha completion <SHELL>
```

`<SHELL>` is one of: `bash`, `zsh`, `fish`, `powershell`, `elvish`.

See [Shell Completions](#shell-completions) for per-shell installation instructions.

```bash
# Quick check — see what the zsh script contains
gha completion zsh | head -30
```

---

## Exit Codes

`gha` uses the following exit codes to indicate the outcome of workflow runs:

| Code | Name | Meaning |
|------|------|---------|
| `0` | OK | Workflow succeeded or was skipped |
| `1` | USAGE | Missing or invalid arguments |
| `65` | DATAERR | Workflow failed, action required, or malformed data |
| `70` | SOFTWARE | Internal error (authentication failure, API error, etc.) |
| `75` | TEMPFAIL | Workflow was canceled or timed out |

These codes allow scripts to distinguish between transient failures (retry-able) and permanent failures (non-retry-able).

---

## Verbose Logging

All log output goes to **stderr** so that stdout remains clean for machine-readable output (curl command text, generated Makefile content, etc.).

| Flag | Log level |
|------|-----------|
| *(none)* | `INFO` — significant events (dispatch success, errors) |
| `-v` | `DEBUG` — HTTP request URLs, response status codes, payloads |
| `-vv` | `TRACE` — individual request/response headers, request body |

```bash
# See the full HTTP exchange
gha -vv run deploy.yml environment=prod
```

---

## Environment & `.env` Files

Before CLI argument parsing, `gha` automatically searches for a `.env` file. It starts from the current working directory and walks upward toward `$HOME`, loading the first `.env` it finds.

This means you can place a project-level `.env` alongside your workflow files:

```bash
# .env
GITHUB_TOKEN=ghp_xxxx
REPO=myorg/myrepo
```

The file is silently skipped if not found; no error is produced.

> **Security note:** never commit `.env` files containing real tokens. Add `.env` to `.gitignore`.

---

## Quick-start example

```bash
# 1. Install
cargo install --path .

# 2. Set up auth
export GITHUB_TOKEN=ghp_xxxx

# 3. Run a workflow on the current branch (repo and ref auto-detected)
gha run deploy.yml environment=staging version=1.0.0

# 4. Run with waiting (blocks until completion, exit code reflects result)
gha run deploy.yml --wait environment=staging

# 5. Or dispatch asynchronously, then wait later
gha run deploy.yml environment=staging  # get run ID from output
# ... do other work ...
gha wait --repo myorg/myrepo 123456789

# 6. Generate a Makefile client for all workflows in this repo
gha gen

# 7. Set up shell completions (zsh example)
gha completion zsh > ~/.zsh/completions/_gha
```

