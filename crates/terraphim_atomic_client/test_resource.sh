#!/bin/bash

# Usage: ./test_resource.sh <shortname> <description> [class]
# Example: ./test_resource.sh my-resource "My test resource" Collection

# Default values
SHORTNAME=${1:-"test-resource-$(date +%s)"}
DESCRIPTION=${2:-"A test resource created via shell script"}
CLASS=${3:-"Collection"}
SERVER_URL=${ATOMIC_SERVER_URL:-"http://localhost:9883"}

# Get the token from .env file
if [ -f .env ]; then
    TOKEN=$(grep ATOMIC_SERVER_SECRET .env | cut -d '=' -f2 | tr -d '"')
else
    echo "Error: .env file not found"
    exit 1
fi

# Run the test_signature program to create the commit
echo "Creating commit for resource: $SHORTNAME ($CLASS)"
cd test_signature && ATOMIC_SERVER_SECRET="$TOKEN" cargo run "$SHORTNAME" "$DESCRIPTION" "$CLASS" > ../commit.json

# Send the commit to the server
cd ..
echo "Sending commit to server..."
curl -v -H "Content-Type: application/json" -d @commit.json $SERVER_URL/commit

# Check if the resource was created
echo -e "\n\nChecking if resource was created..."
RESOURCE_ID=$(jq -r '."https://atomicdata.dev/properties/subject"' commit.json)
echo "Resource ID: $RESOURCE_ID"
curl -s -H "Accept: application/json" $RESOURCE_ID | jq '."@id", .description, ."is-a"'
