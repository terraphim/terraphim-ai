# Rust Error Handling Patterns

## Custom Error Types with thiserror
#rust #error #thiserror #error-handling #custom-errors

Use `thiserror` for custom error types with clear error messages and automatic `From` implementations.

**Good Example:**
```rust
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ServiceError {
    #[error("Database connection failed: {0}")]
    DatabaseConnection(#[from] sqlx::Error),
    
    #[error("User not found: {user_id}")]
    UserNotFound { user_id: u64 },
    
    #[error("Invalid configuration: {field} is missing")]
    InvalidConfig { field: String },
    
    #[error("Network error: {0}")]
    Network(#[from] reqwest::Error),
    
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

pub type Result<T> = std::result::Result<T, ServiceError>;
```

**Bad Example:**
```rust
// String-based errors lose type information
pub type Result<T> = std::result::Result<T, String>;

fn do_thing() -> Result<()> {
    Err("something went wrong".to_string())
}
```

**Rationale:** Typed errors enable proper error handling at different levels. `thiserror` derives reduce boilerplate while maintaining clarity.

**Best Practices:**
1. Create one error type per module or subsystem
2. Use `#[from]` for automatic conversion from dependency errors
3. Include contextual data in error variants
4. Use descriptive error messages with `#[error("...")]`

---

## Result Propagation Pattern
#rust #error #result #question-mark #error-propagation

Use `?` operator for clean error propagation. Add context with `map_err` or `context` when needed.

**Good Example:**
```rust
use anyhow::{Result, Context};

async fn process_user_request(user_id: u64) -> Result<Response> {
    let user = fetch_user(user_id).await
        .context("Failed to fetch user data")?;
    
    let permissions = fetch_permissions(&user).await
        .context("Failed to fetch user permissions")?;
    
    let data = generate_response(&user, &permissions)
        .context("Failed to generate response")?;
    
    Ok(data)
}
```

**Bad Example:**
```rust
// Swallowing errors or converting to panics
async fn process_user_request_bad(user_id: u64) -> Response {
    let user = fetch_user(user_id).await.unwrap();
    let permissions = fetch_permissions(&user).await.unwrap();
    let data = generate_response(&user, &permissions).unwrap();
    data
}
```

**Rationale:** Error context helps debugging without cluttering the happy path. `anyhow::Context` adds descriptive error chains.

---

## Error Recovery Pattern
#rust #error #recovery #fallback #resilience

Implement fallback strategies for recoverable errors.

**Good Example:**
```rust
async fn fetch_with_fallback(primary_url: &str, fallback_url: &str) -> Result<Data> {
    match fetch_data(primary_url).await {
        Ok(data) => Ok(data),
        Err(primary_error) => {
            log::warn!("Primary fetch failed: {}, trying fallback", primary_error);
            
            fetch_data(fallback_url).await
                .map_err(|fallback_error| {
                    anyhow::anyhow!(
                        "Both primary and fallback failed. Primary: {}, Fallback: {}",
                        primary_error,
                        fallback_error
                    )
                })
        }
    }
}
```

**Bad Example:**
```rust
// Fails immediately without retry or fallback
async fn fetch_no_fallback(url: &str) -> Result<Data> {
    fetch_data(url).await
}
```

**Rationale:** Network and external service errors are often transient. Fallback strategies improve reliability.

**Strategies:**
1. **Fallback source**: Alternative data source
2. **Retry with backoff**: Exponential backoff for transient failures
3. **Default value**: Return sensible default when data unavailable
4. **Cached value**: Return stale data with warning

---

## Early Return Pattern
#rust #error #early-return #validation #guard-clause

Validate inputs early and return errors before expensive operations.

**Good Example:**
```rust
fn process_order(order: Order) -> Result<Receipt> {
    if order.items.is_empty() {
        return Err(ServiceError::InvalidOrder { 
            reason: "Order has no items".into() 
        });
    }
    
    if order.total_amount <= 0.0 {
        return Err(ServiceError::InvalidOrder { 
            reason: "Order total must be positive".into() 
        });
    }
    
    let validated = validate_inventory(&order)?;
    let processed = charge_customer(&validated)?;
    let receipt = generate_receipt(&processed)?;
    
    Ok(receipt)
}
```

**Bad Example:**
```rust
// Validates after expensive operations
fn process_order_bad(order: Order) -> Result<Receipt> {
    let processed = charge_customer(&order)?;
    let receipt = generate_receipt(&processed)?;
    
    if order.items.is_empty() {
        return Err(ServiceError::InvalidOrder { 
            reason: "Order has no items".into() 
        });
    }
    
    Ok(receipt)
}
```

**Rationale:** Fail fast on invalid input before consuming resources. This improves performance and prevents partial state changes.

---

## Error Logging Pattern
#rust #error #logging #tracing #observability

Log errors at appropriate levels with context.

**Good Example:**
```rust
use tracing::{error, warn, debug, instrument};

#[instrument(skip(db))]
async fn process_batch(db: &Database, batch_id: u64) -> Result<()> {
    debug!("Starting batch processing");
    
    let items = match db.fetch_batch(batch_id).await {
        Ok(items) => items,
        Err(e) => {
            error!(
                batch_id = %batch_id,
                error = %e,
                "Failed to fetch batch"
            );
            return Err(e.into());
        }
    };
    
    for item in items {
        if let Err(e) = process_item(&item).await {
            warn!(
                item_id = %item.id,
                error = %e,
                "Failed to process item, continuing with next"
            );
        }
    }
    
    Ok(())
}
```

**Bad Example:**
```rust
// No context, wrong log levels
async fn process_batch_bad(db: &Database, batch_id: u64) -> Result<()> {
    let items = db.fetch_batch(batch_id).await
        .map_err(|e| {
            println!("Error: {}", e);  // println! instead of logger
            e
        })?;
    
    for item in items {
        if let Err(e) = process_item(&item).await {
            error!("Error: {}", e);  // Missing context
        }
    }
    
    Ok(())
}
```

**Rationale:** Structured logging with context enables debugging in production. Different log levels indicate severity.

**Log Levels:**
- `error!`: Failures that prevent operation completion
- `warn!`: Recoverable errors or unexpected conditions
- `info!`: Important state changes
- `debug!`: Detailed diagnostic information

---

## Result vs Option Pattern
#rust #error #result #option #type-choice

Choose between `Result` and `Option` based on whether absence is an error.

**Use Result When:**
```rust
// Failure needs explanation
fn parse_config(path: &Path) -> Result<Config> {
    let content = fs::read_to_string(path)
        .context("Failed to read config file")?;
    
    serde_json::from_str(&content)
        .context("Failed to parse config JSON")
}
```

**Use Option When:**
```rust
// Absence is normal, not an error
fn find_user(id: u64) -> Option<User> {
    CACHE.get(&id)
}

// Convert to Result when context is needed
fn find_user_required(id: u64) -> Result<User> {
    find_user(id)
        .ok_or_else(|| ServiceError::UserNotFound { user_id: id })
}
```

**Rationale:** `Option` signals that absence is expected. `Result` signals that absence is an error requiring explanation.

---

## Related Patterns

See also:
- [[async-patterns]] - Error handling in async contexts
- [[testing]] - Testing error conditions
- [[logging]] - Structured logging
- [[resilience]] - Circuit breakers and retry patterns
