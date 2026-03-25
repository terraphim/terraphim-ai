# terraphim_tinyclaw/tests/skills_benchmarks.rs

## Purpose
This file contains benchmarks for validating the performance of the Terraphim TinyClaw skills system. It measures execution times for skill loading, saving, and execution to ensure they meet non-functional requirements (NFRs).

## Key Functionality
- **Skill Load Benchmark**: Measures average time to load skills from storage (NFR: < 100ms)
- **Skill Save Benchmark**: Measures average time to save skills to storage (NFR: reasonably fast, originally < 50ms)
- **Skill Execution Benchmark**: Measures time to execute a simple skill with one LLM step (NFR: should complete quickly, originally < 1000ms)

## Key Components
- Uses `tempfile::TempDir` for isolated testing
- Uses `Instant` for precise timing measurements
- Tests with `SkillExecutor` and `Skill` types from terraphim_tinyclaw
- Includes asynchronous execution testing with tokio runtime

## Benchmark Details
1. **benchmark_skill_load_time**: Loads a skill 100 times and verifies average load time < 100ms
2. **benchmark_skill_save_time**: Saves a skill 100 times and verifies average save time < 50ms
3. **benchmark_execution_small_skill**: Executes a simple skill with one LLM step and verifies execution time < 1000ms (increased to 2000ms to account for system variability)

## Performance Characteristics
- All benchmarks use iteration counts to get statistically meaningful measurements
- Focus on practical performance rather than theoretical limits
- Designed to catch performance regressions during development

## Recent Changes
- Increased execution time threshold from 1000ms to 2000ms in benchmark_execution_small_skill to fix intermittent test failures due to system load variability while maintaining reasonable performance expectations
