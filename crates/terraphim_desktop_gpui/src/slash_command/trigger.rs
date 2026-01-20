//! Trigger Detection System for the Universal Slash Command System
//!
//! This module handles detection of trigger characters (`/`, `++`) and
//! manages debouncing for responsive autocomplete behavior.

use std::time::{Duration, Instant};

use super::types::{TriggerInfo, TriggerType, ViewScope};

/// Configuration for trigger detection
#[derive(Clone, Debug)]
pub struct TriggerConfig {
    /// Trigger characters and their settings
    pub char_triggers: Vec<CharTrigger>,
    /// Debounce duration for auto-trigger
    pub debounce_ms: u64,
    /// Minimum characters for auto-trigger
    pub auto_trigger_min_chars: usize,
}

impl Default for TriggerConfig {
    fn default() -> Self {
        Self {
            char_triggers: vec![
                CharTrigger {
                    sequence: "/".to_string(),
                    start_of_line: true,
                    scopes: vec![ViewScope::Chat, ViewScope::Search, ViewScope::Editor],
                },
                CharTrigger {
                    sequence: "++".to_string(),
                    start_of_line: false,
                    scopes: vec![ViewScope::Chat, ViewScope::Search, ViewScope::Editor],
                },
            ],
            debounce_ms: 150,
            auto_trigger_min_chars: 2,
        }
    }
}

/// Character trigger configuration
#[derive(Clone, Debug)]
pub struct CharTrigger {
    /// The character sequence that triggers (e.g., "/", "++")
    pub sequence: String,
    /// Whether trigger must be at start of line
    pub start_of_line: bool,
    /// View scopes where this trigger is active
    pub scopes: Vec<ViewScope>,
}

/// Trigger detection engine
pub struct TriggerEngine {
    config: TriggerConfig,
    /// Current active trigger (if any)
    active_trigger: Option<ActiveTrigger>,
    /// Last input update time for debouncing
    last_input_time: Option<Instant>,
    /// Current view scope
    current_view: ViewScope,
}

/// Active trigger state
#[derive(Clone, Debug)]
struct ActiveTrigger {
    /// The trigger that was detected
    trigger_type: TriggerType,
    /// Position where trigger started
    start_position: usize,
    /// Current query text (after trigger)
    query: String,
}

/// Result of trigger detection
#[derive(Clone, Debug)]
pub enum TriggerDetectionResult {
    /// A trigger was detected
    Triggered(TriggerInfo),
    /// Input changed but no trigger (may need debounce)
    InputChanged { text: String, cursor: usize },
    /// No trigger, nothing to do
    None,
    /// Trigger was cancelled (e.g., backspace deleted trigger)
    Cancelled,
}

impl TriggerEngine {
    /// Create a new trigger engine with default config
    pub fn new() -> Self {
        Self::with_config(TriggerConfig::default())
    }

    /// Create with custom config
    pub fn with_config(config: TriggerConfig) -> Self {
        Self {
            config,
            active_trigger: None,
            last_input_time: None,
            current_view: ViewScope::Chat,
        }
    }

    /// Set the current view scope
    pub fn set_view(&mut self, view: ViewScope) {
        self.current_view = view;
    }

    /// Process input change and detect triggers
    pub fn process_input(&mut self, text: &str, cursor: usize) -> TriggerDetectionResult {
        self.last_input_time = Some(Instant::now());

        // Check if we have an active trigger
        if let Some(ref active) = self.active_trigger {
            // Check if trigger is still valid
            if let Some(result) = self.update_active_trigger(text, cursor, active.clone()) {
                return result;
            }
        }

        // Try to detect a new trigger
        if let Some(trigger_info) = self.detect_trigger(text, cursor) {
            self.active_trigger = Some(ActiveTrigger {
                trigger_type: trigger_info.trigger_type.clone(),
                start_position: trigger_info.start_position,
                query: trigger_info.query.clone(),
            });
            return TriggerDetectionResult::Triggered(trigger_info);
        }

        // No trigger detected
        if text.is_empty() {
            TriggerDetectionResult::None
        } else {
            TriggerDetectionResult::InputChanged {
                text: text.to_string(),
                cursor,
            }
        }
    }

