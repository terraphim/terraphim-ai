#!/usr/bin/env bash
# check-runner-health.sh -- host-only health/deployment contract for the
# three-instance `terraphim-gitea-runner` pool.
#
# Refs #3222 (Plan Task 4, docs/plans/2026-08-13-native-ci-host-runner-recovery.md).
#
# This is the source-controlled assertion of what a healthy native CI pool looks
# like. For every service in the pool it requires:
#
#   1. the unit is loaded, ActiveState=active, SubState=running, MainPID != 0
#   2. host mode -- RUNNER_VM_MODE unset is the accepted default (the binary's
#      VmMode::default() is Host); an explicit Firecracker mode is rejected,
#      because Firecracker is out of scope for Terraphim project CI
#   3. RUNNER_STATE_FILE is set, absolute, and unique across the pool
#   4. RUNNER_CHECKOUT_DIR is set, absolute, and unique across the pool
#      (shared checkout roots let concurrent jobs corrupt each other)
#   5. the required `terraphim-native` label is declared
#   6. the journal shows recent runner *activity* -- the startup declaration,
#      a fetched task or a completed task. `active` alone does not prove the
#      runner can take work, but a long-lived runner only emits the startup
#      declaration once, so task activity counts as equal evidence.
#
# Configuration is resolved the way systemd resolves it: every `EnvironmentFile=`
# listed in the unit is read in order, then assignments written directly in the
# unit (`Environment=`) override them. Reading the files is required, not
# optional -- `systemctl show --property=Environment` does NOT expand them, and
# the live pool keeps all of its runner settings in
# ~/.config/terraphim-gitea-runner/env[-2|-3]. Env-file *contents* are never
# echoed: only the four known RUNNER_* settings are ever named in output, so a
# registration token sitting in the same file cannot leak into CI logs.
#
# Every failing member is reported; the script does not stop at the first one.
#
# Usage:
#   ./scripts/check-runner-health.sh [--services "a.service b.service"]
#                                    [--journal-window "10 min ago"]
#                                    [--label terraphim-native]
#                                    [--min 3]
#
# Exit: 0 when the whole pool satisfies the contract, 1 otherwise.
#
# All external commands and paths are injectable so this can be exercised
# hermetically -- no systemd, Gitea or network access is required by the tests:
#
#   SYSTEMCTL_BIN     default `systemctl`
#   SYSTEMCTL_SCOPE   default `--user` (these are systemd *user* services)
#   JOURNALCTL_BIN    default `journalctl`
#   RUNNER_SERVICES   space-separated unit list
#   RUNNER_POOL_MIN   minimum pool size, default 3
#   REQUIRED_RUNNER_LABEL  default `terraphim-native`
#   JOURNAL_WINDOW    `journalctl --since` argument, default `10 min ago`
#   POLL_EVIDENCE_PATTERN  extended regex of accepted activity evidence,
#                     default `declared; polling for tasks|fetched task id=|task complete:`

set -uo pipefail

SYSTEMCTL_BIN="${SYSTEMCTL_BIN:-systemctl}"
SYSTEMCTL_SCOPE="${SYSTEMCTL_SCOPE:---user}"
JOURNALCTL_BIN="${JOURNALCTL_BIN:-journalctl}"
RUNNER_SERVICES="${RUNNER_SERVICES:-terraphim-gitea-runner.service terraphim-gitea-runner-2.service terraphim-gitea-runner-3.service}"
RUNNER_POOL_MIN="${RUNNER_POOL_MIN:-3}"
REQUIRED_RUNNER_LABEL="${REQUIRED_RUNNER_LABEL:-terraphim-native}"
JOURNAL_WINDOW="${JOURNAL_WINDOW:-10 min ago}"
# Extended regex. Each alternative is a distinct proof that the runner is
# reachable by the Gitea scheduler: it declared its labels and started polling,
# it accepted a task, or it finished one. Deliberately narrow -- connection,
# heartbeat and config-reload chatter must NOT satisfy the contract, or the
# check degrades into "the process logged something recently".
POLL_EVIDENCE_PATTERN="${POLL_EVIDENCE_PATTERN:-declared; polling for tasks|fetched task id=|task complete:}"

