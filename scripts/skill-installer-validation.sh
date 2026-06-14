#!/bin/bash
# skill-installer-validation.sh -- Terraphim Skills multi-CLI validation runner
#
# Used by the ADF skills-installer / ci-native agent to install and verify
# Terraphim skills across Claude Code, Codex, Pi, pi-rust, and Grok.
#
# Environment:
#   TERRAPHIM_SKILLS_SENTINEL_SKILLS   Comma-separated skill names (default: code-review)
#   TERRAPHIM_SKILLS_CATALOGUE         Path to JSON catalogue manifest (overrides sentinel)
#   TERRAPHIM_SKILLS_MAX_CELLS         Safety cap on total cells (default: 1000)
#   TERRAPHIM_SKILLS_MANAGER_BIN       Skills manager binary (default: tsm)
#   TERRAPHIM_SKILLS_GROK_DIR          Grok skills install directory (default: $HOME/.grok/skills)
#   TERRAPHIM_SKILLS_PI_DIR            Pi skills install directory (default: $HOME/.pi/agent/skills)
#   TERRAPHIM_SKILLS_DRY_RUN           Set to 1 to skip live install/verify (ci-native mode)
#   TERRAPHIM_SKILLS_TARGET_CLIS       Comma-separated target CLIs (default: claude-code,codex,pi,pi-rust,grok)
#   TSM_API_URL                        Marketplace API base URL
#   TSM_TOKEN                          Bearer token for subscription-gated downloads
#
# Output:
#   JSON report to stdout, Markdown matrix to stderr

set -euo pipefail

# --- Helpers -----------------------------------------------------------------
# Strip ANSI escape codes (CSI sequences) from a string
strip_ansi() {
    printf '%s' "$1" | perl -pe 's/\e\[[0-9;]*[a-zA-Z]//g; s/\e\][^\e\007]*(\e\007|\e\\)//g'
}

# Sanitize evidence for safe JSON embedding
sanitize_evidence() {
    local s="$1"
    s=$(strip_ansi "$s")
    # Truncate to reasonable length
    s=$(printf '%s' "$s" | head -c 200)
    # Escape backslashes and double quotes for JSON
    s="${s//\\/\\\\}"
    s="${s//\"/\\\"}"
    s="${s//	/ }"
    # Collapse newlines
    s=$(printf '%s' "$s" | tr '\n' ' ' | tr -s ' ')
    printf '%s' "$s"
}

# --- Configuration -----------------------------------------------------------
SENTINEL_SKILLS="${TERRAPHIM_SKILLS_SENTINEL_SKILLS:-code-review}"
CATALOGUE="${TERRAPHIM_SKILLS_CATALOGUE:-}"
MAX_CELLS="${TERRAPHIM_SKILLS_MAX_CELLS:-1000}"
MANAGER_BIN="${TERRAPHIM_SKILLS_MANAGER_BIN:-tsm}"
GROK_DIR="${TERRAPHIM_SKILLS_GROK_DIR:-$HOME/.grok/skills}"
PI_DIR="${TERRAPHIM_SKILLS_PI_DIR:-$HOME/.pi/agent/skills}"
PI_RUST_DIR="${TERRAPHIM_SKILLS_PI_RUST_DIR:-$HOME/.agents/skills}"
TARGET_CLIS="${TERRAPHIM_SKILLS_TARGET_CLIS:-claude-code,codex,pi,pi-rust,grok}"
DRY_RUN="${TERRAPHIM_SKILLS_DRY_RUN:-0}"
WORKSPACE="${TERRAPHIM_SKILLS_WORKSPACE:-}"
CATALOGUE_SOURCE="sentinel"

if [ "$DRY_RUN" = "0" ]; then
    if [ -z "${TSM_TOKEN:-}" ] && [ ! -s "$HOME/.terraphim/token" ]; then
        echo "FAIL: TSM_TOKEN not set and ~/.terraphim/token not found; set TERRAPHIM_SKILLS_DRY_RUN=1 for ci-native" >&2
        exit 1
    fi
fi

