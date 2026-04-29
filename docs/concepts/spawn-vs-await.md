# Concept: Spawn vs Await

`gha` supports two execution styles:

- asynchronous dispatch (`gha spawn ...`)
- synchronous await (`gha spawn --await ...` or `gha await ...`)

## Asynchronous Dispatch

Use `gha spawn <WORKFLOW>` when you only need to trigger a workflow.

```bash
gha spawn deploy.yml --repo myorg/myrepo -b main environment=prod
```

Behavior:

- sends workflow dispatch request
- exits immediately

## Synchronous Dispatch + Await

Use `gha spawn --await` when the caller needs final run status.

```bash
gha spawn deploy.yml --await --repo myorg/myrepo -b main environment=prod
```

Behavior:

- dispatches workflow
- resolves run id
- awaits terminal state
- exits using conclusion-mapped exit code

## Await Existing Run

Use `gha await` when run id is already known.

```bash
gha await --repo myorg/myrepo 123456789
```

Useful for scripts that separate dispatch and status tracking.

## Await Options

Both await paths support:

- `--timeout <SECONDS>`
- `--poll-interval <MS>`
- `--output human|json`
- `--webhook` + webhook setup
- `--follow-logs`

Example:

```bash
gha await --repo myorg/myrepo 123456789 --output json --poll-interval 1000
```

