//! URL protection for text replacement.
//!
//! This module provides functionality to protect URLs from being corrupted
//! during text replacement operations. It works by:
//! 1. Identifying URL patterns in text
//! 2. Masking them with placeholders before replacement
//! 3. Restoring them after replacement
//!
//! # Example
//!
//! ```
//! use terraphim_automata::url_protector::UrlProtector;
//!
//! let protector = UrlProtector::new();
//! let text = "Visit https://example.com for more info";
//!
//! // Mask URLs before replacement
//! let (masked, placeholders) = protector.mask_urls(text);
//!
//! // Perform replacement on masked text...
//! let replaced = masked.replace("example", "sample");
//!
//! // Restore URLs after replacement
//! let result = protector.restore_urls(&replaced, &placeholders);
//! assert!(result.contains("https://example.com")); // URL preserved
//! ```

use regex::Regex;
use std::sync::LazyLock;

/// Compiled regex patterns for URL detection.
/// Using LazyLock for thread-safe, one-time initialization.
static URL_PATTERN: LazyLock<Regex> = LazyLock::new(|| {
    // Match common URL patterns:
    // - http:// or https:// URLs
    // - mailto: links
    // - Email addresses
    // Note: Use [>] instead of \> since \> is not a valid regex escape
    Regex::new(r"(?:https?://[^\s\)\]>]+)|(?:mailto:[^\s\)\]>]+)|(?:[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\.[a-zA-Z]{2,})")
        .expect("URL regex should compile")
});

/// Regex to match markdown link URLs: [text](url)
static MARKDOWN_LINK_URL: LazyLock<Regex> = LazyLock::new(|| {
    // Match the URL part of markdown links, preserving the display text
    Regex::new(r"\]\(([^)]+)\)").expect("Markdown link regex should compile")
});

/// Placeholder format for masked URLs
const PLACEHOLDER_PREFIX: &str = "\x00URL_PLACEHOLDER_";
const PLACEHOLDER_SUFFIX: &str = "\x00";

/// A protected URL span with its original text.
#[derive(Debug, Clone)]
pub struct ProtectedUrl {
    /// The original URL text
    pub url: String,
    /// The placeholder used to mask this URL
    pub placeholder: String,
}

/// URL protector for masking and restoring URLs during text replacement.
#[derive(Debug, Clone)]
pub struct UrlProtector {
    /// Counter for generating unique placeholders
    _placeholder_counter: u32,
}

impl Default for UrlProtector {
    fn default() -> Self {
        Self::new()
    }
}

impl UrlProtector {
    /// Create a new URL protector.
    pub fn new() -> Self {
        Self {
            _placeholder_counter: 0,
        }
    }

    /// Mask all URLs in the text with placeholders.
    ///
    /// Returns the masked text and a map of placeholders to original URLs.
    pub fn mask_urls(&self, text: &str) -> (String, Vec<ProtectedUrl>) {
        let mut result = text.to_string();
        let mut protected_urls = Vec::new();
        let mut placeholder_id = 0u32;

        // First, protect markdown link URLs (the URL part only)
        // We collect matches first to avoid modifying while iterating
        let markdown_matches: Vec<_> = MARKDOWN_LINK_URL
            .find_iter(&result)
            .map(|m| (m.start(), m.end(), m.as_str().to_string()))
            .collect();

        // Process markdown links in reverse order to preserve positions
        for (start, end, matched) in markdown_matches.into_iter().rev() {
            // Extract just the URL part (without ]( and ))
            if let Some(url) = matched.strip_prefix("](").and_then(|s| s.strip_suffix(')')) {
                let placeholder = format!(
                    "{}{}{}",
                    PLACEHOLDER_PREFIX, placeholder_id, PLACEHOLDER_SUFFIX
                );
                protected_urls.push(ProtectedUrl {
                    url: url.to_string(),
                    placeholder: placeholder.clone(),
                });
                // Replace just the URL part, keeping ]( and )
                let new_content = format!("]({})", placeholder);
                result.replace_range(start..end, &new_content);
                placeholder_id += 1;
            }
        }

        // Then, protect standalone URLs (not already in markdown links)
        let mut url_matches: Vec<(usize, usize, String)> = Vec::new();
        for mat in URL_PATTERN.find_iter(&result) {
            let url_str = mat.as_str();
            if !url_str.starts_with(PLACEHOLDER_PREFIX) {
                url_matches.push((mat.start(), mat.end(), url_str.to_string()));
            }
        }

        for (start, end, url) in url_matches.into_iter().rev() {
            let placeholder = format!(
                "{}{}{}",
                PLACEHOLDER_PREFIX, placeholder_id, PLACEHOLDER_SUFFIX
            );
            protected_urls.push(ProtectedUrl {
                url,
                placeholder: placeholder.clone(),
            });
            result.replace_range(start..end, &placeholder);
            placeholder_id += 1;
        }

        (result, protected_urls)
    }