# --- Load skills from catalogue if specified --------------------------------
if [ -n "$CATALOGUE" ] && [ -f "$CATALOGUE" ]; then
    if command -v python3 >/dev/null 2>&1; then
        CATALOGUE_NAMES=$(python3 -c "
import json, sys
with open('$CATALOGUE') as f:
    d = json.load(f)
names = [s.get('name','') for s in d.get('skills', []) if s.get('name')]
print(','.join(names))
" 2>/dev/null)
        if [ -n "$CATALOGUE_NAMES" ]; then
            SENTINEL_SKILLS="$CATALOGUE_NAMES"
            CATALOGUE_SOURCE="catalogue"
        else
            echo "WARN: catalogue $CATALOGUE produced no skill names; falling back to sentinel" >&2
        fi
    else
        echo "FAIL: TERRAPHIM_SKILLS_CATALOGUE requires python3 for JSON parsing" >&2
        exit 1
    fi
fi

# --- Parse arrays ------------------------------------------------------------
IFS=',' read -ra SKILLS <<< "$SENTINEL_SKILLS"
IFS=',' read -ra CLIS <<< "$TARGET_CLIS"

# Filter empty entries
SKILLS_CLEAN=()
for s in "${SKILLS[@]}"; do s=$(echo "$s" | xargs); [ -n "$s" ] && SKILLS_CLEAN+=("$s"); done
CLIS_CLEAN=()
for c in "${CLIS[@]}"; do c=$(echo "$c" | xargs); [ -n "$c" ] && CLIS_CLEAN+=("$c"); done

# Safety cap: total cells = skills * CLIs
PLANNED_CELLS=$(( ${#SKILLS_CLEAN[@]} * ${#CLIS_CLEAN[@]} ))
if [ "$PLANNED_CELLS" -gt "$MAX_CELLS" ]; then
    echo "FAIL: planned cells ($PLANNED_CELLS = ${#SKILLS_CLEAN[@]} skills x ${#CLIS_CLEAN[@]} CLIs) exceeds MAX_CELLS=$MAX_CELLS" >&2
    echo "       raise TERRAPHIM_SKILLS_MAX_CELLS or reduce catalogue/CLI set" >&2
    exit 1
fi

# Cell storage: key = "skill|cli", value = "status colour evidence"
declare -A CELLS
TOTAL=0
PASSED=0
FAILED=0
SKIPPED=0

# --- Pre-flight --------------------------------------------------------------
if [ "$DRY_RUN" = "0" ]; then
    command -v "$MANAGER_BIN" >/dev/null 2>&1 || { echo "FAIL: $MANAGER_BIN not found" >&2; exit 1; }
    "$MANAGER_BIN" --version >/dev/null 2>&1 || { echo "FAIL: $MANAGER_BIN --version failed" >&2; exit 1; }
fi

# --- Workspace ---------------------------------------------------------------
if [ -n "$WORKSPACE" ]; then
    WORKDIR="$WORKSPACE"
    mkdir -p "$WORKDIR"
else
    WORKDIR=$(mktemp -d /tmp/terraphim-skills-validation.XXXXXX)
fi
trap "rm -rf \"$WORKDIR\"" EXIT

# --- Install/verify helpers (dry-run exits early) ----------------------------
if [ "$DRY_RUN" = "1" ]; then
    for SKILL in "${SKILLS_CLEAN[@]}"; do
        for CLI in "${CLIS_CLEAN[@]}"; do
            KEY="${SKILL}|${CLI}"
            CELLS["$KEY"]="SKIP grey ci-native dry-run"
            SKIPPED=$((SKIPPED + 1))
            TOTAL=$((TOTAL + 1))
        done
    done
    JSON_OVERALL="PASS"
else
    for SKILL in "${SKILLS_CLEAN[@]}"; do
        for CLI in "${CLIS_CLEAN[@]}"; do
            KEY="${SKILL}|${CLI}"
            TOTAL=$((TOTAL + 1))

            echo "=== $SKILL @ $CLI ===" >&2

            # Pre-checks for CLI availability / config
            case "$CLI" in
                claude-code)
                    command -v claude >/dev/null 2>&1 || { CELLS["$KEY"]="SKIP grey claude not available"; SKIPPED=$((SKIPPED + 1)); continue; }
                    ;;
                codex)
                    command -v codex >/dev/null 2>&1 || { CELLS["$KEY"]="SKIP grey codex not available"; SKIPPED=$((SKIPPED + 1)); continue; }
                    ;;
                pi-rust)
                    command -v pi-rust >/dev/null 2>&1 || { CELLS["$KEY"]="SKIP grey pi-rust not available"; SKIPPED=$((SKIPPED + 1)); continue; }
                    ;;
                pi)
                    [ -z "$PI_DIR" ] && { CELLS["$KEY"]="UNSUPPORTED amber PI_DIR not configured"; SKIPPED=$((SKIPPED + 1)); continue; }
                    ;;
                grok)
                    command -v grok >/dev/null 2>&1 || { CELLS["$KEY"]="SKIP grey grok not available"; SKIPPED=$((SKIPPED + 1)); continue; }
                    ;;
            esac

            # Install
            INSTALL_RC=0
            AGENT_FLAG=""
            case "$CLI" in
                claude-code|codex|pi-rust) AGENT_FLAG="--agent $CLI"; TARGET_DIR="" ;;
                grok) AGENT_FLAG=""; TARGET_DIR="$GROK_DIR" ;;
                pi) AGENT_FLAG=""; TARGET_DIR="$PI_DIR" ;;
            esac

            if [ -n "$AGENT_FLAG" ]; then
                INSTALL_OUT=$("$MANAGER_BIN" $AGENT_FLAG install "$SKILL" 2>&1) || INSTALL_RC=$?
            else
                INSTALL_OUT=$("$MANAGER_BIN" --install-dir "$TARGET_DIR" install "$SKILL" 2>&1) || INSTALL_RC=$?
            fi

            if [ "${INSTALL_RC:-0}" != "0" ]; then
                CELLS["$KEY"]="FAIL red install: $(sanitize_evidence "$INSTALL_OUT")"
                FAILED=$((FAILED + 1))
                continue
            fi

            # Verify
            VERIFY_RC=0
            if [ -n "$AGENT_FLAG" ]; then
                VERIFY_OUT=$("$MANAGER_BIN" $AGENT_FLAG verify "$SKILL" 2>&1) || VERIFY_RC=$?
            else
                VERIFY_OUT=$("$MANAGER_BIN" --install-dir "$TARGET_DIR" verify "$SKILL" 2>&1) || VERIFY_RC=$?
            fi

            if [ "${VERIFY_RC:-0}" != "0" ]; then
                CELLS["$KEY"]="FAIL red verify: $(sanitize_evidence "$VERIFY_OUT")"
                FAILED=$((FAILED + 1))
                continue
            fi

            # Extra Grok probe
            if [ "$CLI" = "grok" ]; then
                if command -v grok >/dev/null 2>&1 && grok --cwd "$GROK_DIR" inspect --json 2>/dev/null | grep -q "$SKILL" 2>/dev/null; then
                    CELLS["$KEY"]="PASS green grok inspect lists skill"
                elif [ -f "$GROK_DIR/$SKILL/SKILL.md" ] && [ -f "$GROK_DIR/$SKILL/skill.toml" ]; then
                    CELLS["$KEY"]="PASS green tsm install+verify OK; grok skill files present"
                else
                    CELLS["$KEY"]="FAIL red grok skill files not present"
                    FAILED=$((FAILED + 1))
                    continue
                fi
            else
                CELLS["$KEY"]="PASS green tsm install+verify OK"
            fi
            PASSED=$((PASSED + 1))
        done
    done
