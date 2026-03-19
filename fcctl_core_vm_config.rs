use crate::firecracker::VmType;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VmConfig {
    pub vcpus: u32,
    pub memory_mb: u32,
    pub kernel_path: String,
    pub rootfs_path: String,
    pub initrd_path: Option<String>,
    pub boot_args: Option<String>,
    pub vm_type: VmType,

    // NEW: Extended fields from terraphim_firecracker::VmRequirements
    /// Timeout for VM operations in seconds
    pub timeout_seconds: Option<u32>,
    /// Whether networking is enabled for this VM
    pub network_enabled: Option<bool>,
    /// Storage allocation in GB
    pub storage_gb: Option<u32>,
    /// Labels for VM categorisation and filtering
    pub labels: Option<HashMap<String, String>>,
}

impl VmConfig {
    pub fn atomic() -> Self {
        Self {
            vcpus: 2,
            memory_mb: 4096,
            kernel_path: "images/focal/focal.vmlinux".to_string(),
            rootfs_path: "images/focal/focal.rootfs".to_string(),
            initrd_path: None,
            boot_args: Some("console=ttyS0 reboot=k panic=1 pci=off".to_string()),
            vm_type: VmType::Atomic,
            timeout_seconds: Some(300),
            network_enabled: Some(false),
            storage_gb: Some(10),
            labels: None,
        }
    }

    pub fn terraphim() -> Self {
        Self {
            vcpus: 4,
            memory_mb: 8192,
            kernel_path: "terraphim-firecracker/bionic.vmlinux".to_string(),
            rootfs_path: "terraphim-firecracker/images/bionic/terraphim-bionic.local.rootfs"
                .to_string(),
            initrd_path: None,
            boot_args: Some("console=ttyS0 reboot=k panic=1 pci=off".to_string()),
            vm_type: VmType::Terraphim,
            timeout_seconds: Some(600),
            network_enabled: Some(true),
            storage_gb: Some(50),
            labels: None,
        }
    }

    pub fn terraphim_minimal() -> Self {
        let mut config = Self::terraphim();
        config.memory_mb = 4096;
        config.vcpus = 2;
        config.vm_type = VmType::TerrraphimMinimal;
        config.timeout_seconds = Some(300);
        config.network_enabled = Some(false);
        config.storage_gb = Some(20);
        config
    }

    pub fn minimal() -> Self {
        Self {
            vcpus: 1,
            memory_mb: 1024,
            kernel_path: "kernel.bin".to_string(),
            rootfs_path: "rootfs.ext4".to_string(),
            initrd_path: None,
            boot_args: Some("console=ttyS0 reboot=k panic=1 pci=off".to_string()),
            vm_type: VmType::Minimal,
            timeout_seconds: Some(180),
            network_enabled: Some(false),
            storage_gb: Some(5),
            labels: None,
        }
    }

    /// Create a new VmConfig with only required fields, leaving extended fields as None
    pub fn new(
        vcpus: u32,
        memory_mb: u32,
        kernel_path: impl Into<String>,
        rootfs_path: impl Into<String>,
        vm_type: VmType,
    ) -> Self {
        Self {
            vcpus,
            memory_mb,
            kernel_path: kernel_path.into(),
            rootfs_path: rootfs_path.into(),
            initrd_path: None,
            boot_args: None,
            vm_type,
            timeout_seconds: None,
            network_enabled: None,
            storage_gb: None,
            labels: None,
        }
    }

    /// Set timeout_seconds
    pub fn with_timeout(mut self, timeout_seconds: u32) -> Self {
        self.timeout_seconds = Some(timeout_seconds);
        self
    }

    /// Set network_enabled
    pub fn with_networking(mut self, enabled: bool) -> Self {
        self.network_enabled = Some(enabled);
        self
    }

    /// Set storage_gb
    pub fn with_storage(mut self, storage_gb: u32) -> Self {
        self.storage_gb = Some(storage_gb);
        self
    }

    /// Set labels
    pub fn with_labels(mut self, labels: HashMap<String, String>) -> Self {
        self.labels = Some(labels);
        self
    }

    /// Add a single label
    pub fn with_label(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        let mut labels = self.labels.unwrap_or_default();
        labels.insert(key.into(), value.into());
        self.labels = Some(labels);
        self
    }
}