while [[ $# -gt 0 ]]; do
    case "$1" in
        --services)       RUNNER_SERVICES="$2"; shift 2 ;;
        --journal-window) JOURNAL_WINDOW="$2"; shift 2 ;;
        --label)          REQUIRED_RUNNER_LABEL="$2"; shift 2 ;;
        --min)            RUNNER_POOL_MIN="$2"; shift 2 ;;
        -h|--help)        sed -n '2,53p' "$0"; exit 0 ;;
        *) echo "Unknown argument: $1" >&2; exit 2 ;;
    esac
done

FAILURES=0
bad() { echo "FAIL: $*" >&2; FAILURES=$((FAILURES + 1)); }
ok()  { echo "OK: $*"; }

# Firecracker aliases recognised by VmMode::from_env_str (config.rs). Anything
# else the binary treats as Host, so it is accepted here with a warning rather
# than a failure -- the health check must not disagree with the binary.
is_firecracker_mode() {
    case "$(printf '%s' "$1" | tr '[:upper:]' '[:lower:]')" in
        firecracker|fc|vm) return 0 ;;
        *) return 1 ;;
    esac
}

# Resolved configuration for the unit currently under inspection. Rebuilt from
# scratch per unit so no value can bleed between pool members.
declare -A UNIT_ENV=()

# env_value <key> -- echoes the resolved value, empty when unset.
env_value() {
    printf '%s' "${UNIT_ENV[$1]-}"
}

# env_file_path <EnvironmentFiles-property-value> -- systemd renders the
# property as `/path/to/file (ignore_errors=no)`; recover the bare path.
env_file_path() {
    local raw="$1"
    raw="${raw% (ignore_errors=*)}"
    printf '%s' "$raw"
}

# env_file_optional <EnvironmentFiles-property-value> -- true when the unit used
# `EnvironmentFile=-/path`, i.e. a missing file is not an error for systemd.
env_file_optional() {
    [[ "$1" == *"(ignore_errors=yes)" ]]
}