fi

if [ "$FAILED" -gt 0 ]; then JSON_OVERALL="FAIL"; else JSON_OVERALL="PASS"; fi

# --- JSON report (stdout) ----------------------------------------------------
skills_json=""
for s in "${SKILLS_CLEAN[@]}"; do
    [ -n "$skills_json" ] && skills_json="$skills_json,"
    skills_json="${skills_json}\"$s\""
done
clis_json=""
for c in "${CLIS_CLEAN[@]}"; do
    [ -n "$clis_json" ] && clis_json="$clis_json,"
    clis_json="${clis_json}\"$c\""
done

echo "{"
echo "  \"catalogue_source\": \"$CATALOGUE_SOURCE\","
echo "  \"catalogue_path\": \"$CATALOGUE\","
echo "  \"skills\": [$skills_json],"
echo "  \"clis\": [$clis_json],"
echo "  \"matrix\": ["
first_skill=1
for SKILL in "${SKILLS_CLEAN[@]}"; do
    [ "$first_skill" = "0" ] && echo ","
    first_skill=0
    echo "    {"
    echo "      \"skill\": \"$SKILL\","
    first_cli=1
    for CLI in "${CLIS_CLEAN[@]}"; do
        [ "$first_cli" = "0" ] && echo ","
        first_cli=0
        KEY="${SKILL}|${CLI}"
        VAL="${CELLS[$KEY]:-FAIL red no result recorded}"
        read -r STATUS COLOUR EVIDENCE <<< "$VAL"
        echo -n "      \"$CLI\": {\"status\":\"$STATUS\",\"colour\":\"$COLOUR\",\"evidence\":\"$EVIDENCE\"}"
    done
    echo ""
    echo -n "    }"
done
echo ""
echo "  ],"
echo "  \"summary\": {"
echo "    \"total\": $TOTAL,"
echo "    \"passed\": $PASSED,"
echo "    \"failed\": $FAILED,"
echo "    \"skipped\": $SKIPPED"
echo "  },"
echo "  \"overall\": \"$JSON_OVERALL\""
echo "}"

# --- Markdown matrix (stderr) ------------------------------------------------
echo "" >&2
echo "## Skills x CLI Status Matrix" >&2
echo "" >&2
HEADER="| Skill |"
for CLI in "${CLIS_CLEAN[@]}"; do HEADER="$HEADER $CLI |"; done
echo "$HEADER" >&2
SEP="|-------|"
for CLI in "${CLIS_CLEAN[@]}"; do SEP="$SEP-----|"; done
echo "$SEP" >&2

COLOUR_HEX() {
    case "$1" in
        green) echo "#15803d" ;;
        red) echo "#b91c1c" ;;
        grey) echo "#6b7280" ;;
        amber) echo "#b45309" ;;
        *) echo "#6b7280" ;;
    esac
}

for SKILL in "${SKILLS_CLEAN[@]}"; do
    ROW="| $SKILL |"
    for CLI in "${CLIS_CLEAN[@]}"; do
        KEY="${SKILL}|${CLI}"
        VAL="${CELLS[$KEY]:-FAIL red no result recorded}"
        read -r STATUS COLOUR EVIDENCE <<< "$VAL"
        HEX=$(COLOUR_HEX "$COLOUR")
        ROW="$ROW <span style=\"color:$HEX\">$STATUS</span> $EVIDENCE |"
    done
    echo "$ROW" >&2
done
echo "" >&2
echo "**Summary:** $PASSED passed, $FAILED failed, $SKIPPED skipped (total $TOTAL)" >&2

if [ "$FAILED" -gt 0 ]; then exit 1; fi
exit 0
