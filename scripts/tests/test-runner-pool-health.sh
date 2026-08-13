#!/usr/bin/env bash
# test-runner-pool-health.sh -- contract tests for scripts/check-runner-health.sh
#
# Locks the source-controlled deployment contract for the three-instance
# `terraphim-gitea-runner` pool (Plan Task 4, Refs #3222):
#
#   * each service is loaded, active, running, with a live MainPID
#   * host mode -- unset RUNNER_VM_MODE is the accepted default; an explicit
#     Firecracker mode (firecracker/fc/vm, per VmMode::from_env_str) is rejected
#   * RUNNER_STATE_FILE is absolute and unique across the pool
#   * RUNNER_CHECKOUT_DIR is absolute and unique across the pool
#   * the required `terraphim-native` label is declared
#   * the journal shows recent runner *activity* -- startup polling, a fetched
#     task or a completed task -- but not arbitrary unrelated log noise
#
# The live bigbox pool configures every one of those values through
# `EnvironmentFiles=` (~/.config/terraphim-gitea-runner/env[-2|-3]), which
# `systemctl show --property=Environment` does NOT expand. So the contract is
# also asserted over env-file-sourced configuration, with direct `Environment=`
# taking precedence, and with the guarantee that no env-file content or secret
# key ever reaches the output.
#
# Hermetic: systemd, Gitea and the network are never touched. `systemctl` and
# `journalctl` are injected through SYSTEMCTL_BIN/JOURNALCTL_BIN and read
# fixtures from $FAKE_ROOT, so this runs anywhere the repo checks out.

set -uo pipefail

THIS_DIR="$(cd "$(dirname "$0")" && pwd)"
HEALTH_SH="${THIS_DIR}/../check-runner-health.sh"

[ -f "$HEALTH_SH" ] || { echo "FAIL: missing $HEALTH_SH" >&2; exit 1; }

TMP="$(mktemp -d)"
trap '/bin/rm -rf "$TMP"' EXIT

BIN="${TMP}/bin"
mkdir -p "$BIN"

# --- injected systemctl -------------------------------------------------------
# Serves `systemctl --user show <unit> --property=...` from $FAKE_ROOT/<unit>.show.
# A unit with no fixture behaves like a real missing unit: LoadState=not-found.
cat > "${BIN}/systemctl" <<'EOF'
#!/usr/bin/env bash
unit=""
for a in "$@"; do case "$a" in *.service) unit="$a" ;; esac; done
printf '%s\n' "$*" >> "${FAKE_ROOT}/systemctl.argv"
if [ -f "${FAKE_ROOT}/${unit}.show" ]; then
    cat "${FAKE_ROOT}/${unit}.show"
else
    printf 'LoadState=not-found\nActiveState=inactive\nSubState=dead\nMainPID=0\nEnvironment=\n'
fi
EOF

# --- injected journalctl ------------------------------------------------------
# Serves $FAKE_ROOT/<unit>.journal and records argv so the test can assert the
# script actually bounds the query with --since (i.e. asks for *recent* evidence).
cat > "${BIN}/journalctl" <<'EOF'
#!/usr/bin/env bash
unit=""
prev=""
for a in "$@"; do
    case "$prev" in -u|--unit) unit="$a" ;; esac
    prev="$a"
done
printf '%s\n' "$*" >> "${FAKE_ROOT}/journalctl.argv"
[ -f "${FAKE_ROOT}/${unit}.journal" ] && cat "${FAKE_ROOT}/${unit}.journal"
exit 0
EOF

chmod +x "${BIN}/systemctl" "${BIN}/journalctl"

SERVICES="terraphim-gitea-runner.service terraphim-gitea-runner-2.service terraphim-gitea-runner-3.service"

FAILURES=0
CASE=""
fail() { echo "FAIL [${CASE}]: $*" >&2; FAILURES=$((FAILURES + 1)); }

# write_unit <root> <unit> <active> <sub> <mainpid> <environment-line>
write_unit() {
    local root="$1" unit="$2" active="$3" sub="$4" pid="$5" env="$6"
    cat > "${root}/${unit}.show" <<EOF
LoadState=loaded
ActiveState=${active}
SubState=${sub}
MainPID=${pid}
Environment=${env}
EOF
}

