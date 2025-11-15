#[cfg(feature = "repl")]
use terraphim_agent::repl::commands::{ReplCommand, WebConfigSubcommand, WebSubcommand};
#[cfg(feature = "repl")]
use terraphim_agent::repl::web_operations::*;

#[cfg(all(test, feature = "repl"))]
mod tests {
    use super::*;
    use terraphim_agent::repl::commands::{ReplCommand, WebConfigSubcommand, WebSubcommand};
    use terraphim_agent::repl::web_operations::utils::*;
    use terraphim_agent::repl::web_operations::*;

    #[test]
    fn test_web_get_command_parsing() {
        // Test basic GET command
        let cmd = ReplCommand::from_str("/web get https://httpbin.org/get").unwrap();
        match cmd {
            ReplCommand::Web { subcommand } => match subcommand {
                WebSubcommand::Get { url, headers } => {
                    assert_eq!(url, "https://httpbin.org/get");
                    assert!(headers.is_none());
                }
                _ => panic!("Expected WebSubcommand::Get"),
            },
            _ => panic!("Expected ReplCommand::Web"),
        }
    }

    #[test]
    fn test_web_get_with_headers_parsing() {
        let json_headers = r#"{"Accept": "application/json", "User-Agent": "TestBot"}"#;
        let cmd = ReplCommand::from_str(&format!(
            "/web get https://api.github.com/users --headers {}",
            json_headers
        ))
        .unwrap();

        match cmd {
            ReplCommand::Web { subcommand } => match subcommand {
                WebSubcommand::Get { url, headers } => {
                    assert_eq!(url, "https://api.github.com/users");
                    assert!(headers.is_some());
                    let headers = headers.unwrap();
                    assert_eq!(headers.get("Accept"), Some(&"application/json".to_string()));
                    assert_eq!(headers.get("User-Agent"), Some(&"TestBot".to_string()));
                }
                _ => panic!("Expected WebSubcommand::Get"),
            },
            _ => panic!("Expected ReplCommand::Web"),
        }
    }

    #[test]
    fn test_web_post_command_parsing() {
        let cmd =
            ReplCommand::from_str("/web post https://httpbin.org/post '{\"test\": \"data\"}'")
                .unwrap();

        match cmd {
            ReplCommand::Web { subcommand } => match subcommand {
                WebSubcommand::Post { url, body, headers } => {
                    assert_eq!(url, "https://httpbin.org/post");
                    assert_eq!(body, "{\"test\": \"data\"}");
                    assert!(headers.is_none());
                }
                _ => panic!("Expected WebSubcommand::Post"),
            },
            _ => panic!("Expected ReplCommand::Web"),
        }
    }

    #[test]
    fn test_web_post_with_headers_parsing() {
        let json_headers = r#"{"Content-Type": "application/json"}"#;
        let cmd = ReplCommand::from_str(&format!(
            "/web post https://api.example.com/data '{{\"name\": \"test\"}}' --headers {}",
            json_headers
        ))
        .unwrap();

        match cmd {
            ReplCommand::Web { subcommand } => match subcommand {
                WebSubcommand::Post { url, body, headers } => {
                    assert_eq!(url, "https://api.example.com/data");
                    assert_eq!(body, "{\"name\": \"test\"}");
                    assert!(headers.is_some());
                    let headers = headers.unwrap();
                    assert_eq!(
                        headers.get("Content-Type"),
                        Some(&"application/json".to_string())
                    );
                }
                _ => panic!("Expected WebSubcommand::Post"),
            },
            _ => panic!("Expected ReplCommand::Web"),
        }
    }

    #[test]
    fn test_web_scrape_command_parsing() {
        let cmd = ReplCommand::from_str("/web scrape https://example.com '.content'").unwrap();

        match cmd {
            ReplCommand::Web { subcommand } => match subcommand {
                WebSubcommand::Scrape {
                    url,
                    selector,
                    wait,
                } => {
                    assert_eq!(url, "https://example.com");
                    assert_eq!(selector, ".content");
                    assert!(wait.is_none());
                }
                _ => panic!("Expected WebSubcommand::Scrape"),
            },
            _ => panic!("Expected ReplCommand::Web"),
        }
    }

