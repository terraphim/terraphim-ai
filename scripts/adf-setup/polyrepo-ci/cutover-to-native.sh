#!/usr/bin/env bash
#
# cutover-to-native.sh -- move one polyrepo from the interim ADF lane
# (adf/build) to the native Gitea Actions runner lane (native-ci / build (push)).
#
# This codifies the runbook validated on terraphim-kg-agents for Gitea #2080.
# It is idempotent: re-running re-checks each step and only acts where needed.
#
# WHAT IT DOES (in order, with safety gates):
#   1. Add the repo to the native runner's RUNNER_ACTIVE_REPOS and restart it.
#   2. Land .gitea/workflows/native-ci.yml on the default branch via a PR
#      (the PR's required adf/build gate must pass; native-ci also runs).
#   3. Wait for native-ci to go green on the default branch.
#   4. Swap the required branch-protection context: adf/build -> native-ci/build.
#   5. Disable the interim build-runner (conf.d enabled=false) + restart the
#      ADF orchestrator, so pushes stop double-building.
#   6. Print the final state for verification.
#
# It refuses to swap branch protection (step 4) until native-ci is proven green
# (step 3), so a failed native build can never leave the repo ungated.
#
# REQUIREMENTS (operator machine):
#   - GITEA_TOKEN exported (write access to the terraphim org), used over HTTPS.
#   - gtr (gitea-robot) on PATH for PR create/merge.
#   - ssh access to `bigbox` (the native runner + ADF orchestrator host).
#
# USAGE:
#   GITEA_TOKEN=... ./cutover-to-native.sh <repo>            # e.g. terraphim-core
#   GITEA_TOKEN=... ./cutover-to-native.sh <repo> --rollback # reverse steps 4-5
#
# ROLLBACK reverses the irreversible-ish steps: re-require adf/build and
# re-enable the interim build-runner. It does NOT remove native-ci.yml (leaving
# it is harmless -- it simply stops being the required gate).

set -euo pipefail

OWNER="terraphim"
GITEA_URL="${GITEA_URL:-https://git.terraphim.cloud}"
BIGBOX="${BIGBOX_HOST:-bigbox}"
RUNNER_ENV="/home/alex/.config/terraphim-gitea-runner/env"
RUNNER_SVC="terraphim-gitea-runner.service"        # systemd --user on bigbox
ADF_CONF_DIR="/opt/ai-dark-factory/conf.d"
ADF_SVC="adf-orchestrator.service"                 # system service on bigbox
NATIVE_CONTEXT="native-ci / build (push)"
INTERIM_CONTEXT="adf/build"

REPO="${1:-}"
MODE="${2:-cutover}"
[ -n "$REPO" ] || { echo "usage: $0 <repo> [--rollback]" >&2; exit 2; }
[ -n "${GITEA_TOKEN:-}" ] || { echo "GITEA_TOKEN must be set (HTTPS git + API auth)" >&2; exit 2; }

AUTH=(-H "Authorization: token ${GITEA_TOKEN}")
API="${GITEA_URL}/api/v1/repos/${OWNER}/${REPO}"
PUSH_URL="https://oauth2:${GITEA_TOKEN}@${GITEA_URL#https://}/${OWNER}/${REPO}.git"

log() { printf '\n=== %s ===\n' "$*"; }

# --- the workflow the native lane runs (rch offloads compilation; cargo test
# runs on host with sccache via the runner's environment). ---
native_ci_yaml() {
  cat <<'YAML'
name: native-ci
on:
  push:
  workflow_dispatch:
jobs:
  build:
    runs-on: terraphim-native
    steps:
      - run: cargo fmt --all -- --check
      - run: cargo clippy --workspace --all-targets -- -D warnings
      - run: cargo build --workspace
      - run: cargo test --workspace --lib --no-fail-fast
YAML
}

# Latest state for a given status context on a commit (newest-first dedupe).
context_state() { # $1=sha $2=context
  curl -fsS "${AUTH[@]}" "${API}/commits/${1}/statuses" 2>/dev/null | \
    REPO_CTX="$2" python3 -c '
import sys, json, os
ctx = os.environ["REPO_CTX"]
seen = {}
for s in json.load(sys.stdin):
    seen.setdefault(s["context"], s["status"])
print(seen.get(ctx, "none"))'
}

wait_for_context() { # $1=sha $2=context $3=max_polls
  local sha="$1" ctx="$2" max="${3:-40}" i st
  for ((i=1; i<=max; i++)); do
    st="$(context_state "$sha" "$ctx")"
    echo "  [$i] ${ctx} = ${st}"
    case "$st" in
      success) return 0 ;;
      failure|error) return 1 ;;
    esac
    sleep 15
  done
  return 2
}

