use terraphim_settings;

/// Errors arising from persistence layer operations.
#[derive(thiserror::Error, Debug)]
pub enum Error {
    /// A named storage profile could not be loaded or parsed.
    #[error("Error with profile: {0}")]
    Profile(String),

    /// An OpenDAL storage operation failed.
    #[error("OpenDal error: {0}")]
    OpenDal(Box<opendal::Error>),

    /// JSON serialisation or deserialisation failed.
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    /// No storage operator is configured or available.
    #[error("No operator found")]
    NoOperator,

    /// The requested key or record does not exist in the store.
    #[error("Not found: {0}")]
    NotFound(String),

    /// Loading device settings failed.
    #[error("Settings error: {0}")]
    Settings(#[from] terraphim_settings::Error),

    /// An I/O operation on the filesystem failed.
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    /// Serialisation to or from a non-JSON format failed.
    #[error("Serialization error: {0}")]
    Serde(String),
}

impl From<opendal::Error> for Error {
    fn from(error: opendal::Error) -> Self {
        Error::OpenDal(Box::new(error))
    }
}

/// Convenience alias for `Result<T, Error>` used throughout this crate.
pub type Result<T> = std::result::Result<T, Error>;
