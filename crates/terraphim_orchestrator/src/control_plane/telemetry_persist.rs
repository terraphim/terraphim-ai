//! Persistable implementation for TelemetrySummary.
//!
//! Stores aggregated model performance snapshots via terraphim_persistence
//! so routing decisions survive orchestrator restarts.

use async_trait::async_trait;
use terraphim_persistence::{Persistable, Result};

use super::telemetry::TelemetrySummary;

const TELEMETRY_SUMMARY_KEY: &str = "telemetry_summary";

#[async_trait]
impl Persistable for TelemetrySummary {
    fn new(key: String) -> Self {
        TelemetrySummary {
            id: if key.is_empty() {
                TELEMETRY_SUMMARY_KEY.to_string()
            } else {
                key
            },
            model_performances: Vec::new(),
            window_secs: 3600,
            exported_at: chrono::Utc::now(),
        }
    }

    async fn save_to_one(&self, profile_name: &str) -> Result<()> {
        self.save_to_profile(profile_name).await
    }

    async fn save(&self) -> Result<()> {
        self.save_to_all().await
    }

    async fn load(&mut self) -> Result<Self> {
        let op = &self.load_config().await?.1;
        let key = self.get_key();
        self.load_from_operator(&key, op).await
    }

    fn get_key(&self) -> String {
        let normalized = self.normalize_key(&self.id);
        format!("{}.json", normalized)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;
    use terraphim_persistence::DeviceStorage;

    async fn init_test_persistence() -> Result<()> {
        DeviceStorage::init_memory_only().await?;
        Ok(())
    }

    #[tokio::test]
    #[serial]
    async fn test_key_format() {
        let summary = TelemetrySummary::new("telemetry_summary".to_string());
        assert_eq!(summary.get_key(), "telemetry_summary.json");
    }

    #[tokio::test]
    #[serial]
    async fn test_empty_key_uses_default() {
        let summary = TelemetrySummary::new(String::new());
        assert_eq!(summary.id, "telemetry_summary");
    }

    #[tokio::test]
    #[serial]
    async fn test_round_trip_save_load() -> Result<()> {
        init_test_persistence().await?;

        use super::super::telemetry::{ModelPerformanceSnapshot, TelemetrySummary};
        use chrono::Utc;

        let original = TelemetrySummary {
            id: "telemetry_summary".to_string(),
            model_performances: vec![
                ModelPerformanceSnapshot::empty("model-a", 3600),
                ModelPerformanceSnapshot::empty("model-b", 3600),
            ],
            window_secs: 3600,
            exported_at: Utc::now(),
        };

        original.save_to_one("memory").await?;

        let mut loaded = TelemetrySummary::new("telemetry_summary".to_string());
        loaded = loaded.load().await?;

        assert_eq!(loaded.id, "telemetry_summary");
        assert_eq!(loaded.model_performances.len(), 2);
        assert_eq!(loaded.model_performances[0].model, "model-a");
        assert_eq!(loaded.model_performances[1].model, "model-b");
        assert_eq!(loaded.window_secs, 3600);

        Ok(())
    }

    #[tokio::test]
    #[serial]
    async fn test_save_to_all_profiles() -> Result<()> {
        init_test_persistence().await?;

        let summary = TelemetrySummary::new("telemetry_summary".to_string());
        summary.save().await?;

        let mut loaded = TelemetrySummary::new("telemetry_summary".to_string());
        loaded = loaded.load().await?;

        assert_eq!(loaded.id, "telemetry_summary");
        Ok(())
    }

    #[tokio::test]
    #[serial]
    async fn test_load_nonexistent_returns_error() -> Result<()> {
        init_test_persistence().await?;

        let mut summary = TelemetrySummary::new("telemetry_summary_nonexistent".to_string());
        let result = summary.load().await;
        assert!(result.is_err());
        Ok(())
    }
}
