# Changelog

All notable changes to jira-cli are documented in this file.

Format loosely follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).
Versions follow [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.0] — unreleased

Initial release. Agent-first CLI for legacy Jira Server 8.13.5.

### Commands
- Core: `ping`, `whoami`, `config show|init`, `schema`, `session new`, `raw`
- Issue: `get`, `create`, `update`, `delete`, `assign`, `bulk-create`, `changelog`, `comment {list|add|update|delete}`, `transitions list`, `transition`, `link {list|add|delete}`, `attachment {list|upload|download|delete}`, `worklog {list|add|delete}`, `watchers {list|add|remove}`
- Search: `search <JQL>` with streaming pagination, `--keys-only`, `--max`
- Field: `list`, `resolve`
- Project: `list`, `get`, `statuses`, `components`
- User: `get`, `search`
- Agile: `board {list|get|backlog}`, `sprint {list|get|create|update|delete|move|issues}`, `epic {get|issues|add-issues|remove-issues}`, `backlog move`
- Bulk: `transition`, `comment` (parallel fan-out)

### Agent features
- Stable exit codes (0/2/3/4/5/6/7) + structured stderr JSON errors with hints
- `--fields` dot-path projection + `--jira-fields` server-side field selection
- `[field_aliases]` display-name → id for `--set` writes
- `[field_renames]` customfield id → readable name on output
- `auto_rename_custom_fields` opt-in automatic slug generation with collision detection
- `[jql_aliases]` named JQL snippets (`search @alias`)
- `[defaults]` per-command default `jira-fields`
- `jira-cli schema` for CLI capability discovery

### Distribution
- Pre-built binaries for 7 targets (Darwin x86/arm, Linux gnu/musl × x86/arm, Windows x86)
- `install.sh` + `install.ps1` POSIX/Windows installers
- Homebrew formula at `dist/homebrew/jira-cli.rb`
