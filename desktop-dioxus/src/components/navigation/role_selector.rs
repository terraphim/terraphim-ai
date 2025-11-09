use dioxus::prelude::*;
use crate::state::ConfigState;

#[component]
pub fn RoleSelector() -> Element {
    let config_state = use_context::<ConfigState>();
    let selected_role = config_state.selected_role();

    let roles = use_resource(move || {
        let config_state = config_state.clone();
        async move {
            let config = config_state.get_config().await;
            config.roles.keys().map(|k| k.original.clone()).collect::<Vec<_>>()
        }
    });

    rsx! {
        div { class: "select",
            select {
                value: "{selected_role}",
                onchange: move |evt| {
                    let role_name = evt.value().clone();
                    let config_state = config_state.clone();
                    spawn(async move {
                        if let Err(e) = config_state.select_role(role_name).await {
                            tracing::error!("Failed to select role: {:?}", e);
                        }
                    });
                },

                match &*roles.read() {
                    Some(role_list) => rsx! {
                        for role_name in role_list {
                            option { value: "{role_name}", "{role_name}" }
                        }
                    },
                    None => rsx! {
                        option { "Loading roles..." }
                    }
                }
            }
        }
    }
}
