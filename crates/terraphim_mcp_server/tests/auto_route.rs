//! T10: MCP routing line surfaced in CallToolResult.contents.
//!
//! Constructs `McpService` directly with a hand-built `ConfigState` (one role
//! with a small KG) and calls `search` twice:
//!   - With `role` argument set: response should NOT begin with `[auto-route]`.
//!   - With `role` unset: response's first text content should start with
//!     `[auto-route]` and subsequent contents should be unchanged.

use ahash::AHashMap;
use std::sync::Arc;
use terraphim_config::{Config, ConfigId, ConfigState, Role};
use terraphim_mcp_server::McpService;
use terraphim_rolegraph::{RoleGraph, RoleGraphSync};
use terraphim_types::{
    NormalizedTerm, NormalizedTermValue, RelevanceFunction, RoleName, Thesaurus,
};
use tokio::sync::Mutex;

fn build_thesaurus(name: &str, terms: &[(&str, u64, &str)]) -> Thesaurus {
    let mut t = Thesaurus::new(name.to_string());
    for (synonym, id, concept) in terms {
        t.insert(
            NormalizedTermValue::from(*synonym),
            NormalizedTerm::new(*id, NormalizedTermValue::from(*concept)),
        );
    }
    t
}

async fn fixture() -> Arc<ConfigState> {
    let role_name = RoleName::new("Test Engineer");

    // Build a tiny rolegraph with one matched term.
    let thesaurus = build_thesaurus("test", &[("rfp", 1, "rfp")]);
    let rg = RoleGraph::new(role_name.clone(), thesaurus).await.unwrap();
    let rg_sync = RoleGraphSync::from(rg);

    // Use TitleScorer so service.search() does not require a KG configured on
    // disk; auto-routing still works because it only inspects the in-memory
    // rolegraphs we put in `state.roles`.
    let mut role = Role::new(role_name.clone());
    role.relevance_function = RelevanceFunction::TitleScorer;

    let mut roles_map = AHashMap::new();
    roles_map.insert(role_name.clone(), role);
    let mut rg_map = AHashMap::new();
    rg_map.insert(role_name.clone(), rg_sync);

    let config = Config {
        id: ConfigId::Embedded,
        global_shortcut: "Ctrl+X".to_string(),
        roles: roles_map,
        default_role: role_name.clone(),
        selected_role: role_name.clone(),
    };

    Arc::new(ConfigState {
        config: Arc::new(Mutex::new(config)),
        roles: rg_map,
    })
}

fn first_text(result: &rmcp::model::CallToolResult) -> Option<String> {
    use rmcp::model::RawContent;
    result.content.first().and_then(|c| match &c.raw {
        RawContent::Text(t) => Some(t.text.clone()),
        _ => None,
    })
}

#[tokio::test]
async fn t10_auto_route_line_prepended_when_role_omitted() {
    let state = fixture().await;
    let service = McpService::new(state);

    let with_role = service
        .search(
            "rfp".to_string(),
            Some("Test Engineer".to_string()),
            Some(5),
            None,
        )
        .await
        .expect("search with explicit role should not error");

    let without_role = service
        .search("rfp".to_string(), None, Some(5), None)
        .await
        .expect("search without role should not error");

    let auto_text = first_text(&without_role).expect("expected text content first");
    assert!(
        auto_text.starts_with("[auto-route]"),
        "expected first content to start with [auto-route]; got: {auto_text}"
    );

    // Explicit-role response should NOT lead with [auto-route].
    let explicit_first = first_text(&with_role).expect("expected text content first");
    assert!(
        !explicit_first.starts_with("[auto-route]"),
        "explicit role should not emit [auto-route]; got: {explicit_first}"
    );

    // Auto-routed response should have exactly one extra leading text item
    // compared to the explicit-role response.
    assert_eq!(
        without_role.content.len(),
        with_role.content.len() + 1,
        "auto-routed response should have one extra (leading) content item"
    );
}
