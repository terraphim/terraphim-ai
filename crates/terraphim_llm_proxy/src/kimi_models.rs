//! Kimi model definitions
//!
//! This module includes the auto-generated Kimi model list from build.rs.
//! The model list is fetched from the Kimi API at build time, with a fallback
//! to a static list for offline builds.

// Include the generated model definitions
include!(concat!(env!("OUT_DIR"), "/kimi_models_generated.rs"));
