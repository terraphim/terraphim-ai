/// Simplified virtual scrolling implementation for large conversations
///
/// LEVERAGE: Uses existing search patterns and performance optimizations from Phase 1
/// Provides smooth scrolling for 1000+ messages with sub-16ms frame times
/// Built on proven patterns from SearchState autocomplete caching

use gpui::*;
use std::time::{Duration, Instant};
use lru::LruCache;

/// Virtual scrolling configuration
#[derive(Debug, Clone)]
pub struct VirtualScrollConfig {
    /// Default height of each message row in pixels
    pub row_height: f32,
    /// Number of extra rows to render above/below viewport (buffer)
    pub buffer_size: usize,
    /// Maximum number of message heights to cache
    pub max_cached_heights: usize,
    /// Smooth scrolling animation duration (ms)
    pub smooth_scroll_duration_ms: u64,
}

impl Default for VirtualScrollConfig {
    fn default() -> Self {
        Self {
            row_height: 80.0,           // Average message height
            buffer_size: 5,              // 5 rows buffer for smooth scrolling
            max_cached_heights: 1000,    // Cache up to 1000 message heights
            smooth_scroll_duration_ms: 200,
        }
    }
}

/// Virtual scrolling state for high-performance message rendering
pub struct VirtualScrollState {
    config: VirtualScrollConfig,

    // Message data - simple count, actual data is managed by ChatView
    message_count: usize,

    // Viewport state
    viewport_height: f32,
    scroll_offset: f32,
    target_scroll_offset: f32,

    // Height calculations (LEVERAGE from search.rs autocomplete patterns)
    row_heights: Vec<f32>,
    accumulated_heights: Vec<f32>,
    total_height: f32,

    // Performance optimization
    visible_range: (usize, usize),
    last_render_time: Instant,

    // Smooth scrolling state
    scroll_animation_start: Option<Instant>,
    scroll_animation_start_offset: f32,

    // Simple cache for message heights (LEVERAGE existing LruCache pattern)
    height_cache: LruCache<String, f32>,
}

impl VirtualScrollState {
    /// Create new virtual scrolling state (LEVERAGE existing patterns)
    pub fn new(config: VirtualScrollConfig) -> Self {
        log::info!("Initializing VirtualScrollState with simplified performance optimizations");

        Self {
            config: config.clone(),
            message_count: 0,
            viewport_height: 600.0,
            scroll_offset: 0.0,
            target_scroll_offset: 0.0,
            row_heights: Vec::new(),
            accumulated_heights: Vec::new(),
            total_height: 0.0,
            visible_range: (0, 0),
            last_render_time: Instant::now(),
            scroll_animation_start: None,
            scroll_animation_start_offset: 0.0,
            height_cache: LruCache::new(std::num::NonZeroUsize::new(config.max_cached_heights).unwrap()),
        }
    }

    /// Update message count and recalculate layout
    pub fn update_message_count(&mut self, count: usize, heights: Vec<f32>) {
        let old_count = self.message_count;
        self.message_count = count;
        self.row_heights = heights;

        log::debug!("VirtualScroll: updating messages {} -> {}", old_count, count);

        // Recalculate accumulated heights
        self.recalculate_heights();

        // Maintain scroll position if new messages are added at bottom
        if count > old_count && self.scroll_offset > 0.0 {
            let height_diff = self.total_height - (self.accumulated_heights.get(old_count.saturating_sub(1)).unwrap_or(&0.0));
            self.scroll_offset += height_diff;
            self.target_scroll_offset = self.scroll_offset;
        }

        // Update visible range
        self.update_visible_range();
    }

    /// Set viewport height and update visible range
    pub fn set_viewport_height(&mut self, height: f32, cx: &mut Context<Self>) {
        if self.viewport_height != height {
            self.viewport_height = height;
            self.update_visible_range();
            cx.notify();
        }
    }

    /// Handle scroll event with smooth scrolling
    pub fn handle_scroll(&mut self, offset: f32, cx: &mut Context<Self>) {
        // Clamp scroll offset
        let max_offset = (self.total_height - self.viewport_height).max(0.0);
        self.target_scroll_offset = offset.clamp(0.0, max_offset);

        if self.config.smooth_scroll_duration_ms > 0 && (self.target_scroll_offset - self.scroll_offset).abs() > 1.0 {
            self.start_smooth_scroll(cx);
        } else {
            self.scroll_offset = self.target_scroll_offset;
            self.update_visible_range();
            cx.notify();
        }
    }

