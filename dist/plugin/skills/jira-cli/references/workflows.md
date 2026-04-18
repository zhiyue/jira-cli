# Curated jira-cli workflows

Each section stands alone. Run commands in order. Subshells like
`$(…)` and `--file -` piping are bash/zsh idioms.

## W1. Read a single issue with just the fields you need

```bash
jira-cli issue get PROJ-123 \
  --jira-fields summary,status,assignee,customfield_10020 \
  --fields summary,status.name,assignee.displayName
```

- `--jira-fields` tells Jira which fields to serialize on the server.
- `--fields` is a client-side dot-path projection; nest with `.`, pick
  multiple with commas.

Add `auto_rename_custom_fields = true` to `jira.toml` and rerun — the
output replaces `customfield_10020` with a slug like `story_points`.

## W2. Triage every open issue assigned to me

```bash
# One-time alias in jira.toml:
#   [jql_aliases]
#   my-open = "assignee = currentUser() AND resolution = Unresolved"

jira-cli search @my-open --keys-only | \
  xargs -I{} jira-cli issue get {} --fields summary,priority,updated
```

Substitute `@my-open` with raw JQL if you do not want an alias:
`jira-cli search 'assignee = currentUser() AND resolution = Unresolved' --keys-only`.

## W3. File a bug with required custom fields

```bash
jira-cli issue create \
  --project PROJ \
  --type Bug \
  --summary "Login form regresses on Safari 18" \
  --set priority=High \
  --set labels=regression,ios \
  --component Auth \
  --component Frontend
```

`--type` and `--summary` are required flags. `--component` is repeatable
and maps to the `components` Jira field. Other fields use `--set
KEY=VALUE`, resolved via `[field_aliases]` in `jira.toml` — without
aliases use explicit ids like `--set priority.id=2`.

## W4. Move an issue through a transition

```bash
jira-cli issue transitions list PROJ-123         # discover valid names
jira-cli issue transition PROJ-123 --to "In Progress"
jira-cli issue transition PROJ-123 --to Done \
  --set resolution.name=Fixed
```

The transition name is passed with `--to`, not as a positional. Extra
field updates ride along via `--set` (e.g. resolution). `issue
transition` does **not** take `--comment`; if you also want to drop a
note, add a separate step:

```bash
jira-cli issue comment add PROJ-123 --body "Released in v2.4.0"
```

## W5. Parallel bulk actions

`bulk transition` and `bulk comment` read a JSONL file (one action per
line). Use `--file -` to stream from stdin.

```bash
# Close every issue matching a JQL alias
jira-cli search @stale-triage --keys-only \
  | awk '{printf "{\"key\":\"%s\",\"to\":\"Done\"}\n",$0}' \
  | jira-cli bulk transition --file -

# Post the same comment across an explicit key list
printf '%s\n' \
  '{"key":"PROJ-1","body":"Superseded by PROJ-42"}' \
  '{"key":"PROJ-2","body":"Superseded by PROJ-42"}' \
  '{"key":"PROJ-3","body":"Superseded by PROJ-42"}' \
  | jira-cli bulk comment --file -
```

Both commands fan out in parallel. Output is JSONL with per-key
success/failure; the process exits non-zero if any target failed.

## W6. Sprint operations

```bash
# Discover boards and active sprints
jira-cli board list
jira-cli sprint list --board 42 --state active

# Inspect a sprint's issues
jira-cli sprint issues 100 --keys-only

# Move issues into / out of a sprint
jira-cli sprint move 100 PROJ-10 PROJ-11     # <ID> <KEYS>… positional
jira-cli backlog move PROJ-12                # pull back to backlog
```

## W7. Epic children

```bash
jira-cli epic get PROJ-10
jira-cli epic issues PROJ-10 --keys-only
jira-cli epic add-issues PROJ-10 PROJ-21 PROJ-22
jira-cli epic remove-issues PROJ-10 PROJ-23
```

`add-issues` and `remove-issues` take the epic key plus space-separated
issue keys as positional arguments.

## W8. Attachments, worklogs, watchers

```bash
jira-cli issue attachment list PROJ-123
jira-cli issue attachment upload PROJ-123 ./screenshot.png
jira-cli issue attachment download PROJ-123 42 --out ./out.png

jira-cli issue worklog list PROJ-123
jira-cli issue worklog add PROJ-123 --time 2h --comment "pair on reproduction"

jira-cli issue watchers list PROJ-123
jira-cli issue watchers add PROJ-123 alice.smith
```

## W9. Raw API escape hatch

If no subcommand covers it, drop to `raw`:

```bash
jira-cli raw GET /rest/api/2/serverInfo
jira-cli raw POST /rest/api/2/issue/PROJ-123/comment \
  --data '{"body": "Released in v2.4.0"}'
```

Use `-d, --data` for the request body (JSON literal, `@file`, or `-`
for stdin). Response is the raw server body; exit codes still follow
the standard table (auth / not found / conflict / 5xx / network).
