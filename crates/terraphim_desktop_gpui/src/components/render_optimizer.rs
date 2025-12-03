/// GPUI Rendering Optimization System
///
/// This module provides advanced rendering optimizations for GPUI applications,
/// including render batching, dirty region tracking, and GPU acceleration.
///
/// Key Features:
/// - Intelligent render batching and merging
/// - Dirty region optimization
/// - Render caching and memoization
/// - GPU-accelerated compositing
/// - Frame rate optimization
/// - Hierarchical Z-ordering

use gpui::*;
use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant};
use parking_lot::Mutex;
use anyhow::Result;

/// Render optimizer configuration
#[derive(Debug, Clone)]
pub struct RenderOptimizerConfig {
    /// Enable render batching
    pub enable_batching: bool,
    /// Batch size limit
    pub max_batch_size: usize,
    /// Enable dirty region tracking
    pub enable_dirty_regions: bool,
    /// Minimum dirty region size to track
    pub min_dirty_region_size: Size<Pixels>,
    /// Enable render caching
    pub enable_caching: bool,
    /// Cache size in pixels
    pub cache_size_pixels: u64,
    /// Enable GPU acceleration
    pub enable_gpu: bool,
    /// Target frame rate
    pub target_frame_rate: f32,
    /// Enable frame skipping under load
    pub enable_frame_skipping: bool,
    /// Frame skip threshold (ms)
    pub frame_skip_threshold: Duration,
    /// Z-ordering optimization
    pub enable_z_ordering: bool,
}

impl Default for RenderOptimizerConfig {
    fn default() -> Self {
        Self {
            enable_batching: true,
            max_batch_size: 100,
            enable_dirty_regions: true,
            min_dirty_region_size: Size::new(px(1.0), px(1.0)),
            enable_caching: true,
            cache_size_pixels: 1920 * 1080 * 4, // 4K at 32bpp
            enable_gpu: true,
            target_frame_rate: 60.0,
            enable_frame_skipping: true,
            frame_skip_threshold: Duration::from_millis(16),
            enable_z_ordering: true,
        }
    }
}

/// Render node type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RenderNodeType {
    /// Simple rectangle
    Rectangle,
    /// Text element
    Text,
    /// Image
    Image,
    /// Complex path
    Path,
    /// Custom element
    Custom,
    /// Container/Group
    Container,
}

/// Render node with optimization metadata
#[derive(Debug, Clone)]
pub struct RenderNode {
    /// Unique identifier
    pub id: String,
    /// Node type
    pub node_type: RenderNodeType,
    /// Bounding box
    pub bounds: Bounds<Pixels>,
    /// Z-order depth
    pub z_index: f32,
    /// Whether this node is dirty
    pub dirty: bool,
    /// Last render timestamp
    pub last_rendered: Option<Instant>,
    /// Render cache key
    pub cache_key: Option<String>,
    /// Batch group this node belongs to
    pub batch_group: Option<String>,
    /// Parent container
    pub parent: Option<String>,
    /// Children
    pub children: Vec<String>,
    /// Render complexity score (0-1)
    pub complexity: f32,
    /// Static flag (doesn't change often)
    pub static_node: bool,
}

/// Dirty region for partial invalidation
#[derive(Debug, Clone)]
pub struct DirtyRegion {
    /// Region bounds
    pub bounds: Bounds<Pixels>,
    /// Dirty timestamp
    pub timestamp: Instant,
    /// Priority of this region
    pub priority: RenderPriority,
}

/// Render priority levels
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum RenderPriority {
    Low = 0,
    Medium = 1,
    High = 2,
    Critical = 3,
}

/// Render batch for batching similar operations
#[derive(Debug, Clone)]
pub struct RenderBatch {
    /// Batch identifier
    pub id: String,
    /// Batch type
    pub batch_type: RenderNodeType,
    /// Nodes in this batch
    pub nodes: Vec<String>,
    /// Combined bounds
    pub bounds: Bounds<Pixels>,
    /// Whether batch is dirty
    pub dirty: bool,
    /// Render cost estimate
    pub cost: f32,
}