# write_unit_ef <root> <unit> <mainpid> <direct-environment> [envfile-property-value...]
# Renders a unit exactly the way `systemctl show` does for a service configured
# through EnvironmentFile=: one `EnvironmentFiles=<path> (ignore_errors=<bool>)`
# line per file, and an `Environment=` line that holds ONLY the assignments
# written directly in the unit (systemd does not expand env files into it).
write_unit_ef() {
    local root="$1" unit="$2" pid="$3" env="$4" f
    shift 4
    {
        printf 'LoadState=loaded\nActiveState=active\nSubState=running\nMainPID=%s\n' "$pid"
        for f in "$@"; do printf 'EnvironmentFiles=%s\n' "$f"; done
        printf 'Environment=%s\n' "$env"
    } > "${root}/${unit}.show"
}

# write_env_file <path> <line...> -- a systemd EnvironmentFile.
write_env_file() {
    local path="$1"
    shift
    printf '%s\n' "$@" > "$path"
}

SECRET_KEY="GITEA_RUNNER_REGISTRATION_TOKEN"
SECRET_VALUE="s3cr3t-token-must-never-be-printed"

# write_journal <root> <unit> <content>
write_journal() {
    printf '%s\n' "$3" > "${1}/${2}.journal"
}

POLL_LINE='Aug 13 09:14:02 bigbox terraphim-gitea-runner[4211]: [INFO] declared; polling for tasks (labels=["terraphim-native"])'

# healthy_pool <root> -- three host-mode runners with isolated state/checkout.
healthy_pool() {
    local root="$1" i=1 unit
    mkdir -p "$root"
    for unit in $SERVICES; do
        write_unit "$root" "$unit" active running "$((4200 + i))" \
            "RUNNER_VM_MODE=host RUNNER_STATE_FILE=/home/alex/gitea-runner/${i}/.runner RUNNER_CHECKOUT_DIR=/home/alex/gitea-runner/${i}/checkout RUNNER_LABELS=terraphim-native"
        write_journal "$root" "$unit" "$POLL_LINE"
        i=$((i + 1))
    done
}

# env_file_pool <root> -- the live bigbox shape: nothing in `Environment=`, all
# three runners configured through their own EnvironmentFile, each carrying a
# secret alongside the runner settings.
env_file_pool() {
    local root="$1" i=1 unit
    mkdir -p "$root"
    for unit in $SERVICES; do
        write_env_file "${root}/env-${i}" \
            '# managed by deploy -- do not edit by hand' \
            '' \
            'RUNNER_VM_MODE=host' \
            "RUNNER_STATE_FILE=/home/alex/gitea-runner/${i}/.runner" \
            "RUNNER_CHECKOUT_DIR=/home/alex/gitea-runner/${i}/checkout" \
            'RUNNER_LABELS=terraphim-native' \
            "${SECRET_KEY}=${SECRET_VALUE}"
        write_unit_ef "$root" "$unit" "$((4200 + i))" "" \
            "${root}/env-${i} (ignore_errors=no)"
        write_journal "$root" "$unit" "$POLL_LINE"
        i=$((i + 1))
    done
}

# run_health <root> -> captures OUT, returns exit code
OUT=""
run_health() {
    OUT="$(FAKE_ROOT="$1" SYSTEMCTL_BIN="${BIN}/systemctl" JOURNALCTL_BIN="${BIN}/journalctl" \
        RUNNER_SERVICES="$SERVICES" bash "$HEALTH_SH" 2>&1)"
}

new_root() {
    local root
    root="${TMP}/case-$(printf '%s' "$CASE" | tr -c 'a-zA-Z0-9' '-')"
    mkdir -p "$root"
    healthy_pool "$root"
    printf '%s' "$root"
}

new_env_root() {
    local root
    root="${TMP}/case-$(printf '%s' "$CASE" | tr -c 'a-zA-Z0-9' '-')"
    mkdir -p "$root"
    env_file_pool "$root"
    printf '%s' "$root"
}

# assert_no_secret_leak -- the output must never carry env-file contents.
assert_no_secret_leak() {
    case "$OUT" in
        *"$SECRET_VALUE"*) fail "env-file secret VALUE leaked into output" ;;
    esac
    case "$OUT" in
        *"$SECRET_KEY"*) fail "env-file secret KEY leaked into output" ;;
    esac
}

