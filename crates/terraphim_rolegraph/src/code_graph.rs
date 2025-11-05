// Code symbol knowledge graph extension
//
// Extends RoleGraph to support code symbols alongside concepts.
// Maintains backward compatibility by using separate storage.

use ahash::{AHashMap, AHashSet};
use terraphim_types::{CodeReference, CodeSymbol, SymbolKind};

/// Extension to RoleGraph for code symbol storage
#[derive(Debug, Clone)]
pub struct CodeGraph {
    /// Storage for code symbols by ID
    code_symbols: AHashMap<u64, CodeSymbol>,
    /// Storage for code references (dependencies)
    code_references: Vec<CodeReference>,
    /// Index: file_path -> symbol IDs
    symbols_by_file: AHashMap<String, Vec<u64>>,
    /// Index: symbol_name -> symbol IDs (for lookup)
    symbols_by_name: AHashMap<String, Vec<u64>>,
    /// Index: symbol_kind -> symbol IDs
    symbols_by_kind: AHashMap<SymbolKind, Vec<u64>>,
}

impl CodeGraph {
    /// Create a new empty code graph
    pub fn new() -> Self {
        Self {
            code_symbols: AHashMap::new(),
            code_references: Vec::new(),
            symbols_by_file: AHashMap::new(),
            symbols_by_name: AHashMap::new(),
            symbols_by_kind: AHashMap::new(),
        }
    }

    /// Add a code symbol to the graph
    pub fn add_symbol(&mut self, symbol: CodeSymbol) {
        let id = symbol.id;
        let file = symbol.file_path.clone();
        let name = symbol.name.clone();
        let kind = symbol.kind;

        // Store symbol
        self.code_symbols.insert(id, symbol);

        // Update file index
        self.symbols_by_file.entry(file).or_default().push(id);

        // Update name index
        self.symbols_by_name.entry(name).or_default().push(id);

        // Update kind index
        self.symbols_by_kind.entry(kind).or_default().push(id);
    }

    /// Add a reference between code symbols
    pub fn add_reference(&mut self, reference: CodeReference) {
        self.code_references.push(reference);
    }

    /// Get symbol by ID
    pub fn get_symbol(&self, id: u64) -> Option<&CodeSymbol> {
        self.code_symbols.get(&id)
    }

