# jira-cli skill + Claude Code plugin — design

- **Status**: Approved, ready for implementation plan
- **Date**: 2026-04-18
- **Scope**: Ship a reusable "skill" for the `jira-cli` binary so both
  Claude Code and Codex CLI users can discover and use the tool
  through their native agent surfaces.

## Problem

`jira-cli` exposes a large command surface (issues, JQL, sprints, epics,
bulk ops) and deliberately agent-first features (`schema`, exit codes,
JSON stderr, field aliases). Agents currently have to rediscover all of
this on every session. We want:

- Claude Code users: `/plugin install` once and the skill is always
  available, auto-triggering on Jira-shaped conversations.
- Codex CLI users: one install command drops a matching skill into
  `~/.agents/skills/` and gets the same behavior.
- A single source of truth — no drift between the two platforms.

## Non-goals

- Wrapping every subcommand in prose. The binary already has `--help`
  and `jira-cli schema --format=json`; the skill points to them.
- Bundling the binary. Installation stays the user's responsibility;
  the skill detects and prompts.
- Slash commands (`/jira:triage` etc.). Out of scope for v1; the
  structure leaves room to add them later without disruption.
- MCP server wrapper. Out of scope.

## Decisions

Captured during brainstorming (2026-04-18):

| Axis | Choice | Rationale |
| --- | --- | --- |
| Repo layout | Same repo, `dist/plugin/` subdirectory | Versions, docs, CHANGELOG stay in lockstep with the Rust crate. No second repo to bump on release. |
| Skill scope | Curated workflow guide | 4–6 high-value patterns + pointer to `schema`/`--help`. High info density, accurate triggering, low maintenance. Not a full manual, not paper-thin. |
| Binary handling | Doc + runtime check in the skill | Skill runs `jira-cli --version`; on miss, refuses with an install hint (`brew install zhiyue/tap/jira-cli` or `install.sh`). No silent auto-install hook — symmetric between CC and Codex, easier to debug. |
| Trigger | Auto-trigger via `description` | Jira project keys / JQL / sprint / ticket vocabulary in the description. No `disable-model-invocation`. |

## Architecture

### Directory layout

```
jira-cli/
├── .claude-plugin/
│   └── marketplace.json          # `/plugin marketplace add zhiyue/jira-cli`
└── dist/plugin/
    ├── .claude-plugin/
    │   └── plugin.json           # Claude Code plugin manifest
    ├── skills/
    │   └── jira-cli/
    │       ├── SKILL.md          # single source of truth, CC + Codex compatible
    │       ├── references/
    │       │   ├── workflows.md  # curated scenarios (create / JQL / bulk / sprint / epic)
    │       │   └── config.md     # jira.toml, env, default_project, aliases
    │       └── scripts/
    │           └── check.sh      # `jira-cli --version` probe + install hint on miss
    ├── hooks/
    │   └── hooks.json            # SessionStart → scripts/check.sh (no auto-install)
    └── README.md                 # install instructions for both platforms
```

Rationale: `.claude-plugin/` at the repo root hosts only the marketplace
listing, so `zhiyue/jira-cli` is simultaneously its own marketplace and
the plugin source. `dist/plugin/` is the plugin payload. Keeping skill
content under `dist/plugin/skills/jira-cli/` means the same directory
works verbatim when symlinked into `~/.agents/skills/jira-cli/` on
Codex.

### `SKILL.md` (skeleton)

```markdown
---
name: jira-cli
description: Manage Jira Server 8.13.5 via the `jira-cli` binary —
  issues, JQL search, transitions, sprints, epics, bulk ops. Triggered
  when the user mentions a Jira project key (e.g. PROJ-123), asks about
  tickets, sprint backlog, JQL, or transitions. Requires `jira-cli` on
  PATH; the skill refuses with an install hint if missing.
---

# Jira (on-prem 8.13.5) via jira-cli

## 0. Preflight
Run `jira-cli --version`. If missing: stop, tell the user to install
(`brew install zhiyue/tap/jira-cli` or `install.sh`). Config at
`~/.config/jira-cli/jira.toml` — bootstrap with `jira-cli config init`.

## 1. Discover capabilities
Run `jira-cli schema --format=json` when unsure of flags. Use
`--help` for a specific command.

## 2. Curated workflows
- Read:    `jira-cli issue get PROJ-123 --fields=summary,status,assignee`
- Search:  `jira-cli search 'project=PROJ AND status!=Done' --keys-only`
- Create:  `jira-cli issue create --project PROJ --summary … --set type=Task`
- Transit: `jira-cli issue transition PROJ-123 "In Progress"`
- Bulk:    `jira-cli bulk transition @my-open "Done"`
- Sprint:  `jira-cli sprint issues 42 --keys-only`

See `references/workflows.md` for expanded examples and
`[jql_aliases]` / `[field_aliases]` patterns.

## 3. Output discipline
Prefer `--keys-only` / `--fields=…` to keep context lean. Exit codes
0/2/3/4/5/6/7 are stable; errors are JSON on stderr.
```