    #[test]
    fn test_web_scrape_with_wait_parsing() {
        let cmd = ReplCommand::from_str(
            "/web scrape https://example.com '#dynamic-content' --wait .loader",
        )
        .unwrap();

        match cmd {
            ReplCommand::Web { subcommand } => match subcommand {
                WebSubcommand::Scrape {
                    url,
                    selector,
                    wait,
                } => {
                    assert_eq!(url, "https://example.com");
                    assert_eq!(selector, "#dynamic-content");
                    assert_eq!(wait, Some(".loader".to_string()));
                }
                _ => panic!("Expected WebSubcommand::Scrape"),
            },
            _ => panic!("Expected ReplCommand::Web"),
        }
    }

    #[test]
    fn test_web_screenshot_command_parsing() {
        let cmd = ReplCommand::from_str("/web screenshot https://github.com").unwrap();

        match cmd {
            ReplCommand::Web { subcommand } => match subcommand {
                WebSubcommand::Screenshot {
                    url,
                    width,
                    height,
                    full_page,
                } => {
                    assert_eq!(url, "https://github.com");
                    assert!(width.is_none());
                    assert!(height.is_none());
                    assert!(full_page.is_none());
                }
                _ => panic!("Expected WebSubcommand::Screenshot"),
            },
            _ => panic!("Expected ReplCommand::Web"),
        }
    }

    #[test]
    fn test_web_screenshot_with_dimensions_parsing() {
        let cmd =
            ReplCommand::from_str("/web screenshot https://example.com --width 1920 --height 1080")
                .unwrap();

        match cmd {
            ReplCommand::Web { subcommand } => match subcommand {
                WebSubcommand::Screenshot {
                    url,
                    width,
                    height,
                    full_page,
                } => {
                    assert_eq!(url, "https://example.com");
                    assert_eq!(width, Some(1920));
                    assert_eq!(height, Some(1080));
                    assert!(full_page.is_none());
                }
                _ => panic!("Expected WebSubcommand::Screenshot"),
            },
            _ => panic!("Expected ReplCommand::Web"),
        }
    }

    #[test]
    fn test_web_screenshot_full_page_parsing() {
        let cmd = ReplCommand::from_str("/web screenshot https://docs.rs --full-page").unwrap();

        match cmd {
            ReplCommand::Web { subcommand } => match subcommand {
                WebSubcommand::Screenshot {
                    url,
                    width,
                    height,
                    full_page,
                } => {
                    assert_eq!(url, "https://docs.rs");
                    assert_eq!(full_page, Some(true));
                }
                _ => panic!("Expected WebSubcommand::Screenshot"),
            },
            _ => panic!("Expected ReplCommand::Web"),
        }
    }

    #[test]
    fn test_web_pdf_command_parsing() {
        let cmd = ReplCommand::from_str("/web pdf https://example.com").unwrap();

        match cmd {
            ReplCommand::Web { subcommand } => match subcommand {
                WebSubcommand::Pdf { url, page_size } => {
                    assert_eq!(url, "https://example.com");
                    assert!(page_size.is_none());
                }
                _ => panic!("Expected WebSubcommand::Pdf"),
            },
            _ => panic!("Expected ReplCommand::Web"),
        }
    }

    #[test]
    fn test_web_pdf_with_page_size_parsing() {
        let cmd = ReplCommand::from_str("/web pdf https://example.com --page-size A4").unwrap();

        match cmd {
            ReplCommand::Web { subcommand } => match subcommand {
                WebSubcommand::Pdf { url, page_size } => {
                    assert_eq!(url, "https://example.com");
                    assert_eq!(page_size, Some("A4".to_string()));
                }
                _ => panic!("Expected WebSubcommand::Pdf"),
            },
            _ => panic!("Expected ReplCommand::Web"),
        }
    }

    #[test]
    fn test_web_form_command_parsing() {
        let form_data = r#"{"username": "testuser", "password": "testpass"}"#;
        let cmd = ReplCommand::from_str(&format!(
            "/web form https://example.com/login {}",
            form_data
        ))
        .unwrap();

        match cmd {
            ReplCommand::Web { subcommand } => match subcommand {
                WebSubcommand::Form { url, form_data } => {
                    assert_eq!(url, "https://example.com/login");
                    assert_eq!(form_data.get("username"), Some(&"testuser".to_string()));
                    assert_eq!(form_data.get("password"), Some(&"testpass".to_string()));
                }
                _ => panic!("Expected WebSubcommand::Form"),
            },
            _ => panic!("Expected ReplCommand::Web"),
        }
    }

