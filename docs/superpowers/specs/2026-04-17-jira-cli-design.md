# jira-cli — Design Spec

- **Date**: 2026-04-17
- **Status**: Draft (pending user approval)
- **Owner**: @jabari.dai
- **Target Jira version**: **Jira Server/Data Center 8.13.5** (`v8.13.5#813005-sha1:c18f263`)

---

## 1. Context & Goals

### 1.1 Problem

Team runs a legacy **Jira Server 8.13.5** (released 2020-10). This version predates Personal Access Token (PAT) support (introduced in 8.14) and lacks Atlassian Cloud-only niceties. Modern community Jira CLIs (`ankitpokhrel/jira-cli`, `go-jira/jira`) either assume Cloud or newer Server versions, and their output is tuned for humans, not AI agents.

### 1.2 Goal

Ship a **Rust binary `jira-cli`** tuned for **AI agent consumption**:

- Stable, typed JSON output (and JSONL streaming) as the default
- Authenticates against 8.13.5 using only mechanisms it actually supports (Basic Auth, cookie session)
- Covers the subset of Jira REST v2 + Agile 1.0 APIs the team's agents need day-to-day
- Exposes its own command schema so agents can discover capabilities without hard-coding

### 1.3 Non-Goals

- Atlassian Cloud compatibility (the Cloud API diverges; target is strictly Server 8.13.5)
- Full coverage of every 8.13.5 REST endpoint (audit, dashboards, screens, workflows, etc. are out of scope for v1)
- Interactive TUI / wizard
- Long-running daemon / watch mode (use cron + `issue get`)
- Pentesting / admin-heavy operations (permission schemes, workflow editing)

### 1.4 Success Criteria

1. An agent can run `jira-cli schema` and obtain a complete machine-readable description of every subcommand.
2. All top Tier-1 workflows (read issue, search JQL, create/update issue, add comment, transition, upload attachment, manage sprint) work end-to-end against the real 8.13.5 instance.
3. Every error surfaces as structured JSON on stderr with a stable `kind` enum and an actionable `hint`.
4. Binary size < 12 MB (release, stripped), cold start < 50 ms on typical dev hardware.
5. All integration tests pass against a `wiremock` mock, and the manual smoke checklist passes against the real Jira.

---

## 2. Requirements Summary

Confirmed via brainstorming with user:

| # | Decision | Value |
|---|---|---|
| R1 | Scope | Read + Create/Update + Workflow automation (no admin) |
| R2 | Auth | Basic Auth **and** cookie session (`/rest/auth/1/session`) |
| R3 | Output | Default JSON; JSONL for streamable list/search; `--fields` filter |
| R4 | Config | **Env vars only**, single-instance (no profile, no keychain, no config file) |
| R5 | Custom fields | Auto-discover via `/field`; accept display names in `--set`; per-invocation cache |
| R6 | Feature tiers | Tier 1 core (JQL, comments, transitions, links, attachments, worklog, watchers, schema, structured errors) + Tier 2: Agile API + bulk operations. `dry-run` and `watch` explicitly deferred |
| R7 | Language / runtime | Rust, stable, MSRV 1.75 |

---

## 3. Architecture

### 3.1 Top-level shape

Single Rust crate, binary target `jira-cli`. Layered into modules:

- `http/` — reqwest blocking client, retry, auth middleware
- `api/` — typed, stateless wrappers around Jira REST v2 + Agile 1.0 (one file per resource); no CLI or I/O concerns
- `cli/` — clap derive root + per-noun subcommand files
- `output.rs` — JSON / JSONL formatter, `--fields` filter, header sanitation
- `config.rs` — `JiraConfig::from_env()`
- `error.rs` — `thiserror` enums, stderr JSON writer
- `field_resolver.rs` — per-invocation cache of field id↔name
- `schema.rs` — `jira-cli schema` output generation

