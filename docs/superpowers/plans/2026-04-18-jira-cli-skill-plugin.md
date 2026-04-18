# jira-cli Skill + Claude Code Plugin Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Ship a reusable "skill" for the `jira-cli` binary so Claude Code users can `/plugin install` it and Codex CLI users can drop the same `SKILL.md` into `~/.agents/skills/` — a single source of truth, auto-triggered on Jira-shaped conversations, with a runtime check that prints an install hint if the binary is missing.

**Architecture:** The repo becomes its own Claude Code marketplace — a root `.claude-plugin/marketplace.json` points at a `dist/plugin/` subdirectory that holds the plugin manifest and a single `skills/jira-cli/SKILL.md` (curated workflows + references). Codex users symlink or `$skill-installer` that same skill directory into `~/.agents/skills/jira-cli/`. A preflight shell script detects the binary on session start.

**Tech Stack:** Markdown (skill + references), JSON (manifests), Bash (preflight script), `claude plugin validate` for linting, `shellcheck` / `bash -n` for script sanity. No build step, no runtime dependencies added to the Rust crate.

**Reference spec:** [`docs/superpowers/specs/2026-04-18-jira-cli-skill-plugin-design.md`](../specs/2026-04-18-jira-cli-skill-plugin-design.md) — plan embeds full contents; spec is the source of truth for rationale and decisions.

**Authoring conventions for this plan:**
- Each Task has explicit `Files:` (Create / Modify) and ordered `- [ ]` Steps.
- Every Step that writes content embeds the full final content (no "fill in" placeholders).
- Verification per Task is a concrete command with expected output.
- One commit per Task, using this repo's existing conventional-commit style (`feat(plugin)`, `docs(plugin)`, `test(plugin)`, `chore(plugin)`). `plugin` is the scope label for all Task-level commits in this plan.

---

## File structure

New files (all under `/Users/zhiyue/workspace/jira-cli`):

```
.claude-plugin/
└── marketplace.json                              # Task 1 — root marketplace listing

dist/plugin/
├── .claude-plugin/
│   └── plugin.json                               # Task 2 — Claude Code plugin manifest
├── scripts/
│   └── check.sh                                  # Task 3 — jira-cli presence probe
├── hooks/
│   └── hooks.json                                # Task 4 — SessionStart → check.sh
├── skills/
│   └── jira-cli/
│       ├── SKILL.md                              # Task 5 — main skill content
│       └── references/
│           ├── workflows.md                      # Task 6 — curated scenarios
│           └── config.md                         # Task 7 — jira.toml / env / aliases
└── README.md                                     # Task 8 — install notes for CC + Codex

docs/RELEASING.md                                 # Task 9 — modify: plugin version sync
CHANGELOG.md                                      # Task 10 — add entry for the plugin
```

One directory sits inside `dist/plugin/`: `skills/jira-cli/`. Codex users symlink *that* path (not `dist/plugin/` itself) into `~/.agents/skills/jira-cli/`, which is why it's a free-standing subdirectory with no Claude-Code-specific files inside it.

---

## Task 1: Root marketplace manifest

**Files:**
- Create: `.claude-plugin/marketplace.json`

- [ ] **Step 1: Create the marketplace directory**

```bash
mkdir -p .claude-plugin
```

- [ ] **Step 2: Write `.claude-plugin/marketplace.json`**

```json
{
  "name": "jira-cli",
  "description": "Marketplace for the jira-cli agent skill.",
  "owner": {
    "name": "zhiyue",
    "url": "https://github.com/zhiyue"
  },
  "plugins": [
    {
      "name": "jira-cli",
      "version": "0.2.2",
      "description": "Agent skill wrapping jira-cli — Jira Server 8.13.5 issues, JQL, sprints, epics, bulk ops.",
      "source": "./dist/plugin",
      "author": {
        "name": "zhiyue",
        "url": "https://github.com/zhiyue"
      }
    }
  ]
}
```

- [ ] **Step 3: Validate the JSON syntax**

Run: `python3 -m json.tool < .claude-plugin/marketplace.json > /dev/null && echo OK`
Expected: `OK`

- [ ] **Step 4: Validate with the Claude CLI**

Run: `claude plugin validate .claude-plugin/marketplace.json`
Expected: exit 0, output mentions the `jira-cli` plugin. If `validate` errors on the `source` pointing at a not-yet-created directory, that's fine — move to Task 2 and revisit validation after Task 2 lands the plugin manifest.

