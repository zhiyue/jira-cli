#!/usr/bin/env bash
# API benchmark harness.
#
# Runs 6 scenarios against a real Jira instance, comparing two CLI binaries
# via hyperfine (wall-clock) and /usr/bin/time -l (CPU + maxrss).
#
# Usage:
#     ./bench/run-api-bench.sh [RUST_BIN] [GO_BIN]
# Defaults to `jira-cli` and `jira-go-cli` on PATH.
#
# Env overrides:
#     JIRA_URL, JIRA_USER, JIRA_PASSWORD — or rely on config files
#     ISSUE_KEY (default MGX-14313)
#     PROJECT_KEY (default MGX)
#     HF_WARMUP (default 10)
#     HF_RUNS (default 60)
#     MEM_RUNS (default 20)
set -euo pipefail

RUST_BIN="${1:-jira-cli}"
GO_BIN="${2:-jira-go-cli}"
ISSUE_KEY="${ISSUE_KEY:-MGX-14313}"
PROJECT_KEY="${PROJECT_KEY:-MGX}"
HF_WARMUP="${HF_WARMUP:-10}"
HF_RUNS="${HF_RUNS:-60}"
MEM_RUNS="${MEM_RUNS:-20}"

cd "$(dirname "$0")/.."
OUT="bench/results"
mkdir -p "$OUT"

command -v hyperfine >/dev/null || { echo "hyperfine not found; brew install hyperfine" >&2; exit 1; }
command -v "$RUST_BIN" >/dev/null || { echo "$RUST_BIN not on PATH" >&2; exit 1; }
command -v "$GO_BIN" >/dev/null || { echo "$GO_BIN not on PATH" >&2; exit 1; }

# macOS ships /usr/bin/time with -l (maxrss in bytes); GNU time uses -v.
if /usr/bin/time -l true 2>/dev/null; then
    TIME_FLAG="-l"
    MEM_PARSE='maximum resident set size'
else
    TIME_FLAG="-v"
    MEM_PARSE='Maximum resident set size'
fi

# ---- Scenario matrix ----
# Each row: id, description, rust cmd, go cmd
declare -a SCENARIOS=(
    "ping|ping|$RUST_BIN ping|$GO_BIN ping"
    "whoami|whoami|$RUST_BIN whoami|$GO_BIN whoami"
    "issue_get|issue get (idiomatic, uses default jira-fields)|$RUST_BIN issue get $ISSUE_KEY|$GO_BIN issue view $ISSUE_KEY"
    "issue_get_raw|issue get (raw — all fields)|$RUST_BIN issue get --jira-fields '' $ISSUE_KEY|$GO_BIN issue view $ISSUE_KEY"
    "project_list|project list|$RUST_BIN project list|$GO_BIN project list"
    "search20|search 20 issues|$RUST_BIN search 'project = $PROJECT_KEY' --max 20|$GO_BIN issue list -p $PROJECT_KEY -m 20"
)

echo "# jira-cli API benchmark — $(date -u +"%Y-%m-%dT%H:%M:%SZ")" > "$OUT/BENCHMARK_API.md"
{
    echo
    echo "- Rust binary: \`$RUST_BIN\` ($(command -v "$RUST_BIN"))"
    echo "- Go binary:   \`$GO_BIN\` ($(command -v "$GO_BIN"))"
    echo "- hyperfine:   warmup=$HF_WARMUP runs=$HF_RUNS"
    echo "- time -l:     $MEM_RUNS runs/scenario/tool"
    echo "- Issue key:   $ISSUE_KEY"
    echo "- Project:     $PROJECT_KEY"
    echo
    echo "## Summary"
    echo
    echo "| scenario | Rust wall | Go wall | wall | Rust CPU | Go CPU | CPU | Rust peak | Go peak | mem |"
    echo "|---|---:|---:|:-:|---:|---:|:-:|---:|---:|:-:|"
} >> "$OUT/BENCHMARK_API.md"

# Track summary data for JSON rollup
JSON_ROWS=()

