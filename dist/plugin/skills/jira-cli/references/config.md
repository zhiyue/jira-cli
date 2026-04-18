# jira-cli configuration reference

## Location

`~/.config/jira-cli/jira.toml` by default. Override with
`JIRA_CLI_CONFIG=/path/to/file`. Bootstrap with
`jira-cli config init` and then edit.

## Environment variables

| Variable          | Purpose                                                   |
| ----------------- | --------------------------------------------------------- |
| `JIRA_URL`        | Base URL of the Jira instance. Overrides `url` in TOML.   |
| `JIRA_TOKEN`      | Personal access token. Overrides `token` in TOML.         |
| `JIRA_USER`       | For Basic auth only (rare; PAT recommended).              |
| `JIRA_PROJECT`    | Default project key. Overrides `default_project`.         |
| `JIRA_CLI_CONFIG` | Config file path (see above).                             |

## Core TOML fields

```toml
url = "https://jira.internal.example.com"
token = "Bearer-PAT"                 # or: token_command = "op read …"
default_project = "PROJ"
insecure = false                     # accept self-signed TLS when true
auto_rename_custom_fields = false    # slugify customfield_* on output

[defaults]                           # per-command jira-fields default
search = { jira-fields = "summary,status,assignee" }
"issue get" = { jira-fields = "summary,status,priority,assignee,labels" }

[jql_aliases]
my-open       = "assignee = currentUser() AND resolution = Unresolved"
stale-triage  = "project = PROJ AND status = Triage AND updated < -30d"

[field_aliases]                      # --set <alias>=<value> → Jira field id
type       = "issuetype"
priority   = "priority"
components = "components"
labels     = "labels"
points     = "customfield_10020"     # story points on many instances

[field_renames]                      # customfield_* → readable slug on output
customfield_10020 = "story_points"
customfield_10041 = "epic_link"
```

## `auto_rename_custom_fields`

When `true`, `jira-cli` scans the server's `/rest/api/2/field` catalog
and derives a slug per custom field automatically. Collisions fall back
to the raw `customfield_NNNNN` id with a diagnostic line on stderr.
Prefer explicit `[field_renames]` entries if you care about exact
naming; use the auto flag as a quick start.

## `insecure`

Setting `insecure = true` disables TLS verification — useful for
self-signed internal Jira instances. Since `0.2.1` no stderr warning is
printed (the opt-in is explicit). Do not set in production.

## Auth strategies

1. **PAT in TOML** — simplest. `token = "NjA4..."`.
2. **PAT via secret manager** — `token_command = "op read 'op://Private/JIRA/token'"`
   (any command that prints the token to stdout).
3. **Basic auth** — `JIRA_USER` + password; only for the tiny cohort
   of instances that still require it.

## Verifying the config

```bash
jira-cli config show        # effective config (secrets redacted)
jira-cli ping               # connectivity check
jira-cli whoami             # verifies auth
```