- [ ] **Step 5: Commit**

```bash
git add .claude-plugin/marketplace.json
git commit -m "feat(plugin): add root marketplace manifest pointing at dist/plugin"
```

---

## Task 2: Plugin manifest

**Files:**
- Create: `dist/plugin/.claude-plugin/plugin.json`

- [ ] **Step 1: Create the directory**

```bash
mkdir -p dist/plugin/.claude-plugin
```

- [ ] **Step 2: Write `dist/plugin/.claude-plugin/plugin.json`**

```json
{
  "name": "jira-cli",
  "version": "0.2.2",
  "description": "Agent skill for the jira-cli tool — Jira Server 8.13.5 workflows.",
  "author": {
    "name": "zhiyue",
    "url": "https://github.com/zhiyue"
  },
  "homepage": "https://github.com/zhiyue/jira-cli"
}
```

- [ ] **Step 3: JSON lint**

Run: `python3 -m json.tool < dist/plugin/.claude-plugin/plugin.json > /dev/null && echo OK`
Expected: `OK`

- [ ] **Step 4: Plugin validator**

Run: `claude plugin validate dist/plugin`
Expected: exit 0. Warnings about missing `skills/`/`hooks/` are fine at this point — we add them in later tasks.

- [ ] **Step 5: Commit**

```bash
git add dist/plugin/.claude-plugin/plugin.json
git commit -m "feat(plugin): add Claude Code plugin manifest"
```

---

## Task 3: Preflight script `check.sh` (TDD)

The script must exit 0 if `jira-cli` is on PATH and prints a version, and exit 1 with an install hint if it is not. Keeping it tiny so it's trivial to test by manipulating `PATH`.

**Files:**
- Create: `dist/plugin/scripts/check.sh`
- Create: `dist/plugin/scripts/check_test.sh` (temporary — deleted after we are confident; see Step 8)

- [ ] **Step 1: Write the failing test harness**

```bash
mkdir -p dist/plugin/scripts
cat > dist/plugin/scripts/check_test.sh <<'EOF'
#!/usr/bin/env bash
# Tiny smoke harness for check.sh. Uses a temp dir with fake jira-cli to
# test the "found" path, and an empty PATH to test the "missing" path.
set -u
HERE="$(cd "$(dirname "$0")" && pwd)"
SCRIPT="$HERE/check.sh"

fail() { echo "FAIL: $*" >&2; exit 1; }

# 1) Missing binary → exit 1, stderr contains "install"
out="$(PATH="/usr/bin:/bin" "$SCRIPT" 2>&1 >/dev/null)"
rc=$?
[ "$rc" -eq 1 ]                        || fail "missing case: expected exit 1, got $rc"
echo "$out" | grep -qi "brew install"  || fail "missing case: no install hint on stderr"

# 2) Found binary → exit 0, stdout echoes the version line
tmp="$(mktemp -d)"
cat > "$tmp/jira-cli" <<'FAKE'
#!/usr/bin/env bash
echo "jira-cli 0.2.2 (target=fake, git=deadbeef)"
FAKE
chmod +x "$tmp/jira-cli"
out="$(PATH="$tmp:/usr/bin:/bin" "$SCRIPT")"
rc=$?
[ "$rc" -eq 0 ]                        || fail "found case: expected exit 0, got $rc"
echo "$out" | grep -q "jira-cli 0.2.2" || fail "found case: version not echoed"
rm -rf "$tmp"

echo "OK"
EOF
chmod +x dist/plugin/scripts/check_test.sh
```

- [ ] **Step 2: Run the test with no implementation yet — expect failure**

Run: `dist/plugin/scripts/check_test.sh`
Expected: fails because `check.sh` does not exist. The error will look like `check_test.sh: ... /check.sh: No such file or directory`, which satisfies the "red" step of TDD.

- [ ] **Step 3: Write `dist/plugin/scripts/check.sh`**

```bash
cat > dist/plugin/scripts/check.sh <<'EOF'
#!/usr/bin/env bash
# Preflight probe for the jira-cli skill.
# Exit 0 if the binary is on PATH (echoing its --version line to stdout).
# Exit 1 with an install hint on stderr if it is not.
set -u

if command -v jira-cli >/dev/null 2>&1; then
    jira-cli --version
    exit 0
fi

cat >&2 <<'HINT'
jira-cli: not found on PATH.

Install one of:
  brew install zhiyue/tap/jira-cli
  curl -sSL https://raw.githubusercontent.com/zhiyue/jira-cli/main/install.sh | sh

Docs: https://github.com/zhiyue/jira-cli#install
HINT
exit 1
EOF
chmod +x dist/plugin/scripts/check.sh
```