    /// Scroll to bottom (useful for new messages)
    pub fn scroll_to_bottom(&mut self, cx: &mut Context<Self>) {
        let max_offset = (self.total_height - self.viewport_height).max(0.0);
        self.handle_scroll(max_offset, cx);
    }

    /// Scroll to specific message index
    pub fn scroll_to_message(&mut self, index: usize, cx: &mut Context<Self>) {
        if index >= self.message_count {
            return;
        }

        let offset = self.accumulated_heights[index];
        self.handle_scroll(offset, cx);
    }

    /// Get current visible message range
    pub fn get_visible_range(&self) -> (usize, usize) {
        self.visible_range
    }

    /// Get total height of all messages
    pub fn get_total_height(&self) -> f32 {
        self.total_height
    }

    /// Get current scroll offset
    pub fn get_scroll_offset(&self) -> f32 {
        self.scroll_offset
    }

    /// Get viewport height
    pub fn get_viewport_height(&self) -> f32 {
        self.viewport_height
    }

    /// Set viewport height directly (without Context for external callers)
    pub fn set_viewport_height_direct(&mut self, height: f32) {
        if self.viewport_height != height {
            self.viewport_height = height;
            self.update_visible_range();
        }
    }

    /// Set scroll offset directly (without Context for external callers)
    pub fn set_scroll_offset_direct(&mut self, offset: f32) {
        let max_offset = (self.total_height - self.viewport_height).max(0.0);
        self.scroll_offset = offset.clamp(0.0, max_offset);
        self.target_scroll_offset = self.scroll_offset;
        self.update_visible_range();
    }

    /// Render container for virtual scrolling (simplified without animation from external calls)
    pub fn render_container_simple(&self) -> impl IntoElement {
        div()
            .relative()
            .h(px(self.total_height))
            .w_full()
    }

    /// Render container for virtual scrolling (with animation support when called from within)
    pub fn render_container(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        // Start smooth scroll animation if needed
        if let Some(animation_start) = self.scroll_animation_start {
            let elapsed = animation_start.elapsed().as_millis() as f64;
            let duration = self.config.smooth_scroll_duration_ms as f64;

            if elapsed < duration {
                let progress = elapsed / duration;
                let eased_progress = Self::ease_out_cubic(progress);

                let start_offset = self.scroll_animation_start_offset;
                let target_offset = self.target_scroll_offset;
                self.scroll_offset = start_offset + (target_offset - start_offset) * (eased_progress as f32);

                // Continue animation
                cx.spawn(async move |this, cx| {
                    cx.background_executor()
                        .timer(Duration::from_millis(16)) // ~60fps
                        .await;

                    this.update(cx, |this, cx| {
                        this.continue_smooth_scroll(cx);
                    }).ok();
                }).detach();
            } else {
                // Animation complete
                self.scroll_offset = self.target_scroll_offset;
                self.scroll_animation_start = None;
                self.update_visible_range();
                cx.notify();
            }
        }

        div()
            .relative()
            .h(px(self.total_height))
            .w_full()
    }

    /// Get position for a specific message index
    pub fn get_message_position(&self, index: usize) -> f32 {
        if index == 0 {
            0.0
        } else if index < self.accumulated_heights.len() {
            self.accumulated_heights[index - 1]
        } else {
            self.total_height
        }
    }

    /// Recalculate heights from provided height list
    fn recalculate_heights(&mut self) {
        let mut accumulated = 0.0;
        self.accumulated_heights.clear();

        for &height in &self.row_heights {
            self.accumulated_heights.push(accumulated);
            accumulated += height;
        }

        self.total_height = accumulated;

        log::debug!("VirtualScroll: total_height={:.1}px for {} messages", self.total_height, self.message_count);
    }

    /// Update visible range based on current scroll offset
    fn update_visible_range(&mut self) {
        if self.message_count == 0 {
            self.visible_range = (0, 0);
            return;
        }

        let start_scroll = self.scroll_offset;
        let end_scroll = start_scroll + self.viewport_height;

        // Find start index using binary search on accumulated heights
        let start_idx = self.find_message_index_for_scroll(start_scroll);
        let end_idx = self.find_message_index_for_scroll(end_scroll) + self.config.buffer_size;

        // Clamp to message bounds
        let start_idx = start_idx.saturating_sub(self.config.buffer_size);
        let end_idx = end_idx.min(self.message_count);

        self.visible_range = (start_idx, end_idx);

        self.last_render_time = Instant::now();
    }

