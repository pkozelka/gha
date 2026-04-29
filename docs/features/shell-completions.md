# Feature: Shell Completions

`gha completion <shell>` prints shell completion scripts to stdout.

Supported shells:

- `bash`
- `zsh`
- `fish`
- `powershell`
- `elvish`

## Examples

Zsh install:

```bash
mkdir -p ~/.zsh/completions
gha completion zsh > ~/.zsh/completions/_gha
```

Bash install:

```bash
gha completion bash >> ~/.bash_completion
source ~/.bash_completion
```

Fish install:

```bash
gha completion fish > ~/.config/fish/completions/gha.fish
```

## Choice Input Value Completion

For Bash and Zsh, completion output includes dynamic helpers that suggest values for `choice`-typed workflow inputs discovered in `.github/workflows` at generation time.

Regenerate completion scripts after workflow input changes.

