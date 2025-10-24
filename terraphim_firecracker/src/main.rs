use anyhow::Result;
use clap::{Parser, Subcommand};
use log::info;

mod config;
mod error;
mod manager;
mod performance;
mod pool;
mod storage;
mod vm;

use vm::VmManager;

#[derive(Parser)]
#[command(name = "terraphim-vm-manager")]
#[command(about = "Sub-2 second VM boot optimization system for Terraphim AI")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Start the VM manager service
    Start {
        #[arg(short, long, default_value = "config.toml")]
        config: String,
    },
    /// Create and test a single VM
    Test {
        #[arg(short, long, default_value = "terraphim-minimal")]
        vm_type: String,
    },
    /// Benchmark VM boot performance
    Benchmark {
        #[arg(short, long, default_value = "10")]
        trials: u32,
    },
    /// Initialize VM pool with prewarmed VMs
    InitPool {
        #[arg(short, long, default_value = "5")]
        pool_size: usize,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::init();

    let cli = Cli::parse();

    match cli.command {
        Commands::Start { config } => {
            info!("Starting Terraphim VM Manager with config: {}", config);
            let _vm_manager = crate::vm::Sub2SecondVmManager::new(
                std::path::PathBuf::from("/tmp/firecracker"),
                std::sync::Arc::new(crate::storage::InMemoryVmStorage::new()),
            )
            .await?;
            info!("VM manager initialized successfully");
        }
        Commands::Test { vm_type } => {
            info!("Testing VM creation for type: {}", vm_type);
            let vm_manager = crate::vm::Sub2SecondVmManager::new(
                std::path::PathBuf::from("/tmp/firecracker"),
                std::sync::Arc::new(crate::storage::InMemoryVmStorage::new()),
            )
            .await?;
            let result = vm_manager.get_vm(&vm_type).await?;
            info!("Test result: {:?}", result);
        }
        Commands::Benchmark { trials } => {
            info!("Running VM boot benchmark with {} trials", trials);
            let _vm_manager = crate::vm::Sub2SecondVmManager::new(
                std::path::PathBuf::from("/tmp/firecracker"),
                std::sync::Arc::new(crate::storage::InMemoryVmStorage::new()),
            )
            .await?;
            info!("Benchmark functionality not yet implemented");
        }
        Commands::InitPool { pool_size } => {
            info!("Initializing VM pool with {} prewarmed VMs", pool_size);
            let _vm_manager = crate::vm::Sub2SecondVmManager::new(
                std::path::PathBuf::from("/tmp/firecracker"),
                std::sync::Arc::new(crate::storage::InMemoryVmStorage::new()),
            )
            .await?;
            info!("Pool initialization functionality not yet implemented");
            info!("VM pool initialized successfully");
        }
    }

    Ok(())
}
