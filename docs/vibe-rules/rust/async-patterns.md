# Rust Async Programming Patterns

## Tokio Spawn Pattern
#async #rust #tokio #concurrency #spawn

Always use `tokio::spawn` for concurrent tasks that don't need to share state.

**Good Example:**
```rust
use tokio;

async fn process_items(items: Vec<Item>) {
    for item in items {
        tokio::spawn(async move {
            process_item(item).await;
        });
    }
}
```

**Bad Example:**
```rust
// Unnecessary mutex for independent work
let mutex = Arc::new(Mutex::new(state));
tokio::spawn(async move {
    let state = mutex.lock().await;
    // Work that doesn't actually need shared state
});
```

**Rationale:** Independent tasks should not share state. Using `tokio::spawn` allows the runtime to distribute work across threads efficiently.

---

## Bounded Channels Pattern
#async #rust #channels #backpressure #mpsc

Prefer bounded channels for backpressure. Unbounded channels can cause memory issues under load.

**Good Example:**
```rust
use tokio::sync::mpsc;

async fn producer_consumer() {
    let (tx, mut rx) = mpsc::channel(100);

    tokio::spawn(async move {
        for i in 0..1000 {
            tx.send(i).await.unwrap();
        }
    });

    while let Some(msg) = rx.recv().await {
        process(msg).await;
    }
}
```

**Bad Example:**
```rust
// Can exhaust memory if consumer is slow
let (tx, mut rx) = mpsc::unbounded_channel();
```

**Rationale:** Bounded channels provide natural backpressure. When the channel is full, producers must wait, preventing memory exhaustion.

**When to use unbounded:** Only when you can guarantee bounded producer rate or have explicit backpressure elsewhere.

---

## Cancellation Pattern
#async #rust #cancellation #tokio-select #graceful-shutdown

Use `tokio::select!` for proper cancellation and timeout handling.

**Good Example:**
```rust
use tokio::time::{sleep, Duration};
use tokio::sync::broadcast;

async fn with_cancellation(mut shutdown: broadcast::Receiver<()>) {
    tokio::select! {
        result = long_running_task() => {
            handle_result(result);
        }
        _ = shutdown.recv() => {
            cleanup().await;
            return;
        }
        _ = sleep(Duration::from_secs(30)) => {
            log::warn!("Task timed out");
            return;
        }
    }
}
```

**Bad Example:**
```rust
// No cancellation mechanism
async fn no_cancellation() {
    let result = long_running_task().await;
    handle_result(result);
}
```

**Rationale:** Tasks must be cancellable for graceful shutdown. `tokio::select!` allows racing multiple futures and cancelling losers.

**Best Practices:**
1. Always provide cancellation path via shutdown signal
2. Include timeouts for external operations
3. Clean up resources in cancellation path
4. Log cancellation events for debugging

---

## Error Propagation Pattern
#async #rust #error #result #question-mark

Use `?` operator with `Result` for error propagation in async functions.

**Good Example:**
```rust
use anyhow::Result;

async fn fetch_and_process(url: &str) -> Result<ProcessedData> {
    let response = reqwest::get(url).await?;
    let data = response.json::<RawData>().await?;
    let processed = process_data(data)?;
    Ok(processed)
}
```

**Bad Example:**
```rust
// Manual error handling obscures logic
async fn fetch_and_process_bad(url: &str) -> ProcessedData {
    match reqwest::get(url).await {
        Ok(response) => {
            match response.json::<RawData>().await {
                Ok(data) => {
                    match process_data(data) {
                        Ok(processed) => processed,
                        Err(e) => panic!("Error: {}", e),
                    }
                }
                Err(e) => panic!("Error: {}", e),
            }
        }
        Err(e) => panic!("Error: {}", e),
    }
}
```

**Rationale:** The `?` operator provides clean error propagation. Paired with `anyhow` or `thiserror`, it maintains error context while keeping code readable.

---

## Structured Concurrency Pattern
#async #rust #structured-concurrency #join #tokio-join

Use `tokio::join!` or `tokio::try_join!` for structured concurrency where all tasks must complete.

**Good Example:**
```rust
use tokio;

async fn parallel_fetch(urls: Vec<String>) -> Result<Vec<Data>> {
    let mut futures = Vec::new();

    for url in urls {
        futures.push(fetch_data(&url));
    }

    // All futures complete together
    let results = tokio::try_join_all(futures).await?;
    Ok(results)
}
```

**Bad Example:**
```rust
// Spawning tasks without waiting
async fn unstructured_fetch(urls: Vec<String>) {
    for url in urls {
        tokio::spawn(async move {
            fetch_data(&url).await;
        });
    }
    // Returns before tasks complete!
}
```

**Rationale:** Structured concurrency ensures all child tasks complete before parent returns. This prevents resource leaks and makes error handling predictable.

**Choose Between:**
- `tokio::join!`: For fixed number of independent futures
- `tokio::try_join!`: When any error should cancel others
- `tokio::spawn` + `JoinHandle`: When tasks must outlive parent
- `tokio::task::JoinSet`: For dynamic task sets

---

## Related Patterns

See also:
- [[error-handling]] - Error type design
- [[testing]] - Testing async code
- [[performance]] - Async performance optimization
- [[timeout-strategies]] - Timeout and retry patterns
