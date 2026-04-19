//! Indexes markdown files (notably `CLAUDE.md` and Claude Code skills) as
//! H2/H3-segmented fragments so they can be ranked and selectively loaded by
//! progressive context loading.
//!
//! Uses `terraphim-markdown-parser` to normalize markdown, build a heading
//! tree, and chunk by headings. Each chunk becomes a `Document` whose body is
//! the section text. The search-time scorer (`TerraphimGraph`) handles
//! relevance, so this indexer intentionally emits every fragment regardless
//! of the needle — filtering at index time would blind the scorer.

use std::path::{Path, PathBuf};

use terraphim_config::Haystack;
use terraphim_markdown_parser::{
    build_heading_tree, chunk_by_headings, normalize_markdown, ContentChunk,
};
use terraphim_types::{Document, DocumentType, Index};
use tokio::fs;

use crate::Result;

use super::super::indexer::IndexMiddleware;

/// Middleware that segments markdown files at heading boundaries and emits one
/// `Document` per heading section. The `haystack.location` may be either a
/// single file or a directory (top-level `.md` files are picked up; recursion
/// is intentionally shallow to match the ripgrep indexer's behaviour).
#[derive(Default)]
pub struct ClaudeMdHaystackIndexer;

impl IndexMiddleware for ClaudeMdHaystackIndexer {
    async fn index(&self, _needle: &str, haystack: &Haystack) -> Result<Index> {
        let root = PathBuf::from(&haystack.location);
        if !root.exists() {
            log::warn!("ClaudeMd haystack path does not exist: {}", root.display());
            return Ok(Index::default());
        }

        let files = collect_markdown_files(&root).await;
        let mut index = Index::new();
        for file in files {
            match index_file(&file).await {
                Ok(docs) => {
                    for doc in docs {
                        index.insert(doc.id.clone(), doc);
                    }
                }
                Err(e) => {
                    log::warn!("ClaudeMd haystack skipping {}: {e}", file.display());
                }
            }
        }
        Ok(index)
    }
}

async fn collect_markdown_files(root: &Path) -> Vec<PathBuf> {
    if root.is_file() {
        return vec![root.to_path_buf()];
    }
    let mut out = Vec::new();
    let Ok(mut entries) = fs::read_dir(root).await else {
        return out;
    };
    while let Ok(Some(entry)) = entries.next_entry().await {
        let path = entry.path();
        if path.extension().is_some_and(|ext| ext == "md") {
            out.push(path);
        }
    }
    out
}

async fn index_file(path: &Path) -> Result<Vec<Document>> {
    let raw = fs::read_to_string(path).await?;
    let normalized = normalize_markdown(&raw)
        .map_err(|e| crate::Error::Indexation(format!("normalize_markdown: {e}")))?;
    let tree = build_heading_tree(&normalized)
        .map_err(|e| crate::Error::Indexation(format!("build_heading_tree: {e}")))?;

    let source_id = path.display().to_string();
    let chunks = chunk_by_headings(&source_id, &tree, &normalized);

    // If the file has no headings at all, fall back to a single document
    // containing the whole file — otherwise it would be silently dropped.
    if chunks.is_empty() {
        return Ok(vec![whole_file_document(&source_id, &raw)]);
    }

    Ok(chunks.into_iter().map(chunk_to_document).collect())
}

fn chunk_to_document(chunk: ContentChunk) -> Document {
    let title = first_nonempty_line(&chunk.text).unwrap_or_else(|| chunk.section_path.clone());
    Document {
        id: chunk.chunk_id.clone(),
        url: chunk.content_id.clone(),
        title,
        body: chunk.text,
        description: None,
        summarization: None,
        stub: None,
        tags: None,
        rank: None,
        source_haystack: None,
        doc_type: DocumentType::KgEntry,
        synonyms: None,
        route: None,
        priority: None,
    }
}

fn whole_file_document(source_id: &str, body: &str) -> Document {
    Document {
        id: format!("{source_id}#0"),
        url: source_id.to_string(),
        title: first_nonempty_line(body).unwrap_or_else(|| source_id.to_string()),
        body: body.to_string(),
        description: None,
        summarization: None,
        stub: None,
        tags: None,
        rank: None,
        source_haystack: None,
        doc_type: DocumentType::KgEntry,
        synonyms: None,
        route: None,
        priority: None,
    }
}

fn first_nonempty_line(text: &str) -> Option<String> {
    text.lines()
        .map(str::trim)
        .find(|l| !l.is_empty())
        .map(|l| l.chars().take(80).collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    use terraphim_config::ServiceType;

    #[tokio::test]
    async fn indexes_headings_as_fragments() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("CLAUDE.md");
        tokio::fs::write(
            &path,
            "# Project\n\nIntro.\n\n## Async Programming\n\nUse tokio for mpsc channels.\n\n## Persistence\n\nUse DeviceStorage for multi-backend caching.\n",
        )
        .await
        .unwrap();

        let haystack = Haystack {
            location: path.display().to_string(),
            service: ServiceType::ClaudeMd,
            read_only: true,
            fetch_content: false,
            atomic_server_secret: None,
            extra_parameters: Default::default(),
        };

        let idx = ClaudeMdHaystackIndexer
            .index("tokio", &haystack)
            .await
            .unwrap();

        // 3 sections: Project (root), Async Programming, Persistence.
        assert_eq!(idx.len(), 3, "expected 3 fragments, got {}", idx.len());

        let bodies: Vec<&str> = idx.values().map(|d| d.body.as_str()).collect();
        assert!(bodies.iter().any(|b| b.contains("tokio")));
        assert!(bodies.iter().any(|b| b.contains("DeviceStorage")));
    }

    #[tokio::test]
    async fn indexes_directory_of_markdown_files() {
        let dir = tempfile::tempdir().unwrap();
        tokio::fs::write(dir.path().join("a.md"), "# A\n\nAlpha body.\n")
            .await
            .unwrap();
        tokio::fs::write(dir.path().join("b.md"), "# B\n\nBeta body.\n")
            .await
            .unwrap();
        // Non-markdown should be ignored.
        tokio::fs::write(dir.path().join("c.txt"), "ignore me")
            .await
            .unwrap();

        let haystack = Haystack {
            location: dir.path().display().to_string(),
            service: ServiceType::ClaudeMd,
            read_only: true,
            fetch_content: false,
            atomic_server_secret: None,
            extra_parameters: Default::default(),
        };

        let idx = ClaudeMdHaystackIndexer.index("", &haystack).await.unwrap();

        assert_eq!(idx.len(), 2);
    }

    #[tokio::test]
    async fn missing_path_returns_empty_index() {
        let haystack = Haystack {
            location: "/nonexistent/path/CLAUDE.md".into(),
            service: ServiceType::ClaudeMd,
            read_only: true,
            fetch_content: false,
            atomic_server_secret: None,
            extra_parameters: Default::default(),
        };

        let idx = ClaudeMdHaystackIndexer.index("", &haystack).await.unwrap();

        assert!(idx.is_empty());
    }
}
