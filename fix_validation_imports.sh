#!/bin/bash

# Fix imports in all validation test files
for file in \
    crates/terraphim_validation/src/testing/desktop_ui/cross_platform.rs \
    crates/terraphim_validation/src/testing/desktop_ui/harness.rs \
    crates/terraphim_validation/src/testing/desktop_ui/integration.rs \
    crates/terraphim_validation/src/testing/desktop_ui/orchestrator.rs \
    crates/terraphim_validation/src/testing/desktop_ui/performance.rs \
    crates/terraphim_validation/src/testing/desktop_ui/utils.rs \
    crates/terraphim_validation/src/testing/tui/command_simulator.rs \
    crates/terraphim_validation/src/testing/tui/cross_platform.rs \
    crates/terraphim_validation/src/testing/tui/harness.rs \
    crates/terraphim_validation/src/testing/tui/integration.rs \
    crates/terraphim_validation/src/testing/tui/output_validator.rs \
    crates/terraphim_validation/src/testing/server_api/endpoints.rs \
    crates/terraphim_validation/src/testing/server_api/fixtures.rs \
    crates/terraphim_validation/src/testing/server_api/harness.rs \
    crates/terraphim_validation/src/testing/server_api/performance.rs \
    crates/terraphim_validation/src/testing/server_api/security.rs \
    crates/terraphim_validation/src/testing/server_api/validation.rs \
    crates/terraphim_validation/src/reporting/mod.rs \
    crates/terraphim_validation/src/orchestrator/mod.rs \
    crates/terraphim_validation/src/testing/utils.rs \
    crates/terraphim_validation/src/testing/fixtures.rs
do
    if [ -f "$file" ]; then
        echo "Fixing imports in $file"
        # Replace UITestResult/UITestStatus imports with ValidationResult/ValidationStatus
        sed -i 's/use crate::testing::desktop_ui::{UITestResult, UITestStatus};/use crate::testing::{Result, ValidationResult, ValidationStatus};/g' "$file"
        sed -i 's/use crate::testing::tui::{TUITestResult, TUITestStatus};/use crate::testing::{Result, ValidationResult, ValidationStatus};/g' "$file"
        sed -i 's/use crate::{ValidationResult, ValidationStatus};/use crate::testing::{Result, ValidationResult, ValidationStatus};/g' "$file"
        sed -i 's/-> Result<Vec<UITestResult>>/-> Result<Vec<ValidationResult>>/g' "$file"
        sed -i 's/-> Result<UITestResult>/-> Result<ValidationResult>/g' "$file"
        sed -i 's/-> Result<Vec<TUITestResult>>/-> Result<Vec<ValidationResult>>/g' "$file"
        sed -i 's/-> Result<TUITestResult>/-> Result<ValidationResult>/g' "$file"
        sed -i 's/-> anyhow::Result<Vec<UITestResult>>/-> Result<Vec<ValidationResult>>/g' "$file"
        sed -i 's/-> anyhow::Result<UITestResult>/-> Result<ValidationResult>/g' "$file"
        sed -i 's/-> anyhow::Result<Vec<TUITestResult>>/-> Result<Vec<ValidationResult>>/g' "$file"
        sed -i 's/-> anyhow::Result<TUITestResult>/-> Result<ValidationResult>/g' "$file"
        sed -i 's/use anyhow;//g' "$file"
    fi
done

echo "Import fixes complete"
