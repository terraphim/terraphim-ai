//! Mock Terminal Implementation
//!
//! Provides a mock terminal interface for testing TUI applications.
//! Simulates terminal behavior including cursor positioning, text rendering,
//! and ANSI escape sequence handling.

use anyhow::{Result, anyhow};
use std::collections::VecDeque;
use std::io::{self, Write};
use std::sync::{Arc, Mutex};

/// Mock terminal dimensions
#[derive(Debug, Clone, Copy)]
pub struct TerminalSize {
    pub width: u16,
    pub height: u16,
}

impl TerminalSize {
    pub fn new(width: u16, height: u16) -> Self {
        Self { width, height }
    }
}

/// Mock terminal cursor position
#[derive(Debug, Clone, Copy)]
pub struct CursorPosition {
    pub x: u16,
    pub y: u16,
}

impl CursorPosition {
    pub fn new(x: u16, y: u16) -> Self {
        Self { x, y }
    }
}

/// Mock terminal state
#[derive(Debug)]
struct TerminalState {
    /// Terminal buffer (lines of text)
    buffer: Vec<String>,
    /// Current cursor position
    cursor: CursorPosition,
    /// Terminal size
    size: TerminalSize,
    /// Command history buffer
    input_buffer: String,
    /// Output history for testing
    output_history: VecDeque<String>,
    /// ANSI escape sequence parsing state
    in_escape_sequence: bool,
    /// Current escape sequence
    escape_sequence: String,
}

impl TerminalState {
    fn new(size: TerminalSize) -> Self {
        let mut buffer = Vec::with_capacity(size.height as usize);
        for _ in 0..size.height {
            buffer.push(" ".repeat(size.width as usize));
        }

        Self {
            buffer,
            cursor: CursorPosition::new(0, 0),
            size,
            input_buffer: String::new(),
            output_history: VecDeque::with_capacity(1000),
            in_escape_sequence: false,
            escape_sequence: String::new(),
        }
    }

    /// Write text to the terminal at current cursor position
    fn write_text(&mut self, text: &str) -> Result<()> {
        let chars: Vec<char> = text.chars().collect();
        let mut i = 0;

        while i < chars.len() {
            let ch = chars[i];

            if ch == '\x1b' && i + 1 < chars.len() && chars[i + 1] == '[' {
                // Start of ANSI escape sequence
                self.in_escape_sequence = true;
                self.escape_sequence.clear();
                i += 2; // Skip \x1b[
                continue;
            }

            if self.in_escape_sequence {
                if ch.is_ascii_alphanumeric() || ch == '@' {
                    // End of escape sequence
                    self.escape_sequence.push(ch);
                    let seq = self.escape_sequence.clone();
                    self.handle_escape_sequence(&seq)?;
                    self.in_escape_sequence = false;
                    self.escape_sequence.clear();
                } else {
                    self.escape_sequence.push(ch);
                }
                i += 1;
                continue;
            }

            // Handle regular characters
            match ch {
                '\n' => {
                    self.cursor.y += 1;
                    self.cursor.x = 0;
                    if self.cursor.y >= self.size.height {
                        self.scroll_up();
                        self.cursor.y = self.size.height - 1;
                    }
                }
                '\r' => {
                    self.cursor.x = 0;
                }
                '\t' => {
                    // Tab expands to 4 spaces
                    for _ in 0..4 {
                        self.write_char_at_cursor(' ')?;
                    }
                }
                _ => {
                    self.write_char_at_cursor(ch)?;
                }
            }

            i += 1;
        }

        Ok(())
    }

    /// Write a single character at cursor position
    fn write_char_at_cursor(&mut self, ch: char) -> Result<()> {
        if self.cursor.y >= self.size.height || self.cursor.x >= self.size.width {
            return Ok(()); // Out of bounds, ignore
        }

        let line_idx = self.cursor.y as usize;
        let char_idx = self.cursor.x as usize;

        if line_idx < self.buffer.len() {
            let mut line: Vec<char> = self.buffer[line_idx].chars().collect();
            if char_idx < line.len() {
                line[char_idx] = ch;
                self.buffer[line_idx] = line.into_iter().collect();
            }
        }

        self.cursor.x += 1;
        if self.cursor.x >= self.size.width {
            self.cursor.x = 0;
            self.cursor.y += 1;
            if self.cursor.y >= self.size.height {
                self.scroll_up();
                self.cursor.y = self.size.height - 1;
            }
        }

        Ok(())
    }

