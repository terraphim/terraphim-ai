use terraphim_settings;

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("Error with profile: {0}")]
    Profile(String),

    #[error("OpenDal error: {0}")]
    OpenDal(Box<opendal::Error>),

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("No operator found")]
    NoOperator,

    #[error("Settings error: {0}")]
    Settings(#[from] terraphim_settings::Error),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

impl From<opendal::Error> for Error {
    fn from(error: opendal::Error) -> Self {
        Error::OpenDal(Box::new(error))
    }
}

pub type Result<T> = std::result::Result<T, Error>;
