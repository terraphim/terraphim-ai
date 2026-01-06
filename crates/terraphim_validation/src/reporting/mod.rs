//! Reporting module for validation results
//!
//! This module provides comprehensive reporting capabilities for validation results,
//! including multiple output formats and dashboard integration.

use crate::validators::{ValidationStatistics, ValidationSummary};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Report output formats
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, clap::ValueEnum)]
pub enum ReportFormat {
    Json,
    Yaml,
    Markdown,
    Html,
    Csv,
}

impl std::fmt::Display for ReportFormat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ReportFormat::Json => write!(f, "json"),
            ReportFormat::Yaml => write!(f, "yaml"),
            ReportFormat::Markdown => write!(f, "markdown"),
            ReportFormat::Html => write!(f, "html"),
            ReportFormat::Csv => write!(f, "csv"),
        }
    }
}

/// Validation report containing all validation results
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationReport {
    pub id: Uuid,
    pub version: String,
    pub generated_at: DateTime<Utc>,
    pub summary: ValidationSummary,
    pub metadata: ReportMetadata,
}

impl ValidationReport {
    /// Create a validation report from a validation summary
    pub fn from_summary(summary: ValidationSummary) -> Self {
        Self {
            id: Uuid::new_v4(),
            generated_at: Utc::now(),
            version: summary.version.clone(),
            summary,
            metadata: ReportMetadata::default(),
        }
    }

    /// Get validation statistics
    pub fn get_statistics(&self) -> ValidationStatistics {
        self.summary.get_statistics()
    }

    /// Check if the validation passed overall
    pub fn is_success(&self) -> bool {
        self.summary.overall_status.is_success()
    }

    /// Check if the validation has critical issues
    pub fn has_critical_issues(&self) -> bool {
        self.summary
            .results
            .values()
            .any(|result| result.has_critical_issues())
    }

    /// Generate report in specified format
    pub fn generate(&self, format: &ReportFormat) -> Result<String, anyhow::Error> {
        match format {
            ReportFormat::Json => self.generate_json(),
            ReportFormat::Yaml => self.generate_yaml(),
            ReportFormat::Markdown => self.generate_markdown(),
            ReportFormat::Html => self.generate_html(),
            ReportFormat::Csv => self.generate_csv(),
        }
    }

    /// Generate JSON format report
    fn generate_json(&self) -> Result<String, anyhow::Error> {
        serde_json::to_string_pretty(self)
            .map_err(|e| anyhow::anyhow!("Failed to generate JSON report: {}", e))
    }

    /// Generate YAML format report
    fn generate_yaml(&self) -> Result<String, anyhow::Error> {
        serde_yaml::to_string(self)
            .map_err(|e| anyhow::anyhow!("Failed to generate YAML report: {}", e))
    }

    /// Generate Markdown format report
    fn generate_markdown(&self) -> Result<String, anyhow::Error> {
        let mut content = String::new();

        // Header
        content.push_str("# Terraphim AI Release Validation Report\n\n");

        // Summary section
        content.push_str("## Validation Summary\n\n");
        let stats = self.get_statistics();
        content.push_str(&format!("- **Version**: {}\n", self.version));
        content.push_str(&format!(
            "- **Status**: {:?}\n",
            self.summary.overall_status
        ));
        content.push_str(&format!(
            "- **Total Validations**: {}\n",
            stats.total_validations
        ));
        content.push_str(&format!(
            "- **Passed**: {} ({:.1}%)\n",
            stats.passed_validations,
            stats.success_rate() * 100.0
        ));
        content.push_str(&format!(
            "- **Failed**: {} ({:.1}%)\n",
            stats.failed_validations,
            stats.failure_rate() * 100.0
        ));
        content.push_str(&format!("- **Total Issues**: {}\n", stats.total_issues));
        content.push_str(&format!(
            "- **Critical Issues**: {}\n\n",
            stats.critical_issues
        ));

        // Results section
        content.push_str("## Detailed Results\n\n");

        for (id, result) in &self.summary.results {
            content.push_str(&format!("### {}\n\n", result.name));
            content.push_str(&format!("- **Category**: {}\n", result.category));
            content.push_str(&format!("- **Status**: {:?}\n", result.status));
            content.push_str(&format!("- **Duration**: {}ms\n\n", result.duration_ms));

            if !result.issues.is_empty() {
                content.push_str("#### Issues\n\n");
                for issue in &result.issues {
                    content.push_str(&format!(
                        "- **{:?}**: {} - {}\n",
                        issue.severity, issue.title, issue.description
                    ));
                    if let Some(recommendation) = &issue.recommendation {
                        content.push_str(&format!("  - *Recommendation*: {}\n", recommendation));
                    }
                }
                content.push('\n');
            }
        }

        // Metadata section
        content.push_str("## Report Metadata\n\n");
        content.push_str(&format!("- **Report ID**: {}\n", self.id));
        content.push_str(&format!(
            "- **Generated**: {}\n",
            self.generated_at.format("%Y-%m-%d %H:%M:%S UTC")
        ));
        content.push_str(&format!(
            "- **Environment**: {}\n",
            self.metadata.environment
        ));
        content.push_str(&format!(
            "- **Validator Version**: {}\n",
            self.metadata.validator_version
        ));

        Ok(content)
    }

