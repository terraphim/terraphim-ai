#!/usr/bin/env bash
# Gitea pre-receive hook: reject pushes containing files under projects/
#
# INSTALL (on the Gitea server):
#   1. Copy this file to /var/lib/gitea/repositories/terraphim/terraphim-ai.git/hooks/pre-receive.d/reject-customer-data.sh
#      (adjust path to your Gitea repositories directory)
#   2. chmod +x reject-customer-data.sh
#   3. Test with: git push origin main (should succeed, no projects/ files)
#
# This hook runs server-side and CANNOT be bypassed with --no-verify.
set -euo pipefail

ZERO="0000000000000000000000000000000000000000"

while read -r oldrev newrev refname; do
    if [ "$newrev" = "$ZERO" ]; then
        continue
    fi

    if [ "$oldrev" = "$ZERO" ]; then
        revs="$newrev"
    else
        revs="${oldrev}..${newrev}"
    fi

    blocked=$(git diff --name-only --diff-filter=ACMR "$oldrev" "$newrev" 2>/dev/null | grep '^projects/' || true)

    if [ -n "$blocked" ]; then
        echo "" >&2
        echo "========================================" >&2
        echo "REJECTED: Customer data detected in push" >&2
        echo "========================================" >&2
        echo "" >&2
        echo "The following files are under projects/ and must not" >&2
        echo "be pushed to the public repository:" >&2
        echo "" >&2
        echo "$blocked" | while read -r f; do
            echo "  $f" >&2
        done
        echo "" >&2
        echo "Customer data belongs in the private repository." >&2
        echo "========================================" >&2
        exit 1
    fi
done

exit 0