    #[test]
    fn test_web_api_command_parsing() {
        let cmd = ReplCommand::from_str(
            "/web api https://api.github.com /users/user1,/users/user2,/repos/repo1",
        )
        .unwrap();

        match cmd {
            ReplCommand::Web { subcommand } => match subcommand {
                WebSubcommand::Api {
                    base_url,
                    endpoints,
                    rate_limit,
                } => {
                    assert_eq!(base_url, "https://api.github.com");
                    assert_eq!(
                        endpoints,
                        vec!["/users/user1", "/users/user2", "/repos/repo1"]
                    );
                    assert!(rate_limit.is_none());
                }
                _ => panic!("Expected WebSubcommand::Api"),
            },
            _ => panic!("Expected ReplCommand::Web"),
        }
    }

    #[test]
    fn test_web_api_with_rate_limit_parsing() {
        let cmd = ReplCommand::from_str(
            "/web api https://api.example.com /endpoint1,/endpoint2 --rate-limit 1000",
        )
        .unwrap();

        match cmd {
            ReplCommand::Web { subcommand } => match subcommand {
                WebSubcommand::Api {
                    base_url,
                    endpoints,
                    rate_limit,
                } => {
                    assert_eq!(base_url, "https://api.example.com");
                    assert_eq!(endpoints, vec!["/endpoint1", "/endpoint2"]);
                    assert_eq!(rate_limit, Some(1000));
                }
                _ => panic!("Expected WebSubcommand::Api"),
            },
            _ => panic!("Expected ReplCommand::Web"),
        }
    }

    #[test]
    fn test_web_status_command_parsing() {
        let cmd = ReplCommand::from_str("/web status webop-1642514400000").unwrap();

        match cmd {
            ReplCommand::Web { subcommand } => match subcommand {
                WebSubcommand::Status { operation_id } => {
                    assert_eq!(operation_id, Some("webop-1642514400000".to_string()));
                }
                _ => panic!("Expected WebSubcommand::Status"),
            },
            _ => panic!("Expected ReplCommand::Web"),
        }
    }

    #[test]
    fn test_web_cancel_command_parsing() {
        let cmd = ReplCommand::from_str("/web cancel webop-1642514400000").unwrap();

        match cmd {
            ReplCommand::Web { subcommand } => match subcommand {
                WebSubcommand::Cancel { operation_id } => {
                    assert_eq!(operation_id, "webop-1642514400000");
                }
                _ => panic!("Expected WebSubcommand::Cancel"),
            },
            _ => panic!("Expected ReplCommand::Web"),
        }
    }

    #[test]
    fn test_web_history_command_parsing() {
        let cmd = ReplCommand::from_str("/web history").unwrap();

        match cmd {
            ReplCommand::Web { subcommand } => match subcommand {
                WebSubcommand::History { limit } => {
                    assert!(limit.is_none());
                }
                _ => panic!("Expected WebSubcommand::History"),
            },
            _ => panic!("Expected ReplCommand::Web"),
        }
    }

    #[test]
    fn test_web_history_with_limit_parsing() {
        let cmd = ReplCommand::from_str("/web history --limit 25").unwrap();

        match cmd {
            ReplCommand::Web { subcommand } => match subcommand {
                WebSubcommand::History { limit } => {
                    assert_eq!(limit, Some(25));
                }
                _ => panic!("Expected WebSubcommand::History"),
            },
            _ => panic!("Expected ReplCommand::Web"),
        }
    }

    #[test]
    fn test_web_config_show_command_parsing() {
        let cmd = ReplCommand::from_str("/web config show").unwrap();

        match cmd {
            ReplCommand::Web { subcommand } => {
                match subcommand {
                    WebSubcommand::Config { subcommand } => {
                        match subcommand {
                            WebConfigSubcommand::Show => {
                                // Test passes
                            }
                            _ => panic!("Expected WebConfigSubcommand::Show"),
                        }
                    }
                    _ => panic!("Expected WebSubcommand::Config"),
                }
            }
            _ => panic!("Expected ReplCommand::Web"),
        }
    }

    #[test]
    fn test_web_config_set_command_parsing() {
        let cmd = ReplCommand::from_str("/web config set timeout_ms 45000").unwrap();

        match cmd {
            ReplCommand::Web { subcommand } => match subcommand {
                WebSubcommand::Config { subcommand } => match subcommand {
                    WebConfigSubcommand::Set { key, value } => {
                        assert_eq!(key, "timeout_ms");
                        assert_eq!(value, "45000");
                    }
                    _ => panic!("Expected WebConfigSubcommand::Set"),
                },
                _ => panic!("Expected WebSubcommand::Config"),
            },
            _ => panic!("Expected ReplCommand::Web"),
        }
    }