# --- case 1: healthy three-instance host pool passes ---------------------------
CASE="healthy-pool-passes"
ROOT="$(new_root)"
run_health "$ROOT" && rc=0 || rc=$?
[ "$rc" -eq 0 ] || fail "expected exit 0, got $rc; output: $OUT"

# --- case 2: --since bounds the journal query (recency is real) ----------------
CASE="journal-query-is-time-bounded"
ROOT="$(new_root)"
run_health "$ROOT" >/dev/null 2>&1
if ! grep -q -- '--since' "${ROOT}/journalctl.argv" 2>/dev/null; then
    fail "journalctl was not invoked with --since; poll evidence would not be recent"
fi
if [ "$(wc -l < "${ROOT}/journalctl.argv" 2>/dev/null || echo 0)" -lt 3 ]; then
    fail "expected a journal query per service"
fi

# --- case 3: unset RUNNER_VM_MODE is the accepted host default -----------------
CASE="unset-vm-mode-is-host-default"
ROOT="$(new_root)"
write_unit "$ROOT" terraphim-gitea-runner-3.service active running 4203 \
    "RUNNER_STATE_FILE=/home/alex/gitea-runner/3/.runner RUNNER_CHECKOUT_DIR=/home/alex/gitea-runner/3/checkout RUNNER_LABELS=terraphim-native"
run_health "$ROOT" && rc=0 || rc=$?
[ "$rc" -eq 0 ] || fail "unset RUNNER_VM_MODE must be accepted as host; got $rc; output: $OUT"

# --- case 4: explicit firecracker mode is rejected -----------------------------
for mode in firecracker fc vm FIRECRACKER; do
    CASE="explicit-${mode}-rejected"
    ROOT="$(new_root)"
    write_unit "$ROOT" terraphim-gitea-runner-3.service active running 4203 \
        "RUNNER_VM_MODE=${mode} RUNNER_STATE_FILE=/home/alex/gitea-runner/3/.runner RUNNER_CHECKOUT_DIR=/home/alex/gitea-runner/3/checkout RUNNER_LABELS=terraphim-native"
    run_health "$ROOT" && rc=0 || rc=$?
    [ "$rc" -ne 0 ] || fail "RUNNER_VM_MODE=${mode} must fail the host-only contract"
    case "$OUT" in *RUNNER_VM_MODE*) ;; *) fail "output should name RUNNER_VM_MODE; got: $OUT" ;; esac
done

# --- case 5: an inactive service fails ----------------------------------------
CASE="inactive-service-fails"
ROOT="$(new_root)"
write_unit "$ROOT" terraphim-gitea-runner-2.service failed failed 0 \
    "RUNNER_VM_MODE=host RUNNER_STATE_FILE=/home/alex/gitea-runner/2/.runner RUNNER_CHECKOUT_DIR=/home/alex/gitea-runner/2/checkout RUNNER_LABELS=terraphim-native"
run_health "$ROOT" && rc=0 || rc=$?
[ "$rc" -ne 0 ] || fail "inactive service must fail"
case "$OUT" in *terraphim-gitea-runner-2.service*) ;; *) fail "output should name the failing unit; got: $OUT" ;; esac

# --- case 6: active but no live MainPID fails ---------------------------------
CASE="dead-mainpid-fails"
ROOT="$(new_root)"
write_unit "$ROOT" terraphim-gitea-runner-2.service active running 0 \
    "RUNNER_VM_MODE=host RUNNER_STATE_FILE=/home/alex/gitea-runner/2/.runner RUNNER_CHECKOUT_DIR=/home/alex/gitea-runner/2/checkout RUNNER_LABELS=terraphim-native"
run_health "$ROOT" && rc=0 || rc=$?
[ "$rc" -ne 0 ] || fail "active unit with MainPID=0 must fail"
case "$OUT" in *MainPID*) ;; *) fail "output should name MainPID; got: $OUT" ;; esac

# --- case 7: a not-found unit fails -------------------------------------------
CASE="missing-unit-fails"
ROOT="$(new_root)"
rm -f "${ROOT}/terraphim-gitea-runner-3.service.show"
run_health "$ROOT" && rc=0 || rc=$?
[ "$rc" -ne 0 ] || fail "a unit that is not loaded must fail"

