use anyhow::Result;
use log::{debug, info};
use std::path::PathBuf;
use std::time::Duration;
use tokio::process::Command;

use crate::vm::config::VmConfig;

/// Firecracker client for VM management
pub struct FirecrackerClient {
    socket_dir: PathBuf,
}

impl FirecrackerClient {
    pub fn new(socket_dir: PathBuf) -> Self {
        Self { socket_dir }
    }

    /// Create a VM with Firecracker
    pub async fn create_vm(&self, config: &VmConfig) -> Result<()> {
        info!("Creating Firecracker VM: {}", config.vm_id);

        // Create socket directory
        let socket_path = self.socket_dir.join(format!("{}.sock", config.vm_id));
        tokio::fs::create_dir_all(&socket_path.parent().unwrap()).await?;

        // Start Firecracker process
        let _child = Command::new("firecracker")
            .arg("--api-sock")
            .arg(&socket_path)
            .arg("--id")
            .arg(&config.vm_id)
            .spawn()
            .map_err(|e| anyhow::anyhow!("Failed to start Firecracker: {}", e))?;

        // Wait a moment for the socket to be ready
        tokio::time::sleep(Duration::from_millis(100)).await;

        // Configure VM via API
        self.configure_vm_via_api(&socket_path, config).await?;

        info!("Firecracker VM created: {}", config.vm_id);
        Ok(())
    }

    /// Start a VM
    pub async fn start_vm(&self, vm_id: &str) -> Result<()> {
        info!("Starting Firecracker VM: {}", vm_id);

        let socket_path = self.socket_dir.join(format!("{}.sock", vm_id));

        // Send start command via API
        self.send_api_request(
            &socket_path,
            "actions",
            serde_json::json!({
                "action_type": "InstanceStart"
            }),
        )
        .await?;

        info!("Firecracker VM started: {}", vm_id);
        Ok(())
    }

    /// Stop a VM
    pub async fn stop_vm(&self, vm_id: &str) -> Result<()> {
        info!("Stopping Firecracker VM: {}", vm_id);

        let socket_path = self.socket_dir.join(format!("{}.sock", vm_id));

        // Send stop command via API
        self.send_api_request(
            &socket_path,
            "actions",
            serde_json::json!({
                "action_type": "InstanceStop"
            }),
        )
        .await?;

        info!("Firecracker VM stopped: {}", vm_id);
        Ok(())
    }

    /// Delete a VM
    pub async fn delete_vm(&self, vm_id: &str) -> Result<()> {
        info!("Deleting Firecracker VM: {}", vm_id);

        let socket_path = self.socket_dir.join(format!("{}.sock", vm_id));

        // Clean up socket file
        if socket_path.exists() {
            tokio::fs::remove_file(&socket_path).await?;
        }

        info!("Firecracker VM deleted: {}", vm_id);
        Ok(())
    }

    /// Configure VM via Firecracker API
    async fn configure_vm_via_api(
        &self,
        socket_path: &std::path::Path,
        config: &VmConfig,
    ) -> Result<()> {
        // Convert to Firecracker configuration
        let fc_config = config.to_firecracker_config()?;

        // Configure machine
        self.send_api_request(
            socket_path,
            "machine-config",
            serde_json::to_value(fc_config.machine_config)?,
        )
        .await?;

        // Configure boot source
        self.send_api_request(
            socket_path,
            "boot-source",
            serde_json::to_value(fc_config.boot_source)?,
        )
        .await?;

        // Configure drives
        for drive in fc_config.drives {
            self.send_api_request(socket_path, "drives", serde_json::to_value(drive)?)
                .await?;
        }

        // Configure network interfaces
        for interface in fc_config.network_interfaces {
            self.send_api_request(
                socket_path,
                "network-interfaces",
                serde_json::to_value(interface)?,
            )
            .await?;
        }

        Ok(())
    }

    /// Send request to Firecracker API
    async fn send_api_request(
        &self,
        socket_path: &std::path::Path,
        endpoint: &str,
        _data: serde_json::Value,
    ) -> Result<serde_json::Value> {
        // This would implement actual Firecracker API communication
        // For now, simulate successful API calls
        debug!(
            "Firecracker API call: {} -> {}",
            endpoint,
            socket_path.display()
        );

        // Simulate API response
        Ok(serde_json::json!({
            "status": "success",
            "endpoint": endpoint
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_firecracker_client_creation() {
        let temp_dir = TempDir::new().unwrap();
        let client = FirecrackerClient::new(temp_dir.path().to_path_buf());

        // Test that client was created successfully
        assert_eq!(client.socket_dir, temp_dir.path());
    }

    #[tokio::test]
    async fn test_vm_config_conversion() {
        let config =
            crate::vm::config::VmConfig::new("test-vm".to_string(), "test-type".to_string())
                .with_kernel_path("/path/to/kernel".to_string())
                .with_rootfs_path("/path/to/rootfs".to_string());

        let fc_config = config.to_firecracker_config().unwrap();

        assert_eq!(fc_config.boot_source.kernel_image_path, "/path/to/kernel");
        assert_eq!(fc_config.drives.len(), 1);
        assert_eq!(fc_config.machine_config.vcpu_count, 1);
    }
}