    #[test]
    fn test_web_config_reset_command_parsing() {
        let cmd = ReplCommand::from_str("/web config reset").unwrap();

        match cmd {
            ReplCommand::Web { subcommand } => {
                match subcommand {
                    WebSubcommand::Config { subcommand } => {
                        match subcommand {
                            WebConfigSubcommand::Reset => {
                                // Test passes
                            }
                            _ => panic!("Expected WebConfigSubcommand::Reset"),
                        }
                    }
                    _ => panic!("Expected WebSubcommand::Config"),
                }
            }
            _ => panic!("Expected ReplCommand::Web"),
        }
    }

    #[test]
    fn test_web_operation_type_creation() {
        // Test HTTP GET operation creation
        let get_op = WebOperationType::http_get("https://httpbin.org/get");
        match get_op {
            WebOperationType::HttpGet { url, headers } => {
                assert_eq!(url, "https://httpbin.org/get");
                assert!(headers.is_none());
            }
            _ => panic!("Expected HttpGet"),
        }

        // Test HTTP POST operation creation
        let post_op = WebOperationType::http_post("https://httpbin.org/post", "test data");
        match post_op {
            WebOperationType::HttpPost { url, headers, body } => {
                assert_eq!(url, "https://httpbin.org/post");
                assert_eq!(body, "test data");
                assert!(headers.is_none());
            }
            _ => panic!("Expected HttpPost"),
        }

        // Test web scraping operation creation
        let scrape_op = WebOperationType::scrape("https://example.com", ".content");
        match scrape_op {
            WebOperationType::WebScrape {
                url,
                selector,
                wait_for_element,
            } => {
                assert_eq!(url, "https://example.com");
                assert_eq!(selector, ".content");
                assert!(wait_for_element.is_none());
            }
            _ => panic!("Expected WebScrape"),
        }

        // Test screenshot operation creation
        let screenshot_op = WebOperationType::screenshot("https://github.com");
        match screenshot_op {
            WebOperationType::Screenshot {
                url,
                width,
                height,
                full_page,
            } => {
                assert_eq!(url, "https://github.com");
                assert!(width.is_none());
                assert!(height.is_none());
                assert!(full_page.is_none());
            }
            _ => panic!("Expected Screenshot"),
        }

        // Test PDF generation operation creation
        let pdf_op = WebOperationType::generate_pdf("https://docs.rs");
        match pdf_op {
            WebOperationType::PdfGeneration { url, page_size } => {
                assert_eq!(url, "https://docs.rs");
                assert!(page_size.is_none());
            }
            _ => panic!("Expected PdfGeneration"),
        }
    }

    #[test]
    fn test_web_operation_builder() {
        let operation = WebOperationType::http_get("https://httpbin.org/get");
        let request = WebOperationBuilder::new(operation)
            .timeout_ms(30000)
            .build();

        assert_eq!(
            request.operation,
            WebOperationType::HttpGet {
                url: "https://httpbin.org/get".to_string(),
                headers: None
            }
        );
        assert_eq!(request.timeout_ms, Some(30000));
        assert!(request.vm_id.is_none());
        assert!(request.user_agent.is_none());
        assert!(request.proxy.is_none());
    }

    #[test]
    fn test_web_operation_with_all_options() {
        let operation = WebOperationType::http_post("https://api.example.com", "test body");
        let proxy_config = ProxyConfig {
            host: "proxy.example.com".to_string(),
            port: 8080,
            username: Some("user".to_string()),
            password: Some("pass".to_string()),
        };

        let request = WebOperationBuilder::new(operation)
            .vm_id("vm-123")
            .timeout_ms(60000)
            .user_agent("TestBot/1.0")
            .proxy(proxy_config)
            .build();

        assert_eq!(request.vm_id, Some("vm-123".to_string()));
        assert_eq!(request.timeout_ms, Some(60000));
        assert_eq!(request.user_agent, Some("TestBot/1.0".to_string()));
        assert!(request.proxy.is_some());
    }