default_branch() {
  curl -fsS "${AUTH[@]}" "${API}" 2>/dev/null | python3 -c 'import sys,json;print(json.load(sys.stdin)["default_branch"])'
}

rollback() {
  log "ROLLBACK: re-require ${INTERIM_CONTEXT} and re-enable interim build-runner"
  local br; br="$(default_branch)"
  curl -fsS -X PATCH "${AUTH[@]}" -H "Content-Type: application/json" \
    "${API}/branch_protections/${br}" \
    -d "{\"enable_status_check\":true,\"status_check_contexts\":[\"${INTERIM_CONTEXT}\"]}" >/dev/null
  echo "  branch protection -> requires ${INTERIM_CONTEXT}"
  ssh "$BIGBOX" "sudo python3 - '${ADF_CONF_DIR}/${REPO}.toml' <<'PY'
import sys
p=sys.argv[1]
lines=open(p).read().splitlines()
out=[l for l in lines if l.strip()!='enabled = false  # interim lane retired post native-runner cutover (terraphim-ai #2080)']
open(p,'w').write('\n'.join(out)+'\n')
PY
sudo systemctl restart ${ADF_SVC}"
  echo "  interim build-runner re-enabled; ${ADF_SVC} restarted"
  echo "ROLLBACK complete. (native-ci.yml left in place; harmless.)"
}

[ "$MODE" = "--rollback" ] && { rollback; exit 0; }

# ---------------------------------------------------------------------------
# Step 1: activate the repo on the native runner (idempotent).
# ---------------------------------------------------------------------------
log "Step 1/6: add ${REPO} to RUNNER_ACTIVE_REPOS and restart the native runner"
ssh "$BIGBOX" "export XDG_RUNTIME_DIR=/run/user/\$(id -u)
cp -p '${RUNNER_ENV}' '${RUNNER_ENV}.bak-\$(date +%Y%m%d-%H%M%S)'
REPO='${REPO}' python3 - '${RUNNER_ENV}' <<'PY'
import os, sys
p=sys.argv[1]; repo=os.environ['REPO']
lines=open(p).read().splitlines(); out=[]; touched=False
for l in lines:
    if l.startswith('RUNNER_ACTIVE_REPOS='):
        repos=[r for r in l.split('=',1)[1].split(',') if r]
        if repo not in repos: repos.append(repo)
        out.append('RUNNER_ACTIVE_REPOS='+','.join(repos)); touched=True
    else:
        out.append(l)
if not touched: out.append('RUNNER_ACTIVE_REPOS='+repo)
open(p,'w').write('\n'.join(out)+'\n')
print('RUNNER_ACTIVE_REPOS ->', [o for o in out if o.startswith('RUNNER_ACTIVE_REPOS')][0])
PY
systemctl --user restart '${RUNNER_SVC}'
echo active: \$(systemctl --user is-active '${RUNNER_SVC}')"

# ---------------------------------------------------------------------------
# Step 2: land native-ci.yml on the default branch via a PR.
# ---------------------------------------------------------------------------
BR="$(default_branch)"
log "Step 2/6: open PR to add .gitea/workflows/native-ci.yml on ${BR}"
WORK="$(mktemp -d)"; trap 'rm -rf "$WORK"' EXIT
git -C "$WORK" init -q
git -C "$WORK" remote add origin "$PUSH_URL"
git -C "$WORK" fetch -q --depth 1 origin "$BR"
git -C "$WORK" checkout -q -b "cutover/2080-native-ci" FETCH_HEAD
mkdir -p "$WORK/.gitea/workflows"
native_ci_yaml > "$WORK/.gitea/workflows/native-ci.yml"
if git -C "$WORK" diff --quiet -- .gitea/workflows/native-ci.yml 2>/dev/null && \
   git -C "$WORK" ls-files --error-unmatch .gitea/workflows/native-ci.yml >/dev/null 2>&1; then
  echo "  native-ci.yml already present and unchanged on ${BR}; skipping PR"
