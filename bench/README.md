# Benchmarks

Performance benchmarks comparing this Rust `jira-cli` to the Go [`jira-go-cli`](https://github.com/zhiyue/jira-go-cli) for equivalent operations.

## Why benchmarks

Single invocation performance matters when:

- an agent dispatches many `jira-cli` calls in a loop (cold-start cost compounds)
- wrapped in shell scripts that iterate issues
- running on constrained CI runners

We measure three dimensions:

- **Wall-clock time** via [`hyperfine`](https://github.com/sharkdp/hyperfine) — user-facing latency
- **CPU time** via `/usr/bin/time -l` (`user + sys`) — work excluding network I/O
- **Peak memory** via `/usr/bin/time -l` `max resident set size` — memory footprint

## Methodology

- Both binaries built in `--release` mode against the **same real Jira Server 8.13.5** instance
- Same user credentials for apples-to-apples comparison
- `hyperfine --warmup 10 --runs 60` per scenario (rejects hot-cold jitter)
- `/usr/bin/time -l` 20 runs per scenario to measure peak memory (maxrss)
- Six scenarios spanning read-only hot paths:
  1. `ping` — minimal round-trip
  2. `whoami` — parsed user object
  3. `issue get` (idiomatic — using default `--jira-fields`)
  4. `issue get` (raw passthrough `--jira-fields ""`)
  5. `project list` — flat JSON array
  6. `search 20` — 20 issues via JQL

## Scope & caveats

- Network latency dominates wall-clock (~450ms base). Only `search 20` pulls significantly ahead in wall-clock (1.44×).
- CPU and memory isolate tool overhead from network.
- Results are for one specific Jira instance; absolute numbers will vary elsewhere. **Ratios** are what matter.
- Benchmarks **cannot run in CI** (need a live Jira). Run them locally before tagging a release if you want to refresh the numbers.

## How to run

```bash
# prereqs
brew install hyperfine       # macOS
# apt install hyperfine      # Debian-ish

export JIRA_URL=...           # or rely on ~/.config/jira-cli/config.toml
export ISSUE_KEY=MGX-14313    # pick an issue you can read
export PROJECT_KEY=MGX

./bench/run-api-bench.sh jira-cli jira-go-cli
# Results land in bench/results/
```

The script produces:

```
bench/results/
├── BENCHMARK_API.md      — human-readable comparison table
├── api_summary.json      — machine-readable rollup
├── api_hf_{scenario}.json (× 6)   — raw hyperfine JSON per scenario
└── api_mem_{scenario}_{tool}.txt (× 12)   — raw `time -l` output per scenario × tool
```

## Latest results

See [`results/BENCHMARK_API.md`](results/BENCHMARK_API.md).