    #[test]
    fn test_web_operation_config_default() {
        let config = WebOperationConfig::default();

        assert_eq!(config.default_timeout_ms, 30000);
        assert_eq!(config.max_concurrent_operations, 5);
        assert!(config.security.sandbox_enabled);
        assert!(config.security.allow_javascript);
        assert!(!config.security.allow_cookies);
        assert_eq!(config.security.max_response_size_mb, 10);
        assert_eq!(config.security.max_execution_time_ms, 60000);

        // Test default allowed domains
        assert!(config
            .security
            .allowed_domains
            .contains(&"https://httpbin.org".to_string()));
        assert!(config
            .security
            .allowed_domains
            .contains(&"https://jsonplaceholder.typicode.com".to_string()));
        assert!(config
            .security
            .allowed_domains
            .contains(&"https://api.github.com".to_string()));

        // Test default blocked domains
        assert!(config
            .security
            .blocked_domains
            .contains(&"malware.example.com".to_string()));

        // Test browser settings
        assert_eq!(
            config.browser_settings.user_agent,
            "Terraphim-TUI-WebBot/1.0"
        );
        assert_eq!(config.browser_settings.viewport_width, 1920);
        assert_eq!(config.browser_settings.viewport_height, 1080);
        assert!(config.browser_settings.enable_javascript);
        assert!(config.browser_settings.enable_images);
        assert!(config.browser_settings.enable_css);
        assert_eq!(config.browser_settings.timezone, "UTC");
    }

    #[test]
    fn test_web_operation_complexity_estimation() {
        use terraphim_agent::repl::web_operations::utils::*;

        // Test different operation complexities
        let get_op = WebOperationType::http_get("https://example.com");
        let complexity = estimate_complexity(&get_op);
        assert_eq!(complexity, OperationComplexity::Low);
        assert_eq!(complexity.recommended_timeout_ms(), 10000);
        assert_eq!(complexity.recommended_retries(), 2);

        let post_op = WebOperationType::http_post("https://api.example.com", "data");
        let complexity = estimate_complexity(&post_op);
        assert_eq!(complexity, OperationComplexity::Medium);
        assert_eq!(complexity.recommended_timeout_ms(), 30000);
        assert_eq!(complexity.recommended_retries(), 3);

        let scrape_op = WebOperationType::scrape("https://example.com", ".content");
        let complexity = estimate_complexity(&scrape_op);
        assert_eq!(complexity, OperationComplexity::High);
        assert_eq!(complexity.recommended_timeout_ms(), 60000);
        assert_eq!(complexity.recommended_retries(), 5);

        let screenshot_op = WebOperationType::screenshot("https://example.com");
        let complexity = estimate_complexity(&screenshot_op);
        assert_eq!(complexity, OperationComplexity::High);

        let pdf_op = WebOperationType::generate_pdf("https://example.com");
        let complexity = estimate_complexity(&pdf_op);
        assert_eq!(complexity, OperationComplexity::High);
    }

    #[test]
    fn test_web_operation_json_serialization() {
        let operation = WebOperationType::http_get("https://httpbin.org/get");
        let request = WebOperationBuilder::new(operation).build();

        // Test JSON serialization
        let json_str = serde_json::to_string(&request).unwrap();
        let parsed: WebOperationRequest = serde_json::from_str(&json_str).unwrap();

        assert_eq!(request.operation, parsed.operation);
        assert_eq!(request.timeout_ms, parsed.timeout_ms);
        assert_eq!(request.vm_id, parsed.vm_id);
        assert_eq!(request.user_agent, parsed.user_agent);
    }

    #[test]
    fn test_web_operation_result_types() {
        // Test HTTP response result
        let http_result = WebResultData::HttpResponse {
            status_code: 200,
            status_text: "OK".to_string(),
            headers: std::collections::HashMap::from([(
                "content-type".to_string(),
                "application/json".to_string(),
            )]),
            body: "{\"message\": \"success\"}".to_string(),
            content_type: "application/json".to_string(),
            content_length: 25,
        };

        match http_result {
            WebResultData::HttpResponse {
                status_code, body, ..
            } => {
                assert_eq!(status_code, 200);
                assert_eq!(body, "{\"message\": \"success\"}");
            }
            _ => panic!("Expected HttpResponse"),
        }

        // Test scraped content result
        let scraped_result = WebResultData::ScrapedContent {
            elements: vec![ScrapedElement {
                selector: ".title".to_string(),
                content: "Page Title".to_string(),
                html: "<h1>Page Title</h1>".to_string(),
                attributes: std::collections::HashMap::new(),
                position: ElementPosition {
                    x: 0.0,
                    y: 0.0,
                    width: 100.0,
                    height: 30.0,
                },
            }],
            page_title: "Test Page".to_string(),
            page_url: "https://example.com".to_string(),
            scrape_duration_ms: 1500,
        };

        match scraped_result {
            WebResultData::ScrapedContent {
                page_title,
                elements,
                ..
            } => {
                assert_eq!(page_title, "Test Page");
                assert_eq!(elements.len(), 1);
                assert_eq!(elements[0].content, "Page Title");
            }
            _ => panic!("Expected ScrapedContent"),
        }
    }

