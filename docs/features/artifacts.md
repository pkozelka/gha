# Feature: Artifacts

`gha artifacts` downloads and extracts artifacts from a workflow run.

## Command Shape

```bash
gha artifacts [OPTIONS] --repo <owner/repo> <RUN_ID>
```

## Common Examples

Download all artifacts:

```bash
gha artifacts --repo myorg/myrepo 123456
```

Download to custom directory:

```bash
gha artifacts --repo myorg/myrepo 123456 --output-dir ./outputs
```

Download subset with glob filter:

```bash
gha artifacts --repo myorg/myrepo 123456 --filter "test-*"
```

Use explicit token:

```bash
gha artifacts --repo myorg/myrepo 123456 --token "$GITHUB_TOKEN"
```

## Exit Behavior

- `0`: at least one artifact downloaded
- `65`: no matching artifacts or download/extract failure
- `70`: API/auth/internal error

