# Rust Programming Guide #rust

This is a comprehensive guide about Rust programming language.

## Memory Safety #rust

Rust provides memory safety without garbage collection. This is achieved through:
- Ownership system
- Borrowing and references
- Lifetimes

## Cargo Package Manager #rust #tools

Cargo is Rust's build system and package manager that helps you:
- Build your project
- Download dependencies
- Run tests

## Testing in Rust #rust #test

Rust has built-in support for testing:

```rust
#[test]
fn it_works() {
    assert_eq!(2 + 2, 4);
}
```

This document should appear when searching with tag filter "#rust".