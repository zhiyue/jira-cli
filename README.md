# jira-cli

Agent-first CLI for legacy **Jira Server 8.13.5**. Rust, single binary, no daemon, TOML config.

> Targets Jira Server/DC 8.13.5 specifically — no Cloud, no PAT (PAT ships 8.14+). Uses Basic Auth or cookie session (`/rest/auth/1/session`).

## Why

Modern Jira CLIs assume Cloud or newer Server versions. For teams stuck on 8.13.5, options are thin. This tool:

- Speaks Jira 8.13.5's native REST v2 + Agile 1.0 exactly
- JSON/JSONL output by default (agent-friendly, no HTML table noise)
- Typed errors with stable exit codes + actionable `hint` field
- `schema` self-introspection so agents can discover the CLI
- Field aliases + renames for messy customfield landscapes
- TOML config with defaults, JQL aliases, per-command field projections

## Why a CLI and not an MCP server?

A Jira MCP server and `jira-cli` cover the same ground from an agent's perspective. We went CLI because of **process model economics**:

- **MCP = long-lived daemon.** Every concurrent Claude Code / agent session spawns its own MCP child process and keeps it resident. A typical Node or Python MCP stays **30–80 MiB RSS idle** per instance. Five agent sessions on one laptop → 150–400 MiB just holding idle connections. IPC buffers, language runtime, cached TLS context — none of it freeable while the session lives.
- **CLI = fork-exec, short-lived.** `jira-cli` uses memory **only during the invocation**. Peak ~4 MiB per call ([see benchmarks](bench/results/BENCHMARK_API.md)), freed on exit. A burst of 20 concurrent calls peaks under 100 MiB total and returns to zero when the work's done.
- **Startup cost is negligible** for the usage pattern. Agents run a jira command, read the JSON, move on — not a tight inner loop where 10–20 ms cold start would matter. Wall-clock on real Jira is dominated by ~450 ms of network RTT regardless of tool.
- **Composable by default.** Pipe into `jq`, feed into `xargs`, redirect to file, run in CI — nothing special. An MCP needs a bespoke client per consumer.
- **Zero protocol lock-in.** Jira-ccli speaks JSON-over-stdout; any agent runtime (Claude Code, Codex, Copilot CLI, a shell script) can use it. MCP requires the host to implement the MCP protocol.
- **Observable.** `-vv` gives structured tracing; `time -l` measures usage; binaries signed and checksummed. Debugging an MCP means debugging its host's protocol stack.

**When an MCP would make more sense**: when you need persistent state across calls (cached auth tokens with refresh logic, streaming subscriptions to Jira webhooks, multi-step workflows with inter-call coordination). None of those apply to the workflows we target.

For the rare case where an agent really wants structured tool semantics instead of shelling out, the `schema` command + stable JSON contract make a thin MCP wrapper around this binary trivial (~50 lines of code) — without baking one into the distribution.

## Install

### Homebrew (macOS + Linux)

If the maintainer has published a tap:

```bash
brew tap zhiyue/jira-cli     # or whatever the tap is
brew install jira-cli
```

### Install script (macOS + Linux)

```bash
curl -sSL https://raw.githubusercontent.com/zhiyue/jira-cli/main/install.sh | sh
```

Options: `install.sh -v v0.1.0`, `-d /usr/local/bin`, `-b https://internal-mirror.example.com/...`.

### Install script (Windows PowerShell)

```powershell
iwr -useb https://raw.githubusercontent.com/zhiyue/jira-cli/main/install.ps1 | iex
```

### `cargo install` (any platform with Rust ≥ 1.88)

```bash
cargo install --git https://github.com/zhiyue/jira-cli --locked
```

### Manual