Stateless: every invocation reads env → builds client → runs one API call (or bulk batch) → emits → exits. No daemon, no on-disk session. (Cookie auth users keep the cookie in `JIRA_SESSION_COOKIE`; `session new` is a bootstrap helper that prints to stdout.)

### 3.2 Dependencies

| Purpose | Crate | Features / notes |
|---|---|---|
| HTTP | `reqwest 0.12` | `blocking`, `json`, `multipart`, `rustls-tls`, `cookies`, `gzip`; `default-features = false` (no OpenSSL) |
| CLI | `clap 4` | `derive`, `env`, `wrap_help` |
| JSON | `serde 1`, `serde_json 1` | — |
| Errors | `thiserror 1`, `anyhow 1` | `thiserror` in lib; `anyhow` only in `main.rs` |
| Logging | `tracing`, `tracing-subscriber` | JSON or compact event fmt, stderr |
| Dates | `jiff 0.1` | ISO 8601 parsing of Jira fields |
| URL | `url 2` | Path joining |
| base64 | `base64 0.22` | Basic auth header |
| Test mock | `wiremock 0.6` (dev) | Integration tests |
| CLI test | `assert_cmd`, `predicates` (dev) | End-to-end process tests |

Total release binary estimated 8–10 MB.

### 3.3 Project layout

```
jira-cli/
├── Cargo.toml
├── rust-toolchain.toml               # pin 1.75
├── README.md
├── docs/
│   └── superpowers/specs/2026-04-17-jira-cli-design.md  # this file
├── .github/workflows/ci.yml
├── src/
│   ├── main.rs
│   ├── lib.rs                        # pub use for integration tests
│   ├── config.rs
│   ├── error.rs
│   ├── output.rs
│   ├── field_resolver.rs
│   ├── schema.rs
│   ├── http/
│   │   ├── mod.rs                    # HttpClient
│   │   ├── auth.rs                   # BasicAuth / CookieSession
│   │   └── retry.rs
│   ├── api/
│   │   ├── mod.rs
│   │   ├── issue.rs
│   │   ├── search.rs
│   │   ├── transitions.rs
│   │   ├── comment.rs
│   │   ├── attachment.rs
│   │   ├── link.rs
│   │   ├── worklog.rs
│   │   ├── field.rs
│   │   ├── project.rs
│   │   ├── user.rs
│   │   ├── session.rs
│   │   └── agile.rs                  # board / sprint / epic / backlog
│   └── cli/
│       ├── mod.rs                    # Cli struct (clap root)
│       ├── args.rs                   # shared global flags
│       ├── dispatch.rs
│       └── commands/
│           ├── issue.rs
│           ├── search.rs
│           ├── field.rs
│           ├── project.rs
│           ├── user.rs
│           ├── board.rs
│           ├── sprint.rs
│           ├── epic.rs
│           ├── backlog.rs
│           ├── bulk.rs
│           ├── session.rs
│           └── meta.rs               # ping / whoami / config / schema
└── tests/
    ├── common/mod.rs                 # wiremock fixture
    ├── issue_*.rs
    ├── search_*.rs
    ├── sprint_*.rs
    ├── attachment_upload.rs
    ├── auth_cookie.rs
    └── cli/                          # assert_cmd end-to-end
```

Design principles:

- `api/` is pure-function, takes `&HttpClient + params`, returns `Result<T, Error>`. No stdout, no flag parsing. Unit-testable by request-body assertion.
- `cli/commands/` files stay small (<60 lines per verb): parse clap args → call `api::*` → `output::emit`.
- Field resolver is a per-invocation lazy cache keyed on the client. Next invocation re-fetches `/field` — this keeps the tool stateless and avoids stale-cache bugs.
- `schema.rs` introspects `clap::Command` → emits JSON describing every subcommand, flag, type, and required-ness. Schema emitted by `jira-cli schema` MUST validate against itself (self-describing).

---

## 4. CLI Surface

### 4.1 Global flags (root)

