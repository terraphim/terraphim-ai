use std::path::PathBuf;

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|p| p.parent())
        .map(|p| p.to_path_buf())
        .expect("CARGO_MANIFEST_DIR should be crates/terraphim_agent")
}

pub fn agent_binary() -> String {
    workspace_root()
        .join("target/debug/terraphim-agent")
        .to_string_lossy()
        .to_string()
}

pub fn server_binary() -> String {
    workspace_root()
        .join("target/debug/terraphim_server")
        .to_string_lossy()
        .to_string()
}