Download the tarball for your platform from the [Releases page](https://github.com/zhiyue/jira-cli/releases), extract, put `jira-cli` on your PATH.

## Quickstart

```bash
# one-time
jira-cli config init \
    --url https://jira.internal.example.com \
    --user alice \
    --password "$JIRA_PASSWORD"

jira-cli ping
jira-cli whoami
jira-cli issue get MGX-1 --pretty
jira-cli search "project = MGX AND status = Open" --max 20 --keys-only
```

## Configuration

Default path: `$XDG_CONFIG_HOME/jira-cli/config.toml` (typically `~/.config/jira-cli/config.toml`). Mode `0600` enforced.

Precedence for resolved values: **CLI flag > env var > config file > built-in default**.

Full example:

```toml
# basic connection
url = "https://jira.internal.example.com"
user = "alice"
password = "your-password-or-token"
insecure = false
timeout_secs = 30
concurrency = 4

# auth_method = "cookie"                         # optional; default "basic"
# session_cookie = "JSESSIONID=..."              # required if cookie auth

# default jira-fields (server-side) per command
[defaults]
auto_rename_custom_fields = false                # opt-in: slug field names
search_fields = ["summary", "status", "priority", "assignee", "issuetype", "updated"]
issue_get_fields = [
    "summary", "status", "priority", "assignee",
    "reporter", "issuetype", "components", "labels",
    "created", "updated", "resolution", "description",
    # ... your customfield ids
]

# Display name → field id (write path, --set)
[field_aliases]
"Story Points" = "customfield_10006"

# customfield id → readable key (read path, output rewrite)
[field_renames]
customfield_10006 = "story_points"
customfield_11604 = "bug_url"

# named JQL snippets (use with `search @name`)
[jql_aliases]
mine_open = "assignee = currentUser() AND resolution = Unresolved"
critical = "priority in (Highest, High) AND resolution = Unresolved"
```

### Environment variables

Any of the config keys above can be overridden via env:

| Var | Meaning |
|---|---|
| `JIRA_URL`, `JIRA_USER`, `JIRA_PASSWORD` | Basic auth |
| `JIRA_AUTH_METHOD=cookie` + `JIRA_SESSION_COOKIE` | Cookie auth |
| `JIRA_TIMEOUT`, `JIRA_INSECURE`, `JIRA_CONCURRENCY` | Runtime |
| `XDG_CONFIG_HOME` | Override config dir base |

## Agent quickstart

The tool is optimized for LLM/agent consumption. Four patterns:

**1. Capability discovery**

```bash
jira-cli schema | jq '.commands | keys'
jira-cli schema issue | jq '.subcommands.get'
```

**2. Minimal-token issue inspection**

```bash
jira-cli issue get MGX-1 --pretty                           # defaults apply (configured fields)
jira-cli issue get MGX-1 --jira-fields ""                   # full payload (bypass defaults)
jira-cli issue get MGX-1 --fields "key,fields.summary,fields.status.name,fields.bug_url"
```

With `auto_rename_custom_fields = true` and proper `[field_renames]`, the output uses snake_case keys like `story_points` / `bug_url` / `solution` instead of `customfield_10006` etc.

**3. Streaming JQL for large result sets**

```bash
jira-cli search @mine_open --max 500 --keys-only   # pipe into xargs / other tools
jira-cli search "project = MGX AND updated > -7d" --page-size 100
```

**4. Bulk writes**

```bash
cat <<EOF > comments.jsonl
{"key":"MGX-1","body":"deployed to staging"}
{"key":"MGX-2","body":"deployed to staging"}
EOF
jira-cli bulk comment --file comments.jsonl --concurrency 4
```

## Exit codes

| Code | Meaning |
|---|---|
| 0 | Success |
| 2 | Usage / config error |
| 3 | Jira API business error (400/409/422/5xx) |
| 4 | Network error (DNS / connect / timeout / TLS) |
| 5 | Auth error (incl. CAPTCHA) |
| 6 | Resource not found (404) |
| 7 | Internal / IO / deserialization |

Errors go to **stderr** as JSON:

```json
{"error":{"kind":"not_found","message":"issue not found: MGX-42","hint":"Verify the issue exists or check permissions with `jira-cli whoami`"}}
```

`kind` values: `config | usage | auth | not_found | api_error | network | serialization | field_resolve | io`.

## Command reference

Run `jira-cli schema --pretty` for the complete machine-readable tree. Quick index:

- **Meta**: `ping`, `whoami`, `config {show|init}`, `schema`, `session new`, `raw`
- **Issue core**: `issue {get|create|update|delete|assign|bulk-create|changelog}`
- **Issue sub-resources**: `issue {comment|transitions|transition|link|attachment|worklog|watchers}`
- **Search**: `search <JQL>` with `--max`, `--page-size`, `--start-at`, `--keys-only`, `--jira-fields`, `--expand`
- **Metadata**: `field {list|resolve}`, `project {list|get|statuses|components}`, `user {get|search}`
- **Agile**: `board {list|get|backlog}`, `sprint {list|get|create|update|delete|move|issues}`, `epic {get|issues|add-issues|remove-issues}`, `backlog move`
- **Parallel bulk**: `bulk {transition|comment}`
- **Escape hatch**: `raw <METHOD> <PATH> [-d <body|@file|->] [--query k=v] [--header k:v]`

## Build from source

```bash
git clone https://github.com/zhiyue/jira-cli
cd jira-cli
cargo build --release
./target/release/jira-cli --version
```

Requirements: Rust ≥ 1.88. No C compiler / OpenSSL needed (uses `rustls`).

### Development

```bash
cargo fmt --all -- --check
cargo clippy --all-targets -- -D warnings
cargo test
```

141+ tests, all integration tests use `wiremock` (no real Jira needed).

## Troubleshooting

**`auth: unauthorized` immediately on basic auth**: your Jira may require CAPTCHA after too many bad logins. Log in via browser to clear, or use cookie auth:

```bash
JIRA_USER=alice JIRA_PASSWORD=... jira-cli session new
# copy the cookie into JIRA_SESSION_COOKIE + JIRA_AUTH_METHOD=cookie
```

**`field 'X' is ambiguous`**: the display name maps to multiple customfield ids. Either use the explicit id in `--set "customfield_10006=5"` or add a `[field_aliases]` entry.

**Self-signed TLS certs**: set `insecure = true` in config or `JIRA_INSECURE=1`. A warning is shown on TTY but not on pipes/agent.

**Want to debug a request**: run with `-vv` for `tracing::debug` events on stderr.

## Performance

Comparative benchmarks vs the equivalent Go CLI ([`jira-go-cli`](https://github.com/zhiyue/jira-go-cli)) show Rust on par for single-call wall-clock (network dominates at ~450 ms), but **1.44× faster on JQL search 20**, uses **1.4×–2.0× less CPU** after stripping network, and **1.5×–1.7× less peak memory** across all scenarios.

See [`bench/results/BENCHMARK_API.md`](bench/results/BENCHMARK_API.md) for the full table. Regenerate with `./bench/run-api-bench.sh` (needs `hyperfine` + both binaries on PATH).

## Project layout

```
src/
  api/             # typed wrappers per Jira resource (no I/O besides HttpClient)
  cli/             # clap derive; dispatches to api/
  http/            # reqwest blocking + auth + retry
  config.rs        # TOML + env merge
  field_resolver.rs
  output.rs        # JSON / JSONL / --fields / field_renames
  schema.rs        # clap introspection
  error.rs         # typed errors + exit codes + stderr JSON
```

Design docs live in `docs/superpowers/`:
- [Spec](docs/superpowers/specs/2026-04-17-jira-cli-design.md)
- [Implementation plan](docs/superpowers/plans/2026-04-17-jira-cli.md)

## License

MIT OR Apache-2.0
