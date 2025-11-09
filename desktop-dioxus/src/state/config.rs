use dioxus::prelude::*;
use terraphim_config::{Config, ConfigState as CoreConfigState};
use std::sync::Arc;
use tokio::sync::Mutex as TokioMutex;

#[derive(Clone)]
pub struct ConfigState {
    inner: Arc<TokioMutex<CoreConfigState>>,
    // Signal for reactive updates
    selected_role: Signal<String>,
}

impl ConfigState {
    pub fn new() -> Self {
        // Load config blocking (will be async in actual implementation)
        let config = Self::load_config_blocking();
        let selected_role_name = config.selected_role.original.clone();

        let inner = Arc::new(TokioMutex::new(
            CoreConfigState::new(&mut config.clone())
                .expect("Failed to create ConfigState")
        ));

        Self {
            inner,
            selected_role: Signal::new(selected_role_name),
        }
    }

    fn load_config_blocking() -> Config {
        tokio::runtime::Runtime::new()
            .unwrap()
            .block_on(async {
                terraphim_config::ConfigBuilder::new()
                    .build_default_desktop()
                    .build()
                    .expect("Failed to build default config")
            })
    }

    pub fn selected_role(&self) -> String {
        self.selected_role.read().clone()
    }

    pub async fn select_role(&self, role_name: String) -> anyhow::Result<()> {
        let mut state = self.inner.lock().await;
        let mut config = state.config.lock().await;
        config.selected_role = role_name.clone().into();
        drop(config);
        drop(state);

        self.selected_role.set(role_name);
        Ok(())
    }

    pub async fn get_config(&self) -> Config {
        let state = self.inner.lock().await;
        state.config.lock().await.clone()
    }

    pub fn inner(&self) -> Arc<TokioMutex<CoreConfigState>> {
        self.inner.clone()
    }
}
