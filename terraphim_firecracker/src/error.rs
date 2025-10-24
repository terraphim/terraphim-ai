use thiserror::Error;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Error, Debug)]
#[allow(dead_code)]
pub enum Error {
    #[error("VM configuration error: {0}")]
    VmConfig(String),

    #[error("VM state error: {0}")]
    VmState(String),

    #[error("Firecracker error: {0}")]
    Firecracker(String),

    #[error("Network error: {0}")]
    Network(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("Configuration error: {0}")]
    Config(String),

    #[error("Timeout error: {0}")]
    Timeout(String),

    #[error("Allocation error: {0}")]
    Allocation(String),

    #[error("No available VMs")]
    NoAvailableVms,

    #[error("Insufficient resources: {0}")]
    InsufficientResources(String),

    #[error("Performance optimization error: {0}")]
    PerformanceOptimization(String),

    #[error("Storage error: {0}")]
    Storage(String),

    #[error("Pool error: {0}")]
    Pool(String),

    #[error("Maintenance error: {0}")]
    Maintenance(String),

    #[error("Prewarming error: {0}")]
    Prewarming(String),

    #[error("Invalid argument: {0}")]
    InvalidArgument(String),

    #[error("Not implemented: {0}")]
    NotImplemented(String),

    #[error("Internal error: {0}")]
    Internal(String),
}
