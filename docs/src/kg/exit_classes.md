# Exit Classes

Knowledge graph thesaurus for ADF agent exit classification.
Each exit class is a concept whose synonyms are the stderr/stdout patterns that identify it.

Used by ExitClassifier in `crates/terraphim_orchestrator/src/agent_run_record.rs`
to classify agent exits via terraphim-automata Aho-Corasick matching.

## Timeout

synonyms:: timed out, deadline exceeded, wall-clock kill, context deadline exceeded, operation timed out, execution expired

## RateLimit

synonyms:: 429, rate limit, too many requests, quota exceeded, rate_limit_exceeded, throttled

## CompilationError

synonyms:: error[E, cannot find, unresolved import, cargo build failed, failed to compile, aborting due to, could not compile

## TestFailure

synonyms:: test result: FAILED, failures:, panicked at, assertion failed, thread 'main' panicked, cargo test failed

## ModelError

synonyms:: model not found, context length exceeded, invalid api key, invalid_api_key, model_not_found, insufficient_quota, content_policy_violation

## NetworkError

synonyms:: connection refused, dns resolution, ECONNRESET, ssl handshake, network unreachable, connection reset, ENOTFOUND, ETIMEDOUT

## ResourceExhaustion

synonyms:: out of memory, OOM, no space left, disk full, cannot allocate memory, memory allocation failed

## PermissionDenied

synonyms:: permission denied, EACCES, 403 Forbidden, access denied, insufficient permissions, not authorized

## Crash

synonyms:: SIGSEGV, SIGKILL, panic, stack overflow, SIGABRT, segmentation fault, bus error, SIGBUS
