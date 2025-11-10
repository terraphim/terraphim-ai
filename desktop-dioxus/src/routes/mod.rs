pub mod search;
pub mod chat;
pub mod config;

pub use search::SearchRoute;
pub use chat::ChatRoute;
pub use config::{ConfigWizardRoute, ConfigJsonRoute};
