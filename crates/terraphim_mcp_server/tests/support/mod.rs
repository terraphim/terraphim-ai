/// Resolve the path to the terraphim_mcp_server binary.
///
/// Priority:
/// 1. `TERRAPHIM_MCP_SERVER_BIN` environment variable (set by CI/build-runner)
/// 2. `../../target/debug/terraphim_mcp_server` relative to current dir
/// 3. `../../target/release/terraphim_mcp_server` relative to current dir
pub fn mcp_server_binary() -> anyhow::Result<std::path::PathBuf> {
    if let Ok(bin) = std::env::var("TERRAPHIM_MCP_SERVER_BIN") {
        let path = std::path::PathBuf::from(bin);
        if path.exists() {
            return Ok(path);
        }
    }

    let crate_dir = std::env::current_dir()?;
    let candidates = [
        crate_dir
            .parent()
            .and_then(|p| p.parent())
            .map(|w| w.join("target").join("debug").join("terraphim_mcp_server")),
        crate_dir.parent().and_then(|p| p.parent()).map(|w| {
            w.join("target")
                .join("release")
                .join("terraphim_mcp_server")
        }),
    ];

    for path in candidates.into_iter().flatten() {
        if path.exists() {
            return Ok(path);
        }
    }

    anyhow::bail!(
        "terraphim_mcp_server binary not found. Set TERRAPHIM_MCP_SERVER_BIN or run: cargo build -p terraphim_mcp_server"
    )
}