# --- case 8: shared RUNNER_STATE_FILE fails -----------------------------------
CASE="duplicate-state-file-fails"
ROOT="$(new_root)"
write_unit "$ROOT" terraphim-gitea-runner-2.service active running 4202 \
    "RUNNER_VM_MODE=host RUNNER_STATE_FILE=/home/alex/gitea-runner/1/.runner RUNNER_CHECKOUT_DIR=/home/alex/gitea-runner/2/checkout RUNNER_LABELS=terraphim-native"
run_health "$ROOT" && rc=0 || rc=$?
[ "$rc" -ne 0 ] || fail "shared RUNNER_STATE_FILE must fail"
case "$OUT" in *RUNNER_STATE_FILE*) ;; *) fail "output should name RUNNER_STATE_FILE; got: $OUT" ;; esac

# --- case 9: shared RUNNER_CHECKOUT_DIR fails ---------------------------------
CASE="duplicate-checkout-dir-fails"
ROOT="$(new_root)"
write_unit "$ROOT" terraphim-gitea-runner-2.service active running 4202 \
    "RUNNER_VM_MODE=host RUNNER_STATE_FILE=/home/alex/gitea-runner/2/.runner RUNNER_CHECKOUT_DIR=/home/alex/gitea-runner/1/checkout RUNNER_LABELS=terraphim-native"
run_health "$ROOT" && rc=0 || rc=$?
[ "$rc" -ne 0 ] || fail "shared RUNNER_CHECKOUT_DIR must fail"
case "$OUT" in *RUNNER_CHECKOUT_DIR*) ;; *) fail "output should name RUNNER_CHECKOUT_DIR; got: $OUT" ;; esac

# --- case 10: relative state/checkout paths fail ------------------------------
# `.runner` and `.` are the binary's defaults; they resolve against each unit's
# WorkingDirectory, so they cannot be proven unique. Require absolute paths.
CASE="relative-paths-fail"
ROOT="$(new_root)"
write_unit "$ROOT" terraphim-gitea-runner-2.service active running 4202 \
    "RUNNER_VM_MODE=host RUNNER_STATE_FILE=.runner RUNNER_CHECKOUT_DIR=. RUNNER_LABELS=terraphim-native"
run_health "$ROOT" && rc=0 || rc=$?
[ "$rc" -ne 0 ] || fail "relative RUNNER_STATE_FILE/RUNNER_CHECKOUT_DIR must fail"

# --- case 11: unset state/checkout env fails (silent default collision) -------
CASE="unset-state-and-checkout-fails"
ROOT="$(new_root)"
write_unit "$ROOT" terraphim-gitea-runner-2.service active running 4202 \
    "RUNNER_VM_MODE=host RUNNER_LABELS=terraphim-native"
run_health "$ROOT" && rc=0 || rc=$?
[ "$rc" -ne 0 ] || fail "unset RUNNER_STATE_FILE/RUNNER_CHECKOUT_DIR must fail"

# --- case 12: missing terraphim-native label fails ----------------------------
CASE="missing-required-label-fails"
ROOT="$(new_root)"
write_unit "$ROOT" terraphim-gitea-runner-3.service active running 4203 \
    "RUNNER_VM_MODE=host RUNNER_STATE_FILE=/home/alex/gitea-runner/3/.runner RUNNER_CHECKOUT_DIR=/home/alex/gitea-runner/3/checkout RUNNER_LABELS=terraphim-firecracker"
run_health "$ROOT" && rc=0 || rc=$?
[ "$rc" -ne 0 ] || fail "a pool member without the terraphim-native label must fail"
case "$OUT" in *terraphim-native*) ;; *) fail "output should name the required label; got: $OUT" ;; esac

# --- case 13: unset RUNNER_LABELS is the accepted default ---------------------
CASE="unset-labels-is-default-terraphim-native"
ROOT="$(new_root)"
write_unit "$ROOT" terraphim-gitea-runner-3.service active running 4203 \
    "RUNNER_VM_MODE=host RUNNER_STATE_FILE=/home/alex/gitea-runner/3/.runner RUNNER_CHECKOUT_DIR=/home/alex/gitea-runner/3/checkout"
