//! L3 end-to-end smoke test: full fff -> KG -> sufficiency -> LLM -> citations pipeline
//! against a real free OpenRouter model.
//!
//! Default model is `liquid/lfm-2.5-1.2b-instruct:free` (1.2B params, sub-second responses,
//! lowest quota burn). Override with `OPENROUTER_MODEL` if you want a stronger model for
//! manual quality runs (e.g. `qwen/qwen3-coder:free`).
//!
//! Marked `#[ignore]` so `cargo test` skips it. Run manually with:
//!
//!   OPENROUTER_API_KEY=sk-or-v1-... \
//!     cargo test -p terraphim_grep --features "code-search openrouter" \
//!     --test e2e_smoke_openrouter -- --ignored --nocapture
//!
//! When the key is missing or the account is over quota the test exits cleanly with a
//! warning rather than failing -- mirrors the pattern in
//! `crates/terraphim_service/tests/openrouter_integration_test.rs`.
#![cfg(all(feature = "code-search", feature = "openrouter"))]

use std::sync::Arc;

use terraphim_grep::{
    GrepOptions, Haystack, HybridSearcher, SufficiencyJudge, SufficiencyState, TerraphimGrep,
};
use terraphim_types::Thesaurus;

const DEFAULT_FREE_MODEL: &str = "liquid/lfm-2.5-1.2b-instruct:free";

/// Mirrors the helper from `docs/OPENROUTER_TESTING_PLAN.md`. Free-tier accounts hit 401
/// (User not found), 403 (insufficient credits), and 429 (rate-limit) intermittently --
/// none of these signal a bug in grep, so the smoke degrades to a warning.
fn is_account_issue(err: &str) -> bool {
    let lower = err.to_lowercase();
    lower.contains("user not found")
        || lower.contains("insufficient credits")
        || lower.contains("rate limit")
        || lower.contains("rate-limit")
        || lower.contains("401")
        || lower.contains("403")
        || lower.contains("429")
}

fn build_openrouter_role(api_key: &str, model: &str) -> terraphim_config::Role {
    use serde_json::Value;
    let mut role = terraphim_config::Role::new("openrouter-smoke");
    role.llm_enabled = true;
    role.llm_api_key = Some(api_key.to_string());
    role.llm_model = Some(model.to_string());
    role.extra.insert(
        "llm_provider".to_string(),
        Value::String("openrouter".to_string()),
    );
    role.extra
        .insert("llm_model".to_string(), Value::String(model.to_string()));
    role
}

#[tokio::test]
#[ignore]
async fn e2e_grep_synthesises_answer_via_free_openrouter_model() {
    let api_key = match std::env::var("OPENROUTER_API_KEY") {
        Ok(v) if v.starts_with("sk-or-") => v,
        _ => {
            eprintln!("OPENROUTER_API_KEY not set or not a sk-or-* token; skipping L3 smoke");
            return;
        }
    };

    let model =
        std::env::var("OPENROUTER_MODEL").unwrap_or_else(|_| DEFAULT_FREE_MODEL.to_string());
    eprintln!("Running L3 smoke against {}", model);

    // Tempdir fixture: a small Rust source with a recognisable function the LLM can cite.
    let tmp = tempfile::TempDir::new().expect("tempdir");
    let target_path = tmp.path().join("retry.rs");
    std::fs::write(
        &target_path,
        "// retry policy with exponential backoff\n\
         pub struct RetryPolicy {\n\
         \x20   pub max_attempts: u32,\n\
         \x20   pub initial_delay_ms: u64,\n\
         }\n\
         \n\
         pub async fn with_retry<F, T>(policy: RetryPolicy, op: F) -> T\n\
         where F: Fn() -> T {\n\
         \x20   for _ in 0..policy.max_attempts { return op(); }\n\
         \x20   op()\n\
         }\n",
    )
    .expect("write fixture");

    // Add an unrelated file so the corpus has more than one candidate to choose from.
    let other_path = tmp.path().join("unrelated.rs");
    std::fs::write(&other_path, "pub fn parse_csv() {}\n").expect("write fixture");

    let role = build_openrouter_role(&api_key, &model);
    let Some(llm) = terraphim_service::llm::build_llm_from_role(&role) else {
        eprintln!("build_llm_from_role returned None for openrouter role; skipping");
        return;
    };

    let hybrid = HybridSearcher::new("openrouter-smoke".to_string(), Thesaurus::new("t".into()))
        .expect("build hybrid searcher")
        .with_search_path(tmp.path().to_path_buf());
    let grep = TerraphimGrep::new(Arc::new(hybrid), Arc::new(SufficiencyJudge::default()))
        .with_llm_client(llm);

    let result = match grep
        .search(
            "retry",
            GrepOptions {
                haystack: Haystack::Code,
                max_results: 50,
                force_rlm: true,
                include_answer: true,
                ..GrepOptions::default()
            },
        )
        .await
    {
        Ok(r) => r,
        Err(e) => {
            let msg = e.to_string();
            if is_account_issue(&msg) {
                eprintln!("OpenRouter account issue ({msg}); treating as skip");
                return;
            }
            panic!("e2e grep failed unexpectedly: {msg}");
        }
    };

    // Sufficiency should be RlmSynthesis since we forced the RLM path.
    assert_eq!(
        std::mem::discriminant(&result.sufficiency),
        std::mem::discriminant(&SufficiencyState::RlmSynthesis),
        "expected RlmSynthesis with force_rlm=true, got {:?}",
        result.sufficiency
    );

    // The fff backend should have located retry.rs.
    assert!(
        result.chunks.iter().any(|c| c.source.ends_with("retry.rs")),
        "expected at least one chunk citing retry.rs, got {:?}",
        result.chunks.iter().map(|c| &c.source).collect::<Vec<_>>(),
    );

    // The LLM may or may not return a structured answer depending on the model -- a 1.2B
    // model often returns prose that fails the strict JSON parse. Don't make the test fail
    // on that; the load-bearing assertion is that chunks were retrieved and the LLM was
    // actually called (rlm_latency_ms is populated).
    assert!(
        result.stats.rlm_latency_ms.is_some(),
        "rlm_latency_ms should be populated after a forced LLM call, got {:?}",
        result.stats
    );

    if let Some(ref ans) = result.answer {
        assert!(
            !ans.citations.is_empty(),
            "expected at least one citation when answer is present"
        );
        assert!(
            ans.citations.iter().any(|c| c.source.ends_with("retry.rs")),
            "citations should reference the source file, got {:?}",
            ans.citations.iter().map(|c| &c.source).collect::<Vec<_>>(),
        );
    } else {
        eprintln!(
            "Model {} did not return strict-JSON answer (common for small free models); \
             chunks + rlm_latency assertions still passed",
            model
        );
    }
}
