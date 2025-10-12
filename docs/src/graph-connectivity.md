# Graph Connectivity Check for Matched Terms

Goal: Given an input text, detect terms via automata and verify whether there exists a single path in the role graph that connects all matched terms.

## Algorithm
- Extract matched node IDs from text using `RoleGraph::find_matching_node_ids` (Aho-Corasick based)
- Build an undirected adjacency map from `nodes.connected_with` and `edges`
- For small target sets (k ≤ 8), run DFS/backtracking to determine whether a path exists that visits all target nodes at least once
- Trivial cases: 0 or 1 matched node → true

## API
Implemented in `terraphim_rolegraph`:
- `RoleGraph::is_all_terms_connected_by_path(text: &str) -> bool`

## Usage
```rust
use terraphim_rolegraph::RoleGraph;
// assume rolegraph constructed
let text = "Life cycle concepts ... Paradigm Map ... project planning";
let connected = rolegraph.is_all_terms_connected_by_path(text);
```

## Tests
- Connectivity positive case with frequent terms
- Smoke negative case (non-deterministic based on fixtures)

Run:
```bash
cargo test -p terraphim_rolegraph
```

## Benchmarks
- Added Criterion bench: `is_all_terms_connected_by_path`

Run:
```bash
cargo bench -p terraphim_rolegraph --bench throughput -- is_all_terms_connected_by_path
```
