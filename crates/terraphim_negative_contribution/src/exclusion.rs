const EXCLUDED_PATH_PREFIXES: &[&str] = &["tests/", "examples/", "benches/"];
const EXCLUDED_PATH_INFIXES: &[&str] = &["/tests/", "/examples/", "/benches/"];
const EXCLUDED_FILENAMES: &[&str] = &["build.rs"];
const EXCLUDED_SUFFIXES: &[&str] = &["_test.rs"];
const TEST_MARKERS: &[&str] = &["#[test]", "#[cfg(test)]"];

pub fn is_non_production(path: &str, full_content: &str) -> bool {
    if is_excluded_path(path) {
        return true;
    }
    contains_test_markers(full_content)
}

fn is_excluded_path(path: &str) -> bool {
    let normalized = path.replace('\\', "/");

    for infix in EXCLUDED_PATH_INFIXES {
        if normalized.contains(infix) {
            return true;
        }
    }

    for prefix in EXCLUDED_PATH_PREFIXES {
        if normalized.starts_with(prefix) {
            return true;
        }
    }

    let filename = normalized.rsplit('/').next().unwrap_or(&normalized);

    for excluded in EXCLUDED_FILENAMES {
        if filename == *excluded {
            return true;
        }
    }

    for suffix in EXCLUDED_SUFFIXES {
        if filename.ends_with(suffix) {
            return true;
        }
    }

    false
}

fn contains_test_markers(content: &str) -> bool {
    TEST_MARKERS.iter().any(|marker| content.contains(marker))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tests_dir_excluded() {
        assert!(is_excluded_path("crates/foo/tests/integration.rs"));
        assert!(is_excluded_path("tests/common/mod.rs"));
    }

    #[test]
    fn test_examples_dir_excluded() {
        assert!(is_excluded_path("examples/demo.rs"));
        assert!(is_excluded_path("crates/bar/examples/basic.rs"));
    }

    #[test]
    fn test_benches_dir_excluded() {
        assert!(is_excluded_path("benches/perf.rs"));
        assert!(is_excluded_path("crates/baz/benches/benchmark.rs"));
    }

    #[test]
    fn test_build_rs_excluded() {
        assert!(is_excluded_path("build.rs"));
        assert!(is_excluded_path("crates/qux/build.rs"));
    }

    #[test]
    fn test_test_suffix_excluded() {
        assert!(is_excluded_path("src/foo_test.rs"));
        assert!(is_excluded_path("crates/my_crate/src/bar_test.rs"));
    }

    #[test]
    fn test_normal_file_not_excluded() {
        assert!(!is_excluded_path("src/main.rs"));
        assert!(!is_excluded_path("crates/my_crate/src/lib.rs"));
        assert!(!is_excluded_path("src/some_module.rs"));
    }

    #[test]
    fn test_inline_test_excluded() {
        let content = "fn foo() {}\n#[test]\nfn test_foo() {}\n";
        assert!(contains_test_markers(content));
    }

    #[test]
    fn test_cfg_test_excluded() {
        let content = "#[cfg(test)]\nmod tests {}\n";
        assert!(contains_test_markers(content));
    }

    #[test]
    fn test_no_test_markers() {
        let content = "fn main() { println!(\"hello\"); }\n";
        assert!(!contains_test_markers(content));
    }

    #[test]
    fn test_is_non_production_path_based() {
        assert!(is_non_production("tests/foo.rs", "fn main() {}"));
    }

    #[test]
    fn test_is_non_production_content_based() {
        assert!(is_non_production(
            "src/lib.rs",
            "#[cfg(test)]\nmod tests {}\n"
        ));
    }

    #[test]
    fn test_is_non_production_production_file() {
        assert!(!is_non_production("src/lib.rs", "fn main() {}"));
    }

    #[test]
    fn test_backslash_paths() {
        assert!(is_excluded_path("crates\\foo\\tests\\integration.rs"));
    }
}
