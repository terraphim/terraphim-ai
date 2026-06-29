use anyhow::Result;
use std::collections::HashMap;
use std::path::PathBuf;
use walkdir::WalkDir;

/// Knowledge Graph integration for semantic module labeling
pub struct KnowledgeGraph {
    /// Map of term -> domain concept
    concepts: HashMap<String, Concept>,
    /// KG source directory
    #[allow(dead_code)]
    kg_path: PathBuf,
}

#[derive(Debug, Clone)]
pub struct Concept {
    pub name: String,
    #[allow(dead_code)]
    pub description: String,
    pub synonyms: Vec<String>,
    #[allow(dead_code)]
    pub related_concepts: Vec<String>,
    #[allow(dead_code)]
    pub category: String,
}

impl KnowledgeGraph {
    pub fn new(kg_path: PathBuf) -> Self {
        Self {
            concepts: HashMap::new(),
            kg_path,
        }
    }

    pub fn concept_count(&self) -> usize {
        self.concepts.len()
    }

    /// Load knowledge graph from ~/.config/terraphim/kg/
    pub fn load_default() -> Result<Self> {
        let kg_path = dirs::home_dir()
            .unwrap_or_default()
            .join(".config/terraphim/kg");

        let mut kg = Self::new(kg_path.clone());
        kg.load_from_directory(&kg_path)?;
        Ok(kg)
    }

    /// Load all markdown files from KG directory
    pub fn load_from_directory(&mut self, path: &PathBuf) -> Result<()> {
        if !path.exists() {
            return Ok(());
        }

        for entry in WalkDir::new(path)
            .follow_links(false)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            let path = entry.path();
            if path.extension().is_some_and(|ext| ext == "md")
                && let Ok(content) = std::fs::read_to_string(path)
            {
                let concept = self.parse_concept_file(path, &content);
                self.concepts.insert(concept.name.clone(), concept);
            }
        }

