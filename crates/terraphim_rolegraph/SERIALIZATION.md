# RoleGraph Serialization Support

This document describes the serialization capabilities added to the `terraphim_rolegraph` crate for Node.js NAPI bindings.

## Overview

The serialization support enables RoleGraph instances to be converted to/from JSON format, making them compatible with Node.js environments and allowing for persistent storage and network transmission.

## Key Components

### 1. SerializableRoleGraph
A dedicated struct that represents a JSON-serializable version of RoleGraph:
- Contains all RoleGraph data except non-serializable Aho-Corasick automata
- Includes all necessary data to rebuild the automata from thesaurus
- Provides `to_json()`, `to_json_pretty()`, and `from_json()` methods

### 2. Enhanced RoleGraph
Extended with serialization helper methods:
- `to_serializable()` - Convert to SerializableRoleGraph
- `from_serializable()` - Create from SerializableRoleGraph with rebuilt automata
- `rebuild_automata()` - Manually rebuild Aho-Corasick automata from thesaurus

### 3. Enhanced RoleGraphSync
Added async serialization methods that handle locking internally:
- `to_json()` - Serialize to JSON string
- `to_json_pretty()` - Serialize to pretty JSON string
- `from_json()` - Deserialize from JSON string
- `to_serializable()` - Get serializable representation

### 4. GraphStats
Now fully serializable with serde derives for debugging and monitoring.

## Usage Examples

### Basic RoleGraph Serialization
```rust
use terraphim_rolegraph::{RoleGraph, SerializableRoleGraph};

// Create RoleGraph
let rolegraph = RoleGraph::new(role.into(), thesaurus).await?;

// Convert to serializable representation
let serializable = rolegraph.to_serializable();

// Serialize to JSON
let json = serializable.to_json()?;

// Deserialize from JSON
let deserialized = SerializableRoleGraph::from_json(&json)?;

// Recreate RoleGraph with rebuilt automata
let restored = RoleGraph::from_serializable(deserialized).await?;
```

### RoleGraphSync Serialization
```rust
use terraphim_rolegraph::RoleGraphSync;

let rolegraph_sync = RoleGraphSync::from(rolegraph);

// Serialize to JSON (handles locking internally)
let json = rolegraph_sync.to_json().await?;

// Deserialize back to RoleGraphSync
let restored = RoleGraphSync::from_json(&json).await?;
```

### Graph Statistics
```rust
let stats = rolegraph.get_graph_stats();
let json = serde_json::to_string(&stats)?;
let restored: GraphStats = serde_json::from_str(&json)?;
```

## Important Notes

1. **Aho-Corasick Rebuilding**: The automata is not serialized directly but rebuilt from the thesaurus during deserialization. This ensures compatibility and reduces serialized size.

2. **Performance Considerations**: Large graphs may have significant serialization overhead due to cloning operations.

3. **Thread Safety**: RoleGraphSync serialization methods automatically handle async locking.

4. **Error Handling**: All serialization methods return proper Result types with detailed error information.

## Files Modified

- `src/lib.rs`: Added serialization support, helper methods, and comprehensive tests
- `serialization_example.rs`: Complete example demonstrating usage
- Tests: Added 4 comprehensive serialization tests covering various scenarios

## Testing

The implementation includes comprehensive tests:
- Basic RoleGraph serialization/deserialization
- RoleGraphSync async serialization
- GraphStats serialization
- Edge cases (empty graphs, single documents)

Run tests with:
```bash
cargo test serialization --lib -- --nocapture
```

## Node.js Integration

This serialization support enables seamless integration with Node.js NAPI bindings, allowing RoleGraph instances to be:
- Passed between Rust and Node.js boundaries
- Stored in JSON files or databases
- Transmitted over network protocols
- Persisted across application restarts