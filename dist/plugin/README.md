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

The plugin runs a preflight probe on session start and prints an
install hint if the binary is missing.

## Install in Claude Code

```
/plugin marketplace add zhiyue/jira-cli
/plugin install jira-cli@jira-cli
```

`zhiyue/jira-cli` is the GitHub repo that carries the marketplace
manifest. The `jira-cli@jira-cli` notation is `<plugin-name>@<marketplace-name>`
— both happen to be `jira-cli` in this repo.

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

Pull updates with `git -C ~/src/jira-cli pull`. Restart `codex` to
pick up changes to `SKILL.md`.

## Triggering

The skill auto-triggers when the conversation mentions Jira-shaped
vocabulary (project keys like `PROJ-123`, "sprint", "backlog", "JQL",
"ticket", etc.). To force it, use `$jira-cli` in Codex or
`/jira-cli:jira-cli` in Claude Code.

## Contents

- `skills/jira-cli/SKILL.md` — main entry point
- `skills/jira-cli/references/workflows.md` — expanded scenarios
- `skills/jira-cli/references/config.md` — `config.toml` + env reference
- `scripts/check.sh` — preflight probe run via SessionStart hook
- `hooks/hooks.json` — wires `check.sh` into Claude Code sessions
- `.claude-plugin/plugin.json` — Claude Code plugin manifest