/// Render cache entry
#[derive(Debug, Clone)]
pub struct RenderCacheEntry {
    /// Cache key
    pub key: String,
    /// Cached render data (simplified)
    pub data: Vec<u8>,
    /// Bounds of cached content
    pub bounds: Bounds<Pixels>,
    /// Creation timestamp
    pub created_at: Instant,
    /// Last access timestamp
    pub last_access: Instant,
    /// Access count
    pub access_count: u32,
}

/// Frame render statistics
#[derive(Debug, Clone)]
pub struct FrameStats {
    /// Frame number
    pub frame_number: u64,
    /// Frame start time
    pub start_time: Instant,
    /// Frame end time
    pub end_time: Option<Instant>,
    /// Number of nodes rendered
    pub nodes_rendered: u32,
    /// Number of batches rendered
    pub batches_rendered: u32,
    /// Cache hits
    pub cache_hits: u32,
    /// Cache misses
    pub cache_misses: u32,
    /// Dirty regions rendered
    pub dirty_regions: u32,
    /// Frame time in milliseconds
    pub frame_time_ms: Option<f32>,
    /// Whether frame was skipped
    pub skipped: bool,
}

/// Main render optimizer
pub struct RenderOptimizer {
    config: RenderOptimizerConfig,
    render_nodes: Arc<RwLock<HashMap<String, RenderNode>>>,
    dirty_regions: Arc<Mutex<VecDeque<DirtyRegion>>>,
    render_batches: Arc<RwLock<HashMap<String, RenderBatch>>>,
    render_cache: Arc<RwLock<HashMap<String, RenderCacheEntry>>>,
    frame_stats: Arc<Mutex<VecDeque<FrameStats>>>,
    z_order_tracker: ZOrderTracker,
    frame_counter: u64,
    last_frame_time: Option<Instant>,
    adaptive_quality: AdaptiveQualityController,
}

impl RenderOptimizer {
    /// Create new render optimizer
    pub fn new(config: RenderOptimizerConfig) -> Self {
        Self {
            config,
            render_nodes: Arc::new(RwLock::new(HashMap::new())),
            dirty_regions: Arc::new(Mutex::new(VecDeque::new())),
            render_batches: Arc::new(RwLock::new(HashMap::new())),
            render_cache: Arc::new(RwLock::new(HashMap::new())),
            frame_stats: Arc::new(Mutex::new(VecDeque::new())),
            z_order_tracker: ZOrderTracker::new(),
            frame_counter: 0,
            last_frame_time: None,
            adaptive_quality: AdaptiveQualityController::new(),
        }
    }

    /// Register a render node
    pub fn register_node(&mut self, node: RenderNode) {
        let mut nodes = self.render_nodes.write();
        nodes.insert(node.id.clone(), node);

        // Update Z-order tracker
        self.z_order_tracker.add_node(node.id.clone(), node.z_index);
    }

    /// Update a render node
    pub fn update_node(&mut self, id: &str, updates: impl FnOnce(&mut RenderNode)) {
        let mut nodes = self.render_nodes.write();
        if let Some(node) = nodes.get_mut(id) {
            updates(node);
            node.dirty = true;

            // Mark dirty region
            if self.config.enable_dirty_regions {
                self.mark_dirty_region(node.bounds, RenderPriority::High);
            }
        }
    }

    /// Remove a render node
    pub fn remove_node(&mut self, id: &str) {
        let mut nodes = self.render_nodes.write();
        if nodes.remove(id).is_some() {
            self.z_order_tracker.remove_node(id);

            // Mark entire viewport as dirty
            if self.config.enable_dirty_regions {
                self.mark_dirty_region(
                    Bounds::centered(None, Size::new(px(10000.0), px(10000.0))),
                    RenderPriority::Medium
                );
            }
        }
    }

