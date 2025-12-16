#!/bin/bash

echo "=== Testing op read directly ==="
TOKEN1=$(op read "op://TerraphimPlatform/github-packages-token/token")
echo "Direct read: $TOKEN1"
echo "Length: ${#TOKEN1}"
echo "First 10: ${TOKEN1:0:10}"

echo ""
echo "=== Testing in function ==="

get_token() {
    local op_url="$1"
    local token
    if ! token=$(op read "$op_url" 2>/dev/null); then
        echo "Failed to read token"
        return 1
    fi
    token=$(echo "$token" | tr -d '[:space:]')
    echo "Function read: $token"
    echo "Length: ${#token}"
    echo "First 10: ${token:0:10}"
    echo "$token"
}

get_token "op://TerraphimPlatform/github-packages-token/token"