| Flag | Type | Default | Meaning |
|---|---|---|---|
| `-v` / `-vv` / `-vvv` | count | 0 | stderr log level: warn / info / debug / trace |
| `--output` | enum{json, jsonl} | auto (see below) | Output format |
| `--pretty` | bool | false | Pretty-print JSON (ignored for JSONL) |
| `--fields <csv>` | string | all | Comma-separated dot-path keys to project (applied per line for JSONL) |
| `--timeout` | u64 (sec) | 30 | HTTP timeout per request |
| `--insecure` | bool | false | Skip TLS verification |
| `-h` / `-V` | — | — | help / version |

**Default output auto-detection**:
- **JSONL** (one record per line + trailing `{"summary":…}` where applicable): `search`, any `*-list` / `*-issues` / `*-backlog` verb returning a collection, and all `bulk *` commands.
- **JSON**: every other command (single-resource `get`, create/update/delete ack, `ping`, `whoami`, `schema`, `session new`, `field resolve`).
- `--output json` / `--output jsonl` forces the format. `--fields` applies equally to both.

### 4.2 Environment variables

| Var | Required | Meaning |
|---|---|---|
| `JIRA_URL` | ✅ | Base URL e.g. `https://jira.internal.example.com` |
| `JIRA_AUTH_METHOD` | ❌ | `basic` (default) or `cookie` |
| `JIRA_USER` | basic only | Username |
| `JIRA_PASSWORD` | basic only | Password (read only; never logged) |
| `JIRA_SESSION_COOKIE` | cookie only | Full cookie string e.g. `JSESSIONID=abc...` |
| `JIRA_TIMEOUT` | ❌ | Override `--timeout` |
| `JIRA_INSECURE` | ❌ | `true` / `1` → `--insecure` |
| `JIRA_CONCURRENCY` | ❌ | Default bulk worker count (default 4, max 16) |

### 4.3 Command index

| Command | API endpoint | Notes |
|---|---|---|
| `ping` | `GET /rest/api/2/serverInfo` | Connectivity + server version |
| `whoami` | `GET /rest/api/2/myself` | Current user |
| `config show` | — | Parsed config, secrets redacted |
| `session new` | `POST /rest/auth/1/session` | stdin → stdout cookie |
| `schema [<subcmd>]` | — | CLI capability discovery |
| `issue get <KEY>` | `GET /issue/{key}` | `--expand`, `--fields` |
| `issue create` | `POST /issue` | `--project --type --summary --set …` |
| `issue bulk-create --from-file <json>` | `POST /issue/bulk` | Auto-batches >50 |
| `issue update <KEY>` | `PUT /issue/{key}` | `--set …` |
| `issue delete <KEY>` | `DELETE /issue/{key}` | `--yes` required |
| `issue assign <KEY> --user U` | `PUT /issue/{key}/assignee` | |
| `issue comment list|add|update|delete` | `…/issue/{k}/comment` | |
| `issue transitions list <K>` | `GET /issue/{k}/transitions` | |
| `issue transition <K> --to <name\|id>` | `POST /issue/{k}/transitions` | `--set` for transition fields |
| `issue link list|add|delete` | `/issueLink` | |
| `issue attachment list|upload|download|delete` | `/issue/{k}/attachments` | multipart + `X-Atlassian-Token: no-check` |
| `issue worklog list|add|delete` | `/issue/{k}/worklog` | |
| `issue watchers list|add|remove` | `/issue/{k}/watchers` | |
| `search "<JQL>"` | `POST /search` | Streaming jsonl, auto-paginate, `--max` |
| `field list [--project PRJ]` | `GET /field` [+ `/issue/createmeta`] | |
| `field resolve "<Name>"` | — | Local via field list cache |
| `project list|get|statuses` | `/project` | |
| `user get|search` | `/user` | `8.13.5` uses `name` as identifier, not `accountId` |
| `board list|get|backlog` | `/rest/agile/1.0/board` | |
| `sprint list|get|create|update|delete|move|issues` | `/rest/agile/1.0/sprint` | `move` auto-batches >50 |
| `epic get|issues|add-issues|remove-issues` | `/rest/agile/1.0/epic` | |
| `backlog move <KEY>…` | `POST /rest/agile/1.0/backlog/issue` | |
| `bulk transition --file <jsonl>` | client-side fan-out | Default 4 workers |
| `bulk comment --file <jsonl>` | client-side fan-out | |