    /// Generate HTML format report
    fn generate_html(&self) -> Result<String, anyhow::Error> {
        let mut content = String::new();

        // HTML header
        content.push_str("<!DOCTYPE html>\n<html>\n<head>\n");
        content.push_str("<title>Terraphim AI Release Validation Report</title>\n");
        content.push_str("<style>\n");
        content.push_str("body { font-family: Arial, sans-serif; margin: 40px; }\n");
        content.push_str(".status-passed { color: #28a745; }\n");
        content.push_str(".status-failed { color: #dc3545; }\n");
        content.push_str(".severity-critical { color: #dc3545; font-weight: bold; }\n");
        content.push_str(".severity-warning { color: #ffc107; }\n");
        content.push_str(".severity-info { color: #17a2b8; }\n");
        content.push_str("table { border-collapse: collapse; width: 100%; margin: 20px 0; }\n");
        content.push_str("th, td { border: 1px solid #ddd; padding: 8px; text-align: left; }\n");
        content.push_str("th { background-color: #f2f2f2; }\n");
        content.push_str("</style>\n</head>\n<body>\n");

        // Title
        content.push_str("<h1>Terraphim AI Release Validation Report</h1>\n");

        // Summary
        let stats = self.get_statistics();
        content.push_str("<h2>Validation Summary</h2>\n");
        content.push_str("<table>\n");
        content.push_str(&format!(
            "<tr><td><strong>Version</strong></td><td>{}</td></tr>\n",
            self.version
        ));
        content.push_str(&format!(
            "<tr><td><strong>Status</strong></td><td class=\"status-{:?}\">{:?}</td></tr>\n",
            self.summary.overall_status.to_string().to_lowercase(),
            self.summary.overall_status
        ));
        content.push_str(&format!(
            "<tr><td><strong>Total Validations</strong></td><td>{}</td></tr>\n",
            stats.total_validations
        ));
        content.push_str(&format!(
            "<tr><td><strong>Passed</strong></td><td>{}</td></tr>\n",
            stats.passed_validations
        ));
        content.push_str(&format!(
            "<tr><td><strong>Failed</strong></td><td>{}</td></tr>\n",
            stats.failed_validations
        ));
        content.push_str(&format!(
            "<tr><td><strong>Total Issues</strong></td><td>{}</td></tr>\n",
            stats.total_issues
        ));
        content.push_str(&format!(
            "<tr><td><strong>Critical Issues</strong></td><td>{}</td></tr>\n",
            stats.critical_issues
        ));
        content.push_str("</table>\n");

        // Detailed results
        content.push_str("<h2>Detailed Results</h2>\n");
        for (id, result) in &self.summary.results {
            content.push_str("<h3>");
            content.push_str(&result.name);
            content.push_str("</h3>\n");

            content.push_str("<table>\n");
            content.push_str(&format!(
                "<tr><td><strong>Category</strong></td><td>{}</td></tr>\n",
                result.category
            ));
            content.push_str(&format!(
                "<tr><td><strong>Status</strong></td><td class=\"status-{:?}\">{:?}</td></tr>\n",
                result.status.to_string().to_lowercase(),
                result.status
            ));
            content.push_str(&format!(
                "<tr><td><strong>Duration</strong></td><td>{}ms</td></tr>\n",
                result.duration_ms
            ));
            content.push_str("</table>\n");

            if !result.issues.is_empty() {
                content.push_str("<h4>Issues</h4>\n");
                content.push_str("<table>\n");
                content.push_str("<tr><th>Severity</th><th>Title</th><th>Description</th><th>Recommendation</th></tr>\n");

                for issue in &result.issues {
                    content.push_str("<tr>\n");
                    content.push_str(&format!(
                        "<td class=\"severity-{:?}\">{:?}</td>\n",
                        issue.severity.to_string().to_lowercase(),
                        issue.severity
                    ));
                    content.push_str(&format!("<td>{}</td>\n", issue.title));
                    content.push_str(&format!("<td>{}</td>\n", issue.description));
                    content.push_str(&format!(
                        "<td>{}</td>\n",
                        issue.recommendation.as_ref().unwrap_or(&"None".to_string())
                    ));
                    content.push_str("</tr>\n");
                }

                content.push_str("</table>\n");
            }
        }

        // Footer
        content.push_str("<h2>Report Metadata</h2>\n");
        content.push_str("<table>\n");
        content.push_str(&format!(
            "<tr><td><strong>Report ID</strong></td><td>{}</td></tr>\n",
            self.id
        ));
        content.push_str(&format!(
            "<tr><td><strong>Generated</strong></td><td>{}</td></tr>\n",
            self.generated_at.format("%Y-%m-%d %H:%M:%S UTC")
        ));
        content.push_str(&format!(
            "<tr><td><strong>Environment</strong></td><td>{}</td></tr>\n",
            self.metadata.environment
        ));
        content.push_str(&format!(
            "<tr><td><strong>Validator Version</strong></td><td>{}</td></tr>\n",
            self.metadata.validator_version
        ));
        content.push_str("</table>\n");

        content.push_str("</body>\n</html>");

        Ok(content)
    }

