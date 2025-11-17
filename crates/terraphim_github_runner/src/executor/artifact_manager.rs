//! Artifact management for CI/CD execution

use crate::{RunnerResult, RunnerError, ArtifactRef};
use std::path::{Path, PathBuf};

/// Manager for build artifacts
pub struct ArtifactManager {
    /// Working directory
    work_dir: PathBuf,
    /// Artifact storage directory
    artifact_dir: PathBuf,
}

impl ArtifactManager {
    /// Create a new artifact manager
    pub fn new(work_dir: &str) -> Self {
        let work_path = PathBuf::from(work_dir);
        let artifact_dir = work_path.join(".artifacts");

        Self {
            work_dir: work_path,
            artifact_dir,
        }
    }

    /// Initialize artifact storage
    pub fn initialize(&self) -> RunnerResult<()> {
        if !self.artifact_dir.exists() {
            std::fs::create_dir_all(&self.artifact_dir)?;
        }
        Ok(())
    }

    /// Collect artifacts from produced items
    pub async fn collect_artifacts(&self, produces: &[String]) -> RunnerResult<Vec<ArtifactRef>> {
        let mut artifacts = Vec::new();

        for produced in produces {
            // Try to find the produced item
            let path = self.work_dir.join(produced);

            if path.exists() {
                let artifact = self.create_artifact_ref(&path)?;
                artifacts.push(artifact);
            } else {
                // Try common patterns
                if produced.contains("node_modules") {
                    let nm_path = self.work_dir.join("node_modules");
                    if nm_path.exists() {
                        let artifact = self.create_artifact_ref(&nm_path)?;
                        artifacts.push(artifact);
                    }
                } else if produced.contains("target") {
                    let target_path = self.work_dir.join("target");
                    if target_path.exists() {
                        let artifact = self.create_artifact_ref(&target_path)?;
                        artifacts.push(artifact);
                    }
                }
            }
        }

        Ok(artifacts)
    }

    /// Create an artifact reference
    fn create_artifact_ref(&self, path: &Path) -> RunnerResult<ArtifactRef> {
        let metadata = std::fs::metadata(path)?;
        let size = if metadata.is_dir() {
            self.calculate_dir_size(path)?
        } else {
            metadata.len()
        };

        // Calculate hash (simplified - would use SHA256 in real impl)
        let hash = format!("{:x}", size);

        Ok(ArtifactRef {
            name: path.file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .to_string(),
            path: path.to_string_lossy().to_string(),
            size,
            hash,
        })
    }

    /// Calculate directory size
    fn calculate_dir_size(&self, path: &Path) -> RunnerResult<u64> {
        let mut size = 0;

        if path.is_dir() {
            for entry in std::fs::read_dir(path)? {
                let entry = entry?;
                let metadata = entry.metadata()?;

                if metadata.is_dir() {
                    size += self.calculate_dir_size(&entry.path())?;
                } else {
                    size += metadata.len();
                }
            }
        }

        Ok(size)
    }

    /// Upload artifact to storage
    pub async fn upload_artifact(&self, name: &str, path: &Path) -> RunnerResult<ArtifactRef> {
        let dest = self.artifact_dir.join(name);

        // Copy to artifact storage
        if path.is_dir() {
            self.copy_dir(path, &dest)?;
        } else {
            std::fs::copy(path, &dest)?;
        }

        self.create_artifact_ref(&dest)
    }

    /// Download artifact from storage
    pub async fn download_artifact(&self, name: &str, dest: &Path) -> RunnerResult<()> {
        let src = self.artifact_dir.join(name);

        if !src.exists() {
            return Err(RunnerError::ActionExecution(format!(
                "Artifact not found: {}",
                name
            )));
        }

        // Copy from artifact storage
        if src.is_dir() {
            self.copy_dir(&src, dest)?;
        } else {
            std::fs::copy(&src, dest)?;
        }

        Ok(())
    }

    /// Copy directory recursively
    fn copy_dir(&self, src: &Path, dest: &Path) -> RunnerResult<()> {
        std::fs::create_dir_all(dest)?;

        for entry in std::fs::read_dir(src)? {
            let entry = entry?;
            let src_path = entry.path();
            let dest_path = dest.join(entry.file_name());

            if src_path.is_dir() {
                self.copy_dir(&src_path, &dest_path)?;
            } else {
                std::fs::copy(&src_path, &dest_path)?;
            }
        }

        Ok(())
    }

    /// List artifacts
    pub fn list_artifacts(&self) -> RunnerResult<Vec<String>> {
        let mut artifacts = Vec::new();

        if self.artifact_dir.exists() {
            for entry in std::fs::read_dir(&self.artifact_dir)? {
                let entry = entry?;
                artifacts.push(entry.file_name().to_string_lossy().to_string());
            }
        }

        Ok(artifacts)
    }

    /// Delete artifact
    pub fn delete_artifact(&self, name: &str) -> RunnerResult<()> {
        let path = self.artifact_dir.join(name);

        if path.exists() {
            if path.is_dir() {
                std::fs::remove_dir_all(path)?;
            } else {
                std::fs::remove_file(path)?;
            }
        }

        Ok(())
    }

    /// Clean up old artifacts
    pub fn cleanup(&self, max_age_hours: u64) -> RunnerResult<usize> {
        let mut deleted = 0;
        let cutoff = std::time::SystemTime::now()
            - std::time::Duration::from_secs(max_age_hours * 3600);

        if self.artifact_dir.exists() {
            for entry in std::fs::read_dir(&self.artifact_dir)? {
                let entry = entry?;
                let metadata = entry.metadata()?;

                if let Ok(modified) = metadata.modified() {
                    if modified < cutoff {
                        let path = entry.path();
                        if path.is_dir() {
                            std::fs::remove_dir_all(&path)?;
                        } else {
                            std::fs::remove_file(&path)?;
                        }
                        deleted += 1;
                    }
                }
            }
        }

        Ok(deleted)
    }
}