else
  git -C "$WORK" add .gitea/workflows/native-ci.yml
  git -C "$WORK" -c user.email=adf@terraphim.cloud -c user.name="ADF Cutover" \
    -c core.hooksPath=/dev/null commit -q -m "ci(#2080): add native-ci workflow (terraphim-native cutover)"
  git -C "$WORK" push -q origin cutover/2080-native-ci
  HEAD_SHA="$(git -C "$WORK" rev-parse HEAD)"
  gtr create-pull --owner "$OWNER" --repo "$REPO" \
    --title "ci(#2080): add native-ci workflow (terraphim-native cutover)" \
    --base "$BR" --head cutover/2080-native-ci \
    --body "Native runner cutover. Refs terraphim/terraphim-ai#2080." >/dev/null 2>&1 || true
  PRNUM="$(curl -fsS "${AUTH[@]}" "${API}/pulls?state=open&limit=50" 2>/dev/null | \
    python3 -c 'import sys,json;[print(p["number"]) for p in json.load(sys.stdin) if p["head"]["ref"]=="cutover/2080-native-ci"]' | head -1)"
  echo "  PR #${PRNUM} (head ${HEAD_SHA:0:8}); waiting for both checks"
  wait_for_context "$HEAD_SHA" "$INTERIM_CONTEXT" 40 || { echo "  ${INTERIM_CONTEXT} not green; aborting"; exit 1; }
  wait_for_context "$HEAD_SHA" "$NATIVE_CONTEXT" 40 || { echo "  ${NATIVE_CONTEXT} not green; aborting"; exit 1; }
  gtr merge-pull --owner "$OWNER" --repo "$REPO" --index "$PRNUM"
  curl -fsS -X DELETE "${AUTH[@]}" "${API}/branches/cutover/2080-native-ci" >/dev/null 2>&1 || true
fi

# ---------------------------------------------------------------------------
# Step 3: confirm native-ci is green on the default branch (GATE).
# ---------------------------------------------------------------------------
log "Step 3/6: confirm ${NATIVE_CONTEXT} green on ${BR} (gate before protection swap)"
MAIN_SHA="$(curl -fsS "${AUTH[@]}" "${API}/branches/${BR}" 2>/dev/null | python3 -c 'import sys,json;print(json.load(sys.stdin)["commit"]["id"])')"
wait_for_context "$MAIN_SHA" "$NATIVE_CONTEXT" 40 || { echo "  native-ci not green on ${BR}; NOT swapping protection"; exit 1; }

# ---------------------------------------------------------------------------
# Step 4: swap required branch-protection context.
# ---------------------------------------------------------------------------
log "Step 4/6: branch protection ${INTERIM_CONTEXT} -> ${NATIVE_CONTEXT}"
curl -fsS -X PATCH "${AUTH[@]}" -H "Content-Type: application/json" \
  "${API}/branch_protections/${BR}" \
  -d "{\"enable_status_check\":true,\"status_check_contexts\":[\"${NATIVE_CONTEXT}\"]}" | \
  python3 -c 'import sys,json;print("  required ->",json.load(sys.stdin).get("status_check_contexts"))'

# ---------------------------------------------------------------------------
# Step 5: disable the interim build-runner + restart the orchestrator.
# ---------------------------------------------------------------------------
log "Step 5/6: disable interim build-runner for ${REPO} and restart ${ADF_SVC}"
ssh "$BIGBOX" "set -e
F='${ADF_CONF_DIR}/${REPO}.toml'
sudo cp -p \"\$F\" \"\$F.bak-precutover-\$(date +%Y%m%d-%H%M%S)\"
sudo python3 - \"\$F\" <<'PY'
import sys
p=sys.argv[1]
lines=open(p).read().splitlines(); out=[]; done=False
marker='enabled = false  # interim lane retired post native-runner cutover (terraphim-ai #2080)'
already=any(l.strip()==marker for l in lines)
for l in lines:
    out.append(l)
    if not done and not already and l.strip()=='[[agents]]':
        out.append(marker); done=True
open(p,'w').write('\n'.join(out)+'\n')
print('disabled' if (done or already) else 'NO [[agents]] block found')
PY
sudo systemctl restart '${ADF_SVC}'
echo \"${ADF_SVC}: \$(systemctl is-active '${ADF_SVC}')\""

# ---------------------------------------------------------------------------
# Step 6: report final state.
# ---------------------------------------------------------------------------
log "Step 6/6: final state for ${OWNER}/${REPO}"
curl -fsS "${AUTH[@]}" "${API}/branch_protections/${BR}" 2>/dev/null | \
  python3 -c 'import sys,json;b=json.load(sys.stdin);print("  required contexts:",b.get("status_check_contexts"))'
echo "  Cutover complete. Next push to ${BR} is gated by '${NATIVE_CONTEXT}' only."
echo "  Rollback: $0 ${REPO} --rollback"