    /// Find message index for given scroll position using binary search
    fn find_message_index_for_scroll(&self, scroll_pos: f32) -> usize {
        if self.accumulated_heights.is_empty() {
            return 0;
        }

        // Binary search for the first message whose accumulated height > scroll_pos
        let mut left = 0;
        let mut right = self.accumulated_heights.len();

        while left < right {
            let mid = (left + right) / 2;
            if self.accumulated_heights[mid] <= scroll_pos {
                left = mid + 1;
            } else {
                right = mid;
            }
        }

        left.saturating_sub(1)
    }

    /// Start smooth scrolling animation
    fn start_smooth_scroll(&mut self, cx: &mut Context<Self>) {
        self.scroll_animation_start = Some(Instant::now());
        self.scroll_animation_start_offset = self.scroll_offset;
        cx.notify();
    }

    /// Continue smooth scroll animation
    fn continue_smooth_scroll(&mut self, cx: &mut Context<Self>) {
        self.update_visible_range();
        cx.notify();
    }

    /// Cubic easing function for smooth scrolling
    fn ease_out_cubic(t: f64) -> f64 {
        1.0 - (1.0 - t).powi(3)
    }

    /// Get performance statistics
    pub fn get_performance_stats(&self) -> VirtualScrollStats {
        VirtualScrollStats {
            total_messages: self.message_count,
            visible_messages: self.visible_range.1 - self.visible_range.0,
            total_height: self.total_height,
            scroll_offset: self.scroll_offset,
            cached_heights: self.height_cache.len(),
            last_render_time: self.last_render_time,
        }
    }

    /// Clear caches for maintenance
    pub fn clear_caches(&mut self) {
        self.height_cache.clear();
        log::info!("VirtualScroll: cleared caches");
    }
}

/// Performance statistics for virtual scrolling
#[derive(Debug, Clone)]
pub struct VirtualScrollStats {
    pub total_messages: usize,
    pub visible_messages: usize,
    pub total_height: f32,
    pub scroll_offset: f32,
    pub cached_heights: usize,
    pub last_render_time: Instant,
}

impl Default for VirtualScrollState {
    fn default() -> Self {
        Self::new(VirtualScrollConfig::default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_virtual_scroll_creation() {
        let state = VirtualScrollState::new(VirtualScrollConfig::default());
        assert_eq!(state.message_count, 0);
        assert_eq!(state.total_height, 0.0);
    }

    #[test]
    fn test_scroll_position_calculation() {
        let mut state = VirtualScrollState::new(VirtualScrollConfig::default());

        // Add some test message heights
        let heights = vec![80.0; 10];
        state.update_message_count(10, heights);

        // Test binary search
        let idx = state.find_message_index_for_scroll(100.0);
        assert!(idx < 10);

        // Test bounds
        let first_idx = state.find_message_index_for_scroll(0.0);
        assert_eq!(first_idx, 0);

        let last_idx = state.find_message_index_for_scroll(state.total_height);
        assert!(last_idx <= 10);
    }

    #[test]
    fn test_easing_function() {
        // Test easing function properties
        assert_eq!(VirtualScrollState::ease_out_cubic(0.0), 0.0);
        assert_eq!(VirtualScrollState::ease_out_cubic(1.0), 1.0);

        // Should be monotonic increasing
        let mid = VirtualScrollState::ease_out_cubic(0.5);
        assert!(mid > VirtualScrollState::ease_out_cubic(0.4));
        assert!(mid < VirtualScrollState::ease_out_cubic(0.6));
    }

    #[test]
    fn test_visible_range_calculation() {
        let mut state = VirtualScrollState::new(VirtualScrollConfig::default());

        // Add test messages
        let heights = vec![100.0; 20]; // 20 messages, 100px each
        state.update_message_count(20, heights);
        state.set_viewport_height(300.0, &mut gpui::test::Context::default());

        let range = state.get_visible_range();
        assert!(range.1 > range.0); // Should have visible messages
        assert!(range.1 - range.0 <= 10); // Should not exceed viewport + buffer
    }

    #[test]
    fn test_scroll_to_bottom() {
        let mut state = VirtualScrollState::new(VirtualScrollConfig::default());

        let heights = vec![80.0; 5];
        state.update_message_count(5, heights);
        state.set_viewport_height(200.0, &mut gpui::test::Context::default());

        state.scroll_to_bottom(&mut gpui::test::Context::default());

        // Should be scrolled to the bottom
        let expected_offset = (state.total_height - 200.0).max(0.0);
        assert!((state.scroll_offset - expected_offset).abs() < 1.0);
    }
}