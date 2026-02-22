//! Groq model definitions
//!
//! This module includes the auto-generated Groq model list from build.rs.
//! The model list is fetched from the Groq API at build time, with a fallback
//! to a static list for offline builds.

// Include the generated model definitions
include!(concat!(env!("OUT_DIR"), "/groq_models_generated.rs"));