run_health "$ROOT" && rc=0 || rc=$?
[ "$rc" -eq 0 ] || fail "unset RUNNER_LABELS defaults to terraphim-native; got $rc; output: $OUT"

# --- case 14: a label list containing terraphim-native passes -----------------
CASE="label-list-containing-required-passes"
ROOT="$(new_root)"
write_unit "$ROOT" terraphim-gitea-runner-3.service active running 4203 \
    "RUNNER_VM_MODE=host RUNNER_STATE_FILE=/home/alex/gitea-runner/3/.runner RUNNER_CHECKOUT_DIR=/home/alex/gitea-runner/3/checkout RUNNER_LABELS=linux,terraphim-native,x64"
run_health "$ROOT" && rc=0 || rc=$?
[ "$rc" -eq 0 ] || fail "a CSV label list containing terraphim-native must pass; output: $OUT"

# --- case 15: a label that merely *contains* the required token fails ---------
CASE="label-substring-does-not-satisfy"
ROOT="$(new_root)"
write_unit "$ROOT" terraphim-gitea-runner-3.service active running 4203 \
    "RUNNER_VM_MODE=host RUNNER_STATE_FILE=/home/alex/gitea-runner/3/.runner RUNNER_CHECKOUT_DIR=/home/alex/gitea-runner/3/checkout RUNNER_LABELS=terraphim-native-2"
run_health "$ROOT" && rc=0 || rc=$?
[ "$rc" -ne 0 ] || fail "label 'terraphim-native-2' must not satisfy the terraphim-native requirement"

# --- case 16: no recent declared/polling evidence fails -----------------------
CASE="stale-journal-fails"
ROOT="$(new_root)"
write_journal "$ROOT" terraphim-gitea-runner-2.service \
    'Aug 13 09:14:02 bigbox terraphim-gitea-runner[4202]: [INFO] loaded existing runner state'
run_health "$ROOT" && rc=0 || rc=$?
[ "$rc" -ne 0 ] || fail "a runner with no recent declared/polling evidence must fail"
case "$OUT" in *polling*) ;; *) fail "output should mention polling evidence; got: $OUT" ;; esac

# --- case 17: empty journal window fails --------------------------------------
CASE="empty-journal-fails"
ROOT="$(new_root)"
: > "${ROOT}/terraphim-gitea-runner.service.journal"
run_health "$ROOT" && rc=0 || rc=$?
[ "$rc" -ne 0 ] || fail "an empty recent journal must fail"

# --- case 18: every failing member is reported, not just the first ------------
CASE="reports-all-failures"
ROOT="$(new_root)"
write_unit "$ROOT" terraphim-gitea-runner-2.service failed failed 0 "RUNNER_VM_MODE=host"
write_unit "$ROOT" terraphim-gitea-runner-3.service active running 4203 \
    "RUNNER_VM_MODE=firecracker RUNNER_STATE_FILE=/home/alex/gitea-runner/3/.runner RUNNER_CHECKOUT_DIR=/home/alex/gitea-runner/3/checkout"
run_health "$ROOT" && rc=0 || rc=$?
[ "$rc" -ne 0 ] || fail "expected failure"
case "$OUT" in *terraphim-gitea-runner-2.service*) ;; *) fail "runner-2 not reported; got: $OUT" ;; esac
case "$OUT" in *terraphim-gitea-runner-3.service*) ;; *) fail "runner-3 not reported; got: $OUT" ;; esac

# --- case 19: the pool size is part of the contract ---------------------------
CASE="pool-size-is-three"
ROOT="$(new_root)"
run_health_two() {
    OUT="$(FAKE_ROOT="$1" SYSTEMCTL_BIN="${BIN}/systemctl" JOURNALCTL_BIN="${BIN}/journalctl" \
        RUNNER_SERVICES="terraphim-gitea-runner.service terraphim-gitea-runner-2.service" \
        bash "$HEALTH_SH" 2>&1)"
}
run_health_two "$ROOT" && rc=0 || rc=$?
[ "$rc" -ne 0 ] || fail "a two-instance pool must fail the three-instance contract"

