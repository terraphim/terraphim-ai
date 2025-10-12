use serde::Deserialize;
use std::path::Path;

#[derive(Debug, Deserialize)]
pub struct TaxonomyFunction {
    pub id: String,
    pub name: String,
    pub description: String,
    pub subfunctions: Vec<SubFunction>,
    #[serde(default)]
    pub classification: Option<ScctClassification>,
}

#[derive(Debug, Deserialize)]
pub struct SubFunction {
    pub name: String,
    pub outputs: Vec<String>,
}

#[derive(Debug, Deserialize)]
pub struct ScctClassification {
    #[serde(rename = "issueTypes")]
    pub issue_types: Vec<String>,
    #[serde(rename = "responsibilityAttribution")]
    pub responsibility_attribution: Vec<String>,
}

pub async fn load_truthforge_taxonomy(
    json_path: impl AsRef<Path>,
) -> anyhow::Result<Vec<TaxonomyFunction>> {
    let json_str = tokio::fs::read_to_string(json_path).await?;
    let functions: Vec<TaxonomyFunction> = serde_json::from_str(&json_str)?;
    Ok(functions)
}
