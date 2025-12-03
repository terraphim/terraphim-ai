/// Security Module
///
/// Comprehensive security controls for the Terraphim GPUI desktop application,
/// implementing defense-in-depth security patterns with input validation,
/// secure logging, memory safety, and attack prevention.

pub mod input_validation;

pub use input_validation::*;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_security_module_imports() {
        // Test that all security functions are properly exported
        let _ = validate_search_query("test");
        let _ = validate_file_path("test.txt");
        let _ = validate_username("user");
        let _ = sanitize_error_message("test", true);
    }
}