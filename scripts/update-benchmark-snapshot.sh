#!/usr/bin/env bash
# Update benchmark-results/snapshot.json from Criterion output.
#
# Reads mean point estimates (ns) from target/criterion/<bench>/new/estimates.json
# for the headline benches and emits a flat snapshot conforming to the schema
# consumed by terraphim.ai (content/data/benchmarks.json). Unmatched metrics
# fall back to the hand-curated static values from the 2025-11-11 report.
#
# Tracker: Gitea zestic-ai/terraphim-ai #574
# Design:  terraphim.ai .docs/design-ship-systems-benchmarks.md

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
CRITERION_DIR="${PROJECT_ROOT}/target/criterion"
OUT_DIR="${PROJECT_ROOT}/benchmark-results"
OUT_FILE="${OUT_DIR}/snapshot.json"

mkdir -p "$OUT_DIR"

# --- helpers ---------------------------------------------------------------

# Read mean.point_estimate (ns) from a Criterion estimates.json.
# Echoes the number or an empty string if missing.
read_mean_ns() {
    local file="$1"
    if [ -f "$file" ]; then
        jq -r '.mean.point_estimate // empty' "$file" 2>/dev/null || true
    fi
}

# Format a ns value to a target unit with a given decimal precision.
# Usage: fmt_ns_to <ns> <us|ms|s> <precision>
fmt_ns_to() {
    local ns="$1" unit="$2" precision="$3"
    if [ -z "$ns" ]; then
        echo ""
        return
    fi
    case "$unit" in
        us) awk -v n="$ns" -v p="$precision" 'BEGIN { printf "%.*f", p, n/1000 }' ;;
        ms) awk -v n="$ns" -v p="$precision" 'BEGIN { printf "%.*f", p, n/1000000 }' ;;
        s)  awk -v n="$ns" -v p="$precision" 'BEGIN { printf "%.*f", p, n/1000000000 }' ;;
        ns) awk -v n="$ns" -v p="$precision" 'BEGIN { printf "%.*f", p, n }' ;;
        *)  echo "" ;;
    esac
}

# --- extract metrics -------------------------------------------------------

# index_build_100_terms: mean of build_index_throughput at 100 terms (ns -> us).
# Criterion parameter directory naming varies; probe common shapes.
find_build_bench() {
    local size="$1"
    for candidate in \
        "$CRITERION_DIR/build_index_throughput/${size}/new/estimates.json" \
        "$CRITERION_DIR/build_index_throughput/${size}_terms/new/estimates.json" \
        "$CRITERION_DIR/build_index_throughput/terms_${size}/new/estimates.json"
    do
        if [ -f "$candidate" ]; then
            echo "$candidate"
            return
        fi
    done
    echo ""
}

BUILD_100_FILE="$(find_build_bench 100)"
BUILD_1000_FILE="$(find_build_bench 1000)"

BUILD_100_NS="$(read_mean_ns "$BUILD_100_FILE")"
BUILD_1000_NS="$(read_mean_ns "$BUILD_1000_FILE")"

BUILD_100_VAL="$(fmt_ns_to "$BUILD_100_NS" us 0)"
BUILD_1000_VAL="$(fmt_ns_to "$BUILD_1000_NS" ms 2)"

# Static fallbacks from the 2025-11-11 test report when Criterion output is
# absent (e.g. CI hasn't run the bench yet, or a specific parameter label
# drifted). These match content/data/benchmarks.json in terraphim.ai and keep
# the homepage coherent until a real run populates them.
STATIC_INFERENCE='"5-10"'
STATIC_SEARCH_LATENCY='"< 1"'
STATIC_THROUGHPUT='"~310"'
STATIC_FOOTPRINT='"15-20"'
STATIC_BUILD_100='"~301"'
STATIC_BUILD_1000='"~3.66"'

# Prefer Criterion-measured values where present.
if [ -n "$BUILD_100_VAL" ]; then
    BUILD_100_JSON="\"~${BUILD_100_VAL}\""
    BUILD_100_SOURCE="criterion"
else
    BUILD_100_JSON="$STATIC_BUILD_100"
    BUILD_100_SOURCE="static-fallback"
fi

if [ -n "$BUILD_1000_VAL" ]; then
    BUILD_1000_JSON="\"~${BUILD_1000_VAL}\""
    BUILD_1000_SOURCE="criterion"
else
    BUILD_1000_JSON="$STATIC_BUILD_1000"
    BUILD_1000_SOURCE="static-fallback"
fi

# --- metadata --------------------------------------------------------------

GENERATED_AT="$(date -u +%Y-%m-%dT%H:%M:%SZ)"
COMMIT_SHA="${GITHUB_SHA:-$(git -C "$PROJECT_ROOT" rev-parse HEAD 2>/dev/null || echo "unknown")}"
WORKFLOW_RUN_URL="${WORKFLOW_RUN_URL:-null}"
if [ "$WORKFLOW_RUN_URL" = "null" ]; then
    WORKFLOW_RUN_JSON="null"
else
    WORKFLOW_RUN_JSON="\"$WORKFLOW_RUN_URL\""
fi

NOTES="Produced by scripts/update-benchmark-snapshot.sh. Sources: index_build_100_terms=${BUILD_100_SOURCE}, index_build_1000_terms=${BUILD_1000_SOURCE}. Other metrics (inference_ns, search_latency_ms, graph_footprint_mb, index_throughput_mibps) are currently static fallbacks awaiting direct Criterion wiring."

# --- emit ------------------------------------------------------------------

cat > "$OUT_FILE" <<EOF
{
  "generated_at": "${GENERATED_AT}",
  "source": {
    "repo": "terraphim/terraphim-ai",
    "commit": "${COMMIT_SHA}",
    "workflow_run": ${WORKFLOW_RUN_JSON},
    "notes": "$(echo "$NOTES" | sed 's/"/\\"/g')"
  },
  "metrics": {
    "inference_ns": {
      "label": "Inference",
      "value": ${STATIC_INFERENCE},
      "unit": "ns"
    },
    "search_latency_ms": {
      "label": "Search query",
      "value": ${STATIC_SEARCH_LATENCY},
      "unit": "ms"
    },
    "index_throughput_mibps": {
      "label": "Index throughput",
      "value": ${STATIC_THROUGHPUT},
      "unit": "MiB/s"
    },
    "graph_footprint_mb": {
      "label": "Graph footprint",
      "value": ${STATIC_FOOTPRINT},
      "unit": "MB"
    },
    "index_build_100_terms_us": {
      "label": "Index build (100 terms)",
      "value": ${BUILD_100_JSON},
      "unit": "\u00b5s"
    },
    "index_build_1000_terms_ms": {
      "label": "Index build (1,000 terms)",
      "value": ${BUILD_1000_JSON},
      "unit": "ms"
    }
  }
}
EOF

# --- validate --------------------------------------------------------------

if ! jq empty "$OUT_FILE" 2>/dev/null; then
    echo "ERROR: produced $OUT_FILE is not valid JSON" >&2
    exit 1
fi

METRIC_COUNT="$(jq '.metrics | length' "$OUT_FILE")"
if [ "$METRIC_COUNT" -ne 6 ]; then
    echo "ERROR: expected 6 metrics in snapshot, found $METRIC_COUNT" >&2
    exit 1
fi

echo "Wrote $OUT_FILE"
echo "  generated_at: $GENERATED_AT"
echo "  commit:       $COMMIT_SHA"
echo "  build_100:    $BUILD_100_SOURCE"
echo "  build_1000:   $BUILD_1000_SOURCE"