`references/workflows.md` holds the longer form — create-and-assign,
triage-by-JQL, parallel bulk transitions, sprint move, epic children,
changelog inspection. `references/config.md` documents `jira.toml`,
`JIRA_URL` / `JIRA_TOKEN` / `JIRA_PROJECT` env, and
`auto_rename_custom_fields`.

### Manifests

`dist/plugin/.claude-plugin/plugin.json`:

```json
{
  "name": "jira-cli",
  "version": "0.2.2",
  "description": "Agent skill for the jira-cli tool — Jira Server 8.13.5 workflows.",
  "author": { "name": "zhiyue", "url": "https://github.com/zhiyue" },
  "homepage": "https://github.com/zhiyue/jira-cli"
}
```

Root `.claude-plugin/marketplace.json`:

```json
{
  "name": "jira-cli",
  "owner": { "name": "zhiyue" },
  "plugins": [
    {
      "name": "jira-cli",
      "source": "./dist/plugin",
      "description": "Skill for the jira-cli Jira 8.13.5 CLI."
    }
  ]
}
```

### Install UX

**Claude Code**

```
/plugin marketplace add zhiyue/jira-cli
/plugin install jira-cli@jira-cli
```

On first load, the `SessionStart` hook runs `scripts/check.sh`, which
`jira-cli --version`s and prints the install hint on miss. The skill
itself also repeats the check in step 0 — belt-and-suspenders so
auto-triggered use still catches missing binaries.

**Codex CLI** (two documented options)

```bash
# Option A — inside a codex session
$skill-installer https://github.com/zhiyue/jira-cli/tree/main/dist/plugin/skills/jira-cli

# Option B — clone-and-symlink
git clone https://github.com/zhiyue/jira-cli ~/src/jira-cli
ln -s ~/src/jira-cli/dist/plugin/skills/jira-cli ~/.agents/skills/jira-cli
```

Codex-specific `agents/openai.yaml` metadata is deliberately omitted in
v1. If we later want to declare the `jira-cli` binary as a formal tool
dependency or suppress implicit invocation per user, we add it then.

## Versioning and release

Rule: the plugin's `version` matches `Cargo.toml#version` **at every
tagged release** so a downloaded tarball and a freshly installed
plugin always agree. Between releases the plugin may be bumped alone
for skill-only hotfixes — e.g. `Cargo.toml` stays at `0.2.3` while
`plugin.json` goes to `0.2.3+skill.1` or simply `0.2.4` — and the
next crate release realigns them. Concretely:

- `docs/RELEASING.md` step 1 gains: bump `plugin.json#version` in the
  same commit as `Cargo.toml`.
- Skill-only hotfix flow is documented as a separate short path in
  the same doc.
- `.github/workflows/release.yml` is untouched. The plugin is
  consumed directly from the git ref the user installs (`@main`, a
  tag, a commit) — there is no per-release plugin tarball.

## Impact and risks

- **Repo mix**: a Rust crate repo now also carries non-Rust artifacts
  in `dist/plugin/`. Mitigated by keeping everything under `dist/` —
  `cargo` ignores it, release workflow ignores it.
- **Binary drift vs skill**: if `jira-cli` adds/removes a subcommand,
  the curated workflows can go stale. Accepted risk; `SKILL.md` tells
  the agent to fall back to `schema`/`--help`, so drift degrades
  gracefully rather than silently.
- **Codex `$skill-installer` stability**: still a newer Codex feature;
  if it changes semantics, option B (git clone + symlink) is the
  stable fallback and is already documented.
- **Marketplace collision**: the marketplace name `jira-cli` and the
  plugin name `jira-cli` match; install is `jira-cli@jira-cli`. Ugly
  but valid. Can rename the marketplace later without breaking
  existing installs.

## Verification (definition of done)

1. Fresh machine (no `jira-cli` installed) — Claude Code `/plugin
   install` completes, skill loads, asking a Jira-shaped question
   triggers the skill, and the skill prints the install hint.
2. After `brew install zhiyue/tap/jira-cli`, the same conversation
   yields a correct `jira-cli ...` command that runs successfully
   against a real Jira instance.
3. On a fresh Codex CLI, `$skill-installer` (option A) installs the
   skill into `~/.agents/skills/jira-cli/`, and Codex auto-triggers it
   on the same prompt.
4. `claude plugin validate dist/plugin` (or equivalent lint) passes.
5. Bumping `Cargo.toml` to `0.2.3` and the plugin to `0.2.3` in one
   commit, tagging `v0.2.3`, and following the release SOP publishes
   the binary + makes the new plugin available at the new ref.

## Out of scope (explicit backlog)

- Slash commands `/jira:triage`, `/jira:pick-ticket`, etc.
- MCP-style structured tool bridge.
- `agents/openai.yaml` Codex metadata.
- A `SessionStart` hook that actively `brew install`s the binary.
- Automatic plugin-version bump in CI (stays manual in the SOP for
  now).
