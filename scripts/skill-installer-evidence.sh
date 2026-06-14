#!/bin/bash
# skill-installer-evidence.sh -- Evidence capture wrapper for live-user validation
#
# Runs the full validation pipeline and packages evidence for sign-off:
#   1. Prerequisite check (skill-installer-prereqs.sh)
#   2. Sentinel smoke test (skill-installer-validation.sh)
#   3. Full catalogue validation (skill-installer-validation.sh with CATALOGUE)
#   4. Environment summary
#   5. Failure triage (auto-create follow-up issues for each FAIL/UNSUPPORTED)
#
# Evidence package:
#   $OUT_DIR/
#     01-prereqs.txt
#     02-sentinel.json
#     03-sentinel.md
#     04-full.json
#     05-full.md
#     06-environment.txt
#     07-summary.md
#     08-followups.txt (if gtr available)
#
# Usage:
#   TERRAPHIM_SKILLS_CATALOGUE=docs/runbooks/skills-catalogue.json \
#   TERRAPHIM_SKILLS_MANAGER_BIN=$(which tsm) \
#   TERRAPHIM_SKILLS_OWNER=terraphim TERRAPHIM_SKILLS_REPO=terraphim-ai \
#   TERRAPHIM_SKILLS_GITEA_ISSUE=2707 \
#     scripts/skill-installer-evidence.sh

set -uo pipefail

OUT_DIR="${TERRAPHIM_SKILLS_EVIDENCE_OUT:-$(mktemp -d /tmp/skills-evidence.XXXXXX)}"
CATALOGUE="${TERRAPHIM_SKILLS_CATALOGUE:-}"
MANAGER_BIN="${TERRAPHIM_SKILLS_MANAGER_BIN:-tsm}"
SENTINEL_SKILLS="${TERRAPHIM_SKILLS_SENTINEL_SKILLS:-code-review,testing,security-audit}"
GITEA_OWNER="${TERRAPHIM_SKILLS_OWNER:-terraphim}"
GITEA_REPO="${TERRAPHIM_SKILLS_REPO:-terraphim-ai}"
GITEA_PARENT_ISSUE="${TERRAPHIM_SKILLS_GITEA_ISSUE:-0}"
GITEA_TOKEN="${GITEA_TOKEN:-}"
DRY_RUN="${TERRAPHIM_SKILLS_DRY_RUN:-0}"
MAX_CELLS="${TERRAPHIM_SKILLS_MAX_CELLS:-1000}"

mkdir -p "$OUT_DIR"
echo "Evidence will be written to: $OUT_DIR"

# --- 1. Prerequisites --------------------------------------------------------
echo "" >&2
echo "=== 1/6: Prerequisite check ===" >&2
bash "$(dirname "$0")/skill-installer-prereqs.sh" > "$OUT_DIR/01-prereqs.txt" 2>&1 || true
echo "  -> $OUT_DIR/01-prereqs.txt" >&2

# --- 2. Environment summary --------------------------------------------------
echo "" >&2
echo "=== 2/6: Environment summary ===" >&2
{
    echo "# Environment summary"
    echo ""
    echo "Generated: $(date -u +"%Y-%m-%dT%H:%M:%SZ")"
    echo "Hostname: $(hostname 2>/dev/null || echo unknown)"
    echo "User: $(whoami 2>/dev/null || echo unknown)"
    echo "Shell: $SHELL"
    echo "OS: $(uname -a 2>/dev/null || echo unknown)"
    echo ""
    echo "## CLI versions"
    for BIN in tsm claude codex pi pi-rust grok adf adf-curl adf-ctl; do
        if command -v "$BIN" >/dev/null 2>&1; then
            echo "$BIN: $($BIN --version 2>/dev/null | head -1)"
        fi
    done
    echo ""
    echo "## Token presence"
    if [ -n "${TSM_TOKEN:-}" ]; then
        echo "TSM_TOKEN: set (length ${#TSM_TOKEN}, value redacted)"
    else
        echo "TSM_TOKEN: not set"
    fi
    if [ -s "$HOME/.terraphim/token" ]; then
        echo "~/.terraphim/token: present ($(wc -c < "$HOME/.terraphim/token" | xargs) bytes, content redacted)"
    else
        echo "~/.terraphim/token: missing"
    fi
    echo ""
    echo "## Catalogue"
    if [ -n "$CATALOGUE" ] && [ -f "$CATALOGUE" ]; then
        echo "Path: $CATALOGUE"
        echo "Source: $(python3 -c "import json; print(json.load(open('$CATALOGUE')).get('source','unknown'))" 2>/dev/null)"
        echo "Count: $(python3 -c "import json; print(json.load(open('$CATALOGUE')).get('skill_count','?'))" 2>/dev/null)"
    else
        echo "Path: not set or not found"
    fi
} > "$OUT_DIR/06-environment.txt"
echo "  -> $OUT_DIR/06-environment.txt" >&2