    /// Update an active trigger with new input
    fn update_active_trigger(
        &mut self,
        text: &str,
        cursor: usize,
        active: ActiveTrigger,
    ) -> Option<TriggerDetectionResult> {
        let trigger_len = match &active.trigger_type {
            TriggerType::Char { sequence, .. } => sequence.len(),
            _ => return None,
        };

        // Check if cursor moved before trigger
        if cursor < active.start_position {
            self.active_trigger = None;
            return Some(TriggerDetectionResult::Cancelled);
        }

        // Check if trigger text still exists at the original position
        let trigger_seq = match &active.trigger_type {
            TriggerType::Char { sequence, .. } => sequence.as_str(),
            _ => return None,
        };

        let trigger_end = active.start_position + trigger_len;
        if trigger_end > text.len() {
            self.active_trigger = None;
            return Some(TriggerDetectionResult::Cancelled);
        }

        if &text[active.start_position..trigger_end] != trigger_seq {
            self.active_trigger = None;
            return Some(TriggerDetectionResult::Cancelled);
        }

        // Extract new query (text after trigger, up to cursor)
        let query = if cursor > trigger_end {
            text[trigger_end..cursor].to_string()
        } else {
            String::new()
        };

        // Update active trigger
        self.active_trigger = Some(ActiveTrigger {
            query: query.clone(),
            ..active.clone()
        });

        // Return updated trigger info
        Some(TriggerDetectionResult::Triggered(TriggerInfo {
            trigger_type: active.trigger_type.clone(),
            start_position: active.start_position,
            query,
            view: self.current_view,
        }))
    }

    /// Detect a new trigger in the input
    fn detect_trigger(&self, text: &str, cursor: usize) -> Option<TriggerInfo> {
        for char_trigger in &self.config.char_triggers {
            // Check if this trigger is active for current view
            if !char_trigger.scopes.contains(&self.current_view)
                && !char_trigger.scopes.contains(&ViewScope::Both)
            {
                continue;
            }

            if let Some(trigger_info) = self.detect_char_trigger(text, cursor, char_trigger) {
                return Some(trigger_info);
            }
        }
        None
    }

    /// Detect a specific character trigger
    fn detect_char_trigger(
        &self,
        text: &str,
        cursor: usize,
        trigger: &CharTrigger,
    ) -> Option<TriggerInfo> {
        let trigger_len = trigger.sequence.len();

        // Need at least trigger length of text
        if cursor < trigger_len {
            return None;
        }

        // Only search within the current line to avoid cross-line triggers.
        let line_start = text[..cursor]
            .rfind(|c| c == '\n' || c == '\r')
            .map(|pos| pos + 1)
            .unwrap_or(0);
        let search_end = cursor.saturating_sub(trigger_len);
        if search_end < line_start {
            return None;
        }

        // Look backwards from cursor for the trigger sequence
        // We check if the trigger appears and extract query after it
        for start_pos in (line_start..=search_end).rev() {
            let end_pos = start_pos + trigger_len;
            if end_pos > text.len() {
                continue;
            }

            let potential_trigger = &text[start_pos..end_pos];
            if potential_trigger != trigger.sequence {
                continue;
            }

            // Check start-of-line requirement
            if trigger.start_of_line {
                if !self.is_at_line_start(text, start_pos) {
                    continue;
                }
            }

            // Found a valid trigger
            let query = if cursor > end_pos {
                text[end_pos..cursor].to_string()
            } else {
                String::new()
            };

            return Some(TriggerInfo {
                trigger_type: TriggerType::Char {
                    sequence: trigger.sequence.clone(),
                    start_of_line: trigger.start_of_line,
                },
                start_position: start_pos,
                query,
                view: self.current_view,
            });
        }

        None
    }

    /// Check if a position is at the start of a line
    fn is_at_line_start(&self, text: &str, position: usize) -> bool {
        if position == 0 {
            return true;
        }

        // Check if character before position is newline
        let char_before = text[..position].chars().last();
        matches!(char_before, Some('\n') | Some('\r'))
    }