    /// Begin a new frame
    pub fn begin_frame(&mut self) -> FrameHandle {
        self.frame_counter += 1;
        let now = Instant::now();

        // Check if we should skip this frame
        let should_skip = self.config.enable_frame_skipping &&
            self.last_frame_time.map_or(false, |last| {
                now.duration_since(last) < self.config.frame_skip_threshold
            });

        let stats = FrameStats {
            frame_number: self.frame_counter,
            start_time: now,
            end_time: None,
            nodes_rendered: 0,
            batches_rendered: 0,
            cache_hits: 0,
            cache_misses: 0,
            dirty_regions: 0,
            frame_time_ms: None,
            skipped: should_skip,
        };

        // Store frame stats
        self.frame_stats.lock().push_back(stats);

        FrameHandle {
            frame_number: self.frame_counter,
            optimizer: self as *mut RenderOptimizer,
        }
    }

    /// Render the frame
    pub fn render_frame(&mut self) -> RenderResult {
        let mut result = RenderResult::default();

        if self.frame_counter == 0 {
            return result;
        }

        // Get current frame stats
        let mut stats = self.frame_stats.lock();
        if let Some(current_frame) = stats.back_mut() {
            if current_frame.skipped {
                result.skipped = true;
                return result;
            }

            // Phase 1: Update render batches
            self.update_render_batches(&mut result);

            // Phase 2: Determine what needs rendering
            let nodes_to_render = self.determine_nodes_to_render(&mut result);

            // Phase 3: Sort by Z-order if enabled
            let sorted_nodes = if self.config.enable_z_ordering {
                self.sort_nodes_by_z_order(nodes_to_render)
            } else {
                nodes_to_render
            };

            // Phase 4: Render nodes
            for node_id in sorted_nodes {
                self.render_node(&node_id, &mut result);
                current_frame.nodes_rendered += 1;
            }

            // Phase 5: Update statistics
            current_frame.batches_rendered = result.batches_rendered;
            current_frame.cache_hits = result.cache_hits;
            current_frame.cache_misses = result.cache_misses;
            current_frame.dirty_regions = result.dirty_regions_rendered;
        }

        // Clean up old stats
        while stats.len() > 300 { // Keep last 5 seconds at 60fps
            stats.pop_front();
        }

        self.last_frame_time = Some(Instant::now());

        result
    }

    /// End frame and finalize
    pub fn end_frame(&mut self) {
        // Clear dirty regions
        if self.config.enable_dirty_regions {
            self.dirty_regions.lock().clear();
        }

        // Clear dirty flags
        let mut nodes = self.render_nodes.write();
        for node in nodes.values_mut() {
            node.dirty = false;
        }

        // Update frame end time
        let mut stats = self.frame_stats.lock();
        if let Some(frame) = stats.back_mut() {
            frame.end_time = Some(Instant::now());
            frame.frame_time_ms = frame.end_time
                .map(|end| end.duration_since(frame.start_time).as_millis() as f32);
        }
    }

    /// Get render statistics for recent frames
    pub fn get_render_stats(&self) -> RenderStats {
        let stats = self.frame_stats.lock();
        let recent_frames: Vec<_> = stats.iter()
            .rev()
            .take(60) // Last second at 60fps
            .collect();

        if recent_frames.is_empty() {
            return RenderStats::default();
        }

        let total_nodes: u32 = recent_frames.iter().map(|f| f.nodes_rendered).sum();
        let total_batches: u32 = recent_frames.iter().map(|f| f.batches_rendered).sum();
        let avg_frame_time: f32 = recent_frames.iter()
            .filter_map(|f| f.frame_time_ms)
            .sum::<f32>() / recent_frames.len() as f32;

        RenderStats {
            fps: 1000.0 / avg_frame_time.max(0.1),
            avg_frame_time_ms: avg_frame_time,
            nodes_per_frame: total_nodes / recent_frames.len() as u32,
            batches_per_frame: total_batches / recent_frames.len() as u32,
            cache_hit_rate: self.calculate_cache_hit_rate(),
            dirty_regions_per_frame: recent_frames.iter()
                .map(|f| f.dirty_regions)
                .sum::<u32>() / recent_frames.len() as u32,
            skipped_frames: recent_frames.iter().filter(|f| f.skipped).count() as u32,
        }
    }

