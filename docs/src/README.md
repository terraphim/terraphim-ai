# Terraphim documentation

This folder is a source for Terraphim documentation as well as demonstration for Terraphim Engineer role:

[kg](./kg) folder is an example of personal knowledge graph used for testing, fixtures, and domain-specific terminology including bug reporting and issue tracking

Example configuration for this KG:

```
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
      "path": "docs/src/kg",
      "public": true,
      "publish": true
    }
  },
  "haystacks": [
    {
      "path": "docs/src/",
      "service": "Ripgrep"
    }
  ]
},
```

The relative paths resolve from the working directory, so run the server from the repository root or replace them with absolute paths (`~` and `$HOME` are not expanded in these fields).

- [ ]

- to do list
- todo list

- [ ] My todo

- \[ \]
