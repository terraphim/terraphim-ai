# Terraphim Build Argument Management

This package provides centralized management of build arguments for the Terraphim AI project. It supports configuration of build features, targets, environments, and more, allowing for fine-grained control over the build process.

## Key Features

- Feature flag management
- Build target configuration
- Environment-specific variables
- Docker and Earthly integration
- Cross-compilation support
- Flexible error handling

## Usage

Here's a basic example of using the Terraphim Build Argument Management:

```rust
use terraphim_build_args::{BuildConfig, BuildTarget, FeatureSet};

fn main() {
    // Create a build configuration using the builder pattern
    let config = BuildConfig::builder()
        .target(BuildTarget::NativeRelease)
        .features(FeatureSet::from_string("openrouter,typescript").unwrap())
        .environment("production")
        .build()
        .unwrap();

    // Generate Cargo arguments
    let cargo_args = config.cargo_args();
    println!("Cargo args: {:?}", cargo_args);
}
```

## Modules

- `config`: Configuration management for build settings
- `error`: Error handling utilities
- `features`: Feature flag management
- `targets`: Build target definitions and validation
- `environment`: Environment-specific variable management
- `generator`: Command generation for build systems
- `validation`: Validation logic for configurations

## License

This project is licensed under the Apache License, Version 2.0.
