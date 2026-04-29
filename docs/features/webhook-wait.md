# Feature: Webhook Wait

Webhook wait mode avoids high-frequency polling by waiting for GitHub `workflow_run` webhook events.

## When to Use

Use this for long-running workflows where frequent polling is undesirable.

## Required Setup

See `docs/setup.md` for webhook setup details.

Minimum requirements:

- `--webhook`
- `--webhook-secret` (or `GITHUB_WEBHOOK_SECRET`)
- repository webhook for `workflow_run` events

## Examples

Wait an existing run via webhook:

```bash
gha wait --repo myorg/myrepo 123456789 --webhook --webhook-secret "$GITHUB_WEBHOOK_SECRET"
```

Dispatch and wait via webhook:

```bash
gha run deploy.yml --wait --repo myorg/myrepo -b main --webhook --webhook-secret "$GITHUB_WEBHOOK_SECRET"
```

Use custom local port:

```bash
gha wait --repo myorg/myrepo 123456789 --webhook --webhook-port 4567 --webhook-secret "$GITHUB_WEBHOOK_SECRET"
```

## Notes

- Endpoint path is `/webhook/github`.
- Signature validation uses `X-Hub-Signature-256` HMAC-SHA256.
- Event must match the run id being awaited.

