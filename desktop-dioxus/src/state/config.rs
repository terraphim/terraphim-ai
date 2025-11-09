use dioxus::prelude::*;
use terraphim_config::Config;
use terraphim_types::RoleName;

/// Global configuration state
#[derive(Clone)]
pub struct ConfigState {
    config: Signal<Config>,
    selected_role: Signal<String>,
    loading: Signal<bool>,
}

impl ConfigState {
    pub fn new() -> Self {
        let initial_config = Self::load_config_blocking();
        let selected_role_name = initial_config.selected_role.original.clone();

        Self {
            config: Signal::new(initial_config),
            selected_role: Signal::new(selected_role_name),
            loading: Signal::new(false),
        }
    }

    fn load_config_blocking() -> Config {
        use terraphim_config::{ConfigBuilder, ConfigId};
        use terraphim_persistence::Persistable;

        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            match ConfigBuilder::new_with_id(ConfigId::Desktop).build() {
                Ok(mut builder_config) => match builder_config.load().await {
                    Ok(loaded) => loaded,
                    Err(_) => ConfigBuilder::new().build_default_desktop().build().expect("Failed to build default config")
                },
                Err(_) => ConfigBuilder::new().build_default_desktop().build().expect("Failed to build default config")
            }
        })
    }

    pub fn get_config(&self) -> Config {
        self.config.read().clone()
    }

    pub fn selected_role(&self) -> String {
        self.selected_role.read().clone()
    }

    pub fn available_roles(&self) -> Vec<String> {
        self.config.read().roles.keys().map(|k| k.original.clone()).collect()
    }

    pub fn select_role(&mut self, role_name: String) {
        let mut config = self.config.write();
        config.selected_role = RoleName {
            original: role_name.clone(),
            lowercase: role_name.to_lowercase(),
        };
        drop(config);

        self.selected_role.set(role_name);

        let config_clone = self.config.read().clone();
        spawn(async move {
            use terraphim_persistence::Persistable;
            if let Err(e) = config_clone.save().await {
                tracing::error!("Failed to save config: {:?}", e);
            }

            // Update tray menu to reflect new role
            if let Err(e) = crate::update_tray_menu(&config_clone).await {
                tracing::error!("Failed to update tray menu: {:?}", e);
            }
        });
    }

    pub fn is_loading(&self) -> bool {
        *self.loading.read()
    }

    pub fn update_config(&mut self, new_config: Config) {
        let new_role = new_config.selected_role.original.clone();
        self.config.set(new_config.clone());
        self.selected_role.set(new_role);

        spawn(async move {
            use terraphim_persistence::Persistable;
            if let Err(e) = new_config.save().await {
                tracing::error!("Failed to save config: {:?}", e);
            }
        });
    }
}
