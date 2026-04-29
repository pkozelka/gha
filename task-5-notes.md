# Task 5 Implementation Notes

## Decisions

- Webhook waiting is implemented with an embedded Axum server bound to `127.0.0.1` and configurable port.
- Endpoint is fixed to `/webhook/github` and accepts only `workflow_run` events.
- HMAC-SHA256 validation is mandatory in webhook mode using `X-Hub-Signature-256` and a shared secret.
- `--webhook-secret` (or `GITHUB_WEBHOOK_SECRET`) is required when `--webhook` is set.
- Matching logic filters to terminal `action == completed` events and exact `workflow_run.id`.

## Why

- Mandatory signature verification avoids accepting forged events.
- Run-ID matching prevents cross-run contamination when multiple workflows emit events.

## Validation

- Routing and verification implemented in `src/webhook.rs`.
- `wait::wait_for_run()` routes to webhook mode when enabled.
- Full `cargo test` passes.

