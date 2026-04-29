# Feature: Live Log Streaming

Live log streaming prints run logs while waiting.

## Key Rules

- enable with `--follow-logs`
- for `run`, `--follow-logs` requires `--wait`
- works with both polling wait and webhook wait

## Examples

Run and stream logs:

```bash
gha run deploy.yml --wait --follow-logs --repo myorg/myrepo -b main
```

Wait existing run and stream logs:

```bash
gha wait --repo myorg/myrepo 123456789 --follow-logs
```

Stream logs with custom poll interval:

```bash
gha wait --repo myorg/myrepo 123456789 --follow-logs --poll-interval 1000
```

## Output Style

Streaming output is line-based with:

- timestamps
- per-log-file color tags
- incremental updates as new lines arrive