# =============================================================================
# EnvironmentFiles -- the live bigbox configuration path (canary regression)
#
# On bigbox every runner setting lives in ~/.config/terraphim-gitea-runner/
# env[-2|-3] via `EnvironmentFile=`. `systemctl show --property=Environment`
# renders those as *empty*, so a checker that reads only `Environment=` declares
# every RUNNER_STATE_FILE/RUNNER_CHECKOUT_DIR unset and rejects a healthy pool.
# =============================================================================

# --- case 20: the EnvironmentFiles property is actually queried ---------------
CASE="environment-files-property-is-queried"
ROOT="$(new_env_root)"
run_health "$ROOT" >/dev/null 2>&1
if ! grep -q -- '--property=EnvironmentFiles' "${ROOT}/systemctl.argv" 2>/dev/null; then
    fail "systemctl was not asked for EnvironmentFiles; env-file configuration is invisible"
fi

# --- case 21: an env-file-configured healthy pool passes, without leaking -----
CASE="env-file-configured-pool-passes"
ROOT="$(new_env_root)"
run_health "$ROOT" && rc=0 || rc=$?
[ "$rc" -eq 0 ] || fail "a pool configured through EnvironmentFiles must pass; got $rc; output: $OUT"
assert_no_secret_leak

# --- case 22: env-file values are read for uniqueness, not just presence ------
CASE="env-file-duplicate-state-file-fails"
ROOT="$(new_env_root)"
write_env_file "${ROOT}/env-2" \
    'RUNNER_VM_MODE=host' \
    'RUNNER_STATE_FILE=/home/alex/gitea-runner/1/.runner' \
    'RUNNER_CHECKOUT_DIR=/home/alex/gitea-runner/2/checkout' \
    'RUNNER_LABELS=terraphim-native' \
    "${SECRET_KEY}=${SECRET_VALUE}"
run_health "$ROOT" && rc=0 || rc=$?
[ "$rc" -ne 0 ] || fail "a shared RUNNER_STATE_FILE from an env file must fail"
case "$OUT" in *RUNNER_STATE_FILE*) ;; *) fail "output should name RUNNER_STATE_FILE; got: $OUT" ;; esac
assert_no_secret_leak

CASE="env-file-duplicate-checkout-dir-fails"
ROOT="$(new_env_root)"
write_env_file "${ROOT}/env-3" \
    'RUNNER_VM_MODE=host' \
    'RUNNER_STATE_FILE=/home/alex/gitea-runner/3/.runner' \
    'RUNNER_CHECKOUT_DIR=/home/alex/gitea-runner/1/checkout' \
    'RUNNER_LABELS=terraphim-native'
run_health "$ROOT" && rc=0 || rc=$?
[ "$rc" -ne 0 ] || fail "a shared RUNNER_CHECKOUT_DIR from an env file must fail"
case "$OUT" in *RUNNER_CHECKOUT_DIR*) ;; *) fail "output should name RUNNER_CHECKOUT_DIR; got: $OUT" ;; esac

# --- case 23: host mode and label are enforced from env files too ------------
CASE="env-file-firecracker-mode-fails"
ROOT="$(new_env_root)"
write_env_file "${ROOT}/env-3" \
    'RUNNER_VM_MODE=firecracker' \
    'RUNNER_STATE_FILE=/home/alex/gitea-runner/3/.runner' \
    'RUNNER_CHECKOUT_DIR=/home/alex/gitea-runner/3/checkout' \
    'RUNNER_LABELS=terraphim-native'
run_health "$ROOT" && rc=0 || rc=$?
[ "$rc" -ne 0 ] || fail "RUNNER_VM_MODE=firecracker from an env file must fail"
case "$OUT" in *RUNNER_VM_MODE*) ;; *) fail "output should name RUNNER_VM_MODE; got: $OUT" ;; esac

CASE="env-file-missing-label-fails"
ROOT="$(new_env_root)"
write_env_file "${ROOT}/env-2" \
    'RUNNER_VM_MODE=host' \
    'RUNNER_STATE_FILE=/home/alex/gitea-runner/2/.runner' \
    'RUNNER_CHECKOUT_DIR=/home/alex/gitea-runner/2/checkout' \
    'RUNNER_LABELS=terraphim-firecracker'
