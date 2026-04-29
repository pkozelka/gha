# Concept: Workflow Inputs

`gha spawn` passes workflow inputs as trailing positional arguments.

## Input Formats

Supported forms:

- `name=value`
- `name=@file`

Examples:

```bash
gha spawn deploy.yml environment=prod version=1.2.3
```

```bash
gha spawn deploy.yml release_notes=@./notes.txt
```

In `@file` mode, file contents are read and sent as a string value.

## Common Patterns

### JSON payload as one input

```bash
gha spawn deploy.yml config=@./config.json
```

### Multi-input dispatch

```bash
gha spawn deploy.yml environment=staging version=2.0.0 dry_run=true
```

## Validation

Malformed inputs (missing `=`) are rejected.

Example that fails:

```bash
gha spawn deploy.yml invalid_input
```

## Completion for Choice Inputs

Generated Bash/Zsh completions can suggest values for `choice`-typed workflow inputs when completions are generated from a repository with those workflows.

