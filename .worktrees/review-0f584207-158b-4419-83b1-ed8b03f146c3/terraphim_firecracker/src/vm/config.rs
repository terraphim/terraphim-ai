use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VmConfig {
    pub vm_id: String,
    pub vm_type: String,
    pub memory_mb: u32,
    pub vcpus: u32,
    pub kernel_path: Option<String>,
    pub rootfs_path: Option<String>,
    pub kernel_args: Option<String>,
    pub data_dir: PathBuf,
    pub enable_networking: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VmTypeConfig {
    pub name: String,
    pub memory_mb: u32,
    pub vcpus: u32,
    pub kernel_path: String,
    pub rootfs_path: String,
    pub default_kernel_args: String,
    pub enable_networking: bool,
    pub optimization_level: OptimizationLevel,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum OptimizationLevel {
    Standard,
    Fast,
    UltraFast,
    Sub2Second,
}

#[allow(dead_code)]
impl VmConfig {
    pub fn new(vm_id: String, vm_type: String) -> Self {
        Self {
            vm_id,
            vm_type,
            memory_mb: 512,
            vcpus: 1,
            kernel_path: None,
            rootfs_path: None,
            kernel_args: None,
            data_dir: PathBuf::from("/tmp/terraphim-vms"),
            enable_networking: false,
        }
    }
}

impl Default for VmConfig {
    fn default() -> Self {
        Self {
            vm_id: "default-vm".to_string(),
            vm_type: "default-type".to_string(),
            memory_mb: 512,
            vcpus: 1,
            kernel_path: None,
            rootfs_path: None,
            kernel_args: None,
            data_dir: PathBuf::from("/tmp/terraphim-vms"),
            enable_networking: false,
        }
    }
}

#[allow(dead_code)]
impl VmConfig {
    #[allow(dead_code)]
    pub fn with_memory(mut self, memory_mb: u32) -> Self {
        self.memory_mb = memory_mb;
        self
    }

    #[allow(dead_code)]
    pub fn with_vcpus(mut self, vcpus: u32) -> Self {
        self.vcpus = vcpus;
        self
    }

    #[allow(dead_code)]
    pub fn with_kernel_path(mut self, kernel_path: String) -> Self {
        self.kernel_path = Some(kernel_path);
        self
    }

    #[allow(dead_code)]
    pub fn with_rootfs_path(mut self, rootfs_path: String) -> Self {
        self.rootfs_path = Some(rootfs_path);
        self
    }

    #[allow(dead_code)]
    pub fn with_kernel_args(mut self, kernel_args: String) -> Self {
        self.kernel_args = Some(kernel_args);
        self
    }

    #[allow(dead_code)]
    pub fn with_data_dir(mut self, data_dir: PathBuf) -> Self {
        self.data_dir = data_dir;
        self
    }

    #[allow(dead_code)]
    pub fn with_networking(mut self, enable: bool) -> Self {
        self.enable_networking = enable;
        self
    }

    pub fn to_firecracker_config(&self) -> Result<FirecrackerConfig> {
        let kernel_path = self
            .kernel_path
            .clone()
            .ok_or_else(|| anyhow::anyhow!("Kernel path not specified"))?;

        let rootfs_path = self
            .rootfs_path
            .clone()
            .ok_or_else(|| anyhow::anyhow!("Rootfs path not specified"))?;

        let boot_source = BootSource {
            kernel_image_path: kernel_path,
            boot_args: self.kernel_args.clone(),
            initrd_path: None,
        };

        let drives = vec![Drive {
            drive_id: "rootfs".to_string(),
            path_on_host: rootfs_path,
            is_root_device: true,
            is_read_only: false,
            partuuid: None,
            cache_type: "Unsafe".to_string(),
            io_engine: "Sync".to_string(),
            rate_limiter: None,
        }];

        let machine_config = MachineConfig {
            vcpu_count: self.vcpus,
            mem_size_mib: self.memory_mb,
            smt: false,
            track_dirty_pages: false,
        };

        Ok(FirecrackerConfig {
            boot_source,
            drives,
            machine_config,
            network_interfaces: if self.enable_networking {
                vec![NetworkInterface::default()]
            } else {
                vec![]
            },
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FirecrackerConfig {
    pub boot_source: BootSource,
    pub drives: Vec<Drive>,
    pub machine_config: MachineConfig,
    pub network_interfaces: Vec<NetworkInterface>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BootSource {
    pub kernel_image_path: String,
    pub boot_args: Option<String>,
    pub initrd_path: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Drive {
    pub drive_id: String,
    pub path_on_host: String,
    pub is_root_device: bool,
    pub is_read_only: bool,
    pub partuuid: Option<String>,
    pub cache_type: String,
    pub io_engine: String,
    pub rate_limiter: Option<RateLimiter>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MachineConfig {
    pub vcpu_count: u32,
    pub mem_size_mib: u32,
    pub smt: bool,
    pub track_dirty_pages: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkInterface {
    pub iface_id: String,
    pub host_dev_name: String,
    pub guest_mac: Option<String>,
    pub rx_rate_limiter: Option<RateLimiter>,
    pub tx_rate_limiter: Option<RateLimiter>,
}

impl Default for NetworkInterface {
    fn default() -> Self {
        Self {
            iface_id: "eth0".to_string(),
            host_dev_name: "tap0".to_string(),
            guest_mac: None,
            rx_rate_limiter: None,
            tx_rate_limiter: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RateLimiter {
    pub bandwidth: Option<Bandwidth>,
    pub ops: Option<Ops>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Bandwidth {
    pub size: u64,
    pub refill_time: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Ops {
    pub size: u64,
    pub refill_time: u64,
}

#[allow(dead_code)]
pub fn get_vm_type_configs() -> Vec<VmTypeConfig> {
    vec![
        VmTypeConfig {
            name: "terraphim-minimal".to_string(),
            memory_mb: 256,
            vcpus: 1,
            kernel_path: "images/terraphim-minimal.vmlinux".to_string(),
            rootfs_path: "images/terraphim-minimal.rootfs.ext4".to_string(),
            default_kernel_args: "console=ttyS0 quiet reboot=k panic=1 pci=off random.trust_cpu=on systemd.unit=multi-user.target".to_string(),
            enable_networking: false,
            optimization_level: OptimizationLevel::Sub2Second,
        },
        VmTypeConfig {
            name: "terraphim-standard".to_string(),
            memory_mb: 512,
            vcpus: 1,
            kernel_path: "images/terraphim-standard.vmlinux".to_string(),
            rootfs_path: "images/terraphim-standard.rootfs.ext4".to_string(),
            default_kernel_args: "console=ttyS0 quiet reboot=k panic=1 pci=off random.trust_cpu=on systemd.unit=multi-user.target".to_string(),
            enable_networking: true,
            optimization_level: OptimizationLevel::UltraFast,
        },
        VmTypeConfig {
            name: "terraphim-development".to_string(),
            memory_mb: 1024,
            vcpus: 2,
            kernel_path: "images/terraphim-dev.vmlinux".to_string(),
            rootfs_path: "images/terraphim-dev.rootfs.ext4".to_string(),
            default_kernel_args: "console=ttyS0 reboot=k panic=1 pci=off random.trust_cpu=on".to_string(),
            enable_networking: true,
            optimization_level: OptimizationLevel::Fast,
        },
    ]
}

#[allow(dead_code)]
pub fn get_vm_type_config(vm_type: &str) -> Result<VmTypeConfig> {
    let configs = get_vm_type_configs();
    configs
        .into_iter()
        .find(|config| config.name == vm_type)
        .ok_or_else(|| anyhow::anyhow!("Unknown VM type: {}", vm_type))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vm_config_creation() {
        let config = VmConfig::new("test-vm".to_string(), "terraphim-minimal".to_string())
            .with_memory(256)
            .with_vcpus(1)
            .with_kernel_path("/path/to/kernel".to_string())
            .with_rootfs_path("/path/to/rootfs".to_string());

        assert_eq!(config.vm_id, "test-vm");
        assert_eq!(config.vm_type, "terraphim-minimal");
        assert_eq!(config.memory_mb, 256);
        assert_eq!(config.vcpus, 1);
        assert_eq!(config.kernel_path, Some("/path/to/kernel".to_string()));
        assert_eq!(config.rootfs_path, Some("/path/to/rootfs".to_string()));
    }

    #[test]
    fn test_get_vm_type_config() {
        let config = get_vm_type_config("terraphim-minimal").unwrap();
        assert_eq!(config.name, "terraphim-minimal");
        assert_eq!(config.memory_mb, 256);
        assert_eq!(config.optimization_level, OptimizationLevel::Sub2Second);
    }

    #[test]
    fn test_unknown_vm_type() {
        let result = get_vm_type_config("unknown-type");
        assert!(result.is_err());
    }

    #[test]
    fn test_firecracker_config_conversion() {
        let vm_config = VmConfig::new("test-vm".to_string(), "terraphim-minimal".to_string())
            .with_kernel_path("/path/to/kernel".to_string())
            .with_rootfs_path("/path/to/rootfs".to_string());

        let fc_config = vm_config.to_firecracker_config().unwrap();
        assert_eq!(fc_config.boot_source.kernel_image_path, "/path/to/kernel");
        assert_eq!(fc_config.drives.len(), 1);
        assert_eq!(fc_config.drives[0].path_on_host, "/path/to/rootfs");
        assert_eq!(fc_config.machine_config.vcpu_count, 1);
        assert_eq!(fc_config.machine_config.mem_size_mib, 512);
    }

    #[test]
    fn test_firecracker_config_missing_paths() {
        let vm_config = VmConfig::new("test-vm".to_string(), "terraphim-minimal".to_string());
        let result = vm_config.to_firecracker_config();
        assert!(result.is_err());
    }
}