run_health "$ROOT" && rc=0 || rc=$?
[ "$rc" -ne 0 ] || fail "a env-file label list without terraphim-native must fail"

# --- case 24: direct Environment= overrides the env file ---------------------
CASE="direct-environment-overrides-env-file"
ROOT="$(new_env_root)"
write_env_file "${ROOT}/env-3" \
    'RUNNER_VM_MODE=firecracker' \
    'RUNNER_STATE_FILE=/home/alex/gitea-runner/1/.runner' \
    'RUNNER_CHECKOUT_DIR=/home/alex/gitea-runner/1/checkout' \
    'RUNNER_LABELS=terraphim-native'
write_unit_ef "$ROOT" terraphim-gitea-runner-3.service 4203 \
    "RUNNER_VM_MODE=host RUNNER_STATE_FILE=/home/alex/gitea-runner/3/.runner RUNNER_CHECKOUT_DIR=/home/alex/gitea-runner/3/checkout" \
    "${ROOT}/env-3 (ignore_errors=no)"
run_health "$ROOT" && rc=0 || rc=$?
[ "$rc" -eq 0 ] || fail "direct Environment= must override EnvironmentFile values; got $rc; output: $OUT"

# --- case 25: the last env file wins, and multiple files merge ---------------
CASE="multiple-env-files-merge-last-wins"
ROOT="$(new_env_root)"
write_env_file "${ROOT}/env-3" \
    'RUNNER_VM_MODE=host' \
    'RUNNER_STATE_FILE=/home/alex/gitea-runner/1/.runner' \
    'RUNNER_LABELS=terraphim-native'
write_env_file "${ROOT}/env-3-override" \
    'RUNNER_STATE_FILE=/home/alex/gitea-runner/3/.runner' \
    'RUNNER_CHECKOUT_DIR=/home/alex/gitea-runner/3/checkout'
write_unit_ef "$ROOT" terraphim-gitea-runner-3.service 4203 "" \
    "${ROOT}/env-3 (ignore_errors=no)" \
    "${ROOT}/env-3-override (ignore_errors=no)"
run_health "$ROOT" && rc=0 || rc=$?
[ "$rc" -eq 0 ] || fail "later EnvironmentFiles must override earlier ones; got $rc; output: $OUT"

# --- case 26: quoted / exported / commented env-file forms parse -------------
CASE="env-file-quoting-forms-parse"
ROOT="$(new_env_root)"
write_env_file "${ROOT}/env-2" \
    '# runner 2' \
    '   ' \
    'export RUNNER_VM_MODE="host"' \
    "export RUNNER_STATE_FILE='/home/alex/gitea-runner/2/.runner'" \
    'RUNNER_CHECKOUT_DIR="/home/alex/gitea-runner/2/checkout"' \
    'RUNNER_LABELS="terraphim-native"'
run_health "$ROOT" && rc=0 || rc=$?
[ "$rc" -eq 0 ] || fail "quoted/exported env-file assignments must parse; got $rc; output: $OUT"

# --- case 27: an unreadable required env file fails loudly, not silently -----
CASE="unreadable-required-env-file-fails"
ROOT="$(new_env_root)"
rm -f "${ROOT}/env-2"
run_health "$ROOT" && rc=0 || rc=$?
[ "$rc" -ne 0 ] || fail "a missing ignore_errors=no EnvironmentFile must fail"
case "$OUT" in *terraphim-gitea-runner-2.service*) ;; *) fail "output should name the affected unit; got: $OUT" ;; esac

# --- case 28: an optional missing env file still fails on the unset values ---
CASE="optional-missing-env-file-fails-on-unset-values"
ROOT="$(new_env_root)"
rm -f "${ROOT}/env-2"
write_unit_ef "$ROOT" terraphim-gitea-runner-2.service 4202 "" \
    "${ROOT}/env-2 (ignore_errors=yes)"
run_health "$ROOT" && rc=0 || rc=$?
[ "$rc" -ne 0 ] || fail "an optional missing env file leaves the values unset, which must fail"
case "$OUT" in *RUNNER_STATE_FILE*) ;; *) fail "output should name the unset setting; got: $OUT" ;; esac

