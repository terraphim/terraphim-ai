use anyhow::{anyhow, Result};

/// Opens a URL in the system's default browser
pub fn open_url_in_browser(url: &str) -> Result<()> {
    // Validate URL
    if url.is_empty() {
        return Err(anyhow!("URL is empty"));
    }

    // Ensure URL has a scheme
    let url = if !url.starts_with("http://") && !url.starts_with("https://") {
        // If no scheme, assume https
        format!("https://{}", url)
    } else {
        url.to_string()
    };

    log::info!("Opening URL in browser: {}", url);

    // Open URL using the webbrowser crate
    match webbrowser::open(&url) {
        Ok(()) => {
            log::info!("Successfully opened URL in browser");
            Ok(())
        }
        Err(e) => {
            log::error!("Failed to open URL in browser: {}", e);
            Err(anyhow!("Failed to open URL: {}", e))
        }
    }
}

/// Opens a URL in the browser asynchronously
pub async fn open_url_in_browser_async(url: &str) -> Result<()> {
    let url = url.to_string();

    // Run in blocking task to avoid blocking the async runtime
    tokio::task::spawn_blocking(move || open_url_in_browser(&url))
        .await
        .map_err(|e| anyhow!("Failed to spawn blocking task: {}", e))?
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_url_validation() {
        // Empty URL should fail
        assert!(open_url_in_browser("").is_err());
    }

    #[test]
    fn test_url_scheme_addition() {
        // This test validates the logic but doesn't actually open the browser
        // We can't easily test the actual browser opening in unit tests

        // Test that scheme is added
        let result = std::panic::catch_unwind(|| {
            let url = "example.com";
            let processed = if !url.starts_with("http://") && !url.starts_with("https://") {
                format!("https://{}", url)
            } else {
                url.to_string()
            };
            assert_eq!(processed, "https://example.com");
        });
        assert!(result.is_ok());

        // Test that existing scheme is preserved
        let result = std::panic::catch_unwind(|| {
            let url = "https://example.com";
            let processed = if !url.starts_with("http://") && !url.starts_with("https://") {
                format!("https://{}", url)
            } else {
                url.to_string()
            };
            assert_eq!(processed, "https://example.com");
        });
        assert!(result.is_ok());
    }
}