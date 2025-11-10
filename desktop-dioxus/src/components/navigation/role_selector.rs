use dioxus::prelude::*;
use crate::state::ConfigState;

#[component]
pub fn RoleSelector() -> Element {
    let mut config_state = use_context::<ConfigState>();
    let selected_role = config_state.selected_role();
    let roles = config_state.available_roles();

    rsx! {
        div { class: "select",
            select {
                value: "{selected_role}",
                onchange: move |evt| {
                    let role_name = evt.value();
                    config_state.select_role(role_name);
                    tracing::info!("Role changed to: {}", role_name);
                },
                
                for role_name in roles {
                    option {
                        value: "{role_name}",
                        selected: role_name == selected_role,
                        "{role_name}"
                    }
                }
            }
        }
    }
}
