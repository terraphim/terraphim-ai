//! Logseq is a knowledge graph that uses Markdown files to store notes. This
//! module provides a middleware for creating a Thesaurus from a Logseq
//! haystack.
//!
//! Example:
//!
//! If we parse a file named `path/to/concept.md` with the following content:
//!
//! ```markdown
//! synonyms:: foo, bar, baz
//! ```
//!
//! Then the thesaurus will contain the following entries:
//!
//! ```rust
//! use terraphim_types::{Thesaurus, Concept, NormalizedTerm};
//! let concept = Concept::new("concept".into());
//! let nterm = NormalizedTerm::new(concept.id, concept.value.clone());
//! let mut thesaurus = Thesaurus::new("Engineer".to_string());
//! thesaurus.insert(concept.value.clone(), nterm.clone());
//! thesaurus.insert("foo".to_string().into(),nterm.clone());
//! thesaurus.insert("bar".to_string().to_string().into(), nterm.clone());
//! thesaurus.insert("baz".to_string().into(), nterm.clone());
//! ```
//! The logic as follows: if you ask for concept by name you get concept, if you ask (get) for any of the synonyms you will get concept with id,
//! its pre-computed reverse tree traversal - any of the synonyms (leaf) maps into the concepts (root)

pub use terraphim_automata::builder::{Logseq, ThesaurusBuilder};
use terraphim_config::ConfigState;
use terraphim_config::Role;
use terraphim_persistence::Persistable;
use terraphim_rolegraph::{RoleGraph, RoleGraphSync};
use terraphim_types::SearchQuery;
use terraphim_types::{RoleName, Thesaurus};

use crate::Result;
use std::path::PathBuf;

pub async fn build_thesaurus_from_haystack(
    config_state: &mut ConfigState,
    search_query: &SearchQuery,
) -> Result<()> {
    // build thesaurus from haystack or load from remote
    // FIXME: introduce LRU cache for locally build thesaurus via persistance crate
    log::debug!("Building thesaurus from haystack");
    let config = config_state.config.lock().await.clone();
    let roles = config.roles.clone();
    let default_role = config.default_role.clone();
    let role_name = search_query.role.clone().unwrap_or_default();
    log::debug!("Role name: {}", role_name);
    let role: &mut Role = &mut roles
        .get(&role_name)
        .unwrap_or(&roles[&default_role])
        .to_owned();
    log::debug!("Role: {:?}", role);
    for haystack in &role.haystacks {
        log::debug!("Updating thesaurus for haystack: {:?}", haystack);

        let logseq = Logseq::default();
        let mut thesaurus: Thesaurus = logseq
            .build(
                role_name.as_lowercase().to_string(),
                PathBuf::from(&haystack.location),
            )
            .await?;
        match thesaurus.save().await {
            Ok(_) => {
                log::info!("Thesaurus for role `{}` saved to persistence", role_name);
                // We reload the thesaurus from persistence to ensure we are using the
                // canonical, persisted version going forward.
                thesaurus = thesaurus.load().await?;
            }
            Err(e) => log::error!("Failed to save thesaurus: {:?}", e),
        }

        log::debug!("Make sure thesaurus updated in a role {}", role_name);

        update_thesaurus(config_state, &role_name, thesaurus).await?;
    }
    Ok(())
}

async fn update_thesaurus(
    config_state: &mut ConfigState,
    role_name: &RoleName,
    thesaurus: Thesaurus,
) -> Result<()> {
    log::debug!("Updating thesaurus for role: {}", role_name);
    let mut rolegraphs = config_state.roles.clone();
    let rolegraph = RoleGraph::new(role_name.clone(), thesaurus).await;
    match rolegraph {
        Ok(rolegraph) => {
            let rolegraph_value = RoleGraphSync::from(rolegraph);
            rolegraphs.insert(role_name.clone(), rolegraph_value);
        }
        Err(e) => log::error!("Failed to update role and thesaurus: {:?}", e),
    }

    Ok(())
}
