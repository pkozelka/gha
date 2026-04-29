# Command Reference

## `run` (`r`)

```bash
gha run [OPTIONS] <WORKFLOW> [ARG]...
```

Key flags:

- `--repo <owner/repo>`
- `-b, --ref <REF>`
- `--token <TOKEN>`
- `--wait`
- `--timeout <SECONDS>`
- `--poll-interval <MS>`
- `--output human|json`
- `--webhook`, `--webhook-port`, `--webhook-secret`
- `--follow-logs` (requires `--wait`)

## `wait` (`w`)

```bash
gha wait [OPTIONS] --repo <owner/repo> <RUN_ID>
```

Key flags mirror wait-related `run` flags.

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

## `workflow-dispatch` (`wd`)

```bash
gha workflow-dispatch [OPTIONS]
```

- dispatch helper modes: `--mode curl|make|call`
- repeat input with `--arg name=value`

## `completion` (`comp`)

```bash
gha completion <SHELL>
```

Generates shell completion script to stdout.