    /// Optimize render performance
    pub fn optimize(&mut self) {
        // Clean old cache entries
        self.cleanup_cache();

        // Optimize batch grouping
        self.optimize_batch_groups();

        // Update adaptive quality
        self.adaptive_quality.update(self.get_render_stats());
    }

    /// Clear render cache
    pub fn clear_cache(&mut self) {
        self.render_cache.write().clear();
    }

    /// Force full re-render
    pub fn invalidate_all(&mut self) {
        let mut nodes = self.render_nodes.write();
        for node in nodes.values_mut() {
            node.dirty = true;
        }

        // Mark entire viewport as dirty
        if self.config.enable_dirty_regions {
            self.mark_dirty_region(
                Bounds::centered(None, Size::new(px(10000.0), px(10000.0))),
                RenderPriority::Critical
            );
        }
    }

    // Private methods

    fn mark_dirty_region(&self, bounds: Bounds<Pixels>, priority: RenderPriority) {
        let mut regions = self.dirty_regions.lock();
        regions.push_back(DirtyRegion {
            bounds,
            timestamp: Instant::now(),
            priority,
        });

        // Merge overlapping regions
        self.merge_dirty_regions(&mut regions);
    }

    fn merge_dirty_regions(&self, regions: &mut VecDeque<DirtyRegion>) {
        // Simple region merging - in reality would be more sophisticated
        if regions.len() > 10 {
            regions.retain(|r| r.priority >= RenderPriority::High);
        }
    }

    fn update_render_batches(&mut self, result: &mut RenderResult) {
        if !self.config.enable_batching {
            return;
        }

        let nodes = self.render_nodes.read();
        let mut batches = self.render_batches.write();

        // Clear old batches
        batches.clear();

        // Group nodes by type and attributes for batching
        let mut batch_groups: HashMap<String, Vec<String>> = HashMap::new();

        for (id, node) in nodes.iter() {
            if node.dirty {
                let batch_key = format!("{:?}-{}", node.node_type, node.batch_group.as_deref().unwrap_or(""));
                batch_groups.entry(batch_key).or_default().push(id.clone());
            }
        }

        // Create render batches
        for (batch_key, node_ids) in batch_groups {
            if !node_ids.is_empty() {
                let batch_bounds = self.calculate_batch_bounds(&node_ids, &nodes);

                let batch = RenderBatch {
                    id: format!("batch-{}", batches.len()),
                    batch_type: nodes[&node_ids[0]].node_type,
                    nodes: node_ids,
                    bounds: batch_bounds,
                    dirty: true,
                    cost: self.estimate_batch_cost(&node_ids, &nodes),
                };

                batches.insert(batch.id.clone(), batch);
                result.batches_rendered += 1;
            }
        }
    }

    fn calculate_batch_bounds(&self, node_ids: &[String], nodes: &HashMap<String, RenderNode>) -> Bounds<Pixels> {
        let mut min_x = f32::MAX;
        let mut min_y = f32::MAX;
        let mut max_x = f32::MIN;
        let mut max_y = f32::MIN;

        for id in node_ids {
            if let Some(node) = nodes.get(id) {
                let bounds = node.bounds;
                min_x = min_x.min(bounds.origin.x.0);
                min_y = min_y.min(bounds.origin.y.0);
                max_x = max_x.max(bounds.origin.x.0 + bounds.size.width.0);
                max_y = max_y.max(bounds.origin.y.0 + bounds.size.height.0);
            }
        }

        Bounds::new(
            Point::new(px(min_x), px(min_y)),
            Size::new(px(max_x - min_x), px(max_y - min_y))
        )
    }