- [ ] **Step 4: Bash syntax check**

Run: `bash -n dist/plugin/scripts/check.sh && echo OK`
Expected: `OK`

- [ ] **Step 5: Shellcheck (optional if installed)**

Run: `command -v shellcheck && shellcheck dist/plugin/scripts/check.sh || echo "shellcheck not installed — skipped"`
Expected: either clean shellcheck output or the "skipped" message. Do not fail the task if shellcheck is missing.

- [ ] **Step 6: Run the test harness — expect PASS**

Run: `dist/plugin/scripts/check_test.sh`
Expected: final line is `OK`.

- [ ] **Step 7: Sanity-check against the real binary on this machine**

Run: `dist/plugin/scripts/check.sh`
Expected: exit 0, stdout line like `jira-cli 0.2.2 (target=aarch64-apple-darwin, git=...)`.

- [ ] **Step 8: Remove the test harness (it has done its job; we don't ship tests in the plugin)**

Run: `rm dist/plugin/scripts/check_test.sh`
Expected: `check_test.sh` is gone; only `check.sh` remains in `scripts/`.

- [ ] **Step 9: Commit**

```bash
git add dist/plugin/scripts/check.sh
git commit -m "feat(plugin): add check.sh preflight probe for jira-cli binary"
```

---

## Task 4: SessionStart hook

Runs `check.sh` once per Claude Code session so the user sees the install hint *before* they ask a Jira question, not only after the skill auto-triggers. The hook uses `${CLAUDE_PLUGIN_ROOT}` so it still works after the plugin is copied into the cache directory.

**Files:**
- Create: `dist/plugin/hooks/hooks.json`

- [ ] **Step 1: Create the directory**

```bash
mkdir -p dist/plugin/hooks
```

- [ ] **Step 2: Write `dist/plugin/hooks/hooks.json`**

```json
{
  "SessionStart": [
    {
      "hooks": [
        {
          "type": "command",
          "command": "${CLAUDE_PLUGIN_ROOT}/scripts/check.sh"
        }
      ]
    }
  ]
}
```

- [ ] **Step 3: JSON lint**

Run: `python3 -m json.tool < dist/plugin/hooks/hooks.json > /dev/null && echo OK`
Expected: `OK`

- [ ] **Step 4: Plugin validator picks up hooks**

Run: `claude plugin validate dist/plugin`
Expected: exit 0. If the validator warns that `${CLAUDE_PLUGIN_ROOT}` isn't substituted, that's expected — it is a runtime variable, not a validation one.

- [ ] **Step 5: Commit**

```bash
git add dist/plugin/hooks/hooks.json
git commit -m "feat(plugin): wire SessionStart hook to preflight check"
```

---

## Task 5: `SKILL.md` (main skill body)

**Files:**
- Create: `dist/plugin/skills/jira-cli/SKILL.md`

- [ ] **Step 1: Create the skill directory**

```bash
mkdir -p dist/plugin/skills/jira-cli
```

- [ ] **Step 2: Write `dist/plugin/skills/jira-cli/SKILL.md`**

````markdown
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
jira-cli schema --format=json        # full command tree as JSON
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
jira-cli bulk transition @my-open "Done"
jira-cli bulk comment PROJ-1,PROJ-2 --body "Released in v2.4.0"
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
````

- [ ] **Step 3: Verify the frontmatter description length**

The combined length of `description` must be ≤ 1,536 characters.

Run: `python3 -c "import pathlib,re; body=pathlib.Path('dist/plugin/skills/jira-cli/SKILL.md').read_text(); m=re.search(r'description:\s*(.+?)\n---', body, re.DOTALL); print(len(m.group(1)) if m else 'no match')"`
Expected: a number well below 1536 (the description above is ~550 chars).

- [ ] **Step 4: Plugin validator picks up the skill**

Run: `claude plugin validate dist/plugin`
Expected: exit 0; output mentions the `jira-cli` skill being discovered.

- [ ] **Step 5: Commit**

```bash
git add dist/plugin/skills/jira-cli/SKILL.md
git commit -m "feat(plugin): add jira-cli SKILL.md with preflight + curated workflows"
```

---

## Task 6: `references/workflows.md`

**Files:**
- Create: `dist/plugin/skills/jira-cli/references/workflows.md`

- [ ] **Step 1: Create the references directory**

```bash
mkdir -p dist/plugin/skills/jira-cli/references
```

- [ ] **Step 2: Write `dist/plugin/skills/jira-cli/references/workflows.md`**

````markdown
# Curated jira-cli workflows

Each section stands alone. Run commands in order. Subshells like
`$(…)` are only POSIX; run them in bash/zsh.

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
  --summary "Login form regresses on Safari 18" \
  --set type=Bug \
  --set priority=High \
  --set labels=regression,ios \
  --set components="Auth,Frontend"
```

Readable names (`type`, `priority`, `labels`, `components`) resolve via
`[field_aliases]` in `jira.toml`. Without aliases, use explicit ids:
`--set issuetype=10001`, `--set priority.id=2`.

## W4. Move an issue through a transition

```bash
jira-cli issue transitions list PROJ-123         # discover valid names
jira-cli issue transition PROJ-123 "In Progress"
jira-cli issue transition PROJ-123 Done \
  --resolution Fixed --comment "Released in v2.4.0"
```

`--resolution` and `--comment` are only accepted on transitions that
require / allow them — the `transitions list` output tells you which.

## W5. Parallel bulk actions

```bash
# Close everything matching a JQL alias
jira-cli bulk transition @stale-triage Done --resolution "Won't Fix"

# Comment on a list of keys
jira-cli bulk comment PROJ-1,PROJ-2,PROJ-3 --body "Superseded by PROJ-42"
```

Both commands fan out in parallel and return a JSON summary with per-key
success / failure. Non-zero exit if any target failed.

## W6. Sprint operations

```bash
# Discover boards and active sprints
jira-cli board list
jira-cli sprint list --board 42 --state active

# Inspect a sprint's issues
jira-cli sprint issues 100 --keys-only

# Move issues into / out of a sprint
jira-cli sprint move 100 PROJ-10,PROJ-11
jira-cli backlog move PROJ-12           # pulls back to backlog
```

## W7. Epic children

```bash
jira-cli epic get PROJ-10
jira-cli epic issues PROJ-10 --keys-only
jira-cli epic add-issues PROJ-10 PROJ-21,PROJ-22
jira-cli epic remove-issues PROJ-10 PROJ-23
```

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
  --body '{"body": "..."}'
```

Response is the raw server body; exit codes still follow the standard
table (auth / not found / conflict / 5xx / network).
````

- [ ] **Step 3: Commit**

```bash
git add dist/plugin/skills/jira-cli/references/workflows.md
git commit -m "docs(plugin): add curated workflow reference"
```

---

## Task 7: `references/config.md`

**Files:**
- Create: `dist/plugin/skills/jira-cli/references/config.md`

- [ ] **Step 1: Write `dist/plugin/skills/jira-cli/references/config.md`**

````markdown
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
````

- [ ] **Step 2: Commit**

```bash
git add dist/plugin/skills/jira-cli/references/config.md
git commit -m "docs(plugin): add jira-cli config reference"
```

---

## Task 8: Plugin README

**Files:**
- Create: `dist/plugin/README.md`

- [ ] **Step 1: Write `dist/plugin/README.md`**

````markdown
# jira-cli plugin

Agent skill that wraps the [`jira-cli`](https://github.com/zhiyue/jira-cli)
binary — Jira Server 8.13.5 issues, JQL search, transitions, sprints,
epics, and parallel bulk operations — for Claude Code and Codex CLI.

## Prerequisite — install the `jira-cli` binary

```bash
brew install zhiyue/tap/jira-cli
# or
curl -sSL https://raw.githubusercontent.com/zhiyue/jira-cli/main/install.sh | sh
```

Verify:

```bash
jira-cli --version   # → jira-cli 0.2.2 (target=..., git=...)
jira-cli config init
```

The plugin will run a preflight probe on session start and tell you
how to install if the binary is missing.

## Install in Claude Code

```
/plugin marketplace add zhiyue/jira-cli
/plugin install jira-cli@jira-cli
```

`zhiyue/jira-cli` is the marketplace name (the GitHub repo); the first
`jira-cli` is the plugin name, the second is the marketplace name.

Update later with `/plugin update jira-cli`; remove with
`/plugin uninstall jira-cli`.

## Install in Codex CLI

**Option A — inside a codex session (recommended):**

```
$skill-installer https://github.com/zhiyue/jira-cli/tree/main/dist/plugin/skills/jira-cli
```

**Option B — git clone + symlink:**

```bash
git clone https://github.com/zhiyue/jira-cli ~/src/jira-cli
mkdir -p ~/.agents/skills
ln -s ~/src/jira-cli/dist/plugin/skills/jira-cli ~/.agents/skills/jira-cli
```

Pull updates with `git -C ~/src/jira-cli pull`. Restart `codex` to pick
up changes to `SKILL.md`.

## Triggering

The skill is auto-triggered when the conversation mentions Jira-shaped
vocabulary (project keys like `PROJ-123`, "sprint", "backlog", "JQL",
"ticket", etc.). To force it in Codex use `$jira-cli`; in Claude Code
use `/jira-cli:jira-cli`.

## Contents

- `skills/jira-cli/SKILL.md` — main entry point
- `skills/jira-cli/references/workflows.md` — expanded scenarios
- `skills/jira-cli/references/config.md` — `jira.toml` + env reference
- `scripts/check.sh` — preflight probe run via SessionStart hook
- `hooks/hooks.json` — wires `check.sh` into Claude Code sessions
- `.claude-plugin/plugin.json` — Claude Code plugin manifest
````

- [ ] **Step 2: Commit**

```bash
git add dist/plugin/README.md
git commit -m "docs(plugin): add plugin README with install notes for CC + Codex"
```

---

## Task 9: Update release SOP with plugin-version rules

**Files:**
- Modify: `docs/RELEASING.md`

- [ ] **Step 1: Read the current SOP and locate the "Prepare the release commit locally" section**

Run: `grep -n "^##" docs/RELEASING.md`
Expected output includes the section titled `## 1. Prepare the release commit locally`.

- [ ] **Step 2: Edit `docs/RELEASING.md` — add `plugin.json` bump to Step 1**

Find this existing passage (in `docs/RELEASING.md`):

```
Edit these three files in one commit:

1. `Cargo.toml` — bump `version = "X.Y.Z"`.
2. `CHANGELOG.md` — insert a new section above the latest one:
```

Replace it with:

```
Edit these files in one commit:

1. `Cargo.toml` — bump `version = "X.Y.Z"`.
2. `dist/plugin/.claude-plugin/plugin.json` — bump `version` to `X.Y.Z`.
3. `.claude-plugin/marketplace.json` — bump the `plugins[0].version`
   field to `X.Y.Z` to match.
4. `CHANGELOG.md` — insert a new section above the latest one:
```

Then renumber the existing `Cargo.lock` item from `3` to `5`.

- [ ] **Step 3: Append a new subsection after the existing "Target matrix note"**

Find the final paragraph of `docs/RELEASING.md` (it ends with the
`git=unknown` fallback note) and append this new section at the very
end of the file:

````markdown

## Skill / plugin-only hotfixes

When you need to ship a change that lives entirely under `dist/plugin/`
or in the root `.claude-plugin/marketplace.json` — for example, a
correction to `SKILL.md`, a new workflow in `references/workflows.md`,
or fixing `check.sh` — you do **not** need to cut a crate release.

Flow:

1. Make the change on a branch, land it on `main` via a normal commit.
2. Bump `dist/plugin/.claude-plugin/plugin.json#version` *and* the
   matching entry in `.claude-plugin/marketplace.json` to the next
   patch (e.g. `0.2.2` → `0.2.3`) even though `Cargo.toml` is still
   at `0.2.2`. This is the one situation where the plugin version
   runs ahead of the crate.
3. The next crate release realigns them by bumping `Cargo.toml` to
   the plugin's version (or higher) in the same release commit.

No tag is required for a plugin-only hotfix. Users on Claude Code
pick it up via `/plugin update jira-cli`; Codex users via
`git pull` + `codex` restart.
````

- [ ] **Step 4: Verify the changes look right**

Run: `grep -n "plugin.json" docs/RELEASING.md`
Expected: at least two matches (the bump instruction in Step 1 and the hotfix section).

Run: `grep -c "^## " docs/RELEASING.md`
Expected: one higher than before (new "Skill / plugin-only hotfixes" section).

- [ ] **Step 5: Commit**

```bash
git add docs/RELEASING.md
git commit -m "docs(release): document plugin.json version sync and hotfix flow"
```

---

## Task 10: End-to-end verification + CHANGELOG

This Task does not create new files in `dist/plugin/`; it verifies the
entire plugin works end-to-end and records the user-visible change in
`CHANGELOG.md`.

**Files:**
- Modify: `CHANGELOG.md`

- [ ] **Step 1: Validate the full plugin tree**

Run: `claude plugin validate dist/plugin`
Expected: exit 0, output lists the `jira-cli` skill and the `SessionStart` hook.

Run: `claude plugin validate .claude-plugin/marketplace.json`
Expected: exit 0, output lists one plugin named `jira-cli`.

- [ ] **Step 2: Add the marketplace locally**

Run: `claude plugin marketplace add /Users/zhiyue/workspace/jira-cli`
Expected: the marketplace `jira-cli` is registered.

Run: `claude plugin marketplace list`
Expected: output includes an entry `jira-cli` pointing at this repo.

- [ ] **Step 3: Install the plugin locally and restart Claude Code to pick it up**

Run: `claude plugin install jira-cli@jira-cli`
Expected: success message. Then quit and relaunch your Claude Code session.

- [ ] **Step 4: Verify the SessionStart hook fires**

When a fresh Claude Code session starts, the preflight script runs and
either prints the `jira-cli --version` line or the install hint.
Capture the result into the plan task description if the output is not
as expected; otherwise continue.

- [ ] **Step 5: Smoke-test a Jira-shaped prompt**

In a Claude Code session, type a message like:
`Show me the status of PROJ-123.`

Expected: Claude loads the `jira-cli` skill (its `description` matches),
runs `jira-cli --version`, then proposes an `jira-cli issue get PROJ-123 ...`
command. (It is fine if the actual API call fails against a test Jira
instance; we are verifying the skill triggers, not the integration.)

- [ ] **Step 6: Codex symlink smoke test**

```bash
mkdir -p ~/.agents/skills
ln -s /Users/zhiyue/workspace/jira-cli/dist/plugin/skills/jira-cli ~/.agents/skills/jira-cli-local-test

# Non-interactive one-shot run:
codex exec "I have ticket PROJ-42 assigned to me. Using jira-cli, what command fetches its status?"

# Cleanup:
rm ~/.agents/skills/jira-cli-local-test
```

Expected: Codex loads the `jira-cli-local-test` skill (matching on the
Jira-shaped prompt) and answers with a `jira-cli issue get PROJ-42 ...`
command. If `codex exec` is unavailable in your Codex version, run
`codex` interactively instead, type the prompt, then exit.

The symlink is removed because the real install route is
`$skill-installer` or a persistent symlink at `~/.agents/skills/jira-cli`;
this is only a local smoke test.

- [ ] **Step 7: Add CHANGELOG entry**

Find the top `## [0.2.2]` section in `CHANGELOG.md` and *insert a new
sub-bullet under its existing `### Changed`* (if present) or add a new
`### Added` block:

```
### Added
- `dist/plugin/` — Claude Code plugin + Codex-compatible skill for
  `jira-cli`. Installable via
  `/plugin marketplace add zhiyue/jira-cli && /plugin install jira-cli@jira-cli`
  in Claude Code, or via `$skill-installer <tree-url>` / a symlink to
  `~/.agents/skills/` in Codex CLI. Skill auto-triggers on
  Jira-shaped conversations and preflights `jira-cli --version` with
  an install hint on miss.
```

- [ ] **Step 8: Commit the CHANGELOG update**

```bash
git add CHANGELOG.md
git commit -m "docs(changelog): note jira-cli skill + plugin under 0.2.2"
```

- [ ] **Step 9: Final push**

```bash
git push origin main
```

Note: this is a non-tag push — it does **not** re-trigger the release
workflow. The plugin is installable directly from `main` immediately.

---

## Self-Review Notes

*(This section is reviewed by the plan author before execution starts
and can be deleted once the plan is under way. Items below were
double-checked against the spec.)*

- **Spec coverage:** every decision in the spec (repo layout, curated
  scope, doc-only binary handling, auto-trigger, version sync rule) has
  a task that produces the corresponding file or change. Versioning
  rule → Task 9. Codex install path → Task 8 README + Task 10 smoke test.
- **Placeholder scan:** no "TBD"/"TODO" in content. Every file task
  embeds the final bytes.
- **Type consistency:** plugin name `jira-cli` matches across
  `marketplace.json`, `plugin.json`, and the SKILL frontmatter. Skill
  reference paths (`references/workflows.md`, `references/config.md`)
  match actual file paths created in Tasks 6-7.
