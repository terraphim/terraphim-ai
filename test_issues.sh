#!/bin/bash

echo "=== Testing Current Issues ==="
echo

echo "1. Testing autocomplete endpoint functionality:"
echo "   Testing /thesaurus/Terraphim%20Engineer endpoint..."
curl -s "http://127.0.0.1:8000/thesaurus/Terraphim%20Engineer" | jq '{status, thesaurus_count: (.thesaurus | length), sample_entries: (.thesaurus | to_entries | .[0:3] | map({key: .key, value: .value}))}' 2>/dev/null || echo "   Server may not be running"

echo
echo "2. Testing document search with KG-enabled role:"
echo "   Searching for 'knowledge graph' in Terraphim Engineer role..."
curl -s -X POST "http://127.0.0.1:8000/documents/search" \
  -H "Content-Type: application/json" \
  -d '{"search_term": "knowledge graph", "role": "Terraphim Engineer"}' | \
  jq '{status, results_count: (.results | length), first_result: (.results[0] | {title, body: (.body | length), url})}' 2>/dev/null || echo "   Server may not be running or search failed"

echo
echo "3. Testing document KG term search:"
echo "   Testing KG term search endpoint..."
curl -s "http://127.0.0.1:8000/roles/Terraphim%20Engineer/kg_search?term=knowledge%20graph" | \
  jq '{status, results_count: (.results | length)}' 2>/dev/null || echo "   Server may not be running"

echo
echo "4. Testing document body for kg: links:"
echo "   Searching for documents that should contain kg: links..."
curl -s -X POST "http://127.0.0.1:8000/documents/search" \
  -H "Content-Type: application/json" \
  -d '{"search_term": "knowledge graph", "role": "Terraphim Engineer"}' | \
  jq '.results[0].body' | grep -o 'kg:[^)]*' | head -5 2>/dev/null || echo "   No kg: links found in document bodies"

echo
echo "5. Checking server configuration:"
echo "   Getting server configuration..."
curl -s "http://127.0.0.1:8000/config" | jq '{selected_role: .config.selected_role, roles_with_kg: [.config.roles | to_entries[] | select(.value.kg != null) | .key]}' 2>/dev/null || echo "   Server may not be running"

echo
echo "=== Test completed ==="
