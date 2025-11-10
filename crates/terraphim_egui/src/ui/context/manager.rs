//! Context management for LLM interactions
//!
//! This module provides context management functionality for selecting
//! articles, concepts, and knowledge graph nodes to include in LLM context.

use crate::state::{AppState, ContextManager};
use eframe::egui;
use tracing::info;
use terraphim_types::Document;

pub struct ContextPanel {
    // Context panel state
    pub collapsed: bool,
}

impl ContextPanel {
    pub fn new(_state: &AppState) -> Self {
        Self { collapsed: false }
    }

    pub fn render(&mut self, ui: &mut egui::Ui, state: &AppState) {
        let context = state.get_context_manager();

        // Header
        ui.horizontal(|ui| {
            ui.label(egui::RichText::new("📋 Context").heading().strong());

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.button("Clear All").on_hover_text("Remove all items from context").clicked() {
                    state.clear_context();
                }
            });
        });

        ui.add_space(8.0);

        // Context statistics
        let doc_count = context.selected_documents.len();
        let concept_count = context.selected_concepts.len();
        let node_count = context.selected_kg_nodes.len();
        let total_items = doc_count + concept_count + node_count;

        egui::Frame::group(ui.style())
            .show(ui, |ui| {
                ui.set_min_width(200.0);
                ui.label(egui::RichText::new(format!(
                    "Items: {} (📄 {} documents, 🏷️ {} concepts, 🕸️ {} nodes)",
                    total_items, doc_count, concept_count, node_count
                ))
                .small());

                // Context size indicator
                let max_size = context.max_context_size;
                let current_size = estimate_context_size(&context);
                let size_pct = if max_size > 0 {
                    (current_size as f64 / max_size as f64 * 100.0).min(100.0)
                } else {
                    0.0
                } as f32;

                ui.add_space(4.0);
                ui.horizontal(|ui| {
                    ui.label("Context Size:", );
                    ui.label(
                        egui::RichText::new(format!("{}/{} tokens", current_size, max_size))
                            .color(if size_pct > 90.0 {
                                egui::Color32::RED
                            } else if size_pct > 70.0 {
                                egui::Color32::YELLOW
                            } else {
                                ui.visuals().weak_text_color()
                            })
                            .small(),
                    );
                });

                // Progress bar
                if max_size > 0 {
                    ui.add(
                        egui::ProgressBar::new(size_pct / 100.0)
                            .show_percentage()
                            .desired_width(200.0),
                    );
                }
            });

        ui.add_space(8.0);

        if total_items == 0 {
            ui.centered_and_justified(|ui| {
                ui.label(
                    egui::RichText::new("No items in context")
                        .weak()
                        .italics(),
                );
                ui.add_space(4.0);
                ui.label(
                    egui::RichText::new("Search for documents and click 'Add to Context'")
                        .small()
                        .weak(),
                );
            });
            return;
        }

        // Scroll area for context items
        egui::ScrollArea::vertical()
            .max_height(400.0)
            .show(ui, |ui| {
                // Document items
                for (idx, doc) in context.selected_documents.iter().enumerate() {
                    self.render_document_item(ui, doc, idx, state);
                    ui.add_space(4.0);
                }

                // Concept items
                for (idx, concept) in context.selected_concepts.iter().enumerate() {
                    self.render_concept_item(ui, concept, idx, state);
                    ui.add_space(4.0);
                }

                // Knowledge graph nodes
                for (idx, node_id) in context.selected_kg_nodes.iter().enumerate() {
                    self.render_kg_node_item(ui, node_id, idx, state);
                    ui.add_space(4.0);
                }
            });
    }

    fn render_document_item(
        &self,
        ui: &mut egui::Ui,
        doc: &Document,
        index: usize,
        state: &AppState,
    ) {
        egui::Frame::default()
            .stroke(ui.visuals().window_stroke)
            .inner_margin(8.0)
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    // Document icon and title
                    ui.label("📄");
                    ui.label(egui::RichText::new(&doc.title).strong());

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.button("✕").on_hover_text("Remove from context").clicked() {
                            state.remove_document_from_context(&doc.id);
                        }
                    });
                });

                ui.add_space(2.0);

                // URL
                ui.label(egui::RichText::new(&doc.url).small().weak().monospace());

                ui.add_space(2.0);

                // Description snippet
                if let Some(ref desc) = doc.description {
                    let snippet = if desc.len() > 100 {
                        format!("{}...", &desc[..100])
                    } else {
                        desc.clone()
                    };
                    ui.label(egui::RichText::new(snippet).small());
                } else if !doc.body.is_empty() {
                    let snippet = if doc.body.len() > 100 {
                        format!("{}...", &doc.body[..100])
                    } else {
                        doc.body.clone()
                    };
                    ui.label(egui::RichText::new(snippet).small().weak());
                }

                // Metadata
                ui.add_space(4.0);
                ui.horizontal(|ui| {
                    if let Some(ref source) = doc.source_haystack {
                        ui.label(egui::RichText::new(format!("📁 {}", source)).small().weak());
                    }
                    if let Some(ref tags) = doc.tags {
                        if !tags.is_empty() {
                            ui.add_space(8.0);
                            ui.label(
                                egui::RichText::new(format!("🏷️ {}", tags.join(", ")))
                                    .small()
                                    .weak(),
                            );
                        }
                    }
                });
            });
    }

    fn render_concept_item(
        &self,
        ui: &mut egui::Ui,
        concept: &str,
        index: usize,
        state: &AppState,
    ) {
        egui::Frame::default()
            .stroke(ui.visuals().window_stroke)
            .inner_margin(8.0)
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label("🏷️");
                    ui.label(egui::RichText::new(concept).strong());

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.button("✕").on_hover_text("Remove from context").clicked() {
                            // Remove concept
                            let mut context = state.get_context_manager();
                            context.selected_concepts.remove(index);
                        }
                    });
                });
            });
    }

    fn render_kg_node_item(
        &self,
        ui: &mut egui::Ui,
        node_id: &str,
        index: usize,
        state: &AppState,
    ) {
        egui::Frame::default()
            .stroke(ui.visuals().window_stroke)
            .inner_margin(8.0)
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label("🕸️");
                    ui.label(egui::RichText::new(node_id).strong());

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.button("✕").on_hover_text("Remove from context").clicked() {
                            // Remove node
                            let mut context = state.get_context_manager();
                            context.selected_kg_nodes.remove(index);
                        }
                    });
                });
            });
    }
}

/// Estimate context size in tokens
fn estimate_context_size(context: &ContextManager) -> usize {
    let mut total = 0;

    // Count document content
    for doc in &context.selected_documents {
        total += doc.title.len() + doc.body.len();
        if let Some(ref desc) = doc.description {
            total += desc.len();
        }
    }

    // Count concepts
    for concept in &context.selected_concepts {
        total += concept.len();
    }

    // Count nodes
    for node in &context.selected_kg_nodes {
        total += node.len();
    }

    // Rough estimate: 4 characters = 1 token
    (total / 4).max(1)
}