    /// Cancel the current trigger
    pub fn cancel_trigger(&mut self) {
        self.active_trigger = None;
    }

    /// Get the current active trigger info
    pub fn active_trigger_info(&self) -> Option<TriggerInfo> {
        self.active_trigger.as_ref().map(|active| TriggerInfo {
            trigger_type: active.trigger_type.clone(),
            start_position: active.start_position,
            query: active.query.clone(),
            view: self.current_view,
        })
    }

    /// Check if there's an active trigger
    pub fn has_active_trigger(&self) -> bool {
        self.active_trigger.is_some()
    }

    /// Check if debounce period has passed
    pub fn should_query(&self) -> bool {
        if let Some(last_time) = self.last_input_time {
            let elapsed = last_time.elapsed();
            elapsed >= Duration::from_millis(self.config.debounce_ms)
        } else {
            false
        }
    }

    /// Get the debounce duration
    pub fn debounce_duration(&self) -> Duration {
        Duration::from_millis(self.config.debounce_ms)
    }

    /// Extract the text to insert when a suggestion is selected
    /// Returns (text_to_delete_range, text_to_insert)
    pub fn get_replacement_range(&self, input_text: &str) -> Option<(usize, usize)> {
        let active = self.active_trigger.as_ref()?;
        let trigger_len = match &active.trigger_type {
            TriggerType::Char { sequence, .. } => sequence.len(),
            _ => return None,
        };

        let start = active.start_position;
        let end = start + trigger_len + active.query.len();
        let end = end.min(input_text.len());

        Some((start, end))
    }
}

impl Default for TriggerEngine {
    fn default() -> Self {
        Self::new()
    }
}

/// Debounce manager for input changes
pub struct DebounceManager {
    last_change: Option<Instant>,
    duration: Duration,
}

impl DebounceManager {
    pub fn new(duration_ms: u64) -> Self {
        Self {
            last_change: None,
            duration: Duration::from_millis(duration_ms),
        }
    }

    /// Record an input change
    pub fn record_change(&mut self) {
        self.last_change = Some(Instant::now());
    }

    /// Check if debounce period has passed
    pub fn is_ready(&self) -> bool {
        if let Some(last) = self.last_change {
            last.elapsed() >= self.duration
        } else {
            false
        }
    }

    /// Get time remaining until ready
    pub fn time_remaining(&self) -> Duration {
        if let Some(last) = self.last_change {
            let elapsed = last.elapsed();
            if elapsed >= self.duration {
                Duration::ZERO
            } else {
                self.duration - elapsed
            }
        } else {
            self.duration
        }
    }

    /// Reset the debounce timer
    pub fn reset(&mut self) {
        self.last_change = None;
    }
}