### 4.4 `--set` syntax

Repeatable flag for create / update / transition:

- `--set "Summary=fix login bug"` — scalar string
- `--set "Priority=High"` — option value (resolved to `{"id":…}` via field schema)
- `--set 'Labels=["a","b"]'` — JSON literal array
- `--set "Description=@./desc.md"` — read file contents as string
- `--set "customfield_10020=@-"` — read from stdin
- `--set "Story Points=5"` — display-name→id translation

Value type dispatched by the field schema returned from `/field` or `/issue/{k}/editmeta`.

### 4.5 Output contracts

- **Read single**: `{...issue json as returned by Jira}` (with `--fields` projection applied)
- **Read list/search**: `jsonl`, one issue per line; final summary line `{"summary":{"count":N,"total":M}}` when `total` known
- **Write success**: `{"ok": true, "data": { ... Jira response ... }}`
- **Bulk result**: one jsonl line per item `{"key":"MGX-1","ok":true|false,"error":{…}?,"data":{…}?}` + final summary line `{"summary":{"ok":X,"failed":Y}}`
- **Error (always stderr)**: see §6

---

## 5. Data flow & key interactions

### 5.1 Lifecycle

```
main.rs
  └─ Cli::parse()
  └─ init_tracing(verbosity)
  └─ JiraConfig::from_env()   // fail fast on missing JIRA_URL etc.
  └─ HttpClient::new(&cfg)    // reqwest::blocking, rustls, cookie jar, 30s timeout, UA
  └─ dispatch::run(&client, cli)
  └─ output::emit(result, &cli)
  └─ process::exit(code)
```

### 5.2 Auth middleware (`http::auth`)

- `basic`: inject `Authorization: Basic b64(user:pass)` on every request.
- `cookie`: inject `Cookie: $JIRA_SESSION_COOKIE`. Inspect every response header `X-Seraph-LoginReason`; any value containing `AUTHENTICATION_DENIED`, `AUTHENTICATED_FAILED`, or `CAPTCHA` (Seraph emits several concrete strings like `AUTHENTICATION_DENIED_CAPTCHA_CHALLENGE` and `AUTHENTICATION_DENIED_CAPTCHA_REQUIRED`) raises `Error::Auth(CaptchaRequired)` with a hint to log in via browser (clears CAPTCHA) or regenerate cookie via `session new`.
- Always send `X-Atlassian-Token: no-check` (required for multipart, harmless elsewhere), `Accept: application/json`, `User-Agent: jira-cli/<version>`.

### 5.3 Field resolver

- Lazy: first time a display name appears in `--set`, resolver calls `GET /field` → builds `HashMap<String, FieldInfo>`.
- If multiple fields share a display name: resolver returns `FieldError::Ambiguous { name, candidates: [id, id] }`, user must use `customfield_XXXXX`.
- For value-type hinting, resolver may further call `GET /issue/createmeta?projectKeys=K&issuetypeNames=T&expand=projects.issuetypes.fields` (only when creating/editing an issue), which returns the `allowedValues` schema per field.
- Direct `customfield_XXXXX=…` bypasses the resolver.

### 5.4 Search pagination

```rust
pub struct SearchIter<'a> {
    client: &'a HttpClient,
    jql: String,
    start_at: u64,
    page_size: u64,     // 100
    total: Option<u64>,
    buf: VecDeque<Issue>,
    max: Option<u64>,
    emitted: u64,
}
impl Iterator for SearchIter { type Item = Result<Issue, Error>; ... }
```

- `output=jsonl`: emit as iterated
- `output=json`: collect into array, emit once
- `--max N`: stop after N