    /// Get all symbols in a file
    pub fn get_symbols_in_file(&self, file_path: &str) -> Vec<&CodeSymbol> {
        self.symbols_by_file
            .get(file_path)
            .map(|ids| {
                ids.iter()
                    .filter_map(|id| self.code_symbols.get(id))
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Find symbols by name
    pub fn find_symbols_by_name(&self, name: &str) -> Vec<&CodeSymbol> {
        self.symbols_by_name
            .get(name)
            .map(|ids| {
                ids.iter()
                    .filter_map(|id| self.code_symbols.get(id))
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Get all symbols of a specific kind
    pub fn get_symbols_by_kind(&self, kind: SymbolKind) -> Vec<&CodeSymbol> {
        self.symbols_by_kind
            .get(&kind)
            .map(|ids| {
                ids.iter()
                    .filter_map(|id| self.code_symbols.get(id))
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Get all references from a symbol
    pub fn get_references_from(&self, symbol_id: u64) -> Vec<&CodeReference> {
        self.code_references
            .iter()
            .filter(|r| r.from_symbol_id == symbol_id)
            .collect()
    }

    /// Get all references to a symbol
    pub fn get_references_to(&self, symbol_id: u64) -> Vec<&CodeReference> {
        self.code_references
            .iter()
            .filter(|r| r.to_symbol_id == symbol_id)
            .collect()
    }

    /// Build dependency graph for a set of files
    ///
    /// Returns symbols ranked by importance (PageRank-style)
    pub fn rank_symbols_by_relevance(&self, files: &[String]) -> Vec<(u64, f64)> {
        // Get all symbols in the files
        let mut symbol_ids = AHashSet::new();
        for file in files {
            if let Some(ids) = self.symbols_by_file.get(file) {
                symbol_ids.extend(ids.iter().copied());
            }
        }

        if symbol_ids.is_empty() {
            return Vec::new();
        }

        // Build adjacency for PageRank
        let mut out_links: AHashMap<u64, AHashSet<u64>> = AHashMap::new();
        let mut in_links: AHashMap<u64, AHashSet<u64>> = AHashMap::new();

        for reference in &self.code_references {
            if symbol_ids.contains(&reference.from_symbol_id)
                && symbol_ids.contains(&reference.to_symbol_id)
            {
                out_links
                    .entry(reference.from_symbol_id)
                    .or_default()
                    .insert(reference.to_symbol_id);

                in_links
                    .entry(reference.to_symbol_id)
                    .or_default()
                    .insert(reference.from_symbol_id);
            }
        }

        // Simple PageRank algorithm
        let damping = 0.85;
        let iterations = 10;
        let num_symbols = symbol_ids.len() as f64;

        let mut ranks: AHashMap<u64, f64> = symbol_ids
            .iter()
            .map(|&id| (id, 1.0 / num_symbols))
            .collect();

        for _ in 0..iterations {
            let mut new_ranks = AHashMap::new();

            for &symbol_id in &symbol_ids {
                let mut rank = (1.0 - damping) / num_symbols;

                // Add contributions from incoming links
                if let Some(incoming) = in_links.get(&symbol_id) {
                    for &source_id in incoming {
                        if let Some(&source_rank) = ranks.get(&source_id) {
                            let out_count = out_links.get(&source_id).map(|s| s.len()).unwrap_or(1);
                            rank += damping * source_rank / out_count as f64;
                        }
                    }
                }

                new_ranks.insert(symbol_id, rank);
            }

            ranks = new_ranks;
        }

        // Convert to sorted vec
        let mut ranked: Vec<(u64, f64)> = ranks.into_iter().collect();
        ranked.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());

        ranked
    }

    /// Get statistics about the code graph
    pub fn get_code_stats(&self) -> CodeGraphStats {
        CodeGraphStats {
            symbol_count: self.code_symbols.len(),
            reference_count: self.code_references.len(),
            file_count: self.symbols_by_file.len(),
            functions: self
                .symbols_by_kind
                .get(&SymbolKind::Function)
                .map(|v| v.len())
                .unwrap_or(0),
            classes: self
                .symbols_by_kind
                .get(&SymbolKind::Class)
                .map(|v| v.len())
                .unwrap_or(0),
            methods: self
                .symbols_by_kind
                .get(&SymbolKind::Method)
                .map(|v| v.len())
                .unwrap_or(0),
        }
    }

    /// Clear all code symbols (keeps concept graph intact)
    pub fn clear_code_symbols(&mut self) {
        self.code_symbols.clear();
        self.code_references.clear();
        self.symbols_by_file.clear();
        self.symbols_by_name.clear();
        self.symbols_by_kind.clear();
    }
}

impl Default for CodeGraph {
    fn default() -> Self {
        Self::new()
    }
}

/// Statistics about the code graph
#[derive(Debug, Clone)]
pub struct CodeGraphStats {
    pub symbol_count: usize,
    pub reference_count: usize,
    pub file_count: usize,
    pub functions: usize,
    pub classes: usize,
    pub methods: usize,
}

#[cfg(test)]
mod tests {
    use super::*;
    use terraphim_types::ReferenceType;

    #[test]
    fn test_code_graph_creation() {
        let graph = CodeGraph::new();
        assert_eq!(graph.code_symbols.len(), 0);
        assert_eq!(graph.code_references.len(), 0);
    }

    #[test]
    fn test_add_symbol() {
        let mut graph = CodeGraph::new();

        let symbol = CodeSymbol::new(
            1,
            "calculate".to_string(),
            SymbolKind::Function,
            "src/math.rs".to_string(),
            10,
            "rust".to_string(),
        );

        graph.add_symbol(symbol.clone());

        assert_eq!(graph.code_symbols.len(), 1);
        assert_eq!(graph.get_symbol(1), Some(&symbol));
    }

    #[test]
    fn test_get_symbols_in_file() {
        let mut graph = CodeGraph::new();

        let symbol1 = CodeSymbol::new(
            1,
            "func1".to_string(),
            SymbolKind::Function,
            "src/main.rs".to_string(),
            10,
            "rust".to_string(),
        );

        let symbol2 = CodeSymbol::new(
            2,
            "func2".to_string(),
            SymbolKind::Function,
            "src/main.rs".to_string(),
            20,
            "rust".to_string(),
        );

        graph.add_symbol(symbol1);
        graph.add_symbol(symbol2);

        let symbols = graph.get_symbols_in_file("src/main.rs");
        assert_eq!(symbols.len(), 2);
    }

    #[test]
    fn test_find_symbols_by_name() {
        let mut graph = CodeGraph::new();

        let symbol = CodeSymbol::new(
            1,
            "calculate".to_string(),
            SymbolKind::Function,
            "src/math.rs".to_string(),
            10,
            "rust".to_string(),
        );

        graph.add_symbol(symbol.clone());

        let found = graph.find_symbols_by_name("calculate");
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].name, "calculate");
    }

    #[test]
    fn test_add_reference() {
        let mut graph = CodeGraph::new();

        let reference = CodeReference::new(1, 2, ReferenceType::Calls);
        graph.add_reference(reference.clone());

        assert_eq!(graph.code_references.len(), 1);
        assert_eq!(graph.code_references[0], reference);
    }

    #[test]
    fn test_get_references_from() {
        let mut graph = CodeGraph::new();

        graph.add_reference(CodeReference::new(1, 2, ReferenceType::Calls));
        graph.add_reference(CodeReference::new(1, 3, ReferenceType::Calls));
        graph.add_reference(CodeReference::new(2, 3, ReferenceType::Calls));

        let refs = graph.get_references_from(1);
        assert_eq!(refs.len(), 2);
    }

    #[test]
    fn test_get_references_to() {
        let mut graph = CodeGraph::new();

        graph.add_reference(CodeReference::new(1, 3, ReferenceType::Calls));
        graph.add_reference(CodeReference::new(2, 3, ReferenceType::Calls));

        let refs = graph.get_references_to(3);
        assert_eq!(refs.len(), 2);
    }

    #[test]
    fn test_rank_symbols_by_relevance() {
        let mut graph = CodeGraph::new();

        // Add symbols
        let s1 = CodeSymbol::new(
            1,
            "main".to_string(),
            SymbolKind::Function,
            "src/main.rs".to_string(),
            1,
            "rust".to_string(),
        );
        let s2 = CodeSymbol::new(
            2,
            "helper".to_string(),
            SymbolKind::Function,
            "src/main.rs".to_string(),
            10,
            "rust".to_string(),
        );
        let s3 = CodeSymbol::new(
            3,
            "util".to_string(),
            SymbolKind::Function,
            "src/main.rs".to_string(),
            20,
            "rust".to_string(),
        );

        graph.add_symbol(s1);
        graph.add_symbol(s2);
        graph.add_symbol(s3);

        // Add references: main calls helper, main calls util, helper calls util
        graph.add_reference(CodeReference::new(1, 2, ReferenceType::Calls));
        graph.add_reference(CodeReference::new(1, 3, ReferenceType::Calls));
        graph.add_reference(CodeReference::new(2, 3, ReferenceType::Calls));

        // Rank by relevance (util should rank highest - called by both)
        let ranked = graph.rank_symbols_by_relevance(&["src/main.rs".to_string()]);

        assert_eq!(ranked.len(), 3);
        // util (id=3) should have highest rank due to most incoming references
        assert_eq!(ranked[0].0, 3);
    }

    #[test]
    fn test_get_symbols_by_kind() {
        let mut graph = CodeGraph::new();

        graph.add_symbol(CodeSymbol::new(
            1,
            "MyClass".to_string(),
            SymbolKind::Class,
            "src/types.rs".to_string(),
            10,
            "rust".to_string(),
        ));

        graph.add_symbol(CodeSymbol::new(
            2,
            "calculate".to_string(),
            SymbolKind::Function,
            "src/math.rs".to_string(),
            20,
            "rust".to_string(),
        ));

        let classes = graph.get_symbols_by_kind(SymbolKind::Class);
        assert_eq!(classes.len(), 1);
        assert_eq!(classes[0].name, "MyClass");

        let functions = graph.get_symbols_by_kind(SymbolKind::Function);
        assert_eq!(functions.len(), 1);
        assert_eq!(functions[0].name, "calculate");
    }

    #[test]
    fn test_code_graph_stats() {
        let mut graph = CodeGraph::new();

        graph.add_symbol(CodeSymbol::new(
            1,
            "func1".to_string(),
            SymbolKind::Function,
            "src/main.rs".to_string(),
            10,
            "rust".to_string(),
        ));

        graph.add_symbol(CodeSymbol::new(
            2,
            "Class1".to_string(),
            SymbolKind::Class,
            "src/types.rs".to_string(),
            20,
            "rust".to_string(),
        ));

        graph.add_reference(CodeReference::new(1, 2, ReferenceType::Uses));

        let stats = graph.get_code_stats();
        assert_eq!(stats.symbol_count, 2);
        assert_eq!(stats.reference_count, 1);
        assert_eq!(stats.file_count, 2);
        assert_eq!(stats.functions, 1);
        assert_eq!(stats.classes, 1);
    }

    #[test]
    fn test_clear_code_symbols() {
        let mut graph = CodeGraph::new();

        graph.add_symbol(CodeSymbol::new(
            1,
            "test".to_string(),
            SymbolKind::Function,
            "test.rs".to_string(),
            1,
            "rust".to_string(),
        ));

        graph.add_reference(CodeReference::new(1, 2, ReferenceType::Calls));

        assert_eq!(graph.code_symbols.len(), 1);
        assert_eq!(graph.code_references.len(), 1);

        graph.clear_code_symbols();

        assert_eq!(graph.code_symbols.len(), 0);
        assert_eq!(graph.code_references.len(), 0);
    }
}