    #[test]
    fn test_web_command_error_handling() {
        // Test missing subcommand
        let result = ReplCommand::from_str("/web");
        assert!(result.is_err());

        // Test missing URL for GET
        let result = ReplCommand::from_str("/web get");
        assert!(result.is_err());

        // Test missing URL and body for POST
        let result = ReplCommand::from_str("/web post https://example.com");
        assert!(result.is_err());

        // Test missing URL and selector for scrape
        let result = ReplCommand::from_str("/web scrape https://example.com");
        assert!(result.is_err());

        // Test missing operation ID for status
        let result = ReplCommand::from_str("/web status");
        assert!(result.is_err());

        // Test invalid subcommand
        let result = ReplCommand::from_str("/web invalid_command");
        assert!(result.is_err());

        // Test invalid headers JSON
        let result = ReplCommand::from_str("/web get https://example.com --headers {invalid json}");
        assert!(result.is_err());

        // Test invalid form data JSON
        let result = ReplCommand::from_str("/web form https://example.com {invalid json}");
        assert!(result.is_err());
    }

    #[test]
    fn test_web_command_available_in_help() {
        // Test that web command is included in available commands
        let commands = ReplCommand::available_commands();
        assert!(commands.contains(&"web"));

        // Test that web command has help text
        let help_text = ReplCommand::get_command_help("web");
        assert!(help_text.is_some());
        let help_text = help_text.unwrap();
        assert!(help_text.contains("web operations"));
        assert!(help_text.contains("VM sandboxing"));
    }

    #[test]
    fn test_all_web_subcommands_coverage() {
        // Test that all web subcommands are properly parsed
        let test_cases = vec![
            "/web get https://example.com",
            "/web post https://example.com data",
            "/web scrape https://example.com .content",
            "/web screenshot https://example.com",
            "/web pdf https://example.com",
            "/web form https://example.com {\"key\":\"value\"}",
            "/web api https://api.example.com /endpoint1",
            "/web status op123",
            "/web cancel op123",
            "/web history",
            "/web config show",
        ];

        for test_case in test_cases {
            let result = ReplCommand::from_str(test_case);
            assert!(result.is_ok(), "Failed to parse: {}", test_case);

            match result.unwrap() {
                ReplCommand::Web { .. } => {
                    // Expected
                }
                _ => panic!("Expected Web command for: {}", test_case),
            }
        }
    }

    #[test]
    fn test_web_url_validation() {
        use terraphim_agent::repl::web_operations::utils::*;

        // Test valid URLs
        assert!(validate_url("https://example.com").is_ok());
        assert!(validate_url("http://localhost:8080").is_ok());
        assert!(validate_url("https://api.github.com/users").is_ok());

        // Test invalid URLs
        assert!(validate_url("").is_err());
        assert!(validate_url("not-a-url").is_err());
        assert!(validate_url("ftp://example.com").is_err());

        // Test domain extraction
        let domain = extract_domain("https://api.github.com/users").unwrap();
        assert_eq!(domain, "api.github.com");

        let domain = extract_domain("http://localhost:8080/test").unwrap();
        assert_eq!(domain, "localhost");
    }

    #[test]
    fn test_proxy_config_serialization() {
        let proxy = ProxyConfig {
            host: "proxy.example.com".to_string(),
            port: 8080,
            username: Some("user".to_string()),
            password: Some("pass".to_string()),
        };

        let json_str = serde_json::to_string(&proxy).unwrap();
        let parsed: ProxyConfig = serde_json::from_str(&json_str).unwrap();

        assert_eq!(proxy.host, parsed.host);
        assert_eq!(proxy.port, parsed.port);
        assert_eq!(proxy.username, parsed.username);
        assert_eq!(proxy.password, parsed.password);
    }
}
