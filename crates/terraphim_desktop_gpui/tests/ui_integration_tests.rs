#![cfg(feature = "legacy-components")]

// Legacy component integration tests are gated behind the `legacy-components`
// feature to keep the default crate test surface aligned with the GPUI views.

use std::sync::Arc;
use std::time::Duration;

use ahash::AHashMap;
use gpui::*;
use terraphim_types::{ContextItem, ContextType, Document, RoleName};

use terraphim_desktop_gpui::{
    components::{
        AddDocumentModal, ComponentConfig, ContextComponent, ContextItemComponent,
        EnhancedChatComponent, PerformanceTracker, SearchContextBridge, ServiceRegistry,
    },
    views::search::{SearchComponent, SearchComponentConfig},
};

#[tokio::test]
async fn test_legacy_components_placeholder() {
    // Placeholder to keep the file compiling under the feature.
    assert!(true);
}