impl Default for DebounceManager {
    fn default() -> Self {
        Self::new(150) // Default 150ms debounce
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_slash_trigger_at_start() {
        let mut engine = TriggerEngine::new();
        engine.set_view(ViewScope::Chat);

        let result = engine.process_input("/", 1);
        assert!(matches!(result, TriggerDetectionResult::Triggered(_)));

        if let TriggerDetectionResult::Triggered(info) = result {
            assert_eq!(info.start_position, 0);
            assert_eq!(info.query, "");
            assert!(matches!(
                info.trigger_type,
                TriggerType::Char { sequence, start_of_line: true } if sequence == "/"
            ));
        }
    }

    #[test]
    fn test_slash_trigger_with_query() {
        let mut engine = TriggerEngine::new();
        engine.set_view(ViewScope::Chat);

        let result = engine.process_input("/search", 7);
        assert!(matches!(result, TriggerDetectionResult::Triggered(_)));

        if let TriggerDetectionResult::Triggered(info) = result {
            assert_eq!(info.query, "search");
        }
    }

    #[test]
    fn test_slash_not_at_start() {
        let mut engine = TriggerEngine::new();
        engine.set_view(ViewScope::Chat);

        // Slash not at start of line should not trigger
        let result = engine.process_input("hello /search", 13);
        assert!(!matches!(result, TriggerDetectionResult::Triggered(_)));
    }

    #[test]
    fn test_slash_at_newline_start() {
        let mut engine = TriggerEngine::new();
        engine.set_view(ViewScope::Chat);

        let result = engine.process_input("hello\n/search", 13);
        assert!(matches!(result, TriggerDetectionResult::Triggered(_)));

        if let TriggerDetectionResult::Triggered(info) = result {
            assert_eq!(info.start_position, 6);
            assert_eq!(info.query, "search");
        }
    }

    #[test]
    fn test_plus_plus_trigger() {
        let mut engine = TriggerEngine::new();
        engine.set_view(ViewScope::Chat);

        let result = engine.process_input("hello ++rust", 12);
        assert!(matches!(result, TriggerDetectionResult::Triggered(_)));

        if let TriggerDetectionResult::Triggered(info) = result {
            assert_eq!(info.start_position, 6);
            assert_eq!(info.query, "rust");
            assert!(matches!(
                info.trigger_type,
                TriggerType::Char { sequence, start_of_line: false } if sequence == "++"
            ));
        }
    }

    #[test]
    fn test_trigger_update() {
        let mut engine = TriggerEngine::new();
        engine.set_view(ViewScope::Chat);

        // Initial trigger
        let result = engine.process_input("/se", 3);
        assert!(matches!(result, TriggerDetectionResult::Triggered(_)));

        // Continue typing
        let result = engine.process_input("/search", 7);
        assert!(matches!(result, TriggerDetectionResult::Triggered(_)));

        if let TriggerDetectionResult::Triggered(info) = result {
            assert_eq!(info.query, "search");
        }
    }

    #[test]
    fn test_trigger_cancel_on_backspace() {
        let mut engine = TriggerEngine::new();
        engine.set_view(ViewScope::Chat);

        // Initial trigger
        let result = engine.process_input("/search", 7);
        assert!(matches!(result, TriggerDetectionResult::Triggered(_)));

        // Backspace past trigger
        let result = engine.process_input("", 0);
        assert!(matches!(result, TriggerDetectionResult::None));
        assert!(!engine.has_active_trigger());
    }

    #[test]
    fn test_get_replacement_range() {
        let mut engine = TriggerEngine::new();
        engine.set_view(ViewScope::Chat);

        engine.process_input("/search", 7);

        let range = engine.get_replacement_range("/search");
        assert_eq!(range, Some((0, 7)));
    }

    #[test]
    fn test_debounce_manager() {
        let mut debounce = DebounceManager::new(100);

        assert!(!debounce.is_ready());

        debounce.record_change();
        assert!(!debounce.is_ready());

        // Simulate time passing (in real code, would wait)
        std::thread::sleep(Duration::from_millis(150));
        assert!(debounce.is_ready());
    }

    #[test]
    fn test_view_scope_filtering() {
        let config = TriggerConfig {
            char_triggers: vec![CharTrigger {
                sequence: "/".to_string(),
                start_of_line: true,
                scopes: vec![ViewScope::Chat], // Only Chat
            }],
            ..Default::default()
        };

        let mut engine = TriggerEngine::with_config(config);

        // Chat view should trigger
        engine.set_view(ViewScope::Chat);
        let result = engine.process_input("/test", 5);
        assert!(matches!(result, TriggerDetectionResult::Triggered(_)));

        // Search view should NOT trigger
        engine.cancel_trigger();
        engine.set_view(ViewScope::Search);
        let result = engine.process_input("/test", 5);
        assert!(!matches!(result, TriggerDetectionResult::Triggered(_)));

        // Editor view should NOT trigger (only Chat in config)
        engine.cancel_trigger();
        engine.set_view(ViewScope::Editor);
        let result = engine.process_input("/test", 5);
        assert!(!matches!(result, TriggerDetectionResult::Triggered(_)));
    }

    #[test]
    fn test_is_at_line_start() {
        let engine = TriggerEngine::new();

        assert!(engine.is_at_line_start("hello", 0));
        assert!(!engine.is_at_line_start("hello", 3));
        assert!(engine.is_at_line_start("hello\nworld", 6));
        assert!(!engine.is_at_line_start("hello\nworld", 8));
    }
}
