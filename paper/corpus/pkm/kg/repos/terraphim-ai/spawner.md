# Spawner

Agent process spawning with fallback provider support. spawn_with_fallback tries primary provider/model then falls back to configured fallback. Applies resource limits (CPU, memory, file size, open files) via setrlimit through nix crate. Pre-exec hooks for environment setup.

synonyms:: agent spawner, spawn, process management, terraphim_spawner, spawn_with_fallback, resource limits
