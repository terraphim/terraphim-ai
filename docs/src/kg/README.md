This folder is an example of personal knowledge graph used for testing and fixtures

Example configuration for this KG:

```json
"Terraphim Engineer": {
  "shortname": "terraphim-engineer",
  "name": "Terraphim Engineer",
  "relevance_function": "terraphim-graph",
  "theme": "superhero",
  "kg": {
    "automata_path": {
      "Local": "data/term_to_id_test.json"
    },
    "knowledge_graph_local": {
      "input_type": "markdown",
      "path": "/Users/alex/projects/terraphim/terraphim-ai/docs/src/kg",
      "public": true,
      "publish": true
    }
  },
  "haystacks": [
    {
      "path": "/Users/alex/projects/terraphim/terraphim-ai/docs/src/",
      "service": "Ripgrep"
    }
  ]
},
```