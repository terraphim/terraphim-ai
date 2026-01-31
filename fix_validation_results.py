#!/usr/bin/env python3
import re
import sys
from pathlib import Path

def fix_validation_result_construction(content):
    """Fix ValidationResult struct construction to use builder methods"""

    # Pattern to match old-style struct construction
    # UITestResult or TUITestResult { ... }
    pattern = r'(UI|TUI)TestResult\s*\{\s*name:\s*([^,]+),\s*status:\s*(UITestStatus|TUITestStatus)::(Pass|Fail|Skip),\s*message:\s*([^,]+),\s*details:\s*([^,]+),\s*duration_ms:\s*([^}]+)\}'

    def replace_func(match):
        test_type = match.group(1)  # UI or TUI
        name = match.group(2).strip()
        status = match.group(4).strip()  # Pass, Fail, or Skip

        # Generate the new construction
        if status == "Pass":
            return f'''{{
        let mut result = ValidationResult::new({name}, "test".to_string());
        result.pass(100);
        result
    }}'''
        elif status == "Fail":
            return f'''{{
        let mut result = ValidationResult::new({name}, "test".to_string());
        result.fail(100);
        result
    }}'''
        else:  # Skip
            return f'''{{
        let mut result = ValidationResult::new({name}, "test".to_string());
        result.skip(100);
        result
    }}'''

    # Apply the replacement
    content = re.sub(pattern, replace_func, content, flags=re.DOTALL)

    # Also fix simple inline returns
    simple_pattern = r'Ok\((UI|TUI)TestResult\s*\{[^}]+\}\)'
    if re.search(simple_pattern, content):
        # Need manual fixes for these
        print("Warning: Found simple inline patterns that need manual review")

    return content

def main():
    files = [
        "crates/terraphim_validation/src/testing/desktop_ui/cross_platform.rs",
        "crates/terraphim_validation/src/testing/desktop_ui/harness.rs",
        "crates/terraphim_validation/src/testing/desktop_ui/integration.rs",
        "crates/terraphim_validation/src/testing/desktop_ui/orchestrator.rs",
        "crates/terraphim_validation/src/testing/desktop_ui/performance.rs",
        "crates/terraphim_validation/src/testing/desktop_ui/utils.rs",
        "crates/terraphim_validation/src/testing/tui/command_simulator.rs",
        "crates/terraphim_validation/src/testing/tui/cross_platform.rs",
        "crates/terraphim_validation/src/testing/tui/harness.rs",
        "crates/terraphim_validation/src/testing/tui/integration.rs",
        "crates/terraphim_validation/src/testing/tui/output_validator.rs",
        "crates/terraphim_validation/src/testing/tui/performance_monitor.rs",
        "crates/terraphim_validation/src/testing/server_api/endpoints.rs",
        "crates/terraphim_validation/src/testing/server_api/fixtures.rs",
        "crates/terraphim_validation/src/testing/server_api/harness.rs",
        "crates/terraphim_validation/src/testing/server_api/performance.rs",
        "crates/terraphim_validation/src/testing/server_api/security.rs",
        "crates/terraphim_validation/src/testing/server_api/validation.rs",
    ]

    for file_path in files:
        path = Path(file_path)
        if path.exists():
            print(f"Processing {file_path}")
            content = path.read_text()
            new_content = fix_validation_result_construction(content)
            if content != new_content:
                path.write_text(new_content)
                print(f"  Updated {file_path}")
        else:
            print(f"  Skipping {file_path} (not found)")

if __name__ == "__main__":
    main()
