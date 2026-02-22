//! Output capture with @mention detection

use regex::Regex;
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::{ChildStderr, ChildStdout};
use tokio::sync::mpsc;

use terraphim_types::capability::ProcessId;

/// Events that can be captured from agent output
#[derive(Debug, Clone)]
pub enum OutputEvent {
    /// Standard output line
    Stdout { process_id: ProcessId, line: String },
    /// Standard error line
    Stderr { process_id: ProcessId, line: String },
    /// @mention detected in output
    Mention { 
        process_id: ProcessId, 
        target: String, 
        message: String,
    },
    /// Process completed
    Completed { process_id: ProcessId, exit_code: Option<i32> },
}

/// Captures output from agent processes with @mention detection
#[derive(Debug)]
pub struct OutputCapture {
    process_id: ProcessId,
    mention_regex: Regex,
    event_sender: mpsc::Sender<OutputEvent>,
}

impl OutputCapture {
    /// Create a new output capture
    pub fn new(
        process_id: ProcessId,
        stdout: BufReader<ChildStdout>,
        _stderr: BufReader<ChildStderr>,
    ) -> Self {
        let (event_sender, _event_receiver) = mpsc::channel(100);
        
        let capture = Self {
            process_id,
            mention_regex: Regex::new(r"@(\w+)").unwrap(),
            event_sender,
        };
        
        // Start capturing stdout
        capture.capture_stdout(stdout);
        
        capture
    }
    
    /// Start capturing stdout
    fn capture_stdout(
        &self,
        mut stdout: BufReader<ChildStdout>,
    ) {
        let process_id = self.process_id;
        let mention_regex = self.mention_regex.clone();
        let event_sender = self.event_sender.clone();
        
        tokio::spawn(async move {
            let mut line = String::new();
            
            loop {
                line.clear();
                match stdout.read_line(&mut line).await {
                    Ok(0) => break, // EOF
                    Ok(_) => {
                        let line = line.trim().to_string();
                        if line.is_empty() {
                            continue;
                        }
                        
                        // Check for @mentions
                        if let Some(captures) = mention_regex.captures(&line) {
                            if let Some(target) = captures.get(1) {
                                let target = target.as_str().to_string();
                                let message = line.clone();
                                
                                let _ = event_sender.send(OutputEvent::Mention {
                                    process_id,
                                    target,
                                    message,
                                }).await;
                            }
                        }
                        
                        // Send stdout event
                        let _ = event_sender.send(OutputEvent::Stdout {
                            process_id,
                            line,
                        }).await;
                    }
                    Err(e) => {
                        log::error!("Error reading stdout for process {}: {}", process_id, e);
                        break;
                    }
                }
            }
        });
    }
    
    /// Get the event sender (for external use)
    pub fn event_sender(&self) -> mpsc::Sender<OutputEvent> {
        self.event_sender.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mention_regex() {
        let regex = Regex::new(r"@(\w+)").unwrap();
        
        let text = "Hello @kimiko, can you help?";
        let captures = regex.captures(text).unwrap();
        assert_eq!(captures.get(1).unwrap().as_str(), "kimiko");
        
        let text = "No mentions here";
        assert!(regex.captures(text).is_none());
    }
}
