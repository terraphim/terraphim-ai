use dioxus::prelude::*;
use dioxus_router::prelude::*;
use crate::app::Route;
use crate::components::navigation::RoleSelector;

#[component]
pub fn Navbar() -> Element {
    let nav = navigator();

    rsx! {
        nav { class: "navbar is-light",
            div { class: "navbar-brand",
                a { class: "navbar-item",
                    onclick: move |_| nav.push(Route::SearchPage {}),
                    img { src: "assets/terraphim_gray.png", alt: "Terraphim", width: "32", height: "32" }
                }
            }

            div { class: "navbar-menu",
                div { class: "navbar-start",
                    Link { to: Route::SearchPage {}, class: "navbar-item",
                        span { class: "icon", i { class: "fas fa-search" } }
                        span { "Search" }
                    }
                    Link { to: Route::ChatPage {}, class: "navbar-item",
                        span { class: "icon", i { class: "fas fa-comments" } }
                        span { "Chat" }
                    }
                }

                div { class: "navbar-end",
                    div { class: "navbar-item",
                        RoleSelector {}
                    }
                }
            }
        }
    }
}