### 5.5 Bulk concurrency

`std::thread::scope` + `std::sync::mpsc::channel`:

```rust
thread::scope(|s| {
    let (tx, rx) = mpsc::channel();
    let items = split_batches(input, 50);         // bulk-create: 50/batch
    let chunks = chunked(items, workers);         // balance
    for chunk in chunks {
        let tx = tx.clone();
        s.spawn(move || {
            for item in chunk {
                let result = do_one(item);
                tx.send(result).unwrap();
            }
        });
    }
    drop(tx);
    for result in rx { output::emit_jsonl(result); }
});
```

- Default workers: 4 (`JIRA_CONCURRENCY` / `--concurrency` override, cap 16)
- Per-item failure does not short-circuit others
- Summary line emitted at end: `{"summary":{"ok":X,"failed":Y}}`

### 5.6 Bulk input file formats

`bulk transition --file path.jsonl` — one JSON object per line:

```jsonl
{"key":"MGX-1","to":"In Progress"}
{"key":"MGX-2","to":"Done","fields":{"resolution":{"name":"Fixed"}}}
```

`bulk comment --file path.jsonl`:

```jsonl
{"key":"MGX-1","body":"Deployed to staging."}
{"key":"MGX-2","body":"See Slack thread: ..."}
```

`issue bulk-create --from-file path.json` — a JSON **array** of `fields` objects (not JSONL; matches Jira's `/issue/bulk` native shape):

```json
[
  {"fields":{"project":{"key":"MGX"},"summary":"A","issuetype":{"name":"Task"}}},
  {"fields":{"project":{"key":"MGX"},"summary":"B","issuetype":{"name":"Task"}}}
]
```

All three also accept `--file -` to read from stdin.

---

## 6. Error handling

### 6.1 Error enum (`error.rs`)

```rust
#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("config: {0}")]
    Config(String),                 // → exit 2
    #[error("usage: {0}")]
    Usage(String),                  // → exit 2
    #[error("auth: {0}")]
    Auth(AuthError),                // → exit 5
    #[error("{resource} not found: {key}")]
    NotFound { resource: &'static str, key: String }, // → exit 6
    #[error("api error: {0}")]
    Api(ApiErrorBody),              // → exit 3
    #[error("network: {0}")]
    Network(#[from] reqwest::Error),// → exit 4
    #[error("serialization: {0}")]
    Serialization(#[from] serde_json::Error), // → exit 7
    #[error("field: {0}")]
    FieldResolve(FieldError),       // → exit 2
    #[error("io: {0}")]
    Io(#[from] std::io::Error),     // → exit 7
}

pub struct ApiErrorBody {
    pub status: u16,
    pub error_messages: Vec<String>,
    pub errors: BTreeMap<String, String>,
    pub request_id: Option<String>,     // X-ARequestId
}

pub enum AuthError { Unauthorized, Forbidden, CaptchaRequired, CookieExpired }
pub enum FieldError { Unknown(String), Ambiguous { name: String, candidates: Vec<String> } }
```

### 6.2 stderr JSON

Every non-zero exit writes a single JSON object to stderr:

```json
{
  "error": {
    "kind": "api_error",
    "message": "Issue does not exist or you do not have permission to see it.",
    "status": 404,
    "errorMessages": ["Issue Does Not Exist"],
    "errors": {},
    "request_id": "abc123",
    "hint": "Verify the issue key with `jira-cli search \"key = MGX-1\"` or check permissions with `jira-cli whoami`"
  }
}
```

`kind` ∈ `config | usage | auth | not_found | api_error | network | serialization | field_resolve | io`.

### 6.3 Exit codes

| Code | Meaning |
|---|---|
| 0 | Success |
| 2 | Usage / config error |
| 3 | Jira business error |
| 4 | Network error |
| 5 | Auth error (incl. CAPTCHA) |
| 6 | Not found |
| 7 | Internal / IO / serialization |

### 6.4 Retry

- **GET**: 429 / 5xx / network errors → exponential backoff (100 ms, 400 ms, 1.6 s, 6.4 s), max 3 retries. Respect `Retry-After`.
- **POST / PUT / DELETE**: only on **connect / DNS / timeout-before-send** (request never reached server). **No** 5xx retry on writes by default. `--retry-writes` opt-in.
- On each retry, if `-v`, emit stderr `{"retry":{"attempt":N,"wait_ms":X,"reason":"…"}}`.

### 6.5 Non-JSON error body fallback

Jira sometimes returns an HTML login page or an upstream proxy error:

- Capture status + first 200 bytes of text, emit as `api_error` with `errorMessages: ["<raw excerpt>"]` + `hint: "non-JSON response; check URL or auth"`.

### 6.6 CAPTCHA detection

Response header `X-Seraph-LoginReason` containing `AUTHENTICATION_DENIED`, `AUTHENTICATED_FAILED`, or `CAPTCHA` → `Error::Auth(CaptchaRequired)`, exit 5, hint: "log in once via browser to clear CAPTCHA, or regenerate cookie with `jira-cli session new`".

---

## 7. Testing strategy

### 7.1 Pyramid

**Unit (inside `src/`)**
- `config.rs`: env combinations (valid / missing / conflicting auth mode)
- `field_resolver.rs`: success / ambiguous / unknown / customfield direct
- `output.rs`: `--fields` projection, JSON vs JSONL, header sanitization
- `error.rs`: Jira body parse (standard / partial / HTML fallback)
- `api/*`: request body construction (given params → expected JSON)

**Integration (`tests/` + `wiremock`)**

One happy path + one error path minimum per resource. Must-cover scenarios:

- `issue get` 404 → exit 6 + stderr JSON
- `issue bulk-create` with 120 items → expect 3 batched calls (50 + 50 + 20)
- `search` 3-page pagination → 3 mock responses, verify all issues emitted
- `sprint move` with 60 keys → 2 batched calls
- `attachment upload` → multipart body + `X-Atlassian-Token` header asserted
- `session new` → POST body asserted, `JSESSIONID` parsed from response
- Cookie auth + `X-Seraph-LoginReason: AUTHENTICATION_DENIED` → exit 5
- 429 retry: first response 429 + `Retry-After: 1`, second 200 → success
- Write 5xx no retry by default

Fixture: `tests/common/mod.rs::spawn_mock()` returns `(MockServer, HttpClient)`.

**CLI end-to-end (`tests/cli/`)** using `assert_cmd`:
- `jira-cli schema` output validates against itself via `jsonschema`
- Exit-code matrix: at least one test producing each of 0, 2, 3, 4, 5, 6, 7
- Every error scenario: stderr parses as valid JSON

**Contract smoke (`tests/contract/`, gated on `JIRA_URL` env)**
- Run only when real Jira is reachable (`just smoke`)
- Non-destructive: `ping`, `whoami`, `field list`, `search "project = <TEST_PROJECT>"` with `--max 1`
- Not on CI by default; catches patch-level schema drift

### 7.2 CI

`.github/workflows/ci.yml`:

- Matrix: `{ubuntu-latest, macos-latest} × {stable, 1.75}`; `windows-latest` build-only
- Steps: `fmt --check` → `clippy -D warnings` → `test --all-features` → `deny check` → `build --release`
- Release job: on tag push, cross-build 4 targets (`x86_64-unknown-linux-gnu`, `aarch64-unknown-linux-gnu`, `x86_64-apple-darwin`, `aarch64-apple-darwin`), publish GitHub Release with SHA256 checksums

### 7.3 Manual acceptance checklist

Run before tagging v0.1.0:

- [ ] `ping` and `whoami` succeed against real 8.13.5
- [ ] Both `basic` and `cookie` auth modes connect
- [ ] `issue get EXISTING-1` returns expected JSON; `issue get NONEXISTENT-404` exits 6 with structured JSON
- [ ] `issue create` + `issue transition` + `issue comment add` flow against a test issue
- [ ] `search` with >100 results streams correctly under `--output jsonl`
- [ ] `attachment upload` + `download` round-trips identical content
- [ ] `sprint move` with >50 keys auto-batches
- [ ] `bulk transition` with 20-item file + 4 workers; injected failure for one row continues others
- [ ] `jira-cli schema | jq .` parses cleanly; schema describes itself

---

## 8. Security considerations

- Basic auth password read from `JIRA_PASSWORD` only; never logged (tracing `Debug` impl on config type manually redacts).
- Response bodies are not logged at `info`; full bodies only at `trace` (user must `-vvv`).
- TLS via rustls by default. `--insecure` / `JIRA_INSECURE=1` gated and loud (stderr warn).
- No on-disk secrets. Cookie persistence is the user's responsibility (env var).
- No telemetry, no remote update check.

---

## 9. Open questions / deferred

| # | Question | Resolution deferred to |
|---|---|---|
| Q1 | Do agents ever need OAuth 1.0a? | v0.1 says no (Basic + cookie only). Revisit if SSO-only Jira setups break cookie bootstrap. |
| Q2 | Should `field list` cache to disk across invocations? | v0.1 says no (stateless). Consider if latency on `--set` translation becomes a real problem. |
| Q3 | Do we want `jira-cli watch KEY`? | v0.1 says no (use `cron + issue get`). Reconsider if agents routinely build polling loops. |
| Q4 | Windows support beyond build-only? | v0.1 build-only. Add tests when a real consumer emerges. |
| Q5 | `--fields` dot-path vs JSONPath? | Start with dot-path (`fields.summary,fields.status.name`); upgrade to jsonpath later if users demand. |
| Q6 | `--dry-run` for write ops? | Not in v0.1 (per user scope decision). Low-cost to add later if agent safety demands; implementation would intercept after request construction and print sanitized `{method,url,headers,body}`. |

---

## 10. Future work (out of scope v1)

- Full Jira workflow editing / admin endpoints
- MCP server wrapper reusing `api/` as a library (trivial after extracting `api/` into a `jira-client` crate)
- Pretty human-readable renderer (`--output table`)
- Incremental cache for `/field` across invocations (dot-dir or SQLite)
- PAT support once the team upgrades past 8.14

---

## Appendix A — Representative JSON schemas

### Issue (abridged)
```json
{
  "id": "10001",
  "key": "MGX-42",
  "fields": {
    "summary": "Fix login bug",
    "status": {"name": "In Progress", "id": "3"},
    "assignee": {"name": "alice", "displayName": "Alice Chen"},
    "priority": {"name": "High", "id": "2"},
    "customfield_10020": 5,
    "created": "2026-04-17T09:30:00.000+0800"
  }
}
```

### Error body
```json
{"errorMessages":["Issue Does Not Exist"],"errors":{},"status":404}
```

### `jira-cli schema` excerpt
```json
{
  "version": "0.1.0",
  "commands": {
    "issue": {
      "subcommands": {
        "get": {
          "args": [{"name":"key","required":true,"type":"string"}],
          "flags": [{"name":"expand","type":"csv"},{"name":"fields","type":"csv"}],
          "output": "issue-object"
        }
      }
    }
  }
}
```

## Appendix B — Command → API endpoint map

Full mapping is in §4.3. Additional notes:

- `user get <U>` uses `GET /rest/api/2/user?username=U` (Server 8.x identifier is `name`, not `accountId`).
- `attachment download` follows the `content` URL from attachment metadata with the same auth headers. Default output path: `./<original-filename>`; `--out <path>` overrides (use `--out -` for stdout).
- `issue transition <KEY> --to <name|id>`: first fetches `GET /issue/{k}/transitions` to resolve name→id (adds one round-trip); numeric `--to` skips the lookup.
- `issue bulk-create`: Jira's native `/issue/bulk` caps each call at 50 issues; CLI auto-splits larger inputs and merges the responses into one JSONL stream.
