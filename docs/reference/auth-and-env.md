# Auth and Environment Reference

## Authentication Priority

`gha` resolves GitHub authentication in this order:

1. `--token <TOKEN>`
2. `GITHUB_TOKEN`
3. `~/.netrc`

## `.netrc` Notes

Accepted machine names for token extraction:

- `api.github.com`
- `github.com`

Example:

```text
machine api.github.com
  login any
  password ghp_xxxx
```

## `.env` Loading

Before CLI parsing, `gha` searches for `.env` from current directory upward to `$HOME` and loads the first match.

Example project-level `.env`:

```bash
GITHUB_TOKEN=ghp_xxxx
```

Security tip: never commit real tokens.