        Ok(())
    }

    fn parse_concept_file(&self, path: &std::path::Path, content: &str) -> Concept {
        let name = path
            .file_stem()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();

        let mut description = String::new();
        let mut synonyms = Vec::new();
        let mut related = Vec::new();
        let mut category = "general".to_string();

        for line in content.lines() {
            let line = line.trim();

            if line.starts_with("synonyms::") {
                synonyms = line
                    .trim_start_matches("synonyms::")
                    .split(',')
                    .map(|s| s.trim().to_lowercase())
                    .collect();
            } else if line.starts_with("## Related Concepts") {
                category = "related".to_string();
            } else if line.starts_with("-") && category == "related" {
                related.push(line.trim_start_matches("-").trim().to_string());
            } else if !line.is_empty() && !line.starts_with("#") {
                description.push_str(line);
                description.push(' ');
            }
        }

        Concept {
            name: name.clone(),
            description: description.trim().to_string(),
            synonyms,
            related_concepts: related,
            category,
        }
    }

    /// Match a module path against KG concepts
    pub fn match_module(&self, module_path: &str) -> Vec<&Concept> {
        let mut matches = Vec::new();
        let module_lower = module_path.to_lowercase();

        for concept in self.concepts.values() {
            // Check if concept name appears in module path
            if module_lower.contains(&concept.name.to_lowercase()) {
                matches.push(concept);
                continue;
            }

            // Check synonyms
            for synonym in &concept.synonyms {
                if module_lower.contains(synonym) {
                    matches.push(concept);
                    break;
                }
            }
        }

        matches
    }

    /// Get domain category for a module
    pub fn get_module_category(&self, module_path: &str) -> String {
        let matches = self.match_module(module_path);

        if matches.is_empty() {
            return "uncategorized".to_string();
        }

        // Return the first matched concept's name as category
        matches[0].name.clone()
    }

    /// Group modules by domain concept
    pub fn group_by_concept(&self, modules: &[String]) -> HashMap<String, Vec<String>> {
        let mut groups: HashMap<String, Vec<String>> = HashMap::new();

        for module in modules {
            let category = self.get_module_category(module);
            groups.entry(category).or_default().push(module.clone());
        }

        groups
    }

    /// Check if two modules are semantically related
    #[allow(dead_code)]
    pub fn are_related(&self, module_a: &str, module_b: &str) -> bool {
        let concepts_a = self.match_module(module_a);
        let concepts_b = self.match_module(module_b);

        // Check if they share any concepts
        for ca in &concepts_a {
            for cb in &concepts_b {
                if ca.name == cb.name {
                    return true;
                }
                // Check related concepts
                if ca.related_concepts.contains(&cb.name) {
                    return true;
                }
            }
        }

        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::TempDir;

    #[test]
    fn test_new_creates_empty_graph() {
        let temp_dir = TempDir::new().unwrap();
        let kg = KnowledgeGraph::new(temp_dir.path().to_path_buf());
        assert_eq!(kg.concept_count(), 0);
    }

    #[test]
    fn test_knowledge_graph_loading() {
        let temp_dir = TempDir::new().unwrap();
        let kg_dir = temp_dir.path().join("kg");
        std::fs::create_dir(&kg_dir).unwrap();

        // Create a test concept file
        let mut concept_file = std::fs::File::create(kg_dir.join("Authentication.md")).unwrap();
        writeln!(concept_file, "# Authentication").unwrap();
        writeln!(concept_file).unwrap();
        writeln!(concept_file, "Authentication and authorization concepts").unwrap();
        writeln!(concept_file).unwrap();
        writeln!(concept_file, "synonyms:: auth, login, identity").unwrap();
        writeln!(concept_file).unwrap();
        writeln!(concept_file, "## Related Concepts").unwrap();
        writeln!(concept_file, "- Security").unwrap();
        writeln!(concept_file, "- Identity").unwrap();

        let mut kg = KnowledgeGraph::new(kg_dir.clone());
        kg.load_from_directory(&kg_dir).unwrap();

        assert_eq!(kg.concepts.len(), 1);
        assert!(kg.concepts.contains_key("Authentication"));
    }

    #[test]
    fn test_module_matching() {
        let temp_dir = TempDir::new().unwrap();
        let kg_dir = temp_dir.path().join("kg");
        std::fs::create_dir(&kg_dir).unwrap();

        let mut concept_file = std::fs::File::create(kg_dir.join("Authentication.md")).unwrap();
        writeln!(concept_file, "# Authentication").unwrap();
        writeln!(concept_file, "synonyms:: auth, login").unwrap();

        let mut kg = KnowledgeGraph::new(kg_dir.clone());
        kg.load_from_directory(&kg_dir).unwrap();

        let matches = kg.match_module("terraphim_service::auth_handler");
        assert_eq!(matches.len(), 1);

        let matches = kg.match_module("terraphim_service::login");
        assert_eq!(matches.len(), 1);

        let matches = kg.match_module("terraphim_service::unrelated");
        assert_eq!(matches.len(), 0);
    }

    #[test]
    fn test_semantic_relationship() {
        let temp_dir = TempDir::new().unwrap();
        let kg_dir = temp_dir.path().join("kg");
        std::fs::create_dir(&kg_dir).unwrap();

        let mut auth_file = std::fs::File::create(kg_dir.join("Authentication.md")).unwrap();
        writeln!(auth_file, "# Authentication").unwrap();
        writeln!(auth_file, "synonyms:: auth").unwrap();
        writeln!(auth_file, "## Related Concepts").unwrap();
        writeln!(auth_file, "- Security").unwrap();

        let mut sec_file = std::fs::File::create(kg_dir.join("Security.md")).unwrap();
        writeln!(sec_file, "# Security").unwrap();
        writeln!(sec_file, "synonyms:: security").unwrap();

        let mut kg = KnowledgeGraph::new(kg_dir.clone());
        kg.load_from_directory(&kg_dir).unwrap();

        assert!(kg.are_related("auth_module", "security_handler"));
        assert!(!kg.are_related("auth_module", "ui_component"));
    }

    #[test]
    fn test_group_by_concept_buckets_modules() {
        let temp_dir = TempDir::new().unwrap();
        let kg_dir = temp_dir.path().join("kg");
        std::fs::create_dir(&kg_dir).unwrap();

        let mut auth_file = std::fs::File::create(kg_dir.join("Authentication.md")).unwrap();
        writeln!(auth_file, "# Authentication").unwrap();
        writeln!(auth_file, "synonyms:: auth, login").unwrap();

        let mut kg = KnowledgeGraph::new(kg_dir.clone());
        kg.load_from_directory(&kg_dir).unwrap();

        let modules = vec![
            "service::auth_handler".to_string(),
            "service::login_manager".to_string(),
            "service::unrelated_utils".to_string(),
        ];
        let groups = kg.group_by_concept(&modules);

        assert!(
            groups.contains_key("Authentication"),
            "auth modules must be grouped"
        );
        assert_eq!(groups["Authentication"].len(), 2);
        assert!(
            groups.contains_key("uncategorized"),
            "unmatched modules go to uncategorized"
        );
        assert_eq!(groups["uncategorized"].len(), 1);
    }
}
