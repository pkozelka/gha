# Exit Codes

`gha` uses conventional process exit codes so scripts can react consistently.

## Codes

- `0` (`OK`): success, or run conclusion is success/skipped/neutral
- `1` (`USAGE`): invalid CLI usage
- `65` (`DATAERR`): workflow failure/action required, or artifact no-match/download failure
- `70` (`SOFTWARE`): internal/API/auth failure
- `75` (`TEMPFAIL`): canceled or timed-out workflow

## Script Examples

Treat canceled as retryable:

```bash
gha wait --repo myorg/myrepo 123456789
code=$?
if [ "$code" -eq 75 ]; then
  echo "Retrying after cancellation/timeout"
fi
```

Fail fast on permanent failures:

```bash
gha run deploy.yml --wait --repo myorg/myrepo -b main
code=$?
if [ "$code" -eq 65 ]; then
  echo "Workflow failed"
  exit "$code"
fi
```

