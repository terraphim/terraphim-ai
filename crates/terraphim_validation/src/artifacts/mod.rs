//! Artifact management for release validation
//!
//! This module handles the discovery, download, and management of release artifacts
//! across different platforms and package formats.

use anyhow::{Result, anyhow};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use sha2::Digest;
use std::collections::HashMap;
use tokio::fs;
use uuid::Uuid;

/// Supported platforms for release validation
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum Platform {
    LinuxX86_64,
    LinuxAarch64,
    LinuxArmV7,
    MacOSX86_64,
    MacOSAarch64,
    WindowsX86_64,
}

impl Platform {
    /// Get platform string representation
    pub fn as_str(&self) -> &'static str {
        match self {
            Platform::LinuxX86_64 => "x86_64-unknown-linux-gnu",
            Platform::LinuxAarch64 => "aarch64-unknown-linux-gnu",
            Platform::LinuxArmV7 => "armv7-unknown-linux-gnueabihf",
            Platform::MacOSX86_64 => "x86_64-apple-darwin",
            Platform::MacOSAarch64 => "aarch64-apple-darwin",
            Platform::WindowsX86_64 => "x86_64-pc-windows-msvc",
        }
    }

    /// Get platform family
    pub fn family(&self) -> PlatformFamily {
        match self {
            Platform::LinuxX86_64 | Platform::LinuxAarch64 | Platform::LinuxArmV7 => {
                PlatformFamily::Linux
            }
            Platform::MacOSX86_64 | Platform::MacOSAarch64 => PlatformFamily::MacOS,
            Platform::WindowsX86_64 => PlatformFamily::Windows,
        }
    }
}

/// Platform family for grouping similar platforms
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum PlatformFamily {
    Linux,
    MacOS,
    Windows,
}

/// Artifact types supported by the validation system
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ArtifactType {
    Binary,
    DebPackage,
    RpmPackage,
    TarGz,
    TarZst,
    Dmg,
    Msi,
    Exe,
    AppImage,
    DockerImage,
}

/// Release artifact metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReleaseArtifact {
    pub id: Uuid,
    pub name: String,
    pub version: String,
    pub platform: Platform,
    pub artifact_type: ArtifactType,
    pub download_url: String,
    pub checksum: String,
    pub size_bytes: u64,
    pub local_path: Option<String>,
}

impl ReleaseArtifact {
    /// Create a new release artifact
    pub fn new(
        name: String,
        version: String,
        platform: Platform,
        artifact_type: ArtifactType,
        download_url: String,
        checksum: String,
        size_bytes: u64,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            name,
            version,
            platform,
            artifact_type,
            download_url,
            checksum,
            size_bytes,
            local_path: None,
        }
    }

    /// Download the artifact to a local path
    pub async fn download(&mut self, client: &Client, download_dir: &str) -> Result<()> {
        let filename = self.extract_filename();
        let local_path = format!("{}/{}", download_dir, filename);

        // Download the file
        let response = client.get(&self.download_url).send().await?;
        let bytes = response.bytes().await?;

        // Save to local file
        fs::write(&local_path, bytes).await?;

        // Update local path
        self.local_path = Some(local_path);

        Ok(())
    }

    /// Verify the artifact checksum
    pub async fn verify_checksum(&self) -> Result<bool> {
        let local_path = self
            .local_path
            .as_ref()
            .ok_or_else(|| anyhow!("Artifact not downloaded"))?;

        let contents = fs::read(local_path).await?;
        let mut hasher = sha2::Sha256::new();
        hasher.update(&contents);
        let checksum = hasher.finalize();
        let computed_hash = hex::encode(checksum);

        Ok(computed_hash == self.checksum)
    }

    /// Extract filename from download URL
    fn extract_filename(&self) -> String {
        self.download_url
            .split('/')
            .last()
            .unwrap_or("unknown")
            .to_string()
    }

    /// Check if the artifact is available locally
    pub fn is_available_locally(&self) -> bool {
        self.local_path
            .as_ref()
            .map(|path| std::path::Path::new(path).exists())
            .unwrap_or(false)
    }
}

/// Artifact manager for handling release artifacts
pub struct ArtifactManager {
    client: Client,
    artifacts: HashMap<Uuid, ReleaseArtifact>,
    download_dir: String,
}

impl ArtifactManager {
    /// Create a new artifact manager
    pub fn new(download_dir: String) -> Self {
        Self {
            client: Client::new(),
            artifacts: HashMap::new(),
            download_dir,
        }
    }

    /// Add an artifact to the manager
    pub fn add_artifact(&mut self, artifact: ReleaseArtifact) {
        self.artifacts.insert(artifact.id, artifact);
    }

    /// Get an artifact by ID
    pub fn get_artifact(&self, id: &Uuid) -> Option<&ReleaseArtifact> {
        self.artifacts.get(id)
    }

    /// Get all artifacts for a platform
    pub fn get_artifacts_for_platform(&self, platform: &Platform) -> Vec<&ReleaseArtifact> {
        self.artifacts
            .values()
            .filter(|artifact| artifact.platform == *platform)
            .collect()
    }

    /// Download all artifacts
    pub async fn download_all(&mut self) -> Result<()> {
        // Create download directory if it doesn't exist
        fs::create_dir_all(&self.download_dir).await?;

        for artifact in self.artifacts.values_mut() {
            if !artifact.is_available_locally() {
                artifact.download(&self.client, &self.download_dir).await?;
            }
        }

        Ok(())
    }

    /// Verify all downloaded artifacts
    pub async fn verify_all(&self) -> Result<Vec<(Uuid, bool)>> {
        let mut results = Vec::new();

        for artifact in self.artifacts.values() {
            if artifact.is_available_locally() {
                let is_valid = artifact.verify_checksum().await?;
                results.push((artifact.id, is_valid));
            }
        }

        Ok(results)
    }

    /// Get artifact statistics
    pub fn get_statistics(&self) -> ArtifactStatistics {
        let total_artifacts = self.artifacts.len();
        let downloaded_artifacts = self
            .artifacts
            .values()
            .filter(|artifact| artifact.is_available_locally())
            .count();

        let platform_counts =
            self.artifacts
                .values()
                .fold(HashMap::new(), |mut counts, artifact| {
                    *counts.entry(artifact.platform.clone()).or_insert(0) += 1;
                    counts
                });

        ArtifactStatistics {
            total_artifacts,
            downloaded_artifacts,
            platform_counts,
        }
    }
}

/// Artifact statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArtifactStatistics {
    pub total_artifacts: usize,
    pub downloaded_artifacts: usize,
    pub platform_counts: HashMap<Platform, usize>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_platform_strings() {
        assert_eq!(Platform::LinuxX86_64.as_str(), "x86_64-unknown-linux-gnu");
        assert_eq!(Platform::MacOSAarch64.family(), PlatformFamily::MacOS);
    }

    #[test]
    fn test_artifact_creation() {
        let artifact = ReleaseArtifact::new(
            "test".to_string(),
            "1.0.0".to_string(),
            Platform::LinuxX86_64,
            ArtifactType::Binary,
            "https://example.com/test".to_string(),
            "abc123".to_string(),
            1024,
        );

        assert_eq!(artifact.name, "test");
        assert_eq!(artifact.version, "1.0.0");
        assert_eq!(artifact.platform, Platform::LinuxX86_64);
    }
}
