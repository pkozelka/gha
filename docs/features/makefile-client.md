# Feature: Makefile Client Generation

`gha gen` creates a standalone Makefile client for `workflow_dispatch` workflows.

## Why Use It

Use generated Makefiles when you want:

- repeatable shell-native workflow dispatch
- no `gha` runtime dependency for downstream users
- Make targets for dispatch/wait flows

## Generate

```bash
gha gen
```

Custom input/output paths:

```bash
gha gen -d .github/workflows -o ./workflow_dispatch.Makefile
```

## Use Generated Client

```bash
make -f workflow_dispatch.Makefile deploy
make -f workflow_dispatch.Makefile async-deploy
make -f workflow_dispatch.Makefile await
```

## Input Semantics

- required inputs (`required: true` with no default) get guard checks
- optional inputs are only sent when non-empty
- first input of type `choice` can produce per-option targets

## Notes

`repository_dispatch` workflows are skipped by generator logic.