# --- 3. Sentinel smoke test --------------------------------------------------
echo "" >&2
echo "=== 3/6: Sentinel smoke test ===" >&2
TERRAPHIM_SKILLS_SENTINEL_SKILLS="$SENTINEL_SKILLS" \
TERRAPHIM_SKILLS_DRY_RUN="$DRY_RUN" \
TERRAPHIM_SKILLS_MANAGER_BIN="$MANAGER_BIN" \
  bash "$(dirname "$0")/skill-installer-validation.sh" \
    > "$OUT_DIR/02-sentinel.json" 2> "$OUT_DIR/03-sentinel.md" || true
SENTINEL_RC=$?
echo "  -> $OUT_DIR/02-sentinel.json (exit $SENTINEL_RC)" >&2
echo "  -> $OUT_DIR/03-sentinel.md" >&2

# --- 4. Full catalogue run ---------------------------------------------------
if [ -n "$CATALOGUE" ] && [ -f "$CATALOGUE" ]; then
    echo "" >&2
    echo "=== 4/6: Full catalogue validation ===" >&2
    TERRAPHIM_SKILLS_CATALOGUE="$CATALOGUE" \
    TERRAPHIM_SKILLS_DRY_RUN="$DRY_RUN" \
    TERRAPHIM_SKILLS_MANAGER_BIN="$MANAGER_BIN" \
    TERRAPHIM_SKILLS_MAX_CELLS="$MAX_CELLS" \
      bash "$(dirname "$0")/skill-installer-validation.sh" \
        > "$OUT_DIR/04-full.json" 2> "$OUT_DIR/05-full.md" || true
    FULL_RC=$?
    echo "  -> $OUT_DIR/04-full.json (exit $FULL_RC)" >&2
    echo "  -> $OUT_DIR/05-full.md" >&2
fi

# --- 5. Summary + sign-off checklist ----------------------------------------
echo "" >&2
echo "=== 5/6: Summary and sign-off checklist ===" >&2
{
    echo "# Sign-off summary"
    echo ""
    echo "Evidence dir: $OUT_DIR"
    echo "Generated: $(date -u +"%Y-%m-%dT%H:%M:%SZ")"
    echo ""
    echo "## Sign-off checklist"
    echo ""
    echo "- [ ] All required CLIs installed (see 01-prereqs.txt)"
    echo "- [ ] Token present and working (01-prereqs.txt)"
    echo "- [ ] Sentinel smoke run completed (02-sentinel.json, 03-sentinel.md)"
    echo "- [ ] Full catalogue run completed (04-full.json, 05-full.md)"
    echo "- [ ] Every FAIL/UNSUPPORTED has a follow-up issue or approved waiver"
    echo "- [ ] No secrets emitted in any artefact (06-environment.txt)"
    echo ""
    echo "## Artefacts"
    echo ""
    echo "| # | File | Purpose |"
    echo "|---|------|---------|"
    echo "| 1 | 01-prereqs.txt | Prerequisite check output |"
    echo "| 2 | 02-sentinel.json | Sentinel smoke JSON matrix |"
    echo "| 3 | 03-sentinel.md | Sentinel smoke Markdown matrix |"
    echo "| 4 | 04-full.json | Full catalogue JSON matrix |"
    echo "| 5 | 05-full.md | Full catalogue Markdown matrix |"
    echo "| 6 | 06-environment.txt | Environment summary |"
    echo "| 7 | 07-summary.md | This file |"
    echo "| 8 | 08-followups.txt | Auto-created follow-up issues |"
    echo ""
    echo "## Posting to Gitea"
    echo ""
    if [ -n "$GITEA_TOKEN" ] && [ "$GITEA_PARENT_ISSUE" != "0" ]; then
        echo "Gitea credentials present. To post evidence:"
        echo ""
        echo "  gtr comment --owner $GITEA_OWNER --repo $GITEA_REPO --index $GITEA_PARENT_ISSUE --body \"\$(cat $OUT_DIR/05-full.md)\""
        echo ""
    else
        echo "Gitea credentials not set. Manually post $OUT_DIR/05-full.md as a comment on issue #$GITEA_PARENT_ISSUE."
    fi
} > "$OUT_DIR/07-summary.md"
echo "  -> $OUT_DIR/07-summary.md" >&2