    /// Generate CSV format report
    fn generate_csv(&self) -> Result<String, anyhow::Error> {
        let mut content = String::new();

        // Header
        content.push_str(
            "resultid,result_name,category,status,duration_ms,issue_count,has_critical_issues\n",
        );

        // Data rows
        for (id, result) in &self.summary.results {
            let has_critical = if result.has_critical_issues() {
                "true"
            } else {
                "false"
            };
            content.push_str(&format!(
                "{},{},{},{},{},{},{}\n",
                id,
                result.name,
                result.category,
                result.status,
                result.duration_ms,
                result.issues.len(),
                has_critical
            ));
        }

        Ok(content)
    }
}

/// Report metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReportMetadata {
    pub environment: String,
    pub validator_version: String,
    pub hostname: String,
    pub user: String,
    pub os_version: String,
    pub rust_version: String,
}

impl Default for ReportMetadata {
    fn default() -> Self {
        Self {
            environment: std::env::var("ENVIRONMENT").unwrap_or_else(|_| "unknown".to_string()),
            validator_version: env!("CARGO_PKG_VERSION").to_string(),
            hostname: gethostname::gethostname().to_string_lossy().to_string(),
            user: std::env::var("USER").unwrap_or_else(|_| "unknown".to_string()),
            os_version: os_info::get().to_string(),
            rust_version: format!("{:?}", rustc_version::version_meta()),
        }
    }
}

/// Report generator that manages report creation and output
pub struct ReportGenerator {
    output_dir: String,
}

impl ReportGenerator {
    /// Create a new report generator
    pub fn new(output_dir: String) -> Self {
        Self { output_dir }
    }

    /// Generate and save report in multiple formats
    pub async fn generate_all_formats(
        &self,
        report: &ValidationReport,
        formats: &[ReportFormat],
    ) -> Result<Vec<String>, anyhow::Error> {
        let mut output_files = Vec::new();

        // Create output directory
        tokio::fs::create_dir_all(&self.output_dir).await?;

        for format in formats {
            let content = report.generate(format)?;
            let filename = self.generate_filename(format, &report.id);
            let filepath = format!("{}/{}", self.output_dir, filename);

            tokio::fs::write(&filepath, content).await?;
            output_files.push(filepath.clone());

            log::info!("Generated {} report: {}", format, filepath);
        }

        Ok(output_files)
    }

    /// Generate filename for report
    fn generate_filename(&self, format: &ReportFormat, id: &Uuid) -> String {
        let timestamp = chrono::Utc::now().format("%Y%m%d_%H%M%S");
        let format_str = match format {
            ReportFormat::Json => "json",
            ReportFormat::Yaml => "yaml",
            ReportFormat::Markdown => "md",
            ReportFormat::Html => "html",
            ReportFormat::Csv => "csv",
        };

        format!("validation_report_{}_{}.{}", timestamp, id, format_str)
    }

    /// Send report to webhook if configured
    pub async fn send_webhook(
        &self,
        report: &ValidationReport,
        webhook_url: &str,
    ) -> Result<(), anyhow::Error> {
        let client = reqwest::Client::new();

        let payload = serde_json::json!({
            "report": report,
            "summary": {
                "version": report.version,
                "status": report.summary.overall_status,
                "success": report.is_success(),
                "critical_issues": report.has_critical_issues(),
                "statistics": report.get_statistics()
            }
        });

        let response = client.post(webhook_url).json(&payload).send().await?;

        if response.status().is_success() {
            log::info!("Report successfully sent to webhook: {}", webhook_url);
        } else {
            log::warn!("Failed to send report to webhook: {}", response.status());
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_report_generation() {
        let mut summary = ValidationSummary::new("1.0.0".to_string());
        summary.complete();

        let report = ValidationReport::from_summary(summary);

        // Test JSON generation
        let json_result = report.generate(&ReportFormat::Json);
        assert!(json_result.is_ok());

        // Test Markdown generation
        let md_result = report.generate(&ReportFormat::Markdown);
        assert!(md_result.is_ok());
    }

    #[test]
    fn test_report_generator() {
        let generator = ReportGenerator::new("test-output".to_string());

        let mut summary = ValidationSummary::new("1.0.0".to_string());
        summary.complete();

        let report = ValidationReport::from_summary(summary);

        // Test filename generation
        let filename = generator.generate_filename(&ReportFormat::Json, &report.id);
        assert!(filename.contains("validation_report_"));
        assert!(filename.ends_with(".json"));
    }
}
