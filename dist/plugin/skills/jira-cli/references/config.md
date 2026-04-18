# jira-cli configuration reference

## Location

`~/.config/jira-cli/config.toml` by default (override the base with
`XDG_CONFIG_HOME`). The file is created mode 0600; `jira-cli` warns if
it finds looser permissions. Bootstrap with:

```bash
jira-cli config init
```

## Precedence

For any resolved value: **CLI flag > environment variable > config
file > built-in default**.

## Environment variables

| Variable                               | Purpose                                                            |
| -------------------------------------- | ------------------------------------------------------------------ |
| `JIRA_URL`                             | Base URL of the Jira instance. Required.                           |
| `JIRA_USER`, `JIRA_PASSWORD`           | Basic auth credentials (the default).                              |
| `JIRA_AUTH_METHOD=cookie`              | Switch to session-cookie auth.                                     |
| `JIRA_SESSION_COOKIE`                  | e.g. `JSESSIONID=abc123` — required under cookie auth.             |
| `JIRA_PROJECT`                         | Default project key. Overrides `default_project`.                  |
| `JIRA_TIMEOUT`                         | Per-request timeout in seconds.                                    |
| `JIRA_INSECURE`                        | `true` / `1` to skip TLS verification.                             |
| `JIRA_CONCURRENCY`                     | Worker count for parallel commands.                                |
| `XDG_CONFIG_HOME`                      | Overrides the config directory base.                               |

## Core TOML fields

```toml
url = "https://jira.internal.example.com"
user = "alice"
password = "your-password-or-PAT"    # PATs work wherever a password does
insecure = false
timeout_secs = 30
concurrency = 4

default_project = "PROJ"

# Cookie auth (optional; default is "basic"):
# auth_method    = "cookie"
# session_cookie = "JSESSIONID=abc123"

[defaults]
auto_rename_custom_fields = false     # slugify customfield_* on output
search_fields    = ["summary", "status", "priority", "assignee", "updated"]
issue_get_fields = [
  "summary", "status", "priority", "assignee",
  "reporter", "issuetype", "components", "labels",
  "created", "updated", "resolution", "description",
]

[field_aliases]                       # display name → Jira field id (write path, --set)
"Story Points" = "customfield_10006"

[field_renames]                       # customfield_* → readable key (read path, output)
customfield_10006 = "story_points"
customfield_11604 = "bug_url"

[jql_aliases]                         # `@alias` expansion for `jira-cli search`
mine_open = "assignee = currentUser() AND resolution = Unresolved"
critical  = "priority in (Highest, High) AND resolution = Unresolved"
```

## `auto_rename_custom_fields`

When `true`, `jira-cli` slugs every `customfield_*` key it returns
(e.g. `customfield_10006` → `story_points`). Collisions fall back to
the raw id with a stderr diagnostic. Use `[field_renames]` when you
want exact, hand-picked names; use the auto flag as a quick start.

## `insecure`

Setting `insecure = true` (or `JIRA_INSECURE=1`) disables TLS
verification — useful for self-signed internal Jira instances. Since
`0.2.1` no stderr warning is printed (the opt-in is explicit). Do not
use in production.

## Auth strategies

1. **Basic** — `user` + `password`. For tokens from Jira's PAT feature,
   plug the token into the `password` field; the server accepts a PAT
   wherever a password is expected.
2. **Session cookie** — `auth_method = "cookie"` +
   `session_cookie = "JSESSIONID=..."`. Useful when an SSO proxy won't
   accept Basic and you can grab the session from a browser.

## Verifying the config

```bash
jira-cli config show       # effective config (credentials redacted)
jira-cli ping              # connectivity probe (no auth required)
jira-cli whoami            # verifies auth
```