# load_env_file <path> -- merges KEY=VALUE assignments into UNIT_ENV, later
# assignments winning. Comments, blank lines, a leading `export ` and one layer
# of surrounding quotes are handled the way systemd's parser does.
#
# Values are stored, never echoed: these files hold registration tokens next to
# the runner settings, and only known RUNNER_* keys are ever reported.
load_env_file() {
    local path="$1" line key value
    [[ -r "$path" ]] || return 1
    while IFS= read -r line || [[ -n "$line" ]]; do
        line="${line#"${line%%[![:space:]]*}"}"
        [[ -z "$line" || "$line" == '#'* ]] && continue
        [[ "$line" == "export "* ]] && line="${line#export }"
        [[ "$line" != *=* ]] && continue
        key="${line%%=*}"
        value="${line#*=}"
        key="${key%"${key##*[![:space:]]}"}"
        [[ "$key" =~ ^[A-Za-z_][A-Za-z0-9_]*$ ]] || continue
        value="${value%"${value##*[![:space:]]}"}"
        if [[ ${#value} -ge 2 && "$value" == \"*\" ]]; then
            value="${value:1:${#value}-2}"
        elif [[ ${#value} -ge 2 && "$value" == \'*\' ]]; then
            value="${value:1:${#value}-2}"
        fi
        UNIT_ENV["$key"]="$value"
    done < "$path"
    return 0
}

# load_direct_environment <environment-blob> -- assignments written straight
# into the unit. `systemctl show --property=Environment` renders them
# space-separated on one line; runner values never contain spaces. These are
# applied last, so a unit-level override beats its env files.
load_direct_environment() {
    local tok
    local -a toks
    read -r -a toks <<< "$1"
    for tok in "${toks[@]}"; do
        [[ "$tok" != *=* ]] && continue
        UNIT_ENV["${tok%%=*}"]="${tok#*=}"
    done
}

# csv_contains <csv> <needle> -- exact element match, not substring, so
# `terraphim-native-2` does not satisfy a `terraphim-native` requirement.
csv_contains() {
    local field
    local -a fields
    IFS=',' read -r -a fields <<< "$1"
    for field in "${fields[@]}"; do
        field="${field#"${field%%[![:space:]]*}"}"
        field="${field%"${field##*[![:space:]]}"}"
        [[ "$field" == "$2" ]] && return 0
    done
    return 1
}

read -r -a SERVICES <<< "$RUNNER_SERVICES"

if [[ "${#SERVICES[@]}" -lt "$RUNNER_POOL_MIN" ]]; then
    bad "pool has ${#SERVICES[@]} service(s); the native CI contract requires at least ${RUNNER_POOL_MIN}"
fi

STATE_FILES=()
CHECKOUT_DIRS=()
CHECKED=()

for unit in "${SERVICES[@]}"; do
    if ! show="$("$SYSTEMCTL_BIN" "$SYSTEMCTL_SCOPE" show "$unit" \
        --property=LoadState \
        --property=ActiveState \
        --property=SubState \
        --property=MainPID \
        --property=EnvironmentFiles \
        --property=Environment 2>&1)" && [[ -z "$show" ]]; then
        bad "$unit: could not query systemd"
        continue
    fi

    load_state=""; active_state=""; sub_state=""; main_pid=""; environment=""
    env_files=()
    while IFS= read -r line; do
        case "$line" in
            LoadState=*)   load_state="${line#LoadState=}" ;;
            ActiveState=*) active_state="${line#ActiveState=}" ;;
            SubState=*)    sub_state="${line#SubState=}" ;;
            MainPID=*)     main_pid="${line#MainPID=}" ;;
            # One line per configured file, in unit order.
            EnvironmentFiles=*) env_files+=("${line#EnvironmentFiles=}") ;;
            Environment=*) environment="${environment} ${line#Environment=}" ;;
        esac
    done <<< "$show"

    if [[ "$load_state" != "loaded" ]]; then
        bad "$unit: LoadState=${load_state:-unknown} (expected loaded)"
        continue
    fi
    if [[ "$active_state" != "active" || "$sub_state" != "running" ]]; then
        bad "$unit: ActiveState=${active_state:-unknown} SubState=${sub_state:-unknown} (expected active/running)"
        continue
    fi
    if [[ ! "$main_pid" =~ ^[0-9]+$ || "$main_pid" -eq 0 ]]; then
        bad "$unit: MainPID=${main_pid:-unset} -- active but no live process"
        continue
    fi

    unit_failed=0

    # --- resolve configuration: env files first, unit Environment= last ---
    unset UNIT_ENV
    declare -A UNIT_ENV=()
    for env_file_entry in "${env_files[@]:-}"; do
        [[ -z "$env_file_entry" ]] && continue
        env_file="$(env_file_path "$env_file_entry")"
        if ! load_env_file "$env_file"; then
            if env_file_optional "$env_file_entry"; then
                echo "WARN: $unit: optional EnvironmentFile ${env_file} is unreadable" >&2
            else
                bad "$unit: EnvironmentFile ${env_file} is unreadable but required (ignore_errors=no)"
                unit_failed=1
            fi
        fi
    done
    load_direct_environment "$environment"

    # --- host mode ---
    vm_mode="$(env_value RUNNER_VM_MODE)"
    if [[ -z "$vm_mode" ]]; then
        : # unset is the accepted default (VmMode::default() == Host)
    elif is_firecracker_mode "$vm_mode"; then
        bad "$unit: RUNNER_VM_MODE=${vm_mode} -- Firecracker is out of scope for Terraphim CI; use host mode"
        unit_failed=1
    elif [[ "$(printf '%s' "$vm_mode" | tr '[:upper:]' '[:lower:]')" != "host" ]]; then
        echo "WARN: $unit: RUNNER_VM_MODE=${vm_mode} is unrecognised; the runner will treat it as host" >&2
    fi

    # --- isolated state file ---
    state_file="$(env_value RUNNER_STATE_FILE)"
    if [[ -z "$state_file" ]]; then
        bad "$unit: RUNNER_STATE_FILE unset -- the default '.runner' is relative and cannot be proven unique"
        unit_failed=1
    elif [[ "$state_file" != /* ]]; then
        bad "$unit: RUNNER_STATE_FILE=${state_file} is not absolute -- it resolves against WorkingDirectory"
        unit_failed=1
    else
        for i in "${!STATE_FILES[@]}"; do
            if [[ "${STATE_FILES[$i]}" == "$state_file" ]]; then
                bad "$unit: RUNNER_STATE_FILE=${state_file} is shared with ${CHECKED[$i]} -- runner identities would collide"
                unit_failed=1
            fi
        done
    fi

    # --- isolated checkout root ---
    checkout_dir="$(env_value RUNNER_CHECKOUT_DIR)"
    if [[ -z "$checkout_dir" ]]; then
        bad "$unit: RUNNER_CHECKOUT_DIR unset -- the default '.' is relative and cannot be proven unique"
        unit_failed=1
    elif [[ "$checkout_dir" != /* ]]; then
        bad "$unit: RUNNER_CHECKOUT_DIR=${checkout_dir} is not absolute -- it resolves against WorkingDirectory"
        unit_failed=1
    else
        for i in "${!CHECKOUT_DIRS[@]}"; do
            if [[ "${CHECKOUT_DIRS[$i]}" == "$checkout_dir" ]]; then
                bad "$unit: RUNNER_CHECKOUT_DIR=${checkout_dir} is shared with ${CHECKED[$i]} -- concurrent jobs would corrupt each other"
                unit_failed=1
            fi
        done
    fi

    # --- required label ---
    labels="$(env_value RUNNER_LABELS)"
    if [[ -n "$labels" ]] && ! csv_contains "$labels" "$REQUIRED_RUNNER_LABEL"; then
        bad "$unit: RUNNER_LABELS=${labels} does not declare the required '${REQUIRED_RUNNER_LABEL}' label"
        unit_failed=1
    fi

    # --- recent scheduler-facing activity ---
    # Bounded by --since so the evidence is *recent*, and matched against the
    # narrow alternation so a long-lived runner passes on task activity while a
    # merely chatty process does not.
    journal="$("$JOURNALCTL_BIN" "$SYSTEMCTL_SCOPE" -u "$unit" \
        --since "$JOURNAL_WINDOW" --no-pager -n 500 2>/dev/null)"
    if ! printf '%s' "$journal" | grep -qE "$POLL_EVIDENCE_PATTERN"; then
        bad "$unit: no runner activity matching '${POLL_EVIDENCE_PATTERN}' since '${JOURNAL_WINDOW}' -- active does not prove it is reachable by the scheduler"
        unit_failed=1
    fi

    STATE_FILES+=("$state_file")
    CHECKOUT_DIRS+=("$checkout_dir")
    CHECKED+=("$unit")

    [[ "$unit_failed" -eq 0 ]] && ok "$unit: host mode, pid ${main_pid}, isolated state/checkout, recent task activity"
done

if [[ "$FAILURES" -eq 0 ]]; then
    echo "OK: ${#SERVICES[@]} runner(s) satisfy the host-only native CI pool contract"
    exit 0
fi

echo "ERROR: ${FAILURES} host-runner pool contract violation(s)" >&2
exit 1
