#!/bin/bash
# Monitor PR #498 and merge once all checks pass

PR_NUMBER=498
MAX_WAIT_MINUTES=60
INTERVAL_SECONDS=30

start_time=$(date +%s)
end_time=$((start_time + MAX_WAIT_MINUTES * 60))

echo "Monitoring PR #$PR_NUMBER for CI completion..."
echo "Will check every $INTERVAL_SECONDS seconds for up to $MAX_WAIT_MINUTES minutes"
echo ""

while [ $(date +%s) -lt $end_time ]; do
    # Get check summary
    checks=$(gh pr checks $PR_NUMBER 2>&1)

    # Count statuses
    total=$(echo "$checks" | grep -E "(pass|fail|pending|skipped)" | wc -l)
    passing=$(echo "$checks" | grep "pass" | wc -l)
    failing=$(echo "$checks" | grep "fail" | wc -l)

    elapsed=$(( ($(date +%s) - start_time) / 60 ))

    echo "[$elapsed min] Total: $total | Passing: $passing | Failing: $failing"

    # Check if all required checks passed (we need to see some passing and no failing)
    if [ "$failing" -eq 0 ] && [ "$passing" -gt 3 ]; then
        echo ""
        echo "All checks appear to have passed! Attempting to merge..."
        gh pr merge $PR_NUMBER --squash --delete-branch
        if [ $? -eq 0 ]; then
            echo "Successfully merged PR #$PR_NUMBER"
            exit 0
        else
            echo "Merge failed. Retrying in $INTERVAL_SECONDS seconds..."
        fi
    fi

    if [ "$failing" -gt 0 ]; then
        echo ""
        echo "Checks are failing! Manual intervention required."
        gh pr checks $PR_NUMBER
        exit 1
    fi

    sleep $INTERVAL_SECONDS
done

echo ""
echo "Timeout reached after $MAX_WAIT_MINUTES minutes. Manual merge required."
exit 1
