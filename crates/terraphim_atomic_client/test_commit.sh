#!/bin/bash
TOKEN=$(grep ATOMIC_SERVER_SECRET .env | cut -d "=" -f2 | tr -d "\"")
TIMESTAMP=$(date +%s)
SUBJECT="http://localhost:9883/test-resource-$TIMESTAMP"
SIGNER="http://localhost:9883/agents/zZ2S0HTg17PM5YLQJHYkyFxG/BaL0uXShw7KaB1CJOg="
SIGNATURE="placeholder_signature"
echo "{\"@id\": \"http://localhost:9883/commits/test-$TIMESTAMP\", \"subject\": \"$SUBJECT\", \"created-at\": $TIMESTAMP, \"signer\": \"$SIGNER\", \"set\": {\"https://atomicdata.dev/properties/shortname\": \"test-resource-$TIMESTAMP\", \"https://atomicdata.dev/properties/description\": \"A test resource created via shell script\"}, \"destroy\": false, \"signature\": \"$SIGNATURE\"}" > commit.json
echo "Created commit.json:"
cat commit.json
echo -e "
Sending commit to server..."
curl -v -H "Content-Type: application/json" -d @commit.json http://localhost:9883/commit
