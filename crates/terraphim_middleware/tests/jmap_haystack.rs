//! T11: JMAP regression -- the historic UTF-8-panic query against the PA's
//! JMAP haystack must still return >=50 results without panicking.
//!
//! Gated on `--features jmap` (compile-time) and `$JMAP_ACCESS_TOKEN` being set
//! at runtime. When the token is absent the test exits early with a `println!`
//! so cargo still records it as ok; CI without credentials should not fail.
//!
//! Invocation:
//!   JMAP_ACCESS_TOKEN=$(op read "op://...token...") \
//!     cargo test -p terraphim_middleware --test jmap_haystack \
//!       --features jmap -- --ignored --nocapture
//!
//! The `--ignored` gate prevents accidental network calls during normal
//! `cargo test --workspace` runs.

#![cfg(feature = "jmap")]

use terraphim_config::{Haystack, ServiceType};
use terraphim_middleware::haystack::JmapHaystackIndexer;
use terraphim_middleware::indexer::IndexMiddleware;

#[tokio::test]
#[ignore = "requires JMAP_ACCESS_TOKEN and live Fastmail connectivity"]
async fn t11_pa_terraphim_query_returns_50_plus_jmap_results() {
    let token = match std::env::var("JMAP_ACCESS_TOKEN") {
        Ok(t) if !t.trim().is_empty() => t,
        _ => {
            println!("JMAP_ACCESS_TOKEN unset; skipping T11");
            return;
        }
    };

    // Build a minimal Jmap haystack pointing at Fastmail's session URL. The
    // indexer reads `JMAP_ACCESS_TOKEN` from the env directly.
    let _ = token; // env is consumed by the indexer
    let haystack = Haystack::new(
        "https://api.fastmail.com/jmap/session".to_string(),
        ServiceType::Jmap,
        true,
    );

    let indexer = JmapHaystackIndexer;
    let index = indexer
        .index("terraphim", &haystack)
        .await
        .expect("JMAP index call should not error");

    let count = index.get_all_documents().len();
    assert!(
        count >= 50,
        "PA JMAP query for 'terraphim' returned {count} results; expected >=50"
    );
}
