//! Knowledge graph painter
//!
//! This module provides custom rendering for knowledge graph nodes and edges
//! using egui's painting capabilities.

use eframe::egui;
use std::collections::HashMap;

/// Graph node
#[derive(Debug, Clone)]
pub struct Node {
    pub id: String,
    pub label: String,
    pub x: f32,
    pub y: f32,
    pub radius: f32,
    pub color: egui::Color32,
    pub selected: bool,
}

/// Graph edge
#[derive(Debug, Clone)]
pub struct Edge {
    pub from: String,
    pub to: String,
    pub weight: f32,
}

/// Knowledge graph painter
pub struct KnowledgeGraphPainter {
    /// Graph nodes
    pub nodes: Vec<Node>,
    /// Graph edges
    pub edges: Vec<Edge>,
    /// Pan offset
    pub pan_offset: egui::Vec2,
    /// Zoom level
    pub zoom: f32,
    /// Dragging state
    pub dragging_node: Option<String>,
    /// Panning state
    pub is_panning: bool,
    /// Last mouse position
    pub last_mouse_pos: Option<egui::Pos2>,
}

impl KnowledgeGraphPainter {
    pub fn new() -> Self {
        let mut painter = Self {
            nodes: Vec::new(),
            edges: Vec::new(),
            pan_offset: egui::vec2(0.0, 0.0),
            zoom: 1.0,
            dragging_node: None,
            is_panning: false,
            last_mouse_pos: None,
        };

        // Initialize with demo data
        painter.init_demo_data();
        painter
    }

    /// Initialize with demo knowledge graph data
    fn init_demo_data(&mut self) {
        // Demo nodes
        self.nodes = vec![
            Node {
                id: "rust".to_string(),
                label: "Rust".to_string(),
                x: 0.0,
                y: 0.0,
                radius: 40.0,
                color: egui::Color32::from_rgb(250, 162, 61),
                selected: false,
            },
            Node {
                id: "egui".to_string(),
                label: "Egui".to_string(),
                x: 150.0,
                y: -50.0,
                radius: 35.0,
                color: egui::Color32::from_rgb(80, 170, 255),
                selected: false,
            },
            Node {
                id: "terraphim".to_string(),
                label: "Terraphim".to_string(),
                x: -150.0,
                y: -30.0,
                radius: 40.0,
                color: egui::Color32::from_rgb(200, 100, 220),
                selected: false,
            },
            Node {
                id: "ai".to_string(),
                label: "AI".to_string(),
                x: 80.0,
                y: 100.0,
                radius: 35.0,
                color: egui::Color32::from_rgb(100, 220, 100),
                selected: false,
            },
            Node {
                id: "search".to_string(),
                label: "Search".to_string(),
                x: -80.0,
                y: 90.0,
                radius: 30.0,
                color: egui::Color32::from_rgb(220, 180, 80),
                selected: false,
            },
            Node {
                id: "knowledge".to_string(),
                label: "Knowledge Graph".to_string(),
                x: 0.0,
                y: -150.0,
                radius: 45.0,
                color: egui::Color32::from_rgb(150, 150, 250),
                selected: false,
            },
        ];

        // Demo edges
        self.edges = vec![
            Edge {
                from: "rust".to_string(),
                to: "egui".to_string(),
                weight: 1.0,
            },
            Edge {
                from: "rust".to_string(),
                to: "terraphim".to_string(),
                weight: 1.0,
            },
            Edge {
                from: "terraphim".to_string(),
                to: "ai".to_string(),
                weight: 1.0,
            },
            Edge {
                from: "terraphim".to_string(),
                to: "search".to_string(),
                weight: 1.0,
            },
            Edge {
                from: "terraphim".to_string(),
                to: "knowledge".to_string(),
                weight: 1.0,
            },
            Edge {
                from: "ai".to_string(),
                to: "search".to_string(),
                weight: 1.0,
            },
        ];
    }

    /// Convert screen coordinates to graph coordinates
    fn screen_to_graph(&self, screen_pos: egui::Pos2, ui: &egui::Ui) -> egui::Vec2 {
        let rect = ui.available_rect_before_wrap();
        let center = rect.center();
        egui::vec2(
            (screen_pos.x - center.x - self.pan_offset.x) / self.zoom,
            (screen_pos.y - center.y - self.pan_offset.y) / self.zoom,
        )
    }

    /// Convert graph coordinates to screen coordinates
    fn graph_to_screen(&self, graph_pos: egui::Vec2, ui: &egui::Ui) -> egui::Pos2 {
        let rect = ui.available_rect_before_wrap();
        let center = rect.center();
        egui::pos2(
            center.x + graph_pos.x * self.zoom + self.pan_offset.x,
            center.y + graph_pos.y * self.zoom + self.pan_offset.y,
        )
    }