    /// Handle ANSI escape sequences
    fn handle_escape_sequence(&mut self, seq: &str) -> Result<()> {
        if seq.is_empty() {
            return Ok(());
        }

        let parts: Vec<&str> = seq.split(';').collect();

        match seq.chars().last() {
            Some('H') | Some('f') => {
                // Cursor position (ESC[row;colH or ESC[row;colf)
                if parts.len() >= 2 {
                    if let (Ok(row), Ok(col)) = (
                        parts[parts.len() - 2].parse::<u16>(),
                        parts[parts.len() - 1]
                            .trim_end_matches('H')
                            .trim_end_matches('f')
                            .parse::<u16>(),
                    ) {
                        // ANSI is 1-based, convert to 0-based
                        self.cursor.x = (col - 1).min(self.size.width - 1);
                        self.cursor.y = (row - 1).min(self.size.height - 1);
                    }
                }
            }
            Some('J') => {
                // Clear screen
                match parts[0] {
                    "2" => {
                        // Clear entire screen
                        for line in &mut self.buffer {
                            *line = " ".repeat(self.size.width as usize);
                        }
                        self.cursor = CursorPosition::new(0, 0);
                    }
                    _ => {} // Other clear operations not implemented
                }
            }
            Some('K') => {
                // Clear line
                if self.cursor.y < self.size.height {
                    let line_idx = self.cursor.y as usize;
                    if line_idx < self.buffer.len() {
                        self.buffer[line_idx] = " ".repeat(self.size.width as usize);
                    }
                }
            }
            Some('m') => {
                // Text attributes (colors, etc.) - ignore for now
            }
            _ => {
                // Unknown sequence, ignore
            }
        }

        Ok(())
    }

    /// Scroll terminal up by one line
    fn scroll_up(&mut self) {
        if !self.buffer.is_empty() {
            self.buffer.remove(0);
            self.buffer.push(" ".repeat(self.size.width as usize));
        }
    }

    /// Get current display content
    fn get_display(&self) -> String {
        self.buffer.join("\n")
    }

    /// Clear the terminal
    fn clear(&mut self) -> Result<()> {
        for line in &mut self.buffer {
            *line = " ".repeat(self.size.width as usize);
        }
        self.cursor = CursorPosition::new(0, 0);
        self.input_buffer.clear();
        Ok(())
    }

    /// Add output to history
    fn record_output(&mut self, output: String) {
        self.output_history.push_back(output);
        if self.output_history.len() > 1000 {
            self.output_history.pop_front();
        }
    }
}

/// Mock Terminal for TUI Testing
pub struct MockTerminal {
    state: Arc<Mutex<TerminalState>>,
}

impl MockTerminal {
    /// Create a new mock terminal with specified dimensions
    pub fn new(width: u16, height: u16) -> Result<Self> {
        let size = TerminalSize::new(width, height);
        let state = Arc::new(Mutex::new(TerminalState::new(size)));

        Ok(Self { state })
    }

    /// Write text to the terminal
    pub fn write(&self, text: &str) -> Result<()> {
        let mut state = self
            .state
            .lock()
            .map_err(|_| anyhow!("Failed to lock terminal state"))?;
        state.write_text(text)?;
        state.record_output(text.to_string());
        Ok(())
    }

    /// Write a line to the terminal
    pub fn write_line(&self, line: &str) -> Result<()> {
        self.write(&format!("{}\n", line))
    }

    /// Read from the terminal (simulated input)
    pub fn read_line(&self) -> Result<String> {
        // In a real implementation, this would wait for input
        // For testing, we'll return empty string or mock input
        Ok(String::new())
    }

    /// Send simulated input to the terminal
    pub fn send_input(&self, input: &str) -> Result<()> {
        let mut state = self
            .state
            .lock()
            .map_err(|_| anyhow!("Failed to lock terminal state"))?;
        state.input_buffer.push_str(input);
        Ok(())
    }

    /// Get current terminal display content
    pub fn get_display(&self) -> Result<String> {
        let state = self
            .state
            .lock()
            .map_err(|_| anyhow!("Failed to lock terminal state"))?;
        Ok(state.get_display())
    }

    /// Get current cursor position
    pub fn get_cursor_position(&self) -> Result<CursorPosition> {
        let state = self
            .state
            .lock()
            .map_err(|_| anyhow!("Failed to lock terminal state"))?;
        Ok(state.cursor)
    }