    fn estimate_batch_cost(&self, node_ids: &[String], nodes: &HashMap<String, RenderNode>) -> f32 {
        node_ids.iter()
            .map(|id| nodes.get(id).map(|n| n.complexity).unwrap_or(0.0))
            .sum()
    }

    fn determine_nodes_to_render(&mut self, result: &mut RenderResult) -> Vec<String> {
        let nodes = self.render_nodes.read();
        let mut nodes_to_render = Vec::new();

        if self.config.enable_dirty_regions {
            // Only render nodes in dirty regions
            let dirty_regions = self.dirty_regions.lock();

            for region in dirty_regions.iter() {
                for (id, node) in nodes.iter() {
                    if node.dirty && self.intersects_bounds(node.bounds, region.bounds) {
                        nodes_to_render.push(id.clone());
                        result.dirty_regions_rendered += 1;
                    }
                }
            }
        } else {
            // Render all dirty nodes
            for (id, node) in nodes.iter() {
                if node.dirty {
                    nodes_to_render.push(id.clone());
                }
            }
        }

        nodes_to_render
    }

    fn intersects_bounds(&self, a: Bounds<Pixels>, b: Bounds<Pixels>) -> bool {
        !(a.origin.x > b.origin.x + b.size.width ||
          a.origin.x + a.size.width < b.origin.x ||
          a.origin.y > b.origin.y + b.size.height ||
          a.origin.y + a.size.height < b.origin.y)
    }

    fn sort_nodes_by_z_order(&self, nodes: Vec<String>) -> Vec<String> {
        let render_nodes = self.render_nodes.read();
        let mut nodes_with_z: Vec<_> = nodes.into_iter()
            .filter_map(|id| {
                render_nodes.get(&id).map(|node| {
                    (id, node.z_index, node.node_type)
                })
            })
            .collect();

        // Sort by Z-order (back to front)
        nodes_with_z.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap());

        nodes_with_z.into_iter().map(|(id, _, _)| id).collect()
    }

    fn render_node(&mut self, node_id: &str, result: &mut RenderResult) {
        // Check cache first
        if let Some(node) = self.render_nodes.read().get(node_id) {
            if let Some(cache_key) = &node.cache_key {
                if let Some(_cached) = self.render_cache.read().get(cache_key) {
                    result.cache_hits += 1;
                    return;
                }
            }
        }

        // Render node (simplified - actual rendering would go through GPUI)
        result.cache_misses += 1;

        // Cache result if applicable
        if self.config.enable_caching {
            self.cache_render_result(node_id);
        }
    }

    fn cache_render_result(&mut self, node_id: &str) {
        let nodes = self.render_nodes.read();
        if let Some(node) = nodes.get(node_id) {
            if let Some(cache_key) = &node.cache_key {
                let entry = RenderCacheEntry {
                    key: cache_key.clone(),
                    data: Vec::new(), // Simplified
                    bounds: node.bounds,
                    created_at: Instant::now(),
                    last_access: Instant::now(),
                    access_count: 1,
                };

                self.render_cache.write().insert(cache_key.clone(), entry);
            }
        }
    }

    fn calculate_cache_hit_rate(&self) -> f32 {
        let stats = self.frame_stats.lock();
        let recent_frames: Vec<_> = stats.iter().rev().take(60).collect();

        if recent_frames.is_empty() {
            return 0.0;
        }

        let total_hits: u32 = recent_frames.iter().map(|f| f.cache_hits).sum();
        let total_requests: u32 = recent_frames.iter()
            .map(|f| f.cache_hits + f.cache_misses)
            .sum();

        if total_requests > 0 {
            total_hits as f32 / total_requests as f32 * 100.0
        } else {
            0.0
        }
    }

    fn cleanup_cache(&mut self) {
        let mut cache = self.render_cache.write();
        let now = Instant::now();

        // Remove entries older than 5 minutes
        cache.retain(|_, entry| {
            now.duration_since(entry.last_access) < Duration::from_secs(300)
        });

        // Limit cache size
        if cache.len() > 1000 {
            let mut entries: Vec<_> = cache.iter().collect();
            entries.sort_by(|a, b| a.1.last_access.cmp(&b.1.last_access));

            // Keep only the 1000 most recent
            for (key, _) in entries.iter().skip(1000) {
                cache.remove(*key);
            }
        }
    }

    fn optimize_batch_groups(&mut self) {
        // Analyze batch performance and optimize grouping
        // This would analyze which nodes are frequently rendered together
        // and adjust batch groups accordingly
    }
}

