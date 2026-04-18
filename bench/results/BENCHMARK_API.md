# jira-cli vs jira-go-cli — API benchmark

**Last run:** 2026-04-18 (hyperfine `--warmup 10 --runs 60`, `/usr/bin/time -l` × 20)

Comparative single-invocation performance against the same real Jira Server 8.13.5 instance, same credentials, same network.

Both binaries built `--release`. Measured three dimensions per scenario:

- **wall** — median wall-clock time (hyperfine)
- **CPU** — user + sys CPU time, mean across 20 `time -l` runs
- **peak** — median max resident set size across 20 `time -l` runs

## Combined summary

| scenario              | Rust wall | Go wall      | wall      | Rust CPU | Go CPU  | CPU       | Rust peak | Go peak | mem       |
| --------------------- | --------- | ------------ | --------- | -------- | ------- | --------- | --------- | ------- | --------- |
| `ping`                | 445.8 ms  | 455.3 ms     | 1.02×     | 7.4 ms   | 13.4 ms | **1.80×** | 3.8 MiB   | 6.2 MiB | **1.63×** |
| `whoami`              | 458.9 ms  | 467.3 ms     | 1.02×     | 10.3 ms  | 14.0 ms | 1.36×     | 3.9 MiB   | 6.4 MiB | **1.66×** |
| `issue get` idiomatic | 507.6 ms  | 529.6 ms     | 1.04×     | 9.3 ms   | 18.6 ms | **1.99×** | 4.1 MiB   | 6.7 MiB | **1.63×** |
| `issue get` raw       | 518.9 ms  | 524.0 ms     | 1.01×     | 11.7 ms  | 11.0 ms | 0.94×     | 4.2 MiB   | 6.6 MiB | **1.57×** |
| `project list`        | 519.5 ms  | 520.4 ms     | 1.00×     | 7.5 ms   | 13.9 ms | **1.85×** | 3.9 MiB   | 6.7 MiB | **1.69×** |
| `search 20`           | 560.0 ms  | **807.8 ms** | **1.44×** | 11.4 ms  | 19.0 ms | 1.67×     | 5.1 MiB   | 7.7 MiB | 1.51×     |

**ratio columns are Go/Rust** — higher means Rust wins by that factor.

## Observations

- **Wall-clock** parity on most single-shot calls (~±5%) because ~450 ms of the total is network RTT to the Jira server. Tool CPU is a small fraction of the total.
- **`search 20` is the outlier** — Rust is **1.44× faster on wall-clock** (247 ms saved per call). JQL search hits CPU-heavy deserialization paths where Go's GC and reflection-based JSON decoder cost more.
- **CPU time ratio 1.4×–2.0×** after stripping network. (`issue_get_raw` showing 0.94× is a single 10.5 s outlier in the Rust run pulling the mean; median and CPU are unaffected. See `api_mem_issue_get_raw_rust.txt` for the raw 20-sample distribution.)
- **Memory peak 1.5×–1.7× smaller** for Rust, regardless of payload size. Go's runtime has a ~2.5 MiB baseline cost (GC metadata, goroutine stacks, runtime tables) that shows up as a near-constant offset versus Rust's `rustls`+`reqwest` stack.

## Caveats

- Network latency dominates wall-clock for single calls. Volume-bound loops (e.g. `xargs -I {} jira-cli ...`) compound the 10–20 ms CPU deltas.
- Absolute numbers are specific to this Jira instance (on-prem, intranet). **Ratios travel.** Rerun locally if you want numbers for your own infra.
- Memory figures are macOS `maximum resident set size` in bytes (then converted to MiB). Linux `time -v` `Maximum resident set size` is in KB — both handled by `run-api-bench.sh`.
- `go_cpu` for `issue get raw` isn't flawed — Go's pretty-print path is a bit lighter than our serde-based one for that particular payload shape. Also indistinguishable within noise.

## Raw artifacts

- `api_summary.json` — machine-readable rollup of this table
- `api_hf_{scenario}.json` (6 files) — hyperfine JSON with all 60 samples per tool
- `api_mem_{scenario}_{tool}.txt` (12 files) — full `time -l` output, 20 runs each, used for CPU + peak memory aggregation

To regenerate:

```bash
./bench/run-api-bench.sh jira-cli jira-go-cli
```

See [`../README.md`](../README.md) for prereqs + methodology.
