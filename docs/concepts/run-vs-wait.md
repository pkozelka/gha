# Concept: Run vs Wait

`gha` supports two execution styles:

- asynchronous dispatch (`gha run ...`)
- synchronous wait (`gha run --wait ...` or `gha wait ...`)

## Asynchronous Dispatch

Use `gha run <WORKFLOW>` when you only need to trigger a workflow.

```bash
gha run deploy.yml --repo myorg/myrepo -b main environment=prod
```

Behavior:

- sends workflow dispatch request
- exits immediately

## Synchronous Dispatch + Wait

Use `gha run --wait` when the caller needs final run status.

```bash
gha run deploy.yml --wait --repo myorg/myrepo -b main environment=prod
```

Behavior:

- dispatches workflow
- resolves run id
- waits until terminal state
- exits using conclusion-mapped exit code

## Wait Existing Run

Use `gha wait` when run id is already known.

```bash
gha wait --repo myorg/myrepo 123456789
```

Useful for scripts that separate dispatch and status tracking.

## Wait Options

Both wait paths support:

- `--timeout <SECONDS>`
- `--poll-interval <MS>`
- `--output human|json`
- `--webhook` + webhook setup
- `--follow-logs`

Example:

```bash
gha wait --repo myorg/myrepo 123456789 --output json --poll-interval 1000
```