    /// Restore all masked URLs from their placeholders.
    pub fn restore_urls(&self, text: &str, protected_urls: &[ProtectedUrl]) -> String {
        let mut result = text.to_string();
        for protected in protected_urls {
            result = result.replace(&protected.placeholder, &protected.url);
        }
        result
    }

    /// Check if the given text contains any URLs.
    pub fn contains_urls(text: &str) -> bool {
        URL_PATTERN.is_match(text) || MARKDOWN_LINK_URL.is_match(text)
    }
}

/// Convenience function to protect URLs, apply a transformation, and restore them.
pub fn with_protected_urls<F>(text: &str, transform: F) -> String
where
    F: FnOnce(&str) -> String,
{
    let protector = UrlProtector::new();
    let (masked, protected_urls) = protector.mask_urls(text);
    let transformed = transform(&masked);
    protector.restore_urls(&transformed, &protected_urls)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mask_simple_url() {
        let protector = UrlProtector::new();
        let text = "Visit https://example.com for more";
        let (masked, protected) = protector.mask_urls(text);

        assert!(!masked.contains("https://example.com"));
        assert!(masked.contains(PLACEHOLDER_PREFIX));
        assert_eq!(protected.len(), 1);
        assert_eq!(protected[0].url, "https://example.com");
    }

    #[test]
    fn test_restore_urls() {
        let protector = UrlProtector::new();
        let text = "Check https://example.com and https://other.org";
        let (masked, protected) = protector.mask_urls(text);
        let restored = protector.restore_urls(&masked, &protected);

        assert_eq!(restored, text);
    }

    #[test]
    fn test_markdown_link_url_preserved() {
        let protector = UrlProtector::new();
        let text = "[Claude Code](https://claude.ai/code)";
        let (masked, protected) = protector.mask_urls(text);

        // The display text should remain, only URL is masked
        assert!(masked.contains("[Claude Code]"));
        assert!(!masked.contains("https://claude.ai/code"));
        assert!(protected.iter().any(|p| p.url == "https://claude.ai/code"));
    }

    #[test]
    fn test_email_address_protected() {
        let protector = UrlProtector::new();
        let text = "Contact noreply@anthropic.com for help";
        let (masked, protected) = protector.mask_urls(text);

        assert!(!masked.contains("noreply@anthropic.com"));
        assert!(protected.iter().any(|p| p.url == "noreply@anthropic.com"));
    }

    #[test]
    fn test_multiple_urls() {
        let protector = UrlProtector::new();
        let text = "See https://a.com and https://b.org or email test@example.com";
        let (masked, protected) = protector.mask_urls(text);
        let restored = protector.restore_urls(&masked, &protected);

        assert_eq!(restored, text);
        assert_eq!(protected.len(), 3);
    }

    #[test]
    fn test_no_urls() {
        let protector = UrlProtector::new();
        let text = "No URLs here, just plain text";
        let (masked, protected) = protector.mask_urls(text);

        assert_eq!(masked, text);
        assert!(protected.is_empty());
    }

    #[test]
    fn test_contains_urls() {
        assert!(UrlProtector::contains_urls("Visit https://example.com"));
        assert!(UrlProtector::contains_urls("Email user@example.com"));
        assert!(UrlProtector::contains_urls("[link](https://url.com)"));
        assert!(!UrlProtector::contains_urls("No URLs here"));
    }

    #[test]
    fn test_with_protected_urls() {
        let text = "Replace Claude at https://claude.ai with something";
        let result = with_protected_urls(text, |s| s.replace("Claude", "Assistant"));

        assert!(result.contains("https://claude.ai")); // URL preserved
        assert!(result.contains("Assistant")); // Replacement happened
        assert!(!result.contains("Claude")); // Original replaced (outside URL)
    }

    #[test]
    fn test_complex_markdown_with_urls() {
        let protector = UrlProtector::new();
        let text = "Generated with [Claude Code](https://claude.ai/claude-code) by Claude";
        let (masked, protected) = protector.mask_urls(text);

        // Display text should be visible for replacement
        assert!(masked.contains("[Claude Code]"));
        // URL should be masked
        assert!(!masked.contains("https://claude.ai/claude-code"));

        let restored = protector.restore_urls(&masked, &protected);
        assert_eq!(restored, text);
    }
}
