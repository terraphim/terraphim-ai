# API Reference Snippets

**Generated:** 2026-05-05
**Purpose:** Template doc comments for undocumented items

## Function Documentation Template

```rust
/// Brief one-sentence description of what this function does.
///
/// Longer explanation if needed. Describe behaviour, side effects,
/// and any important caveats.
///
/// # Arguments
///
/// * `arg_name` - Description of the argument and its constraints.
///
/// # Returns
///
/// Description of the return value. When does it succeed? When does it fail?
///
/// # Errors
///
/// Returns [`ErrorType::Variant`] when specific condition occurs.
///
/// # Examples
///
/// ```rust
/// use crate::module::function_name;
///
/// let result = function_name("input");
/// assert!(result.is_ok());
/// ```
pub fn function_name(arg: &str) -> Result<Output, ErrorType> {
    // implementation
}
```

## Struct Documentation Template

```rust
/// Brief description of what this struct represents.
///
/// Longer description of the struct's purpose and how it fits
/// into the system. Mention any invariants.
///
/// # Fields
///
/// * `field_name` - Description of the field and its purpose.
pub struct MyStruct {
    pub field_name: String,
}
```

## Enum Documentation Template

```rust
/// Represents the possible states of a component.
///
/// Each variant corresponds to a specific operational state.
pub enum State {
    /// Initial state before any processing.
    Idle,
    /// Actively processing data.
    Running,
    /// Processing completed successfully.
    Completed,
    /// Processing failed with an error.
    Failed(Error),
}
```

## Module Documentation Template

```rust
//! # Module Name
//!
//! Brief description of what this module provides.
//!
//! ## Overview
//!
//! Longer explanation of the module's purpose and how it fits
//! into the larger system.
//!
//! ## Key Types
//!
//! - [`TypeA`] - Description of TypeA
//! - [`TypeB`] - Description of TypeB
//!
//! ## Examples
//!
//! ```rust
//! use crate::module_name::Feature;
//!
//! let feature = Feature::new();
//! feature.do_thing();
//! ```
```

## Example: Well-Documented Function (from terraphim_types)

```rust
/// Extract the first paragraph from document body text.
///
/// Skips YAML frontmatter (content between `---` markers) and returns
/// the first non-empty line or the first paragraph.
pub fn extract_first_paragraph(body: &str) -> String {
    // implementation
}
```
