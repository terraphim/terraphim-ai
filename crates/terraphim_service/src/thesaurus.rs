//! Thesaurus capability for `TerraphimService` (knowledge-graph thesaurus
//! loading and building). Split from lib.rs as part of the Gitea #1910
//! god-file decomposition; behaviour unchanged. Methods remain on
//! `TerraphimService` via an additional impl block, so the public API is identical.

use ahash::AHashMap;
use terraphim_automata::builder::{Logseq, ThesaurusBuilder, compute_kg_source_hash};
use terraphim_automata::load_thesaurus;
use terraphim_config::ConfigState;
use terraphim_middleware::thesaurus::build_thesaurus_from_haystack;
use terraphim_persistence::Persistable;
use terraphim_rolegraph::{RoleGraph, RoleGraphSync};
use terraphim_types::{RoleName, SearchQuery, Thesaurus};

use super::{Result, ServiceError, TerraphimService};

impl TerraphimService {
    /// Build a thesaurus from the haystack and update the knowledge graph automata URL
    pub(crate) async fn build_thesaurus(&mut self, search_query: &SearchQuery) -> Result<()> {
        Ok(build_thesaurus_from_haystack(&mut self.config_state, search_query).await?)
    }
    /// load thesaurus from config object and if absent make sure it's loaded from automata_url
    pub async fn ensure_thesaurus_loaded(&mut self, role_name: &RoleName) -> Result<Thesaurus> {
        async fn load_thesaurus_from_automata_path(
            config_state: &ConfigState,
            role_name: &RoleName,
            rolegraphs: &mut AHashMap<RoleName, RoleGraphSync>,
        ) -> Result<Thesaurus> {
            // CRITICAL: clone the role out, then drop the config lock before
            // doing I/O. Holding the lock across the network/disk operations
            // below blocks every other endpoint that touches /config (e.g.
            // GET /config from `roles select`, `roles list`, etc.) for the
            // duration of the thesaurus load + persistence + RoleGraph build.
            let role = {
                let config = config_state.config.lock().await;
                let Some(role) = config.roles.get(role_name).cloned() else {
                    return Err(ServiceError::Config(format!(
                        "Role '{}' not found in config",
                        role_name
                    )));
                };
                role
            };
            if let Some(kg) = &role.kg {
                if let Some(automata_path) = &kg.automata_path {
                    log::info!("Loading Role `{}` - URL: {:?}", role_name, automata_path);

                    // Try to load from automata path first
                    match load_thesaurus(automata_path).await {
                        Ok(mut thesaurus) => {
                            log::info!("Successfully loaded thesaurus from automata path");

                            // Save thesaurus to persistence to ensure it's available for future loads
                            match thesaurus.save().await {
                                Ok(_) => {
                                    log::info!(
                                        "Thesaurus for role `{}` saved to persistence",
                                        role_name
                                    );
                                    // Reload from persistence to get canonical version
                                    match thesaurus.load().await {
                                        Ok(persisted_thesaurus) => {
                                            thesaurus = persisted_thesaurus;
                                            log::debug!("Reloaded thesaurus from persistence");
                                        }
                                        Err(e) => {
                                            log::warn!(
                                                "Failed to reload thesaurus from persistence, using in-memory version: {:?}",
                                                e
                                            );
                                        }
                                    }
                                }
                                Err(e) => {
                                    log::warn!("Failed to save thesaurus to persistence: {:?}", e);
                                }
                            }

                            let rolegraph =
                                RoleGraph::new(role_name.clone(), thesaurus.clone()).await;
                            match rolegraph {
                                Ok(rolegraph) => {
                                    let rolegraph_value = RoleGraphSync::from(rolegraph);
                                    rolegraphs.insert(role_name.clone(), rolegraph_value);
                                }
                                Err(e) => {
                                    log::error!("Failed to update role and thesaurus: {:?}", e)
                                }
                            }
                            Ok(thesaurus)
                        }
                        Err(e) => {
                            log::warn!("Failed to load thesaurus from automata path: {:?}", e);
                            // Fallback to building from local KG if available
                            if let Some(kg_local) = &kg.knowledge_graph_local {
                                log::info!(
                                    "Fallback: building thesaurus from local KG for role {}",
                                    role_name
                                );
                                let logseq_builder = Logseq::default();
                                match logseq_builder
                                    .build(
                                        role_name.as_lowercase().to_string(),
                                        kg_local.path.clone(),
                                    )
                                    .await
                                {
                                    Ok(mut thesaurus) => {
                                        // Save thesaurus to persistence to ensure it's available for future loads
                                        match thesaurus.save().await {
                                            Ok(_) => {
                                                log::info!(
                                                    "Fallback thesaurus for role `{}` saved to persistence",
                                                    role_name
                                                );
                                                // Reload from persistence to get canonical version
                                                match thesaurus.load().await {
                                                    Ok(persisted_thesaurus) => {
                                                        thesaurus = persisted_thesaurus;
                                                        log::debug!(
                                                            "Reloaded fallback thesaurus from persistence"
                                                        );
                                                    }
                                                    Err(e) => {
                                                        log::warn!(
                                                            "Failed to reload fallback thesaurus from persistence, using in-memory version: {:?}",
                                                            e
                                                        );
                                                    }
                                                }
                                            }
                                            Err(e) => {
                                                log::warn!(
                                                    "Failed to save fallback thesaurus to persistence: {:?}",
                                                    e
                                                );
                                            }
                                        }

                                        let rolegraph =
                                            RoleGraph::new(role_name.clone(), thesaurus.clone())
                                                .await;
                                        match rolegraph {
                                            Ok(rolegraph) => {
                                                let rolegraph_value =
                                                    RoleGraphSync::from(rolegraph);
                                                rolegraphs
                                                    .insert(role_name.clone(), rolegraph_value);
                                            }
                                            Err(e) => log::error!(
                                                "Failed to update role and thesaurus: {:?}",
                                                e
                                            ),
                                        }

                                        Ok(thesaurus)
                                    }
                                    Err(e) => {
                                        // Check if error is "file not found" (expected for optional files)
                                        // and downgrade log level from ERROR to DEBUG
                                        let is_file_not_found =
                                            e.to_string().contains("file not found")
                                                || e.to_string().contains("not found:");

                                        if is_file_not_found {
                                            log::debug!(
                                                "Failed to build thesaurus from local KG (optional file not found) for role {}: {:?}",
                                                role_name,
                                                e
                                            );
                                        } else {
                                            log::error!(
                                                "Failed to build thesaurus from local KG for role {}: {:?}",
                                                role_name,
                                                e
                                            );
                                        }
                                        Err(ServiceError::Config(
                                            "Failed to load or build thesaurus".into(),
                                        ))
                                    }
                                }
                            } else {
                                log::warn!(
                                    "No fallback available for role {}: no local KG path configured, returning empty thesaurus",
                                    role_name
                                );
                                Ok(Thesaurus::new(role_name.as_lowercase().to_string()))
                            }
                        }
                    }
                } else if let Some(kg_local) = &kg.knowledge_graph_local {
                    // Build thesaurus from local KG
                    log::info!(
                        "Role {} has no automata_path, building thesaurus from local KG files at {:?}",
                        role_name,
                        kg_local.path
                    );
                    let logseq_builder = Logseq::default();
                    match logseq_builder
                        .build(role_name.as_lowercase().to_string(), kg_local.path.clone())
                        .await
                    {
                        Ok(mut thesaurus) => {
                            log::info!(
                                "Successfully built thesaurus from local KG for role {}",
                                role_name
                            );

                            // Save thesaurus to persistence to ensure it's available for future loads
                            match thesaurus.save().await {
                                Ok(_) => {
                                    log::info!(
                                        "Local KG thesaurus for role `{}` saved to persistence",
                                        role_name
                                    );
                                    // Reload from persistence to get canonical version
                                    match thesaurus.load().await {
                                        Ok(persisted_thesaurus) => {
                                            log::info!(
                                                "Reloaded local KG thesaurus from persistence: {} entries",
                                                persisted_thesaurus.len()
                                            );
                                            thesaurus = persisted_thesaurus;
                                        }
                                        Err(e) => {
                                            log::warn!(
                                                "Failed to reload local KG thesaurus from persistence, using in-memory version: {:?}",
                                                e
                                            );
                                        }
                                    }
                                }
                                Err(e) => {
                                    log::warn!(
                                        "Failed to save local KG thesaurus to persistence: {:?}",
                                        e
                                    );
                                }
                            }

                            let rolegraph =
                                RoleGraph::new(role_name.clone(), thesaurus.clone()).await;
                            match rolegraph {
                                Ok(rolegraph) => {
                                    let rolegraph_value = RoleGraphSync::from(rolegraph);
                                    rolegraphs.insert(role_name.clone(), rolegraph_value);
                                }
                                Err(e) => {
                                    log::error!("Failed to update role and thesaurus: {:?}", e)
                                }
                            }

                            Ok(thesaurus)
                        }
                        Err(e) => {
                            // Check if error is "file not found" (expected for optional files)
                            // and downgrade log level from ERROR to DEBUG
                            let is_file_not_found = e.to_string().contains("file not found");

                            if is_file_not_found {
                                log::debug!(
                                    "Failed to build thesaurus from local KG (optional file not found) for role {}: {:?}",
                                    role_name,
                                    e
                                );
                            } else {
                                log::error!(
                                    "Failed to build thesaurus from local KG for role {}: {:?}",
                                    role_name,
                                    e
                                );
                            }
                            Err(ServiceError::Config(format!(
                                "Failed to build thesaurus from local KG for role {}: {}",
                                role_name, e
                            )))
                        }
                    }
                } else {
                    log::warn!(
                        "Role {} is configured for TerraphimGraph but has neither automata_path nor knowledge_graph_local defined.",
                        role_name
                    );
                    if let Some(kg_local) = &kg.knowledge_graph_local {
                        // Build thesaurus from local KG files during startup
                        log::info!(
                            "Building thesaurus from local KG files for role {} at {:?}",
                            role_name,
                            kg_local.path
                        );
                        let logseq_builder = Logseq::default();
                        match logseq_builder
                            .build(role_name.as_lowercase().to_string(), kg_local.path.clone())
                            .await
                        {
                            Ok(mut thesaurus) => {
                                log::info!(
                                    "Successfully built thesaurus from local KG for role {}",
                                    role_name
                                );

                                // Save thesaurus to persistence to ensure it's available for future loads
                                match thesaurus.save().await {
                                    Ok(_) => {
                                        log::info!(
                                            "No-automata thesaurus for role `{}` saved to persistence",
                                            role_name
                                        );
                                        // Reload from persistence to get canonical version
                                        match thesaurus.load().await {
                                            Ok(persisted_thesaurus) => {
                                                thesaurus = persisted_thesaurus;
                                                log::debug!(
                                                    "Reloaded no-automata thesaurus from persistence"
                                                );
                                            }
                                            Err(e) => {
                                                log::warn!(
                                                    "Failed to reload no-automata thesaurus from persistence, using in-memory version: {:?}",
                                                    e
                                                );
                                            }
                                        }
                                    }
                                    Err(e) => {
                                        log::warn!(
                                            "Failed to save no-automata thesaurus to persistence: {:?}",
                                            e
                                        );
                                    }
                                }

                                let rolegraph =
                                    RoleGraph::new(role_name.clone(), thesaurus.clone()).await;
                                match rolegraph {
                                    Ok(rolegraph) => {
                                        let rolegraph_value = RoleGraphSync::from(rolegraph);
                                        rolegraphs.insert(role_name.clone(), rolegraph_value);
                                    }
                                    Err(e) => {
                                        // Check if error is "file not found" (expected for optional files)
                                        // and downgrade log level from ERROR to DEBUG
                                        let is_file_not_found =
                                            e.to_string().contains("file not found");

                                        if is_file_not_found {
                                            log::debug!(
                                                "Failed to update role and thesaurus (optional file not found): {:?}",
                                                e
                                            );
                                        } else {
                                            log::error!(
                                                "Failed to update role and thesaurus: {:?}",
                                                e
                                            );
                                        }
                                    }
                                }

                                Ok(thesaurus)
                            }
                            Err(e) => {
                                log::error!(
                                    "Failed to build thesaurus from local KG for role {}: {:?}",
                                    role_name,
                                    e
                                );
                                Err(ServiceError::Config(
                                    "Failed to build thesaurus from local KG".into(),
                                ))
                            }
                        }
                    } else {
                        log::debug!(
                            "Role '{}' has no local KG path, returning empty thesaurus",
                            role_name
                        );
                        Ok(Thesaurus::new(role_name.as_lowercase().to_string()))
                    }
                }
            } else {
                log::debug!("Role '{}' has no knowledge graph configured", role_name);
                Err(ServiceError::Config(format!(
                    "Knowledge graph not configured for role '{}'",
                    role_name
                )))
            }
        }

        log::debug!("Loading thesaurus for role: {}", role_name);
        log::debug!("Role keys {:?}", self.config_state.roles.keys());

        if let Some(rolegraph_value) = self.config_state.roles.get(role_name) {
            let thesaurus_result = rolegraph_value.lock().await.thesaurus.clone().load().await;
            match thesaurus_result {
                Ok(thesaurus) => {
                    log::debug!("Thesaurus loaded: {:?}", thesaurus);
                    log::info!("Rolegraph loaded: for role name {:?}", role_name);

                    // Check if the cached thesaurus is stale by comparing source hashes
                    let is_stale = if let Some(ref cached_hash) = thesaurus.source_hash {
                        let role = {
                            let config = self.config_state.config.lock().await;
                            config.roles.get(role_name).cloned()
                        };
                        if let Some(role) = role {
                            if let Some(ref kg) = role.kg {
                                if let Some(ref kg_local) = kg.knowledge_graph_local {
                                    match compute_kg_source_hash(&kg_local.path) {
                                        Ok(Some(current_hash)) => {
                                            let stale = current_hash != *cached_hash;
                                            if stale {
                                                log::info!(
                                                    "Thesaurus cache stale for role '{}': hash mismatch (cached {} != current {})",
                                                    role_name,
                                                    cached_hash,
                                                    current_hash
                                                );
                                            }
                                            stale
                                        }
                                        Ok(None) => {
                                            log::debug!(
                                                "No markdown files found in KG path {:?}",
                                                kg_local.path
                                            );
                                            false
                                        }
                                        Err(e) => {
                                            log::warn!(
                                                "Failed to compute source hash for role '{}': {}",
                                                role_name,
                                                e
                                            );
                                            false
                                        }
                                    }
                                } else {
                                    false
                                }
                            } else {
                                false
                            }
                        } else {
                            false
                        }
                    } else {
                        log::debug!(
                            "No source_hash in cached thesaurus for role '{}'",
                            role_name
                        );
                        false
                    };

                    if is_stale {
                        let mut rolegraphs = self.config_state.roles.clone();
                        let result = load_thesaurus_from_automata_path(
                            &self.config_state,
                            role_name,
                            &mut rolegraphs,
                        )
                        .await;

                        if result.is_ok() {
                            if let Some(updated_rolegraph) = rolegraphs.get(role_name) {
                                self.config_state
                                    .roles
                                    .insert(role_name.clone(), updated_rolegraph.clone());
                                log::info!(
                                    "Updated config_state with rebuilt rolegraph for role: {}",
                                    role_name
                                );
                            }
                        }
                        result
                    } else {
                        Ok(thesaurus)
                    }
                }
                Err(e) => {
                    // Check if error is "file not found" (expected for optional files)
                    // and downgrade log level from ERROR to DEBUG
                    let is_file_not_found = e.to_string().contains("file not found")
                        || e.to_string().contains("not found:");

                    if is_file_not_found {
                        log::debug!("Thesaurus file not found (optional): {:?}", e);
                    } else {
                        log::error!("Failed to load thesaurus: {:?}", e);
                    }
                    // Try to build thesaurus from KG and update the config_state directly
                    let mut rolegraphs = self.config_state.roles.clone();
                    let result = load_thesaurus_from_automata_path(
                        &self.config_state,
                        role_name,
                        &mut rolegraphs,
                    )
                    .await;

                    // Update the actual config_state with the new rolegraph
                    if result.is_ok() {
                        if let Some(updated_rolegraph) = rolegraphs.get(role_name) {
                            self.config_state
                                .roles
                                .insert(role_name.clone(), updated_rolegraph.clone());
                            log::info!(
                                "Updated config_state with new rolegraph for role: {}",
                                role_name
                            );
                        }
                    }

                    result
                }
            }
        } else {
            // Role not found, try to build from KG
            let mut rolegraphs = self.config_state.roles.clone();
            let result =
                load_thesaurus_from_automata_path(&self.config_state, role_name, &mut rolegraphs)
                    .await;

            // Update the actual config_state with the new rolegraph
            if result.is_ok() {
                if let Some(new_rolegraph) = rolegraphs.get(role_name) {
                    self.config_state
                        .roles
                        .insert(role_name.clone(), new_rolegraph.clone());
                    log::info!(
                        "Added new rolegraph to config_state for role: {}",
                        role_name
                    );
                }
            }

            result
        }
    }
}
