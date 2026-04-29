# Command Reference

## `spawn` (`s`)

```bash
gha spawn [OPTIONS] <WORKFLOW> [ARG]...
```

Key flags:

- `--repo <owner/repo>`
- `-b, --ref <REF>`
- `--token <TOKEN>`
- `--await`
- `--timeout <SECONDS>`
- `--poll-interval <MS>`
- `--output human|json`
- `--webhook`, `--webhook-port`, `--webhook-secret`
- `--follow-logs` (requires `--await`)

## `await` (`a`)

```bash
gha await [OPTIONS] <RUN_ID>
```

Key flags mirror await-related `spawn` flags.

- `--repo <owner/repo>` is optional and auto-detected from git when omitted
- `--base-dir <DIR>` controls where git detection runs

## `artifacts` (`art`)

```bash
gha artifacts [OPTIONS] --repo <owner/repo> <RUN_ID>
```

Key flags:

- `--output-dir <PATH>`
- `--filter <PATTERN>`
- `--token <TOKEN>`

## `gen-workflow-client` (`gen`)

```bash
gha gen-workflow-client [OPTIONS]
```

- `-d, --workflows-dir <DIR>`
- `-o, --output-file <FILE>`


## `completion` (`comp`)

```bash
gha completion <SHELL>
```

Generates shell completion script to stdout.