/// Handle for managing frame lifecycle
pub struct FrameHandle {
    frame_number: u64,
    optimizer: *mut RenderOptimizer,
}

impl FrameHandle {
    /// Complete the frame
    pub fn complete(self) {
        unsafe {
            if let Some(optimizer) = self.optimizer.as_mut() {
                optimizer.end_frame();
            }
        }
    }
}

/// Render result for a frame
#[derive(Debug, Default)]
pub struct RenderResult {
    pub nodes_rendered: u32,
    pub batches_rendered: u32,
    pub cache_hits: u32,
    pub cache_misses: u32,
    pub dirty_regions_rendered: u32,
    pub skipped: bool,
}

/// Render statistics
#[derive(Debug, Default, Clone)]
pub struct RenderStats {
    pub fps: f32,
    pub avg_frame_time_ms: f32,
    pub nodes_per_frame: u32,
    pub batches_per_frame: u32,
    pub cache_hit_rate: f32,
    pub dirty_regions_per_frame: u32,
    pub skipped_frames: u32,
}

/// Z-order tracker for depth sorting
struct ZOrderTracker {
    z_order_map: HashMap<String, f32>,
    dirty_nodes: HashSet<String>,
}

impl ZOrderTracker {
    fn new() -> Self {
        Self {
            z_order_map: HashMap::new(),
            dirty_nodes: HashSet::new(),
        }
    }

    fn add_node(&mut self, id: String, z_index: f32) {
        self.z_order_map.insert(id.clone(), z_index);
        self.dirty_nodes.insert(id);
    }

    fn remove_node(&mut self, id: &str) {
        self.z_order_map.remove(id);
        self.dirty_nodes.remove(id);
    }

    fn update_node(&mut self, id: String, z_index: f32) {
        if let Some(current_z) = self.z_order_map.get(&id) {
            if *current_z != z_index {
                self.z_order_map.insert(id.clone(), z_index);
                self.dirty_nodes.insert(id);
            }
        }
    }

    fn needs_resort(&self) -> bool {
        !self.dirty_nodes.is_empty()
    }

    fn clear_dirty(&mut self) {
        self.dirty_nodes.clear();
    }
}

/// Adaptive quality controller
struct AdaptiveQualityController {
    target_frame_time: Duration,
    current_quality: f32,
    adjustment_rate: f32,
}

impl AdaptiveQualityController {
    fn new() -> Self {
        Self {
            target_frame_time: Duration::from_millis(16), // 60fps
            current_quality: 1.0,
            adjustment_rate: 0.1,
        }
    }

    fn update(&mut self, stats: RenderStats) {
        if stats.avg_frame_time_ms > 0.0 {
            let frame_time = Duration::from_millis(stats.avg_frame_time_ms as u64);

            if frame_time > self.target_frame_time {
                // Need to reduce quality
                self.current_quality = (self.current_quality - self.adjustment_rate).max(0.5);
            } else if frame_time < self.target_frame_time * 8 / 10 {
                // Can increase quality
                self.current_quality = (self.current_quality + self.adjustment_rate).min(1.0);
            }
        }
    }

