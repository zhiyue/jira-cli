# jira-cli

Agent-first CLI for legacy **Jira Server 8.13.5**. Stateless, JSON-first, typed errors with stable exit codes, self-describing schema for capability discovery.

> Targets Jira Server/DC **8.13.5** specifically — no Atlassian Cloud support, no Personal Access Tokens (PAT only ships 8.14+). Uses Basic auth or cookie session (`/rest/auth/1/session`).

## Quick start

```bash
export JIRA_URL="https://jira.internal.example.com"
export JIRA_USER="alice"
export JIRA_PASSWORD="..."

jira-cli ping
jira-cli whoami
jira-cli issue get MGX-1 --pretty
jira-cli search "project = MGX AND status = Open" --max 50
jira-cli issue create -p MGX -t Task -s "Fix login" \
    --set 'Labels=["urgent"]' --set "Story Points=3"
jira-cli issue transition MGX-1 --to "In Progress"
jira-cli issue comment add MGX-1 --body "Deployed to staging."
jira-cli sprint move 100 MGX-1 MGX-2 MGX-3
```

## Configuration

Environment variables (no config file, no on-disk state):

| Var | Required | Notes |
|---|---|---|
| `JIRA_URL` | ✅ | Base URL |
| `JIRA_AUTH_METHOD` |  | `basic` (default) or `cookie` |
| `JIRA_USER` / `JIRA_PASSWORD` | basic | |
| `JIRA_SESSION_COOKIE` | cookie | e.g. `JSESSIONID=abc...` |
| `JIRA_TIMEOUT` |  | seconds, default 30 |
| `JIRA_INSECURE` |  | `1` to skip TLS verification |
| `JIRA_CONCURRENCY` |  | bulk worker count, default 4, max 16 |

## Cookie auth bootstrap

```bash
# one-time: produce the cookie, stash it in your shell
export JIRA_SESSION_COOKIE=$(
  JIRA_USER=alice JIRA_PASSWORD=... jira-cli session new | jq -r .cookie
)
export JIRA_AUTH_METHOD=cookie
```

## Field aliases

Jira often has multiple custom fields with the same display name (e.g. "Story Points"
across legacy projects). Pin the one you want in `~/.config/jira-cli/config.toml`:

~~~toml
[field_aliases]
"Story Points" = "customfield_10006"
"Epic Link" = "customfield_10000"
~~~

Or override ad-hoc per command: `--field-alias "Story Points=customfield_11322"` (repeatable).

## Agent capability discovery

```bash
jira-cli schema | jq '.commands | keys'
jira-cli schema issue | jq '.subcommands.get.args'
```

## Exit codes

| code | meaning |
|---|---|
| 0 | success |
| 2 | usage / config |
| 3 | Jira business error |
| 4 | network |
| 5 | auth (incl. CAPTCHA) |
| 6 | not found |
| 7 | internal / IO |

Errors go to stderr as JSON:

```json
{"error":{"kind":"not_found","message":"issue not found: MGX-42",
          "hint":"Verify the issue exists or check permissions with `jira-cli whoami`"}}
```

## Development

```bash
cargo fmt --all -- --check
cargo clippy --all-targets -- -D warnings
cargo test

# optional contract smoke test against a real Jira
JIRA_URL=... JIRA_USER=... JIRA_PASSWORD=... cargo test --test contract -- --ignored
```

## License

MIT OR Apache-2.0