# --- case 29: the three env files must be unique per pool member -------------
# Pointing two units at the same env file gives them the same state file and
# checkout root, which is exactly the identity collision the contract forbids.
CASE="shared-env-file-across-units-fails"
ROOT="$(new_env_root)"
write_unit_ef "$ROOT" terraphim-gitea-runner-2.service 4202 "" \
    "${ROOT}/env-1 (ignore_errors=no)"
run_health "$ROOT" && rc=0 || rc=$?
[ "$rc" -ne 0 ] || fail "two units sharing one EnvironmentFile must fail"
assert_no_secret_leak

# =============================================================================
# Journal evidence -- long-lived runners are healthy without a recent startup
#
# "declared; polling for tasks" is emitted once at startup. A runner that has
# been up for days never re-emits it, but does emit task activity. Requiring the
# startup phrase inside a 10-minute window rejected the healthy live pool.
# =============================================================================

# --- case 30: recurring fetched-task activity is accepted --------------------
CASE="fetched-task-activity-passes"
ROOT="$(new_root)"
for unit in $SERVICES; do
    write_journal "$ROOT" "$unit" \
'Aug 13 09:10:11 bigbox terraphim-gitea-runner[4201]: [INFO] fetched task id=90211 (runner_health)
Aug 13 09:12:41 bigbox terraphim-gitea-runner[4201]: [INFO] fetched task id=90214 (runner_health)'
done
run_health "$ROOT" && rc=0 || rc=$?
[ "$rc" -eq 0 ] || fail "recurring 'fetched task id=' activity must prove liveness; got $rc; output: $OUT"

# --- case 31: recurring completed-task activity is accepted ------------------
CASE="task-complete-activity-passes"
ROOT="$(new_root)"
for unit in $SERVICES; do
    write_journal "$ROOT" "$unit" \
'Aug 13 09:11:02 bigbox terraphim-gitea-runner[4201]: [INFO] task complete: id=90211 status=success
Aug 13 09:13:52 bigbox terraphim-gitea-runner[4201]: [INFO] task complete: id=90214 status=success'
done
run_health "$ROOT" && rc=0 || rc=$?
[ "$rc" -eq 0 ] || fail "recurring 'task complete:' activity must prove liveness; got $rc; output: $OUT"

# --- case 32: the startup phrase still counts (freshly restarted runner) -----
CASE="startup-declaration-still-passes"
ROOT="$(new_root)"
run_health "$ROOT" && rc=0 || rc=$?
[ "$rc" -eq 0 ] || fail "the startup declaration must remain valid evidence; got $rc; output: $OUT"

# --- case 33: unrelated recent log noise is NOT accepted as activity ---------
# Guards the widened pattern against degrading into "any recent log line".
CASE="unrelated-log-noise-fails"
ROOT="$(new_root)"
write_journal "$ROOT" terraphim-gitea-runner-2.service \
'Aug 13 09:10:00 bigbox terraphim-gitea-runner[4202]: [INFO] connected to https://git.terraphim.cloud
Aug 13 09:11:00 bigbox terraphim-gitea-runner[4202]: [DEBUG] tls session resumed
Aug 13 09:12:00 bigbox terraphim-gitea-runner[4202]: [WARN] config reloaded'
run_health "$ROOT" && rc=0 || rc=$?
[ "$rc" -ne 0 ] || fail "recent-but-unrelated log lines must not count as runner activity"

# --- case 34: no activity at all fails --------------------------------------
CASE="no-activity-fails"
ROOT="$(new_root)"
for unit in $SERVICES; do : > "${ROOT}/${unit}.journal"; done
run_health "$ROOT" && rc=0 || rc=$?
[ "$rc" -ne 0 ] || fail "a pool with no recent activity of any kind must fail"

# --- case 35: activity evidence stays bounded by --since --------------------
CASE="activity-evidence-remains-time-bounded"
ROOT="$(new_root)"
run_health "$ROOT" >/dev/null 2>&1
grep -q -- '--since' "${ROOT}/journalctl.argv" 2>/dev/null \
    || fail "the widened activity pattern must still be bounded by --since"

if [ "$FAILURES" -eq 0 ]; then
    echo "PASS: runner pool health contract (35 cases)"
    exit 0
fi
echo "FAILURES: $FAILURES" >&2
exit 1
