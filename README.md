# gha

`gha` is a Rust CLI for GitHub Actions workflows.

It supports:

- direct workflow dispatch and awaiting (`spawn`, `await`)
- artifact download (`artifacts`)
- generated Makefile clients for `workflow_dispatch` workflows (`gen`)
- shell completion generation (`completion`)

## Documentation

Detailed guides are under `docs/`:

- `docs/README.md` - docs index
- `docs/setup.md` - setup (base + optional feature-specific setup)
- `docs/reference/commands.md` - command reference
- `docs/reference/exit-codes.md` - exit code semantics

Feature and concept guides:

- `docs/concepts/spawn-vs-await.md`
- `docs/concepts/workflow-inputs.md`
- `docs/features/webhook-await.md`
- `docs/features/live-log-streaming.md`
- `docs/features/artifacts.md`
- `docs/features/makefile-client.md`
- `docs/features/shell-completions.md`

## Quick Start

```bash
cargo install --path .
export GITHUB_TOKEN=ghp_xxxx
gha spawn deploy.yml --await environment=staging
```

