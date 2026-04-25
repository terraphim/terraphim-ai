#!/usr/bin/env bash
# Reject any staged file under projects/ to prevent customer data leaks.
# This hook is the first line of defence alongside .gitignore and
# the Gitea pre-receive hook.
set -euo pipefail

for f in "$@"; do
    case "$f" in
        projects/*)
            echo "BLOCKED: '$f' matches protected path (projects/)." >&2
            echo "Customer data must live in the private repository." >&2
            echo "To bypass (NOT recommended): git commit --no-verify" >&2
            exit 1
            ;;
    esac
done
exit 0
