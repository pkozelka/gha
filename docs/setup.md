# Setup

This page is organized as:

1. base setup required for most usage
2. optional setup sections per feature

## Base Setup

### Install

```bash
git clone https://github.com/<owner>/gha.git
cd gha
cargo install --path .
```

### Authentication

`gha` resolves auth in this order:

1. `--token <TOKEN>`
2. `GITHUB_TOKEN` environment variable
3. `~/.netrc` (`machine api.github.com` or `machine github.com` + `password`)

Example:

```bash
export GITHUB_TOKEN=ghp_xxxx
```

### Optional `.env` Loading

`gha` looks for `.env` from current directory upward to `$HOME` and loads the first one it finds.

Example `.env`:

```bash
GITHUB_TOKEN=ghp_xxxx
```

## Optional Feature Setup

### Webhook Awaiting (`--webhook`)

Only needed if you use webhook mode (`gha await --webhook` or `gha spawn --await --webhook`).

Required:

- a shared secret (`--webhook-secret` or `GITHUB_WEBHOOK_SECRET`)
- GitHub repository webhook that sends `workflow_run` events
- reachable endpoint path: `/webhook/github`

Recommended env var:

```bash
export GITHUB_WEBHOOK_SECRET=replace-with-shared-secret
```

GitHub webhook settings:

- payload URL: `https://<your-host-or-tunnel>/webhook/github`
- content type: `application/json`
- secret: same value as `GITHUB_WEBHOOK_SECRET`
- events: `workflow_run`

### Live Log Streaming (`--follow-logs`)

No special setup required beyond normal API auth.

Usage (must include awaiting):

```bash
gha spawn deploy.yml --await --follow-logs
```

### Artifact Download (`artifacts`)

No extra setup beyond auth.

Optional: choose destination directory and glob filter.

```bash
gha artifacts --repo myorg/myrepo 123456 --output-dir ./outputs --filter "test-*"
```

### Generated Makefile Client (`gen`)

No extra setup required to generate.

To use generated Makefiles, ensure these tools are available on your machine:

- `curl`
- `jq`
- `unzip`

### Shell Completions (`completion`)

Generate completion scripts for your shell and install them in your shell-specific location.

Zsh example:

```bash
mkdir -p ~/.zsh/completions
gha completion zsh > ~/.zsh/completions/_gha
```

Bash example:

```bash
gha completion bash >> ~/.bash_completion
source ~/.bash_completion
```

