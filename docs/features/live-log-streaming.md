# Feature: Live Log Streaming

Live log streaming prints run logs while awaiting completion.

## Key Rules

- enable with `--follow-logs`
- for `spawn`, `--follow-logs` requires `--await`
- works with both polling await and webhook await

## Examples

Spawn and stream logs:

```bash
gha spawn deploy.yml --await --follow-logs --repo myorg/myrepo -b main
```

Await an existing run and stream logs:

```bash
gha await --repo myorg/myrepo 123456789 --follow-logs
```

Stream logs with custom poll interval:

```bash
gha await --repo myorg/myrepo 123456789 --follow-logs --poll-interval 1000
```

## Output Style

Streaming output is line-based with:

- timestamps
- per-log-file color tags
- incremental updates as new lines arrive