    /// Handle mouse interaction
    pub fn handle_input(&mut self, ui: &mut egui::Ui) {
        let response = ui.interact(ui.available_rect_before_wrap(), egui::Id::new("kg_canvas"), egui::Sense::click_and_drag());

        // Handle mouse press
        if response.clicked() {
            let mouse_pos = ui.input(|i| i.pointer.hover_pos().unwrap_or(egui::pos2(0.0, 0.0)));

            // Check if clicking on a node
            let mut clicked_node = None;
            for node in &self.nodes {
                let node_screen_pos = self.graph_to_screen(egui::vec2(node.x, node.y), ui);
                let dist = (mouse_pos.x - node_screen_pos.x).hypot(mouse_pos.y - node_screen_pos.y);
                if dist <= node.radius {
                    clicked_node = Some(node.id.clone());
                    break;
                }
            }

            if let Some(node_id) = clicked_node {
                // Select this node
                for node in &mut self.nodes {
                    node.selected = node.id == node_id;
                }
                self.dragging_node = Some(node_id);
            } else {
                // Start panning
                self.is_panning = true;
            }
        }

        // Handle drag
        if response.dragged() {
            let delta = response.drag_delta();
            if let Some(ref node_id) = self.dragging_node {
                // Drag node
                let graph_delta = egui::vec2(delta.x / self.zoom, delta.y / self.zoom);
                for node in &mut self.nodes {
                    if node.id == *node_id {
                        node.x += graph_delta.x;
                        node.y += graph_delta.y;
                    }
                }
            } else if self.is_panning {
                // Pan the view
                self.pan_offset += delta;
            }
        }

        // Handle mouse release
        if response.clicked() && !response.dragged() {
            // This is a click without drag, so check if it's the same position
            if let Some(last_pos) = self.last_mouse_pos {
                let mouse_pos = ui.input(|i| i.pointer.hover_pos().unwrap_or(egui::pos2(0.0, 0.0)));
                let dist = (mouse_pos.x - last_pos.x).hypot(mouse_pos.y - last_pos.y);
                if dist < 5.0 {
                    // This was a click, not a drag
                    self.dragging_node = None;
                    self.is_panning = false;
                }
            }
        }

        if response.drag_released() {
            self.dragging_node = None;
            self.is_panning = false;
        }

        self.last_mouse_pos = ui.input(|i| i.pointer.hover_pos());

        // Handle mouse wheel for zoom
        let wheel = ui.input(|i| {
            for event in &i.events {
                if let egui::Event::MouseWheel { delta, .. } = event {
                    return Some(delta.y);
                }
            }
            None
        });

        if let Some(wheel_y) = wheel {
            if wheel_y.abs() > 0.0 {
                let zoom_factor = if wheel_y > 0.0 { 1.1 } else { 0.9 };
                self.zoom = (self.zoom * zoom_factor).clamp(0.1, 5.0);
            }
        }
    }

    /// Render the knowledge graph
    pub fn paint(&mut self, ui: &mut egui::Ui) {
        let painter = ui.painter();

        // Draw edges
        for edge in &self.edges {
            let from_node = self.nodes.iter().find(|n| n.id == edge.from).unwrap();
            let to_node = self.nodes.iter().find(|n| n.id == edge.to).unwrap();

            let from_pos = self.graph_to_screen(egui::vec2(from_node.x, from_node.y), ui);
            let to_pos = self.graph_to_screen(egui::vec2(to_node.x, to_node.y), ui);

            painter.line_segment(
                [from_pos, to_pos],
                egui::Stroke::new(2.0, egui::Color32::from_gray(100)),
            );
        }

        // Draw nodes
        for node in &self.nodes {
            let pos = self.graph_to_screen(egui::vec2(node.x, node.y), ui);

            // Draw node circle
            let color = if node.selected {
                egui::Color32::from_rgb(
                    (node.color.r() as f32 * 1.2).min(255.0) as u8,
                    (node.color.g() as f32 * 1.2).min(255.0) as u8,
                    (node.color.b() as f32 * 1.2).min(255.0) as u8,
                )
            } else {
                node.color
            };

            painter.circle_filled(pos, node.radius * self.zoom, color);
            painter.circle_stroke(pos, node.radius * self.zoom, egui::Stroke::new(2.0, egui::Color32::BLACK));

            // Draw node label
            painter.text(
                pos,
                egui::Align2::CENTER_CENTER,
                &node.label,
                egui::FontId::default(),
                egui::Color32::WHITE,
            );
        }
    }

    /// Get selected node IDs
    pub fn get_selected_nodes(&self) -> Vec<String> {
        self.nodes.iter().filter(|n| n.selected).map(|n| n.id.clone()).collect()
    }
}
