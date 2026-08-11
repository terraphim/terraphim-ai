#!/usr/bin/env bash
# Auto-merge agent: reads the latest structural review comment on a PR,
# extracts confidence score, and merges if 4+ with no P0/P1.
set -euo pipefail

REPO="${ADF_REPO:-terraphim-ai}"
OWNER="${ADF_OWNER:-terraphim}"
PR_NUM="${1:-}"

if [ -z "$PR_NUM" ]; then
    echo "[auto-merge] Usage: $0 <PR_NUMBER>" >&2
    exit 1
fi

echo "[auto-merge] Checking PR #${PR_NUM} on ${OWNER}/${REPO}"

# Fetch PR state
PR_STATE=$(gitea-robot view-pull --owner "$OWNER" --repo "$REPO" --index "$PR_NUM" 2>/dev/null | python3 -c "
import json,sys; d=json.load(sys.stdin)
print(json.dumps({'merged': d.get('merged', False), 'merged_by': d.get('merged_by'), 'mergeable': d.get('mergeable', True)}))
" 2>/dev/null)

ALREADY_MERGED=$(echo "$PR_STATE" | python3 -c "import json,sys; print(json.load(sys.stdin).get('merged', False))" 2>/dev/null)

if [ "$ALREADY_MERGED" = "True" ]; then
    echo "[auto-merge] PR #${PR_NUM} already merged — nothing to do"
    exit 0
fi

# Fetch PR comments for review scores
COMMENTS=$(gitea-robot view-pull --owner "$OWNER" --repo "$REPO" --index "$PR_NUM" 2>/dev/null | python3 -c "
import json,sys; d=json.load(sys.stdin)
print(d.get('comments', 0))
" 2>/dev/null)

echo "  Comments: $COMMENTS"

# Search review comments for Confidence Score
REVIEW_DATA=$(curl -sf -H "Authorization: token ${GITEA_TOKEN}" \
  "https://git.terraphim.cloud/api/v1/repos/${OWNER}/${REPO}/issues/${PR_NUM}/comments" 2>/dev/null | python3 -c "
import json,sys,re
comments = json.load(sys.stdin)
for c in (comments if isinstance(comments, list) else []):
    body = c.get('body', '')
    match = re.search(r'Confidence Score:\s*(\d)/5', body)
    if match:
        score = int(match.group(1))
        # Check for P0/P1 findings
        has_p0 = bool(re.search(r'\*\*P0\b', body))
        has_p1 = bool(re.search(r'\*\*P1\b', body))
        print(json.dumps({'score': score, 'p0': has_p0, 'p1': has_p1}))
        break
" 2>/dev/null)

if [ -z "$REVIEW_DATA" ] || [ "$REVIEW_DATA" = "null" ]; then
    echo "[auto-merge] ⚠️  No structural review found on PR #${PR_NUM}"
    echo "  Post a review with 'Confidence Score: X/5' before auto-merge can proceed."
    exit 0
fi

SCORE=$(echo "$REVIEW_DATA" | python3 -c "import json,sys; print(json.load(sys.stdin)['score'])")
HAS_P0=$(echo "$REVIEW_DATA" | python3 -c "import json,sys; print(json.load(sys.stdin)['p0'])")
HAS_P1=$(echo "$REVIEW_DATA" | python3 -c "import json,sys; print(json.load(sys.stdin)['p1'])")

echo "  Review score: ${SCORE}/5"
echo "  P0 findings: ${HAS_P0}"
echo "  P1 findings: ${HAS_P1}"

if [ "$SCORE" -ge 4 ] && [ "$HAS_P0" = "False" ] && [ "$HAS_P1" = "False" ]; then
    echo "[auto-merge] ✅ MERGE CRITERIA MET (score=${SCORE}/5, no P0/P1)"
    gitea-robot merge-pull --owner "$OWNER" --repo "$REPO" --index "$PR_NUM" 2>&1
    echo "[auto-merge] PR #${PR_NUM} merged"
else
    echo "[auto-merge] ❌ NOT MERGING — criteria not met:"
    echo "  Required: score >= 4, no P0, no P1"
    echo "  Actual:   score=${SCORE}/5, P0=${HAS_P0}, P1=${HAS_P1}"
fi
