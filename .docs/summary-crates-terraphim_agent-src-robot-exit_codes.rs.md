# Summary: crates/terraphim_agent/src/robot/exit_codes.rs

**Purpose**: Defines typed exit codes for robot/CLI mode following Unix conventions.

**Key Components**:
- `ExitCode` enum with 8 variants (0-7): Success, ErrorGeneral, ErrorUsage, ErrorIndexMissing, ErrorNotFound, ErrorAuth, ErrorNetwork, ErrorTimeout
- Methods: `code()` (numeric value), `description()` (human text), `name()` (JSON name), `from_code()` (u8 conversion)
- Implementations: `From<ExitCode>` for `std::process::ExitCode`, `Termination` trait, `Display`

**Recent Change (Commit 4f9beed1)**:
- Added explicit `1 => ExitCode::ErrorGeneral` arm in `from_code()` for self-documentation
- Behavior unchanged; improves clarity that code 1 maps to ErrorGeneral

**Usage**: Imported by main.rs for typed exit code handling in CLI and robot modes. Enables consistent error propagation across listen mode, forgiving mode, and command execution paths.

**Tests**: Unit tests verify code mappings and round-trip conversion.
