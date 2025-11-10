//! Knowledge graph viewer
//!
//! This module provides knowledge graph visualization with interactive
//! node/edge rendering, pan/zoom, and node selection.

use crate::state::AppState;
use eframe::egui;
use super::painter::KnowledgeGraphPainter;

pub struct KnowledgeGraphViewer {
    /// Graph painter
    pub painter: KnowledgeGraphPainter,
    /// Show controls panel
    show_controls: bool,
    /// Filter text
    filter_text: String,
}

impl KnowledgeGraphViewer {
    pub fn new(_state: &AppState) -> Self {
        Self {
            painter: KnowledgeGraphPainter::new(),
            show_controls: true,
            filter_text: String::new(),
        }
    }

    pub fn render(&mut self, ui: &mut egui::Ui, _state: &AppState) {
        ui.label(egui::RichText::new("🕸️ Knowledge Graph").heading().strong());
        ui.add_space(8.0);

        // Controls panel
        if self.show_controls {
            egui::CollapsingHeader::new("Controls")
                .default_open(true)
                .show(ui, |ui| {
                    self.render_controls(ui);
                });
            ui.add_space(8.0);
        }

        // Graph canvas
        egui::Frame::group(ui.style())
            .show(ui, |ui| {
                let available_size = ui.available_size();
                ui.allocate_space(available_size);

                // Handle input first
                self.painter.handle_input(ui);

                // Paint the graph
                self.painter.paint(ui);

                // Draw overlay text for instructions
                self.render_instructions(ui);
            });

        // Selected nodes panel
        self.render_selected_nodes(ui);
    }

    fn render_controls(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.label("Zoom:");
            let zoom_percentage = (self.painter.zoom * 100.0) as i32;
            ui.label(format!("{}%", zoom_percentage));

            if ui.button("🔍 Zoom In").clicked() {
                self.painter.zoom = (self.painter.zoom * 1.1).clamp(0.1, 5.0);
            }

            if ui.button("🔎 Zoom Out").clicked() {
                self.painter.zoom = (self.painter.zoom * 0.9).clamp(0.1, 5.0);
            }

            if ui.button("🏠 Reset View").clicked() {
                self.painter.pan_offset = egui::vec2(0.0, 0.0);
                self.painter.zoom = 1.0;
            }
        });

        ui.add_space(4.0);

        ui.horizontal(|ui| {
            ui.label("Filter:");
            ui.text_edit_singleline(&mut self.filter_text);
        });

        ui.add_space(4.0);

        ui.horizontal(|ui| {
            if ui.button("✨ Auto Layout").clicked() {
                self.auto_layout();
            }

            if ui.button("🎨 Randomize Colors").clicked() {
                self.randomize_colors();
            }
        });
    }

    fn render_instructions(&self, ui: &mut egui::Ui) {
        let instructions = "Click and drag to pan • Scroll to zoom • Click nodes to select • Drag nodes to move";

        let rect = ui.available_rect_before_wrap();
        let pos = egui::pos2(rect.left() + 10.0, rect.bottom() - 20.0);

        ui.ctx().debug_painter().text(
            pos,
            egui::Align2::LEFT_BOTTOM,
            instructions,
            egui::FontId::default(),
            egui::Color32::from_gray(150),
        );
    }

    fn render_selected_nodes(&self, ui: &mut egui::Ui) {
        let selected = self.painter.get_selected_nodes();

        if !selected.is_empty() {
            ui.add_space(8.0);
            egui::Frame::group(ui.style())
                .show(ui, |ui| {
                    ui.label(egui::RichText::new("Selected Nodes:").strong());
                    ui.add_space(4.0);

                    for node_id in selected {
                        ui.horizontal(|ui| {
                            ui.label("●");
                            ui.label(node_id);
                        });
                    }
                });
        }
    }

    /// Auto-layout nodes in a circle
    fn auto_layout(&mut self) {
        let node_count = self.painter.nodes.len();
        if node_count == 0 {
            return;
        }

        let radius = 200.0;
        for (i, node) in self.painter.nodes.iter_mut().enumerate() {
            let angle = (i as f32 / node_count as f32) * std::f32::consts::TAU;
            node.x = radius * angle.cos();
            node.y = radius * angle.sin();
        }
    }

    /// Randomize node colors
    fn randomize_colors(&mut self) {
        use rand::Rng;
        let mut rng = rand::thread_rng();

        for node in &mut self.painter.nodes {
            node.color = egui::Color32::from_rgb(
                rng.gen_range(50..=255),
                rng.gen_range(50..=255),
                rng.gen_range(50..=255),
            );
        }
    }
}