    /// Clear the terminal
    pub fn clear(&self) -> Result<()> {
        let mut state = self
            .state
            .lock()
            .map_err(|_| anyhow!("Failed to lock terminal state"))?;
        state.clear()
    }

    /// Get terminal size
    pub fn get_size(&self) -> Result<TerminalSize> {
        let state = self
            .state
            .lock()
            .map_err(|_| anyhow!("Failed to lock terminal state"))?;
        Ok(state.size)
    }

    /// Resize terminal
    pub fn resize(&self, width: u16, height: u16) -> Result<()> {
        let mut state = self
            .state
            .lock()
            .map_err(|_| anyhow!("Failed to lock terminal state"))?;
        state.size = TerminalSize::new(width, height);

        // Resize buffer
        let new_buffer_size = height as usize;
        let line_width = width as usize;

        if new_buffer_size > state.buffer.len() {
            // Add lines
            for _ in state.buffer.len()..new_buffer_size {
                state.buffer.push(" ".repeat(line_width));
            }
        } else if new_buffer_size < state.buffer.len() {
            // Remove lines
            state.buffer.truncate(new_buffer_size);
        }

        // Resize existing lines
        for line in &mut state.buffer {
            if line_width > line.len() {
                *line = format!("{}{}", line, " ".repeat(line_width - line.len()));
            } else {
                *line = line.chars().take(line_width).collect();
            }
        }

        // Adjust cursor if out of bounds
        state.cursor.x = state.cursor.x.min(width - 1);
        state.cursor.y = state.cursor.y.min(height - 1);

        Ok(())
    }

    /// Get output history for testing
    pub fn get_output_history(&self) -> Result<Vec<String>> {
        let state = self
            .state
            .lock()
            .map_err(|_| anyhow!("Failed to lock terminal state"))?;
        Ok(state.output_history.iter().cloned().collect())
    }

    /// Check if terminal supports ANSI colors
    pub fn supports_ansi_colors(&self) -> bool {
        // Mock terminals always support ANSI for testing
        true
    }

    /// Check if terminal supports Unicode
    pub fn supports_unicode(&self) -> bool {
        // Mock terminals always support Unicode for testing
        true
    }
}

impl Write for MockTerminal {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        let text = String::from_utf8_lossy(buf);
        // Call the MockTerminal's write method with &str
        MockTerminal::write(self, &text).map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

impl Clone for MockTerminal {
    fn clone(&self) -> Self {
        Self {
            state: Arc::clone(&self.state),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_terminal_creation() {
        let terminal = MockTerminal::new(80, 24);
        assert!(terminal.is_ok());
    }

    #[test]
    fn test_terminal_write() {
        let terminal = MockTerminal::new(80, 24).unwrap();
        assert!(terminal.write("Hello, World!").is_ok());

        let display = terminal.get_display().unwrap();
        assert!(display.contains("Hello, World!"));
    }

    #[test]
    fn test_terminal_clear() {
        let terminal = MockTerminal::new(80, 24).unwrap();
        terminal.write("Test content").unwrap();
        terminal.clear().unwrap();

        let display = terminal.get_display().unwrap();
        assert!(!display.contains("Test content"));
    }

    #[test]
    fn test_cursor_position() {
        let terminal = MockTerminal::new(80, 24).unwrap();
        let pos = terminal.get_cursor_position().unwrap();
        assert_eq!(pos.x, 0);
        assert_eq!(pos.y, 0);
    }

    #[test]
    fn test_terminal_resize() {
        let terminal = MockTerminal::new(80, 24).unwrap();
        terminal.resize(120, 30).unwrap();

        let size = terminal.get_size().unwrap();
        assert_eq!(size.width, 120);
        assert_eq!(size.height, 30);
    }

    #[test]
    fn test_ansi_escape_sequences() {
        let terminal = MockTerminal::new(80, 24).unwrap();

        // Test cursor positioning
        terminal.write("\x1b[5;10H").unwrap();
        let pos = terminal.get_cursor_position().unwrap();
        assert_eq!(pos.x, 9); // 0-based
        assert_eq!(pos.y, 4); // 0-based

        // Test clear screen
        terminal.write("Some content").unwrap();
        terminal.write("\x1b[2J").unwrap();
        let display = terminal.get_display().unwrap();
        // Should be mostly empty after clear
        assert!(display.chars().filter(|&c| c != ' ').count() < 20);
    }
}
