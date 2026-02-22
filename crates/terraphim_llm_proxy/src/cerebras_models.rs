//! Cerebras model definitions
//!
//! This module includes auto-generated Cerebras model constants from the build script.
//! The build script fetches models from the Cerebras API at build time, with a fallback
//! to a static list for offline builds.

include!(concat!(env!("OUT_DIR"), "/cerebras_models_generated.rs"));
