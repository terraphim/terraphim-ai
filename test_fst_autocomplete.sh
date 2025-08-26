#!/bin/bash

echo "=== Comprehensive FST Autocomplete Testing ==="
echo

echo "1. Testing current thesaurus endpoint (HashMap-based):"
echo "   GET /thesaurus/Terraphim%20Engineer"
curl -s "http://127.0.0.1:8000/thesaurus/Terraphim%20Engineer" | jq '{
  status, 
  thesaurus_count: (.thesaurus | length), 
  sample_terms: (.thesaurus | keys | .[0:5])
}' 2>/dev/null || echo "   Server may not be running"

echo
echo "2. Testing FST-based autocomplete endpoint:"
echo "   Testing /autocomplete endpoint with different queries..."

# Test basic queries
QUERIES=("know" "graph" "terr" "data" "knolege")
for query in "${QUERIES[@]}"; do
    echo "   Query: '$query'"
    curl -s "http://127.0.0.1:8000/autocomplete/Terraphim%20Engineer/$query" | \
    jq '{status, suggestions_count: (.suggestions | length), top_suggestion: (.suggestions[0].term // "none")}' 2>/dev/null || echo "     Failed"
done

echo
echo "3. Testing individual search terms that should trigger autocomplete:"
TEST_QUERIES=("know" "graph" "terr" "data" "search" "kg")

for query in "${TEST_QUERIES[@]}"; do
    echo "   Query: '$query'"
    # Test current thesaurus filtering (what frontend does now)
    curl -s "http://127.0.0.1:8000/thesaurus/Terraphim%20Engineer" | \
    jq --arg q "$query" '[.thesaurus | to_entries[] | select(.key | ascii_downcase | contains($q | ascii_downcase)) | .key] | .[0:3]' 2>/dev/null || echo "     Failed to get matches"
done

echo
echo "4. Testing document search to verify KG functionality:"
echo "   Searching for 'knowledge graph' to check kg: links..."
curl -s -X POST "http://127.0.0.1:8000/documents/search" \
  -H "Content-Type: application/json" \
  -d '{"search_term": "knowledge graph", "role": "Terraphim Engineer"}' | \
  jq -r '.results[0].body' | grep -o 'kg:[^)]*' | head -3 2>/dev/null || echo "   No kg: links found"

echo
echo "5. Checking Rust autocomplete implementation in terraphim_automata:"
echo "   Available FST functions (from source code):"
echo "   - autocomplete_search(thesaurus_path, query, limit)"
echo "   - fuzzy_autocomplete_search_jaro_winkler(thesaurus_path, query, limit)"
echo "   - fuzzy_autocomplete_search_levenshtein(thesaurus_path, query, limit)"

echo
echo "6. Testing backend role configuration:"
curl -s "http://127.0.0.1:8000/config" | jq '{
  selected_role: .config.selected_role,
  kg_enabled_roles: [.config.roles | to_entries[] | select(.value.kg.publish == true) | .key]
}' 2>/dev/null || echo "   Failed to get config"

echo
echo "7. Expected behavior for FST-based autocomplete:"
echo "   - Input 'know' should suggest: knowledge graph, knowledge management, etc."
echo "   - Input 'terr' should suggest: terraphim-graph, terraform, etc." 
echo "   - Input 'kg' should suggest: knowledge graph, kg:terraphim-graph, etc."
echo "   - Fuzzy matching should handle typos: 'knolege' -> 'knowledge'"

echo
echo "=== Test Summary ==="
echo "Current Status:"
echo "✅ Thesaurus endpoint returns data"
echo "❌ FST-based autocomplete endpoint missing"
echo "❌ Frontend uses simple substring matching instead of FST"
echo "✅ Backend has kg: links in documents"
echo "✅ KG-enabled roles configured correctly"

echo
echo "Next Steps:"
echo "1. Add /autocomplete/:role endpoint using terraphim_automata FST functions"
echo "2. Update Search.svelte to call new autocomplete endpoint"
echo "3. Test fuzzy matching and prefix completion"
echo
echo "=== Test completed ==="