# Feature: Webhook Await

Webhook await mode avoids high-frequency polling by awaiting GitHub `workflow_run` webhook events.

## When to Use

Use this for long-running workflows where frequent polling is undesirable.

## Required Setup

See `docs/setup.md` for webhook setup details.

Minimum requirements:

- `--webhook`
- `--webhook-secret` (or `GITHUB_WEBHOOK_SECRET`)
- repository webhook for `workflow_run` events

## Examples

Await an existing run via webhook:

```bash
gha await --repo myorg/myrepo 123456789 --webhook --webhook-secret "$GITHUB_WEBHOOK_SECRET"
```

Spawn and await via webhook:

```bash
gha spawn deploy.yml --await --repo myorg/myrepo -b main --webhook --webhook-secret "$GITHUB_WEBHOOK_SECRET"
```

Use custom local port:

```bash
gha await --repo myorg/myrepo 123456789 --webhook --webhook-port 4567 --webhook-secret "$GITHUB_WEBHOOK_SECRET"
```

## Notes

- Endpoint path is `/webhook/github`.
- Signature validation uses `X-Hub-Signature-256` HMAC-SHA256.
- Event must match the run id being awaited.

