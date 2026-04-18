---
name: jira-cli
description: Manage Jira Server 8.13.5 via the `jira-cli` binary — read/create/update issues, run JQL searches, transition status, operate on sprints and epics, and perform parallel bulk actions. Trigger when the user mentions a Jira project key pattern (PROJ-123), asks about tickets, the backlog, sprint contents, JQL, or workflow transitions; or when they want to file/update/bulk-act on Jira issues. Requires `jira-cli` on PATH — the skill refuses with an install hint if missing.
---

# Jira (on-prem 8.13.5) via jira-cli

`jira-cli` is an agent-first Rust CLI for legacy Jira Server 8.13.5. It
speaks REST v2 + Agile 1.0, returns JSON on stdout, errors as JSON on
stderr, and exposes a `schema` command for capability discovery.

## 0. Preflight (always run first)

```bash
jira-cli --version
```

If the binary is not on PATH, stop and tell the user to install it:

```bash
brew install zhiyue/tap/jira-cli
# or
curl -sSL https://raw.githubusercontent.com/zhiyue/jira-cli/main/install.sh | sh
```

Do not attempt to run any other command until `--version` prints a line
like `jira-cli 0.2.2 (target=..., git=...)`.

Config lives at `~/.config/jira-cli/jira.toml`. Bootstrap with:

```bash
jira-cli config init
```

See `references/config.md` in this skill for the full config surface
(`JIRA_URL`, `JIRA_TOKEN`, `default_project`, `[jql_aliases]`,
`[field_aliases]`, `[field_renames]`, `auto_rename_custom_fields`,
`insecure`, `[defaults]`).

## 1. Discover capabilities before guessing flags

```bash
jira-cli schema                      # full command tree as JSON (default)
jira-cli schema <subcommand>         # just that subtree
jira-cli <command> --help            # per-command help
```

Use `schema` when unsure which subcommand or flag to reach for. The
output is stable machine-readable JSON designed for this kind of
introspection.

## 2. Curated workflows

Each scenario below has an expanded walkthrough in
`references/workflows.md`.

### Read an issue

```bash
jira-cli issue get PROJ-123 --fields=summary,status,assignee
```

Prefer `--fields=<dot.path,…>` to trim the response. Combine with
`--jira-fields` to select server-side fields first.

### Search with JQL

```bash
jira-cli search 'project = PROJ AND status != Done' --keys-only
jira-cli search @my-open --max 50       # @alias resolves via [jql_aliases]
```

`--keys-only` keeps the response a single line per hit — use it when
you only need issue keys for follow-up commands.

### Create an issue

```bash
jira-cli issue create \
  --project PROJ \
  --summary "Investigate flaky login test" \
  --set type=Bug --set priority=High
```

`--project` can be omitted if `default_project` is set in config or
`JIRA_PROJECT` is exported.

### Transition status

```bash
jira-cli issue transitions list PROJ-123
jira-cli issue transition PROJ-123 "In Progress"
```

### Bulk operations (parallel fan-out)

```bash
# bulk transition — feed a JSONL stream of {"key": "...", "to": "..."}
jira-cli search 'project=PROJ AND status=Review' --keys-only \
  | awk '{printf "{\"key\":\"%s\",\"to\":\"Done\"}\n",$0}' \
  | jira-cli bulk transition --file -

# bulk comment — each line: {"key": "...", "body": "..."}
printf '%s\n' \
  '{"key":"PROJ-1","body":"Released in v2.4.0"}' \
  '{"key":"PROJ-2","body":"Released in v2.4.0"}' \
  | jira-cli bulk comment --file -
```

### Sprint / epic

```bash
jira-cli sprint list --board 42
jira-cli sprint issues 100 --keys-only
jira-cli epic issues PROJ-10 --keys-only
```

## 3. Output discipline

- Prefer `--keys-only` / `--fields=…` to keep returned context lean.
- Exit codes are stable: **0** success, **2** usage, **3** auth, **4**
  not-found, **5** remote 5xx, **6** network, **7** conflict.
- Errors are JSON on stderr; parse `.error` and `.hint` for actionable
  diagnostics rather than scraping free text.

## 4. When something is off

- Auth failure (exit 3) → check `JIRA_TOKEN` is a current PAT and that
  the URL in `jira.toml` matches the PAT's tenant.
- Custom fields showing as `customfield_NNNNN` → set
  `auto_rename_custom_fields = true` in `jira.toml`, or add explicit
  `[field_renames]` / `[field_aliases]` entries. See `references/config.md`.
- TLS errors against self-signed instances → `insecure = true` in
  `jira.toml` (no stderr warning — explicit opt-in).