# --- 6. Failure triage --------------------------------------------------------
if [ -f "$OUT_DIR/04-full.json" ] && command -v gtr >/dev/null 2>&1 && [ -n "$GITEA_TOKEN" ]; then
    echo "" >&2
    echo "=== 6/6: Failure triage (auto-creating follow-up issues) ===" >&2
    {
        echo "# Follow-up issues"
        echo ""
        FOLLOWUP_COUNT=$(python3 -c "
import json
with open('$OUT_DIR/04-full.json') as f:
    d = json.load(f)
count = 0
for row in d.get('matrix', []):
    skill = row.get('skill','')
    for cli, cell in row.items():
        if cli == 'skill':
            continue
        if isinstance(cell, dict):
            status = cell.get('status','')
            if status in ('FAIL','UNSUPPORTED'):
                count += 1
print(count)
" 2>/dev/null)
        echo "Total follow-up issues: $FOLLOWUP_COUNT"
        if [ "$FOLLOWUP_COUNT" = "0" ]; then
            echo "None required."
        fi
    } > "$OUT_DIR/08-followups.txt"
    echo "  -> $OUT_DIR/08-followups.txt (summary)" >&2

    # Auto-create issues for each FAIL/UNSUPPORTED
    python3 -c "
import json, subprocess, sys, os
with open('$OUT_DIR/04-full.json') as f:
    d = json.load(f)
issues_created = 0
parent = os.environ.get('GITEA_PARENT_ISSUE', '0')
for row in d.get('matrix', []):
    skill = row.get('skill','')
    for cli, cell in row.items():
        if cli == 'skill':
            continue
        if not isinstance(cell, dict):
            continue
        status = cell.get('status','')
        if status not in ('FAIL','UNSUPPORTED'):
            continue
        evidence = cell.get('evidence','')
        title = f'Skills validation: {status} for {skill} on {cli}'
        body = f'''## Validation failure

- **Skill**: {skill}
- **CLI**: {cli}
- **Status**: {status}
- **Evidence**: {evidence}

## Source

Generated by \`scripts/skill-installer-evidence.sh\` from issue #{parent}.

Ref parent: #{parent}
'''
        result = subprocess.run([
            'gtr', 'create-issue',
            '--owner', '$GITEA_OWNER',
            '--repo', '$GITEA_REPO',
            '--title', title,
            '--body', body,
        ], capture_output=True, text=True, env=dict(os.environ))
        if result.returncode == 0:
            issues_created += 1
            print(f'  Created: {title}', file=sys.stderr)
        else:
            print(f'  Failed: {result.stderr}', file=sys.stderr)
" 2>>"$OUT_DIR/08-followups.txt" || true
    echo "  Follow-up issues created and appended to 08-followups.txt" >&2
else
    {
        echo "# Follow-up issues"
        echo ""
        echo "Auto-create not run (gtr unavailable or no Gitea token)."
        echo "Manually create issues for each FAIL/UNSUPPORTED cell in 05-full.md."
    } > "$OUT_DIR/08-followups.txt"
    echo "  -> $OUT_DIR/08-followups.txt (manual mode)" >&2
fi

# --- Final -------------------------------------------------------------------
echo "" >&2
echo "=== Evidence capture complete ===" >&2
echo "Package: $OUT_DIR" >&2
echo "Summary: $OUT_DIR/07-summary.md" >&2
