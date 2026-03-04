use pyo3::prelude::*;

/// Python module for Terraphim RoleGraph
///
/// This module provides knowledge graph operations for AI agents,
/// including document insertion, graph queries, path connectivity,
/// serialization, and statistics.
#[pymodule]
fn terraphim_rolegraph(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add("__version__", env!("CARGO_PKG_VERSION"))?;
    Ok(())
}