    fn get_quality(&self) -> f32 {
        self.current_quality
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_render_optimizer_creation() {
        let config = RenderOptimizerConfig::default();
        let optimizer = RenderOptimizer::new(config);

        assert_eq!(optimizer.frame_counter, 0);
        assert!(optimizer.render_nodes.read().is_empty());
    }

    #[test]
    fn test_node_registration() {
        let mut optimizer = RenderOptimizer::new(RenderOptimizerConfig::default());

        let node = RenderNode {
            id: "test-node".to_string(),
            node_type: RenderNodeType::Rectangle,
            bounds: Bounds::new(Point::zero(), Size::new(px(100.0), px(100.0))),
            z_index: 0.0,
            dirty: true,
            last_rendered: None,
            cache_key: None,
            batch_group: None,
            parent: None,
            children: Vec::new(),
            complexity: 0.5,
            static_node: false,
        };

        optimizer.register_node(node);

        let nodes = optimizer.render_nodes.read();
        assert_eq!(nodes.len(), 1);
        assert!(nodes.contains_key("test-node"));
    }

    #[test]
    fn test_dirty_region_tracking() {
        let optimizer = RenderOptimizer::new(RenderOptimizerConfig::default());
        let bounds = Bounds::new(Point::zero(), Size::new(px(50.0), px(50.0)));

        optimizer.mark_dirty_region(bounds, RenderPriority::High);

        let regions = optimizer.dirty_regions.lock();
        assert_eq!(regions.len(), 1);
        assert_eq!(regions[0].priority, RenderPriority::High);
    }

    #[test]
    fn test_frame_lifecycle() {
        let mut optimizer = RenderOptimizer::new(RenderOptimizerConfig::default());

        // Begin frame
        let frame = optimizer.begin_frame();
        assert_eq!(frame.frame_number, 1);

        // Render frame
        let result = optimizer.render_frame();
        assert!(!result.skipped);

        // End frame
        frame.complete();

        // Check frame stats
        let stats = optimizer.frame_stats.lock();
        assert_eq!(stats.len(), 1);
        assert!(stats[0].end_time.is_some());
    }

    #[test]
    fn test_z_order_tracker() {
        let mut tracker = ZOrderTracker::new();

        tracker.add_node("node1".to_string(), 1.0);
        tracker.add_node("node2".to_string(), 2.0);

        assert_eq!(tracker.z_order_map.get("node1"), Some(&1.0));
        assert_eq!(tracker.z_order_map.get("node2"), Some(&2.0));
        assert!(tracker.needs_resort());

        tracker.clear_dirty();
        assert!(!tracker.needs_resort());

        tracker.remove_node("node1");
        assert!(!tracker.z_order_map.contains_key("node1"));
    }

    #[test]
    fn test_adaptive_quality() {
        let mut controller = AdaptiveQualityController::new();

        assert_eq!(controller.get_quality(), 1.0);

        // Simulate poor performance
        let poor_stats = RenderStats {
            avg_frame_time_ms: 25.0, // 40fps
            ..Default::default()
        };
        controller.update(poor_stats);

        assert!(controller.get_quality() < 1.0);
    }

    #[test]
    fn test_render_stats_calculation() {
        let optimizer = RenderOptimizer::new(RenderOptimizerConfig::default());
        let stats = optimizer.get_render_stats();

        // Should handle empty state gracefully
        assert!(stats.fps >= 0.0);
        assert!(stats.avg_frame_time_ms >= 0.0);
    }

    #[test]
    fn test_cache_hit_rate() {
        let optimizer = RenderOptimizer::new(RenderOptimizerConfig::default());
        let hit_rate = optimizer.calculate_cache_hit_rate();

        assert!(hit_rate >= 0.0);
        assert!(hit_rate <= 100.0);
    }
}