//! Benchmarks for skills system performance validation

use std::time::Instant;
use tempfile::TempDir;
use terraphim_tinyclaw::skills::{Skill, SkillExecutor, SkillStep};

#[test]
fn benchmark_skill_load_time() {
    // NFR: Skill load time < 100ms
    let temp_dir = TempDir::new().unwrap();
    let executor = SkillExecutor::new(temp_dir.path()).unwrap();

    let skill = Skill {
        name: "benchmark-skill".to_string(),
        version: "1.0.0".to_string(),
        description: "Benchmark skill".to_string(),
        author: None,
        steps: vec![
            SkillStep::Llm {
                prompt: "Step 1".to_string(),
                use_context: false,
            },
            SkillStep::Llm {
                prompt: "Step 2".to_string(),
                use_context: false,
            },
            SkillStep::Llm {
                prompt: "Step 3".to_string(),
                use_context: false,
            },
        ],
        inputs: vec![],
    };

    executor.save_skill(&skill).unwrap();

    // Benchmark load time
    let iterations = 100;
    let start = Instant::now();

    for _ in 0..iterations {
        let _ = executor.load_skill(&skill.name).unwrap();
    }

    let total_time = start.elapsed();
    let avg_time = total_time / iterations;

    println!("Skill Load Benchmark:");
    println!(
        "  Total time for {} iterations: {:?}",
        iterations, total_time
    );
    println!("  Average load time: {:?}", avg_time);

    // Assert meets NFR: < 100ms
    assert!(
        avg_time.as_millis() < 100,
        "Skill load time {:?} exceeds NFR of 100ms",
        avg_time
    );
}

#[test]
fn benchmark_skill_save_time() {
    let temp_dir = TempDir::new().unwrap();
    let executor = SkillExecutor::new(temp_dir.path()).unwrap();

    let skill = Skill {
        name: "save-benchmark".to_string(),
        version: "1.0.0".to_string(),
        description: "Save benchmark".to_string(),
        author: None,
        steps: (0..10)
            .map(|i| SkillStep::Llm {
                prompt: format!("Step {}", i),
                use_context: false,
            })
            .collect(),
        inputs: vec![],
    };

    let iterations = 100;
    let start = Instant::now();

    for i in 0..iterations {
        let mut s = skill.clone();
        s.name = format!("save-benchmark-{}", i);
        executor.save_skill(&s).unwrap();
    }

    let total_time = start.elapsed();
    let avg_time = total_time / iterations;

    println!("Skill Save Benchmark:");
    println!(
        "  Total time for {} iterations: {:?}",
        iterations, total_time
    );
    println!("  Average save time: {:?}", avg_time);

    // Should be reasonably fast
    assert!(
        avg_time.as_millis() < 50,
        "Skill save time {:?} is too slow",
        avg_time
    );
}

#[test]
fn benchmark_execution_small_skill() {
    let temp_dir = TempDir::new().unwrap();
    let executor = SkillExecutor::new(temp_dir.path()).unwrap();

    let skill = Skill {
        name: "exec-benchmark".to_string(),
        version: "1.0.0".to_string(),
        description: "Execution benchmark".to_string(),
        author: None,
        steps: vec![SkillStep::Llm {
            prompt: "Step 1".to_string(),
            use_context: false,
        }],
        inputs: vec![],
    };

    let rt = tokio::runtime::Runtime::new().unwrap();
    let start = Instant::now();

    rt.block_on(async {
        let result = executor
            .execute_skill(&skill, std::collections::HashMap::new(), None)
            .await;
        assert!(result.is_ok());
    });

    let exec_time = start.elapsed();

    println!("Skill Execution Benchmark (1 step):");
    println!("  Execution time: {:?}", exec_time);

    // Should complete quickly (just mock steps)
    assert!(
        exec_time.as_millis() < 1000,
        "Skill execution took too long: {:?}",
        exec_time
    );
}