for row in "${SCENARIOS[@]}"; do
    id="${row%%|*}"
    rest="${row#*|}"
    desc="${rest%%|*}"
    rest="${rest#*|}"
    rust_cmd="${rest%%|*}"
    go_cmd="${rest#*|}"

    echo "::: scenario $id — $desc"
    echo "    Rust: $rust_cmd"
    echo "    Go:   $go_cmd"

    HF_FILE="$OUT/api_hf_${id}.json"
    hyperfine \
        --warmup "$HF_WARMUP" --runs "$HF_RUNS" \
        --export-json "$HF_FILE" \
        --command-name "rust" "bash -c \"$rust_cmd > /dev/null\"" \
        --command-name "go"   "bash -c \"$go_cmd   > /dev/null\""

    # Pull median wall
    rust_wall_ms=$(jq -r '.results[] | select(.command=="rust") | .median*1000' "$HF_FILE")
    go_wall_ms=$(jq -r '.results[] | select(.command=="go")   | .median*1000' "$HF_FILE")

    # Memory + CPU via time -l across $MEM_RUNS runs each
    for tool in rust go; do
        cmd_var="${tool}_cmd"
        cmd="${!cmd_var}"
        MEM_FILE="$OUT/api_mem_${id}_${tool}.txt"
        : > "$MEM_FILE"
        for _ in $(seq 1 "$MEM_RUNS"); do
            { /usr/bin/time $TIME_FLAG bash -c "$cmd > /dev/null"; } 2>> "$MEM_FILE" || true
        done
    done

    # Parse CPU (user+sys) and peak mem
    parse_mem_cpu() {
        local file="$1"
        # Fields on macOS: "XXXXX  maximum resident set size" in bytes.
        # CPU: "real sys user" lines or "X.XX user  X.XX sys" — mac shows both.
        local peak_bytes cpu_secs
        peak_bytes=$(awk '/'"$MEM_PARSE"'/ {print $1}' "$file" | sort -n | awk 'BEGIN{c=0} {a[c++]=$1} END{print a[int(c/2)]}')
        cpu_secs=$(awk '/user/ && /sys/ {for(i=1;i<=NF;i++) if($i=="user") u+=$(i-1); if ($i=="sys") s+=$(i-1); n++} END{if(n>0) print (u+s)/n}' "$file")
        # On macOS, `time -l` prints `X.YY user  X.YY sys`; parse both:
        if [ -z "$cpu_secs" ] || [ "$cpu_secs" = "0" ]; then
            cpu_secs=$(awk '
                /user$/ {u += $(NF-1); c++}
                /sys$/  {s += $(NF-1)}
                END{if (c>0) printf "%.6f\n", (u+s)/c}
            ' "$file")
        fi
        echo "$peak_bytes $cpu_secs"
    }

    read -r rust_peak rust_cpu <<< "$(parse_mem_cpu "$OUT/api_mem_${id}_rust.txt")"
    read -r go_peak   go_cpu   <<< "$(parse_mem_cpu "$OUT/api_mem_${id}_go.txt")"

    # Normalize peak to MiB on macOS (bytes) vs KB on linux (kb)
    if [[ "$(uname)" == "Darwin" ]]; then
        rust_peak_mib=$(awk -v b="$rust_peak" 'BEGIN{printf "%.1f", b/1048576}')
        go_peak_mib=$(awk -v b="$go_peak"   'BEGIN{printf "%.1f", b/1048576}')
    else
        rust_peak_mib=$(awk -v kb="$rust_peak" 'BEGIN{printf "%.1f", kb/1024}')
        go_peak_mib=$(awk -v kb="$go_peak"   'BEGIN{printf "%.1f", kb/1024}')
    fi
    rust_cpu_ms=$(awk -v s="$rust_cpu" 'BEGIN{printf "%.1f", s*1000}')
    go_cpu_ms=$(awk -v s="$go_cpu"     'BEGIN{printf "%.1f", s*1000}')

    wall_ratio=$(awk -v r="$rust_wall_ms" -v g="$go_wall_ms" 'BEGIN{printf "%.2f", g/r}')
    cpu_ratio=$(awk -v r="$rust_cpu_ms"   -v g="$go_cpu_ms"   'BEGIN{printf "%.2f", g/r}')
    mem_ratio=$(awk -v r="$rust_peak_mib" -v g="$go_peak_mib" 'BEGIN{printf "%.2f", g/r}')

    printf "| %s | %.1f ms | %.1f ms | %sx | %s ms | %s ms | %sx | %s MiB | %s MiB | %sx |\n" \
        "$id" "$rust_wall_ms" "$go_wall_ms" "$wall_ratio" \
        "$rust_cpu_ms" "$go_cpu_ms" "$cpu_ratio" \
        "$rust_peak_mib" "$go_peak_mib" "$mem_ratio" >> "$OUT/BENCHMARK_API.md"

    JSON_ROWS+=("{\"id\":\"$id\",\"desc\":\"$desc\",\"rust_wall_ms\":$rust_wall_ms,\"go_wall_ms\":$go_wall_ms,\"wall_ratio\":$wall_ratio,\"rust_cpu_ms\":$rust_cpu_ms,\"go_cpu_ms\":$go_cpu_ms,\"cpu_ratio\":$cpu_ratio,\"rust_peak_mib\":$rust_peak_mib,\"go_peak_mib\":$go_peak_mib,\"mem_ratio\":$mem_ratio}")
done

# JSON summary
{
    echo "{"
    echo "  \"generated_at\": \"$(date -u +"%Y-%m-%dT%H:%M:%SZ")\","
    echo "  \"rust_bin\": \"$(command -v "$RUST_BIN")\","
    echo "  \"go_bin\": \"$(command -v "$GO_BIN")\","
    echo "  \"hf_warmup\": $HF_WARMUP,"
    echo "  \"hf_runs\": $HF_RUNS,"
    echo "  \"mem_runs\": $MEM_RUNS,"
    echo "  \"scenarios\": ["
    printf "    %s" "${JSON_ROWS[0]}"
    for row in "${JSON_ROWS[@]:1}"; do
        printf ",\n    %s" "$row"
    done
    echo ""
    echo "  ]"
    echo "}"
} > "$OUT/api_summary.json"

echo
echo "Done. See $OUT/BENCHMARK_API.md and $OUT/api_summary.json"
