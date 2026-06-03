#!/usr/bin/env bash
# deploy-interim-ci.sh -- interim CI (Part A) for the #1910 polyrepos via ADF.
#
# Wires the 6 new Gitea polyrepos into the existing ADF orchestrator so each
# push/PR gets an `adf/build` commit status, reusing rch + sccache. No act_runner.
# Mirrors the proven better-auth-rust.toml pattern (project header + event_only
# build-runner spawned by handle_push). Run ON bigbox after review.
#
# Idempotent: safe to re-run. Does NOT touch terraphim.toml or the orchestrator
# binary -- only adds conf.d/<repo>.toml files, clones, and Gitea webhooks.
#
# Refs #1910. Convergence: each build-runner runs build-runner-llm.sh against the
# repo's own BUILD.md (the single command source shared with the future native
# runner), so cargo->rch transform + sccache->SeaweedFS apply automatically.
set -euo pipefail

GITEA_URL="${GITEA_URL:-https://git.terraphim.cloud}"
ORG="${ORG:-terraphim}"
CONFD="${CONFD:-/opt/ai-dark-factory/conf.d}"
WORKROOT="${WORKROOT:-/home/alex/projects/terraphim}"
RUNNER_SCRIPT="${RUNNER_SCRIPT:-/data/projects/terraphim/terraphim-ai/scripts/build-runner-llm.sh}"
# ADF webhook receiver. The orchestrator accepts unsigned webhooks (the live
# [webhook] block binds 172.18.0.1:9091 with no secret; existing project hooks
# have has_secret=false), so no HMAC secret is configured here.
ADF_WEBHOOK_URL="${ADF_WEBHOOK_URL:-http://172.18.0.1:9091/webhooks/gitea}"

# Run on bigbox as root / with sudo: conf.d is root-owned and the orchestrator is
# a system service.
: "${GITEA_TOKEN:?set GITEA_TOKEN (the bigbox gitea token used by other conf.d projects)}"

REPOS=(terraphim-core terraphim-config-persistence terraphim-service \
       terraphim-agents terraphim-kg-agents terraphim-clients)

emit_confd() {
  local repo="$1" wd="$2"
  # conf.d is root-owned; write via sudo so the rest of the script can run as the
  # unprivileged user (clones/webhooks owned by that user, not root).
  sudo tee "${CONFD}/${repo}.toml" >/dev/null <<TOML
[[projects]]
id = "${repo}"
name = "${repo}"
working_dir = "${wd}"
gitea = { owner = "${ORG}", repo = "${repo}", base_url = "${GITEA_URL}", token = "${GITEA_TOKEN}" }

# Interim CI build-runner (Part A, #1910). event_only -> spawned by handle_push.
# The agent MUST be named exactly "build-runner": handle_push resolves it via
# agent_registry.lookup_project(project, "build-runner") (scoped per project).
# Runs build-runner-llm.sh against this repo's BUILD.md: KG transforms
# cargo build/clippy/test -> rch exec (sccache->SeaweedFS); cargo fmt stays host.
# Posts adf/build commit status. Retire on native-runner cutover (active_lane).
[[agents]]
evolution_enabled = true
name = "build-runner"
layer = "Growth"
cli_tool = "/bin/bash"
max_cpu_seconds = 1800
grace_period_secs = 30
capabilities = ["build", "test", "adaptive-ci"]
event_only = true
project = "${repo}"
task = '''
source ~/.profile
export GITEA_URL=${GITEA_URL}

if [ -z "\${ADF_PUSH_SHA:-}" ]; then echo "build-runner: missing ADF_PUSH_SHA" >&2; exit 1; fi
if [ -z "\${ADF_PUSH_REF:-}" ]; then echo "build-runner: missing ADF_PUSH_REF" >&2; exit 1; fi

cd "\$GITEA_WORKING_DIR"
git fetch origin "\$ADF_PUSH_REF"
git checkout -f "\$ADF_PUSH_SHA"

# Reuse the canonical KG-first runner against THIS repo's BUILD.md.
# build-runner-llm.sh reads \$ADF_WORKING_DIR/BUILD.md, transforms cargo->rch,
# and posts adf/build for \$ADF_PUSH_SHA on \$GITEA_OWNER/\$GITEA_REPO.
export ADF_WORKING_DIR="\$GITEA_WORKING_DIR"
bash ${RUNNER_SCRIPT}
'''
TOML
  echo "  wrote ${CONFD}/${repo}.toml"
}

for repo in "${REPOS[@]}"; do
  echo "=== ${repo} ==="
  wd="${WORKROOT}/${repo}"

  # 1. clone if absent. Auth via a one-shot header (not the URL), so the token is
  #    never written into .git/config or the persisted remote. The remote stays
  #    token-free; the build-runner agent authenticates fetches via its GITEA_TOKEN env.
  if [ ! -d "${wd}/.git" ]; then
    echo "  cloning -> ${wd}"
    git -c "http.extraHeader=Authorization: token ${GITEA_TOKEN}" \
      clone "${GITEA_URL}/${ORG}/${repo}.git" "${wd}"
  else
    echo "  clone present"
  fi

  # 2. conf.d project + build-runner agent
  emit_confd "${repo}" "${wd}"

  # 3. Gitea webhook -> ADF (push + pull_request), idempotent by URL
  existing=$(curl -fsS -H "Authorization: token ${GITEA_TOKEN}" \
    "${GITEA_URL}/api/v1/repos/${ORG}/${repo}/hooks" \
    | python3 -c "import sys,json;print(any(h.get('config',{}).get('url')=='${ADF_WEBHOOK_URL}' for h in json.load(sys.stdin)))" 2>/dev/null || echo False)
  if [ "${existing}" = "True" ]; then
    echo "  webhook present"
  else
    curl -fsS -X POST -H "Authorization: token ${GITEA_TOKEN}" -H "Content-Type: application/json" \
      "${GITEA_URL}/api/v1/repos/${ORG}/${repo}/hooks" \
      -d "{\"type\":\"gitea\",\"active\":true,\"events\":[\"push\",\"pull_request\"],\"config\":{\"url\":\"${ADF_WEBHOOK_URL}\",\"content_type\":\"json\"}}" \
      >/dev/null && echo "  webhook created"
  fi
done

echo
echo "Restart the orchestrator to pick up conf.d/*.toml:"
echo "  sudo systemctl restart adf-orchestrator"
echo
echo "Then verify per repo: push a no-op commit and watch for adf/build pending->success."
