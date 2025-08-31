---
File: modelcontextprotocol-rust-sdk-13e72ec/.devcontainer/devcontainer.json
---

// For format details, see https://aka.ms/devcontainer.json. For config options, see the
// README at: https://github.com/devcontainers/templates/tree/main/src/rust
{
	"name": "Rust",
	// Or use a Dockerfile or Docker Compose file. More info: https://containers.dev/guide/dockerfile
	"image": "mcr.microsoft.com/devcontainers/rust:1-1-bullseye",
	"features": {
		"ghcr.io/devcontainers/features/node:1": {},
		"ghcr.io/devcontainers/features/python:1": {
			"version": "3.10",
			"toolsToInstall": "uv"
		}
	},
	// Configure tool-specific properties.
	"customizations": {
		"vscode": {
			"settings": {
				"editor.formatOnSave": true,
				"[rust]": {
					"editor.defaultFormatter": "rust-lang.rust-analyzer"
				}
			}
		}
	},
	// Use 'postCreateCommand' to run commands after the container is created.
	"postCreateCommand": "uv venv"
	// Use 'mounts' to make the cargo cache persistent in a Docker Volume.
	// "mounts": [
	// 	{
	// 		"source": "devcontainer-cargo-cache-${devcontainerId}",
	// 		"target": "/usr/local/cargo",
	// 		"type": "volume"
	// 	}
	// ]
	// Features to add to the dev container. More info: https://containers.dev/features.
	// "features": {},
	// Use 'forwardPorts' to make a list of ports inside the container available locally.
	// "forwardPorts": [],
	// Uncomment to connect as root instead. More info: https://aka.ms/dev-containers-non-root.
	// "remoteUser": "root"
}

---
File: modelcontextprotocol-rust-sdk-13e72ec/.github/dependabot.yml
---

version: 2
updates:
  - package-ecosystem: "cargo"
    directory: "/"
    schedule:
      interval: "weekly"
    labels:
      - T-dependencies
    open-pull-requests-limit: 3
    commit-message:
      prefix: "chore"
      include: "scope"

  # Ensure that references to actions in a repository's workflow.yml file are kept up to date.
  # See https://docs.github.com/en/code-security/supply-chain-security/keeping-your-dependencies-updated-automatically/keeping-your-actions-up-to-date-with-dependabot
  - package-ecosystem: "github-actions"
    directory: "/"
    schedule:
      interval: "daily"
    labels:
      # Mark PRs as CI related change.
      - T-CI
    open-pull-requests-limit: 3
    commit-message:
      prefix: "chore"
      include: "scope"

---
File: modelcontextprotocol-rust-sdk-13e72ec/.github/labeler.yml
---

# Configuration for GitHub Labeler Action
# https://github.com/actions/labeler

# Documentation changes
T-documentation:
  - changed-files:
    - any-glob-to-any-file: ['**/*.md', 'docs/**/*', '**/*.rst']

# Core library changes
T-core:
  - changed-files:
    - any-glob-to-any-file: ['crates/rmcp/src/**/*']

# Macro changes
T-macros:
  - changed-files:
    - any-glob-to-any-file: ['crates/rmcp-macros/**/*']

# Example changes
T-examples:
  - changed-files:
    - any-glob-to-any-file: ['examples/**/*']

# CI/CD changes
T-CI:
  - changed-files:
    - any-glob-to-any-file: ['.github/**/*', '**/*.yml', '**/*.yaml', '**/Dockerfile*']

# Dependencies
T-dependencies:
  - changed-files:
    - any-glob-to-any-file: ['**/Cargo.toml', '**/Cargo.lock', '**/package.json', '**/package-lock.json', '**/requirements.txt', '**/pyproject.toml']

# Tests
T-test:
  - changed-files:
    - any-glob-to-any-file: ['**/tests/**/*', '**/*test*.rs', '**/benches/**/*']

# Transport layer changes
T-transport:
  - changed-files:
    - any-glob-to-any-file: ['crates/rmcp/src/transport/**/*']

# Service layer changes
T-service:
  - changed-files:
    - any-glob-to-any-file: ['crates/rmcp/src/service/**/*']

# Handler changes
T-handler:
  - changed-files:
    - any-glob-to-any-file: ['crates/rmcp/src/handler/**/*']

# Model changes
T-model:
  - changed-files:
    - any-glob-to-any-file: ['crates/rmcp/src/model/**/*']

# Configuration files
T-config:
  - changed-files:
    - any-glob-to-any-file: ['**/*.toml', '**/*.json', '**/*.yaml', '**/*.yml', '**/*.env*']

# Security related files
T-security:
  - changed-files:
    - any-glob-to-any-file: ['**/security.md', '**/SECURITY.md', '**/audit.toml']

---
File: modelcontextprotocol-rust-sdk-13e72ec/.github/workflows/auto-label-pr.yml
---

name: Auto Label PR
on:
  # Runs workflow when activity on a PR in the workflow's repository occurs.
  pull_request_target:

jobs:
  auto-label:
    permissions:
      contents: read
      pull-requests: write
      issues: write

    name: Assign labels
    runs-on: ubuntu-latest
    timeout-minutes: 5

    # Required by gh
    env:
      GITHUB_TOKEN: ${{ secrets.GITHUB_TOKEN }}
      PR_URL: ${{ github.event.pull_request.html_url }}

    steps:
    - uses: actions/labeler@v5
      with:
        # Auto-include paths starting with dot (e.g. .github)
        dot: true
        # Remove labels when matching files are reverted or no longer changed by the PR
        sync-labels: true

---
File: modelcontextprotocol-rust-sdk-13e72ec/.github/workflows/ci.yml
---

name: CI

on:
  push:
    branches: [ main, release ]
    tags:
      - 'release-*'
  pull_request:
    branches: [ main, release ]

env:
  CARGO_TERM_COLOR: always
  ARTIFACT_DIR: release-artifacts

jobs:
  commit-lint:
    name: Lint Commit Messages
    runs-on: ubuntu-latest
    if: github.event_name == 'pull_request'
    steps:
      - uses: actions/checkout@v4
        with:
          fetch-depth: 0
      - name: Setup Node.js
        uses: actions/setup-node@v4
        with:
          node-version: '20'
      - name: Install commitlint
        run: |
          npm install --save-dev @commitlint/cli @commitlint/config-conventional
          echo "module.exports = {extends: ['@commitlint/config-conventional']}" > commitlint.config.js
      - name: Lint commit messages
        run: npx commitlint --from ${{ github.event.pull_request.base.sha }} --to ${{ github.event.pull_request.head.sha }} --verbose

  fmt:
    name: Code Formatting
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4

      - name: Install Rust fmt
        run: rustup toolchain install nightly --component rustfmt

      - name: Check formatting
        run: cargo +nightly fmt --all -- --check

  clippy:
    name: Lint with Clippy
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4

      - name: Install Rust
        uses: dtolnay/rust-toolchain@stable

      - uses: Swatinem/rust-cache@v2

      - name: Run clippy
        run: cargo clippy --all-targets --all-features -- -D warnings
  spelling:
    name: spell check with typos
    runs-on: ubuntu-latest
    steps:
    - uses: actions/checkout@v4
    - name: Spell Check Repo
      uses: crate-ci/typos@master

  test:
    name: Run Tests
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      # install nodejs
      - name: Setup Node.js
        uses: actions/setup-node@v4
        with:
          node-version: '20'
      - name: Install uv
        uses: astral-sh/setup-uv@v6
      - name: Install Rust
        uses: dtolnay/rust-toolchain@stable
      - name: Set up Python
        run: uv python install
      - name: Create venv for python
        run: uv venv
      - uses: Swatinem/rust-cache@v2

      - name: Run tests
        run: cargo test --all-features
  coverage:
    name: Code Coverage
    runs-on: ubuntu-latest
    permissions:
      contents: write
    steps:
      - uses: actions/checkout@v4

      # install nodejs
      - name: Setup Node.js
        uses: actions/setup-node@v4
        with:
          node-version: '20'
      - name: Install uv
        uses: astral-sh/setup-uv@v6
      - name: Install Rust
        uses: dtolnay/rust-toolchain@stable
      - name: Set up Python
        run: uv python install
      - name: Create venv for python
        run: uv venv

      - uses: Swatinem/rust-cache@v2

      - name: Install cargo-llvm-cov
        run: cargo install cargo-llvm-cov

      - name: Install llvm-tools-preview
        run: rustup component add llvm-tools-preview

      - name: Run tests with coverage
        run: cargo llvm-cov --all-features

  example-test:
    name: Example test
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4

      # install nodejs
      - name: Setup Node.js
        uses: actions/setup-node@v4
        with:
          node-version: '20'

      - name: Install uv
        uses: astral-sh/setup-uv@v6

      - name: Install Rust
        uses: dtolnay/rust-toolchain@stable

      - name: Set up Python
        run: uv python install

      - name: Create venv for python
        run: uv venv

      - uses: Swatinem/rust-cache@v2

      - name: Add target WASI preview 2
        run: |
          rustup target add wasm32-wasip2

      - name: Build examples
        run: |
          for dir in examples/*/ ; do
            if [ -f "$dir/Cargo.toml" ]; then
              echo "Building $dir"

              if [[ "$dir" == *"wasi"* ]]; then
                cargo build --manifest-path "$dir/Cargo.toml" --target wasm32-wasip2
              else
                cargo build --manifest-path "$dir/Cargo.toml" --all-features --tests
              fi
            fi
          done

      - name: Run tests in examples
        run: |
          # Tests are run for each subdirectory in the example directory.
          for dir in examples/*/ ; do
            if [ -f "$dir/Cargo.toml" ]; then
              if [[ "$dir" != *"wasi"* ]]; then
                echo "Testing $dir"
                cargo test --manifest-path "$dir/Cargo.toml" --all-features
              fi
            fi
          done

  security_audit:
    name: Security Audit
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4

      - name: Install Rust
        uses: dtolnay/rust-toolchain@stable
      - uses: Swatinem/rust-cache@v2

      - name: Install cargo-audit
        run: cargo install cargo-audit
      - name: Run cargo-audit
        run: cargo audit

  doc:
    name: Generate Documentation
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4

      - name: Install Rust
        uses: dtolnay/rust-toolchain@nightly

      - uses: Swatinem/rust-cache@v2

      - name: Generate documentation
        run: |
          cargo +nightly doc --no-deps -p rmcp -p rmcp-macros --all-features
        env:
          RUSTDOCFLAGS: --cfg docsrs -Dwarnings
          RUSTFLAGS: --cfg docsrs

  release:
    name: Release crates
    runs-on: ubuntu-latest
    if: github.ref == 'refs/heads/release' || startsWith(github.ref, 'refs/tags/release')
    needs: [fmt, clippy, test]
    steps:
      # Since this job has access to the `CRATES_TOKEN`, it's probably a good
      # idea to be extra careful about what Actions are being called. The reason
      # is that if an attacker gains access to other actions such as
      # `Swatinem/rust-cache`, they could use that to steal the `CRATES_TOKEN`.
      # This happened recently in the attack on `tj-actions/changed-files`, but
      # has happened many times before as well.

      - uses: actions/checkout@v4

      - name: Update Rust
        run: |
          rustup update stable
          rustup default stable

      - name: Cargo login
        run: cargo login ${{ secrets.CRATES_TOKEN }}
      - name: Publish macros dry run
        run: cargo publish -p rmcp-macros --dry-run
        continue-on-error: true
      - name: Publish rmcp dry run
        run: cargo publish -p rmcp --dry-run
        continue-on-error: true
      - name: Publish macro
        if: ${{ startsWith(github.ref, 'refs/tags/release') }}
        continue-on-error: true
        run: cargo publish -p rmcp-macros
      - name: Publish rmcp
        if: ${{ startsWith(github.ref, 'refs/tags/release') }}
        continue-on-error: true
        run: cargo publish -p rmcp

---
File: modelcontextprotocol-rust-sdk-13e72ec/.gitignore
---

# Generated by Cargo
# will have compiled files and executables
debug/
target/

# Remove Cargo.lock from gitignore if creating an executable, leave it for libraries
# More information here https://doc.rust-lang.org/cargo/guide/cargo-toml-vs-cargo-lock.html
Cargo.lock

# These are backup files generated by rustfmt
**/*.rs.bk

# MSVC Windows builds of rustc generate these, which store debugging information
*.pdb

# RustRover
#  JetBrains specific template is maintained in a separate JetBrains.gitignore that can
#  be found at https://github.com/github/gitignore/blob/main/Global/JetBrains.gitignore
#  and can be added to the global gitignore or merged into this file.  For a more nuclear
#  option (not recommended) you can uncomment the following to ignore the entire idea folder.
#.idea/

---
File: modelcontextprotocol-rust-sdk-13e72ec/Cargo.toml
---

[workspace]
members = ["crates/rmcp", "crates/rmcp-macros", "examples/*"]
resolver = "2"

[workspace.dependencies]
rmcp = { version = "0.1.5", path = "./crates/rmcp" }
rmcp-macros = { version = "0.1.5", path = "./crates/rmcp-macros" }

[workspace.package]
edition = "2024"
version = "0.1.5"
authors = ["4t145 <u4t145@163.com>"]
license = "MIT/Apache-2.0"
repository = "https://github.com/modelcontextprotocol/rust-sdk/"
description = "Rust SDK for Model Context Protocol"
keywords = ["mcp", "sdk", "tokio", "modelcontextprotocol"]
homepage = "https://github.com/modelcontextprotocol/rust-sdk"
categories = [
    "network-programming",
    "asynchronous",
]
readme = "README.md"

---
File: modelcontextprotocol-rust-sdk-13e72ec/LICENSE
---

MIT License

Copyright (c) 2025 Model Context Protocol

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE.

---
File: modelcontextprotocol-rust-sdk-13e72ec/README.md
---

<div align = "right">
<a href="docs/readme/README.zh-cn.md">简体中文(待更新)</a>
</div>

# RMCP
Wait for the first release.
<!-- [![Crates.io Version](todo)](todo) -->
<!-- ![Release status](https://github.com/modelcontextprotocol/rust-sdk/actions/workflows/release.yml/badge.svg) -->
<!-- [![docs.rs](todo)](todo) -->
![Coverage](docs/coverage.svg)

An official rust Model Context Protocol SDK implementation with tokio async runtime.


This repository contains the following crates:

- [rmcp](crates/rmcp): The core crate providing the RMCP protocol implementation( If you want to get more information, please visit [rmcp](crates/rmcp/README.md))
- [rmcp-macros](crates/rmcp-macros): A procedural macro crate for generating RMCP tool implementations(If you want to get more information, please visit [rmcp-macros](crates/rmcp-macros/README.md))

## Usage

### Import the crate

```toml
rmcp = { version = "0.1", features = ["server"] }
## or dev channel
rmcp = { git = "https://github.com/modelcontextprotocol/rust-sdk", branch = "main" }
```
### Third Dependencies
Basic dependencies:
- [tokio required](https://github.com/tokio-rs/tokio)
- [serde required](https://github.com/serde-rs/serde)



### Build a Client
<details>
<summary>Start a client</summary>

```rust, ignore
use rmcp::{ServiceExt, transport::{TokioChildProcess, ConfigureCommandExt}};
use tokio::process::Command;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = ().serve(TokioChildProcess::new(Command::new("npx").configure(|cmd| {
        cmd.arg("-y").arg("@modelcontextprotocol/server-everything");
    }))?).await?;
    Ok(())
}
```
</details>

### Build a Server

<details>
<summary>Build a transport</summary>

```rust, ignore
use tokio::io::{stdin, stdout};
let transport = (stdin(), stdout());
```

</details>

<details>
<summary>Build a service</summary>

You can easily build a service by using [`ServerHandler`](crates/rmcp/src/handler/server.rs) or [`ClientHandler`](crates/rmcp/src/handler/client.rs).

```rust, ignore
let service = common::counter::Counter::new();
```
</details>

<details>
<summary>Start the server</summary>

```rust, ignore
// this call will finish the initialization process
let server = service.serve(transport).await?;
```
</details>

<details>
<summary>Interact with the server</summary>

Once the server is initialized, you can send requests or notifications:

```rust, ignore
// request
let roots = server.list_roots().await?;

// or send notification
server.notify_cancelled(...).await?;
```
</details>

<details>
<summary>Waiting for service shutdown</summary>

```rust, ignore
let quit_reason = server.waiting().await?;
// or cancel it
let quit_reason = server.cancel().await?;
```
</details>


## Examples

See [examples](examples/README.md)

## OAuth Support

See [oauth_support](docs/OAUTH_SUPPORT.md) for details.


## Related Resources

- [MCP Specification](https://spec.modelcontextprotocol.io/specification/2024-11-05/)
- [Schema](https://github.com/modelcontextprotocol/specification/blob/main/schema/2024-11-05/schema.ts)

## Related Projects
- [containerd-mcp-server](https://github.com/jokemanfire/mcp-containerd) - A containerd-based MCP server implementation

## Development with Dev Container
See [docs/DEVCONTAINER.md](docs/DEVCONTAINER.md) for instructions on using Dev Container for development.

---
File: modelcontextprotocol-rust-sdk-13e72ec/clippy.toml
---

msrv = "1.85"
too-many-arguments-threshold = 10
check-private-items = false

---
File: modelcontextprotocol-rust-sdk-13e72ec/crates/rmcp-macros/Cargo.toml
---

[package]
name = "rmcp-macros"
license = { workspace = true }
version = { workspace = true }
edition = { workspace = true }
repository = { workspace = true }
homepage = { workspace = true }
readme = { workspace = true }
description = "Rust SDK for Model Context Protocol macros library"
documentation = "https://docs.rs/rmcp-macros"

[lib]
proc-macro = true

[dependencies]
syn = {version = "2", features = ["full"]}
quote = "1"
proc-macro2 = "1"
serde_json = "1.0"


[features]
[dev-dependencies]

---
File: modelcontextprotocol-rust-sdk-13e72ec/crates/rmcp-macros/README.md
---

# rmcp-macros

`rmcp-macros` is a procedural macro library for the Rust Model Context Protocol (RMCP) SDK, providing macros that facilitate the development of RMCP applications.

## Features

This library primarily provides the following macros:

- `#[tool]`: Used to mark functions as RMCP tools, automatically generating necessary metadata and invocation mechanisms

## Usage

### Tool Macro

Mark a function as a tool:

```rust ignore
#[tool]
fn calculator(&self, #[tool(param)] a: i32, #[tool(param)] b: i32) -> Result<CallToolResult, Error> {
    // Implement tool functionality
    Ok(CallToolResult::success(vec![Content::text((a + b).to_string())]))
}

```

Use on an impl block to automatically register multiple tools:

```rust ignore
#[tool(tool_box)]
impl MyHandler {
    #[tool]
    fn tool1(&self) -> Result<CallToolResult, Error> {
        // Tool 1 implementation
    }
    #[tool]
    fn tool2(&self) -> Result<CallToolResult, Error> {
        // Tool 2 implementation
    }
}
```



## Advanced Features

- Support for parameter aggregation (`#[tool(aggr)]`)
- Support for custom tool names and descriptions
- Automatic generation of tool descriptions from documentation comments
- JSON Schema generation for tool parameters

## License

Please refer to the LICENSE file in the project root directory.

---
File: modelcontextprotocol-rust-sdk-13e72ec/crates/rmcp-macros/src/lib.rs
---

#[allow(unused_imports)]
use proc_macro::TokenStream;

mod tool;

#[proc_macro_attribute]
pub fn tool(attr: TokenStream, input: TokenStream) -> TokenStream {
    tool::tool(attr.into(), input.into())
        .unwrap_or_else(|err| err.to_compile_error())
        .into()
}

---
File: modelcontextprotocol-rust-sdk-13e72ec/crates/rmcp-macros/src/tool.rs
---

use std::collections::HashSet;

use proc_macro2::TokenStream;
use quote::{ToTokens, quote};
use serde_json::json;
use syn::{
    Expr, FnArg, Ident, ItemFn, ItemImpl, Lit, MetaList, PatType, Token, Type, Visibility,
    parse::Parse, parse_quote, spanned::Spanned,
};

/// Stores tool annotation attributes
#[derive(Default, Clone)]
struct ToolAnnotationAttrs(pub serde_json::Map<String, serde_json::Value>);

impl Parse for ToolAnnotationAttrs {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let mut attrs = serde_json::Map::new();

        while !input.is_empty() {
            let key: Ident = input.parse()?;
            input.parse::<Token![:]>()?;
            let value: Lit = input.parse()?;
            let value = match value {
                Lit::Str(s) => json!(s.value()),
                Lit::Bool(b) => json!(b.value),
                _ => {
                    return Err(syn::Error::new(
                        key.span(),
                        "annotations must be string or boolean literals",
                    ));
                }
            };
            attrs.insert(key.to_string(), value);
            if input.is_empty() {
                break;
            }
            input.parse::<Token![,]>()?;
        }

        Ok(ToolAnnotationAttrs(attrs))
    }
}

#[derive(Default)]
struct ToolImplItemAttrs {
    tool_box: Option<Option<Ident>>,
}

impl Parse for ToolImplItemAttrs {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let mut tool_box = None;
        while !input.is_empty() {
            let key: Ident = input.parse()?;
            match key.to_string().as_str() {
                "tool_box" => {
                    tool_box = Some(None);
                    if input.lookahead1().peek(Token![=]) {
                        input.parse::<Token![=]>()?;
                        let value: Ident = input.parse()?;
                        tool_box = Some(Some(value));
                    }
                }
                _ => {
                    return Err(syn::Error::new(key.span(), "unknown attribute"));
                }
            }
            if input.is_empty() {
                break;
            }
            input.parse::<Token![,]>()?;
        }

        Ok(ToolImplItemAttrs { tool_box })
    }
}

#[derive(Default)]
struct ToolFnItemAttrs {
    name: Option<Expr>,
    description: Option<Expr>,
    vis: Option<Visibility>,
    annotations: Option<ToolAnnotationAttrs>,
}

impl Parse for ToolFnItemAttrs {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let mut name = None;
        let mut description = None;
        let mut vis = None;
        let mut annotations = None;

        while !input.is_empty() {
            let key: Ident = input.parse()?;
            input.parse::<Token![=]>()?;
            match key.to_string().as_str() {
                "name" => {
                    let value: Expr = input.parse()?;
                    name = Some(value);
                }
                "description" => {
                    let value: Expr = input.parse()?;
                    description = Some(value);
                }
                "vis" => {
                    let value: Visibility = input.parse()?;
                    vis = Some(value);
                }
                "annotations" => {
                    // Parse the annotations as a nested structure
                    let content;
                    syn::braced!(content in input);
                    let value = content.parse()?;
                    annotations = Some(value);
                }
                _ => {
                    return Err(syn::Error::new(key.span(), "unknown attribute"));
                }
            }
            if input.is_empty() {
                break;
            }
            input.parse::<Token![,]>()?;
        }

        Ok(ToolFnItemAttrs {
            name,
            description,
            vis,
            annotations,
        })
    }
}

struct ToolFnParamAttrs {
    serde_meta: Vec<MetaList>,
    schemars_meta: Vec<MetaList>,
    ident: Ident,
    rust_type: Box<Type>,
}

impl ToTokens for ToolFnParamAttrs {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let ident = &self.ident;
        let rust_type = &self.rust_type;
        let serde_meta = &self.serde_meta;
        let schemars_meta = &self.schemars_meta;
        tokens.extend(quote! {
            #(#[#serde_meta])*
            #(#[#schemars_meta])*
            pub #ident: #rust_type,
        });
    }
}

#[derive(Default)]

enum ToolParams {
    Aggregated {
        rust_type: PatType,
    },
    Params {
        attrs: Vec<ToolFnParamAttrs>,
    },
    #[default]
    NoParam,
}

#[derive(Default)]
struct ToolAttrs {
    fn_item: ToolFnItemAttrs,
    params: ToolParams,
}
const TOOL_IDENT: &str = "tool";
const SERDE_IDENT: &str = "serde";
const SCHEMARS_IDENT: &str = "schemars";
const PARAM_IDENT: &str = "param";
const AGGREGATED_IDENT: &str = "aggr";
const REQ_IDENT: &str = "req";

pub enum ParamMarker {
    Param,
    Aggregated,
}

impl Parse for ParamMarker {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let ident: Ident = input.parse()?;
        match ident.to_string().as_str() {
            PARAM_IDENT => Ok(ParamMarker::Param),
            AGGREGATED_IDENT | REQ_IDENT => Ok(ParamMarker::Aggregated),
            _ => Err(syn::Error::new(ident.span(), "unknown attribute")),
        }
    }
}

pub enum ToolItem {
    Fn(ItemFn),
    Impl(ItemImpl),
}

impl Parse for ToolItem {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let lookahead = input.lookahead1();
        if lookahead.peek(Token![impl]) {
            let item = input.parse::<ItemImpl>()?;
            Ok(ToolItem::Impl(item))
        } else {
            let item = input.parse::<ItemFn>()?;
            Ok(ToolItem::Fn(item))
        }
    }
}

// dispatch impl function item and impl block item
pub(crate) fn tool(attr: TokenStream, input: TokenStream) -> syn::Result<TokenStream> {
    let tool_item = syn::parse2::<ToolItem>(input)?;
    match tool_item {
        ToolItem::Fn(item) => tool_fn_item(attr, item),
        ToolItem::Impl(item) => tool_impl_item(attr, item),
    }
}

pub(crate) fn tool_impl_item(attr: TokenStream, mut input: ItemImpl) -> syn::Result<TokenStream> {
    let tool_impl_attr: ToolImplItemAttrs = syn::parse2(attr)?;
    let tool_box_ident = tool_impl_attr.tool_box;

    // get all tool function ident
    let mut tool_fn_idents = Vec::new();
    for item in &input.items {
        if let syn::ImplItem::Fn(method) = item {
            for attr in &method.attrs {
                if attr.path().is_ident(TOOL_IDENT) {
                    tool_fn_idents.push(method.sig.ident.clone());
                }
            }
        }
    }

    // handle different cases
    if input.trait_.is_some() {
        if let Some(ident) = tool_box_ident {
            // check if there are generic parameters
            if !input.generics.params.is_empty() {
                // for trait implementation with generic parameters, directly use the already generated *_inner method

                // generate call_tool method
                input.items.push(parse_quote! {
                    async fn call_tool(
                        &self,
                        request: rmcp::model::CallToolRequestParam,
                        context: rmcp::service::RequestContext<rmcp::RoleServer>,
                    ) -> Result<rmcp::model::CallToolResult, rmcp::Error> {
                        self.call_tool_inner(request, context).await
                    }
                });

                // generate list_tools method
                input.items.push(parse_quote! {
                    async fn list_tools(
                        &self,
                        request: Option<rmcp::model::PaginatedRequestParam>,
                        context: rmcp::service::RequestContext<rmcp::RoleServer>,
                    ) -> Result<rmcp::model::ListToolsResult, rmcp::Error> {
                        self.list_tools_inner(request, context).await
                    }
                });
            } else {
                // if there are no generic parameters, add tool box derive
                input.items.push(parse_quote!(
                    rmcp::tool_box!(@derive #ident);
                ));
            }
        } else {
            return Err(syn::Error::new(
                proc_macro2::Span::call_site(),
                "tool_box attribute is required for trait implementation",
            ));
        }
    } else if let Some(ident) = tool_box_ident {
        // if it is a normal impl block
        if !input.generics.params.is_empty() {
            // if there are generic parameters, not use tool_box! macro, but generate code directly

            // create call code for each tool function
            let match_arms = tool_fn_idents.iter().map(|ident| {
                let attr_fn = Ident::new(&format!("{}_tool_attr", ident), ident.span());
                let call_fn = Ident::new(&format!("{}_tool_call", ident), ident.span());
                quote! {
                    name if name == Self::#attr_fn().name => {
                        Self::#call_fn(tcc).await
                    }
                }
            });

            let tool_attrs = tool_fn_idents.iter().map(|ident| {
                let attr_fn = Ident::new(&format!("{}_tool_attr", ident), ident.span());
                quote! { Self::#attr_fn() }
            });

            // implement call_tool method
            input.items.push(parse_quote! {
                async fn call_tool_inner(
                    &self,
                    request: rmcp::model::CallToolRequestParam,
                    context: rmcp::service::RequestContext<rmcp::RoleServer>,
                ) -> Result<rmcp::model::CallToolResult, rmcp::Error> {
                    let tcc = rmcp::handler::server::tool::ToolCallContext::new(self, request, context);
                    match tcc.name() {
                        #(#match_arms,)*
                        _ => Err(rmcp::Error::invalid_params("tool not found", None)),
                    }
                }
            });

            // implement list_tools method
            input.items.push(parse_quote! {
                async fn list_tools_inner(
                    &self,
                    _: Option<rmcp::model::PaginatedRequestParam>,
                    _: rmcp::service::RequestContext<rmcp::RoleServer>,
                ) -> Result<rmcp::model::ListToolsResult, rmcp::Error> {
                    Ok(rmcp::model::ListToolsResult {
                        next_cursor: None,
                        tools: vec![#(#tool_attrs),*],
                    })
                }
            });
        } else {
            // if there are no generic parameters, use the original tool_box! macro
            let this_type_ident = &input.self_ty;
            input.items.push(parse_quote!(
                rmcp::tool_box!(#this_type_ident {
                    #(#tool_fn_idents),*
                } #ident);
            ));
        }
    }

    Ok(quote! {
        #input
    })
}

// extract doc line from attribute
fn extract_doc_line(attr: &syn::Attribute) -> Option<String> {
    if !attr.path().is_ident("doc") {
        return None;
    }

    let syn::Meta::NameValue(name_value) = &attr.meta else {
        return None;
    };

    let syn::Expr::Lit(expr_lit) = &name_value.value else {
        return None;
    };

    let syn::Lit::Str(lit_str) = &expr_lit.lit else {
        return None;
    };

    let content = lit_str.value().trim().to_string();

    (!content.is_empty()).then_some(content)
}

pub(crate) fn tool_fn_item(attr: TokenStream, mut input_fn: ItemFn) -> syn::Result<TokenStream> {
    let mut tool_macro_attrs = ToolAttrs::default();
    let args: ToolFnItemAttrs = syn::parse2(attr)?;
    tool_macro_attrs.fn_item = args;
    // let mut fommated_fn_args: Punctuated<FnArg, Comma> = Punctuated::new();
    let mut unextractable_args_indexes = HashSet::new();
    for (index, mut fn_arg) in input_fn.sig.inputs.iter_mut().enumerate() {
        enum Caught {
            Param(ToolFnParamAttrs),
            Aggregated(PatType),
        }
        let mut caught = None;
        match &mut fn_arg {
            FnArg::Receiver(_) => {
                continue;
            }
            FnArg::Typed(pat_type) => {
                let mut serde_metas = Vec::new();
                let mut schemars_metas = Vec::new();
                let mut arg_ident = match pat_type.pat.as_ref() {
                    syn::Pat::Ident(pat_ident) => Some(pat_ident.ident.clone()),
                    _ => None,
                };
                let raw_attrs: Vec<_> = pat_type.attrs.drain(..).collect();
                for attr in raw_attrs {
                    match &attr.meta {
                        syn::Meta::List(meta_list) => {
                            if meta_list.path.is_ident(TOOL_IDENT) {
                                let pat_type = pat_type.clone();
                                let marker = meta_list.parse_args::<ParamMarker>()?;
                                match marker {
                                    ParamMarker::Param => {
                                        let Some(arg_ident) = arg_ident.take() else {
                                            return Err(syn::Error::new(
                                                proc_macro2::Span::call_site(),
                                                "input param must have an ident as name",
                                            ));
                                        };
                                        caught.replace(Caught::Param(ToolFnParamAttrs {
                                            serde_meta: Vec::new(),
                                            schemars_meta: Vec::new(),
                                            ident: arg_ident,
                                            rust_type: pat_type.ty.clone(),
                                        }));
                                    }
                                    ParamMarker::Aggregated => {
                                        caught.replace(Caught::Aggregated(pat_type.clone()));
                                    }
                                }
                            } else if meta_list.path.is_ident(SERDE_IDENT) {
                                serde_metas.push(meta_list.clone());
                            } else if meta_list.path.is_ident(SCHEMARS_IDENT) {
                                schemars_metas.push(meta_list.clone());
                            } else {
                                pat_type.attrs.push(attr);
                            }
                        }
                        _ => {
                            pat_type.attrs.push(attr);
                        }
                    }
                }
                match caught {
                    Some(Caught::Param(mut param)) => {
                        param.serde_meta = serde_metas;
                        param.schemars_meta = schemars_metas;
                        match &mut tool_macro_attrs.params {
                            ToolParams::Params { attrs } => {
                                attrs.push(param);
                            }
                            _ => {
                                tool_macro_attrs.params = ToolParams::Params { attrs: vec![param] };
                            }
                        }
                        unextractable_args_indexes.insert(index);
                    }
                    Some(Caught::Aggregated(rust_type)) => {
                        if let ToolParams::Params { .. } = tool_macro_attrs.params {
                            return Err(syn::Error::new(
                                rust_type.span(),
                                "cannot mix aggregated and individual parameters",
                            ));
                        }
                        tool_macro_attrs.params = ToolParams::Aggregated { rust_type };
                        unextractable_args_indexes.insert(index);
                    }
                    None => {}
                }
            }
        }
    }

    // input_fn.sig.inputs = fommated_fn_args;
    let name = if let Some(expr) = tool_macro_attrs.fn_item.name {
        expr
    } else {
        let fn_name = &input_fn.sig.ident;
        parse_quote! {
            stringify!(#fn_name)
        }
    };
    let tool_attr_fn_ident = Ident::new(
        &format!("{}_tool_attr", input_fn.sig.ident),
        proc_macro2::Span::call_site(),
    );

    // generate get tool attr function
    let tool_attr_fn = {
        let description = if let Some(expr) = tool_macro_attrs.fn_item.description {
            // Use explicitly provided description if available
            expr
        } else {
            // Try to extract documentation comments
            let doc_content = input_fn
                .attrs
                .iter()
                .filter_map(extract_doc_line)
                .collect::<Vec<_>>()
                .join("\n");

            parse_quote! {
                    #doc_content.trim().to_string()
            }
        };
        let schema = match &tool_macro_attrs.params {
            ToolParams::Aggregated { rust_type } => {
                let ty = &rust_type.ty;
                let schema = quote! {
                    rmcp::handler::server::tool::cached_schema_for_type::<#ty>()
                };
                schema
            }
            ToolParams::Params { attrs, .. } => {
                let (param_type, temp_param_type_name) =
                    create_request_type(attrs, input_fn.sig.ident.to_string());
                let schema = quote! {
                    {
                        #param_type
                        rmcp::handler::server::tool::cached_schema_for_type::<#temp_param_type_name>()
                    }
                };
                schema
            }
            ToolParams::NoParam => {
                quote! {
                    rmcp::handler::server::tool::cached_schema_for_type::<rmcp::model::EmptyObject>()
                }
            }
        };
        let input_fn_attrs = &input_fn.attrs;
        let input_fn_vis = &input_fn.vis;

        let annotations_code = if let Some(annotations) = &tool_macro_attrs.fn_item.annotations {
            let annotations =
                serde_json::to_string(&annotations.0).expect("failed to serialize annotations");
            quote! {
                Some(serde_json::from_str::<rmcp::model::ToolAnnotations>(&#annotations).expect("Could not parse tool annotations"))
            }
        } else {
            quote! { None }
        };

        quote! {
            #(#input_fn_attrs)*
            #input_fn_vis fn #tool_attr_fn_ident() -> rmcp::model::Tool {
                rmcp::model::Tool {
                    name: #name.into(),
                    description: Some(#description.into()),
                    input_schema: #schema.into(),
                    annotations: #annotations_code,
                }
            }
        }
    };

    // generate wrapped tool function
    let tool_call_fn = {
        // wrapper function have the same sig:
        // async fn #tool_tool_call(context: rmcp::handler::server::tool::ToolCallContext<'_, Self>)
        //      -> std::result::Result<rmcp::model::CallToolResult, rmcp::Error>
        //
        // and the block part should be like:
        // {
        //      use rmcp::handler::server::tool::*;
        //      let (t0, context) = <T0>::from_tool_call_context_part(context)?;
        //      let (t1, context) = <T1>::from_tool_call_context_part(context)?;
        //      ...
        //      let (tn, context) = <Tn>::from_tool_call_context_part(context)?;
        //      // for params
        //      ... expand helper types here
        //      let (__rmcp_tool_req, context) = rmcp::model::JsonObject::from_tool_call_context_part(context)?;
        //      let __#TOOL_ToolCallParam { param_0, param_1, param_2, .. } = parse_json_object(__rmcp_tool_req)?;
        //      // for aggr
        //      let (Parameters(aggr), context) = <Parameters<AggrType>>::from_tool_call_context_part(context)?;
        //      Self::#tool_ident(to, param_0, t1, param_1, ..., param_2, tn, aggr).await.into_call_tool_result()
        //
        // }
        //
        //
        //

        // for receiver type, name it as __rmcp_tool_receiver
        let is_async = input_fn.sig.asyncness.is_some();
        let receiver_ident = || Ident::new("__rmcp_tool_receiver", proc_macro2::Span::call_site());
        // generate the extraction part for trivial args
        let trivial_args = input_fn
            .sig
            .inputs
            .iter()
            .enumerate()
            .filter_map(|(index, arg)| {
                if unextractable_args_indexes.contains(&index) {
                    None
                } else {
                    // get ident/type pair
                    let line = match arg {
                        FnArg::Typed(pat_type) => {
                            let pat = &pat_type.pat;
                            let ty = &pat_type.ty;
                            quote! {
                                let (#pat, context) = <#ty>::from_tool_call_context_part(context)?;
                            }
                        }
                        FnArg::Receiver(r) => {
                            let ty = r.ty.clone();
                            let pat = receiver_ident();
                            quote! {
                                let  (#pat, context) = <#ty>::from_tool_call_context_part(context)?;
                            }
                        }
                    };
                    Some(line)
                }
            });
        let trivial_arg_extraction_part = quote! {
            #(#trivial_args)*
        };
        let processed_arg_extraction_part = match &mut tool_macro_attrs.params {
            ToolParams::Aggregated { rust_type } => {
                let PatType { pat, ty, .. } = rust_type;
                quote! {
                    let (Parameters(#pat), context) = <Parameters<#ty>>::from_tool_call_context_part(context)?;
                }
            }
            ToolParams::Params { attrs } => {
                let (param_type, temp_param_type_name) =
                    create_request_type(attrs, input_fn.sig.ident.to_string());

                let params_ident = attrs.iter().map(|attr| &attr.ident).collect::<Vec<_>>();
                quote! {
                    #param_type
                    let (__rmcp_tool_req, context) = rmcp::model::JsonObject::from_tool_call_context_part(context)?;
                    let #temp_param_type_name {
                        #(#params_ident,)*
                    } = parse_json_object(__rmcp_tool_req)?;
                }
            }
            ToolParams::NoParam => {
                quote! {}
            }
        };
        // generate the execution part
        // has receiver?
        let params = &input_fn
            .sig
            .inputs
            .iter()
            .map(|fn_arg| match fn_arg {
                FnArg::Receiver(_) => {
                    let pat = receiver_ident();
                    quote! { #pat }
                }
                FnArg::Typed(pat_type) => {
                    let pat = &pat_type.pat.clone();
                    quote! { #pat }
                }
            })
            .collect::<Vec<_>>();
        let raw_fn_ident = &input_fn.sig.ident;
        let call = if is_async {
            quote! {
                Self::#raw_fn_ident(#(#params),*).await.into_call_tool_result()
            }
        } else {
            quote! {
                Self::#raw_fn_ident(#(#params),*).into_call_tool_result()
            }
        };
        // assemble the whole function
        let tool_call_fn_ident = Ident::new(
            &format!("{}_tool_call", input_fn.sig.ident),
            proc_macro2::Span::call_site(),
        );
        let raw_fn_vis = tool_macro_attrs
            .fn_item
            .vis
            .as_ref()
            .unwrap_or(&input_fn.vis);
        let raw_fn_attr = &input_fn
            .attrs
            .iter()
            .filter(|attr| !attr.path().is_ident(TOOL_IDENT))
            .collect::<Vec<_>>();
        quote! {
            #(#raw_fn_attr)*
            #raw_fn_vis async fn #tool_call_fn_ident(context: rmcp::handler::server::tool::ToolCallContext<'_, Self>)
                -> std::result::Result<rmcp::model::CallToolResult, rmcp::Error> {
                use rmcp::handler::server::tool::*;
                #trivial_arg_extraction_part
                #processed_arg_extraction_part
                #call
            }
        }
    };
    Ok(quote! {
        #tool_attr_fn
        #tool_call_fn
        #input_fn
    })
}

fn create_request_type(attrs: &[ToolFnParamAttrs], tool_name: String) -> (TokenStream, Ident) {
    let pascal_case_tool_name = tool_name.to_ascii_uppercase();
    let temp_param_type_name = Ident::new(
        &format!("__{pascal_case_tool_name}ToolCallParam",),
        proc_macro2::Span::call_site(),
    );
    (
        quote! {
            use rmcp::{serde, schemars};
            #[derive(serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
            pub struct #temp_param_type_name {
                #(#attrs)*
            }
        },
        temp_param_type_name,
    )
}

#[cfg(test)]
mod test {
    use super::*;
    #[test]
    fn test_tool_sync_macro() -> syn::Result<()> {
        let attr = quote! {
            name = "test_tool",
            description = "test tool",
            vis =
        };
        let input = quote! {
            fn sum(&self, #[tool(aggr)] req: StructRequest) -> Result<CallToolResult, McpError> {
                Ok(CallToolResult::success(vec![Content::text((req.a + req.b).to_string())]))
            }
        };
        let input = tool(attr, input)?;

        println!("input: {:#}", input);
        Ok(())
    }

    #[test]
    fn test_trait_tool_macro() -> syn::Result<()> {
        let attr = quote! {
            tool_box = Calculator
        };
        let input = quote! {
            impl ServerHandler for Calculator {
                #[tool]
                fn get_info(&self) -> ServerInfo {
                    ServerInfo {
                        instructions: Some("A simple calculator".into()),
                        ..Default::default()
                    }
                }
            }
        };
        let input = tool(attr, input)?;

        println!("input: {:#}", input);
        Ok(())
    }
    #[test]
    fn test_doc_comment_description() -> syn::Result<()> {
        let attr = quote! {}; // No explicit description
        let input = quote! {
            /// This is a test description from doc comments
            /// with multiple lines
            fn test_function(&self) -> Result<(), Error> {
                Ok(())
            }
        };
        let result = tool(attr, input)?;

        // The output should contain the description from doc comments
        let result_str = result.to_string();
        assert!(result_str.contains("This is a test description from doc comments"));
        assert!(result_str.contains("with multiple lines"));

        Ok(())
    }
    #[test]
    fn test_explicit_description_priority() -> syn::Result<()> {
        let attr = quote! {
            description = "Explicit description has priority"
        };
        let input = quote! {
            /// Doc comment description that should be ignored
            fn test_function(&self) -> Result<(), Error> {
                Ok(())
            }
        };
        let result = tool(attr, input)?;

        // The output should contain the explicit description
        let result_str = result.to_string();
        assert!(result_str.contains("Explicit description has priority"));
        Ok(())
    }
}

---
File: modelcontextprotocol-rust-sdk-13e72ec/crates/rmcp/Cargo.toml
---

[package]
name = "rmcp"
license = { workspace = true }
version = { workspace = true }
edition = { workspace = true }
repository = { workspace = true }
homepage = { workspace = true }
readme = { workspace = true }
description = "Rust SDK for Model Context Protocol"
documentation = "https://docs.rs/rmcp"

[package.metadata.docs.rs]
all-features = true
rustdoc-args = ["--cfg", "docsrs"]

[dependencies]
serde = { version = "1.0", features = ["derive", "rc"] }
serde_json = "1.0"
thiserror = "2"
tokio = { version = "1", features = ["sync", "macros", "rt", "time"] }
futures = "0.3"
tracing = { version = "0.1" }
tokio-util = { version = "0.7" }
pin-project-lite = "0.2"
paste = { version = "1", optional = true }

# oauth2 support
oauth2 = { version = "5.0", optional = true }

# for auto generate schema
schemars = { version = "0.8", optional = true, features = ["chrono"] }

# for image encoding
base64 = { version = "0.22", optional = true }

# for SSE client
reqwest = { version = "0.12", default-features = false, features = [
    "json",
    "stream",
], optional = true }

sse-stream = { version = "0.2", optional = true }

http = { version = "1", optional = true }
url = { version = "2.4", optional = true }

# For tower compatibility
tower-service = { version = "0.3", optional = true }

# for child process transport
process-wrap = { version = "8.2", features = ["tokio1"], optional = true }

# for ws transport
# tokio-tungstenite ={ version = "0.26", optional = true }

# for http-server transport
axum = { version = "0.8", features = [], optional = true }
rand = { version = "0.9", optional = true }
tokio-stream = { version = "0.1", optional = true }
uuid = { version = "1", features = ["v4"], optional = true }
http-body = { version = "1", optional = true }
http-body-util = { version = "0.1", optional = true }
bytes = { version = "1", optional = true }
# macro
rmcp-macros = { version = "0.1", workspace = true, optional = true }

[target.'cfg(not(all(target_family = "wasm", target_os = "unknown")))'.dependencies]
chrono = { version = "0.4.38", features = ["serde"] }

[target.'cfg(all(target_family = "wasm", target_os = "unknown"))'.dependencies]
chrono = { version = "0.4.38", default-features = false, features = ["serde", "clock", "std", "oldtime"] }

[features]
default = ["base64", "macros", "server"]
client = []
server = ["transport-async-rw", "dep:schemars"]
macros = ["dep:rmcp-macros", "dep:paste"]

# reqwest http client
__reqwest = ["dep:reqwest"]

reqwest = ["__reqwest", "reqwest?/rustls-tls"]

reqwest-tls-no-provider = ["__reqwest", "reqwest?/rustls-tls-no-provider"]

server-side-http = [
    "uuid",
    "dep:rand",
    "dep:tokio-stream",
    "dep:http",
    "dep:http-body",
    "dep:http-body-util",
    "dep:bytes",
    "dep:sse-stream",
    "tower",
]
# SSE client
client-side-sse = ["dep:sse-stream", "dep:http"]

transport-sse-client = ["client-side-sse", "transport-worker"]

transport-worker = ["dep:tokio-stream"]


# Streamable HTTP client
transport-streamable-http-client = ["client-side-sse", "transport-worker"]


transport-async-rw = ["tokio/io-util", "tokio-util/codec"]
transport-io = ["transport-async-rw", "tokio/io-std"]
transport-child-process = [
    "transport-async-rw",
    "tokio/process",
    "dep:process-wrap",
]
transport-sse-server = [
    "transport-async-rw",
    "transport-worker",
    "server-side-http",
    "dep:axum",
]
transport-streamable-http-server = [
    "transport-streamable-http-server-session",
    "server-side-http",
]
transport-streamable-http-server-session = [
    "transport-async-rw",
    "dep:tokio-stream",
]
# transport-ws = ["transport-io", "dep:tokio-tungstenite"]
tower = ["dep:tower-service"]
auth = ["dep:oauth2", "__reqwest", "dep:url"]
schemars = ["dep:schemars"]

[dev-dependencies]
tokio = { version = "1", features = ["full"] }
schemars = { version = "0.8" }

anyhow = "1.0"
tracing-subscriber = { version = "0.3", features = [
    "env-filter",
    "std",
    "fmt",
] }
async-trait = "0.1"
[[test]]
name = "test_tool_macros"
required-features = ["server", "client"]
path = "tests/test_tool_macros.rs"

[[test]]
name = "test_with_python"
required-features = [
    "reqwest",
    "server",
    "client",
    "transport-sse-server",
    "transport-sse-client",
    "transport-child-process",
]
path = "tests/test_with_python.rs"

[[test]]
name = "test_with_js"
required-features = [
    "server",
    "client",
    "transport-sse-server",
    "transport-child-process",
    "transport-streamable-http-server",
    "transport-streamable-http-client",
    "__reqwest",
]
path = "tests/test_with_js.rs"

[[test]]
name = "test_notification"
required-features = ["server", "client"]
path = "tests/test_notification.rs"

[[test]]
name = "test_logging"
required-features = ["server", "client"]
path = "tests/test_logging.rs"

[[test]]
name = "test_message_protocol"
required-features = ["client"]
path = "tests/test_message_protocol.rs"

[[test]]
name = "test_message_schema"
required-features = ["server", "client", "schemars"]
path = "tests/test_message_schema.rs"

---
File: modelcontextprotocol-rust-sdk-13e72ec/crates/rmcp/README.md
---

# RMCP: Rust Model Context Protocol

`rmcp` is the official Rust implementation of the Model Context Protocol (MCP), a protocol designed for AI assistants to communicate with other services. This library can be used to build both servers that expose capabilities to AI assistants and clients that interact with such servers.

wait for the first release.
<!-- [![Crates.io](todo)](todo)
[![Documentation](todo)](todo) -->



## Quick Start

### Server Implementation

Creating a server with tools is simple using the `#[tool]` macro:

```rust, ignore
use rmcp::{Error as McpError, ServiceExt, model::*, tool, transport::stdio};
use std::sync::Arc;
use tokio::sync::Mutex;

#[derive(Clone)]
pub struct Counter {
    counter: Arc<Mutex<i32>>,
}

#[tool(tool_box)]
impl Counter {
    fn new() -> Self {
        Self {
            counter: Arc::new(Mutex::new(0)),
        }
    }

    #[tool(description = "Increment the counter by 1")]
    async fn increment(&self) -> Result<CallToolResult, McpError> {
        let mut counter = self.counter.lock().await;
        *counter += 1;
        Ok(CallToolResult::success(vec![Content::text(
            counter.to_string(),
        )]))
    }

    #[tool(description = "Get the current counter value")]
    async fn get(&self) -> Result<CallToolResult, McpError> {
        let counter = self.counter.lock().await;
        Ok(CallToolResult::success(vec![Content::text(
            counter.to_string(),
        )]))
    }
}

// Implement the server handler
#[tool(tool_box)]
impl rmcp::ServerHandler for Counter {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            instructions: Some("A simple calculator".into()),
            capabilities: ServerCapabilities::builder().enable_tools().build(),
            ..Default::default()
        }
    }
}

// Run the server
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create and run the server with STDIO transport
    let service = Counter::new().serve(stdio()).await.inspect_err(|e| {
        println!("Error starting server: {}", e);
    })?;
    service.waiting().await?;

    Ok(())
}
```

### Client Implementation

Creating a client to interact with a server:

```rust, ignore
use rmcp::{
    model::CallToolRequestParam,
    service::ServiceExt,
    transport::{TokioChildProcess, ConfigureCommandExt}
};
use tokio::process::Command;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Connect to a server running as a child process
    let service = ()
    .serve(TokioChildProcess::new(Command::new("uvx").configure(
        |cmd| {
            cmd.arg("mcp-server-git");
        },
    ))?)
    .await?;

    // Get server information
    let server_info = service.peer_info();
    println!("Connected to server: {server_info:#?}");

    // List available tools
    let tools = service.list_tools(Default::default()).await?;
    println!("Available tools: {tools:#?}");

    // Call a tool
    let result = service
        .call_tool(CallToolRequestParam {
            name: "increment".into(),
            arguments: None,
        })
        .await?;
    println!("Result: {result:#?}");

    // Gracefully close the connection
    service.cancel().await?;
    Ok(())
}
```

## Transport Options

RMCP supports multiple transport mechanisms, each suited for different use cases:

### `transport-async-rw`
Low-level interface for asynchronous read/write operations. This is the foundation for many other transports.

### `transport-io`
For working directly with I/O streams (`tokio::io::AsyncRead` and `tokio::io::AsyncWrite`).

### `transport-child-process`
Run MCP servers as child processes and communicate via standard I/O.

Example:
```rust
use rmcp::transport::TokioChildProcess;
use tokio::process::Command;

let transport = TokioChildProcess::new(Command::new("mcp-server"))?;
let service = client.serve(transport).await?;
```



## Access with peer interface when handling message

You can get the [`Peer`](crate::service::Peer) struct from [`NotificationContext`](crate::service::NotificationContext) and [`RequestContext`](crate::service::RequestContext).

```rust, ignore
# use rmcp::{
#     ServerHandler,
#     model::{LoggingLevel, LoggingMessageNotificationParam, ProgressNotificationParam},
#     service::{NotificationContext, RoleServer},
# };
# pub struct Handler;

impl ServerHandler for Handler {
    async fn on_progress(
        &self,
        notification: ProgressNotificationParam,
        context: NotificationContext<RoleServer>,
    ) {
        let peer = context.peer;
        let _ = peer
            .notify_logging_message(LoggingMessageNotificationParam {
                level: LoggingLevel::Info,
                logger: None,
                data: serde_json::json!({
                    "message": format!("Progress: {}", notification.progress),
                }),
            })
            .await;
    }
}
```


## Manage Multi Services

For many cases you need to manage several service in a collection, you can call `into_dyn` to convert services into the same type.
```rust, ignore
let service = service.into_dyn();
```

## Feature Flags

RMCP uses feature flags to control which components are included:

- `client`: Enable client functionality
- `server`: Enable server functionality and the tool system
- `macros`: Enable the `#[tool]` macro (enabled by default)
- Transport-specific features:
  - `transport-async-rw`: Async read/write support
  - `transport-io`: I/O stream support
  - `transport-child-process`: Child process support
  - `transport-sse-client` / `transport-sse-server`: SSE support
  - `transport-streamable-http-client` / `transport-streamable-http-server`: HTTP streaming
- `auth`: OAuth2 authentication support
- `schemars`: JSON Schema generation (for tool definitions)


## Transports

- `transport-io`: Server stdio transport
- `transport-sse-server`: Server SSE transport
- `transport-child-process`: Client stdio transport
- `transport-sse-client`: Client sse transport
- `transport-streamable-http-server` streamable http server transport
- `transport-streamable-http-client` streamable http client transport

<details>
<summary>Transport</summary>
The transport type must implemented [`Transport`] trait, which allow it send message concurrently and receive message sequentially.
There are 3 pairs of standard transport types:

| transport         | client                                                    | server                                                |
|:-:                |:-:                                                        |:-:                                                    |
| std IO            | [`child_process::TokioChildProcess`]                      | [`io::stdio`]                                         |
| streamable http   | [`streamable_http_client::StreamableHttpClientTransport`] | [`streamable_http_server::session::create_session`]   |
| sse               | [`sse_client::SseClientTransport`]                        | [`sse_server::SseServer`]                             |

#### [IntoTransport](`IntoTransport`) trait
[`IntoTransport`] is a helper trait that implicitly convert a type into a transport type.

These types is automatically implemented [`IntoTransport`] trait
1. A type that already implement both [`futures::Sink`] and [`futures::Stream`] trait, or a tuple `(Tx, Rx)`  where `Tx` is [`futures::Sink`] and `Rx` is [`futures::Stream`].
2. A type that implement both [`tokio::io::AsyncRead`] and [`tokio::io::AsyncWrite`] trait. or a tuple `(R, W)` where `R` is [`tokio::io::AsyncRead`] and `W` is [`tokio::io::AsyncWrite`].
3. A type that implement [Worker](`worker::Worker`) trait.
4. A type that implement [`Transport`] trait.

</details>

## License

This project is licensed under the terms specified in the repository's LICENSE file.

---
File: modelcontextprotocol-rust-sdk-13e72ec/crates/rmcp/src/error.rs
---

use std::fmt::Display;

use crate::model::ErrorData;

pub type Error = ErrorData;

impl Display for ErrorData {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.code.0, self.message)?;
        if let Some(data) = &self.data {
            write!(f, "({})", data)?;
        }
        Ok(())
    }
}

impl std::error::Error for ErrorData {}

---
File: modelcontextprotocol-rust-sdk-13e72ec/crates/rmcp/src/handler.rs
---

#[cfg(feature = "client")]
#[cfg_attr(docsrs, doc(cfg(feature = "client")))]
pub mod client;
#[cfg(feature = "server")]
#[cfg_attr(docsrs, doc(cfg(feature = "server")))]
pub mod server;

---
File: modelcontextprotocol-rust-sdk-13e72ec/crates/rmcp/src/handler/client.rs
---

use crate::{
    error::Error as McpError,
    model::*,
    service::{NotificationContext, RequestContext, RoleClient, Service, ServiceRole},
};

impl<H: ClientHandler> Service<RoleClient> for H {
    async fn handle_request(
        &self,
        request: <RoleClient as ServiceRole>::PeerReq,
        context: RequestContext<RoleClient>,
    ) -> Result<<RoleClient as ServiceRole>::Resp, McpError> {
        match request {
            ServerRequest::PingRequest(_) => self.ping(context).await.map(ClientResult::empty),
            ServerRequest::CreateMessageRequest(request) => self
                .create_message(request.params, context)
                .await
                .map(ClientResult::CreateMessageResult),
            ServerRequest::ListRootsRequest(_) => self
                .list_roots(context)
                .await
                .map(ClientResult::ListRootsResult),
        }
    }

    async fn handle_notification(
        &self,
        notification: <RoleClient as ServiceRole>::PeerNot,
        context: NotificationContext<RoleClient>,
    ) -> Result<(), McpError> {
        match notification {
            ServerNotification::CancelledNotification(notification) => {
                self.on_cancelled(notification.params, context).await
            }
            ServerNotification::ProgressNotification(notification) => {
                self.on_progress(notification.params, context).await
            }
            ServerNotification::LoggingMessageNotification(notification) => {
                self.on_logging_message(notification.params, context).await
            }
            ServerNotification::ResourceUpdatedNotification(notification) => {
                self.on_resource_updated(notification.params, context).await
            }
            ServerNotification::ResourceListChangedNotification(_notification_no_param) => {
                self.on_resource_list_changed(context).await
            }
            ServerNotification::ToolListChangedNotification(_notification_no_param) => {
                self.on_tool_list_changed(context).await
            }
            ServerNotification::PromptListChangedNotification(_notification_no_param) => {
                self.on_prompt_list_changed(context).await
            }
        };
        Ok(())
    }

    fn get_info(&self) -> <RoleClient as ServiceRole>::Info {
        self.get_info()
    }
}

#[allow(unused_variables)]
pub trait ClientHandler: Sized + Send + Sync + 'static {
    fn ping(
        &self,
        context: RequestContext<RoleClient>,
    ) -> impl Future<Output = Result<(), McpError>> + Send + '_ {
        std::future::ready(Ok(()))
    }

    fn create_message(
        &self,
        params: CreateMessageRequestParam,
        context: RequestContext<RoleClient>,
    ) -> impl Future<Output = Result<CreateMessageResult, McpError>> + Send + '_ {
        std::future::ready(Err(
            McpError::method_not_found::<CreateMessageRequestMethod>(),
        ))
    }

    fn list_roots(
        &self,
        context: RequestContext<RoleClient>,
    ) -> impl Future<Output = Result<ListRootsResult, McpError>> + Send + '_ {
        std::future::ready(Ok(ListRootsResult::default()))
    }

    fn on_cancelled(
        &self,
        params: CancelledNotificationParam,
        context: NotificationContext<RoleClient>,
    ) -> impl Future<Output = ()> + Send + '_ {
        std::future::ready(())
    }
    fn on_progress(
        &self,
        params: ProgressNotificationParam,
        context: NotificationContext<RoleClient>,
    ) -> impl Future<Output = ()> + Send + '_ {
        std::future::ready(())
    }
    fn on_logging_message(
        &self,
        params: LoggingMessageNotificationParam,
        context: NotificationContext<RoleClient>,
    ) -> impl Future<Output = ()> + Send + '_ {
        std::future::ready(())
    }
    fn on_resource_updated(
        &self,
        params: ResourceUpdatedNotificationParam,
        context: NotificationContext<RoleClient>,
    ) -> impl Future<Output = ()> + Send + '_ {
        std::future::ready(())
    }
    fn on_resource_list_changed(
        &self,
        context: NotificationContext<RoleClient>,
    ) -> impl Future<Output = ()> + Send + '_ {
        std::future::ready(())
    }
    fn on_tool_list_changed(
        &self,
        context: NotificationContext<RoleClient>,
    ) -> impl Future<Output = ()> + Send + '_ {
        std::future::ready(())
    }
    fn on_prompt_list_changed(
        &self,
        context: NotificationContext<RoleClient>,
    ) -> impl Future<Output = ()> + Send + '_ {
        std::future::ready(())
    }

    fn get_info(&self) -> ClientInfo {
        ClientInfo::default()
    }
}

/// Do nothing, with default client info.
impl ClientHandler for () {}

/// Do nothing, with a specific client info.
impl ClientHandler for ClientInfo {
    fn get_info(&self) -> ClientInfo {
        self.clone()
    }
}

---
File: modelcontextprotocol-rust-sdk-13e72ec/crates/rmcp/src/handler/server.rs
---

use crate::{
    error::Error as McpError,
    model::*,
    service::{NotificationContext, RequestContext, RoleServer, Service, ServiceRole},
};

mod resource;
pub mod tool;
pub mod wrapper;
impl<H: ServerHandler> Service<RoleServer> for H {
    async fn handle_request(
        &self,
        request: <RoleServer as ServiceRole>::PeerReq,
        context: RequestContext<RoleServer>,
    ) -> Result<<RoleServer as ServiceRole>::Resp, McpError> {
        match request {
            ClientRequest::InitializeRequest(request) => self
                .initialize(request.params, context)
                .await
                .map(ServerResult::InitializeResult),
            ClientRequest::PingRequest(_request) => {
                self.ping(context).await.map(ServerResult::empty)
            }
            ClientRequest::CompleteRequest(request) => self
                .complete(request.params, context)
                .await
                .map(ServerResult::CompleteResult),
            ClientRequest::SetLevelRequest(request) => self
                .set_level(request.params, context)
                .await
                .map(ServerResult::empty),
            ClientRequest::GetPromptRequest(request) => self
                .get_prompt(request.params, context)
                .await
                .map(ServerResult::GetPromptResult),
            ClientRequest::ListPromptsRequest(request) => self
                .list_prompts(request.params, context)
                .await
                .map(ServerResult::ListPromptsResult),
            ClientRequest::ListResourcesRequest(request) => self
                .list_resources(request.params, context)
                .await
                .map(ServerResult::ListResourcesResult),
            ClientRequest::ListResourceTemplatesRequest(request) => self
                .list_resource_templates(request.params, context)
                .await
                .map(ServerResult::ListResourceTemplatesResult),
            ClientRequest::ReadResourceRequest(request) => self
                .read_resource(request.params, context)
                .await
                .map(ServerResult::ReadResourceResult),
            ClientRequest::SubscribeRequest(request) => self
                .subscribe(request.params, context)
                .await
                .map(ServerResult::empty),
            ClientRequest::UnsubscribeRequest(request) => self
                .unsubscribe(request.params, context)
                .await
                .map(ServerResult::empty),
            ClientRequest::CallToolRequest(request) => self
                .call_tool(request.params, context)
                .await
                .map(ServerResult::CallToolResult),
            ClientRequest::ListToolsRequest(request) => self
                .list_tools(request.params, context)
                .await
                .map(ServerResult::ListToolsResult),
        }
    }

    async fn handle_notification(
        &self,
        notification: <RoleServer as ServiceRole>::PeerNot,
        context: NotificationContext<RoleServer>,
    ) -> Result<(), McpError> {
        match notification {
            ClientNotification::CancelledNotification(notification) => {
                self.on_cancelled(notification.params, context).await
            }
            ClientNotification::ProgressNotification(notification) => {
                self.on_progress(notification.params, context).await
            }
            ClientNotification::InitializedNotification(_notification) => {
                self.on_initialized(context).await
            }
            ClientNotification::RootsListChangedNotification(_notification) => {
                self.on_roots_list_changed(context).await
            }
        };
        Ok(())
    }

    fn get_info(&self) -> <RoleServer as ServiceRole>::Info {
        self.get_info()
    }
}

#[allow(unused_variables)]
pub trait ServerHandler: Sized + Send + Sync + 'static {
    fn ping(
        &self,
        context: RequestContext<RoleServer>,
    ) -> impl Future<Output = Result<(), McpError>> + Send + '_ {
        std::future::ready(Ok(()))
    }
    // handle requests
    fn initialize(
        &self,
        request: InitializeRequestParam,
        context: RequestContext<RoleServer>,
    ) -> impl Future<Output = Result<InitializeResult, McpError>> + Send + '_ {
        if context.peer.peer_info().is_none() {
            context.peer.set_peer_info(request);
        }
        std::future::ready(Ok(self.get_info()))
    }
    fn complete(
        &self,
        request: CompleteRequestParam,
        context: RequestContext<RoleServer>,
    ) -> impl Future<Output = Result<CompleteResult, McpError>> + Send + '_ {
        std::future::ready(Err(McpError::method_not_found::<CompleteRequestMethod>()))
    }
    fn set_level(
        &self,
        request: SetLevelRequestParam,
        context: RequestContext<RoleServer>,
    ) -> impl Future<Output = Result<(), McpError>> + Send + '_ {
        std::future::ready(Err(McpError::method_not_found::<SetLevelRequestMethod>()))
    }
    fn get_prompt(
        &self,
        request: GetPromptRequestParam,
        context: RequestContext<RoleServer>,
    ) -> impl Future<Output = Result<GetPromptResult, McpError>> + Send + '_ {
        std::future::ready(Err(McpError::method_not_found::<GetPromptRequestMethod>()))
    }
    fn list_prompts(
        &self,
        request: Option<PaginatedRequestParam>,
        context: RequestContext<RoleServer>,
    ) -> impl Future<Output = Result<ListPromptsResult, McpError>> + Send + '_ {
        std::future::ready(Ok(ListPromptsResult::default()))
    }
    fn list_resources(
        &self,
        request: Option<PaginatedRequestParam>,
        context: RequestContext<RoleServer>,
    ) -> impl Future<Output = Result<ListResourcesResult, McpError>> + Send + '_ {
        std::future::ready(Ok(ListResourcesResult::default()))
    }
    fn list_resource_templates(
        &self,
        request: Option<PaginatedRequestParam>,
        context: RequestContext<RoleServer>,
    ) -> impl Future<Output = Result<ListResourceTemplatesResult, McpError>> + Send + '_ {
        std::future::ready(Ok(ListResourceTemplatesResult::default()))
    }
    fn read_resource(
        &self,
        request: ReadResourceRequestParam,
        context: RequestContext<RoleServer>,
    ) -> impl Future<Output = Result<ReadResourceResult, McpError>> + Send + '_ {
        std::future::ready(Err(
            McpError::method_not_found::<ReadResourceRequestMethod>(),
        ))
    }
    fn subscribe(
        &self,
        request: SubscribeRequestParam,
        context: RequestContext<RoleServer>,
    ) -> impl Future<Output = Result<(), McpError>> + Send + '_ {
        std::future::ready(Err(McpError::method_not_found::<SubscribeRequestMethod>()))
    }
    fn unsubscribe(
        &self,
        request: UnsubscribeRequestParam,
        context: RequestContext<RoleServer>,
    ) -> impl Future<Output = Result<(), McpError>> + Send + '_ {
        std::future::ready(Err(McpError::method_not_found::<UnsubscribeRequestMethod>()))
    }
    fn call_tool(
        &self,
        request: CallToolRequestParam,
        context: RequestContext<RoleServer>,
    ) -> impl Future<Output = Result<CallToolResult, McpError>> + Send + '_ {
        std::future::ready(Err(McpError::method_not_found::<CallToolRequestMethod>()))
    }
    fn list_tools(
        &self,
        request: Option<PaginatedRequestParam>,
        context: RequestContext<RoleServer>,
    ) -> impl Future<Output = Result<ListToolsResult, McpError>> + Send + '_ {
        std::future::ready(Ok(ListToolsResult::default()))
    }

    fn on_cancelled(
        &self,
        notification: CancelledNotificationParam,
        context: NotificationContext<RoleServer>,
    ) -> impl Future<Output = ()> + Send + '_ {
        std::future::ready(())
    }
    fn on_progress(
        &self,
        notification: ProgressNotificationParam,
        context: NotificationContext<RoleServer>,
    ) -> impl Future<Output = ()> + Send + '_ {
        std::future::ready(())
    }
    fn on_initialized(
        &self,
        context: NotificationContext<RoleServer>,
    ) -> impl Future<Output = ()> + Send + '_ {
        tracing::info!("client initialized");
        std::future::ready(())
    }
    fn on_roots_list_changed(
        &self,
        context: NotificationContext<RoleServer>,
    ) -> impl Future<Output = ()> + Send + '_ {
        std::future::ready(())
    }

    fn get_info(&self) -> ServerInfo {
        ServerInfo::default()
    }
}

---
File: modelcontextprotocol-rust-sdk-13e72ec/crates/rmcp/src/handler/server/tool.rs
---

use std::{
    any::TypeId, borrow::Cow, collections::HashMap, future::Ready, marker::PhantomData, sync::Arc,
};

use futures::future::BoxFuture;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use tokio_util::sync::CancellationToken;

use crate::{
    RoleServer,
    model::{CallToolRequestParam, CallToolResult, ConstString, IntoContents, JsonObject},
    service::RequestContext,
};
/// A shortcut for generating a JSON schema for a type.
pub fn schema_for_type<T: JsonSchema>() -> JsonObject {
    // explicitly to align json schema version to official specifications.
    // https://github.com/modelcontextprotocol/modelcontextprotocol/blob/main/schema/2025-03-26/schema.json
    let mut settings = schemars::r#gen::SchemaSettings::draft07();
    settings.option_nullable = true;
    settings.option_add_null_type = false;
    settings.visitors = Vec::default();
    let generator = settings.into_generator();
    let schema = generator.into_root_schema_for::<T>();
    let object = serde_json::to_value(schema).expect("failed to serialize schema");
    match object {
        serde_json::Value::Object(object) => object,
        _ => panic!("unexpected schema value"),
    }
}

/// Call [`schema_for_type`] with a cache
pub fn cached_schema_for_type<T: JsonSchema + std::any::Any>() -> Arc<JsonObject> {
    thread_local! {
        static CACHE_FOR_TYPE: std::sync::RwLock<HashMap<TypeId, Arc<JsonObject>>> = Default::default();
    };
    CACHE_FOR_TYPE.with(|cache| {
        if let Some(x) = cache
            .read()
            .expect("schema cache lock poisoned")
            .get(&TypeId::of::<T>())
        {
            x.clone()
        } else {
            let schema = schema_for_type::<T>();
            let schema = Arc::new(schema);
            cache
                .write()
                .expect("schema cache lock poisoned")
                .insert(TypeId::of::<T>(), schema.clone());
            schema
        }
    })
}

/// Deserialize a JSON object into a type
pub fn parse_json_object<T: DeserializeOwned>(input: JsonObject) -> Result<T, crate::Error> {
    serde_json::from_value(serde_json::Value::Object(input)).map_err(|e| {
        crate::Error::invalid_params(
            format!("failed to deserialize parameters: {error}", error = e),
            None,
        )
    })
}
pub struct ToolCallContext<'service, S> {
    request_context: RequestContext<RoleServer>,
    service: &'service S,
    name: Cow<'static, str>,
    arguments: Option<JsonObject>,
}

impl<'service, S> ToolCallContext<'service, S> {
    pub fn new(
        service: &'service S,
        CallToolRequestParam { name, arguments }: CallToolRequestParam,
        request_context: RequestContext<RoleServer>,
    ) -> Self {
        Self {
            request_context,
            service,
            name,
            arguments,
        }
    }
    pub fn name(&self) -> &str {
        &self.name
    }
    pub fn request_context(&self) -> &RequestContext<RoleServer> {
        &self.request_context
    }
}

pub trait FromToolCallContextPart<'a, S>: Sized {
    fn from_tool_call_context_part(
        context: ToolCallContext<'a, S>,
    ) -> Result<(Self, ToolCallContext<'a, S>), crate::Error>;
}

pub trait IntoCallToolResult {
    fn into_call_tool_result(self) -> Result<CallToolResult, crate::Error>;
}
impl IntoCallToolResult for () {
    fn into_call_tool_result(self) -> Result<CallToolResult, crate::Error> {
        Ok(CallToolResult::success(vec![]))
    }
}

impl<T: IntoContents> IntoCallToolResult for T {
    fn into_call_tool_result(self) -> Result<CallToolResult, crate::Error> {
        Ok(CallToolResult::success(self.into_contents()))
    }
}

impl<T: IntoContents, E: IntoContents> IntoCallToolResult for Result<T, E> {
    fn into_call_tool_result(self) -> Result<CallToolResult, crate::Error> {
        match self {
            Ok(value) => Ok(CallToolResult::success(value.into_contents())),
            Err(error) => Ok(CallToolResult::error(error.into_contents())),
        }
    }
}

pin_project_lite::pin_project! {
    #[project = IntoCallToolResultFutProj]
    pub enum IntoCallToolResultFut<F, R> {
        Pending {
            #[pin]
            fut: F,
            _marker: PhantomData<R>,
        },
        Ready {
            #[pin]
            result: Ready<Result<CallToolResult, crate::Error>>,
        }
    }
}

impl<F, R> Future for IntoCallToolResultFut<F, R>
where
    F: Future<Output = R>,
    R: IntoCallToolResult,
{
    type Output = Result<CallToolResult, crate::Error>;

    fn poll(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Self::Output> {
        match self.project() {
            IntoCallToolResultFutProj::Pending { fut, _marker } => {
                fut.poll(cx).map(IntoCallToolResult::into_call_tool_result)
            }
            IntoCallToolResultFutProj::Ready { result } => result.poll(cx),
        }
    }
}

impl IntoCallToolResult for Result<CallToolResult, crate::Error> {
    fn into_call_tool_result(self) -> Result<CallToolResult, crate::Error> {
        self
    }
}

pub trait CallToolHandler<'a, S, A> {
    type Fut: Future<Output = Result<CallToolResult, crate::Error>> + Send + 'a;
    fn call(self, context: ToolCallContext<'a, S>) -> Self::Fut;
}

pub type DynCallToolHandler<S> = dyn Fn(ToolCallContext<'_, S>) -> BoxFuture<'_, Result<CallToolResult, crate::Error>>
    + Send
    + Sync;
/// Parameter Extractor
pub struct Parameter<K: ConstString, V>(pub K, pub V);

/// Parameter Extractor
///
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Parameters<P>(pub P);

impl<P: JsonSchema> JsonSchema for Parameters<P> {
    fn schema_name() -> String {
        P::schema_name()
    }

    fn json_schema(generator: &mut schemars::r#gen::SchemaGenerator) -> schemars::schema::Schema {
        P::json_schema(generator)
    }
}

/// Callee Extractor
pub struct Callee<'a, S>(pub &'a S);

impl<'a, S> FromToolCallContextPart<'a, S> for CancellationToken {
    fn from_tool_call_context_part(
        context: ToolCallContext<'a, S>,
    ) -> Result<(Self, ToolCallContext<'a, S>), crate::Error> {
        Ok((context.request_context.ct.clone(), context))
    }
}

impl<'a, S> FromToolCallContextPart<'a, S> for Callee<'a, S> {
    fn from_tool_call_context_part(
        context: ToolCallContext<'a, S>,
    ) -> Result<(Self, ToolCallContext<'a, S>), crate::Error> {
        Ok((Callee(context.service), context))
    }
}

pub struct ToolName(pub Cow<'static, str>);

impl<'a, S> FromToolCallContextPart<'a, S> for ToolName {
    fn from_tool_call_context_part(
        context: ToolCallContext<'a, S>,
    ) -> Result<(Self, ToolCallContext<'a, S>), crate::Error> {
        Ok((Self(context.name.clone()), context))
    }
}

impl<'a, S> FromToolCallContextPart<'a, S> for &'a S {
    fn from_tool_call_context_part(
        context: ToolCallContext<'a, S>,
    ) -> Result<(Self, ToolCallContext<'a, S>), crate::Error> {
        Ok((context.service, context))
    }
}

impl<'a, S, K, V> FromToolCallContextPart<'a, S> for Parameter<K, V>
where
    K: ConstString,
    V: DeserializeOwned,
{
    fn from_tool_call_context_part(
        context: ToolCallContext<'a, S>,
    ) -> Result<(Self, ToolCallContext<'a, S>), crate::Error> {
        let arguments = context
            .arguments
            .as_ref()
            .ok_or(crate::Error::invalid_params(
                format!("missing parameter {field}", field = K::VALUE),
                None,
            ))?;
        let value = arguments.get(K::VALUE).ok_or(crate::Error::invalid_params(
            format!("missing parameter {field}", field = K::VALUE),
            None,
        ))?;
        let value: V = serde_json::from_value(value.clone()).map_err(|e| {
            crate::Error::invalid_params(
                format!(
                    "failed to deserialize parameter {field}: {error}",
                    field = K::VALUE,
                    error = e
                ),
                None,
            )
        })?;
        Ok((Parameter(K::default(), value), context))
    }
}

impl<'a, S, P> FromToolCallContextPart<'a, S> for Parameters<P>
where
    P: DeserializeOwned,
{
    fn from_tool_call_context_part(
        mut context: ToolCallContext<'a, S>,
    ) -> Result<(Self, ToolCallContext<'a, S>), crate::Error> {
        let arguments = context.arguments.take().unwrap_or_default();
        let value: P =
            serde_json::from_value(serde_json::Value::Object(arguments)).map_err(|e| {
                crate::Error::invalid_params(
                    format!("failed to deserialize parameters: {error}", error = e),
                    None,
                )
            })?;
        Ok((Parameters(value), context))
    }
}

impl<'a, S> FromToolCallContextPart<'a, S> for JsonObject {
    fn from_tool_call_context_part(
        mut context: ToolCallContext<'a, S>,
    ) -> Result<(Self, ToolCallContext<'a, S>), crate::Error> {
        let object = context.arguments.take().unwrap_or_default();
        Ok((object, context))
    }
}

impl<'a, S> FromToolCallContextPart<'a, S> for crate::model::Extensions {
    fn from_tool_call_context_part(
        context: ToolCallContext<'a, S>,
    ) -> Result<(Self, ToolCallContext<'a, S>), crate::Error> {
        let extensions = context.request_context.extensions.clone();
        Ok((extensions, context))
    }
}

pub struct Extension<T>(pub T);

impl<'a, S, T> FromToolCallContextPart<'a, S> for Extension<T>
where
    T: Send + Sync + 'static + Clone,
{
    fn from_tool_call_context_part(
        context: ToolCallContext<'a, S>,
    ) -> Result<(Self, ToolCallContext<'a, S>), crate::Error> {
        let extension = context
            .request_context
            .extensions
            .get::<T>()
            .cloned()
            .ok_or_else(|| {
                crate::Error::invalid_params(
                    format!("missing extension {}", std::any::type_name::<T>()),
                    None,
                )
            })?;
        Ok((Extension(extension), context))
    }
}

impl<'a, S> FromToolCallContextPart<'a, S> for crate::Peer<RoleServer> {
    fn from_tool_call_context_part(
        context: ToolCallContext<'a, S>,
    ) -> Result<(Self, ToolCallContext<'a, S>), crate::Error> {
        let peer = context.request_context.peer.clone();
        Ok((peer, context))
    }
}

impl<'a, S> FromToolCallContextPart<'a, S> for crate::model::Meta {
    fn from_tool_call_context_part(
        mut context: ToolCallContext<'a, S>,
    ) -> Result<(Self, ToolCallContext<'a, S>), crate::Error> {
        let mut meta = crate::model::Meta::default();
        std::mem::swap(&mut meta, &mut context.request_context.meta);
        Ok((meta, context))
    }
}

pub struct RequestId(pub crate::model::RequestId);
impl<'a, S> FromToolCallContextPart<'a, S> for RequestId {
    fn from_tool_call_context_part(
        context: ToolCallContext<'a, S>,
    ) -> Result<(Self, ToolCallContext<'a, S>), crate::Error> {
        Ok((RequestId(context.request_context.id.clone()), context))
    }
}

impl<'a, S> FromToolCallContextPart<'a, S> for RequestContext<RoleServer> {
    fn from_tool_call_context_part(
        context: ToolCallContext<'a, S>,
    ) -> Result<(Self, ToolCallContext<'a, S>), crate::Error> {
        Ok((context.request_context.clone(), context))
    }
}

impl<'s, S> ToolCallContext<'s, S> {
    pub fn invoke<H, A>(self, h: H) -> H::Fut
    where
        H: CallToolHandler<'s, S, A>,
    {
        h.call(self)
    }
}

#[allow(clippy::type_complexity)]
pub struct AsyncAdapter<P, Fut, R>(PhantomData<(fn(P) -> Fut, fn(Fut) -> R)>);
pub struct SyncAdapter<P, R>(PhantomData<fn(P) -> R>);

macro_rules! impl_for {
    ($($T: ident)*) => {
        impl_for!([] [$($T)*]);
    };
    // finished
    ([$($Tn: ident)*] []) => {
        impl_for!(@impl $($Tn)*);
    };
    ([$($Tn: ident)*] [$Tn_1: ident $($Rest: ident)*]) => {
        impl_for!(@impl $($Tn)*);
        impl_for!([$($Tn)* $Tn_1] [$($Rest)*]);
    };
    (@impl $($Tn: ident)*) => {
        impl<'s, $($Tn,)* S, F, Fut, R> CallToolHandler<'s, S, AsyncAdapter<($($Tn,)*), Fut, R>> for F
        where
            $(
                $Tn: FromToolCallContextPart<'s, S> + 's,
            )*
            F: FnOnce($($Tn,)*) -> Fut + Send + 's,
            Fut: Future<Output = R> + Send + 's,
            R: IntoCallToolResult + Send + 's,
            S: Send + Sync,
        {
            type Fut = IntoCallToolResultFut<Fut, R>;
            #[allow(unused_variables, non_snake_case)]
            fn call(
                self,
                context: ToolCallContext<'s, S>,
            ) -> Self::Fut {
                $(
                    let result = $Tn::from_tool_call_context_part(context);
                    let ($Tn, context) = match result {
                        Ok((value, context)) => (value, context),
                        Err(e) => return IntoCallToolResultFut::Ready {
                            result: std::future::ready(Err(e)),
                        },
                    };
                )*
                IntoCallToolResultFut::Pending {
                    fut: self($($Tn,)*),
                    _marker: PhantomData
                }
            }
        }

        impl<'s, $($Tn,)* S, F, R> CallToolHandler<'s, S, SyncAdapter<($($Tn,)*), R>> for F
        where
            $(
                $Tn: FromToolCallContextPart<'s, S> + 's,
            )*
            F: FnOnce($($Tn,)*) -> R + Send + 's,
            R: IntoCallToolResult + Send + 's,
            S: Send + Sync,
        {
            type Fut = Ready<Result<CallToolResult, crate::Error>>;
            #[allow(unused_variables, non_snake_case)]
            fn call(
                self,
                context: ToolCallContext<'s, S>,
            ) -> Self::Fut {
                $(
                    let result = $Tn::from_tool_call_context_part(context);
                    let ($Tn, context) = match result {
                        Ok((value, context)) => (value, context),
                        Err(e) => return std::future::ready(Err(e)),
                    };
                )*
                std::future::ready(self($($Tn,)*).into_call_tool_result())
            }
        }
    };
}
impl_for!(T0 T1 T2 T3 T4 T5 T6 T7 T8 T9 T10 T11 T12 T13 T14 T15);
pub struct ToolBoxItem<S> {
    #[allow(clippy::type_complexity)]
    pub call: Box<DynCallToolHandler<S>>,
    pub attr: crate::model::Tool,
}

impl<S: Send + Sync + 'static + Clone> ToolBoxItem<S> {
    pub fn new<C>(attr: crate::model::Tool, call: C) -> Self
    where
        C: Fn(ToolCallContext<'_, S>) -> BoxFuture<'_, Result<CallToolResult, crate::Error>>
            + Send
            + Sync
            + 'static,
    {
        Self {
            call: Box::new(call),
            attr,
        }
    }
    pub fn name(&self) -> &str {
        &self.attr.name
    }
}

#[derive(Default)]
pub struct ToolBox<S> {
    #[allow(clippy::type_complexity)]
    pub map: std::collections::HashMap<Cow<'static, str>, ToolBoxItem<S>>,
}

impl<S> ToolBox<S> {
    pub fn new() -> Self {
        Self {
            map: std::collections::HashMap::new(),
        }
    }
    pub fn add(&mut self, item: ToolBoxItem<S>) {
        self.map.insert(item.attr.name.clone(), item);
    }

    pub fn remove<H, A>(&mut self, name: &str) {
        self.map.remove(name);
    }

    pub async fn call(
        &self,
        context: ToolCallContext<'_, S>,
    ) -> Result<CallToolResult, crate::Error> {
        let item = self
            .map
            .get(context.name())
            .ok_or_else(|| crate::Error::invalid_params("tool not found", None))?;
        (item.call)(context).await
    }

    pub fn list(&self) -> Vec<crate::model::Tool> {
        self.map.values().map(|item| item.attr.clone()).collect()
    }
}

#[cfg(feature = "macros")]
#[cfg_attr(docsrs, doc(cfg(feature = "macros")))]
#[macro_export]
macro_rules! tool_box {
    (@pin_add $callee: ident, $attr: expr, $f: expr) => {
        $callee.add(ToolBoxItem::new($attr, |context| Box::pin($f(context))));
    };
    ($server: ident { $($tool: ident),* $(,)?} ) => {
        $crate::tool_box!($server { $($tool),* }  tool_box);
    };
    ($server: ident { $($tool: ident),* $(,)?} $tool_box: ident) => {
        fn $tool_box() -> &'static $crate::handler::server::tool::ToolBox<$server> {
            use $crate::handler::server::tool::{ToolBox, ToolBoxItem};
            static TOOL_BOX: std::sync::OnceLock<ToolBox<$server>> = std::sync::OnceLock::new();
            TOOL_BOX.get_or_init(|| {
                let mut tool_box = ToolBox::new();
                $crate::paste!{
                    $(
                        $crate::tool_box!(@pin_add tool_box, $server::[< $tool _tool_attr>](), $server::[<$tool _tool_call>]);
                    )*
                }
                tool_box
            })
        }
    };
    (@derive) => {
        $crate::tool_box!(@derive tool_box);
    };

    (@derive $tool_box:ident) => {
        async fn list_tools(
            &self,
            _: Option<$crate::model::PaginatedRequestParam>,
            _: $crate::service::RequestContext<$crate::service::RoleServer>,
        ) -> Result<$crate::model::ListToolsResult, $crate::Error> {
            Ok($crate::model::ListToolsResult {
                next_cursor: None,
                tools: Self::tool_box().list(),
            })
        }

        async fn call_tool(
            &self,
            call_tool_request_param: $crate::model::CallToolRequestParam,
            context: $crate::service::RequestContext<$crate::service::RoleServer>,
        ) -> Result<$crate::model::CallToolResult, $crate::Error> {
            let context = $crate::handler::server::tool::ToolCallContext::new(self, call_tool_request_param, context);
            Self::$tool_box().call(context).await
        }
    }
}

---
File: modelcontextprotocol-rust-sdk-13e72ec/crates/rmcp/src/handler/server/wrapper.rs
---

mod json;
pub use json::*;

---
File: modelcontextprotocol-rust-sdk-13e72ec/crates/rmcp/src/handler/server/wrapper/json.rs
---

use serde::Serialize;

use crate::model::IntoContents;

/// Json wrapper
///
/// This is used to tell the SDK to serialize the inner value into json
pub struct Json<T>(pub T);

impl<T> IntoContents for Json<T>
where
    T: Serialize,
{
    fn into_contents(self) -> Vec<crate::model::Content> {
        let result = crate::model::Content::json(self.0);
        debug_assert!(
            result.is_ok(),
            "Json wrapped content should be able to serialized into json"
        );
        match result {
            Ok(content) => vec![content],
            Err(e) => {
                tracing::error!("failed to convert json content: {e}");
                vec![]
            }
        }
    }
}

---
File: modelcontextprotocol-rust-sdk-13e72ec/crates/rmcp/src/lib.rs
---

#![cfg_attr(docsrs, feature(doc_cfg))]
#![cfg_attr(docsrs, allow(unused_attributes))]
//! The official Rust SDK for the Model Context Protocol (MCP).
//!
//! The MCP is a protocol that allows AI assistants to communicate with other
//! services. `rmcp` is the official Rust implementation of this protocol.
//!
//! There are two ways in which the library can be used, namely to build a
//! server or to build a client.
//!
//! ## Server
//!
//! A server is a service that exposes capabilities. For example, a common
//! use-case is for the server to make multiple tools available to clients such
//! as Claude Desktop or the Cursor IDE.
//!
//! For example, to implement a server that has a tool that can count, you would
//! make an object for that tool and add an implementation with the `#[tool(tool_box)]` macro:
//!
//! ```rust
//! use std::sync::Arc;
//! use rmcp::{Error as McpError, model::*, tool};
//! use tokio::sync::Mutex;
//!
//! #[derive(Clone)]
//! pub struct Counter {
//!     counter: Arc<Mutex<i32>>,
//! }
//!
//! #[tool(tool_box)]
//! impl Counter {
//!     fn new() -> Self {
//!         Self {
//!             counter: Arc::new(Mutex::new(0)),
//!         }
//!     }
//!
//!     #[tool(description = "Increment the counter by 1")]
//!     async fn increment(&self) -> Result<CallToolResult, McpError> {
//!         let mut counter = self.counter.lock().await;
//!         *counter += 1;
//!         Ok(CallToolResult::success(vec![Content::text(
//!             counter.to_string(),
//!         )]))
//!     }
//! }
//! ```
//!
//! Next also implement [ServerHandler] for `Counter` and start the server inside
//! `main` by calling `Counter::new().serve(...)`. See the examples directory in the repository for more information.
//!
//! ## Client
//!
//! A client can be used to interact with a server. Clients can be used to get a
//! list of the available tools and to call them. For example, we can `uv` to
//! start a MCP server in Python and then list the tools and call `git status`
//! as follows:
//!
//! ```rust
//! use anyhow::Result;
//! use rmcp::{model::CallToolRequestParam, service::ServiceExt, transport::{TokioChildProcess, ConfigureCommandExt}};
//! use tokio::process::Command;
//!
//! async fn client() -> Result<()> {
//!     let service = ().serve(TokioChildProcess::new(Command::new("uvx").configure(|cmd| {
//!         cmd.arg("mcp-server-git");
//!     }))?).await?;
//!
//!     // Initialize
//!     let server_info = service.peer_info();
//!     println!("Connected to server: {server_info:#?}");
//!
//!     // List tools
//!     let tools = service.list_tools(Default::default()).await?;
//!     println!("Available tools: {tools:#?}");
//!
//!     // Call tool 'git_status' with arguments = {"repo_path": "."}
//!     let tool_result = service
//!         .call_tool(CallToolRequestParam {
//!             name: "git_status".into(),
//!             arguments: serde_json::json!({ "repo_path": "." }).as_object().cloned(),
//!         })
//!         .await?;
//!     println!("Tool result: {tool_result:#?}");
//!
//!     service.cancel().await?;
//!     Ok(())
//! }
//! ```
mod error;
pub use error::Error;

/// Basic data types in MCP specification
pub mod model;
#[cfg(any(feature = "client", feature = "server"))]
#[cfg_attr(docsrs, doc(cfg(any(feature = "client", feature = "server"))))]
pub mod service;
#[cfg(feature = "client")]
#[cfg_attr(docsrs, doc(cfg(feature = "client")))]
pub use handler::client::ClientHandler;
#[cfg(feature = "server")]
#[cfg_attr(docsrs, doc(cfg(feature = "server")))]
pub use handler::server::ServerHandler;
#[cfg(any(feature = "client", feature = "server"))]
#[cfg_attr(docsrs, doc(cfg(any(feature = "client", feature = "server"))))]
pub use service::{Peer, Service, ServiceError, ServiceExt};
#[cfg(feature = "client")]
#[cfg_attr(docsrs, doc(cfg(feature = "client")))]
pub use service::{RoleClient, serve_client};
#[cfg(feature = "server")]
#[cfg_attr(docsrs, doc(cfg(feature = "server")))]
pub use service::{RoleServer, serve_server};

pub mod handler;
pub mod transport;

// re-export
#[cfg(all(feature = "macros", feature = "server"))]
#[cfg_attr(docsrs, doc(cfg(all(feature = "macros", feature = "server"))))]
pub use paste::paste;
#[cfg(all(feature = "macros", feature = "server"))]
#[cfg_attr(docsrs, doc(cfg(all(feature = "macros", feature = "server"))))]
pub use rmcp_macros::tool;
#[cfg(all(feature = "macros", feature = "server"))]
#[cfg_attr(docsrs, doc(cfg(all(feature = "macros", feature = "server"))))]
pub use schemars;
#[cfg(feature = "macros")]
#[cfg_attr(docsrs, doc(cfg(feature = "macros")))]
pub use serde;
#[cfg(feature = "macros")]
#[cfg_attr(docsrs, doc(cfg(feature = "macros")))]
pub use serde_json;

---
File: modelcontextprotocol-rust-sdk-13e72ec/crates/rmcp/src/model.rs
---

use std::{borrow::Cow, sync::Arc};
mod annotated;
mod capabilities;
mod content;
mod extension;
mod meta;
mod prompt;
mod resource;
mod serde_impl;
mod tool;
pub use annotated::*;
pub use capabilities::*;
pub use content::*;
pub use extension::*;
pub use meta::*;
pub use prompt::*;
pub use resource::*;
use serde::{Deserialize, Serialize};
use serde_json::Value;
pub use tool::*;

/// You can use [`crate::object!`] or [`crate::model::object`] to create a json object quickly.
pub type JsonObject<F = Value> = serde_json::Map<String, F>;

/// unwrap the JsonObject under [`serde_json::Value`]
///
/// # Panic
/// This will panic when the value is not a object in debug mode.
pub fn object(value: serde_json::Value) -> JsonObject {
    debug_assert!(value.is_object());
    match value {
        serde_json::Value::Object(map) => map,
        _ => JsonObject::default(),
    }
}

/// Use this macro just like [`serde_json::json!`]
#[cfg(feature = "macros")]
#[cfg_attr(docsrs, doc(cfg(feature = "macros")))]
#[macro_export]
macro_rules! object {
    ({$($tt:tt)*}) => {
        $crate::model::object(serde_json::json! {
            {$($tt)*}
        })
    };
}
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Copy, Eq)]
#[cfg_attr(feature = "server", derive(schemars::JsonSchema))]
pub struct EmptyObject {}

pub trait ConstString: Default {
    const VALUE: &str;
}
#[macro_export]
macro_rules! const_string {
    ($name:ident = $value:literal) => {
        #[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
        pub struct $name;

        impl ConstString for $name {
            const VALUE: &str = $value;
        }

        impl serde::Serialize for $name {
            fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
            where
                S: serde::Serializer,
            {
                $value.serialize(serializer)
            }
        }

        impl<'de> serde::Deserialize<'de> for $name {
            fn deserialize<D>(deserializer: D) -> Result<$name, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                let s: String = serde::Deserialize::deserialize(deserializer)?;
                if s == $value {
                    Ok($name)
                } else {
                    Err(serde::de::Error::custom(format!(concat!(
                        "expect const string value \"",
                        $value,
                        "\""
                    ))))
                }
            }
        }

        #[cfg(feature = "schemars")]
        impl schemars::JsonSchema for $name {
            fn schema_name() -> String {
                stringify!($name).to_string()
            }

            fn json_schema(_: &mut schemars::SchemaGenerator) -> schemars::schema::Schema {
                // Create a schema for a constant value of type String
                schemars::schema::Schema::Object(schemars::schema::SchemaObject {
                    instance_type: Some(schemars::schema::InstanceType::String.into()),
                    format: Some("const".to_string()),
                    const_value: Some(serde_json::Value::String($value.into())),
                    ..Default::default()
                })
            }
        }
    };
}

const_string!(JsonRpcVersion2_0 = "2.0");

#[derive(Debug, Clone, Eq, PartialEq, Hash, PartialOrd)]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
pub struct ProtocolVersion(Cow<'static, str>);

impl Default for ProtocolVersion {
    fn default() -> Self {
        Self::LATEST
    }
}

impl std::fmt::Display for ProtocolVersion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

impl ProtocolVersion {
    pub const V_2025_03_26: Self = Self(Cow::Borrowed("2025-03-26"));
    pub const V_2024_11_05: Self = Self(Cow::Borrowed("2024-11-05"));
    pub const LATEST: Self = Self::V_2025_03_26;
}

impl Serialize for ProtocolVersion {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        self.0.serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for ProtocolVersion {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s: String = Deserialize::deserialize(deserializer)?;
        #[allow(clippy::single_match)]
        match s.as_str() {
            "2024-11-05" => return Ok(ProtocolVersion::V_2024_11_05),
            "2025-03-26" => return Ok(ProtocolVersion::V_2025_03_26),
            _ => {}
        }
        Ok(ProtocolVersion(Cow::Owned(s)))
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub enum NumberOrString {
    Number(u32),
    String(Arc<str>),
}

impl NumberOrString {
    pub fn into_json_value(self) -> Value {
        match self {
            NumberOrString::Number(n) => Value::Number(serde_json::Number::from(n)),
            NumberOrString::String(s) => Value::String(s.to_string()),
        }
    }
}

impl std::fmt::Display for NumberOrString {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            NumberOrString::Number(n) => n.fmt(f),
            NumberOrString::String(s) => s.fmt(f),
        }
    }
}

impl Serialize for NumberOrString {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        match self {
            NumberOrString::Number(n) => n.serialize(serializer),
            NumberOrString::String(s) => s.serialize(serializer),
        }
    }
}

impl<'de> Deserialize<'de> for NumberOrString {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let value: Value = Deserialize::deserialize(deserializer)?;
        match value {
            Value::Number(n) => Ok(NumberOrString::Number(
                n.as_u64()
                    .ok_or(serde::de::Error::custom("Expect an integer"))? as u32,
            )),
            Value::String(s) => Ok(NumberOrString::String(s.into())),
            _ => Err(serde::de::Error::custom("Expect number or string")),
        }
    }
}

#[cfg(feature = "schemars")]
impl schemars::JsonSchema for NumberOrString {
    fn schema_name() -> String {
        "NumberOrString".to_string()
    }

    fn json_schema(_: &mut schemars::SchemaGenerator) -> schemars::schema::Schema {
        schemars::schema::Schema::Object(schemars::schema::SchemaObject {
            subschemas: Some(Box::new(schemars::schema::SubschemaValidation {
                one_of: Some(vec![
                    schemars::schema::Schema::Object(schemars::schema::SchemaObject {
                        instance_type: Some(schemars::schema::InstanceType::Number.into()),
                        ..Default::default()
                    }),
                    schemars::schema::Schema::Object(schemars::schema::SchemaObject {
                        instance_type: Some(schemars::schema::InstanceType::String.into()),
                        ..Default::default()
                    }),
                ]),
                ..Default::default()
            })),
            ..Default::default()
        })
    }
}

pub type RequestId = NumberOrString;

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Hash, Eq)]
#[serde(transparent)]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
pub struct ProgressToken(pub NumberOrString);
#[derive(Debug, Clone)]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
pub struct Request<M = String, P = JsonObject> {
    pub method: M,
    // #[serde(skip_serializing_if = "Option::is_none")]
    pub params: P,
    /// extensions will carry anything possible in the context, including [`Meta`]
    ///
    /// this is similar with the Extensions in `http` crate
    #[cfg_attr(feature = "schemars", schemars(skip))]
    pub extensions: Extensions,
}

impl<M, P> GetExtensions for Request<M, P> {
    fn extensions(&self) -> &Extensions {
        &self.extensions
    }
    fn extensions_mut(&mut self) -> &mut Extensions {
        &mut self.extensions
    }
}

#[derive(Debug, Clone)]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
pub struct RequestOptionalParam<M = String, P = JsonObject> {
    pub method: M,
    // #[serde(skip_serializing_if = "Option::is_none")]
    pub params: Option<P>,
    /// extensions will carry anything possible in the context, including [`Meta`]
    ///
    /// this is similar with the Extensions in `http` crate
    #[cfg_attr(feature = "schemars", schemars(skip))]
    pub extensions: Extensions,
}

#[derive(Debug, Clone)]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
pub struct RequestNoParam<M = String> {
    pub method: M,
    /// extensions will carry anything possible in the context, including [`Meta`]
    ///
    /// this is similar with the Extensions in `http` crate
    #[cfg_attr(feature = "schemars", schemars(skip))]
    pub extensions: Extensions,
}

impl<M> GetExtensions for RequestNoParam<M> {
    fn extensions(&self) -> &Extensions {
        &self.extensions
    }
    fn extensions_mut(&mut self) -> &mut Extensions {
        &mut self.extensions
    }
}
#[derive(Debug, Clone)]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
pub struct Notification<M = String, P = JsonObject> {
    pub method: M,
    pub params: P,
    /// extensions will carry anything possible in the context, including [`Meta`]
    ///
    /// this is similar with the Extensions in `http` crate
    #[cfg_attr(feature = "schemars", schemars(skip))]
    pub extensions: Extensions,
}

#[derive(Debug, Clone)]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
pub struct NotificationNoParam<M = String> {
    pub method: M,
    /// extensions will carry anything possible in the context, including [`Meta`]
    ///
    /// this is similar with the Extensions in `http` crate
    #[cfg_attr(feature = "schemars", schemars(skip))]
    pub extensions: Extensions,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
pub struct JsonRpcRequest<R = Request> {
    pub jsonrpc: JsonRpcVersion2_0,
    pub id: RequestId,
    #[serde(flatten)]
    pub request: R,
}

type DefaultResponse = JsonObject;
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
pub struct JsonRpcResponse<R = JsonObject> {
    pub jsonrpc: JsonRpcVersion2_0,
    pub id: RequestId,
    pub result: R,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
pub struct JsonRpcError {
    pub jsonrpc: JsonRpcVersion2_0,
    pub id: RequestId,
    pub error: ErrorData,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
pub struct JsonRpcNotification<N = Notification> {
    pub jsonrpc: JsonRpcVersion2_0,
    #[serde(flatten)]
    pub notification: N,
}

// Standard JSON-RPC error codes
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(transparent)]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
pub struct ErrorCode(pub i32);

impl ErrorCode {
    pub const RESOURCE_NOT_FOUND: Self = Self(-32002);
    pub const INVALID_REQUEST: Self = Self(-32600);
    pub const METHOD_NOT_FOUND: Self = Self(-32601);
    pub const INVALID_PARAMS: Self = Self(-32602);
    pub const INTERNAL_ERROR: Self = Self(-32603);
    pub const PARSE_ERROR: Self = Self(-32700);
}

/// Error information for JSON-RPC error responses.
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
pub struct ErrorData {
    /// The error type that occurred.
    pub code: ErrorCode,

    /// A short description of the error. The message SHOULD be limited to a concise single sentence.
    pub message: Cow<'static, str>,

    /// Additional information about the error. The value of this member is defined by the
    /// sender (e.g. detailed error information, nested errors etc.).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<Value>,
}

impl ErrorData {
    pub fn new(
        code: ErrorCode,
        message: impl Into<Cow<'static, str>>,
        data: Option<Value>,
    ) -> Self {
        Self {
            code,
            message: message.into(),
            data,
        }
    }
    pub fn resource_not_found(message: impl Into<Cow<'static, str>>, data: Option<Value>) -> Self {
        Self::new(ErrorCode::RESOURCE_NOT_FOUND, message, data)
    }
    pub fn parse_error(message: impl Into<Cow<'static, str>>, data: Option<Value>) -> Self {
        Self::new(ErrorCode::PARSE_ERROR, message, data)
    }
    pub fn invalid_request(message: impl Into<Cow<'static, str>>, data: Option<Value>) -> Self {
        Self::new(ErrorCode::INVALID_REQUEST, message, data)
    }
    pub fn method_not_found<M: ConstString>() -> Self {
        Self::new(ErrorCode::METHOD_NOT_FOUND, M::VALUE, None)
    }
    pub fn invalid_params(message: impl Into<Cow<'static, str>>, data: Option<Value>) -> Self {
        Self::new(ErrorCode::INVALID_PARAMS, message, data)
    }
    pub fn internal_error(message: impl Into<Cow<'static, str>>, data: Option<Value>) -> Self {
        Self::new(ErrorCode::INTERNAL_ERROR, message, data)
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
#[serde(untagged)]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
pub enum JsonRpcBatchRequestItem<Req, Not> {
    Request(JsonRpcRequest<Req>),
    Notification(JsonRpcNotification<Not>),
}

impl<Req, Not> JsonRpcBatchRequestItem<Req, Not> {
    pub fn into_non_batch_message<Resp>(self) -> JsonRpcMessage<Req, Resp, Not> {
        match self {
            JsonRpcBatchRequestItem::Request(r) => JsonRpcMessage::Request(r),
            JsonRpcBatchRequestItem::Notification(n) => JsonRpcMessage::Notification(n),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
#[serde(untagged)]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
pub enum JsonRpcBatchResponseItem<Resp> {
    Response(JsonRpcResponse<Resp>),
    Error(JsonRpcError),
}

impl<Resp> JsonRpcBatchResponseItem<Resp> {
    pub fn into_non_batch_message<Req, Not>(self) -> JsonRpcMessage<Req, Resp, Not> {
        match self {
            JsonRpcBatchResponseItem::Response(r) => JsonRpcMessage::Response(r),
            JsonRpcBatchResponseItem::Error(e) => JsonRpcMessage::Error(e),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
#[serde(untagged)]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
pub enum JsonRpcMessage<Req = Request, Resp = DefaultResponse, Noti = Notification> {
    Request(JsonRpcRequest<Req>),
    Response(JsonRpcResponse<Resp>),
    Notification(JsonRpcNotification<Noti>),
    BatchRequest(Vec<JsonRpcBatchRequestItem<Req, Noti>>),
    BatchResponse(Vec<JsonRpcBatchResponseItem<Resp>>),
    Error(JsonRpcError),
}

impl<Req, Resp, Not> JsonRpcMessage<Req, Resp, Not> {
    #[inline]
    pub const fn request(request: Req, id: RequestId) -> Self {
        JsonRpcMessage::Request(JsonRpcRequest {
            jsonrpc: JsonRpcVersion2_0,
            id,
            request,
        })
    }
    #[inline]
    pub const fn response(response: Resp, id: RequestId) -> Self {
        JsonRpcMessage::Response(JsonRpcResponse {
            jsonrpc: JsonRpcVersion2_0,
            id,
            result: response,
        })
    }
    #[inline]
    pub const fn error(error: ErrorData, id: RequestId) -> Self {
        JsonRpcMessage::Error(JsonRpcError {
            jsonrpc: JsonRpcVersion2_0,
            id,
            error,
        })
    }
    #[inline]
    pub const fn notification(notification: Not) -> Self {
        JsonRpcMessage::Notification(JsonRpcNotification {
            jsonrpc: JsonRpcVersion2_0,
            notification,
        })
    }
    pub fn into_request(self) -> Option<(Req, RequestId)> {
        match self {
            JsonRpcMessage::Request(r) => Some((r.request, r.id)),
            _ => None,
        }
    }
    pub fn into_response(self) -> Option<(Resp, RequestId)> {
        match self {
            JsonRpcMessage::Response(r) => Some((r.result, r.id)),
            _ => None,
        }
    }
    pub fn into_notification(self) -> Option<Not> {
        match self {
            JsonRpcMessage::Notification(n) => Some(n.notification),
            _ => None,
        }
    }
    pub fn into_error(self) -> Option<(ErrorData, RequestId)> {
        match self {
            JsonRpcMessage::Error(e) => Some((e.error, e.id)),
            _ => None,
        }
    }
    pub fn into_result(self) -> Option<(Result<Resp, ErrorData>, RequestId)> {
        match self {
            JsonRpcMessage::Response(r) => Some((Ok(r.result), r.id)),
            JsonRpcMessage::Error(e) => Some((Err(e.error), e.id)),

            _ => None,
        }
    }
}

/// # Empty result
/// A response that indicates success but carries no data.
pub type EmptyResult = EmptyObject;

impl From<()> for EmptyResult {
    fn from(_value: ()) -> Self {
        EmptyResult {}
    }
}

impl From<EmptyResult> for () {
    fn from(_value: EmptyResult) {}
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
pub struct CancelledNotificationParam {
    pub request_id: RequestId,
    pub reason: Option<String>,
}

const_string!(CancelledNotificationMethod = "notifications/cancelled");

/// # Cancellation
/// This notification can be sent by either side to indicate that it is cancelling a previously-issued request.
///
/// The request SHOULD still be in-flight, but due to communication latency, it is always possible that this notification MAY arrive after the request has already finished.
///
/// This notification indicates that the result will be unused, so any associated processing SHOULD cease.
///
/// A client MUST NOT attempt to cancel its `initialize` request.
pub type CancelledNotification =
    Notification<CancelledNotificationMethod, CancelledNotificationParam>;

const_string!(InitializeResultMethod = "initialize");
/// # Initialization
/// This request is sent from the client to the server when it first connects, asking it to begin initialization.
pub type InitializeRequest = Request<InitializeResultMethod, InitializeRequestParam>;

const_string!(InitializedNotificationMethod = "notifications/initialized");
/// This notification is sent from the client to the server after initialization has finished.
pub type InitializedNotification = NotificationNoParam<InitializedNotificationMethod>;
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
pub struct InitializeRequestParam {
    pub protocol_version: ProtocolVersion,
    pub capabilities: ClientCapabilities,
    pub client_info: Implementation,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
pub struct InitializeResult {
    pub protocol_version: ProtocolVersion,
    pub capabilities: ServerCapabilities,
    pub server_info: Implementation,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub instructions: Option<String>,
}

pub type ServerInfo = InitializeResult;
pub type ClientInfo = InitializeRequestParam;

impl Default for ServerInfo {
    fn default() -> Self {
        ServerInfo {
            protocol_version: ProtocolVersion::default(),
            capabilities: ServerCapabilities::default(),
            server_info: Implementation::from_build_env(),
            instructions: None,
        }
    }
}

impl Default for ClientInfo {
    fn default() -> Self {
        ClientInfo {
            protocol_version: ProtocolVersion::default(),
            capabilities: ClientCapabilities::default(),
            client_info: Implementation::from_build_env(),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
pub struct Implementation {
    pub name: String,
    pub version: String,
}

impl Default for Implementation {
    fn default() -> Self {
        Self::from_build_env()
    }
}

impl Implementation {
    pub fn from_build_env() -> Self {
        Implementation {
            name: env!("CARGO_CRATE_NAME").to_owned(),
            version: env!("CARGO_PKG_VERSION").to_owned(),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Default)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
pub struct PaginatedRequestParam {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cursor: Option<String>,
}
const_string!(PingRequestMethod = "ping");
pub type PingRequest = RequestNoParam<PingRequestMethod>;

const_string!(ProgressNotificationMethod = "notifications/progress");
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
pub struct ProgressNotificationParam {
    pub progress_token: ProgressToken,
    /// The progress thus far. This should increase every time progress is made, even if the total is unknown.
    pub progress: u32,
    /// Total number of items to process (or total progress required), if known
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total: Option<u32>,
    /// An optional message describing the current progress.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}

pub type ProgressNotification = Notification<ProgressNotificationMethod, ProgressNotificationParam>;

pub type Cursor = String;

macro_rules! paginated_result {
    ($t:ident {
        $i_item: ident: $t_item: ty
    }) => {
        #[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Default)]
        #[serde(rename_all = "camelCase")]
        #[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
        pub struct $t {
            #[serde(skip_serializing_if = "Option::is_none")]
            pub next_cursor: Option<Cursor>,
            pub $i_item: $t_item,
        }
    };
}

const_string!(ListResourcesRequestMethod = "resources/list");
pub type ListResourcesRequest =
    RequestOptionalParam<ListResourcesRequestMethod, PaginatedRequestParam>;
paginated_result!(ListResourcesResult {
    resources: Vec<Resource>
});

const_string!(ListResourceTemplatesRequestMethod = "resources/templates/list");
pub type ListResourceTemplatesRequest =
    RequestOptionalParam<ListResourceTemplatesRequestMethod, PaginatedRequestParam>;
paginated_result!(ListResourceTemplatesResult {
    resource_templates: Vec<ResourceTemplate>
});

const_string!(ReadResourceRequestMethod = "resources/read");
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
pub struct ReadResourceRequestParam {
    pub uri: String,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
pub struct ReadResourceResult {
    pub contents: Vec<ResourceContents>,
}

pub type ReadResourceRequest = Request<ReadResourceRequestMethod, ReadResourceRequestParam>;

const_string!(ResourceListChangedNotificationMethod = "notifications/resources/list_changed");
pub type ResourceListChangedNotification =
    NotificationNoParam<ResourceListChangedNotificationMethod>;

const_string!(SubscribeRequestMethod = "resources/subscribe");
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
pub struct SubscribeRequestParam {
    pub uri: String,
}
pub type SubscribeRequest = Request<SubscribeRequestMethod, SubscribeRequestParam>;

const_string!(UnsubscribeRequestMethod = "resources/unsubscribe");
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
pub struct UnsubscribeRequestParam {
    pub uri: String,
}
pub type UnsubscribeRequest = Request<UnsubscribeRequestMethod, UnsubscribeRequestParam>;

const_string!(ResourceUpdatedNotificationMethod = "notifications/resources/updated");
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
pub struct ResourceUpdatedNotificationParam {
    pub uri: String,
}
pub type ResourceUpdatedNotification =
    Notification<ResourceUpdatedNotificationMethod, ResourceUpdatedNotificationParam>;

const_string!(ListPromptsRequestMethod = "prompts/list");
pub type ListPromptsRequest = RequestOptionalParam<ListPromptsRequestMethod, PaginatedRequestParam>;
paginated_result!(ListPromptsResult {
    prompts: Vec<Prompt>
});

const_string!(GetPromptRequestMethod = "prompts/get");
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
pub struct GetPromptRequestParam {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub arguments: Option<JsonObject>,
}
pub type GetPromptRequest = Request<GetPromptRequestMethod, GetPromptRequestParam>;

const_string!(PromptListChangedNotificationMethod = "notifications/prompts/list_changed");
pub type PromptListChangedNotification = NotificationNoParam<PromptListChangedNotificationMethod>;

const_string!(ToolListChangedNotificationMethod = "notifications/tools/list_changed");
pub type ToolListChangedNotification = NotificationNoParam<ToolListChangedNotificationMethod>;
// 日志相关
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Copy)]
#[serde(rename_all = "lowercase")] //match spec
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
pub enum LoggingLevel {
    Debug,
    Info,
    Notice,
    Warning,
    Error,
    Critical,
    Alert,
    Emergency,
}

const_string!(SetLevelRequestMethod = "logging/setLevel");
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
pub struct SetLevelRequestParam {
    pub level: LoggingLevel,
}
pub type SetLevelRequest = Request<SetLevelRequestMethod, SetLevelRequestParam>;

const_string!(LoggingMessageNotificationMethod = "notifications/message");
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
pub struct LoggingMessageNotificationParam {
    pub level: LoggingLevel,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub logger: Option<String>,
    pub data: Value,
}
pub type LoggingMessageNotification =
    Notification<LoggingMessageNotificationMethod, LoggingMessageNotificationParam>;

const_string!(CreateMessageRequestMethod = "sampling/createMessage");
pub type CreateMessageRequest = Request<CreateMessageRequestMethod, CreateMessageRequestParam>;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
pub enum Role {
    User,
    Assistant,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
pub struct SamplingMessage {
    pub role: Role,
    pub content: Content,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
pub enum ContextInclusion {
    #[serde(rename = "allServers")]
    AllServers,
    #[serde(rename = "none")]
    None,
    #[serde(rename = "thisServer")]
    ThisServer,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
pub struct CreateMessageRequestParam {
    pub messages: Vec<SamplingMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model_preferences: Option<ModelPreferences>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub system_prompt: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub include_context: Option<ContextInclusion>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,
    pub max_tokens: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stop_sequences: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<Value>,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
pub struct ModelPreferences {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hints: Option<Vec<ModelHint>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cost_priority: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub speed_priority: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub intelligence_priority: Option<f32>,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
pub struct ModelHint {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
pub struct CompleteRequestParam {
    pub r#ref: Reference,
    pub argument: ArgumentInfo,
}

pub type CompleteRequest = Request<CompleteRequestMethod, CompleteRequestParam>;

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
pub struct CompletionInfo {
    pub values: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub has_more: Option<bool>,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
pub struct CompleteResult {
    pub completion: CompletionInfo,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
#[serde(tag = "type")]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
pub enum Reference {
    #[serde(rename = "ref/resource")]
    Resource(ResourceReference),
    #[serde(rename = "ref/prompt")]
    Prompt(PromptReference),
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
pub struct ResourceReference {
    pub uri: String,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
pub struct PromptReference {
    pub name: String,
}

const_string!(CompleteRequestMethod = "completion/complete");
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
pub struct ArgumentInfo {
    pub name: String,
    pub value: String,
}

// 根目录相关
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
pub struct Root {
    pub uri: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
}

const_string!(ListRootsRequestMethod = "roots/list");
pub type ListRootsRequest = RequestNoParam<ListRootsRequestMethod>;

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Default)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
pub struct ListRootsResult {
    pub roots: Vec<Root>,
}

const_string!(RootsListChangedNotificationMethod = "notifications/roots/list_changed");
pub type RootsListChangedNotification = NotificationNoParam<RootsListChangedNotificationMethod>;

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
pub struct CallToolResult {
    pub content: Vec<Content>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_error: Option<bool>,
}

impl CallToolResult {
    pub fn success(content: Vec<Content>) -> Self {
        CallToolResult {
            content,
            is_error: Some(false),
        }
    }
    pub fn error(content: Vec<Content>) -> Self {
        CallToolResult {
            content,
            is_error: Some(true),
        }
    }
}

const_string!(ListToolsRequestMethod = "tools/list");
pub type ListToolsRequest = RequestOptionalParam<ListToolsRequestMethod, PaginatedRequestParam>;
paginated_result!(
    ListToolsResult {
        tools: Vec<Tool>
    }
);

const_string!(CallToolRequestMethod = "tools/call");
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
pub struct CallToolRequestParam {
    pub name: Cow<'static, str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub arguments: Option<JsonObject>,
}

pub type CallToolRequest = Request<CallToolRequestMethod, CallToolRequestParam>;

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
pub struct CreateMessageResult {
    pub model: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stop_reason: Option<String>,
    #[serde(flatten)]
    pub message: SamplingMessage,
}

impl CreateMessageResult {
    pub const STOP_REASON_END_TURN: &str = "endTurn";
    pub const STOP_REASON_END_SEQUENCE: &str = "stopSequence";
    pub const STOP_REASON_END_MAX_TOKEN: &str = "maxTokens";
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
pub struct GetPromptResult {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub messages: Vec<PromptMessage>,
}

macro_rules! ts_union {
    (
        export type $U: ident =
            $(|)?$($V: ident)|*;
    ) => {
        #[derive(Debug, Serialize, Deserialize, Clone)]
        #[serde(untagged)]
        #[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
        pub enum $U {
            $($V($V),)*
        }
    };
}

ts_union!(
    export type ClientRequest =
    | PingRequest
    | InitializeRequest
    | CompleteRequest
    | SetLevelRequest
    | GetPromptRequest
    | ListPromptsRequest
    | ListResourcesRequest
    | ListResourceTemplatesRequest
    | ReadResourceRequest
    | SubscribeRequest
    | UnsubscribeRequest
    | CallToolRequest
    | ListToolsRequest;
);

ts_union!(
    export type ClientNotification =
    | CancelledNotification
    | ProgressNotification
    | InitializedNotification
    | RootsListChangedNotification;
);

ts_union!(
    export type ClientResult = CreateMessageResult | ListRootsResult | EmptyResult;
);

impl ClientResult {
    pub fn empty(_: ()) -> ClientResult {
        ClientResult::EmptyResult(EmptyResult {})
    }
}

pub type ClientJsonRpcMessage = JsonRpcMessage<ClientRequest, ClientResult, ClientNotification>;

ts_union!(
    export type ServerRequest =
    | PingRequest
    | CreateMessageRequest
    | ListRootsRequest;
);

ts_union!(
    export type ServerNotification =
    | CancelledNotification
    | ProgressNotification
    | LoggingMessageNotification
    | ResourceUpdatedNotification
    | ResourceListChangedNotification
    | ToolListChangedNotification
    | PromptListChangedNotification;
);

ts_union!(
    export type ServerResult =
    | InitializeResult
    | CompleteResult
    | GetPromptResult
    | ListPromptsResult
    | ListResourcesResult
    | ListResourceTemplatesResult
    | ReadResourceResult
    | CallToolResult
    | ListToolsResult
    | EmptyResult
    ;
);

impl ServerResult {
    pub fn empty(_: ()) -> ServerResult {
        ServerResult::EmptyResult(EmptyResult {})
    }
}

pub type ServerJsonRpcMessage = JsonRpcMessage<ServerRequest, ServerResult, ServerNotification>;

impl TryInto<CancelledNotification> for ServerNotification {
    type Error = ServerNotification;
    fn try_into(self) -> Result<CancelledNotification, Self::Error> {
        if let ServerNotification::CancelledNotification(t) = self {
            Ok(t)
        } else {
            Err(self)
        }
    }
}

impl TryInto<CancelledNotification> for ClientNotification {
    type Error = ClientNotification;
    fn try_into(self) -> Result<CancelledNotification, Self::Error> {
        if let ClientNotification::CancelledNotification(t) = self {
            Ok(t)
        } else {
            Err(self)
        }
    }
}
impl From<CancelledNotification> for ServerNotification {
    fn from(value: CancelledNotification) -> Self {
        ServerNotification::CancelledNotification(value)
    }
}

impl From<CancelledNotification> for ClientNotification {
    fn from(value: CancelledNotification) -> Self {
        ClientNotification::CancelledNotification(value)
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[test]
    fn test_notification_serde() {
        let raw = json!( {
            "jsonrpc": JsonRpcVersion2_0,
            "method": InitializedNotificationMethod,
        });
        let message: ClientJsonRpcMessage =
            serde_json::from_value(raw.clone()).expect("invalid notification");
        match &message {
            ClientJsonRpcMessage::Notification(JsonRpcNotification {
                notification: ClientNotification::InitializedNotification(_n),
                ..
            }) => {}
            _ => panic!("Expected Notification"),
        }
        let json = serde_json::to_value(message).expect("valid json");
        assert_eq!(json, raw);
    }

    #[test]
    fn test_request_conversion() {
        let raw = json!( {
            "jsonrpc": JsonRpcVersion2_0,
            "id": 1,
            "method": "request",
            "params": {"key": "value"},
        });
        let message: JsonRpcMessage = serde_json::from_value(raw.clone()).expect("invalid request");

        match &message {
            JsonRpcMessage::Request(r) => {
                assert_eq!(r.id, RequestId::Number(1));
                assert_eq!(r.request.method, "request");
                assert_eq!(
                    &r.request.params,
                    json!({"key": "value"})
                        .as_object()
                        .expect("should be an object")
                );
            }
            _ => panic!("Expected Request"),
        }
        let json = serde_json::to_value(&message).expect("valid json");
        assert_eq!(json, raw);
    }

    #[test]
    fn test_initial_request_response_serde() {
        let request = json!({
          "jsonrpc": "2.0",
          "id": 1,
          "method": "initialize",
          "params": {
            "protocolVersion": "2024-11-05",
            "capabilities": {
              "roots": {
                "listChanged": true
              },
              "sampling": {}
            },
            "clientInfo": {
              "name": "ExampleClient",
              "version": "1.0.0"
            }
          }
        });
        let raw_response_json = json!({
          "jsonrpc": "2.0",
          "id": 1,
          "result": {
            "protocolVersion": "2024-11-05",
            "capabilities": {
              "logging": {},
              "prompts": {
                "listChanged": true
              },
              "resources": {
                "subscribe": true,
                "listChanged": true
              },
              "tools": {
                "listChanged": true
              }
            },
            "serverInfo": {
              "name": "ExampleServer",
              "version": "1.0.0"
            }
          }
        });
        let request: ClientJsonRpcMessage =
            serde_json::from_value(request.clone()).expect("invalid request");
        let (request, id) = request.into_request().expect("should be a request");
        assert_eq!(id, RequestId::Number(1));
        match request {
            ClientRequest::InitializeRequest(Request {
                method: _,
                params:
                    InitializeRequestParam {
                        protocol_version: _,
                        capabilities,
                        client_info,
                    },
                ..
            }) => {
                assert_eq!(capabilities.roots.unwrap().list_changed, Some(true));
                assert_eq!(capabilities.sampling.unwrap().len(), 0);
                assert_eq!(client_info.name, "ExampleClient");
                assert_eq!(client_info.version, "1.0.0");
            }
            _ => panic!("Expected InitializeRequest"),
        }
        let server_response: ServerJsonRpcMessage =
            serde_json::from_value(raw_response_json.clone()).expect("invalid response");
        let (response, id) = server_response
            .clone()
            .into_response()
            .expect("expect response");
        assert_eq!(id, RequestId::Number(1));
        match response {
            ServerResult::InitializeResult(InitializeResult {
                protocol_version: _,
                capabilities,
                server_info,
                instructions,
            }) => {
                assert_eq!(capabilities.logging.unwrap().len(), 0);
                assert_eq!(capabilities.prompts.unwrap().list_changed, Some(true));
                assert_eq!(
                    capabilities.resources.as_ref().unwrap().subscribe,
                    Some(true)
                );
                assert_eq!(capabilities.resources.unwrap().list_changed, Some(true));
                assert_eq!(capabilities.tools.unwrap().list_changed, Some(true));
                assert_eq!(server_info.name, "ExampleServer");
                assert_eq!(server_info.version, "1.0.0");
                assert_eq!(instructions, None);
            }
            other => panic!("Expected InitializeResult, got {other:?}"),
        }

        let server_response_json: Value = serde_json::to_value(&server_response).expect("msg");

        assert_eq!(server_response_json, raw_response_json);
    }

    #[test]
    fn test_protocol_version_order() {
        let v1 = ProtocolVersion::V_2024_11_05;
        let v2 = ProtocolVersion::V_2025_03_26;
        assert!(v1 < v2);
    }
}

---
File: modelcontextprotocol-rust-sdk-13e72ec/crates/rmcp/src/model/annotated.rs
---

use std::ops::{Deref, DerefMut};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use super::{
    RawAudioContent, RawContent, RawEmbeddedResource, RawImageContent, RawResource,
    RawResourceTemplate, RawTextContent, Role,
};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
pub struct Annotations {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub audience: Option<Vec<Role>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub priority: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timestamp: Option<DateTime<Utc>>,
}

impl Annotations {
    /// Creates a new Annotations instance specifically for resources
    /// optional priority, and a timestamp (defaults to now if None)
    pub fn for_resource(priority: f32, timestamp: DateTime<Utc>) -> Self {
        assert!(
            (0.0..=1.0).contains(&priority),
            "Priority {priority} must be between 0.0 and 1.0"
        );
        Annotations {
            priority: Some(priority),
            timestamp: Some(timestamp),
            audience: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
pub struct Annotated<T: AnnotateAble> {
    #[serde(flatten)]
    pub raw: T,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub annotations: Option<Annotations>,
}

impl<T: AnnotateAble> Deref for Annotated<T> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        &self.raw
    }
}

impl<T: AnnotateAble> DerefMut for Annotated<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.raw
    }
}

impl<T: AnnotateAble> Annotated<T> {
    pub fn new(raw: T, annotations: Option<Annotations>) -> Self {
        Self { raw, annotations }
    }
    pub fn remove_annotation(&mut self) -> Option<Annotations> {
        self.annotations.take()
    }
    pub fn audience(&self) -> Option<&Vec<Role>> {
        self.annotations.as_ref().and_then(|a| a.audience.as_ref())
    }
    pub fn priority(&self) -> Option<f32> {
        self.annotations.as_ref().and_then(|a| a.priority)
    }
    pub fn timestamp(&self) -> Option<DateTime<Utc>> {
        self.annotations.as_ref().and_then(|a| a.timestamp)
    }
    pub fn with_audience(self, audience: Vec<Role>) -> Annotated<T>
    where
        Self: Sized,
    {
        if let Some(annotations) = self.annotations {
            Annotated {
                raw: self.raw,
                annotations: Some(Annotations {
                    audience: Some(audience),
                    ..annotations
                }),
            }
        } else {
            Annotated {
                raw: self.raw,
                annotations: Some(Annotations {
                    audience: Some(audience),
                    priority: None,
                    timestamp: None,
                }),
            }
        }
    }
    pub fn with_priority(self, priority: f32) -> Annotated<T>
    where
        Self: Sized,
    {
        if let Some(annotations) = self.annotations {
            Annotated {
                raw: self.raw,
                annotations: Some(Annotations {
                    priority: Some(priority),
                    ..annotations
                }),
            }
        } else {
            Annotated {
                raw: self.raw,
                annotations: Some(Annotations {
                    priority: Some(priority),
                    timestamp: None,
                    audience: None,
                }),
            }
        }
    }
    pub fn with_timestamp(self, timestamp: DateTime<Utc>) -> Annotated<T>
    where
        Self: Sized,
    {
        if let Some(annotations) = self.annotations {
            Annotated {
                raw: self.raw,
                annotations: Some(Annotations {
                    timestamp: Some(timestamp),
                    ..annotations
                }),
            }
        } else {
            Annotated {
                raw: self.raw,
                annotations: Some(Annotations {
                    timestamp: Some(timestamp),
                    priority: None,
                    audience: None,
                }),
            }
        }
    }
    pub fn with_timestamp_now(self) -> Annotated<T>
    where
        Self: Sized,
    {
        self.with_timestamp(Utc::now())
    }
}

mod sealed {
    pub trait Sealed {}
}
macro_rules! annotate {
    ($T: ident) => {
        impl sealed::Sealed for $T {}
        impl AnnotateAble for $T {}
    };
}

annotate!(RawContent);
annotate!(RawTextContent);
annotate!(RawImageContent);
annotate!(RawAudioContent);
annotate!(RawEmbeddedResource);
annotate!(RawResource);
annotate!(RawResourceTemplate);

pub trait AnnotateAble: sealed::Sealed {
    fn optional_annotate(self, annotations: Option<Annotations>) -> Annotated<Self>
    where
        Self: Sized,
    {
        Annotated::new(self, annotations)
    }
    fn annotate(self, annotations: Annotations) -> Annotated<Self>
    where
        Self: Sized,
    {
        Annotated::new(self, Some(annotations))
    }
    fn no_annotation(self) -> Annotated<Self>
    where
        Self: Sized,
    {
        Annotated::new(self, None)
    }
    fn with_audience(self, audience: Vec<Role>) -> Annotated<Self>
    where
        Self: Sized,
    {
        self.annotate(Annotations {
            audience: Some(audience),
            ..Default::default()
        })
    }
    fn with_priority(self, priority: f32) -> Annotated<Self>
    where
        Self: Sized,
    {
        self.annotate(Annotations {
            priority: Some(priority),
            ..Default::default()
        })
    }
    fn with_timestamp(self, timestamp: DateTime<Utc>) -> Annotated<Self>
    where
        Self: Sized,
    {
        self.annotate(Annotations {
            timestamp: Some(timestamp),
            ..Default::default()
        })
    }
    fn with_timestamp_now(self) -> Annotated<Self>
    where
        Self: Sized,
    {
        self.with_timestamp(Utc::now())
    }
}

---
File: modelcontextprotocol-rust-sdk-13e72ec/crates/rmcp/src/model/capabilities.rs
---

use std::{collections::BTreeMap, marker::PhantomData};

use paste::paste;
use serde::{Deserialize, Serialize};

use super::JsonObject;
pub type ExperimentalCapabilities = BTreeMap<String, JsonObject>;

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Default)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
pub struct PromptsCapability {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub list_changed: Option<bool>,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Default)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
pub struct ResourcesCapability {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub subscribe: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub list_changed: Option<bool>,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Default)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
pub struct ToolsCapability {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub list_changed: Option<bool>,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Default)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
pub struct RootsCapabilities {
    pub list_changed: Option<bool>,
}

///
/// # Builder
/// ```rust
/// # use rmcp::model::ClientCapabilities;
/// let cap = ClientCapabilities::builder()
///     .enable_experimental()
///     .enable_roots()
///     .enable_roots_list_changed()
///     .build();
/// ```
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Default)]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
pub struct ClientCapabilities {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub experimental: Option<ExperimentalCapabilities>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub roots: Option<RootsCapabilities>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sampling: Option<JsonObject>,
}

///
/// ## Builder
/// ```rust
/// # use rmcp::model::ServerCapabilities;
/// let cap = ServerCapabilities::builder()
///     .enable_logging()
///     .enable_experimental()
///     .enable_prompts()
///     .enable_resources()
///     .enable_tools()
///     .enable_tool_list_changed()
///     .build();
/// ```
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Default)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
pub struct ServerCapabilities {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub experimental: Option<ExperimentalCapabilities>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub logging: Option<JsonObject>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub completions: Option<JsonObject>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prompts: Option<PromptsCapability>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resources: Option<ResourcesCapability>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tools: Option<ToolsCapability>,
}

macro_rules! builder {
    ($Target: ident {$($f: ident: $T: ty),* $(,)?}) => {
        paste! {
            #[derive(Default, Clone, Copy, Debug)]
            pub struct [<$Target BuilderState>]<
                $(const [<$f:upper>]: bool = false,)*
            >;
            #[derive(Debug, Default)]
            pub struct [<$Target Builder>]<S = [<$Target BuilderState>]> {
                $(pub $f: Option<$T>,)*
                pub state: PhantomData<S>
            }
            impl $Target {
                #[doc = "Create a new [`" $Target "`] builder."]
                pub fn builder() -> [<$Target Builder>] {
                    <[<$Target Builder>]>::default()
                }
            }
            impl<S> [<$Target Builder>]<S> {
                pub fn build(self) -> $Target {
                    $Target {
                        $( $f: self.$f, )*
                    }
                }
            }
            impl<S> From<[<$Target Builder>]<S>> for $Target {
                fn from(builder: [<$Target Builder>]<S>) -> Self {
                    builder.build()
                }
            }
        }
        builder!($Target @toggle $($f: $T,) *);

    };
    ($Target: ident @toggle $f0: ident: $T0: ty, $($f: ident: $T: ty,)*) => {
        builder!($Target @toggle [][$f0: $T0][$($f: $T,)*]);
    };
    ($Target: ident @toggle [$($ff: ident: $Tf: ty,)*][$fn: ident: $TN: ty][$fn_1: ident: $Tn_1: ty, $($ft: ident: $Tt: ty,)*]) => {
        builder!($Target @impl_toggle [$($ff: $Tf,)*][$fn: $TN][$fn_1: $Tn_1, $($ft:$Tt,)*]);
        builder!($Target @toggle [$($ff: $Tf,)* $fn: $TN,][$fn_1: $Tn_1][$($ft:$Tt,)*]);
    };
    ($Target: ident @toggle [$($ff: ident: $Tf: ty,)*][$fn: ident: $TN: ty][]) => {
        builder!($Target @impl_toggle [$($ff: $Tf,)*][$fn: $TN][]);
    };
    ($Target: ident @impl_toggle [$($ff: ident: $Tf: ty,)*][$fn: ident: $TN: ty][$($ft: ident: $Tt: ty,)*]) => {
        paste! {
            impl<
                $(const [<$ff:upper>]: bool,)*
                $(const [<$ft:upper>]: bool,)*
            > [<$Target Builder>]<[<$Target BuilderState>]<
                $([<$ff:upper>],)*
                false,
                $([<$ft:upper>],)*
            >> {
                pub fn [<enable_ $fn>](self) -> [<$Target Builder>]<[<$Target BuilderState>]<
                    $([<$ff:upper>],)*
                    true,
                    $([<$ft:upper>],)*
                >> {
                    [<$Target Builder>] {
                        $( $ff: self.$ff, )*
                        $fn: Some($TN::default()),
                        $( $ft: self.$ft, )*
                        state: PhantomData
                    }
                }
                pub fn [<enable_ $fn _with>](self, $fn: $TN) -> [<$Target Builder>]<[<$Target BuilderState>]<
                    $([<$ff:upper>],)*
                    true,
                    $([<$ft:upper>],)*
                >> {
                    [<$Target Builder>] {
                        $( $ff: self.$ff, )*
                        $fn: Some($fn),
                        $( $ft: self.$ft, )*
                        state: PhantomData
                    }
                }
            }
            // do we really need to disable some thing in builder?
            // impl<
            //     $(const [<$ff:upper>]: bool,)*
            //     $(const [<$ft:upper>]: bool,)*
            // > [<$Target Builder>]<[<$Target BuilderState>]<
            //     $([<$ff:upper>],)*
            //     true,
            //     $([<$ft:upper>],)*
            // >> {
            //     pub fn [<disable_ $fn>](self) -> [<$Target Builder>]<[<$Target BuilderState>]<
            //         $([<$ff:upper>],)*
            //         false,
            //         $([<$ft:upper>],)*
            //     >> {
            //         [<$Target Builder>] {
            //             $( $ff: self.$ff, )*
            //             $fn: None,
            //             $( $ft: self.$ft, )*
            //             state: PhantomData
            //         }
            //     }
            // }
        }
    }
}

builder! {
    ServerCapabilities {
        experimental: ExperimentalCapabilities,
        logging: JsonObject,
        completions: JsonObject,
        prompts: PromptsCapability,
        resources: ResourcesCapability,
        tools: ToolsCapability
    }
}

impl<const E: bool, const L: bool, const C: bool, const P: bool, const R: bool>
    ServerCapabilitiesBuilder<ServerCapabilitiesBuilderState<E, L, C, P, R, true>>
{
    pub fn enable_tool_list_changed(mut self) -> Self {
        if let Some(c) = self.tools.as_mut() {
            c.list_changed = Some(true);
        }
        self
    }
}

impl<const E: bool, const L: bool, const C: bool, const R: bool, const T: bool>
    ServerCapabilitiesBuilder<ServerCapabilitiesBuilderState<E, L, C, true, R, T>>
{
    pub fn enable_prompts_list_changed(mut self) -> Self {
        if let Some(c) = self.prompts.as_mut() {
            c.list_changed = Some(true);
        }
        self
    }
}

impl<const E: bool, const L: bool, const C: bool, const P: bool, const T: bool>
    ServerCapabilitiesBuilder<ServerCapabilitiesBuilderState<E, L, C, P, true, T>>
{
    pub fn enable_resources_list_changed(mut self) -> Self {
        if let Some(c) = self.resources.as_mut() {
            c.list_changed = Some(true);
        }
        self
    }

    pub fn enable_resources_subscribe(mut self) -> Self {
        if let Some(c) = self.resources.as_mut() {
            c.subscribe = Some(true);
        }
        self
    }
}

builder! {
    ClientCapabilities{
        experimental: ExperimentalCapabilities,
        roots: RootsCapabilities,
        sampling: JsonObject,
    }
}

impl<const E: bool, const S: bool>
    ClientCapabilitiesBuilder<ClientCapabilitiesBuilderState<E, true, S>>
{
    pub fn enable_roots_list_changed(mut self) -> Self {
        if let Some(c) = self.roots.as_mut() {
            c.list_changed = Some(true);
        }
        self
    }
}

#[cfg(test)]
mod test {
    use super::*;
    #[test]
    fn test_builder() {
        let builder = <ServerCapabilitiesBuilder>::default()
            .enable_logging()
            .enable_experimental()
            .enable_prompts()
            .enable_resources()
            .enable_tools()
            .enable_tool_list_changed();
        assert_eq!(builder.logging, Some(JsonObject::default()));
        assert_eq!(builder.prompts, Some(PromptsCapability::default()));
        assert_eq!(builder.resources, Some(ResourcesCapability::default()));
        assert_eq!(
            builder.tools,
            Some(ToolsCapability {
                list_changed: Some(true),
            })
        );
        assert_eq!(
            builder.experimental,
            Some(ExperimentalCapabilities::default())
        );
        let client_builder = <ClientCapabilitiesBuilder>::default()
            .enable_experimental()
            .enable_roots()
            .enable_roots_list_changed()
            .enable_sampling();
        assert_eq!(
            client_builder.experimental,
            Some(ExperimentalCapabilities::default())
        );
        assert_eq!(
            client_builder.roots,
            Some(RootsCapabilities {
                list_changed: Some(true),
            })
        );
    }
}

---
File: modelcontextprotocol-rust-sdk-13e72ec/crates/rmcp/src/model/content.rs
---

//! Content sent around agents, extensions, and LLMs
//! The various content types can be display to humans but also understood by models
//! They include optional annotations used to help inform agent usage
use serde::{Deserialize, Serialize};
use serde_json::json;

use super::{AnnotateAble, Annotated, resource::ResourceContents};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
pub struct RawTextContent {
    pub text: String,
}
pub type TextContent = Annotated<RawTextContent>;
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
pub struct RawImageContent {
    /// The base64-encoded image
    pub data: String,
    pub mime_type: String,
}

pub type ImageContent = Annotated<RawImageContent>;
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
pub struct RawEmbeddedResource {
    pub resource: ResourceContents,
}
pub type EmbeddedResource = Annotated<RawEmbeddedResource>;

impl EmbeddedResource {
    pub fn get_text(&self) -> String {
        match &self.resource {
            ResourceContents::TextResourceContents { text, .. } => text.clone(),
            _ => String::new(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
pub struct RawAudioContent {
    pub data: String,
    pub mime_type: String,
}

pub type AudioContent = Annotated<RawAudioContent>;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
pub enum RawContent {
    Text(RawTextContent),
    Image(RawImageContent),
    Resource(RawEmbeddedResource),
    Audio(AudioContent),
}

pub type Content = Annotated<RawContent>;

impl RawContent {
    pub fn json<S: Serialize>(json: S) -> Result<Self, crate::Error> {
        let json = serde_json::to_string(&json).map_err(|e| {
            crate::Error::internal_error(
                "fail to serialize response to json",
                Some(json!(
                    {"reason": e.to_string()}
                )),
            )
        })?;
        Ok(RawContent::text(json))
    }

    pub fn text<S: Into<String>>(text: S) -> Self {
        RawContent::Text(RawTextContent { text: text.into() })
    }

    pub fn image<S: Into<String>, T: Into<String>>(data: S, mime_type: T) -> Self {
        RawContent::Image(RawImageContent {
            data: data.into(),
            mime_type: mime_type.into(),
        })
    }

    pub fn resource(resource: ResourceContents) -> Self {
        RawContent::Resource(RawEmbeddedResource { resource })
    }

    pub fn embedded_text<S: Into<String>, T: Into<String>>(uri: S, content: T) -> Self {
        RawContent::Resource(RawEmbeddedResource {
            resource: ResourceContents::TextResourceContents {
                uri: uri.into(),
                mime_type: Some("text".to_string()),
                text: content.into(),
            },
        })
    }

    /// Get the text content if this is a TextContent variant
    pub fn as_text(&self) -> Option<&RawTextContent> {
        match self {
            RawContent::Text(text) => Some(text),
            _ => None,
        }
    }

    /// Get the image content if this is an ImageContent variant
    pub fn as_image(&self) -> Option<&RawImageContent> {
        match self {
            RawContent::Image(image) => Some(image),
            _ => None,
        }
    }

    /// Get the resource content if this is an ImageContent variant
    pub fn as_resource(&self) -> Option<&RawEmbeddedResource> {
        match self {
            RawContent::Resource(resource) => Some(resource),
            _ => None,
        }
    }
}

impl Content {
    pub fn text<S: Into<String>>(text: S) -> Self {
        RawContent::text(text).no_annotation()
    }

    pub fn image<S: Into<String>, T: Into<String>>(data: S, mime_type: T) -> Self {
        RawContent::image(data, mime_type).no_annotation()
    }

    pub fn resource(resource: ResourceContents) -> Self {
        RawContent::resource(resource).no_annotation()
    }

    pub fn embedded_text<S: Into<String>, T: Into<String>>(uri: S, content: T) -> Self {
        RawContent::embedded_text(uri, content).no_annotation()
    }

    pub fn json<S: Serialize>(json: S) -> Result<Self, crate::Error> {
        RawContent::json(json).map(|c| c.no_annotation())
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct JsonContent<S: Serialize>(S);
/// Types that can be converted into a list of contents
pub trait IntoContents {
    fn into_contents(self) -> Vec<Content>;
}

impl IntoContents for Content {
    fn into_contents(self) -> Vec<Content> {
        vec![self]
    }
}

impl IntoContents for String {
    fn into_contents(self) -> Vec<Content> {
        vec![Content::text(self)]
    }
}

---
File: modelcontextprotocol-rust-sdk-13e72ec/crates/rmcp/src/model/extension.rs
---

//! A container for those extra data could be carried on request or notification.
//!
//! This file is copied and modified from crate [http](https://github.com/hyperium/http).
//!
//! - Original code license: <https://github.com/hyperium/http/blob/master/LICENSE-MIT>
//! - Original code: <https://github.com/hyperium/http/blob/master/src/extensions.rs>
use std::{
    any::{Any, TypeId},
    collections::HashMap,
    fmt,
    hash::{BuildHasherDefault, Hasher},
};

type AnyMap = HashMap<TypeId, Box<dyn AnyClone + Send + Sync>, BuildHasherDefault<IdHasher>>;

// With TypeIds as keys, there's no need to hash them. They are already hashes
// themselves, coming from the compiler. The IdHasher just holds the u64 of
// the TypeId, and then returns it, instead of doing any bit fiddling.
#[derive(Default)]
struct IdHasher(u64);

impl Hasher for IdHasher {
    fn write(&mut self, _: &[u8]) {
        unreachable!("TypeId calls write_u64");
    }

    #[inline]
    fn write_u64(&mut self, id: u64) {
        self.0 = id;
    }

    #[inline]
    fn finish(&self) -> u64 {
        self.0
    }
}

/// A type map of protocol extensions.
///
/// `Extensions` can be used by `Request` `Notification` and `Response` to store
/// extra data derived from the underlying protocol.
#[derive(Clone, Default)]
pub struct Extensions {
    // If extensions are never used, no need to carry around an empty HashMap.
    // That's 3 words. Instead, this is only 1 word.
    map: Option<Box<AnyMap>>,
}

impl Extensions {
    /// Create an empty `Extensions`.
    #[inline]
    pub const fn new() -> Extensions {
        Extensions { map: None }
    }

    /// Insert a type into this `Extensions`.
    ///
    /// If a extension of this type already existed, it will
    /// be returned and replaced with the new one.
    ///
    /// # Example
    ///
    /// ```
    /// # use rmcp::model::Extensions;
    /// let mut ext = Extensions::new();
    /// assert!(ext.insert(5i32).is_none());
    /// assert!(ext.insert(4u8).is_none());
    /// assert_eq!(ext.insert(9i32), Some(5i32));
    /// ```
    pub fn insert<T: Clone + Send + Sync + 'static>(&mut self, val: T) -> Option<T> {
        self.map
            .get_or_insert_with(Box::default)
            .insert(TypeId::of::<T>(), Box::new(val))
            .and_then(|boxed| boxed.into_any().downcast().ok().map(|boxed| *boxed))
    }

    /// Get a reference to a type previously inserted on this `Extensions`.
    ///
    /// # Example
    ///
    /// ```
    /// # use rmcp::model::Extensions;
    /// let mut ext = Extensions::new();
    /// assert!(ext.get::<i32>().is_none());
    /// ext.insert(5i32);
    ///
    /// assert_eq!(ext.get::<i32>(), Some(&5i32));
    /// ```
    pub fn get<T: Send + Sync + 'static>(&self) -> Option<&T> {
        self.map
            .as_ref()
            .and_then(|map| map.get(&TypeId::of::<T>()))
            .and_then(|boxed| (**boxed).as_any().downcast_ref())
    }

    /// Get a mutable reference to a type previously inserted on this `Extensions`.
    ///
    /// # Example
    ///
    /// ```
    /// # use rmcp::model::Extensions;
    /// let mut ext = Extensions::new();
    /// ext.insert(String::from("Hello"));
    /// ext.get_mut::<String>().unwrap().push_str(" World");
    ///
    /// assert_eq!(ext.get::<String>().unwrap(), "Hello World");
    /// ```
    pub fn get_mut<T: Send + Sync + 'static>(&mut self) -> Option<&mut T> {
        self.map
            .as_mut()
            .and_then(|map| map.get_mut(&TypeId::of::<T>()))
            .and_then(|boxed| (**boxed).as_any_mut().downcast_mut())
    }

    /// Get a mutable reference to a type, inserting `value` if not already present on this
    /// `Extensions`.
    ///
    /// # Example
    ///
    /// ```
    /// # use rmcp::model::Extensions;
    /// let mut ext = Extensions::new();
    /// *ext.get_or_insert(1i32) += 2;
    ///
    /// assert_eq!(*ext.get::<i32>().unwrap(), 3);
    /// ```
    pub fn get_or_insert<T: Clone + Send + Sync + 'static>(&mut self, value: T) -> &mut T {
        self.get_or_insert_with(|| value)
    }

    /// Get a mutable reference to a type, inserting the value created by `f` if not already present
    /// on this `Extensions`.
    ///
    /// # Example
    ///
    /// ```
    /// # use rmcp::model::Extensions;
    /// let mut ext = Extensions::new();
    /// *ext.get_or_insert_with(|| 1i32) += 2;
    ///
    /// assert_eq!(*ext.get::<i32>().unwrap(), 3);
    /// ```
    pub fn get_or_insert_with<T: Clone + Send + Sync + 'static, F: FnOnce() -> T>(
        &mut self,
        f: F,
    ) -> &mut T {
        let out = self
            .map
            .get_or_insert_with(Box::default)
            .entry(TypeId::of::<T>())
            .or_insert_with(|| Box::new(f()));
        (**out).as_any_mut().downcast_mut().unwrap()
    }

    /// Get a mutable reference to a type, inserting the type's default value if not already present
    /// on this `Extensions`.
    ///
    /// # Example
    ///
    /// ```
    /// # use rmcp::model::Extensions;
    /// let mut ext = Extensions::new();
    /// *ext.get_or_insert_default::<i32>() += 2;
    ///
    /// assert_eq!(*ext.get::<i32>().unwrap(), 2);
    /// ```
    pub fn get_or_insert_default<T: Default + Clone + Send + Sync + 'static>(&mut self) -> &mut T {
        self.get_or_insert_with(T::default)
    }

    /// Remove a type from this `Extensions`.
    ///
    /// If a extension of this type existed, it will be returned.
    ///
    /// # Example
    ///
    /// ```
    /// # use rmcp::model::Extensions;
    /// let mut ext = Extensions::new();
    /// ext.insert(5i32);
    /// assert_eq!(ext.remove::<i32>(), Some(5i32));
    /// assert!(ext.get::<i32>().is_none());
    /// ```
    pub fn remove<T: Send + Sync + 'static>(&mut self) -> Option<T> {
        self.map
            .as_mut()
            .and_then(|map| map.remove(&TypeId::of::<T>()))
            .and_then(|boxed| boxed.into_any().downcast().ok().map(|boxed| *boxed))
    }

    /// Clear the `Extensions` of all inserted extensions.
    ///
    /// # Example
    ///
    /// ```
    /// # use rmcp::model::Extensions;
    /// let mut ext = Extensions::new();
    /// ext.insert(5i32);
    /// ext.clear();
    ///
    /// assert!(ext.get::<i32>().is_none());
    /// ```
    #[inline]
    pub fn clear(&mut self) {
        if let Some(ref mut map) = self.map {
            map.clear();
        }
    }

    /// Check whether the extension set is empty or not.
    ///
    /// # Example
    ///
    /// ```
    /// # use rmcp::model::Extensions;
    /// let mut ext = Extensions::new();
    /// assert!(ext.is_empty());
    /// ext.insert(5i32);
    /// assert!(!ext.is_empty());
    /// ```
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.map.as_ref().is_none_or(|map| map.is_empty())
    }

    /// Get the number of extensions available.
    ///
    /// # Example
    ///
    /// ```
    /// # use rmcp::model::Extensions;
    /// let mut ext = Extensions::new();
    /// assert_eq!(ext.len(), 0);
    /// ext.insert(5i32);
    /// assert_eq!(ext.len(), 1);
    /// ```
    #[inline]
    pub fn len(&self) -> usize {
        self.map.as_ref().map_or(0, |map| map.len())
    }

    /// Extends `self` with another `Extensions`.
    ///
    /// If an instance of a specific type exists in both, the one in `self` is overwritten with the
    /// one from `other`.
    ///
    /// # Example
    ///
    /// ```
    /// # use rmcp::model::Extensions;
    /// let mut ext_a = Extensions::new();
    /// ext_a.insert(8u8);
    /// ext_a.insert(16u16);
    ///
    /// let mut ext_b = Extensions::new();
    /// ext_b.insert(4u8);
    /// ext_b.insert("hello");
    ///
    /// ext_a.extend(ext_b);
    /// assert_eq!(ext_a.len(), 3);
    /// assert_eq!(ext_a.get::<u8>(), Some(&4u8));
    /// assert_eq!(ext_a.get::<u16>(), Some(&16u16));
    /// assert_eq!(ext_a.get::<&'static str>().copied(), Some("hello"));
    /// ```
    pub fn extend(&mut self, other: Self) {
        if let Some(other) = other.map {
            if let Some(map) = &mut self.map {
                map.extend(*other);
            } else {
                self.map = Some(other);
            }
        }
    }
}

impl fmt::Debug for Extensions {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Extensions").finish()
    }
}

trait AnyClone: Any {
    fn clone_box(&self) -> Box<dyn AnyClone + Send + Sync>;
    fn as_any(&self) -> &dyn Any;
    fn as_any_mut(&mut self) -> &mut dyn Any;
    fn into_any(self: Box<Self>) -> Box<dyn Any>;
}

impl<T: Clone + Send + Sync + 'static> AnyClone for T {
    fn clone_box(&self) -> Box<dyn AnyClone + Send + Sync> {
        Box::new(self.clone())
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }

    fn into_any(self: Box<Self>) -> Box<dyn Any> {
        self
    }
}

impl Clone for Box<dyn AnyClone + Send + Sync> {
    fn clone(&self) -> Self {
        (**self).clone_box()
    }
}

#[test]
fn test_extensions() {
    #[derive(Clone, Debug, PartialEq)]
    struct MyType(i32);

    let mut extensions = Extensions::new();

    extensions.insert(5i32);
    extensions.insert(MyType(10));

    assert_eq!(extensions.get(), Some(&5i32));
    assert_eq!(extensions.get_mut(), Some(&mut 5i32));

    let ext2 = extensions.clone();

    assert_eq!(extensions.remove::<i32>(), Some(5i32));
    assert!(extensions.get::<i32>().is_none());

    // clone still has it
    assert_eq!(ext2.get(), Some(&5i32));
    assert_eq!(ext2.get(), Some(&MyType(10)));

    assert_eq!(extensions.get::<bool>(), None);
    assert_eq!(extensions.get(), Some(&MyType(10)));
}

---
File: modelcontextprotocol-rust-sdk-13e72ec/crates/rmcp/src/model/meta.rs
---

use std::ops::{Deref, DerefMut};

use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::{
    ClientNotification, ClientRequest, Extensions, JsonObject, JsonRpcMessage, NumberOrString,
    ProgressToken, ServerNotification, ServerRequest,
};

pub trait GetMeta {
    fn get_meta_mut(&mut self) -> &mut Meta;
    fn get_meta(&self) -> &Meta;
}

pub trait GetExtensions {
    fn extensions(&self) -> &Extensions;
    fn extensions_mut(&mut self) -> &mut Extensions;
}

macro_rules! variant_extension {
    (
        $Enum: ident {
            $($variant: ident)*
        }
    ) => {
        impl GetExtensions for $Enum {
            fn extensions(&self) -> &Extensions {
                match self {
                    $(
                        $Enum::$variant(v) => &v.extensions,
                    )*
                }
            }
            fn extensions_mut(&mut self) -> &mut Extensions {
                match self {
                    $(
                        $Enum::$variant(v) => &mut v.extensions,
                    )*
                }
            }
        }
        impl GetMeta for $Enum {
            fn get_meta_mut(&mut self) -> &mut Meta {
                self.extensions_mut().get_or_insert_default()
            }
            fn get_meta(&self) -> &Meta {
                self.extensions().get::<Meta>().unwrap_or(Meta::static_empty())
            }
        }
    };
}

variant_extension! {
    ClientRequest {
        PingRequest
        InitializeRequest
        CompleteRequest
        SetLevelRequest
        GetPromptRequest
        ListPromptsRequest
        ListResourcesRequest
        ListResourceTemplatesRequest
        ReadResourceRequest
        SubscribeRequest
        UnsubscribeRequest
        CallToolRequest
        ListToolsRequest
    }
}

variant_extension! {
    ServerRequest {
        PingRequest
        CreateMessageRequest
        ListRootsRequest
    }
}

variant_extension! {
    ClientNotification {
        CancelledNotification
        ProgressNotification
        InitializedNotification
        RootsListChangedNotification
    }
}

variant_extension! {
    ServerNotification {
        CancelledNotification
        ProgressNotification
        LoggingMessageNotification
        ResourceUpdatedNotification
        ResourceListChangedNotification
        ToolListChangedNotification
        PromptListChangedNotification
    }
}
#[derive(Debug, Serialize, Deserialize, Clone, Default)]
#[serde(transparent)]
pub struct Meta(pub JsonObject);
const PROGRESS_TOKEN_FIELD: &str = "progressToken";
impl Meta {
    pub fn new() -> Self {
        Self(JsonObject::new())
    }

    pub(crate) fn static_empty() -> &'static Self {
        static EMPTY: std::sync::OnceLock<Meta> = std::sync::OnceLock::new();
        EMPTY.get_or_init(Default::default)
    }

    pub fn get_progress_token(&self) -> Option<ProgressToken> {
        self.0.get(PROGRESS_TOKEN_FIELD).and_then(|v| match v {
            Value::String(s) => Some(ProgressToken(NumberOrString::String(s.to_string().into()))),
            Value::Number(n) => n
                .as_u64()
                .map(|n| ProgressToken(NumberOrString::Number(n as u32))),
            _ => None,
        })
    }

    pub fn set_progress_token(&mut self, token: ProgressToken) {
        match token.0 {
            NumberOrString::String(ref s) => self.0.insert(
                PROGRESS_TOKEN_FIELD.to_string(),
                Value::String(s.to_string()),
            ),
            NumberOrString::Number(n) => self
                .0
                .insert(PROGRESS_TOKEN_FIELD.to_string(), Value::Number(n.into())),
        };
    }

    pub fn extend(&mut self, other: Meta) {
        for (k, v) in other.0.into_iter() {
            self.0.insert(k, v);
        }
    }
}

impl Deref for Meta {
    type Target = JsonObject;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for Meta {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl<Req, Resp, Noti> JsonRpcMessage<Req, Resp, Noti>
where
    Req: GetExtensions,
    Noti: GetExtensions,
{
    pub fn insert_extension<T: Clone + Send + Sync + 'static>(&mut self, value: T) {
        match self {
            JsonRpcMessage::Request(json_rpc_request) => {
                json_rpc_request.request.extensions_mut().insert(value);
            }
            JsonRpcMessage::Notification(json_rpc_notification) => {
                json_rpc_notification
                    .notification
                    .extensions_mut()
                    .insert(value);
            }
            JsonRpcMessage::BatchRequest(json_rpc_batch_request_items) => {
                for item in json_rpc_batch_request_items {
                    match item {
                        super::JsonRpcBatchRequestItem::Request(json_rpc_request) => {
                            json_rpc_request
                                .request
                                .extensions_mut()
                                .insert(value.clone());
                        }
                        super::JsonRpcBatchRequestItem::Notification(json_rpc_notification) => {
                            json_rpc_notification
                                .notification
                                .extensions_mut()
                                .insert(value.clone());
                        }
                    }
                }
            }
            _ => {}
        }
    }
}

---
File: modelcontextprotocol-rust-sdk-13e72ec/crates/rmcp/src/model/prompt.rs
---

use base64::engine::{Engine, general_purpose::STANDARD as BASE64_STANDARD};
use serde::{Deserialize, Serialize};

use super::{
    AnnotateAble, Annotations, RawEmbeddedResource, RawImageContent,
    content::{EmbeddedResource, ImageContent},
    resource::ResourceContents,
};

/// A prompt that can be used to generate text from a model
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
pub struct Prompt {
    /// The name of the prompt
    pub name: String,
    /// Optional description of what the prompt does
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Optional arguments that can be passed to customize the prompt
    #[serde(skip_serializing_if = "Option::is_none")]
    pub arguments: Option<Vec<PromptArgument>>,
}

impl Prompt {
    /// Create a new prompt with the given name, description and arguments
    pub fn new<N, D>(
        name: N,
        description: Option<D>,
        arguments: Option<Vec<PromptArgument>>,
    ) -> Self
    where
        N: Into<String>,
        D: Into<String>,
    {
        Prompt {
            name: name.into(),
            description: description.map(Into::into),
            arguments,
        }
    }
}

/// Represents a prompt argument that can be passed to customize the prompt
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
pub struct PromptArgument {
    /// The name of the argument
    pub name: String,
    /// A description of what the argument is used for
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Whether this argument is required
    #[serde(skip_serializing_if = "Option::is_none")]
    pub required: Option<bool>,
}

/// Represents the role of a message sender in a prompt conversation
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
pub enum PromptMessageRole {
    User,
    Assistant,
}

/// Content types that can be included in prompt messages
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
pub enum PromptMessageContent {
    /// Plain text content
    Text { text: String },
    /// Image content with base64-encoded data
    Image {
        #[serde(flatten)]
        image: ImageContent,
    },
    /// Embedded server-side resource
    Resource { resource: EmbeddedResource },
}

impl PromptMessageContent {
    pub fn text(text: impl Into<String>) -> Self {
        Self::Text { text: text.into() }
    }
}

/// A message in a prompt conversation
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
pub struct PromptMessage {
    /// The role of the message sender
    pub role: PromptMessageRole,
    /// The content of the message
    pub content: PromptMessageContent,
}

impl PromptMessage {
    /// Create a new text message with the given role and text content
    pub fn new_text<S: Into<String>>(role: PromptMessageRole, text: S) -> Self {
        Self {
            role,
            content: PromptMessageContent::Text { text: text.into() },
        }
    }
    #[cfg(feature = "base64")]
    pub fn new_image(
        role: PromptMessageRole,
        data: &[u8],
        mime_type: &str,
        annotations: Option<Annotations>,
    ) -> Self {
        let mime_type = mime_type.into();

        let base64 = BASE64_STANDARD.encode(data);

        Self {
            role,
            content: PromptMessageContent::Image {
                image: RawImageContent {
                    data: base64,
                    mime_type,
                }
                .optional_annotate(annotations),
            },
        }
    }

    /// Create a new resource message
    pub fn new_resource(
        role: PromptMessageRole,
        uri: String,
        mime_type: String,
        text: Option<String>,
        annotations: Option<Annotations>,
    ) -> Self {
        let resource_contents = ResourceContents::TextResourceContents {
            uri,
            mime_type: Some(mime_type),
            text: text.unwrap_or_default(),
        };

        Self {
            role,
            content: PromptMessageContent::Resource {
                resource: RawEmbeddedResource {
                    resource: resource_contents,
                }
                .optional_annotate(annotations),
            },
        }
    }
}

/// A template for a prompt
#[derive(Debug, Serialize, Deserialize)]
pub struct PromptTemplate {
    pub id: String,
    pub template: String,
    pub arguments: Vec<PromptArgumentTemplate>,
}

/// A template for a prompt argument, this should be identical to PromptArgument
#[derive(Debug, Serialize, Deserialize)]
pub struct PromptArgumentTemplate {
    pub name: String,
    pub description: Option<String>,
    pub required: Option<bool>,
}

---
File: modelcontextprotocol-rust-sdk-13e72ec/crates/rmcp/src/model/resource.rs
---

use serde::{Deserialize, Serialize};

use super::Annotated;

/// Represents a resource in the extension with metadata
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
pub struct RawResource {
    /// URI representing the resource location (e.g., "file:///path/to/file" or "str:///content")
    pub uri: String,
    /// Name of the resource
    pub name: String,
    /// Optional description of the resource
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// MIME type of the resource content ("text" or "blob")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mime_type: Option<String>,

    /// The size of the raw resource content, in bytes (i.e., before base64 encoding or any tokenization), if known.
    ///
    /// This can be used by Hosts to display file sizes and estimate context window us
    #[serde(skip_serializing_if = "Option::is_none")]
    pub size: Option<u32>,
}

pub type Resource = Annotated<RawResource>;

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
pub struct RawResourceTemplate {
    pub uri_template: String,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mime_type: Option<String>,
}

pub type ResourceTemplate = Annotated<RawResourceTemplate>;

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
#[serde(rename_all = "camelCase", untagged)]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
pub enum ResourceContents {
    TextResourceContents {
        uri: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        mime_type: Option<String>,
        text: String,
    },
    BlobResourceContents {
        uri: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        mime_type: Option<String>,
        blob: String,
    },
}

impl ResourceContents {
    pub fn text(text: impl Into<String>, uri: impl Into<String>) -> Self {
        Self::TextResourceContents {
            uri: uri.into(),
            mime_type: Some("text".into()),
            text: text.into(),
        }
    }
}

impl RawResource {
    /// Creates a new Resource from a URI with explicit mime type
    pub fn new(uri: impl Into<String>, name: impl Into<String>) -> Self {
        Self {
            uri: uri.into(),
            name: name.into(),
            description: None,
            mime_type: None,
            size: None,
        }
    }
}

---
File: modelcontextprotocol-rust-sdk-13e72ec/crates/rmcp/src/model/serde_impl.rs
---

use std::borrow::Cow;

use serde::{Deserialize, Serialize};

use super::{
    Extensions, Meta, Notification, NotificationNoParam, Request, RequestNoParam,
    RequestOptionalParam,
};
#[derive(Serialize, Deserialize)]
struct WithMeta<'a, P> {
    #[serde(skip_serializing_if = "Option::is_none")]
    _meta: Option<Cow<'a, Meta>>,
    #[serde(flatten)]
    _rest: P,
}

#[derive(Serialize, Deserialize)]
struct Proxy<'a, M, P> {
    method: M,
    params: WithMeta<'a, P>,
}

#[derive(Serialize, Deserialize)]
struct ProxyOptionalParam<'a, M, P> {
    method: M,
    params: Option<WithMeta<'a, P>>,
}

#[derive(Serialize, Deserialize)]
struct ProxyNoParam<M> {
    method: M,
}

impl<M, R> Serialize for Request<M, R>
where
    M: Serialize,
    R: Serialize,
{
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let extensions = &self.extensions;
        let _meta = extensions.get::<Meta>().map(Cow::Borrowed);
        Proxy::serialize(
            &Proxy {
                method: &self.method,
                params: WithMeta {
                    _rest: &self.params,
                    _meta,
                },
            },
            serializer,
        )
    }
}

impl<'de, M, R> Deserialize<'de> for Request<M, R>
where
    M: Deserialize<'de>,
    R: Deserialize<'de>,
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let body = Proxy::deserialize(deserializer)?;
        let _meta = body.params._meta.map(|m| m.into_owned());
        let mut extensions = Extensions::new();
        if let Some(meta) = _meta {
            extensions.insert(meta);
        }
        Ok(Request {
            extensions,
            method: body.method,
            params: body.params._rest,
        })
    }
}

impl<M, R> Serialize for RequestOptionalParam<M, R>
where
    M: Serialize,
    R: Serialize,
{
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let extensions = &self.extensions;
        let _meta = extensions.get::<Meta>().map(Cow::Borrowed);
        Proxy::serialize(
            &Proxy {
                method: &self.method,
                params: WithMeta {
                    _rest: &self.params,
                    _meta,
                },
            },
            serializer,
        )
    }
}

impl<'de, M, R> Deserialize<'de> for RequestOptionalParam<M, R>
where
    M: Deserialize<'de>,
    R: Deserialize<'de>,
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let body = ProxyOptionalParam::<'_, _, Option<R>>::deserialize(deserializer)?;
        let mut params = None;
        let mut _meta = None;
        if let Some(body_params) = body.params {
            params = body_params._rest;
            _meta = body_params._meta.map(|m| m.into_owned());
        }
        let mut extensions = Extensions::new();
        if let Some(meta) = _meta {
            extensions.insert(meta);
        }
        Ok(RequestOptionalParam {
            extensions,
            method: body.method,
            params,
        })
    }
}

impl<M> Serialize for RequestNoParam<M>
where
    M: Serialize,
{
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let extensions = &self.extensions;
        let _meta = extensions.get::<Meta>().map(Cow::Borrowed);
        ProxyNoParam::serialize(
            &ProxyNoParam {
                method: &self.method,
            },
            serializer,
        )
    }
}

impl<'de, M> Deserialize<'de> for RequestNoParam<M>
where
    M: Deserialize<'de>,
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let body = ProxyNoParam::<_>::deserialize(deserializer)?;
        let extensions = Extensions::new();
        Ok(RequestNoParam {
            extensions,
            method: body.method,
        })
    }
}

impl<M, R> Serialize for Notification<M, R>
where
    M: Serialize,
    R: Serialize,
{
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let extensions = &self.extensions;
        let _meta = extensions.get::<Meta>().map(Cow::Borrowed);
        Proxy::serialize(
            &Proxy {
                method: &self.method,
                params: WithMeta {
                    _rest: &self.params,
                    _meta,
                },
            },
            serializer,
        )
    }
}

impl<'de, M, R> Deserialize<'de> for Notification<M, R>
where
    M: Deserialize<'de>,
    R: Deserialize<'de>,
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let body = Proxy::deserialize(deserializer)?;
        let _meta = body.params._meta.map(|m| m.into_owned());
        let mut extensions = Extensions::new();
        if let Some(meta) = _meta {
            extensions.insert(meta);
        }
        Ok(Notification {
            extensions,
            method: body.method,
            params: body.params._rest,
        })
    }
}

impl<M> Serialize for NotificationNoParam<M>
where
    M: Serialize,
{
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let extensions = &self.extensions;
        let _meta = extensions.get::<Meta>().map(Cow::Borrowed);
        ProxyNoParam::serialize(
            &ProxyNoParam {
                method: &self.method,
            },
            serializer,
        )
    }
}

impl<'de, M> Deserialize<'de> for NotificationNoParam<M>
where
    M: Deserialize<'de>,
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let body = ProxyNoParam::<_>::deserialize(deserializer)?;
        let extensions = Extensions::new();
        Ok(NotificationNoParam {
            extensions,
            method: body.method,
        })
    }
}

#[cfg(test)]
mod test {
    use serde_json::json;

    use crate::model::ListToolsRequest;

    #[test]
    fn test_deserialize_lost_tools_request() {
        let _req: ListToolsRequest = serde_json::from_value(json!(
            {
                "method": "tools/list",
            }
        ))
        .unwrap();
    }
}

---
File: modelcontextprotocol-rust-sdk-13e72ec/crates/rmcp/src/model/tool.rs
---

use std::{borrow::Cow, sync::Arc};

/// Tools represent a routine that a server can execute
/// Tool calls represent requests from the client to execute one
use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::JsonObject;

/// A tool that can be used by a model.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
pub struct Tool {
    /// The name of the tool
    pub name: Cow<'static, str>,
    /// A description of what the tool does
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<Cow<'static, str>>,
    /// A JSON Schema object defining the expected parameters for the tool
    pub input_schema: Arc<JsonObject>,
    #[serde(skip_serializing_if = "Option::is_none")]
    /// Optional additional tool information.
    pub annotations: Option<ToolAnnotations>,
}

/// Additional properties describing a Tool to clients.
///
/// NOTE: all properties in ToolAnnotations are **hints**.
/// They are not guaranteed to provide a faithful description of
/// tool behavior (including descriptive properties like `title`).
///
/// Clients should never make tool use decisions based on ToolAnnotations
/// received from untrusted servers.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
pub struct ToolAnnotations {
    /// A human-readable title for the tool.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,

    /// If true, the tool does not modify its environment.
    ///
    /// Default: false
    #[serde(skip_serializing_if = "Option::is_none")]
    pub read_only_hint: Option<bool>,

    /// If true, the tool may perform destructive updates to its environment.
    /// If false, the tool performs only additive updates.
    ///
    /// (This property is meaningful only when `readOnlyHint == false`)
    ///
    /// Default: true
    /// A human-readable description of the tool's purpose.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub destructive_hint: Option<bool>,

    /// If true, calling the tool repeatedly with the same arguments
    /// will have no additional effect on the its environment.
    ///
    /// (This property is meaningful only when `readOnlyHint == false`)
    ///
    /// Default: false.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub idempotent_hint: Option<bool>,

    /// If true, this tool may interact with an "open world" of external
    /// entities. If false, the tool's domain of interaction is closed.
    /// For example, the world of a web search tool is open, whereas that
    /// of a memory tool is not.
    ///
    /// Default: true
    #[serde(skip_serializing_if = "Option::is_none")]
    pub open_world_hint: Option<bool>,
}

impl ToolAnnotations {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn with_title<T>(title: T) -> Self
    where
        T: Into<String>,
    {
        ToolAnnotations {
            title: Some(title.into()),
            ..Self::default()
        }
    }
    pub fn read_only(self, read_only: bool) -> Self {
        ToolAnnotations {
            read_only_hint: Some(read_only),
            ..self
        }
    }
    pub fn destructive(self, destructive: bool) -> Self {
        ToolAnnotations {
            destructive_hint: Some(destructive),
            ..self
        }
    }
    pub fn idempotent(self, idempotent: bool) -> Self {
        ToolAnnotations {
            idempotent_hint: Some(idempotent),
            ..self
        }
    }
    pub fn open_world(self, open_world: bool) -> Self {
        ToolAnnotations {
            open_world_hint: Some(open_world),
            ..self
        }
    }

    /// If not set, defaults to true.
    pub fn is_destructive(&self) -> bool {
        self.destructive_hint.unwrap_or(true)
    }

    /// If not set, defaults to false.
    pub fn is_idempotent(&self) -> bool {
        self.idempotent_hint.unwrap_or(false)
    }
}

impl Tool {
    /// Create a new tool with the given name and description
    pub fn new<N, D, S>(name: N, description: D, input_schema: S) -> Self
    where
        N: Into<Cow<'static, str>>,
        D: Into<Cow<'static, str>>,
        S: Into<Arc<JsonObject>>,
    {
        Tool {
            name: name.into(),
            description: Some(description.into()),
            input_schema: input_schema.into(),
            annotations: None,
        }
    }

    pub fn annotate(self, annotations: ToolAnnotations) -> Self {
        Tool {
            annotations: Some(annotations),
            ..self
        }
    }

    /// Get the schema as json value
    pub fn schema_as_json_value(&self) -> Value {
        Value::Object(self.input_schema.as_ref().clone())
    }
}

---
File: modelcontextprotocol-rust-sdk-13e72ec/crates/rmcp/src/service.rs
---

use futures::{FutureExt, future::BoxFuture};
use thiserror::Error;

use crate::{
    error::Error as McpError,
    model::{
        CancelledNotification, CancelledNotificationParam, Extensions, GetExtensions, GetMeta,
        JsonRpcBatchRequestItem, JsonRpcBatchResponseItem, JsonRpcError, JsonRpcMessage,
        JsonRpcNotification, JsonRpcRequest, JsonRpcResponse, Meta, NumberOrString, ProgressToken,
        RequestId, ServerJsonRpcMessage,
    },
    transport::{IntoTransport, Transport},
};
#[cfg(feature = "client")]
#[cfg_attr(docsrs, doc(cfg(feature = "client")))]
mod client;
#[cfg(feature = "client")]
#[cfg_attr(docsrs, doc(cfg(feature = "client")))]
pub use client::*;
#[cfg(feature = "server")]
#[cfg_attr(docsrs, doc(cfg(feature = "server")))]
mod server;
#[cfg(feature = "server")]
#[cfg_attr(docsrs, doc(cfg(feature = "server")))]
pub use server::*;
#[cfg(feature = "tower")]
#[cfg_attr(docsrs, doc(cfg(feature = "tower")))]
mod tower;
use tokio_util::sync::{CancellationToken, DropGuard};
#[cfg(feature = "tower")]
#[cfg_attr(docsrs, doc(cfg(feature = "tower")))]
pub use tower::*;
use tracing::instrument;
#[derive(Error, Debug)]
#[non_exhaustive]
pub enum ServiceError {
    #[error("Mcp error: {0}")]
    McpError(McpError),
    #[error("Transport send error: {0}")]
    TransportSend(Box<dyn std::error::Error + Send + Sync>),
    #[error("Transport closed")]
    TransportClosed,
    #[error("Unexpected response type")]
    UnexpectedResponse,
    #[error("task cancelled for reason {}", reason.as_deref().unwrap_or("<unknown>"))]
    Cancelled { reason: Option<String> },
    #[error("request timeout after {}", chrono::Duration::from_std(*timeout).unwrap_or_default())]
    Timeout { timeout: Duration },
}

impl ServiceError {}
trait TransferObject:
    std::fmt::Debug + Clone + serde::Serialize + serde::de::DeserializeOwned + Send + Sync + 'static
{
}

impl<T> TransferObject for T where
    T: std::fmt::Debug
        + serde::Serialize
        + serde::de::DeserializeOwned
        + Send
        + Sync
        + 'static
        + Clone
{
}

#[allow(private_bounds, reason = "there's no the third implementation")]
pub trait ServiceRole: std::fmt::Debug + Send + Sync + 'static + Copy + Clone {
    type Req: TransferObject + GetMeta + GetExtensions;
    type Resp: TransferObject;
    type Not: TryInto<CancelledNotification, Error = Self::Not>
        + From<CancelledNotification>
        + TransferObject;
    type PeerReq: TransferObject + GetMeta + GetExtensions;
    type PeerResp: TransferObject;
    type PeerNot: TryInto<CancelledNotification, Error = Self::PeerNot>
        + From<CancelledNotification>
        + TransferObject
        + GetMeta
        + GetExtensions;
    type InitializeError<E>;
    const IS_CLIENT: bool;
    type Info: TransferObject;
    type PeerInfo: TransferObject;
}

pub type TxJsonRpcMessage<R> =
    JsonRpcMessage<<R as ServiceRole>::Req, <R as ServiceRole>::Resp, <R as ServiceRole>::Not>;
pub type RxJsonRpcMessage<R> = JsonRpcMessage<
    <R as ServiceRole>::PeerReq,
    <R as ServiceRole>::PeerResp,
    <R as ServiceRole>::PeerNot,
>;

pub trait Service<R: ServiceRole>: Send + Sync + 'static {
    fn handle_request(
        &self,
        request: R::PeerReq,
        context: RequestContext<R>,
    ) -> impl Future<Output = Result<R::Resp, McpError>> + Send + '_;
    fn handle_notification(
        &self,
        notification: R::PeerNot,
        context: NotificationContext<R>,
    ) -> impl Future<Output = Result<(), McpError>> + Send + '_;
    fn get_info(&self) -> R::Info;
}

pub trait ServiceExt<R: ServiceRole>: Service<R> + Sized {
    /// Convert this service to a dynamic boxed service
    ///
    /// This could be very helpful when you want to store the services in a collection
    fn into_dyn(self) -> Box<dyn DynService<R>> {
        Box::new(self)
    }
    fn serve<T, E, A>(
        self,
        transport: T,
    ) -> impl Future<Output = Result<RunningService<R, Self>, R::InitializeError<E>>> + Send
    where
        T: IntoTransport<R, E, A>,
        E: std::error::Error + From<std::io::Error> + Send + Sync + 'static,
        Self: Sized,
    {
        Self::serve_with_ct(self, transport, Default::default())
    }
    fn serve_with_ct<T, E, A>(
        self,
        transport: T,
        ct: CancellationToken,
    ) -> impl Future<Output = Result<RunningService<R, Self>, R::InitializeError<E>>> + Send
    where
        T: IntoTransport<R, E, A>,
        E: std::error::Error + From<std::io::Error> + Send + Sync + 'static,
        Self: Sized;
}

impl<R: ServiceRole> Service<R> for Box<dyn DynService<R>> {
    fn handle_request(
        &self,
        request: R::PeerReq,
        context: RequestContext<R>,
    ) -> impl Future<Output = Result<R::Resp, McpError>> + Send + '_ {
        DynService::handle_request(self.as_ref(), request, context)
    }

    fn handle_notification(
        &self,
        notification: R::PeerNot,
        context: NotificationContext<R>,
    ) -> impl Future<Output = Result<(), McpError>> + Send + '_ {
        DynService::handle_notification(self.as_ref(), notification, context)
    }

    fn get_info(&self) -> R::Info {
        DynService::get_info(self.as_ref())
    }
}

pub trait DynService<R: ServiceRole>: Send + Sync {
    fn handle_request(
        &self,
        request: R::PeerReq,
        context: RequestContext<R>,
    ) -> BoxFuture<Result<R::Resp, McpError>>;
    fn handle_notification(
        &self,
        notification: R::PeerNot,
        context: NotificationContext<R>,
    ) -> BoxFuture<Result<(), McpError>>;
    fn get_info(&self) -> R::Info;
}

impl<R: ServiceRole, S: Service<R>> DynService<R> for S {
    fn handle_request(
        &self,
        request: R::PeerReq,
        context: RequestContext<R>,
    ) -> BoxFuture<Result<R::Resp, McpError>> {
        Box::pin(self.handle_request(request, context))
    }
    fn handle_notification(
        &self,
        notification: R::PeerNot,
        context: NotificationContext<R>,
    ) -> BoxFuture<Result<(), McpError>> {
        Box::pin(self.handle_notification(notification, context))
    }
    fn get_info(&self) -> R::Info {
        self.get_info()
    }
}

use std::{
    collections::{HashMap, VecDeque},
    ops::Deref,
    sync::{Arc, atomic::AtomicU32},
    time::Duration,
};

use tokio::sync::mpsc;

pub trait RequestIdProvider: Send + Sync + 'static {
    fn next_request_id(&self) -> RequestId;
}

pub trait ProgressTokenProvider: Send + Sync + 'static {
    fn next_progress_token(&self) -> ProgressToken;
}

pub type AtomicU32RequestIdProvider = AtomicU32Provider;
pub type AtomicU32ProgressTokenProvider = AtomicU32Provider;

#[derive(Debug, Default)]
pub struct AtomicU32Provider {
    id: AtomicU32,
}

impl RequestIdProvider for AtomicU32Provider {
    fn next_request_id(&self) -> RequestId {
        RequestId::Number(self.id.fetch_add(1, std::sync::atomic::Ordering::SeqCst))
    }
}

impl ProgressTokenProvider for AtomicU32Provider {
    fn next_progress_token(&self) -> ProgressToken {
        ProgressToken(NumberOrString::Number(
            self.id.fetch_add(1, std::sync::atomic::Ordering::SeqCst),
        ))
    }
}

type Responder<T> = tokio::sync::oneshot::Sender<T>;

/// A handle to a remote request
///
/// You can cancel it by call [`RequestHandle::cancel`] with a reason,
///
/// or wait for response by call [`RequestHandle::await_response`]
#[derive(Debug)]
pub struct RequestHandle<R: ServiceRole> {
    pub rx: tokio::sync::oneshot::Receiver<Result<R::PeerResp, ServiceError>>,
    pub options: PeerRequestOptions,
    pub peer: Peer<R>,
    pub id: RequestId,
    pub progress_token: ProgressToken,
}

impl<R: ServiceRole> RequestHandle<R> {
    pub const REQUEST_TIMEOUT_REASON: &str = "request timeout";
    pub async fn await_response(self) -> Result<R::PeerResp, ServiceError> {
        if let Some(timeout) = self.options.timeout {
            let timeout_result = tokio::time::timeout(timeout, async move {
                self.rx.await.map_err(|_e| ServiceError::TransportClosed)?
            })
            .await;
            match timeout_result {
                Ok(response) => response,
                Err(_) => {
                    let error = Err(ServiceError::Timeout { timeout });
                    // cancel this request
                    let notification = CancelledNotification {
                        params: CancelledNotificationParam {
                            request_id: self.id,
                            reason: Some(Self::REQUEST_TIMEOUT_REASON.to_owned()),
                        },
                        method: crate::model::CancelledNotificationMethod,
                        extensions: Default::default(),
                    };
                    let _ = self.peer.send_notification(notification.into()).await;
                    error
                }
            }
        } else {
            self.rx.await.map_err(|_e| ServiceError::TransportClosed)?
        }
    }

    /// Cancel this request
    pub async fn cancel(self, reason: Option<String>) -> Result<(), ServiceError> {
        let notification = CancelledNotification {
            params: CancelledNotificationParam {
                request_id: self.id,
                reason,
            },
            method: crate::model::CancelledNotificationMethod,
            extensions: Default::default(),
        };
        self.peer.send_notification(notification.into()).await?;
        Ok(())
    }
}

#[derive(Debug)]
pub(crate) enum PeerSinkMessage<R: ServiceRole> {
    Request {
        request: R::Req,
        id: RequestId,
        responder: Responder<Result<R::PeerResp, ServiceError>>,
    },
    Notification {
        notification: R::Not,
        responder: Responder<Result<(), ServiceError>>,
    },
}

/// An interface to fetch the remote client or server
///
/// For general purpose, call [`Peer::send_request`] or [`Peer::send_notification`] to send message to remote peer.
///
/// To create a cancellable request, call [`Peer::send_request_with_option`].
#[derive(Clone)]
pub struct Peer<R: ServiceRole> {
    tx: mpsc::Sender<PeerSinkMessage<R>>,
    request_id_provider: Arc<dyn RequestIdProvider>,
    progress_token_provider: Arc<dyn ProgressTokenProvider>,
    info: Arc<tokio::sync::OnceCell<R::PeerInfo>>,
}

impl<R: ServiceRole> std::fmt::Debug for Peer<R> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PeerSink")
            .field("tx", &self.tx)
            .field("is_client", &R::IS_CLIENT)
            .finish()
    }
}

type ProxyOutbound<R> = mpsc::Receiver<PeerSinkMessage<R>>;

#[derive(Debug, Default)]
pub struct PeerRequestOptions {
    pub timeout: Option<Duration>,
    pub meta: Option<Meta>,
}

impl PeerRequestOptions {
    pub fn no_options() -> Self {
        Self::default()
    }
}

impl<R: ServiceRole> Peer<R> {
    const CLIENT_CHANNEL_BUFFER_SIZE: usize = 1024;
    pub(crate) fn new(
        request_id_provider: Arc<dyn RequestIdProvider>,
        peer_info: Option<R::PeerInfo>,
    ) -> (Peer<R>, ProxyOutbound<R>) {
        let (tx, rx) = mpsc::channel(Self::CLIENT_CHANNEL_BUFFER_SIZE);
        (
            Self {
                tx,
                request_id_provider,
                progress_token_provider: Arc::new(AtomicU32ProgressTokenProvider::default()),
                info: Arc::new(tokio::sync::OnceCell::new_with(peer_info)),
            },
            rx,
        )
    }
    pub async fn send_notification(&self, notification: R::Not) -> Result<(), ServiceError> {
        let (responder, receiver) = tokio::sync::oneshot::channel();
        self.tx
            .send(PeerSinkMessage::Notification {
                notification,
                responder,
            })
            .await
            .map_err(|_m| ServiceError::TransportClosed)?;
        receiver.await.map_err(|_e| ServiceError::TransportClosed)?
    }
    pub async fn send_request(&self, request: R::Req) -> Result<R::PeerResp, ServiceError> {
        self.send_request_with_option(request, PeerRequestOptions::no_options())
            .await?
            .await_response()
            .await
    }

    pub async fn send_cancellable_request(
        &self,
        request: R::Req,
        options: PeerRequestOptions,
    ) -> Result<RequestHandle<R>, ServiceError> {
        self.send_request_with_option(request, options).await
    }

    pub async fn send_request_with_option(
        &self,
        mut request: R::Req,
        options: PeerRequestOptions,
    ) -> Result<RequestHandle<R>, ServiceError> {
        let id = self.request_id_provider.next_request_id();
        let progress_token = self.progress_token_provider.next_progress_token();
        request
            .get_meta_mut()
            .set_progress_token(progress_token.clone());
        if let Some(meta) = options.meta.clone() {
            request.get_meta_mut().extend(meta);
        }
        let (responder, receiver) = tokio::sync::oneshot::channel();
        self.tx
            .send(PeerSinkMessage::Request {
                request,
                id: id.clone(),
                responder,
            })
            .await
            .map_err(|_m| ServiceError::TransportClosed)?;
        Ok(RequestHandle {
            id,
            rx: receiver,
            progress_token,
            options,
            peer: self.clone(),
        })
    }
    pub fn peer_info(&self) -> Option<&R::PeerInfo> {
        self.info.get()
    }

    pub fn set_peer_info(&self, info: R::PeerInfo) {
        if self.info.initialized() {
            tracing::warn!("trying to set peer info, which is already initialized");
        } else {
            let _ = self.info.set(info);
        }
    }

    pub fn is_transport_closed(&self) -> bool {
        self.tx.is_closed()
    }
}

#[derive(Debug)]
pub struct RunningService<R: ServiceRole, S: Service<R>> {
    service: Arc<S>,
    peer: Peer<R>,
    handle: tokio::task::JoinHandle<QuitReason>,
    cancellation_token: CancellationToken,
    dg: DropGuard,
}
impl<R: ServiceRole, S: Service<R>> Deref for RunningService<R, S> {
    type Target = Peer<R>;

    fn deref(&self) -> &Self::Target {
        &self.peer
    }
}

impl<R: ServiceRole, S: Service<R>> RunningService<R, S> {
    #[inline]
    pub fn peer(&self) -> &Peer<R> {
        &self.peer
    }
    #[inline]
    pub fn service(&self) -> &S {
        self.service.as_ref()
    }
    #[inline]
    pub fn cancellation_token(&self) -> RunningServiceCancellationToken {
        RunningServiceCancellationToken(self.cancellation_token.clone())
    }
    #[inline]
    pub async fn waiting(self) -> Result<QuitReason, tokio::task::JoinError> {
        self.handle.await
    }
    pub async fn cancel(self) -> Result<QuitReason, tokio::task::JoinError> {
        let RunningService { dg, handle, .. } = self;
        dg.disarm().cancel();
        handle.await
    }
}

// use a wrapper type so we can tweak the implementation if needed
pub struct RunningServiceCancellationToken(CancellationToken);

impl RunningServiceCancellationToken {
    pub fn cancel(self) {
        self.0.cancel();
    }
}

#[derive(Debug)]
pub enum QuitReason {
    Cancelled,
    Closed,
    JoinError(tokio::task::JoinError),
}

/// Request execution context
#[derive(Debug, Clone)]
pub struct RequestContext<R: ServiceRole> {
    /// this token will be cancelled when the [`CancelledNotification`] is received.
    pub ct: CancellationToken,
    pub id: RequestId,
    pub meta: Meta,
    pub extensions: Extensions,
    /// An interface to fetch the remote client or server
    pub peer: Peer<R>,
}

/// Request execution context
#[derive(Debug, Clone)]
pub struct NotificationContext<R: ServiceRole> {
    pub meta: Meta,
    pub extensions: Extensions,
    /// An interface to fetch the remote client or server
    pub peer: Peer<R>,
}

/// Use this function to skip initialization process
pub fn serve_directly<R, S, T, E, A>(
    service: S,
    transport: T,
    peer_info: Option<R::PeerInfo>,
) -> RunningService<R, S>
where
    R: ServiceRole,
    S: Service<R>,
    T: IntoTransport<R, E, A>,
    E: std::error::Error + Send + Sync + 'static,
{
    serve_directly_with_ct(service, transport, peer_info, Default::default())
}

/// Use this function to skip initialization process
pub fn serve_directly_with_ct<R, S, T, E, A>(
    service: S,
    transport: T,
    peer_info: Option<R::PeerInfo>,
    ct: CancellationToken,
) -> RunningService<R, S>
where
    R: ServiceRole,
    S: Service<R>,
    T: IntoTransport<R, E, A>,
    E: std::error::Error + Send + Sync + 'static,
{
    let (peer, peer_rx) = Peer::new(Arc::new(AtomicU32RequestIdProvider::default()), peer_info);
    serve_inner(service, transport, peer, peer_rx, ct)
}

#[instrument(skip_all)]
fn serve_inner<R, S, T, E, A>(
    service: S,
    transport: T,
    peer: Peer<R>,
    mut peer_rx: tokio::sync::mpsc::Receiver<PeerSinkMessage<R>>,
    ct: CancellationToken,
) -> RunningService<R, S>
where
    R: ServiceRole,
    S: Service<R>,
    T: IntoTransport<R, E, A>,
    E: std::error::Error + Send + Sync + 'static,
{
    const SINK_PROXY_BUFFER_SIZE: usize = 64;
    let (sink_proxy_tx, mut sink_proxy_rx) =
        tokio::sync::mpsc::channel::<TxJsonRpcMessage<R>>(SINK_PROXY_BUFFER_SIZE);
    let peer_info = peer.peer_info();
    if R::IS_CLIENT {
        tracing::info!(?peer_info, "Service initialized as client");
    } else {
        tracing::info!(?peer_info, "Service initialized as server");
    }

    let mut local_responder_pool =
        HashMap::<RequestId, Responder<Result<R::PeerResp, ServiceError>>>::new();
    let mut local_ct_pool = HashMap::<RequestId, CancellationToken>::new();
    let shared_service = Arc::new(service);
    // for return
    let service = shared_service.clone();

    // let message_sink = tokio::sync::
    // let mut stream = std::pin::pin!(stream);
    let serve_loop_ct = ct.child_token();
    let peer_return: Peer<R> = peer.clone();
    let handle = tokio::spawn(async move {
        let mut transport = transport.into_transport();
        let mut batch_messages = VecDeque::<RxJsonRpcMessage<R>>::new();
        let mut send_task_set = tokio::task::JoinSet::<SendTaskResult<E>>::new();
        #[derive(Debug)]
        enum SendTaskResult<E> {
            Request {
                id: RequestId,
                result: Result<(), E>,
            },
            Notification {
                responder: Responder<Result<(), ServiceError>>,
                cancellation_param: Option<CancelledNotificationParam>,
                result: Result<(), E>,
            },
        }
        #[derive(Debug)]
        enum Event<R: ServiceRole, E> {
            ProxyMessage(PeerSinkMessage<R>),
            PeerMessage(RxJsonRpcMessage<R>),
            ToSink(TxJsonRpcMessage<R>),
            SendTaskResult(SendTaskResult<E>),
        }

        let quit_reason = loop {
            let evt = if let Some(m) = batch_messages.pop_front() {
                Event::PeerMessage(m)
            } else {
                tokio::select! {
                    m = sink_proxy_rx.recv(), if !sink_proxy_rx.is_closed() => {
                        if let Some(m) = m {
                            Event::ToSink(m)
                        } else {
                            continue
                        }
                    }
                    m = transport.receive() => {
                        if let Some(m) = m {
                            Event::PeerMessage(m)
                        } else {
                            // input stream closed
                            tracing::info!("input stream terminated");
                            break QuitReason::Closed
                        }
                    }
                    m = peer_rx.recv(), if !peer_rx.is_closed() => {
                        if let Some(m) = m {
                            Event::ProxyMessage(m)
                        } else {
                            continue
                        }
                    }
                    m = send_task_set.join_next(), if !send_task_set.is_empty() => {
                        let Some(result) = m else {
                            continue
                        };
                        match result {
                            Err(e) => {
                                // join error, which is serious, we should quit.
                                tracing::error!(%e, "send request task encounter a tokio join error");
                                break QuitReason::JoinError(e)
                            }
                            Ok(result) => {
                                Event::SendTaskResult(result)
                            }
                        }
                    }
                    _ = serve_loop_ct.cancelled() => {
                        tracing::info!("task cancelled");
                        break QuitReason::Cancelled
                    }
                }
            };

            tracing::trace!(?evt, "new event");
            match evt {
                Event::SendTaskResult(SendTaskResult::Request { id, result }) => {
                    if let Err(e) = result {
                        if let Some(responder) = local_responder_pool.remove(&id) {
                            let _ = responder.send(Err(ServiceError::TransportSend(Box::new(e))));
                        }
                    }
                }
                Event::SendTaskResult(SendTaskResult::Notification {
                    responder,
                    result,
                    cancellation_param,
                }) => {
                    let response = if let Err(e) = result {
                        Err(ServiceError::TransportSend(Box::new(e)))
                    } else {
                        Ok(())
                    };
                    let _ = responder.send(response);
                    if let Some(param) = cancellation_param {
                        if let Some(responder) = local_responder_pool.remove(&param.request_id) {
                            tracing::info!(id = %param.request_id, reason = param.reason, "cancelled");
                            let _response_result = responder.send(Err(ServiceError::Cancelled {
                                reason: param.reason.clone(),
                            }));
                        }
                    }
                }
                // response and error
                Event::ToSink(m) => {
                    if let Some(id) = match &m {
                        JsonRpcMessage::Response(response) => Some(&response.id),
                        JsonRpcMessage::Error(error) => Some(&error.id),
                        _ => None,
                    } {
                        if let Some(ct) = local_ct_pool.remove(id) {
                            ct.cancel();
                        }
                        let send = transport.send(m);
                        tokio::spawn(async move {
                            let send_result = send.await;
                            if let Err(error) = send_result {
                                tracing::error!(%error, "fail to response message");
                            }
                        });
                    }
                }
                Event::ProxyMessage(PeerSinkMessage::Request {
                    request,
                    id,
                    responder,
                }) => {
                    local_responder_pool.insert(id.clone(), responder);
                    let send = transport.send(JsonRpcMessage::request(request, id.clone()));
                    {
                        let id = id.clone();
                        send_task_set
                            .spawn(send.map(move |r| SendTaskResult::Request { id, result: r }));
                    }
                }
                Event::ProxyMessage(PeerSinkMessage::Notification {
                    notification,
                    responder,
                }) => {
                    // catch cancellation notification
                    let mut cancellation_param = None;
                    let notification = match notification.try_into() {
                        Ok::<CancelledNotification, _>(cancelled) => {
                            cancellation_param.replace(cancelled.params.clone());
                            cancelled.into()
                        }
                        Err(notification) => notification,
                    };
                    let send = transport.send(JsonRpcMessage::notification(notification));
                    send_task_set.spawn(send.map(move |result| SendTaskResult::Notification {
                        responder,
                        cancellation_param,
                        result,
                    }));
                }
                Event::PeerMessage(JsonRpcMessage::Request(JsonRpcRequest {
                    id,
                    mut request,
                    ..
                })) => {
                    tracing::debug!(%id, ?request, "received request");
                    {
                        let service = shared_service.clone();
                        let sink = sink_proxy_tx.clone();
                        let request_ct = serve_loop_ct.child_token();
                        let context_ct = request_ct.child_token();
                        local_ct_pool.insert(id.clone(), request_ct);
                        let mut extensions = Extensions::new();
                        let mut meta = Meta::new();
                        // avoid clone
                        std::mem::swap(&mut extensions, request.extensions_mut());
                        std::mem::swap(&mut meta, request.get_meta_mut());
                        let context = RequestContext {
                            ct: context_ct,
                            id: id.clone(),
                            peer: peer.clone(),
                            meta,
                            extensions,
                        };
                        tokio::spawn(async move {
                            let result = service.handle_request(request, context).await;
                            let response = match result {
                                Ok(result) => {
                                    tracing::debug!(%id, ?result, "response message");
                                    JsonRpcMessage::response(result, id)
                                }
                                Err(error) => {
                                    tracing::warn!(%id, ?error, "response error");
                                    JsonRpcMessage::error(error, id)
                                }
                            };
                            let _send_result = sink.send(response).await;
                        });
                    }
                }
                Event::PeerMessage(JsonRpcMessage::Notification(JsonRpcNotification {
                    notification,
                    ..
                })) => {
                    tracing::info!(?notification, "received notification");
                    // catch cancelled notification
                    let mut notification = match notification.try_into() {
                        Ok::<CancelledNotification, _>(cancelled) => {
                            if let Some(ct) = local_ct_pool.remove(&cancelled.params.request_id) {
                                tracing::info!(id = %cancelled.params.request_id, reason = cancelled.params.reason, "cancelled");
                                ct.cancel();
                            }
                            cancelled.into()
                        }
                        Err(notification) => notification,
                    };
                    {
                        let service = shared_service.clone();
                        let mut extensions = Extensions::new();
                        let mut meta = Meta::new();
                        // avoid clone
                        std::mem::swap(&mut extensions, notification.extensions_mut());
                        std::mem::swap(&mut meta, notification.get_meta_mut());
                        let context = NotificationContext {
                            peer: peer.clone(),
                            meta,
                            extensions,
                        };
                        tokio::spawn(async move {
                            let result = service.handle_notification(notification, context).await;
                            if let Err(error) = result {
                                tracing::warn!(%error, "Error sending notification");
                            }
                        });
                    }
                }
                Event::PeerMessage(JsonRpcMessage::Response(JsonRpcResponse {
                    result,
                    id,
                    ..
                })) => {
                    if let Some(responder) = local_responder_pool.remove(&id) {
                        let response_result = responder.send(Ok(result));
                        if let Err(_error) = response_result {
                            tracing::warn!(%id, "Error sending response");
                        }
                    }
                }
                Event::PeerMessage(JsonRpcMessage::Error(JsonRpcError { error, id, .. })) => {
                    if let Some(responder) = local_responder_pool.remove(&id) {
                        let _response_result = responder.send(Err(ServiceError::McpError(error)));
                        if let Err(_error) = _response_result {
                            tracing::warn!(%id, "Error sending response");
                        }
                    }
                }
                Event::PeerMessage(JsonRpcMessage::BatchRequest(batch)) => {
                    batch_messages.extend(
                        batch
                            .into_iter()
                            .map(JsonRpcBatchRequestItem::into_non_batch_message),
                    );
                }
                Event::PeerMessage(JsonRpcMessage::BatchResponse(batch)) => {
                    batch_messages.extend(
                        batch
                            .into_iter()
                            .map(JsonRpcBatchResponseItem::into_non_batch_message),
                    );
                }
            }
        };
        let sink_close_result = transport.close().await;
        if let Err(e) = sink_close_result {
            tracing::error!(%e, "fail to close sink");
        }
        tracing::info!(?quit_reason, "serve finished");
        quit_reason
    });
    RunningService {
        service,
        peer: peer_return,
        handle,
        cancellation_token: ct.clone(),
        dg: ct.drop_guard(),
    }
}

---
File: modelcontextprotocol-rust-sdk-13e72ec/crates/rmcp/src/service/client.rs
---

use std::borrow::Cow;

use thiserror::Error;

use super::*;
use crate::model::{
    CallToolRequest, CallToolRequestParam, CallToolResult, CancelledNotification,
    CancelledNotificationParam, ClientInfo, ClientJsonRpcMessage, ClientNotification,
    ClientRequest, ClientResult, CompleteRequest, CompleteRequestParam, CompleteResult,
    GetPromptRequest, GetPromptRequestParam, GetPromptResult, InitializeRequest,
    InitializedNotification, JsonRpcResponse, ListPromptsRequest, ListPromptsResult,
    ListResourceTemplatesRequest, ListResourceTemplatesResult, ListResourcesRequest,
    ListResourcesResult, ListToolsRequest, ListToolsResult, PaginatedRequestParam,
    ProgressNotification, ProgressNotificationParam, ReadResourceRequest, ReadResourceRequestParam,
    ReadResourceResult, RequestId, RootsListChangedNotification, ServerInfo, ServerJsonRpcMessage,
    ServerNotification, ServerRequest, ServerResult, SetLevelRequest, SetLevelRequestParam,
    SubscribeRequest, SubscribeRequestParam, UnsubscribeRequest, UnsubscribeRequestParam,
};

/// It represents the error that may occur when serving the client.
///
/// if you want to handle the error, you can use `serve_client_with_ct` or `serve_client` with `Result<RunningService<RoleClient, S>, ClientError>`
#[derive(Error, Debug)]
pub enum ClientInitializeError<E> {
    #[error("expect initialized response, but received: {0:?}")]
    ExpectedInitResponse(Option<ServerJsonRpcMessage>),

    #[error("expect initialized result, but received: {0:?}")]
    ExpectedInitResult(Option<ServerResult>),

    #[error("conflict initialized response id: expected {0}, got {1}")]
    ConflictInitResponseId(RequestId, RequestId),

    #[error("connection closed: {0}")]
    ConnectionClosed(String),

    #[error("Send message error {error}, when {context}")]
    TransportError {
        error: E,
        context: Cow<'static, str>,
    },
}

/// Helper function to get the next message from the stream
async fn expect_next_message<T, E>(
    transport: &mut T,
    context: &str,
) -> Result<ServerJsonRpcMessage, ClientInitializeError<E>>
where
    T: Transport<RoleClient>,
{
    transport
        .receive()
        .await
        .ok_or_else(|| ClientInitializeError::ConnectionClosed(context.to_string()))
}

/// Helper function to expect a response from the stream
async fn expect_response<T, E>(
    transport: &mut T,
    context: &str,
) -> Result<(ServerResult, RequestId), ClientInitializeError<E>>
where
    T: Transport<RoleClient>,
{
    let msg = expect_next_message(transport, context).await?;

    match msg {
        ServerJsonRpcMessage::Response(JsonRpcResponse { id, result, .. }) => Ok((result, id)),
        _ => Err(ClientInitializeError::ExpectedInitResponse(Some(msg))),
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct RoleClient;

impl ServiceRole for RoleClient {
    type Req = ClientRequest;
    type Resp = ClientResult;
    type Not = ClientNotification;
    type PeerReq = ServerRequest;
    type PeerResp = ServerResult;
    type PeerNot = ServerNotification;
    type Info = ClientInfo;
    type PeerInfo = ServerInfo;
    type InitializeError<E> = ClientInitializeError<E>;
    const IS_CLIENT: bool = true;
}

pub type ServerSink = Peer<RoleClient>;

impl<S: Service<RoleClient>> ServiceExt<RoleClient> for S {
    fn serve_with_ct<T, E, A>(
        self,
        transport: T,
        ct: CancellationToken,
    ) -> impl Future<Output = Result<RunningService<RoleClient, Self>, ClientInitializeError<E>>> + Send
    where
        T: IntoTransport<RoleClient, E, A>,
        E: std::error::Error + From<std::io::Error> + Send + Sync + 'static,
        Self: Sized,
    {
        serve_client_with_ct(self, transport, ct)
    }
}

pub async fn serve_client<S, T, E, A>(
    service: S,
    transport: T,
) -> Result<RunningService<RoleClient, S>, ClientInitializeError<E>>
where
    S: Service<RoleClient>,
    T: IntoTransport<RoleClient, E, A>,
    E: std::error::Error + Send + Sync + 'static,
{
    serve_client_with_ct(service, transport, Default::default()).await
}

pub async fn serve_client_with_ct<S, T, E, A>(
    service: S,
    transport: T,
    ct: CancellationToken,
) -> Result<RunningService<RoleClient, S>, ClientInitializeError<E>>
where
    S: Service<RoleClient>,
    T: IntoTransport<RoleClient, E, A>,
    E: std::error::Error + Send + Sync + 'static,
{
    let mut transport = transport.into_transport();
    let id_provider = <Arc<AtomicU32RequestIdProvider>>::default();

    // service
    let id = id_provider.next_request_id();
    let init_request = InitializeRequest {
        method: Default::default(),
        params: service.get_info(),
        extensions: Default::default(),
    };
    transport
        .send(ClientJsonRpcMessage::request(
            ClientRequest::InitializeRequest(init_request),
            id.clone(),
        ))
        .await
        .map_err(|error| ClientInitializeError::TransportError {
            error,
            context: "send initialize request".into(),
        })?;

    let (response, response_id) = expect_response(&mut transport, "initialize response").await?;

    if id != response_id {
        return Err(ClientInitializeError::ConflictInitResponseId(
            id,
            response_id,
        ));
    }

    let ServerResult::InitializeResult(initialize_result) = response else {
        return Err(ClientInitializeError::ExpectedInitResult(Some(response)));
    };

    // send notification
    let notification = ClientJsonRpcMessage::notification(
        ClientNotification::InitializedNotification(InitializedNotification {
            method: Default::default(),
            extensions: Default::default(),
        }),
    );
    transport
        .send(notification)
        .await
        .map_err(|error| ClientInitializeError::TransportError {
            error,
            context: "send initialized notification".into(),
        })?;
    let (peer, peer_rx) = Peer::new(id_provider, Some(initialize_result));
    Ok(serve_inner(service, transport, peer, peer_rx, ct))
}

macro_rules! method {
    (peer_req $method:ident $Req:ident() => $Resp: ident ) => {
        pub async fn $method(&self) -> Result<$Resp, ServiceError> {
            let result = self
                .send_request(ClientRequest::$Req($Req {
                    method: Default::default(),
                }))
                .await?;
            match result {
                ServerResult::$Resp(result) => Ok(result),
                _ => Err(ServiceError::UnexpectedResponse),
            }
        }
    };
    (peer_req $method:ident $Req:ident($Param: ident) => $Resp: ident ) => {
        pub async fn $method(&self, params: $Param) -> Result<$Resp, ServiceError> {
            let result = self
                .send_request(ClientRequest::$Req($Req {
                    method: Default::default(),
                    params,
                    extensions: Default::default(),
                }))
                .await?;
            match result {
                ServerResult::$Resp(result) => Ok(result),
                _ => Err(ServiceError::UnexpectedResponse),
            }
        }
    };
    (peer_req $method:ident $Req:ident($Param: ident)? => $Resp: ident ) => {
        pub async fn $method(&self, params: Option<$Param>) -> Result<$Resp, ServiceError> {
            let result = self
                .send_request(ClientRequest::$Req($Req {
                    method: Default::default(),
                    params,
                    extensions: Default::default(),
                }))
                .await?;
            match result {
                ServerResult::$Resp(result) => Ok(result),
                _ => Err(ServiceError::UnexpectedResponse),
            }
        }
    };
    (peer_req $method:ident $Req:ident($Param: ident)) => {
        pub async fn $method(&self, params: $Param) -> Result<(), ServiceError> {
            let result = self
                .send_request(ClientRequest::$Req($Req {
                    method: Default::default(),
                    params,
                    extensions: Default::default(),
                }))
                .await?;
            match result {
                ServerResult::EmptyResult(_) => Ok(()),
                _ => Err(ServiceError::UnexpectedResponse),
            }
        }
    };

    (peer_not $method:ident $Not:ident($Param: ident)) => {
        pub async fn $method(&self, params: $Param) -> Result<(), ServiceError> {
            self.send_notification(ClientNotification::$Not($Not {
                method: Default::default(),
                params,
                extensions: Default::default(),
            }))
            .await?;
            Ok(())
        }
    };
    (peer_not $method:ident $Not:ident) => {
        pub async fn $method(&self) -> Result<(), ServiceError> {
            self.send_notification(ClientNotification::$Not($Not {
                method: Default::default(),
                extensions: Default::default(),
            }))
            .await?;
            Ok(())
        }
    };
}

impl Peer<RoleClient> {
    method!(peer_req complete CompleteRequest(CompleteRequestParam) => CompleteResult);
    method!(peer_req set_level SetLevelRequest(SetLevelRequestParam));
    method!(peer_req get_prompt GetPromptRequest(GetPromptRequestParam) => GetPromptResult);
    method!(peer_req list_prompts ListPromptsRequest(PaginatedRequestParam)? => ListPromptsResult);
    method!(peer_req list_resources ListResourcesRequest(PaginatedRequestParam)? => ListResourcesResult);
    method!(peer_req list_resource_templates ListResourceTemplatesRequest(PaginatedRequestParam)? => ListResourceTemplatesResult);
    method!(peer_req read_resource ReadResourceRequest(ReadResourceRequestParam) => ReadResourceResult);
    method!(peer_req subscribe SubscribeRequest(SubscribeRequestParam) );
    method!(peer_req unsubscribe UnsubscribeRequest(UnsubscribeRequestParam));
    method!(peer_req call_tool CallToolRequest(CallToolRequestParam) => CallToolResult);
    method!(peer_req list_tools ListToolsRequest(PaginatedRequestParam)? => ListToolsResult);

    method!(peer_not notify_cancelled CancelledNotification(CancelledNotificationParam));
    method!(peer_not notify_progress ProgressNotification(ProgressNotificationParam));
    method!(peer_not notify_initialized InitializedNotification);
    method!(peer_not notify_roots_list_changed RootsListChangedNotification);
}

impl Peer<RoleClient> {
    /// A wrapper method for [`Peer<RoleClient>::list_tools`].
    ///
    /// This function will call [`Peer<RoleClient>::list_tools`] multiple times until all tools are listed.
    pub async fn list_all_tools(&self) -> Result<Vec<crate::model::Tool>, ServiceError> {
        let mut tools = Vec::new();
        let mut cursor = None;
        loop {
            let result = self
                .list_tools(Some(PaginatedRequestParam { cursor }))
                .await?;
            tools.extend(result.tools);
            cursor = result.next_cursor;
            if cursor.is_none() {
                break;
            }
        }
        Ok(tools)
    }

    /// A wrapper method for [`Peer<RoleClient>::list_prompts`].
    ///
    /// This function will call [`Peer<RoleClient>::list_prompts`] multiple times until all prompts are listed.
    pub async fn list_all_prompts(&self) -> Result<Vec<crate::model::Prompt>, ServiceError> {
        let mut prompts = Vec::new();
        let mut cursor = None;
        loop {
            let result = self
                .list_prompts(Some(PaginatedRequestParam { cursor }))
                .await?;
            prompts.extend(result.prompts);
            cursor = result.next_cursor;
            if cursor.is_none() {
                break;
            }
        }
        Ok(prompts)
    }

    /// A wrapper method for [`Peer<RoleClient>::list_resources`].
    ///
    /// This function will call [`Peer<RoleClient>::list_resources`] multiple times until all resources are listed.
    pub async fn list_all_resources(&self) -> Result<Vec<crate::model::Resource>, ServiceError> {
        let mut resources = Vec::new();
        let mut cursor = None;
        loop {
            let result = self
                .list_resources(Some(PaginatedRequestParam { cursor }))
                .await?;
            resources.extend(result.resources);
            cursor = result.next_cursor;
            if cursor.is_none() {
                break;
            }
        }
        Ok(resources)
    }

    /// A wrapper method for [`Peer<RoleClient>::list_resource_templates`].
    ///
    /// This function will call [`Peer<RoleClient>::list_resource_templates`] multiple times until all resource templates are listed.
    pub async fn list_all_resource_templates(
        &self,
    ) -> Result<Vec<crate::model::ResourceTemplate>, ServiceError> {
        let mut resource_templates = Vec::new();
        let mut cursor = None;
        loop {
            let result = self
                .list_resource_templates(Some(PaginatedRequestParam { cursor }))
                .await?;
            resource_templates.extend(result.resource_templates);
            cursor = result.next_cursor;
            if cursor.is_none() {
                break;
            }
        }
        Ok(resource_templates)
    }
}

---
File: modelcontextprotocol-rust-sdk-13e72ec/crates/rmcp/src/service/server.rs
---

use std::borrow::Cow;

use thiserror::Error;

use super::*;
use crate::model::{
    CancelledNotification, CancelledNotificationParam, ClientInfo, ClientJsonRpcMessage,
    ClientNotification, ClientRequest, ClientResult, CreateMessageRequest,
    CreateMessageRequestParam, CreateMessageResult, ErrorData, ListRootsRequest, ListRootsResult,
    LoggingMessageNotification, LoggingMessageNotificationParam, ProgressNotification,
    ProgressNotificationParam, PromptListChangedNotification, ProtocolVersion,
    ResourceListChangedNotification, ResourceUpdatedNotification, ResourceUpdatedNotificationParam,
    ServerInfo, ServerNotification, ServerRequest, ServerResult, ToolListChangedNotification,
};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct RoleServer;

impl ServiceRole for RoleServer {
    type Req = ServerRequest;
    type Resp = ServerResult;
    type Not = ServerNotification;
    type PeerReq = ClientRequest;
    type PeerResp = ClientResult;
    type PeerNot = ClientNotification;
    type Info = ServerInfo;
    type PeerInfo = ClientInfo;

    type InitializeError<E> = ServerInitializeError<E>;
    const IS_CLIENT: bool = false;
}

/// It represents the error that may occur when serving the server.
///
/// if you want to handle the error, you can use `serve_server_with_ct` or `serve_server` with `Result<RunningService<RoleServer, S>, ServerError>`
#[derive(Error, Debug)]
pub enum ServerInitializeError<E> {
    #[error("expect initialized request, but received: {0:?}")]
    ExpectedInitializeRequest(Option<ClientJsonRpcMessage>),

    #[error("expect initialized notification, but received: {0:?}")]
    ExpectedInitializedNotification(Option<ClientJsonRpcMessage>),

    #[error("connection closed: {0}")]
    ConnectionClosed(String),

    #[error("unexpected initialize result: {0:?}")]
    UnexpectedInitializeResponse(ServerResult),

    #[error("initialize failed: {0}")]
    InitializeFailed(ErrorData),

    #[error("unsupported protocol version: {0}")]
    UnsupportedProtocolVersion(ProtocolVersion),

    #[error("Send message error {error}, when {context}")]
    TransportError {
        error: E,
        context: Cow<'static, str>,
    },
}

pub type ClientSink = Peer<RoleServer>;

impl<S: Service<RoleServer>> ServiceExt<RoleServer> for S {
    fn serve_with_ct<T, E, A>(
        self,
        transport: T,
        ct: CancellationToken,
    ) -> impl Future<Output = Result<RunningService<RoleServer, Self>, ServerInitializeError<E>>> + Send
    where
        T: IntoTransport<RoleServer, E, A>,
        E: std::error::Error + Send + Sync + 'static,
        Self: Sized,
    {
        serve_server_with_ct(self, transport, ct)
    }
}

pub async fn serve_server<S, T, E, A>(
    service: S,
    transport: T,
) -> Result<RunningService<RoleServer, S>, ServerInitializeError<E>>
where
    S: Service<RoleServer>,
    T: IntoTransport<RoleServer, E, A>,
    E: std::error::Error + Send + Sync + 'static,
{
    serve_server_with_ct(service, transport, CancellationToken::new()).await
}

/// Helper function to get the next message from the stream
async fn expect_next_message<T, E>(
    transport: &mut T,
    context: &str,
) -> Result<ClientJsonRpcMessage, ServerInitializeError<E>>
where
    T: Transport<RoleServer>,
{
    transport
        .receive()
        .await
        .ok_or_else(|| ServerInitializeError::ConnectionClosed(context.to_string()))
}

/// Helper function to expect a request from the stream
async fn expect_request<T, E>(
    transport: &mut T,
    context: &str,
) -> Result<(ClientRequest, RequestId), ServerInitializeError<E>>
where
    T: Transport<RoleServer>,
{
    let msg = expect_next_message(transport, context).await?;
    let msg_clone = msg.clone();
    msg.into_request()
        .ok_or(ServerInitializeError::ExpectedInitializeRequest(Some(
            msg_clone,
        )))
}

/// Helper function to expect a notification from the stream
async fn expect_notification<T, E>(
    transport: &mut T,
    context: &str,
) -> Result<ClientNotification, ServerInitializeError<E>>
where
    T: Transport<RoleServer>,
{
    let msg = expect_next_message(transport, context).await?;
    let msg_clone = msg.clone();
    msg.into_notification()
        .ok_or(ServerInitializeError::ExpectedInitializedNotification(
            Some(msg_clone),
        ))
}

pub async fn serve_server_with_ct<S, T, E, A>(
    service: S,
    transport: T,
    ct: CancellationToken,
) -> Result<RunningService<RoleServer, S>, ServerInitializeError<E>>
where
    S: Service<RoleServer>,
    T: IntoTransport<RoleServer, E, A>,
    E: std::error::Error + Send + Sync + 'static,
{
    let mut transport = transport.into_transport();
    let id_provider = <Arc<AtomicU32RequestIdProvider>>::default();

    // Get initialize request
    let (request, id) = expect_request(&mut transport, "initialized request").await?;

    let ClientRequest::InitializeRequest(peer_info) = &request else {
        return Err(ServerInitializeError::ExpectedInitializeRequest(Some(
            ClientJsonRpcMessage::request(request, id),
        )));
    };
    let (peer, peer_rx) = Peer::new(id_provider, Some(peer_info.params.clone()));
    let context = RequestContext {
        ct: ct.child_token(),
        id: id.clone(),
        meta: request.get_meta().clone(),
        extensions: request.extensions().clone(),
        peer: peer.clone(),
    };
    // Send initialize response
    let init_response = service.handle_request(request.clone(), context).await;
    let mut init_response = match init_response {
        Ok(ServerResult::InitializeResult(init_response)) => init_response,
        Ok(result) => {
            return Err(ServerInitializeError::UnexpectedInitializeResponse(result));
        }
        Err(e) => {
            transport
                .send(ServerJsonRpcMessage::error(e.clone(), id))
                .await
                .map_err(|error| ServerInitializeError::TransportError {
                    error,
                    context: "sending error response".into(),
                })?;
            return Err(ServerInitializeError::InitializeFailed(e));
        }
    };
    let peer_protocol_version = peer_info.params.protocol_version.clone();
    let protocol_version = match peer_protocol_version
        .partial_cmp(&init_response.protocol_version)
        .ok_or(ServerInitializeError::UnsupportedProtocolVersion(
            peer_protocol_version,
        ))? {
        std::cmp::Ordering::Less => peer_info.params.protocol_version.clone(),
        _ => init_response.protocol_version,
    };
    init_response.protocol_version = protocol_version;
    transport
        .send(ServerJsonRpcMessage::response(
            ServerResult::InitializeResult(init_response),
            id,
        ))
        .await
        .map_err(|error| ServerInitializeError::TransportError {
            error,
            context: "sending initialize response".into(),
        })?;

    // Wait for initialize notification
    let notification = expect_notification(&mut transport, "initialize notification").await?;
    let ClientNotification::InitializedNotification(_) = notification else {
        return Err(ServerInitializeError::ExpectedInitializedNotification(
            Some(ClientJsonRpcMessage::notification(notification)),
        ));
    };
    let context = NotificationContext {
        meta: notification.get_meta().clone(),
        extensions: notification.extensions().clone(),
        peer: peer.clone(),
    };
    let _ = service.handle_notification(notification, context).await;
    // Continue processing service
    Ok(serve_inner(service, transport, peer, peer_rx, ct))
}

macro_rules! method {
    (peer_req $method:ident $Req:ident() => $Resp: ident ) => {
        pub async fn $method(&self) -> Result<$Resp, ServiceError> {
            let result = self
                .send_request(ServerRequest::$Req($Req {
                    method: Default::default(),
                    extensions: Default::default(),
                }))
                .await?;
            match result {
                ClientResult::$Resp(result) => Ok(result),
                _ => Err(ServiceError::UnexpectedResponse),
            }
        }
    };
    (peer_req $method:ident $Req:ident($Param: ident) => $Resp: ident ) => {
        pub async fn $method(&self, params: $Param) -> Result<$Resp, ServiceError> {
            let result = self
                .send_request(ServerRequest::$Req($Req {
                    method: Default::default(),
                    params,
                    extensions: Default::default(),
                }))
                .await?;
            match result {
                ClientResult::$Resp(result) => Ok(result),
                _ => Err(ServiceError::UnexpectedResponse),
            }
        }
    };
    (peer_req $method:ident $Req:ident($Param: ident)) => {
        pub fn $method(
            &self,
            params: $Param,
        ) -> impl Future<Output = Result<(), ServiceError>> + Send + '_ {
            async move {
                let result = self
                    .send_request(ServerRequest::$Req($Req {
                        method: Default::default(),
                        params,
                    }))
                    .await?;
                match result {
                    ClientResult::EmptyResult(_) => Ok(()),
                    _ => Err(ServiceError::UnexpectedResponse),
                }
            }
        }
    };

    (peer_not $method:ident $Not:ident($Param: ident)) => {
        pub async fn $method(&self, params: $Param) -> Result<(), ServiceError> {
            self.send_notification(ServerNotification::$Not($Not {
                method: Default::default(),
                params,
                extensions: Default::default(),
            }))
            .await?;
            Ok(())
        }
    };
    (peer_not $method:ident $Not:ident) => {
        pub async fn $method(&self) -> Result<(), ServiceError> {
            self.send_notification(ServerNotification::$Not($Not {
                method: Default::default(),
                extensions: Default::default(),
            }))
            .await?;
            Ok(())
        }
    };
}

impl Peer<RoleServer> {
    method!(peer_req create_message CreateMessageRequest(CreateMessageRequestParam) => CreateMessageResult);
    method!(peer_req list_roots ListRootsRequest() => ListRootsResult);

    method!(peer_not notify_cancelled CancelledNotification(CancelledNotificationParam));
    method!(peer_not notify_progress ProgressNotification(ProgressNotificationParam));
    method!(peer_not notify_logging_message LoggingMessageNotification(LoggingMessageNotificationParam));
    method!(peer_not notify_resource_updated ResourceUpdatedNotification(ResourceUpdatedNotificationParam));
    method!(peer_not notify_resource_list_changed ResourceListChangedNotification);
    method!(peer_not notify_tool_list_changed ToolListChangedNotification);
    method!(peer_not notify_prompt_list_changed PromptListChangedNotification);
}

---
File: modelcontextprotocol-rust-sdk-13e72ec/crates/rmcp/src/service/tower.rs
---

use std::{future::poll_fn, marker::PhantomData};

use tower_service::Service as TowerService;

use super::NotificationContext;
use crate::service::{RequestContext, Service, ServiceRole};

pub struct TowerHandler<S, R: ServiceRole> {
    pub service: S,
    pub info: R::Info,
    role: PhantomData<R>,
}

impl<S, R: ServiceRole> TowerHandler<S, R> {
    pub fn new(service: S, info: R::Info) -> Self {
        Self {
            service,
            role: PhantomData,
            info,
        }
    }
}

impl<S, R: ServiceRole> Service<R> for TowerHandler<S, R>
where
    S: TowerService<R::PeerReq, Response = R::Resp> + Sync + Send + Clone + 'static,
    S::Error: Into<crate::Error>,
    S::Future: Send,
{
    async fn handle_request(
        &self,
        request: R::PeerReq,
        _context: RequestContext<R>,
    ) -> Result<R::Resp, crate::Error> {
        let mut service = self.service.clone();
        poll_fn(|cx| service.poll_ready(cx))
            .await
            .map_err(Into::into)?;
        let resp = service.call(request).await.map_err(Into::into)?;
        Ok(resp)
    }

    fn handle_notification(
        &self,
        _notification: R::PeerNot,
        _context: NotificationContext<R>,
    ) -> impl Future<Output = Result<(), crate::Error>> + Send + '_ {
        std::future::ready(Ok(()))
    }

    fn get_info(&self) -> R::Info {
        self.info.clone()
    }
}

---
File: modelcontextprotocol-rust-sdk-13e72ec/crates/rmcp/src/transport.rs
---

//! # Transport
//! The transport type must implemented [`Transport`] trait, which allow it send message concurrently and receive message sequentially.
//！
//! ## Standard Transport Types
//! There are 3 pairs of standard transport types:
//!
//! | transport         | client                                                    | server                                                |
//! |:-:                |:-:                                                        |:-:                                                    |
//! | std IO            | [`child_process::TokioChildProcess`]                      | [`io::stdio`]                                         |
//! | streamable http   | [`streamable_http_client::StreamableHttpClientTransport`] | [`streamable_http_server::StreamableHttpService`]     |
//! | sse               | [`sse_client::SseClientTransport`]                        | [`sse_server::SseServer`]                             |
//!
//！## Helper Transport Types
//! Thers are several helper transport types that can help you to create transport quickly.
//!
//! ### [Worker Transport](`worker::WorkerTransport`)
//! Which allows you to run a worker and process messages in another tokio task.
//!
//! ### [Async Read/Write Transport](`async_rw::AsyncRwTransport`)
//! You need to enable `transport-async-rw` feature to use this transport.
//!
//! This transport is used to create a transport from a byte stream which implemented [`tokio::io::AsyncRead`] and [`tokio::io::AsyncWrite`].
//!
//! This could be very helpful when you want to create a transport from a byte stream, such as a file or a tcp connection.
//!
//! ### [Sink/Stream Transport](`sink_stream::SinkStreamTransport`)
//! This transport is used to create a transport from a sink and a stream.
//!
//! This could be very helpful when you want to create a transport from a duplex object stream, such as a websocket connection.
//!
//! ## [IntoTransport](`IntoTransport`) trait
//! [`IntoTransport`] is a helper trait that implicitly convert a type into a transport type.
//!
//! ### These types is automatically implemented [`IntoTransport`] trait
//! 1. A type that already implement both [`futures::Sink`] and [`futures::Stream`] trait, or a tuple `(Tx, Rx)`  where `Tx` is [`futures::Sink`] and `Rx` is [`futures::Stream`].
//! 2. A type that implement both [`tokio::io::AsyncRead`] and [`tokio::io::AsyncWrite`] trait. or a tuple `(R, W)` where `R` is [`tokio::io::AsyncRead`] and `W` is [`tokio::io::AsyncWrite`].
//! 3. A type that implement [Worker](`worker::Worker`) trait.
//! 4. A type that implement [`Transport`] trait.
//!
//! ## Examples
//!
//! ```rust
//! # use rmcp::{
//! #     ServiceExt, serve_client, serve_server,
//! # };
//!
//! // create transport from tcp stream
//! async fn client() -> Result<(), Box<dyn std::error::Error>> {
//!     let stream = tokio::net::TcpSocket::new_v4()?
//!         .connect("127.0.0.1:8001".parse()?)
//!         .await?;
//!     let client = ().serve(stream).await?;
//!     let tools = client.peer().list_tools(Default::default()).await?;
//!     println!("{:?}", tools);
//!     Ok(())
//! }
//!
//! // create transport from std io
//! async fn io()  -> Result<(), Box<dyn std::error::Error>> {
//!     let client = ().serve((tokio::io::stdin(), tokio::io::stdout())).await?;
//!     let tools = client.peer().list_tools(Default::default()).await?;
//!     println!("{:?}", tools);
//!     Ok(())
//! }
//! ```

use std::sync::Arc;

use crate::service::{RxJsonRpcMessage, ServiceRole, TxJsonRpcMessage};

pub mod sink_stream;

#[cfg(feature = "transport-async-rw")]
#[cfg_attr(docsrs, doc(cfg(feature = "transport-async-rw")))]
pub mod async_rw;

#[cfg(feature = "transport-worker")]
#[cfg_attr(docsrs, doc(cfg(feature = "transport-worker")))]
pub mod worker;
#[cfg(feature = "transport-worker")]
#[cfg_attr(docsrs, doc(cfg(feature = "transport-worker")))]
pub use worker::WorkerTransport;

#[cfg(feature = "transport-child-process")]
#[cfg_attr(docsrs, doc(cfg(feature = "transport-child-process")))]
pub mod child_process;
#[cfg(feature = "transport-child-process")]
#[cfg_attr(docsrs, doc(cfg(feature = "transport-child-process")))]
pub use child_process::{ConfigureCommandExt, TokioChildProcess};

#[cfg(feature = "transport-io")]
#[cfg_attr(docsrs, doc(cfg(feature = "transport-io")))]
pub mod io;
#[cfg(feature = "transport-io")]
#[cfg_attr(docsrs, doc(cfg(feature = "transport-io")))]
pub use io::stdio;

#[cfg(feature = "transport-sse-client")]
#[cfg_attr(docsrs, doc(cfg(feature = "transport-sse-client")))]
pub mod sse_client;
#[cfg(feature = "transport-sse-client")]
#[cfg_attr(docsrs, doc(cfg(feature = "transport-sse-client")))]
pub use sse_client::SseClientTransport;

#[cfg(feature = "transport-sse-server")]
#[cfg_attr(docsrs, doc(cfg(feature = "transport-sse-server")))]
pub mod sse_server;
#[cfg(feature = "transport-sse-server")]
#[cfg_attr(docsrs, doc(cfg(feature = "transport-sse-server")))]
pub use sse_server::SseServer;

#[cfg(feature = "auth")]
#[cfg_attr(docsrs, doc(cfg(feature = "auth")))]
pub mod auth;
#[cfg(feature = "auth")]
#[cfg_attr(docsrs, doc(cfg(feature = "auth")))]
pub use auth::{AuthError, AuthorizationManager, AuthorizationSession, AuthorizedHttpClient};

// #[cfg(feature = "transport-ws")]
// #[cfg_attr(docsrs, doc(cfg(feature = "transport-ws")))]
// pub mod ws;
#[cfg(feature = "transport-streamable-http-server-session")]
#[cfg_attr(docsrs, doc(cfg(feature = "transport-streamable-http-server-session")))]
pub mod streamable_http_server;
#[cfg(feature = "transport-streamable-http-server")]
#[cfg_attr(docsrs, doc(cfg(feature = "transport-streamable-http-server")))]
pub use streamable_http_server::tower::{StreamableHttpServerConfig, StreamableHttpService};

#[cfg(feature = "transport-streamable-http-client")]
#[cfg_attr(docsrs, doc(cfg(feature = "transport-streamable-http-client")))]
pub mod streamable_http_client;
#[cfg(feature = "transport-streamable-http-client")]
#[cfg_attr(docsrs, doc(cfg(feature = "transport-streamable-http-client")))]
pub use streamable_http_client::StreamableHttpClientTransport;

/// Common use codes
pub mod common;

pub trait Transport<R>: Send
where
    R: ServiceRole,
{
    type Error: std::error::Error + Send + Sync + 'static;
    /// Send a message to the transport
    ///
    /// Notice that the future returned by this function should be `Send` and `'static`.
    /// It's because the sending message could be executed concurrently.
    ///
    fn send(
        &mut self,
        item: TxJsonRpcMessage<R>,
    ) -> impl Future<Output = Result<(), Self::Error>> + Send + 'static;

    /// Receive a message from the transport, this operation is sequential.
    fn receive(&mut self) -> impl Future<Output = Option<RxJsonRpcMessage<R>>> + Send;

    /// Close the transport
    fn close(&mut self) -> impl Future<Output = Result<(), Self::Error>> + Send;
}

pub trait IntoTransport<R, E, A>: Send + 'static
where
    R: ServiceRole,
    E: std::error::Error + Send + 'static,
{
    fn into_transport(self) -> impl Transport<R, Error = E> + 'static;
}

pub enum TransportAdapterIdentity {}
impl<R, T, E> IntoTransport<R, E, TransportAdapterIdentity> for T
where
    T: Transport<R, Error = E> + Send + 'static,
    R: ServiceRole,
    E: std::error::Error + Send + Sync + 'static,
{
    fn into_transport(self) -> impl Transport<R, Error = E> + 'static {
        self
    }
}

/// A transport that can send a single message and then close itself
pub struct OneshotTransport<R>
where
    R: ServiceRole,
{
    message: Option<RxJsonRpcMessage<R>>,
    sender: tokio::sync::mpsc::Sender<TxJsonRpcMessage<R>>,
    finished_signal: Arc<tokio::sync::Notify>,
}

impl<R> OneshotTransport<R>
where
    R: ServiceRole,
{
    pub fn new(
        message: RxJsonRpcMessage<R>,
    ) -> (Self, tokio::sync::mpsc::Receiver<TxJsonRpcMessage<R>>) {
        let (sender, receiver) = tokio::sync::mpsc::channel(16);
        (
            Self {
                message: Some(message),
                sender,
                finished_signal: Arc::new(tokio::sync::Notify::new()),
            },
            receiver,
        )
    }
}

impl<R> Transport<R> for OneshotTransport<R>
where
    R: ServiceRole,
{
    type Error = tokio::sync::mpsc::error::SendError<TxJsonRpcMessage<R>>;

    fn send(
        &mut self,
        item: TxJsonRpcMessage<R>,
    ) -> impl Future<Output = Result<(), Self::Error>> + Send + 'static {
        let sender = self.sender.clone();
        let terminate = matches!(item, TxJsonRpcMessage::<R>::Response(_));
        let signal = self.finished_signal.clone();
        async move {
            sender.send(item).await?;
            if terminate {
                signal.notify_waiters();
            }
            Ok(())
        }
    }

    async fn receive(&mut self) -> Option<RxJsonRpcMessage<R>> {
        if self.message.is_none() {
            self.finished_signal.notified().await;
        }
        self.message.take()
    }

    fn close(&mut self) -> impl Future<Output = Result<(), Self::Error>> + Send {
        self.message.take();
        std::future::ready(Ok(()))
    }
}

---
File: modelcontextprotocol-rust-sdk-13e72ec/crates/rmcp/src/transport/async_rw.rs
---

use std::{marker::PhantomData, sync::Arc};

// use crate::schema::*;
use futures::{SinkExt, StreamExt};
use serde::{Serialize, de::DeserializeOwned};
use thiserror::Error;
use tokio::{
    io::{AsyncRead, AsyncWrite},
    sync::Mutex,
};
use tokio_util::{
    bytes::{Buf, BufMut, BytesMut},
    codec::{Decoder, Encoder, FramedRead, FramedWrite},
};

use super::{IntoTransport, Transport};
use crate::service::{RxJsonRpcMessage, ServiceRole, TxJsonRpcMessage};

pub enum TransportAdapterAsyncRW {}

impl<Role, R, W> IntoTransport<Role, std::io::Error, TransportAdapterAsyncRW> for (R, W)
where
    Role: ServiceRole,
    R: AsyncRead + Send + 'static + Unpin,
    W: AsyncWrite + Send + 'static + Unpin,
{
    fn into_transport(self) -> impl Transport<Role, Error = std::io::Error> + 'static {
        AsyncRwTransport::new(self.0, self.1)
    }
}

pub enum TransportAdapterAsyncCombinedRW {}
impl<Role, S> IntoTransport<Role, std::io::Error, TransportAdapterAsyncCombinedRW> for S
where
    Role: ServiceRole,
    S: AsyncRead + AsyncWrite + Send + 'static,
{
    fn into_transport(self) -> impl Transport<Role, Error = std::io::Error> + 'static {
        IntoTransport::<Role, std::io::Error, TransportAdapterAsyncRW>::into_transport(
            tokio::io::split(self),
        )
    }
}

pub struct AsyncRwTransport<Role: ServiceRole, R: AsyncRead, W: AsyncWrite> {
    read: FramedRead<R, JsonRpcMessageCodec<RxJsonRpcMessage<Role>>>,
    write: Arc<Mutex<FramedWrite<W, JsonRpcMessageCodec<TxJsonRpcMessage<Role>>>>>,
}

impl<Role: ServiceRole, R, W> AsyncRwTransport<Role, R, W>
where
    R: Send + AsyncRead + Unpin,
    W: Send + AsyncWrite + Unpin + 'static,
{
    pub fn new(read: R, write: W) -> Self {
        let read = FramedRead::new(
            read,
            JsonRpcMessageCodec::<RxJsonRpcMessage<Role>>::default(),
        );
        let write = Arc::new(Mutex::new(FramedWrite::new(
            write,
            JsonRpcMessageCodec::<TxJsonRpcMessage<Role>>::default(),
        )));
        Self { read, write }
    }
}

#[cfg(feature = "client")]
#[cfg_attr(docsrs, doc(cfg(feature = "client")))]
impl<R, W> AsyncRwTransport<crate::RoleClient, R, W>
where
    R: Send + AsyncRead + Unpin,
    W: Send + AsyncWrite + Unpin + 'static,
{
    pub fn new_client(read: R, write: W) -> Self {
        Self::new(read, write)
    }
}

#[cfg(feature = "server")]
#[cfg_attr(docsrs, doc(cfg(feature = "server")))]
impl<R, W> AsyncRwTransport<crate::RoleServer, R, W>
where
    R: Send + AsyncRead + Unpin,
    W: Send + AsyncWrite + Unpin + 'static,
{
    pub fn new_server(read: R, write: W) -> Self {
        Self::new(read, write)
    }
}

impl<Role: ServiceRole, R, W> Transport<Role> for AsyncRwTransport<Role, R, W>
where
    R: Send + AsyncRead + Unpin,
    W: Send + AsyncWrite + Unpin + 'static,
{
    type Error = std::io::Error;

    fn send(
        &mut self,
        item: TxJsonRpcMessage<Role>,
    ) -> impl Future<Output = Result<(), Self::Error>> + Send + 'static {
        let lock = self.write.clone();
        async move {
            let mut write = lock.lock().await;
            write.send(item).await.map_err(Into::into)
        }
    }

    fn receive(&mut self) -> impl Future<Output = Option<RxJsonRpcMessage<Role>>> {
        let next = self.read.next();
        async {
            next.await.and_then(|e| {
                e.inspect_err(|e| {
                    tracing::error!("Error reading from stream: {}", e);
                })
                .ok()
            })
        }
    }

    async fn close(&mut self) -> Result<(), Self::Error> {
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct JsonRpcMessageCodec<T> {
    _marker: PhantomData<fn() -> T>,
    next_index: usize,
    max_length: usize,
    is_discarding: bool,
}

impl<T> Default for JsonRpcMessageCodec<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T> JsonRpcMessageCodec<T> {
    pub fn new() -> Self {
        Self {
            _marker: PhantomData,
            next_index: 0,
            max_length: usize::MAX,
            is_discarding: false,
        }
    }

    pub fn new_with_max_length(max_length: usize) -> Self {
        Self {
            max_length,
            ..Self::new()
        }
    }

    pub fn max_length(&self) -> usize {
        self.max_length
    }
}

fn without_carriage_return(s: &[u8]) -> &[u8] {
    if let Some(&b'\r') = s.last() {
        &s[..s.len() - 1]
    } else {
        s
    }
}

/// Check if a notification method is a standard MCP notification
/// should update this when MCP spec is updated about new notifications
fn is_standard_notification(method: &str) -> bool {
    matches!(
        method,
        "notifications/cancelled"
            | "notifications/initialized"
            | "notifications/message"
            | "notifications/progress"
            | "notifications/prompts/list_changed"
            | "notifications/resources/list_changed"
            | "notifications/resources/updated"
            | "notifications/roots/list_changed"
            | "notifications/tools/list_changed"
    )
}

/// Try to parse a message with compatibility handling for non-standard notifications
fn try_parse_with_compatibility<T: serde::de::DeserializeOwned>(
    line: &[u8],
    context: &str,
) -> Result<Option<T>, JsonRpcMessageCodecError> {
    if let Ok(line_str) = std::str::from_utf8(line) {
        match serde_json::from_slice(line) {
            Ok(item) => Ok(Some(item)),
            Err(e) => {
                // Check if this is a non-standard notification that should be ignored
                if line_str.contains("\"method\":\"notifications/") {
                    // Extract the method name to check if it's standard
                    if let Ok(json_value) = serde_json::from_str::<serde_json::Value>(line_str) {
                        if let Some(method) = json_value.get("method").and_then(|m| m.as_str()) {
                            if method.starts_with("notifications/")
                                && !is_standard_notification(method)
                            {
                                tracing::debug!(
                                    "Ignoring non-standard notification {} {}: {}",
                                    method,
                                    context,
                                    line_str
                                );
                                return Ok(None); // Skip this message
                            }
                        }
                    }
                }

                tracing::debug!(
                    "Failed to parse message {}: {} | Error: {}",
                    context,
                    line_str,
                    e
                );
                Err(JsonRpcMessageCodecError::Serde(e))
            }
        }
    } else {
        serde_json::from_slice(line)
            .map(Some)
            .map_err(JsonRpcMessageCodecError::Serde)
    }
}

#[derive(Debug, Error)]
pub enum JsonRpcMessageCodecError {
    #[error("max line length exceeded")]
    MaxLineLengthExceeded,
    #[error("serde error {0}")]
    Serde(#[from] serde_json::Error),
    #[error("io error {0}")]
    Io(#[from] std::io::Error),
}

impl From<JsonRpcMessageCodecError> for std::io::Error {
    fn from(value: JsonRpcMessageCodecError) -> Self {
        match value {
            JsonRpcMessageCodecError::MaxLineLengthExceeded => {
                std::io::Error::new(std::io::ErrorKind::InvalidData, value)
            }
            JsonRpcMessageCodecError::Serde(e) => e.into(),
            JsonRpcMessageCodecError::Io(e) => e,
        }
    }
}

impl<T: DeserializeOwned> Decoder for JsonRpcMessageCodec<T> {
    type Item = T;

    type Error = JsonRpcMessageCodecError;

    fn decode(
        &mut self,
        buf: &mut BytesMut,
    ) -> Result<Option<Self::Item>, JsonRpcMessageCodecError> {
        loop {
            // Determine how far into the buffer we'll search for a newline. If
            // there's no max_length set, we'll read to the end of the buffer.
            let read_to = std::cmp::min(self.max_length.saturating_add(1), buf.len());

            let newline_offset = buf[self.next_index..read_to]
                .iter()
                .position(|b| *b == b'\n');

            match (self.is_discarding, newline_offset) {
                (true, Some(offset)) => {
                    // If we found a newline, discard up to that offset and
                    // then stop discarding. On the next iteration, we'll try
                    // to read a line normally.
                    buf.advance(offset + self.next_index + 1);
                    self.is_discarding = false;
                    self.next_index = 0;
                }
                (true, None) => {
                    // Otherwise, we didn't find a newline, so we'll discard
                    // everything we read. On the next iteration, we'll continue
                    // discarding up to max_len bytes unless we find a newline.
                    buf.advance(read_to);
                    self.next_index = 0;
                    if buf.is_empty() {
                        return Ok(None);
                    }
                }
                (false, Some(offset)) => {
                    // Found a line!
                    let newline_index = offset + self.next_index;
                    self.next_index = 0;
                    let line = buf.split_to(newline_index + 1);
                    let line = &line[..line.len() - 1];
                    let line = without_carriage_return(line);

                    // Use compatibility handling function
                    let item = match try_parse_with_compatibility(line, "decode")? {
                        Some(item) => item,
                        None => return Ok(None), // Skip non-standard message
                    };
                    return Ok(Some(item));
                }
                (false, None) if buf.len() > self.max_length => {
                    // Reached the maximum length without finding a
                    // newline, return an error and start discarding on the
                    // next call.
                    self.is_discarding = true;
                    return Err(JsonRpcMessageCodecError::MaxLineLengthExceeded);
                }
                (false, None) => {
                    // We didn't find a line or reach the length limit, so the next
                    // call will resume searching at the current offset.
                    self.next_index = read_to;
                    return Ok(None);
                }
            }
        }
    }

    fn decode_eof(&mut self, buf: &mut BytesMut) -> Result<Option<T>, JsonRpcMessageCodecError> {
        Ok(match self.decode(buf)? {
            Some(frame) => Some(frame),
            None => {
                self.next_index = 0;
                // No terminating newline - return remaining data, if any
                if buf.is_empty() || buf == &b"\r"[..] {
                    None
                } else {
                    let line = buf.split_to(buf.len());
                    let line = without_carriage_return(&line);

                    // Use compatibility handling function
                    let item = match try_parse_with_compatibility(line, "decode_eof")? {
                        Some(item) => item,
                        None => return Ok(None), // Skip non-standard message
                    };
                    Some(item)
                }
            }
        })
    }
}

impl<T: Serialize> Encoder<T> for JsonRpcMessageCodec<T> {
    type Error = JsonRpcMessageCodecError;

    fn encode(&mut self, item: T, buf: &mut BytesMut) -> Result<(), JsonRpcMessageCodecError> {
        serde_json::to_writer(buf.writer(), &item)?;
        buf.put_u8(b'\n');
        Ok(())
    }
}

#[cfg(test)]
mod test {
    use futures::{Sink, Stream};

    use super::*;
    fn from_async_read<T: DeserializeOwned, R: AsyncRead>(reader: R) -> impl Stream<Item = T> {
        FramedRead::new(reader, JsonRpcMessageCodec::<T>::default()).filter_map(|result| {
            if let Err(e) = &result {
                tracing::error!("Error reading from stream: {}", e);
            }
            futures::future::ready(result.ok())
        })
    }

    fn from_async_write<T: Serialize, W: AsyncWrite + Send>(
        writer: W,
    ) -> impl Sink<T, Error = std::io::Error> {
        FramedWrite::new(writer, JsonRpcMessageCodec::<T>::default()).sink_map_err(Into::into)
    }
    #[tokio::test]
    async fn test_decode() {
        use futures::StreamExt;
        use tokio::io::BufReader;

        let data = r#"{"jsonrpc":"2.0","method":"subtract","params":[42,23],"id":1}
    {"jsonrpc":"2.0","method":"subtract","params":[23,42],"id":2}
    {"jsonrpc":"2.0","method":"subtract","params":[42,23],"id":3}
    {"jsonrpc":"2.0","method":"subtract","params":[23,42],"id":4}
    {"jsonrpc":"2.0","method":"subtract","params":[42,23],"id":5}
    {"jsonrpc":"2.0","method":"subtract","params":[23,42],"id":6}
    {"jsonrpc":"2.0","method":"subtract","params":[42,23],"id":7}
    {"jsonrpc":"2.0","method":"subtract","params":[23,42],"id":8}
    {"jsonrpc":"2.0","method":"subtract","params":[42,23],"id":9}
    {"jsonrpc":"2.0","method":"subtract","params":[23,42],"id":10}

    "#;

        let mut cursor = BufReader::new(data.as_bytes());
        let mut stream = from_async_read::<serde_json::Value, _>(&mut cursor);

        for i in 1..=10 {
            let item = stream.next().await.unwrap();
            assert_eq!(
                item,
                serde_json::json!({
                    "jsonrpc": "2.0",
                    "method": "subtract",
                    "params": if i % 2 != 0 { [42, 23] } else { [23, 42] },
                    "id": i,
                })
            );
        }
    }

    #[tokio::test]
    async fn test_encode() {
        let test_messages = vec![
            serde_json::json!({
                "jsonrpc": "2.0",
                "method": "subtract",
                "params": [42, 23],
                "id": 1,
            }),
            serde_json::json!({
                "jsonrpc": "2.0",
                "method": "subtract",
                "params": [23, 42],
                "id": 2,
            }),
        ];

        // Create a buffer to write to
        let mut buffer = Vec::new();
        let mut writer = from_async_write(&mut buffer);

        // Write the test messages
        for message in test_messages.iter() {
            writer.send(message.clone()).await.unwrap();
        }
        writer.close().await.unwrap();
        drop(writer);
        // Parse the buffer back into lines and check each one
        let output = String::from_utf8_lossy(&buffer);
        let mut lines = output.lines();

        for expected_message in test_messages {
            let line = lines.next().unwrap();
            let parsed_message: serde_json::Value = serde_json::from_str(line).unwrap();
            assert_eq!(parsed_message, expected_message);
        }

        // Make sure there are no extra lines
        assert!(lines.next().is_none());
    }

    #[test]
    fn test_standard_notification_check() {
        // Test that all standard notifications are recognized
        assert!(is_standard_notification("notifications/cancelled"));
        assert!(is_standard_notification("notifications/initialized"));
        assert!(is_standard_notification("notifications/progress"));
        assert!(is_standard_notification(
            "notifications/resources/list_changed"
        ));
        assert!(is_standard_notification("notifications/resources/updated"));
        assert!(is_standard_notification(
            "notifications/prompts/list_changed"
        ));
        assert!(is_standard_notification("notifications/tools/list_changed"));
        assert!(is_standard_notification("notifications/message"));
        assert!(is_standard_notification("notifications/roots/list_changed"));

        // Test that non-standard notifications are not recognized
        assert!(!is_standard_notification("notifications/stderr"));
        assert!(!is_standard_notification("notifications/custom"));
        assert!(!is_standard_notification("notifications/debug"));
        assert!(!is_standard_notification("some/other/method"));
    }

    #[test]
    fn test_compatibility_function() {
        // Test the compatibility function directly
        let stderr_message =
            r#"{"method":"notifications/stderr","params":{"content":"stderr message"}}"#;
        let custom_message = r#"{"method":"notifications/custom","params":{"data":"custom"}}"#;
        let standard_message =
            r#"{"method":"notifications/message","params":{"level":"info","data":"standard"}}"#;
        let progress_message = r#"{"method":"notifications/progress","params":{"progressToken":"token","progress":50}}"#;

        // Test with valid JSON - all should parse successfully
        let result1 =
            try_parse_with_compatibility::<serde_json::Value>(stderr_message.as_bytes(), "test");
        let result2 =
            try_parse_with_compatibility::<serde_json::Value>(custom_message.as_bytes(), "test");
        let result3 =
            try_parse_with_compatibility::<serde_json::Value>(standard_message.as_bytes(), "test");
        let result4 =
            try_parse_with_compatibility::<serde_json::Value>(progress_message.as_bytes(), "test");

        // All should parse successfully since they're valid JSON
        assert!(result1.is_ok());
        assert!(result2.is_ok());
        assert!(result3.is_ok());
        assert!(result4.is_ok());

        // Standard notifications should return Some(value)
        assert!(result3.unwrap().is_some());
        assert!(result4.unwrap().is_some());

        println!("Standard notifications are preserved, non-standard are handled gracefully");
    }
}

---
File: modelcontextprotocol-rust-sdk-13e72ec/crates/rmcp/src/transport/auth.rs
---

use std::{
    collections::HashMap,
    sync::Arc,
    time::{Duration, Instant},
};

use oauth2::{
    AuthUrl, AuthorizationCode, ClientId, ClientSecret, CsrfToken, EmptyExtraTokenFields,
    PkceCodeChallenge, PkceCodeVerifier, RedirectUrl, RefreshToken, Scope, StandardTokenResponse,
    TokenResponse, TokenUrl,
    basic::{BasicClient, BasicTokenType},
};
use reqwest::{Client as HttpClient, IntoUrl, StatusCode, Url, header::AUTHORIZATION};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use tokio::sync::{Mutex, RwLock};
use tracing::{debug, error};

/// sse client with oauth2 authorization
#[derive(Clone)]
pub struct AuthClient<C> {
    pub http_client: C,
    pub auth_manager: Arc<Mutex<AuthorizationManager>>,
}

impl<C: std::fmt::Debug> std::fmt::Debug for AuthClient<C> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AuthorizedClient")
            .field("http_client", &self.http_client)
            .field("auth_manager", &"...")
            .finish()
    }
}

impl<C> AuthClient<C> {
    /// create new authorized sse client
    pub fn new(http_client: C, auth_manager: AuthorizationManager) -> Self {
        Self {
            http_client,
            auth_manager: Arc::new(Mutex::new(auth_manager)),
        }
    }
}

impl<C> AuthClient<C> {
    pub fn get_access_token(&self) -> impl Future<Output = Result<String, AuthError>> + Send {
        let auth_manager = self.auth_manager.clone();
        async move { auth_manager.lock().await.get_access_token().await }
    }
}

/// Auth error
#[derive(Debug, Error)]
pub enum AuthError {
    #[error("OAuth authorization required")]
    AuthorizationRequired,

    #[error("OAuth authorization failed: {0}")]
    AuthorizationFailed(String),

    #[error("OAuth token exchange failed: {0}")]
    TokenExchangeFailed(String),

    #[error("OAuth token refresh failed: {0}")]
    TokenRefreshFailed(String),

    #[error("HTTP error: {0}")]
    HttpError(#[from] reqwest::Error),

    #[error("OAuth error: {0}")]
    OAuthError(String),

    #[error("Metadata error: {0}")]
    MetadataError(String),

    #[error("URL parse error: {0}")]
    UrlError(#[from] url::ParseError),

    #[error("No authorization support detected")]
    NoAuthorizationSupport,

    #[error("Internal error: {0}")]
    InternalError(String),

    #[error("Invalid token type: {0}")]
    InvalidTokenType(String),

    #[error("Token expired")]
    TokenExpired,

    #[error("Invalid scope: {0}")]
    InvalidScope(String),

    #[error("Registration failed: {0}")]
    RegistrationFailed(String),
}

/// oauth2 metadata
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AuthorizationMetadata {
    pub authorization_endpoint: String,
    pub token_endpoint: String,
    pub registration_endpoint: String,
    pub issuer: Option<String>,
    pub jwks_uri: Option<String>,
    pub scopes_supported: Option<Vec<String>>,
    // allow additional fields
    #[serde(flatten)]
    pub additional_fields: HashMap<String, serde_json::Value>,
}

/// oauth2 client config
#[derive(Debug, Clone)]
pub struct OAuthClientConfig {
    pub client_id: String,
    pub client_secret: Option<String>,
    pub scopes: Vec<String>,
    pub redirect_uri: String,
}

// add type aliases for oauth2 types
type OAuthErrorResponse = oauth2::StandardErrorResponse<oauth2::basic::BasicErrorResponseType>;
type OAuthTokenResponse = StandardTokenResponse<EmptyExtraTokenFields, BasicTokenType>;
type OAuthTokenIntrospection =
    oauth2::StandardTokenIntrospectionResponse<EmptyExtraTokenFields, BasicTokenType>;
type OAuthRevocableToken = oauth2::StandardRevocableToken;
type OAuthRevocationError = oauth2::StandardErrorResponse<oauth2::RevocationErrorResponseType>;
type OAuthClient = oauth2::Client<
    OAuthErrorResponse,
    OAuthTokenResponse,
    OAuthTokenIntrospection,
    OAuthRevocableToken,
    OAuthRevocationError,
    oauth2::EndpointSet,
    oauth2::EndpointNotSet,
    oauth2::EndpointNotSet,
    oauth2::EndpointNotSet,
    oauth2::EndpointSet,
>;
type Credentials = (String, Option<OAuthTokenResponse>);

/// oauth2 auth manager
pub struct AuthorizationManager {
    http_client: HttpClient,
    metadata: Option<AuthorizationMetadata>,
    oauth_client: Option<OAuthClient>,
    credentials: RwLock<Option<OAuthTokenResponse>>,
    pkce_verifier: RwLock<Option<PkceCodeVerifier>>,
    expires_at: RwLock<Option<Instant>>,
    base_url: Url,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClientRegistrationRequest {
    pub client_name: String,
    pub redirect_uris: Vec<String>,
    pub grant_types: Vec<String>,
    pub token_endpoint_auth_method: String,
    pub response_types: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClientRegistrationResponse {
    pub client_id: String,
    pub client_secret: Option<String>,
    pub client_name: String,
    pub redirect_uris: Vec<String>,
    // allow additional fields
    #[serde(flatten)]
    pub additional_fields: HashMap<String, serde_json::Value>,
}

impl AuthorizationManager {
    /// create new auth manager with base url
    pub async fn new<U: IntoUrl>(base_url: U) -> Result<Self, AuthError> {
        let base_url = base_url.into_url()?;
        let http_client = HttpClient::builder()
            .timeout(Duration::from_secs(30))
            .build()
            .map_err(|e| AuthError::InternalError(e.to_string()))?;

        let manager = Self {
            http_client,
            metadata: None,
            oauth_client: None,
            credentials: RwLock::new(None),
            pkce_verifier: RwLock::new(None),
            expires_at: RwLock::new(None),
            base_url,
        };

        Ok(manager)
    }

    pub fn with_client(&mut self, http_client: HttpClient) -> Result<(), AuthError> {
        self.http_client = http_client;
        Ok(())
    }

    /// discover oauth2 metadata
    pub async fn discover_metadata(&self) -> Result<AuthorizationMetadata, AuthError> {
        // according to the specification, the metadata should be located at "/.well-known/oauth-authorization-server"
        let mut discovery_url = self.base_url.clone();
        discovery_url.set_path("/.well-known/oauth-authorization-server");
        debug!("discovery url: {:?}", discovery_url);
        let response = self
            .http_client
            .get(discovery_url)
            .header("MCP-Protocol-Version", "2024-11-05")
            .send()
            .await?;

        if response.status() == StatusCode::OK {
            let metadata = response
                .json::<AuthorizationMetadata>()
                .await
                .map_err(|e| {
                    AuthError::MetadataError(format!("Failed to parse metadata: {}", e))
                })?;
            debug!("metadata: {:?}", metadata);
            Ok(metadata)
        } else {
            // fallback to default endpoints
            let mut auth_base = self.base_url.clone();
            // discard the path part, only keep scheme, host, port
            auth_base.set_path("");

            Ok(AuthorizationMetadata {
                authorization_endpoint: format!("{}/authorize", auth_base),
                token_endpoint: format!("{}/token", auth_base),
                registration_endpoint: format!("{}/register", auth_base),
                issuer: None,
                jwks_uri: None,
                scopes_supported: None,
                additional_fields: HashMap::new(),
            })
        }
    }

    /// get client id and credentials
    pub async fn get_credentials(&self) -> Result<Credentials, AuthError> {
        let credentials = self.credentials.read().await;
        let client_id = self
            .oauth_client
            .as_ref()
            .ok_or_else(|| AuthError::InternalError("OAuth client not configured".to_string()))?
            .client_id();
        Ok((client_id.to_string(), credentials.clone()))
    }

    /// configure oauth2 client with client credentials
    pub fn configure_client(&mut self, config: OAuthClientConfig) -> Result<(), AuthError> {
        if self.metadata.is_none() {
            return Err(AuthError::NoAuthorizationSupport);
        }

        let metadata = self.metadata.as_ref().unwrap();

        let auth_url = AuthUrl::new(metadata.authorization_endpoint.clone())
            .map_err(|e| AuthError::OAuthError(format!("Invalid authorization URL: {}", e)))?;

        let token_url = TokenUrl::new(metadata.token_endpoint.clone())
            .map_err(|e| AuthError::OAuthError(format!("Invalid token URL: {}", e)))?;

        // debug!("token url: {:?}", token_url);
        let client_id = ClientId::new(config.client_id);
        let redirect_url = RedirectUrl::new(config.redirect_uri.clone())
            .map_err(|e| AuthError::OAuthError(format!("Invalid re URL: {}", e)))?;

        debug!("client_id: {:?}", client_id);
        let mut client_builder = BasicClient::new(client_id.clone())
            .set_auth_uri(auth_url)
            .set_token_uri(token_url)
            .set_redirect_uri(redirect_url);

        if let Some(secret) = config.client_secret {
            client_builder = client_builder.set_client_secret(ClientSecret::new(secret));
        }

        self.oauth_client = Some(client_builder);
        Ok(())
    }

    /// dynamic register oauth2 client
    pub async fn register_client(
        &mut self,
        name: &str,
        redirect_uri: &str,
    ) -> Result<OAuthClientConfig, AuthError> {
        if self.metadata.is_none() {
            error!("No authorization support detected");
            return Err(AuthError::NoAuthorizationSupport);
        }

        let metadata = self.metadata.as_ref().unwrap();
        let registration_url = metadata.registration_endpoint.clone();

        debug!("registration url: {:?}", registration_url);
        // prepare registration request
        let registration_request = ClientRegistrationRequest {
            client_name: name.to_string(),
            redirect_uris: vec![redirect_uri.to_string()],
            grant_types: vec![
                "authorization_code".to_string(),
                "refresh_token".to_string(),
            ],
            token_endpoint_auth_method: "none".to_string(), // public client
            response_types: vec!["code".to_string()],
        };

        debug!("registration request: {:?}", registration_request);

        let response = match self
            .http_client
            .post(registration_url)
            .json(&registration_request)
            .send()
            .await
        {
            Ok(response) => response,
            Err(e) => {
                error!("Registration request failed: {}", e);
                return Err(AuthError::RegistrationFailed(format!(
                    "HTTP request error: {}",
                    e
                )));
            }
        };

        if !response.status().is_success() {
            let status = response.status();
            let error_text = match response.text().await {
                Ok(text) => text,
                Err(_) => "cannot get error details".to_string(),
            };

            error!("Registration failed: HTTP {} - {}", status, error_text);
            return Err(AuthError::RegistrationFailed(format!(
                "HTTP {}: {}",
                status, error_text
            )));
        }

        debug!("registration response: {:?}", response);
        let reg_response = match response.json::<ClientRegistrationResponse>().await {
            Ok(response) => response,
            Err(e) => {
                error!("Failed to parse registration response: {}", e);
                return Err(AuthError::RegistrationFailed(format!(
                    "analyze response error: {}",
                    e
                )));
            }
        };

        let config = OAuthClientConfig {
            client_id: reg_response.client_id,
            client_secret: reg_response.client_secret,
            redirect_uri: redirect_uri.to_string(),
            scopes: vec![],
        };

        self.configure_client(config.clone())?;
        Ok(config)
    }

    /// use provided client id to configure oauth2 client instead of dynamic registration
    /// this is useful when you have a stored client id from previous registration
    pub fn configure_client_id(&mut self, client_id: &str) -> Result<(), AuthError> {
        let config = OAuthClientConfig {
            client_id: client_id.to_string(),
            client_secret: None,
            scopes: vec![],
            redirect_uri: self.base_url.to_string(),
        };
        self.configure_client(config)
    }

    /// generate authorization url
    pub async fn get_authorization_url(&self, scopes: &[&str]) -> Result<String, AuthError> {
        let oauth_client = self
            .oauth_client
            .as_ref()
            .ok_or_else(|| AuthError::InternalError("OAuth client not configured".to_string()))?;

        // generate pkce challenge
        let (pkce_challenge, pkce_verifier) = PkceCodeChallenge::new_random_sha256();

        // build authorization request
        let mut auth_request = oauth_client
            .authorize_url(CsrfToken::new_random)
            .set_pkce_challenge(pkce_challenge);

        // add request scopes
        for scope in scopes {
            auth_request = auth_request.add_scope(Scope::new(scope.to_string()));
        }

        let (auth_url, _csrf_token) = auth_request.url();

        // store pkce verifier for later use
        *self.pkce_verifier.write().await = Some(pkce_verifier);
        debug!("set pkce verifier: {:?}", self.pkce_verifier.read().await);

        Ok(auth_url.to_string())
    }

    /// exchange authorization code for access token
    pub async fn exchange_code_for_token(
        &self,
        code: &str,
    ) -> Result<StandardTokenResponse<EmptyExtraTokenFields, BasicTokenType>, AuthError> {
        debug!("start exchange code for token: {:?}", code);
        let oauth_client = self
            .oauth_client
            .as_ref()
            .ok_or_else(|| AuthError::InternalError("OAuth client not configured".to_string()))?;

        let pkce_verifier = self
            .pkce_verifier
            .write()
            .await
            .take()
            .ok_or_else(|| AuthError::InternalError("PKCE verifier not found".to_string()))?;

        let http_client = reqwest::ClientBuilder::new()
            .redirect(reqwest::redirect::Policy::none())
            .build()
            .map_err(|e| AuthError::InternalError(e.to_string()))?;
        debug!("client_id: {:?}", oauth_client.client_id());

        // exchange token
        let token_result = oauth_client
            .exchange_code(AuthorizationCode::new(code.to_string()))
            .add_extra_param("client_id", oauth_client.client_id().to_string())
            .set_pkce_verifier(pkce_verifier)
            .request_async(&http_client)
            .await
            .map_err(|e| AuthError::TokenExchangeFailed(e.to_string()))?;

        // get expires_in from token response
        let expires_in = token_result.expires_in();
        if let Some(expires_in) = expires_in {
            let expires_at = Instant::now() + expires_in;
            *self.expires_at.write().await = Some(expires_at);
        }
        debug!("exchange token result: {:?}", token_result);
        // store credentials
        *self.credentials.write().await = Some(token_result.clone());

        Ok(token_result)
    }

    /// get access token, if expired, refresh it automatically
    pub async fn get_access_token(&self) -> Result<String, AuthError> {
        let credentials = self.credentials.read().await;

        if let Some(creds) = credentials.as_ref() {
            // check if the token is expire
            if let Some(expires_at) = *self.expires_at.read().await {
                if expires_at < Instant::now() {
                    // token expired, try to refresh , release the lock
                    drop(credentials);
                    let new_creds = self.refresh_token().await?;
                    return Ok(new_creds.access_token().secret().to_string());
                }
            }

            Ok(creds.access_token().secret().to_string())
        } else {
            Err(AuthError::AuthorizationRequired)
        }
    }

    /// refresh access token
    pub async fn refresh_token(
        &self,
    ) -> Result<StandardTokenResponse<EmptyExtraTokenFields, BasicTokenType>, AuthError> {
        let oauth_client = self
            .oauth_client
            .as_ref()
            .ok_or_else(|| AuthError::InternalError("OAuth client not configured".to_string()))?;

        let current_credentials = self
            .credentials
            .read()
            .await
            .clone()
            .ok_or_else(|| AuthError::AuthorizationRequired)?;

        let refresh_token = current_credentials.refresh_token().ok_or_else(|| {
            AuthError::TokenRefreshFailed("No refresh token available".to_string())
        })?;
        debug!("refresh token: {:?}", refresh_token);
        // refresh token
        let token_result = oauth_client
            .exchange_refresh_token(&RefreshToken::new(refresh_token.secret().to_string()))
            .request_async(&self.http_client)
            .await
            .map_err(|e| AuthError::TokenRefreshFailed(e.to_string()))?;

        // store new credentials
        *self.credentials.write().await = Some(token_result.clone());

        // get expires_in from token response
        let expires_in = token_result.expires_in();
        if let Some(expires_in) = expires_in {
            let expires_at = Instant::now() + expires_in;
            *self.expires_at.write().await = Some(expires_at);
        }
        Ok(token_result)
    }

    /// prepare request, add authorization header
    pub async fn prepare_request(
        &self,
        request: reqwest::RequestBuilder,
    ) -> Result<reqwest::RequestBuilder, AuthError> {
        let token = self.get_access_token().await?;
        Ok(request.header(AUTHORIZATION, format!("Bearer {}", token)))
    }

    /// handle response, check if need to re-authorize
    pub async fn handle_response(
        &self,
        response: reqwest::Response,
    ) -> Result<reqwest::Response, AuthError> {
        if response.status() == StatusCode::UNAUTHORIZED {
            // 401 Unauthorized, need to re-authorize
            Err(AuthError::AuthorizationRequired)
        } else {
            Ok(response)
        }
    }
}

/// oauth2 authorization session, for guiding user to complete the authorization process
pub struct AuthorizationSession {
    pub auth_manager: AuthorizationManager,
    pub auth_url: String,
    pub redirect_uri: String,
}

impl AuthorizationSession {
    /// create new authorization session
    pub async fn new(
        mut auth_manager: AuthorizationManager,
        scopes: &[&str],
        redirect_uri: &str,
    ) -> Result<Self, AuthError> {
        // set redirect uri
        let config = OAuthClientConfig {
            client_id: "mcp-client".to_string(), // temporary id, will be updated by dynamic registration
            client_secret: None,
            scopes: scopes.iter().map(|s| s.to_string()).collect(),
            redirect_uri: redirect_uri.to_string(),
        };

        // try to dynamic register client
        let config = match auth_manager
            .register_client("MCP Client", redirect_uri)
            .await
        {
            Ok(config) => config,
            Err(e) => {
                eprintln!("Dynamic registration failed: {}", e);
                // fallback to default config
                config
            }
        };
        // reset client config
        auth_manager.configure_client(config)?;
        let auth_url = auth_manager.get_authorization_url(scopes).await?;

        Ok(Self {
            auth_manager,
            auth_url,
            redirect_uri: redirect_uri.to_string(),
        })
    }

    /// get client_id and credentials
    pub async fn get_credentials(&self) -> Result<Credentials, AuthError> {
        self.auth_manager.get_credentials().await
    }

    /// get authorization url
    pub fn get_authorization_url(&self) -> &str {
        &self.auth_url
    }

    /// handle authorization code callback
    pub async fn handle_callback(
        &self,
        code: &str,
    ) -> Result<StandardTokenResponse<EmptyExtraTokenFields, BasicTokenType>, AuthError> {
        self.auth_manager.exchange_code_for_token(code).await
    }
}

/// http client extension, automatically add authorization header
pub struct AuthorizedHttpClient {
    auth_manager: Arc<AuthorizationManager>,
    inner_client: HttpClient,
}

impl AuthorizedHttpClient {
    /// create new authorized http client
    pub fn new(auth_manager: Arc<AuthorizationManager>, client: Option<HttpClient>) -> Self {
        let inner_client = client.unwrap_or_default();
        Self {
            auth_manager,
            inner_client,
        }
    }

    /// send authorized request
    pub async fn request<U: IntoUrl>(
        &self,
        method: reqwest::Method,
        url: U,
    ) -> Result<reqwest::RequestBuilder, AuthError> {
        let request = self.inner_client.request(method, url);
        self.auth_manager.prepare_request(request).await
    }

    /// send get request
    pub async fn get<U: IntoUrl>(&self, url: U) -> Result<reqwest::Response, AuthError> {
        let request = self.request(reqwest::Method::GET, url).await?;
        let response = request.send().await?;
        self.auth_manager.handle_response(response).await
    }

    /// send post request
    pub async fn post<U: IntoUrl>(&self, url: U) -> Result<reqwest::RequestBuilder, AuthError> {
        self.request(reqwest::Method::POST, url).await
    }
}

/// OAuth state machine
/// Use the OAuthState to manage the OAuth client is more recommend
/// But also you can use the AuthorizationManager,AuthorizationSession,AuthorizedHttpClient directly
pub enum OAuthState {
    /// the AuthorizationManager
    Unauthorized(AuthorizationManager),
    /// the AuthorizationSession
    Session(AuthorizationSession),
    /// the authd AuthorizationManager
    Authorized(AuthorizationManager),
    /// the authd http client
    AuthorizedHttpClient(AuthorizedHttpClient),
}

impl OAuthState {
    /// Create new OAuth state machine
    pub async fn new<U: IntoUrl>(
        base_url: U,
        client: Option<HttpClient>,
    ) -> Result<Self, AuthError> {
        let mut manager = AuthorizationManager::new(base_url).await?;
        if let Some(client) = client {
            manager.with_client(client)?;
        }

        Ok(OAuthState::Unauthorized(manager))
    }

    /// Get client_id and OAuth credentials
    pub async fn get_credentials(&self) -> Result<Credentials, AuthError> {
        // return client_id and credentials
        match self {
            OAuthState::Unauthorized(manager) | OAuthState::Authorized(manager) => {
                manager.get_credentials().await
            }
            OAuthState::Session(session) => session.get_credentials().await,
            OAuthState::AuthorizedHttpClient(client) => client.auth_manager.get_credentials().await,
        }
    }

    /// Manually set credentials and move into authorized state
    /// Useful if you're caching credentials externally and wish to reuse them
    pub async fn set_credentials(
        &mut self,
        client_id: &str,
        credentials: OAuthTokenResponse,
    ) -> Result<(), AuthError> {
        if let OAuthState::Unauthorized(manager) = self {
            let mut manager = std::mem::replace(
                manager,
                AuthorizationManager::new("http://localhost").await?,
            );

            // write credentials
            *manager.credentials.write().await = Some(credentials);

            // discover metadata
            let metadata = manager.discover_metadata().await?;
            manager.metadata = Some(metadata);

            // set client id and secret
            manager.configure_client_id(client_id)?;

            *self = OAuthState::Authorized(manager);
            Ok(())
        } else {
            Err(AuthError::InternalError(
                "Cannot set credentials in this state".to_string(),
            ))
        }
    }

    /// start authorization
    pub async fn start_authorization(
        &mut self,
        scopes: &[&str],
        redirect_uri: &str,
    ) -> Result<(), AuthError> {
        if let OAuthState::Unauthorized(mut manager) = std::mem::replace(
            self,
            OAuthState::Unauthorized(AuthorizationManager::new("http://localhost").await?),
        ) {
            debug!("start discovery");
            let metadata = manager.discover_metadata().await?;
            manager.metadata = Some(metadata);
            debug!("start session");
            let session = AuthorizationSession::new(manager, scopes, redirect_uri).await?;
            *self = OAuthState::Session(session);
            Ok(())
        } else {
            Err(AuthError::InternalError(
                "Already in session state".to_string(),
            ))
        }
    }

    /// complete authorization
    pub async fn complete_authorization(&mut self) -> Result<(), AuthError> {
        if let OAuthState::Session(session) = std::mem::replace(
            self,
            OAuthState::Unauthorized(AuthorizationManager::new("http://localhost").await?),
        ) {
            *self = OAuthState::Authorized(session.auth_manager);
            Ok(())
        } else {
            Err(AuthError::InternalError("Not in session state".to_string()))
        }
    }
    /// covert to authorized http client
    pub async fn to_authorized_http_client(&mut self) -> Result<(), AuthError> {
        if let OAuthState::Authorized(manager) = std::mem::replace(
            self,
            OAuthState::Authorized(AuthorizationManager::new("http://localhost").await?),
        ) {
            *self = OAuthState::AuthorizedHttpClient(AuthorizedHttpClient::new(
                Arc::new(manager),
                None,
            ));
            Ok(())
        } else {
            Err(AuthError::InternalError(
                "Not in authorized state".to_string(),
            ))
        }
    }
    /// get current authorization url
    pub async fn get_authorization_url(&self) -> Result<String, AuthError> {
        match self {
            OAuthState::Session(session) => Ok(session.get_authorization_url().to_string()),
            OAuthState::Unauthorized(_) => {
                Err(AuthError::InternalError("Not in session state".to_string()))
            }
            OAuthState::Authorized(_) => {
                Err(AuthError::InternalError("Already authorized".to_string()))
            }
            OAuthState::AuthorizedHttpClient(_) => {
                Err(AuthError::InternalError("Already authorized".to_string()))
            }
        }
    }

    /// handle authorization callback
    pub async fn handle_callback(&mut self, code: &str) -> Result<(), AuthError> {
        match self {
            OAuthState::Session(session) => {
                session.handle_callback(code).await?;
                self.complete_authorization().await
            }
            OAuthState::Unauthorized(_) => {
                Err(AuthError::InternalError("Not in session state".to_string()))
            }
            OAuthState::Authorized(_) => {
                Err(AuthError::InternalError("Already authorized".to_string()))
            }
            OAuthState::AuthorizedHttpClient(_) => {
                Err(AuthError::InternalError("Already authorized".to_string()))
            }
        }
    }

    /// get access token
    pub async fn get_access_token(&self) -> Result<String, AuthError> {
        match self {
            OAuthState::Unauthorized(manager) => manager.get_access_token().await,
            OAuthState::Session(_) => {
                Err(AuthError::InternalError("Not in manager state".to_string()))
            }
            OAuthState::Authorized(_) => {
                Err(AuthError::InternalError("Already authorized".to_string()))
            }
            OAuthState::AuthorizedHttpClient(_) => {
                Err(AuthError::InternalError("Already authorized".to_string()))
            }
        }
    }

    /// refresh access token
    pub async fn refresh_token(&self) -> Result<(), AuthError> {
        match self {
            OAuthState::Unauthorized(_) => {
                Err(AuthError::InternalError("Not in manager state".to_string()))
            }
            OAuthState::Session(_) => {
                Err(AuthError::InternalError("Not in manager state".to_string()))
            }
            OAuthState::Authorized(manager) => {
                manager.refresh_token().await?;
                Ok(())
            }
            OAuthState::AuthorizedHttpClient(_) => {
                Err(AuthError::InternalError("Already authorized".to_string()))
            }
        }
    }

    pub fn into_authorization_manager(self) -> Option<AuthorizationManager> {
        match self {
            OAuthState::Authorized(manager) => Some(manager),
            _ => None,
        }
    }
}

---
File: modelcontextprotocol-rust-sdk-13e72ec/crates/rmcp/src/transport/child_process.rs
---

use process_wrap::tokio::{TokioChildWrapper, TokioCommandWrap};
use tokio::{
    io::AsyncRead,
    process::{ChildStdin, ChildStdout},
};

use super::{IntoTransport, Transport};
use crate::service::ServiceRole;

pub(crate) fn child_process(
    mut child: Box<dyn TokioChildWrapper>,
) -> std::io::Result<(Box<dyn TokioChildWrapper>, (ChildStdout, ChildStdin))> {
    let child_stdin = match child.inner_mut().stdin().take() {
        Some(stdin) => stdin,
        None => return Err(std::io::Error::other("std in was taken")),
    };
    let child_stdout = match child.inner_mut().stdout().take() {
        Some(stdout) => stdout,
        None => return Err(std::io::Error::other("std out was taken")),
    };
    Ok((child, (child_stdout, child_stdin)))
}

pub struct TokioChildProcess {
    child: ChildWithCleanup,
    child_stdin: ChildStdin,
    child_stdout: ChildStdout,
}

pub struct ChildWithCleanup {
    inner: Box<dyn TokioChildWrapper>,
}

impl Drop for ChildWithCleanup {
    fn drop(&mut self) {
        if let Err(e) = self.inner.start_kill() {
            tracing::warn!("Failed to kill child process: {e}");
        }
    }
}

// we hold the child process with stdout, for it's easier to implement AsyncRead
pin_project_lite::pin_project! {
    pub struct TokioChildProcessOut {
        child: ChildWithCleanup,
        #[pin]
        child_stdout: ChildStdout,
    }
}

impl TokioChildProcessOut {
    /// Get the process ID of the child process.
    pub fn id(&self) -> Option<u32> {
        self.child.inner.id()
    }
}

impl AsyncRead for TokioChildProcessOut {
    fn poll_read(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        buf: &mut tokio::io::ReadBuf<'_>,
    ) -> std::task::Poll<std::io::Result<()>> {
        self.project().child_stdout.poll_read(cx, buf)
    }
}

impl TokioChildProcess {
    pub fn new(command: impl Into<TokioCommandWrap>) -> std::io::Result<Self> {
        let mut command_wrap = command.into();
        command_wrap
            .command_mut()
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped());
        #[cfg(unix)]
        command_wrap.wrap(process_wrap::tokio::ProcessGroup::leader());
        #[cfg(windows)]
        command_wrap.wrap(process_wrap::tokio::JobObject);
        let (child, (child_stdout, child_stdin)) = child_process(command_wrap.spawn()?)?;
        Ok(Self {
            child: ChildWithCleanup { inner: child },
            child_stdin,
            child_stdout,
        })
    }

    /// Get the process ID of the child process.
    pub fn id(&self) -> Option<u32> {
        self.child.inner.id()
    }

    pub fn split(self) -> (TokioChildProcessOut, ChildStdin) {
        let TokioChildProcess {
            child,
            child_stdin,
            child_stdout,
        } = self;
        (
            TokioChildProcessOut {
                child,
                child_stdout,
            },
            child_stdin,
        )
    }
}

impl<R: ServiceRole> IntoTransport<R, std::io::Error, ()> for TokioChildProcess {
    fn into_transport(self) -> impl Transport<R, Error = std::io::Error> + 'static {
        IntoTransport::<R, std::io::Error, super::async_rw::TransportAdapterAsyncRW>::into_transport(
            self.split(),
        )
    }
}

pub trait ConfigureCommandExt {
    fn configure(self, f: impl FnOnce(&mut Self)) -> Self;
}

impl ConfigureCommandExt for tokio::process::Command {
    fn configure(mut self, f: impl FnOnce(&mut Self)) -> Self {
        f(&mut self);
        self
    }
}

---
File: modelcontextprotocol-rust-sdk-13e72ec/crates/rmcp/src/transport/common.rs
---

#[cfg(any(
    feature = "transport-streamable-http-server",
    feature = "transport-sse-server"
))]
pub mod server_side_http;

pub mod http_header;

#[cfg(feature = "__reqwest")]
#[cfg_attr(docsrs, doc(cfg(feature = "reqwest")))]
mod reqwest;

#[cfg(feature = "client-side-sse")]
#[cfg_attr(docsrs, doc(cfg(feature = "client-side-sse")))]
pub mod client_side_sse;

#[cfg(feature = "auth")]
#[cfg_attr(docsrs, doc(cfg(feature = "auth")))]
pub mod auth;

---
File: modelcontextprotocol-rust-sdk-13e72ec/crates/rmcp/src/transport/common/auth.rs
---

#[cfg(feature = "transport-streamable-http-client")]
#[cfg_attr(docsrs, doc(cfg(feature = "transport-streamable-http-client")))]
mod streamable_http_client;

#[cfg(feature = "transport-sse-client")]
#[cfg_attr(docsrs, doc(cfg(feature = "transport-sse-client")))]
mod sse_client;

---
File: modelcontextprotocol-rust-sdk-13e72ec/crates/rmcp/src/transport/common/auth/sse_client.rs
---

use http::Uri;

use crate::transport::{
    auth::AuthClient,
    sse_client::{SseClient, SseTransportError},
};
impl<C> SseClient for AuthClient<C>
where
    C: SseClient,
{
    type Error = SseTransportError<C::Error>;

    async fn post_message(
        &self,
        uri: Uri,
        message: crate::model::ClientJsonRpcMessage,
        mut auth_token: Option<String>,
    ) -> Result<(), SseTransportError<Self::Error>> {
        if auth_token.is_none() {
            auth_token = Some(self.get_access_token().await?);
        }
        self.http_client
            .post_message(uri, message, auth_token)
            .await
            .map_err(SseTransportError::Client)
    }

    async fn get_stream(
        &self,
        uri: Uri,
        last_event_id: Option<String>,
        mut auth_token: Option<String>,
    ) -> Result<
        crate::transport::common::client_side_sse::BoxedSseResponse,
        SseTransportError<Self::Error>,
    > {
        if auth_token.is_none() {
            auth_token = Some(self.get_access_token().await?);
        }
        self.http_client
            .get_stream(uri, last_event_id, auth_token)
            .await
            .map_err(SseTransportError::Client)
    }
}

---
File: modelcontextprotocol-rust-sdk-13e72ec/crates/rmcp/src/transport/common/auth/streamable_http_client.rs
---

use crate::transport::{
    auth::AuthClient,
    streamable_http_client::{StreamableHttpClient, StreamableHttpError},
};
impl<C> StreamableHttpClient for AuthClient<C>
where
    C: StreamableHttpClient + Send + Sync,
{
    type Error = StreamableHttpError<C::Error>;

    async fn delete_session(
        &self,
        uri: std::sync::Arc<str>,
        session_id: std::sync::Arc<str>,
        mut auth_token: Option<String>,
    ) -> Result<(), crate::transport::streamable_http_client::StreamableHttpError<Self::Error>>
    {
        if auth_token.is_none() {
            auth_token = Some(self.get_access_token().await?);
        }
        self.http_client
            .delete_session(uri, session_id, auth_token)
            .await
            .map_err(StreamableHttpError::Client)
    }

    async fn get_stream(
        &self,
        uri: std::sync::Arc<str>,
        session_id: std::sync::Arc<str>,
        last_event_id: Option<String>,
        mut auth_token: Option<String>,
    ) -> Result<
        futures::stream::BoxStream<'static, Result<sse_stream::Sse, sse_stream::Error>>,
        crate::transport::streamable_http_client::StreamableHttpError<Self::Error>,
    > {
        if auth_token.is_none() {
            auth_token = Some(self.get_access_token().await?);
        }
        self.http_client
            .get_stream(uri, session_id, last_event_id, auth_token)
            .await
            .map_err(StreamableHttpError::Client)
    }

    async fn post_message(
        &self,
        uri: std::sync::Arc<str>,
        message: crate::model::ClientJsonRpcMessage,
        session_id: Option<std::sync::Arc<str>>,
        mut auth_token: Option<String>,
    ) -> Result<
        crate::transport::streamable_http_client::StreamableHttpPostResponse,
        StreamableHttpError<Self::Error>,
    > {
        if auth_token.is_none() {
            auth_token = Some(self.get_access_token().await?);
        }
        self.http_client
            .post_message(uri, message, session_id, auth_token)
            .await
            .map_err(StreamableHttpError::Client)
    }
}

---
File: modelcontextprotocol-rust-sdk-13e72ec/crates/rmcp/src/transport/common/client_side_sse.rs
---

use std::{
    pin::Pin,
    sync::Arc,
    task::{Poll, ready},
    time::Duration,
};

use futures::{Stream, stream::BoxStream};
use sse_stream::{Error as SseError, Sse};

use crate::model::ServerJsonRpcMessage;

pub type BoxedSseResponse = BoxStream<'static, Result<Sse, SseError>>;

pub trait SseRetryPolicy: std::fmt::Debug + Send + Sync {
    fn retry(&self, current_times: usize) -> Option<Duration>;
}

#[derive(Debug, Clone)]
pub struct FixedInterval {
    pub max_times: Option<usize>,
    pub duration: Duration,
}

impl SseRetryPolicy for FixedInterval {
    fn retry(&self, current_times: usize) -> Option<Duration> {
        if let Some(max_times) = self.max_times {
            if current_times >= max_times {
                return None;
            }
        }
        Some(self.duration)
    }
}

impl FixedInterval {
    pub const DEFAULT_MIN_DURATION: Duration = Duration::from_millis(1000);
}

impl Default for FixedInterval {
    fn default() -> Self {
        Self {
            max_times: None,
            duration: Self::DEFAULT_MIN_DURATION,
        }
    }
}

#[derive(Debug, Clone)]
pub struct ExponentialBackoff {
    pub max_times: Option<usize>,
    pub base_duration: Duration,
}

impl ExponentialBackoff {
    pub const DEFAULT_DURATION: Duration = Duration::from_millis(1000);
}

impl Default for ExponentialBackoff {
    fn default() -> Self {
        Self {
            max_times: None,
            base_duration: Self::DEFAULT_DURATION,
        }
    }
}

impl SseRetryPolicy for ExponentialBackoff {
    fn retry(&self, current_times: usize) -> Option<Duration> {
        if let Some(max_times) = self.max_times {
            if current_times >= max_times {
                return None;
            }
        }
        Some(self.base_duration * (2u32.pow(current_times as u32)))
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub struct NeverRetry;

impl SseRetryPolicy for NeverRetry {
    fn retry(&self, _current_times: usize) -> Option<Duration> {
        None
    }
}

#[derive(Debug, Default)]
pub struct NeverReconnect<E> {
    error: Option<E>,
}

impl<E: std::error::Error + Send> SseStreamReconnect for NeverReconnect<E> {
    type Error = E;
    type Future = futures::future::Ready<Result<BoxedSseResponse, Self::Error>>;
    fn retry_connection(&mut self, _last_event_id: Option<&str>) -> Self::Future {
        futures::future::ready(Err(self.error.take().expect("should not be called again")))
    }
}

pub(crate) trait SseStreamReconnect {
    type Error: std::error::Error;
    type Future: Future<Output = Result<BoxedSseResponse, Self::Error>> + Send;
    fn retry_connection(&mut self, last_event_id: Option<&str>) -> Self::Future;
}

pin_project_lite::pin_project! {
    pub(crate) struct SseAutoReconnectStream<R>
    where R: SseStreamReconnect
     {
        retry_policy: Arc<dyn SseRetryPolicy>,
        last_event_id: Option<String>,
        server_retry_interval: Option<Duration>,
        connector: R,
        #[pin]
        state: SseAutoReconnectStreamState<R::Future>,
    }
}

impl<R: SseStreamReconnect> SseAutoReconnectStream<R> {
    pub fn new(
        stream: BoxedSseResponse,
        connector: R,
        retry_policy: Arc<dyn SseRetryPolicy>,
    ) -> Self {
        Self {
            retry_policy,
            last_event_id: None,
            server_retry_interval: None,
            connector,
            state: SseAutoReconnectStreamState::Connected { stream },
        }
    }
}

impl<E: std::error::Error + Send> SseAutoReconnectStream<NeverReconnect<E>> {
    pub fn never_reconnect(stream: BoxedSseResponse, error_when_reconnect: E) -> Self {
        Self {
            retry_policy: Arc::new(NeverRetry),
            last_event_id: None,
            server_retry_interval: None,
            connector: NeverReconnect {
                error: Some(error_when_reconnect),
            },
            state: SseAutoReconnectStreamState::Connected { stream },
        }
    }
}

pin_project_lite::pin_project! {
    #[project = SseAutoReconnectStreamStateProj]
    pub enum SseAutoReconnectStreamState<F> {
        Connected {
            #[pin]
            stream: BoxedSseResponse,
        },
        Retrying {
            retry_times: usize,
            #[pin]
            retrying: F,
        },
        WaitingNextRetry {
            #[pin]
            sleep: tokio::time::Sleep,
            retry_times: usize,
        },
        Terminated,
    }
}

impl<R> Stream for SseAutoReconnectStream<R>
where
    R: SseStreamReconnect,
{
    type Item = Result<ServerJsonRpcMessage, R::Error>;
    fn poll_next(
        mut self: Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> Poll<Option<Self::Item>> {
        let mut this = self.as_mut().project();
        // let this_state = this.state.as_mut().project()
        let state = this.state.as_mut().project();
        let next_state = match state {
            SseAutoReconnectStreamStateProj::Connected { stream } => {
                match ready!(stream.poll_next(cx)) {
                    Some(Ok(sse)) => {
                        if let Some(new_server_retry) = sse.retry {
                            *this.server_retry_interval =
                                Some(Duration::from_millis(new_server_retry));
                        }
                        if let Some(event_id) = sse.id {
                            *this.last_event_id = Some(event_id);
                        }
                        if let Some(data) = sse.data {
                            match serde_json::from_str::<ServerJsonRpcMessage>(&data) {
                                Err(e) => {
                                    // not sure should this be a hard error
                                    tracing::warn!("failed to deserialize server message: {e}");
                                    return self.poll_next(cx);
                                }
                                Ok(message) => {
                                    return Poll::Ready(Some(Ok(message)));
                                }
                            };
                        } else {
                            return self.poll_next(cx);
                        }
                    }
                    Some(Err(e)) => {
                        tracing::warn!("sse stream error: {e}");
                        let retrying = this
                            .connector
                            .retry_connection(this.last_event_id.as_deref());
                        SseAutoReconnectStreamState::Retrying {
                            retry_times: 0,
                            retrying,
                        }
                    }
                    None => {
                        tracing::debug!("sse stream terminated");
                        return Poll::Ready(None);
                    }
                }
            }
            SseAutoReconnectStreamStateProj::Retrying {
                retry_times,
                retrying,
            } => {
                let retry_result = ready!(retrying.poll(cx));
                match retry_result {
                    Ok(new_stream) => SseAutoReconnectStreamState::Connected { stream: new_stream },
                    Err(e) => {
                        tracing::debug!("retry sse stream error: {e}");
                        *retry_times += 1;
                        if let Some(interval) = this.retry_policy.retry(*retry_times) {
                            let interval = this
                                .server_retry_interval
                                .map(|server_retry_interval| server_retry_interval.max(interval))
                                .unwrap_or(interval);
                            let sleep = tokio::time::sleep(interval);
                            SseAutoReconnectStreamState::WaitingNextRetry {
                                sleep,
                                retry_times: *retry_times,
                            }
                        } else {
                            tracing::error!("sse stream error: {e}, max retry times reached");
                            this.state.set(SseAutoReconnectStreamState::Terminated);
                            return Poll::Ready(Some(Err(e)));
                        }
                    }
                }
            }
            SseAutoReconnectStreamStateProj::WaitingNextRetry { sleep, retry_times } => {
                ready!(sleep.poll(cx));
                let retrying = this
                    .connector
                    .retry_connection(this.last_event_id.as_deref());
                let retry_times = *retry_times;
                SseAutoReconnectStreamState::Retrying {
                    retry_times,
                    retrying,
                }
            }
            SseAutoReconnectStreamStateProj::Terminated => {
                return Poll::Ready(None);
            }
        };
        // update the state
        this.state.set(next_state);
        self.poll_next(cx)
    }
}

---
File: modelcontextprotocol-rust-sdk-13e72ec/crates/rmcp/src/transport/common/http_header.rs
---

pub const HEADER_SESSION_ID: &str = "Mcp-Session-Id";
pub const HEADER_LAST_EVENT_ID: &str = "Last-Event-Id";
pub const EVENT_STREAM_MIME_TYPE: &str = "text/event-stream";
pub const JSON_MIME_TYPE: &str = "application/json";

---
File: modelcontextprotocol-rust-sdk-13e72ec/crates/rmcp/src/transport/common/reqwest.rs
---

#[cfg(feature = "transport-streamable-http-client")]
#[cfg_attr(docsrs, doc(cfg(feature = "transport-streamable-http-client")))]
mod streamable_http_client;

#[cfg(feature = "transport-sse-client")]
#[cfg_attr(docsrs, doc(cfg(feature = "transport-sse-client")))]
mod sse_client;

---
File: modelcontextprotocol-rust-sdk-13e72ec/crates/rmcp/src/transport/common/reqwest/sse_client.rs
---

use std::sync::Arc;

use futures::StreamExt;
use http::Uri;
use reqwest::header::ACCEPT;
use sse_stream::SseStream;

use crate::transport::{
    SseClientTransport,
    common::http_header::{EVENT_STREAM_MIME_TYPE, HEADER_LAST_EVENT_ID},
    sse_client::{SseClient, SseClientConfig, SseTransportError},
};

impl SseClient for reqwest::Client {
    type Error = reqwest::Error;

    async fn post_message(
        &self,
        uri: Uri,
        message: crate::model::ClientJsonRpcMessage,
        auth_token: Option<String>,
    ) -> Result<(), SseTransportError<Self::Error>> {
        let mut request_builder = self.post(uri.to_string()).json(&message);
        if let Some(auth_header) = auth_token {
            request_builder = request_builder.bearer_auth(auth_header);
        }
        request_builder
            .send()
            .await
            .and_then(|resp| resp.error_for_status())
            .map_err(SseTransportError::from)
            .map(drop)
    }

    async fn get_stream(
        &self,
        uri: Uri,
        last_event_id: Option<String>,
        auth_token: Option<String>,
    ) -> Result<
        crate::transport::common::client_side_sse::BoxedSseResponse,
        SseTransportError<Self::Error>,
    > {
        let mut request_builder = self
            .get(uri.to_string())
            .header(ACCEPT, EVENT_STREAM_MIME_TYPE);
        if let Some(auth_header) = auth_token {
            request_builder = request_builder.bearer_auth(auth_header);
        }
        if let Some(last_event_id) = last_event_id {
            request_builder = request_builder.header(HEADER_LAST_EVENT_ID, last_event_id);
        }
        let response = request_builder.send().await?;
        let response = response.error_for_status()?;
        match response.headers().get(reqwest::header::CONTENT_TYPE) {
            Some(ct) => {
                if !ct.as_bytes().starts_with(EVENT_STREAM_MIME_TYPE.as_bytes()) {
                    return Err(SseTransportError::UnexpectedContentType(Some(ct.clone())));
                }
            }
            None => {
                return Err(SseTransportError::UnexpectedContentType(None));
            }
        }
        let event_stream = SseStream::from_byte_stream(response.bytes_stream()).boxed();
        Ok(event_stream)
    }
}

impl SseClientTransport<reqwest::Client> {
    pub async fn start(
        uri: impl Into<Arc<str>>,
    ) -> Result<Self, SseTransportError<reqwest::Error>> {
        SseClientTransport::start_with_client(
            reqwest::Client::default(),
            SseClientConfig {
                sse_endpoint: uri.into(),
                ..Default::default()
            },
        )
        .await
    }
}

---
File: modelcontextprotocol-rust-sdk-13e72ec/crates/rmcp/src/transport/common/reqwest/streamable_http_client.rs
---

use std::sync::Arc;

use futures::{StreamExt, stream::BoxStream};
use reqwest::header::ACCEPT;
use sse_stream::{Sse, SseStream};

use crate::{
    model::{ClientJsonRpcMessage, ServerJsonRpcMessage},
    transport::{
        common::http_header::{
            EVENT_STREAM_MIME_TYPE, HEADER_LAST_EVENT_ID, HEADER_SESSION_ID, JSON_MIME_TYPE,
        },
        streamable_http_client::*,
    },
};

impl StreamableHttpClient for reqwest::Client {
    type Error = reqwest::Error;

    async fn get_stream(
        &self,
        uri: Arc<str>,
        session_id: Arc<str>,
        last_event_id: Option<String>,
        auth_token: Option<String>,
    ) -> Result<BoxStream<'static, Result<Sse, SseError>>, StreamableHttpError<Self::Error>> {
        let mut request_builder = self
            .get(uri.as_ref())
            .header(ACCEPT, EVENT_STREAM_MIME_TYPE)
            .header(HEADER_SESSION_ID, session_id.as_ref());
        if let Some(last_event_id) = last_event_id {
            request_builder = request_builder.header(HEADER_LAST_EVENT_ID, last_event_id);
        }
        if let Some(auth_header) = auth_token {
            request_builder = request_builder.bearer_auth(auth_header);
        }
        let response = request_builder.send().await?;
        if response.status() == reqwest::StatusCode::METHOD_NOT_ALLOWED {
            return Err(StreamableHttpError::SeverDoesNotSupportSse);
        }
        let response = response.error_for_status()?;
        match response.headers().get(reqwest::header::CONTENT_TYPE) {
            Some(ct) => {
                if !ct.as_bytes().starts_with(EVENT_STREAM_MIME_TYPE.as_bytes()) {
                    return Err(StreamableHttpError::UnexpectedContentType(Some(
                        String::from_utf8_lossy(ct.as_bytes()).to_string(),
                    )));
                }
            }
            None => {
                return Err(StreamableHttpError::UnexpectedContentType(None));
            }
        }
        let event_stream = SseStream::from_byte_stream(response.bytes_stream()).boxed();
        Ok(event_stream)
    }

    async fn delete_session(
        &self,
        uri: Arc<str>,
        session: Arc<str>,
        auth_token: Option<String>,
    ) -> Result<(), StreamableHttpError<Self::Error>> {
        let mut request_builder = self.delete(uri.as_ref());
        if let Some(auth_header) = auth_token {
            request_builder = request_builder.bearer_auth(auth_header);
        }
        let response = request_builder
            .header(HEADER_SESSION_ID, session.as_ref())
            .send()
            .await?;

        // if method no allowed
        if response.status() == reqwest::StatusCode::METHOD_NOT_ALLOWED {
            tracing::debug!("this server doesn't support deleting session");
            return Ok(());
        }
        let _response = response.error_for_status()?;
        Ok(())
    }

    async fn post_message(
        &self,
        uri: Arc<str>,
        message: ClientJsonRpcMessage,
        session_id: Option<Arc<str>>,
        auth_token: Option<String>,
    ) -> Result<StreamableHttpPostResponse, StreamableHttpError<Self::Error>> {
        let mut request = self
            .post(uri.as_ref())
            .header(ACCEPT, [EVENT_STREAM_MIME_TYPE, JSON_MIME_TYPE].join(", "));
        if let Some(auth_header) = auth_token {
            request = request.bearer_auth(auth_header);
        }
        if let Some(session_id) = session_id {
            request = request.header(HEADER_SESSION_ID, session_id.as_ref());
        }
        let response = request.json(&message).send().await?.error_for_status()?;
        if response.status() == reqwest::StatusCode::ACCEPTED {
            return Ok(StreamableHttpPostResponse::Accepted);
        }
        let content_type = response.headers().get(reqwest::header::CONTENT_TYPE);
        let session_id = response.headers().get(HEADER_SESSION_ID);
        let session_id = session_id
            .and_then(|v| v.to_str().ok())
            .map(|s| s.to_string());
        match content_type {
            Some(ct) if ct.as_bytes().starts_with(EVENT_STREAM_MIME_TYPE.as_bytes()) => {
                let event_stream = SseStream::from_byte_stream(response.bytes_stream()).boxed();
                Ok(StreamableHttpPostResponse::Sse(event_stream, session_id))
            }
            Some(ct) if ct.as_bytes().starts_with(JSON_MIME_TYPE.as_bytes()) => {
                let message: ServerJsonRpcMessage = response.json().await?;
                Ok(StreamableHttpPostResponse::Json(message, session_id))
            }
            _ => {
                // unexpected content type
                tracing::error!("unexpected content type: {:?}", content_type);
                Err(StreamableHttpError::UnexpectedContentType(
                    content_type.map(|ct| String::from_utf8_lossy(ct.as_bytes()).to_string()),
                ))
            }
        }
    }
}

impl StreamableHttpClientTransport<reqwest::Client> {
    pub fn from_uri(uri: impl Into<Arc<str>>) -> Self {
        StreamableHttpClientTransport::with_client(
            reqwest::Client::default(),
            StreamableHttpClientTransportConfig {
                uri: uri.into(),
                ..Default::default()
            },
        )
    }
}

---
File: modelcontextprotocol-rust-sdk-13e72ec/crates/rmcp/src/transport/common/server_side_http.rs
---

use std::{convert::Infallible, fmt::Display, sync::Arc, time::Duration};

use bytes::{Buf, Bytes};
use http::Response;
use http_body::Body;
use http_body_util::{BodyExt, Empty, Full, combinators::UnsyncBoxBody};
use sse_stream::{KeepAlive, Sse, SseBody};

use super::http_header::EVENT_STREAM_MIME_TYPE;
use crate::model::{ClientJsonRpcMessage, ServerJsonRpcMessage};

pub type SessionId = Arc<str>;

pub fn session_id() -> SessionId {
    uuid::Uuid::new_v4().to_string().into()
}

pub const DEFAULT_AUTO_PING_INTERVAL: Duration = Duration::from_secs(15);

pub(crate) type BoxResponse = Response<UnsyncBoxBody<Bytes, Infallible>>;

pub(crate) fn accepted_response() -> Response<UnsyncBoxBody<Bytes, Infallible>> {
    Response::builder()
        .status(http::StatusCode::ACCEPTED)
        .body(Empty::new().boxed_unsync())
        .expect("valid response")
}
pin_project_lite::pin_project! {
    struct TokioTimer {
        #[pin]
        sleep: tokio::time::Sleep,
    }
}
impl Future for TokioTimer {
    type Output = ();

    fn poll(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Self::Output> {
        let this = self.project();
        this.sleep.poll(cx)
    }
}
impl sse_stream::Timer for TokioTimer {
    fn from_duration(duration: Duration) -> Self {
        Self {
            sleep: tokio::time::sleep(duration),
        }
    }

    fn reset(self: std::pin::Pin<&mut Self>, when: std::time::Instant) {
        let this = self.project();
        this.sleep.reset(tokio::time::Instant::from_std(when));
    }
}

#[derive(Debug, Clone)]
pub struct ServerSseMessage {
    pub event_id: Option<String>,
    pub message: Arc<ServerJsonRpcMessage>,
}

pub(crate) fn sse_stream_response(
    stream: impl futures::Stream<Item = ServerSseMessage> + Send + 'static,
    keep_alive: Option<Duration>,
) -> Response<UnsyncBoxBody<Bytes, Infallible>> {
    use futures::StreamExt;
    let stream = SseBody::new(stream.map(|message| {
        let data = serde_json::to_string(&message.message).expect("valid message");
        let mut sse = Sse::default().data(data);
        sse.id = message.event_id;
        Result::<Sse, Infallible>::Ok(sse)
    }));
    let stream = match keep_alive {
        Some(duration) => stream
            .with_keep_alive::<TokioTimer>(KeepAlive::new().interval(duration))
            .boxed_unsync(),
        None => stream.boxed_unsync(),
    };
    Response::builder()
        .status(http::StatusCode::OK)
        .header(http::header::CONTENT_TYPE, EVENT_STREAM_MIME_TYPE)
        .header(http::header::CACHE_CONTROL, "no-cache")
        .body(stream)
        .expect("valid response")
}

pub(crate) const fn internal_error_response<E: Display>(
    context: &str,
) -> impl FnOnce(E) -> Response<UnsyncBoxBody<Bytes, Infallible>> {
    move |error| {
        tracing::error!("Internal server error when {context}: {error}");
        Response::builder()
            .status(http::StatusCode::INTERNAL_SERVER_ERROR)
            .body(
                Full::new(Bytes::from(format!(
                    "Encounter an error when {context}: {error}"
                )))
                .boxed_unsync(),
            )
            .expect("valid response")
    }
}

pub(crate) async fn expect_json<B>(
    body: B,
) -> Result<ClientJsonRpcMessage, Response<UnsyncBoxBody<Bytes, Infallible>>>
where
    B: Body + Send + 'static,
    B::Error: Display,
{
    match body.collect().await {
        Ok(bytes) => {
            match serde_json::from_reader::<_, ClientJsonRpcMessage>(bytes.aggregate().reader()) {
                Ok(message) => Ok(message),
                Err(e) => {
                    let response = Response::builder()
                        .status(http::StatusCode::UNSUPPORTED_MEDIA_TYPE)
                        .body(
                            Full::new(Bytes::from(format!("fail to deserialize request body {e}")))
                                .boxed_unsync(),
                        )
                        .expect("valid response");
                    Err(response)
                }
            }
        }
        Err(e) => {
            let response = Response::builder()
                .status(http::StatusCode::INTERNAL_SERVER_ERROR)
                .body(
                    Full::new(Bytes::from(format!("Failed to read request body: {e}")))
                        .boxed_unsync(),
                )
                .expect("valid response");
            Err(response)
        }
    }
}

---
File: modelcontextprotocol-rust-sdk-13e72ec/crates/rmcp/src/transport/io.rs
---

/// # StdIO Transport
///
/// Create a pair of [`tokio::io::Stdin`] and [`tokio::io::Stdout`].
pub fn stdio() -> (tokio::io::Stdin, tokio::io::Stdout) {
    (tokio::io::stdin(), tokio::io::stdout())
}

---
File: modelcontextprotocol-rust-sdk-13e72ec/crates/rmcp/src/transport/sink_stream.rs
---

use std::sync::Arc;

use futures::{Sink, Stream};
use tokio::sync::Mutex;

use super::{IntoTransport, Transport};
use crate::service::{RxJsonRpcMessage, ServiceRole, TxJsonRpcMessage};

pub struct SinkStreamTransport<Si, St> {
    stream: St,
    sink: Arc<Mutex<Si>>,
}

impl<Si, St> SinkStreamTransport<Si, St> {
    pub fn new(sink: Si, stream: St) -> Self {
        Self {
            stream,
            sink: Arc::new(Mutex::new(sink)),
        }
    }
}

impl<Role: ServiceRole, Si, St> Transport<Role> for SinkStreamTransport<Si, St>
where
    St: Send + Stream<Item = RxJsonRpcMessage<Role>> + Unpin,
    Si: Send + Sink<TxJsonRpcMessage<Role>> + Unpin + 'static,
    Si::Error: std::error::Error + Send + Sync + 'static,
{
    type Error = Si::Error;

    fn send(
        &mut self,
        item: TxJsonRpcMessage<Role>,
    ) -> impl Future<Output = Result<(), Self::Error>> + Send + 'static {
        use futures::SinkExt;
        let lock = self.sink.clone();
        async move {
            let mut write = lock.lock().await;
            write.send(item).await
        }
    }

    fn receive(&mut self) -> impl Future<Output = Option<RxJsonRpcMessage<Role>>> {
        use futures::StreamExt;
        self.stream.next()
    }

    async fn close(&mut self) -> Result<(), Self::Error> {
        Ok(())
    }
}

pub enum TransportAdapterSinkStream {}

impl<Role, Si, St> IntoTransport<Role, Si::Error, TransportAdapterSinkStream> for (Si, St)
where
    Role: ServiceRole,
    Si: Send + Sink<TxJsonRpcMessage<Role>> + Unpin + 'static,
    St: Send + Stream<Item = RxJsonRpcMessage<Role>> + Unpin + 'static,
    Si::Error: std::error::Error + Send + Sync + 'static,
{
    fn into_transport(self) -> impl Transport<Role, Error = Si::Error> + 'static {
        SinkStreamTransport::new(self.0, self.1)
    }
}

pub enum TransportAdapterAsyncCombinedRW {}
impl<Role, S> IntoTransport<Role, S::Error, TransportAdapterAsyncCombinedRW> for S
where
    Role: ServiceRole,
    S: Sink<TxJsonRpcMessage<Role>> + Stream<Item = RxJsonRpcMessage<Role>> + Send + 'static,
    S::Error: std::error::Error + Send + Sync + 'static,
{
    fn into_transport(self) -> impl Transport<Role, Error = S::Error> + 'static {
        use futures::StreamExt;
        IntoTransport::<Role, S::Error, TransportAdapterSinkStream>::into_transport(self.split())
    }
}

---
File: modelcontextprotocol-rust-sdk-13e72ec/crates/rmcp/src/transport/sse_client.rs
---

//ÔºÅ reference: https://html.spec.whatwg.org/multipage/server-sent-events.html
use std::{pin::Pin, sync::Arc};

use futures::{StreamExt, future::BoxFuture};
use http::Uri;
use reqwest::header::HeaderValue;
use sse_stream::Error as SseError;
use thiserror::Error;

use super::{
    Transport,
    common::client_side_sse::{BoxedSseResponse, SseRetryPolicy, SseStreamReconnect},
};
use crate::{
    RoleClient,
    model::{ClientJsonRpcMessage, ServerJsonRpcMessage},
    transport::common::client_side_sse::SseAutoReconnectStream,
};

#[derive(Error, Debug)]
pub enum SseTransportError<E: std::error::Error + Send + Sync + 'static> {
    #[error("SSE error: {0}")]
    Sse(#[from] SseError),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Client error: {0}")]
    Client(E),
    #[error("unexpected end of stream")]
    UnexpectedEndOfStream,
    #[error("Unexpected content type: {0:?}")]
    UnexpectedContentType(Option<HeaderValue>),
    #[cfg(feature = "auth")]
    #[cfg_attr(docsrs, doc(cfg(feature = "auth")))]
    #[error("Auth error: {0}")]
    Auth(#[from] crate::transport::auth::AuthError),
    #[error("Invalid uri: {0}")]
    InvalidUri(#[from] http::uri::InvalidUri),
    #[error("Invalid uri parts: {0}")]
    InvalidUriParts(#[from] http::uri::InvalidUriParts),
}

impl From<reqwest::Error> for SseTransportError<reqwest::Error> {
    fn from(e: reqwest::Error) -> Self {
        SseTransportError::Client(e)
    }
}

pub trait SseClient: Clone + Send + Sync + 'static {
    type Error: std::error::Error + Send + Sync + 'static;
    fn post_message(
        &self,
        uri: Uri,
        message: ClientJsonRpcMessage,
        auth_token: Option<String>,
    ) -> impl Future<Output = Result<(), SseTransportError<Self::Error>>> + Send + '_;
    fn get_stream(
        &self,
        uri: Uri,
        last_event_id: Option<String>,
        auth_token: Option<String>,
    ) -> impl Future<Output = Result<BoxedSseResponse, SseTransportError<Self::Error>>> + Send + '_;
}

struct SseClientReconnect<C> {
    pub client: C,
    pub uri: Uri,
}

impl<C: SseClient> SseStreamReconnect for SseClientReconnect<C> {
    type Error = SseTransportError<C::Error>;
    type Future = BoxFuture<'static, Result<BoxedSseResponse, Self::Error>>;
    fn retry_connection(&mut self, last_event_id: Option<&str>) -> Self::Future {
        let client = self.client.clone();
        let uri = self.uri.clone();
        let last_event_id = last_event_id.map(|s| s.to_owned());
        Box::pin(async move { client.get_stream(uri, last_event_id, None).await })
    }
}
type ServerMessageStream<C> = Pin<Box<SseAutoReconnectStream<SseClientReconnect<C>>>>;
pub struct SseClientTransport<C: SseClient> {
    client: C,
    config: SseClientConfig,
    message_endpoint: Uri,
    stream: Option<ServerMessageStream<C>>,
}

impl<C: SseClient> Transport<RoleClient> for SseClientTransport<C> {
    type Error = SseTransportError<C::Error>;
    async fn receive(&mut self) -> Option<ServerJsonRpcMessage> {
        self.stream.as_mut()?.next().await?.ok()
    }
    fn send(
        &mut self,
        item: crate::service::TxJsonRpcMessage<RoleClient>,
    ) -> impl Future<Output = Result<(), Self::Error>> + Send + 'static {
        let client = self.client.clone();
        let uri = self.message_endpoint.clone();
        async move { client.post_message(uri, item, None).await }
    }
    async fn close(&mut self) -> Result<(), Self::Error> {
        self.stream.take();
        Ok(())
    }
}

impl<C: SseClient + std::fmt::Debug> std::fmt::Debug for SseClientTransport<C> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SseClientWorker")
            .field("client", &self.client)
            .field("config", &self.config)
            .finish()
    }
}

impl<C: SseClient> SseClientTransport<C> {
    pub async fn start_with_client(
        client: C,
        config: SseClientConfig,
    ) -> Result<Self, SseTransportError<C::Error>> {
        let sse_endpoint = config.sse_endpoint.as_ref().parse::<http::Uri>()?;

        let mut sse_stream = client.get_stream(sse_endpoint.clone(), None, None).await?;
        let message_endpoint = if let Some(endpoint) = config.use_message_endpoint.clone() {
            let ep = endpoint.parse::<http::Uri>()?;
            let mut sse_endpoint_parts = sse_endpoint.clone().into_parts();
            sse_endpoint_parts.path_and_query = ep.into_parts().path_and_query;
            Uri::from_parts(sse_endpoint_parts)?
        } else {
            // wait the endpoint event
            loop {
                let sse = sse_stream
                    .next()
                    .await
                    .ok_or(SseTransportError::UnexpectedEndOfStream)??;
                let Some("endpoint") = sse.event.as_deref() else {
                    continue;
                };
                let ep = sse.data.unwrap_or_default();

                break message_endpoint(sse_endpoint.clone(), ep)?;
            }
        };

        let stream = Box::pin(SseAutoReconnectStream::new(
            sse_stream,
            SseClientReconnect {
                client: client.clone(),
                uri: sse_endpoint.clone(),
            },
            config.retry_policy.clone(),
        ));
        Ok(Self {
            client,
            config,
            message_endpoint,
            stream: Some(stream),
        })
    }
}

fn message_endpoint(base: http::Uri, endpoint: String) -> Result<http::Uri, http::uri::InvalidUri> {
    // If endpoint is a full URL, parse and return it directly
    if endpoint.starts_with("http://") || endpoint.starts_with("https://") {
        return endpoint.parse::<http::Uri>();
    }

    let mut base_parts = base.into_parts();
    let endpoint_clone = endpoint.clone();

    if endpoint.starts_with("?") {
        // Query only - keep base path and append query
        if let Some(base_path_and_query) = &base_parts.path_and_query {
            let base_path = base_path_and_query.path();
            base_parts.path_and_query = Some(format!("{}{}", base_path, endpoint).parse()?);
        } else {
            base_parts.path_and_query = Some(format!("/{}", endpoint).parse()?);
        }
    } else {
        // Path (with optional query) - replace entire path_and_query
        let path_to_use = if endpoint.starts_with("/") {
            endpoint // Use absolute path as-is
        } else {
            format!("/{}", endpoint) // Make relative path absolute
        };
        base_parts.path_and_query = Some(path_to_use.parse()?);
    }

    http::Uri::from_parts(base_parts).map_err(|_| endpoint_clone.parse::<http::Uri>().unwrap_err())
}

#[derive(Debug, Clone)]
pub struct SseClientConfig {
    /// client sse endpoint
    ///
    /// # How this client resolve the message endpoint
    /// if sse_endpoint has this format: `<schema><authority?><sse_pq>`,
    /// then the message endpoint will be `<schema><authority?><message_pq>`.
    ///
    /// For example, if you config the sse_endpoint as `http://example.com/some_path/sse`,
    /// and the server send the message endpoint event as `message?session_id=123`,
    /// then the message endpoint will be `http://example.com/message`.
    ///
    /// This follow the rules of JavaScript's [`new URL(url, base)`](https://developer.mozilla.org/zh-CN/docs/Web/API/URL/URL)
    pub sse_endpoint: Arc<str>,
    pub retry_policy: Arc<dyn SseRetryPolicy>,
    /// if this is settled, the client will use this endpoint to send message and skip get the endpoint event
    pub use_message_endpoint: Option<String>,
}

impl Default for SseClientConfig {
    fn default() -> Self {
        Self {
            sse_endpoint: "".into(),
            retry_policy: Arc::new(super::common::client_side_sse::FixedInterval::default()),
            use_message_endpoint: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_message_endpoint() {
        let base_url = "https://localhost/sse".parse::<http::Uri>().unwrap();

        // Query only
        let result = message_endpoint(base_url.clone(), "?sessionId=x".to_string()).unwrap();
        assert_eq!(result.to_string(), "https://localhost/sse?sessionId=x");

        // Relative path with query
        let result = message_endpoint(base_url.clone(), "mypath?sessionId=x".to_string()).unwrap();
        assert_eq!(result.to_string(), "https://localhost/mypath?sessionId=x");

        // Absolute path with query
        let result = message_endpoint(base_url.clone(), "/xxx?sessionId=x".to_string()).unwrap();
        assert_eq!(result.to_string(), "https://localhost/xxx?sessionId=x");

        // Full URL
        let result = message_endpoint(
            base_url.clone(),
            "http://example.com/xxx?sessionId=x".to_string(),
        )
        .unwrap();
        assert_eq!(result.to_string(), "http://example.com/xxx?sessionId=x");
    }
}

---
File: modelcontextprotocol-rust-sdk-13e72ec/crates/rmcp/src/transport/sse_server.rs
---

use std::{collections::HashMap, io, net::SocketAddr, sync::Arc, time::Duration};

use axum::{
    Extension, Json, Router,
    extract::{NestedPath, Query, State},
    http::{StatusCode, request::Parts},
    response::{
        Response,
        sse::{Event, KeepAlive, Sse},
    },
    routing::{get, post},
};
use futures::{Sink, SinkExt, Stream};
use tokio_stream::wrappers::ReceiverStream;
use tokio_util::sync::{CancellationToken, PollSender};
use tracing::Instrument;

use crate::{
    RoleServer, Service,
    model::ClientJsonRpcMessage,
    service::{RxJsonRpcMessage, TxJsonRpcMessage, serve_directly_with_ct},
    transport::common::server_side_http::{DEFAULT_AUTO_PING_INTERVAL, SessionId, session_id},
};

type TxStore =
    Arc<tokio::sync::RwLock<HashMap<SessionId, tokio::sync::mpsc::Sender<ClientJsonRpcMessage>>>>;
pub type TransportReceiver = ReceiverStream<RxJsonRpcMessage<RoleServer>>;

#[derive(Clone)]
struct App {
    txs: TxStore,
    transport_tx: tokio::sync::mpsc::UnboundedSender<SseServerTransport>,
    post_path: Arc<str>,
    sse_ping_interval: Duration,
}

impl App {
    pub fn new(
        post_path: String,
        sse_ping_interval: Duration,
    ) -> (
        Self,
        tokio::sync::mpsc::UnboundedReceiver<SseServerTransport>,
    ) {
        let (transport_tx, transport_rx) = tokio::sync::mpsc::unbounded_channel();
        (
            Self {
                txs: Default::default(),
                transport_tx,
                post_path: post_path.into(),
                sse_ping_interval,
            },
            transport_rx,
        )
    }
}

#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PostEventQuery {
    pub session_id: String,
}

async fn post_event_handler(
    State(app): State<App>,
    Query(PostEventQuery { session_id }): Query<PostEventQuery>,
    parts: Parts,
    Json(mut message): Json<ClientJsonRpcMessage>,
) -> Result<StatusCode, StatusCode> {
    tracing::debug!(session_id, ?parts, ?message, "new client message");
    let tx = {
        let rg = app.txs.read().await;
        rg.get(session_id.as_str())
            .ok_or(StatusCode::NOT_FOUND)?
            .clone()
    };
    message.insert_extension(parts);
    if tx.send(message).await.is_err() {
        tracing::error!("send message error");
        return Err(StatusCode::GONE);
    }
    Ok(StatusCode::ACCEPTED)
}

async fn sse_handler(
    State(app): State<App>,
    nested_path: Option<Extension<NestedPath>>,
    parts: Parts,
) -> Result<Sse<impl Stream<Item = Result<Event, io::Error>>>, Response<String>> {
    let session = session_id();
    tracing::info!(%session, ?parts, "sse connection");
    use tokio_stream::{StreamExt, wrappers::ReceiverStream};
    use tokio_util::sync::PollSender;
    let (from_client_tx, from_client_rx) = tokio::sync::mpsc::channel(64);
    let (to_client_tx, to_client_rx) = tokio::sync::mpsc::channel(64);
    let to_client_tx_clone = to_client_tx.clone();

    app.txs
        .write()
        .await
        .insert(session.clone(), from_client_tx);
    let session = session.clone();
    let stream = ReceiverStream::new(from_client_rx);
    let sink = PollSender::new(to_client_tx);
    let transport = SseServerTransport {
        stream,
        sink,
        session_id: session.clone(),
        tx_store: app.txs.clone(),
    };
    let transport_send_result = app.transport_tx.send(transport);
    if transport_send_result.is_err() {
        tracing::warn!("send transport out error");
        let mut response =
            Response::new("fail to send out transport, it seems server is closed".to_string());
        *response.status_mut() = StatusCode::INTERNAL_SERVER_ERROR;
        return Err(response);
    }
    let nested_path = nested_path.as_deref().map(NestedPath::as_str).unwrap_or("");
    let post_path = app.post_path.as_ref();
    let ping_interval = app.sse_ping_interval;
    let stream = futures::stream::once(futures::future::ok(
        Event::default()
            .event("endpoint")
            .data(format!("{nested_path}{post_path}?sessionId={session}")),
    ))
    .chain(ReceiverStream::new(to_client_rx).map(|message| {
        match serde_json::to_string(&message) {
            Ok(bytes) => Ok(Event::default().event("message").data(&bytes)),
            Err(e) => Err(io::Error::new(io::ErrorKind::InvalidData, e)),
        }
    }));

    tokio::spawn(async move {
        // Wait for connection closure
        to_client_tx_clone.closed().await;

        // Clean up session
        let session_id = session.clone();
        let tx_store = app.txs.clone();
        let mut txs = tx_store.write().await;
        txs.remove(&session_id);
        tracing::debug!(%session_id, "Closed session and cleaned up resources");
    });

    Ok(Sse::new(stream).keep_alive(KeepAlive::new().interval(ping_interval)))
}

pub struct SseServerTransport {
    stream: ReceiverStream<RxJsonRpcMessage<RoleServer>>,
    sink: PollSender<TxJsonRpcMessage<RoleServer>>,
    session_id: SessionId,
    tx_store: TxStore,
}

impl Sink<TxJsonRpcMessage<RoleServer>> for SseServerTransport {
    type Error = io::Error;

    fn poll_ready(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), Self::Error>> {
        self.sink
            .poll_ready_unpin(cx)
            .map_err(std::io::Error::other)
    }

    fn start_send(
        mut self: std::pin::Pin<&mut Self>,
        item: TxJsonRpcMessage<RoleServer>,
    ) -> Result<(), Self::Error> {
        self.sink
            .start_send_unpin(item)
            .map_err(std::io::Error::other)
    }

    fn poll_flush(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), Self::Error>> {
        self.sink
            .poll_flush_unpin(cx)
            .map_err(std::io::Error::other)
    }

    fn poll_close(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), Self::Error>> {
        let inner_close_result = self
            .sink
            .poll_close_unpin(cx)
            .map_err(std::io::Error::other);
        if inner_close_result.is_ready() {
            let session_id = self.session_id.clone();
            let tx_store = self.tx_store.clone();
            tokio::spawn(async move {
                tx_store.write().await.remove(&session_id);
            });
        }
        inner_close_result
    }
}

impl Stream for SseServerTransport {
    type Item = RxJsonRpcMessage<RoleServer>;

    fn poll_next(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Option<Self::Item>> {
        use futures::StreamExt;
        self.stream.poll_next_unpin(cx)
    }
}

#[derive(Debug, Clone)]
pub struct SseServerConfig {
    pub bind: SocketAddr,
    pub sse_path: String,
    pub post_path: String,
    pub ct: CancellationToken,
    pub sse_keep_alive: Option<Duration>,
}

#[derive(Debug)]
pub struct SseServer {
    transport_rx: tokio::sync::mpsc::UnboundedReceiver<SseServerTransport>,
    pub config: SseServerConfig,
}

impl SseServer {
    pub async fn serve(bind: SocketAddr) -> io::Result<Self> {
        Self::serve_with_config(SseServerConfig {
            bind,
            sse_path: "/sse".to_string(),
            post_path: "/message".to_string(),
            ct: CancellationToken::new(),
            sse_keep_alive: None,
        })
        .await
    }
    pub async fn serve_with_config(config: SseServerConfig) -> io::Result<Self> {
        let (sse_server, service) = Self::new(config);
        let listener = tokio::net::TcpListener::bind(sse_server.config.bind).await?;
        let ct = sse_server.config.ct.child_token();
        let server = axum::serve(listener, service).with_graceful_shutdown(async move {
            ct.cancelled().await;
            tracing::info!("sse server cancelled");
        });
        tokio::spawn(
            async move {
                if let Err(e) = server.await {
                    tracing::error!(error = %e, "sse server shutdown with error");
                }
            }
            .instrument(tracing::info_span!("sse-server", bind_address = %sse_server.config.bind)),
        );
        Ok(sse_server)
    }

    pub fn new(config: SseServerConfig) -> (SseServer, Router) {
        let (app, transport_rx) = App::new(
            config.post_path.clone(),
            config.sse_keep_alive.unwrap_or(DEFAULT_AUTO_PING_INTERVAL),
        );
        let router = Router::new()
            .route(&config.sse_path, get(sse_handler))
            .route(&config.post_path, post(post_event_handler))
            .with_state(app);

        let server = SseServer {
            transport_rx,
            config,
        };

        (server, router)
    }

    pub fn with_service<S, F>(mut self, service_provider: F) -> CancellationToken
    where
        S: Service<RoleServer>,
        F: Fn() -> S + Send + 'static,
    {
        use crate::service::ServiceExt;
        let ct = self.config.ct.clone();
        tokio::spawn(async move {
            while let Some(transport) = self.next_transport().await {
                let service = service_provider();
                let ct = self.config.ct.child_token();
                tokio::spawn(async move {
                    let server = service
                        .serve_with_ct(transport, ct)
                        .await
                        .map_err(std::io::Error::other)?;
                    server.waiting().await?;
                    tokio::io::Result::Ok(())
                });
            }
        });
        ct
    }

    /// This allows you to skip the initialization steps for incoming request.
    pub fn with_service_directly<S, F>(mut self, service_provider: F) -> CancellationToken
    where
        S: Service<RoleServer>,
        F: Fn() -> S + Send + 'static,
    {
        let ct = self.config.ct.clone();
        tokio::spawn(async move {
            while let Some(transport) = self.next_transport().await {
                let service = service_provider();
                let ct = self.config.ct.child_token();
                tokio::spawn(async move {
                    let server = serve_directly_with_ct(service, transport, None, ct);
                    server.waiting().await?;
                    tokio::io::Result::Ok(())
                });
            }
        });
        ct
    }

    pub fn cancel(&self) {
        self.config.ct.cancel();
    }

    pub async fn next_transport(&mut self) -> Option<SseServerTransport> {
        self.transport_rx.recv().await
    }
}

impl Stream for SseServer {
    type Item = SseServerTransport;

    fn poll_next(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Option<Self::Item>> {
        self.transport_rx.poll_recv(cx)
    }
}

---
File: modelcontextprotocol-rust-sdk-13e72ec/crates/rmcp/src/transport/streamable_http_client.rs
---

use std::{borrow::Cow, sync::Arc, time::Duration};

use futures::{Stream, StreamExt, future::BoxFuture, stream::BoxStream};
pub use sse_stream::Error as SseError;
use sse_stream::Sse;
use thiserror::Error;
use tokio_util::sync::CancellationToken;

use super::common::client_side_sse::{ExponentialBackoff, SseRetryPolicy, SseStreamReconnect};
use crate::{
    RoleClient,
    model::{ClientJsonRpcMessage, ServerJsonRpcMessage},
    transport::{
        common::client_side_sse::SseAutoReconnectStream,
        worker::{Worker, WorkerQuitReason, WorkerSendRequest, WorkerTransport},
    },
};

type BoxedSseStream = BoxStream<'static, Result<Sse, SseError>>;

#[derive(Error, Debug)]
pub enum StreamableHttpError<E: std::error::Error + Send + Sync + 'static> {
    #[error("SSE error: {0}")]
    Sse(#[from] SseError),
    #[error("Io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Client error: {0}")]
    Client(E),
    #[error("unexpected end of stream")]
    UnexpectedEndOfStream,
    #[error("unexpected server response: {0}")]
    UnexpectedServerResponse(Cow<'static, str>),
    #[error("Unexpected content type: {0:?}")]
    UnexpectedContentType(Option<String>),
    #[error("Server does not support SSE")]
    SeverDoesNotSupportSse,
    #[error("Server does not support delete session")]
    SeverDoesNotSupportDeleteSession,
    #[error("Tokio join error: {0}")]
    TokioJoinError(#[from] tokio::task::JoinError),
    #[error("Deserialize error: {0}")]
    Deserialize(#[from] serde_json::Error),
    #[error("Transport channel closed")]
    TransportChannelClosed,
    #[cfg(feature = "auth")]
    #[cfg_attr(docsrs, doc(cfg(feature = "auth")))]
    #[error("Auth error: {0}")]
    Auth(#[from] crate::transport::auth::AuthError),
}

impl From<reqwest::Error> for StreamableHttpError<reqwest::Error> {
    fn from(e: reqwest::Error) -> Self {
        StreamableHttpError::Client(e)
    }
}

pub enum StreamableHttpPostResponse {
    Accepted,
    Json(ServerJsonRpcMessage, Option<String>),
    Sse(BoxedSseStream, Option<String>),
}

impl std::fmt::Debug for StreamableHttpPostResponse {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Accepted => write!(f, "Accepted"),
            Self::Json(arg0, arg1) => f.debug_tuple("Json").field(arg0).field(arg1).finish(),
            Self::Sse(_, arg1) => f.debug_tuple("Sse").field(arg1).finish(),
        }
    }
}

impl StreamableHttpPostResponse {
    pub async fn expect_initialized<E>(
        self,
    ) -> Result<(ServerJsonRpcMessage, Option<String>), StreamableHttpError<E>>
    where
        E: std::error::Error + Send + Sync + 'static,
    {
        match self {
            Self::Json(message, session_id) => Ok((message, session_id)),
            Self::Sse(mut stream, session_id) => {
                let event =
                    stream
                        .next()
                        .await
                        .ok_or(StreamableHttpError::UnexpectedServerResponse(
                            "empty sse stream".into(),
                        ))??;
                let message: ServerJsonRpcMessage =
                    serde_json::from_str(&event.data.unwrap_or_default())?;
                Ok((message, session_id))
            }
            _ => Err(StreamableHttpError::UnexpectedServerResponse(
                "expect initialized, accepted".into(),
            )),
        }
    }

    pub fn expect_json<E>(self) -> Result<ServerJsonRpcMessage, StreamableHttpError<E>>
    where
        E: std::error::Error + Send + Sync + 'static,
    {
        match self {
            Self::Json(message, ..) => Ok(message),
            got => Err(StreamableHttpError::UnexpectedServerResponse(
                format!("expect json, got {got:?}").into(),
            )),
        }
    }

    pub fn expect_accepted<E>(self) -> Result<(), StreamableHttpError<E>>
    where
        E: std::error::Error + Send + Sync + 'static,
    {
        match self {
            Self::Accepted => Ok(()),
            got => Err(StreamableHttpError::UnexpectedServerResponse(
                format!("expect accepted, got {got:?}").into(),
            )),
        }
    }
}

pub trait StreamableHttpClient: Clone + Send + 'static {
    type Error: std::error::Error + Send + Sync + 'static;
    fn post_message(
        &self,
        uri: Arc<str>,
        message: ClientJsonRpcMessage,
        session_id: Option<Arc<str>>,
        auth_header: Option<String>,
    ) -> impl Future<Output = Result<StreamableHttpPostResponse, StreamableHttpError<Self::Error>>>
    + Send
    + '_;
    fn delete_session(
        &self,
        uri: Arc<str>,
        session_id: Arc<str>,
        auth_header: Option<String>,
    ) -> impl Future<Output = Result<(), StreamableHttpError<Self::Error>>> + Send + '_;
    fn get_stream(
        &self,
        uri: Arc<str>,
        session_id: Arc<str>,
        last_event_id: Option<String>,
        auth_header: Option<String>,
    ) -> impl Future<
        Output = Result<
            BoxStream<'static, Result<Sse, SseError>>,
            StreamableHttpError<Self::Error>,
        >,
    > + Send
    + '_;
}

pub struct RetryConfig {
    pub max_times: Option<usize>,
    pub min_duration: Duration,
}

struct StreamableHttpClientReconnect<C> {
    pub client: C,
    pub session_id: Arc<str>,
    pub uri: Arc<str>,
}

impl<C: StreamableHttpClient> SseStreamReconnect for StreamableHttpClientReconnect<C> {
    type Error = StreamableHttpError<C::Error>;
    type Future = BoxFuture<'static, Result<BoxedSseStream, Self::Error>>;
    fn retry_connection(&mut self, last_event_id: Option<&str>) -> Self::Future {
        let client = self.client.clone();
        let uri = self.uri.clone();
        let session_id = self.session_id.clone();
        let last_event_id = last_event_id.map(|s| s.to_owned());
        Box::pin(async move {
            client
                .get_stream(uri, session_id, last_event_id, None)
                .await
        })
    }
}

#[derive(Debug, Clone, Default)]
pub struct StreamableHttpClientWorker<C: StreamableHttpClient> {
    pub client: C,
    pub config: StreamableHttpClientTransportConfig,
}

impl<C: StreamableHttpClient + Default> StreamableHttpClientWorker<C> {
    pub fn new_simple(url: impl Into<Arc<str>>) -> Self {
        Self {
            client: C::default(),
            config: StreamableHttpClientTransportConfig {
                uri: url.into(),
                ..Default::default()
            },
        }
    }
}

impl<C: StreamableHttpClient> StreamableHttpClientWorker<C> {
    pub fn new(client: C, config: StreamableHttpClientTransportConfig) -> Self {
        Self { client, config }
    }
}

impl<C: StreamableHttpClient> StreamableHttpClientWorker<C> {
    async fn execute_sse_stream(
        sse_stream: impl Stream<Item = Result<ServerJsonRpcMessage, StreamableHttpError<C::Error>>>
        + Send
        + 'static,
        sse_worker_tx: tokio::sync::mpsc::Sender<ServerJsonRpcMessage>,
        close_on_response: bool,
        ct: CancellationToken,
    ) -> Result<(), StreamableHttpError<C::Error>> {
        let mut sse_stream = std::pin::pin!(sse_stream);
        loop {
            let message = tokio::select! {
                event = sse_stream.next() => {
                    event
                }
                _ = ct.cancelled() => {
                    tracing::debug!("cancelled");
                    break;
                }
            };
            let Some(message) = message.transpose()? else {
                break;
            };
            let is_response = matches!(message, ServerJsonRpcMessage::Response(_));
            let yield_result = sse_worker_tx.send(message).await;
            if yield_result.is_err() {
                tracing::trace!("streamable http transport worker dropped, exiting");
                break;
            }
            if close_on_response && is_response {
                tracing::debug!("got response, closing sse stream");
                break;
            }
        }
        Ok(())
    }
}

impl<C: StreamableHttpClient> Worker for StreamableHttpClientWorker<C> {
    type Role = RoleClient;
    type Error = StreamableHttpError<C::Error>;
    fn err_closed() -> Self::Error {
        StreamableHttpError::TransportChannelClosed
    }
    fn err_join(e: tokio::task::JoinError) -> Self::Error {
        StreamableHttpError::TokioJoinError(e)
    }
    fn config(&self) -> super::worker::WorkerConfig {
        super::worker::WorkerConfig {
            name: Some("StreamableHttpClientWorker".into()),
            channel_buffer_capacity: self.config.channel_buffer_capacity,
        }
    }
    async fn run(
        self,
        mut context: super::worker::WorkerContext<Self>,
    ) -> Result<(), WorkerQuitReason> {
        let channel_buffer_capacity = self.config.channel_buffer_capacity;
        let (sse_worker_tx, mut sse_worker_rx) =
            tokio::sync::mpsc::channel::<ServerJsonRpcMessage>(channel_buffer_capacity);
        let config = self.config.clone();
        let transport_task_ct = context.cancellation_token.clone();
        let _drop_guard = transport_task_ct.clone().drop_guard();
        let WorkerSendRequest {
            responder,
            message: initialize_request,
        } = context.recv_from_handler().await?;
        let _ = responder.send(Ok(()));
        let (message, session_id) = self
            .client
            .post_message(config.uri.clone(), initialize_request, None, None)
            .await
            .map_err(WorkerQuitReason::fatal_context("send initialize request"))?
            .expect_initialized::<Self::Error>()
            .await
            .map_err(WorkerQuitReason::fatal_context(
                "process initialize response",
            ))?;
        let session_id: Option<Arc<str>> = if let Some(session_id) = session_id {
            Some(session_id.into())
        } else {
            if !self.config.allow_stateless {
                return Err(WorkerQuitReason::fatal(
                    "missing session id in initialize response",
                    "process initialize response",
                ));
            }
            None
        };
        // delete session when drop guard is dropped
        if let Some(session_id) = &session_id {
            let ct = transport_task_ct.clone();
            let client = self.client.clone();
            let session_id = session_id.clone();
            let url = config.uri.clone();
            tokio::spawn(async move {
                ct.cancelled().await;
                let delete_session_result =
                    client.delete_session(url, session_id.clone(), None).await;
                match delete_session_result {
                    Ok(_) => {
                        tracing::info!(session_id = session_id.as_ref(), "delete session success")
                    }
                    Err(StreamableHttpError::SeverDoesNotSupportDeleteSession) => {
                        tracing::info!(
                            session_id = session_id.as_ref(),
                            "server doesn't support delete session"
                        )
                    }
                    Err(e) => {
                        tracing::error!(
                            session_id = session_id.as_ref(),
                            "fail to delete session: {e}"
                        );
                    }
                };
            });
        }

        context.send_to_handler(message).await?;
        let initialized_notification = context.recv_from_handler().await?;
        // expect a initialized response
        self.client
            .post_message(
                config.uri.clone(),
                initialized_notification.message,
                session_id.clone(),
                None,
            )
            .await
            .map_err(WorkerQuitReason::fatal_context(
                "send initialized notification",
            ))?
            .expect_accepted::<Self::Error>()
            .map_err(WorkerQuitReason::fatal_context(
                "process initialized notification response",
            ))?;
        let _ = initialized_notification.responder.send(Ok(()));
        enum Event<W: Worker, E: std::error::Error + Send + Sync + 'static> {
            ClientMessage(WorkerSendRequest<W>),
            ServerMessage(ServerJsonRpcMessage),
            StreamResult(Result<(), StreamableHttpError<E>>),
        }
        let mut streams = tokio::task::JoinSet::new();
        if let Some(session_id) = &session_id {
            match self
                .client
                .get_stream(config.uri.clone(), session_id.clone(), None, None)
                .await
            {
                Ok(stream) => {
                    let sse_stream = SseAutoReconnectStream::new(
                        stream,
                        StreamableHttpClientReconnect {
                            client: self.client.clone(),
                            session_id: session_id.clone(),
                            uri: config.uri.clone(),
                        },
                        self.config.retry_config.clone(),
                    );
                    streams.spawn(Self::execute_sse_stream(
                        sse_stream,
                        sse_worker_tx.clone(),
                        false,
                        transport_task_ct.child_token(),
                    ));
                    tracing::debug!("got common stream");
                }
                Err(StreamableHttpError::SeverDoesNotSupportSse) => {
                    tracing::debug!("server doesn't support sse, skip common stream");
                }
                Err(e) => {
                    // fail to get common stream
                    tracing::error!("fail to get common stream: {e}");
                    return Err(WorkerQuitReason::fatal(
                        "fail to get general purpose event stream",
                        "get general purpose event stream",
                    ));
                }
            }
        }
        loop {
            let event = tokio::select! {
                _ = transport_task_ct.cancelled() => {
                    tracing::debug!("cancelled");
                    return Err(WorkerQuitReason::Cancelled);
                }
                message = context.recv_from_handler() => {
                    let message = message?;
                    Event::ClientMessage(message)
                },
                message = sse_worker_rx.recv() => {
                    let Some(message) = message else {
                        tracing::trace!("transport dropped, exiting");
                        return Err(WorkerQuitReason::HandlerTerminated);
                    };
                    Event::ServerMessage(message)
                },
                terminated_stream = streams.join_next(), if !streams.is_empty() => {
                    match terminated_stream {
                        Some(result) => {
                            Event::StreamResult(result.map_err(StreamableHttpError::TokioJoinError).and_then(std::convert::identity))
                        }
                        None => {
                            continue
                        }
                    }
                }
            };
            match event {
                Event::ClientMessage(send_request) => {
                    let WorkerSendRequest { message, responder } = send_request;
                    let response = self
                        .client
                        .post_message(config.uri.clone(), message, session_id.clone(), None)
                        .await;
                    let send_result = match response {
                        Err(e) => Err(e),
                        Ok(StreamableHttpPostResponse::Accepted) => {
                            tracing::trace!("client message accepted");
                            Ok(())
                        }
                        Ok(StreamableHttpPostResponse::Json(message, ..)) => {
                            context.send_to_handler(message).await?;
                            Ok(())
                        }
                        Ok(StreamableHttpPostResponse::Sse(stream, ..)) => {
                            if let Some(session_id) = &session_id {
                                let sse_stream = SseAutoReconnectStream::new(
                                    stream,
                                    StreamableHttpClientReconnect {
                                        client: self.client.clone(),
                                        session_id: session_id.clone(),
                                        uri: config.uri.clone(),
                                    },
                                    self.config.retry_config.clone(),
                                );
                                streams.spawn(Self::execute_sse_stream(
                                    sse_stream,
                                    sse_worker_tx.clone(),
                                    true,
                                    transport_task_ct.child_token(),
                                ));
                            } else {
                                let sse_stream = SseAutoReconnectStream::never_reconnect(
                                    stream,
                                    StreamableHttpError::<C::Error>::UnexpectedEndOfStream,
                                );
                                streams.spawn(Self::execute_sse_stream(
                                    sse_stream,
                                    sse_worker_tx.clone(),
                                    true,
                                    transport_task_ct.child_token(),
                                ));
                            }
                            tracing::trace!("got new sse stream");
                            Ok(())
                        }
                    };
                    let _ = responder.send(send_result);
                }
                Event::ServerMessage(json_rpc_message) => {
                    // send the message to the handler
                    context.send_to_handler(json_rpc_message).await?;
                }
                Event::StreamResult(result) => {
                    if result.is_err() {
                        tracing::warn!(
                            "sse client event stream terminated with error: {:?}",
                            result
                        );
                    }
                }
            }
        }
    }
}

pub type StreamableHttpClientTransport<C> = WorkerTransport<StreamableHttpClientWorker<C>>;

impl<C: StreamableHttpClient> StreamableHttpClientTransport<C> {
    pub fn with_client(client: C, config: StreamableHttpClientTransportConfig) -> Self {
        let worker = StreamableHttpClientWorker::new(client, config);
        WorkerTransport::spawn(worker)
    }
}
#[derive(Debug, Clone)]
pub struct StreamableHttpClientTransportConfig {
    pub uri: Arc<str>,
    pub retry_config: Arc<dyn SseRetryPolicy>,
    pub channel_buffer_capacity: usize,
    /// if true, the transport will not require a session to be established
    pub allow_stateless: bool,
}

impl StreamableHttpClientTransportConfig {
    pub fn with_uri(uri: impl Into<Arc<str>>) -> Self {
        Self {
            uri: uri.into(),
            ..Default::default()
        }
    }
}

impl Default for StreamableHttpClientTransportConfig {
    fn default() -> Self {
        Self {
            uri: "localhost".into(),
            retry_config: Arc::new(ExponentialBackoff::default()),
            channel_buffer_capacity: 16,
            allow_stateless: true,
        }
    }
}

---
File: modelcontextprotocol-rust-sdk-13e72ec/crates/rmcp/src/transport/streamable_http_server.rs
---

pub mod session;
#[cfg(feature = "transport-streamable-http-server")]
#[cfg_attr(docsrs, doc(cfg(feature = "transport-streamable-http-server")))]
pub mod tower;
pub use session::{SessionId, SessionManager};
#[cfg(feature = "transport-streamable-http-server")]
#[cfg_attr(docsrs, doc(cfg(feature = "transport-streamable-http-server")))]
pub use tower::{StreamableHttpServerConfig, StreamableHttpService};

---
File: modelcontextprotocol-rust-sdk-13e72ec/crates/rmcp/src/transport/streamable_http_server/session.rs
---

use futures::Stream;

pub use crate::transport::common::server_side_http::SessionId;
use crate::{
    RoleServer,
    model::{ClientJsonRpcMessage, ServerJsonRpcMessage},
    transport::common::server_side_http::ServerSseMessage,
};

pub mod local;
pub mod never;

pub trait SessionManager: Send + Sync + 'static {
    type Error: std::error::Error + Send + 'static;
    type Transport: crate::transport::Transport<RoleServer>;
    /// Create a new session with the given id and configuration.
    fn create_session(
        &self,
    ) -> impl Future<Output = Result<(SessionId, Self::Transport), Self::Error>> + Send;
    fn initialize_session(
        &self,
        id: &SessionId,
        message: ClientJsonRpcMessage,
    ) -> impl Future<Output = Result<ServerJsonRpcMessage, Self::Error>> + Send;
    fn has_session(&self, id: &SessionId)
    -> impl Future<Output = Result<bool, Self::Error>> + Send;
    fn close_session(&self, id: &SessionId)
    -> impl Future<Output = Result<(), Self::Error>> + Send;
    fn create_stream(
        &self,
        id: &SessionId,
        message: ClientJsonRpcMessage,
    ) -> impl Future<
        Output = Result<impl Stream<Item = ServerSseMessage> + Send + 'static, Self::Error>,
    > + Send;
    fn accept_message(
        &self,
        id: &SessionId,
        message: ClientJsonRpcMessage,
    ) -> impl Future<Output = Result<(), Self::Error>> + Send;
    fn create_standalone_stream(
        &self,
        id: &SessionId,
    ) -> impl Future<
        Output = Result<impl Stream<Item = ServerSseMessage> + Send + 'static, Self::Error>,
    > + Send;
    fn resume(
        &self,
        id: &SessionId,
        last_event_id: String,
    ) -> impl Future<
        Output = Result<impl Stream<Item = ServerSseMessage> + Send + 'static, Self::Error>,
    > + Send;
}

---
File: modelcontextprotocol-rust-sdk-13e72ec/crates/rmcp/src/transport/streamable_http_server/session/local.rs
---

use std::{
    collections::{HashMap, HashSet, VecDeque},
    num::ParseIntError,
    sync::Arc,
    time::Duration,
};

use futures::Stream;
use thiserror::Error;
use tokio::sync::{
    mpsc::{Receiver, Sender},
    oneshot,
};
use tokio_stream::wrappers::ReceiverStream;
use tracing::instrument;

use crate::{
    RoleServer,
    model::{
        CancelledNotificationParam, ClientJsonRpcMessage, ClientNotification, ClientRequest,
        JsonRpcNotification, JsonRpcRequest, Notification, ProgressNotificationParam,
        ProgressToken, RequestId, ServerJsonRpcMessage, ServerNotification,
    },
    transport::{
        WorkerTransport,
        common::server_side_http::{SessionId, session_id},
        worker::{Worker, WorkerContext, WorkerQuitReason, WorkerSendRequest},
    },
};

#[derive(Debug, Default)]
pub struct LocalSessionManager {
    pub sessions: tokio::sync::RwLock<HashMap<SessionId, LocalSessionHandle>>,
    pub session_config: SessionConfig,
}

#[derive(Debug, Error)]
pub enum LocalSessionManagerError {
    #[error("Session not found: {0}")]
    SessionNotFound(SessionId),
    #[error("Session error: {0}")]
    SessionError(#[from] SessionError),
    #[error("Invalid event id: {0}")]
    InvalidEventId(#[from] EventIdParseError),
}
impl SessionManager for LocalSessionManager {
    type Error = LocalSessionManagerError;
    type Transport = WorkerTransport<LocalSessionWorker>;
    async fn create_session(&self) -> Result<(SessionId, Self::Transport), Self::Error> {
        let id = session_id();
        let (handle, worker) = create_local_session(id.clone(), self.session_config.clone());
        self.sessions.write().await.insert(id.clone(), handle);
        Ok((id, WorkerTransport::spawn(worker)))
    }
    async fn initialize_session(
        &self,
        id: &SessionId,
        message: ClientJsonRpcMessage,
    ) -> Result<ServerJsonRpcMessage, Self::Error> {
        let sessions = self.sessions.read().await;
        let handle = sessions
            .get(id)
            .ok_or(LocalSessionManagerError::SessionNotFound(id.clone()))?;
        let response = handle.initialize(message).await?;
        Ok(response)
    }
    async fn close_session(&self, id: &SessionId) -> Result<(), Self::Error> {
        let mut sessions = self.sessions.write().await;
        if let Some(handle) = sessions.remove(id) {
            handle.close().await?;
        }
        Ok(())
    }
    async fn has_session(&self, id: &SessionId) -> Result<bool, Self::Error> {
        let sessions = self.sessions.read().await;
        Ok(sessions.contains_key(id))
    }
    async fn create_stream(
        &self,
        id: &SessionId,
        message: ClientJsonRpcMessage,
    ) -> Result<impl Stream<Item = ServerSseMessage> + Send + 'static, Self::Error> {
        let sessions = self.sessions.read().await;
        let handle = sessions
            .get(id)
            .ok_or(LocalSessionManagerError::SessionNotFound(id.clone()))?;
        let receiver = handle.establish_request_wise_channel().await?;
        handle
            .push_message(message, receiver.http_request_id)
            .await?;
        Ok(ReceiverStream::new(receiver.inner))
    }

    async fn create_standalone_stream(
        &self,
        id: &SessionId,
    ) -> Result<impl Stream<Item = ServerSseMessage> + Send + 'static, Self::Error> {
        let sessions = self.sessions.read().await;
        let handle = sessions
            .get(id)
            .ok_or(LocalSessionManagerError::SessionNotFound(id.clone()))?;
        let receiver = handle.establish_common_channel().await?;
        Ok(ReceiverStream::new(receiver.inner))
    }

    async fn resume(
        &self,
        id: &SessionId,
        last_event_id: String,
    ) -> Result<impl Stream<Item = ServerSseMessage> + Send + 'static, Self::Error> {
        let sessions = self.sessions.read().await;
        let handle = sessions
            .get(id)
            .ok_or(LocalSessionManagerError::SessionNotFound(id.clone()))?;
        let receiver = handle.resume(last_event_id.parse()?).await?;
        Ok(ReceiverStream::new(receiver.inner))
    }

    async fn accept_message(
        &self,
        id: &SessionId,
        message: ClientJsonRpcMessage,
    ) -> Result<(), Self::Error> {
        let sessions = self.sessions.read().await;
        let handle = sessions
            .get(id)
            .ok_or(LocalSessionManagerError::SessionNotFound(id.clone()))?;
        handle.push_message(message, None).await?;
        Ok(())
    }
}

/// `<index>/request_id>`
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct EventId {
    http_request_id: Option<HttpRequestId>,
    index: usize,
}

impl std::fmt::Display for EventId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.index)?;
        match &self.http_request_id {
            Some(http_request_id) => write!(f, "/{http_request_id}"),
            None => write!(f, ""),
        }
    }
}

#[derive(Debug, Clone, Error)]
pub enum EventIdParseError {
    #[error("Invalid index: {0}")]
    InvalidIndex(ParseIntError),
    #[error("Invalid numeric request id: {0}")]
    InvalidNumericRequestId(ParseIntError),
    #[error("Missing request id type")]
    InvalidRequestIdType,
    #[error("Missing request id")]
    MissingRequestId,
}

impl std::str::FromStr for EventId {
    type Err = EventIdParseError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if let Some((index, request_id)) = s.split_once("/") {
            let index = usize::from_str(index).map_err(EventIdParseError::InvalidIndex)?;
            let request_id = u64::from_str(request_id).map_err(EventIdParseError::InvalidIndex)?;
            Ok(EventId {
                http_request_id: Some(request_id),
                index,
            })
        } else {
            let index = usize::from_str(s).map_err(EventIdParseError::InvalidIndex)?;
            Ok(EventId {
                http_request_id: None,
                index,
            })
        }
    }
}

use super::{ServerSseMessage, SessionManager};

struct CachedTx {
    tx: Sender<ServerSseMessage>,
    cache: VecDeque<ServerSseMessage>,
    http_request_id: Option<HttpRequestId>,
    capacity: usize,
}

impl CachedTx {
    fn new(tx: Sender<ServerSseMessage>, http_request_id: Option<HttpRequestId>) -> Self {
        Self {
            cache: VecDeque::with_capacity(tx.capacity()),
            capacity: tx.capacity(),
            tx,
            http_request_id,
        }
    }
    fn new_common(tx: Sender<ServerSseMessage>) -> Self {
        Self::new(tx, None)
    }

    async fn send(&mut self, message: ServerJsonRpcMessage) {
        let index = self.cache.back().map_or(0, |m| {
            m.event_id
                .as_deref()
                .unwrap_or_default()
                .parse::<EventId>()
                .expect("valid event id")
                .index
                + 1
        });
        let event_id = EventId {
            http_request_id: self.http_request_id,
            index,
        };
        let message = ServerSseMessage {
            event_id: Some(event_id.to_string()),
            message: Arc::new(message),
        };
        if self.cache.len() >= self.capacity {
            self.cache.pop_front();
            self.cache.push_back(message.clone());
        } else {
            self.cache.push_back(message.clone());
        }
        let _ = self.tx.send(message).await.inspect_err(|e| {
            let event_id = &e.0.event_id;
            tracing::trace!(?event_id, "trying to send message in a closed session")
        });
    }

    async fn sync(&mut self, index: usize) -> Result<(), SessionError> {
        let Some(front) = self.cache.front() else {
            return Ok(());
        };
        let front_event_id = front
            .event_id
            .as_deref()
            .unwrap_or_default()
            .parse::<EventId>()?;
        let sync_index = index.saturating_sub(front_event_id.index);
        if sync_index > self.cache.len() {
            // invalid index
            return Err(SessionError::InvalidEventId);
        }
        for message in self.cache.iter().skip(sync_index) {
            let send_result = self.tx.send(message.clone()).await;
            if send_result.is_err() {
                let event_id: EventId = message.event_id.as_deref().unwrap_or_default().parse()?;
                return Err(SessionError::ChannelClosed(Some(event_id.index as u64)));
            }
        }
        Ok(())
    }
}

struct HttpRequestWise {
    resources: HashSet<ResourceKey>,
    tx: CachedTx,
}

type HttpRequestId = u64;
#[derive(Debug, Clone, Hash, PartialEq, Eq)]
enum ResourceKey {
    McpRequestId(RequestId),
    ProgressToken(ProgressToken),
}

pub struct LocalSessionWorker {
    id: SessionId,
    next_http_request_id: HttpRequestId,
    tx_router: HashMap<HttpRequestId, HttpRequestWise>,
    resource_router: HashMap<ResourceKey, HttpRequestId>,
    common: CachedTx,
    event_rx: Receiver<SessionEvent>,
    session_config: SessionConfig,
}

impl LocalSessionWorker {
    pub fn id(&self) -> &SessionId {
        &self.id
    }
}

#[derive(Debug, Error)]
pub enum SessionError {
    #[error("Invalid request id: {0}")]
    DuplicatedRequestId(HttpRequestId),
    #[error("Channel closed: {0:?}")]
    ChannelClosed(Option<HttpRequestId>),
    #[error("Cannot parse event id: {0}")]
    EventIdParseError(#[from] EventIdParseError),
    #[error("Session service terminated")]
    SessionServiceTerminated,
    #[error("Invalid event id")]
    InvalidEventId,
    #[error("Transport closed")]
    TransportClosed,
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Tokio join error {0}")]
    TokioJoinError(#[from] tokio::task::JoinError),
}

impl From<SessionError> for std::io::Error {
    fn from(value: SessionError) -> Self {
        match value {
            SessionError::Io(io) => io,
            _ => std::io::Error::new(std::io::ErrorKind::Other, format!("Session error: {value}")),
        }
    }
}

enum OutboundChannel {
    RequestWise { id: HttpRequestId, close: bool },
    Common,
}

pub struct StreamableHttpMessageReceiver {
    pub http_request_id: Option<HttpRequestId>,
    pub inner: Receiver<ServerSseMessage>,
}

impl LocalSessionWorker {
    fn unregister_resource(&mut self, resource: &ResourceKey) {
        if let Some(http_request_id) = self.resource_router.remove(resource) {
            tracing::trace!(?resource, http_request_id, "unregister resource");
            if let Some(channel) = self.tx_router.get_mut(&http_request_id) {
                // It's okey to do so, since we don't handle batch json rpc request anymore
                // and this can be refactored after the batch request is removed in the coming version.
                if channel.resources.is_empty() || matches!(resource, ResourceKey::McpRequestId(_))
                {
                    tracing::debug!(http_request_id, "close http request wise channel");
                    if let Some(channel) = self.tx_router.remove(&http_request_id) {
                        for resource in channel.resources {
                            self.resource_router.remove(&resource);
                        }
                    }
                }
            } else {
                tracing::warn!(http_request_id, "http request wise channel not found");
            }
        }
    }
    fn register_resource(&mut self, resource: ResourceKey, http_request_id: HttpRequestId) {
        tracing::trace!(?resource, http_request_id, "register resource");
        if let Some(channel) = self.tx_router.get_mut(&http_request_id) {
            channel.resources.insert(resource.clone());
            self.resource_router.insert(resource, http_request_id);
        }
    }
    fn register_request(
        &mut self,
        request: &JsonRpcRequest<ClientRequest>,
        http_request_id: HttpRequestId,
    ) {
        use crate::model::GetMeta;
        self.register_resource(
            ResourceKey::McpRequestId(request.id.clone()),
            http_request_id,
        );
        if let Some(progress_token) = request.request.get_meta().get_progress_token() {
            self.register_resource(
                ResourceKey::ProgressToken(progress_token.clone()),
                http_request_id,
            );
        }
    }
    fn catch_cancellation_notification(
        &mut self,
        notification: &JsonRpcNotification<ClientNotification>,
    ) {
        if let ClientNotification::CancelledNotification(n) = &notification.notification {
            let request_id = n.params.request_id.clone();
            let resource = ResourceKey::McpRequestId(request_id);
            self.unregister_resource(&resource);
        }
    }
    fn next_http_request_id(&mut self) -> HttpRequestId {
        let id = self.next_http_request_id;
        self.next_http_request_id = self.next_http_request_id.wrapping_add(1);
        id
    }
    async fn establish_request_wise_channel(
        &mut self,
    ) -> Result<StreamableHttpMessageReceiver, SessionError> {
        let http_request_id = self.next_http_request_id();
        let (tx, rx) = tokio::sync::mpsc::channel(self.session_config.channel_capacity);
        self.tx_router.insert(
            http_request_id,
            HttpRequestWise {
                resources: Default::default(),
                tx: CachedTx::new(tx, Some(http_request_id)),
            },
        );
        tracing::debug!(http_request_id, "establish new request wise channel");
        Ok(StreamableHttpMessageReceiver {
            http_request_id: Some(http_request_id),
            inner: rx,
        })
    }
    fn resolve_outbound_channel(&self, message: &ServerJsonRpcMessage) -> OutboundChannel {
        match &message {
            ServerJsonRpcMessage::Request(_) => OutboundChannel::Common,
            ServerJsonRpcMessage::Notification(JsonRpcNotification {
                notification:
                    ServerNotification::ProgressNotification(Notification {
                        params: ProgressNotificationParam { progress_token, .. },
                        ..
                    }),
                ..
            }) => {
                let id = self
                    .resource_router
                    .get(&ResourceKey::ProgressToken(progress_token.clone()));

                if let Some(id) = id {
                    OutboundChannel::RequestWise {
                        id: *id,
                        close: false,
                    }
                } else {
                    OutboundChannel::Common
                }
            }
            ServerJsonRpcMessage::Notification(JsonRpcNotification {
                notification:
                    ServerNotification::CancelledNotification(Notification {
                        params: CancelledNotificationParam { request_id, .. },
                        ..
                    }),
                ..
            }) => {
                if let Some(id) = self
                    .resource_router
                    .get(&ResourceKey::McpRequestId(request_id.clone()))
                {
                    OutboundChannel::RequestWise {
                        id: *id,
                        close: false,
                    }
                } else {
                    OutboundChannel::Common
                }
            }
            ServerJsonRpcMessage::Notification(_) => OutboundChannel::Common,
            ServerJsonRpcMessage::Response(json_rpc_response) => {
                if let Some(id) = self
                    .resource_router
                    .get(&ResourceKey::McpRequestId(json_rpc_response.id.clone()))
                {
                    OutboundChannel::RequestWise {
                        id: *id,
                        close: false,
                    }
                } else {
                    OutboundChannel::Common
                }
            }
            ServerJsonRpcMessage::Error(json_rpc_error) => {
                if let Some(id) = self
                    .resource_router
                    .get(&ResourceKey::McpRequestId(json_rpc_error.id.clone()))
                {
                    OutboundChannel::RequestWise {
                        id: *id,
                        close: false,
                    }
                } else {
                    OutboundChannel::Common
                }
            }
            ServerJsonRpcMessage::BatchRequest(_) | ServerJsonRpcMessage::BatchResponse(_) => {
                // the server side should never yield a batch request or response now
                unreachable!("server side won't yield batch request or response")
            }
        }
    }
    async fn handle_server_message(
        &mut self,
        message: ServerJsonRpcMessage,
    ) -> Result<(), SessionError> {
        let outbound_channel = self.resolve_outbound_channel(&message);
        match outbound_channel {
            OutboundChannel::RequestWise { id, close } => {
                if let Some(request_wise) = self.tx_router.get_mut(&id) {
                    request_wise.tx.send(message).await;
                    if close {
                        self.tx_router.remove(&id);
                    }
                } else {
                    return Err(SessionError::ChannelClosed(Some(id)));
                }
            }
            OutboundChannel::Common => self.common.send(message).await,
        }
        Ok(())
    }
    async fn resume(
        &mut self,
        last_event_id: EventId,
    ) -> Result<StreamableHttpMessageReceiver, SessionError> {
        match last_event_id.http_request_id {
            Some(http_request_id) => {
                let request_wise = self
                    .tx_router
                    .get_mut(&http_request_id)
                    .ok_or(SessionError::ChannelClosed(Some(http_request_id)))?;
                let channel = tokio::sync::mpsc::channel(self.session_config.channel_capacity);
                let (tx, rx) = channel;
                request_wise.tx.tx = tx;
                let index = last_event_id.index;
                // sync messages after index
                request_wise.tx.sync(index).await?;
                Ok(StreamableHttpMessageReceiver {
                    http_request_id: Some(http_request_id),
                    inner: rx,
                })
            }
            None => {
                let channel = tokio::sync::mpsc::channel(self.session_config.channel_capacity);
                let (tx, rx) = channel;
                self.common.tx = tx;
                let index = last_event_id.index;
                // sync messages after index
                self.common.sync(index).await?;
                Ok(StreamableHttpMessageReceiver {
                    http_request_id: None,
                    inner: rx,
                })
            }
        }
    }
}

enum SessionEvent {
    ClientMessage {
        message: ClientJsonRpcMessage,
        http_request_id: Option<HttpRequestId>,
    },
    EstablishRequestWiseChannel {
        responder: oneshot::Sender<Result<StreamableHttpMessageReceiver, SessionError>>,
    },
    CloseRequestWiseChannel {
        id: HttpRequestId,
        responder: oneshot::Sender<Result<(), SessionError>>,
    },
    Resume {
        last_event_id: EventId,
        responder: oneshot::Sender<Result<StreamableHttpMessageReceiver, SessionError>>,
    },
    InitializeRequest {
        request: ClientJsonRpcMessage,
        responder: oneshot::Sender<Result<ServerJsonRpcMessage, SessionError>>,
    },
    Close,
}

#[derive(Debug, Clone)]
pub enum SessionQuitReason {
    ServiceTerminated,
    ClientTerminated,
    ExpectInitializeRequest,
    ExpectInitializeResponse,
    Cancelled,
}

#[derive(Debug, Clone)]
pub struct LocalSessionHandle {
    id: SessionId,
    // after all event_tx drop, inner task will be terminated
    event_tx: Sender<SessionEvent>,
}

impl LocalSessionHandle {
    /// Get the session id
    pub fn id(&self) -> &SessionId {
        &self.id
    }

    /// Close the session
    pub async fn close(&self) -> Result<(), SessionError> {
        self.event_tx
            .send(SessionEvent::Close)
            .await
            .map_err(|_| SessionError::SessionServiceTerminated)?;
        Ok(())
    }

    /// Send a message to the session
    pub async fn push_message(
        &self,
        message: ClientJsonRpcMessage,
        http_request_id: Option<HttpRequestId>,
    ) -> Result<(), SessionError> {
        self.event_tx
            .send(SessionEvent::ClientMessage {
                message,
                http_request_id,
            })
            .await
            .map_err(|_| SessionError::SessionServiceTerminated)?;
        Ok(())
    }

    /// establish a channel for a http-request, the corresponded message from server will be
    /// sent through this channel. The channel will be closed when the request is completed,
    /// or you can close it manually by calling [`LocalSessionHandle::close_request_wise_channel`].
    pub async fn establish_request_wise_channel(
        &self,
    ) -> Result<StreamableHttpMessageReceiver, SessionError> {
        let (tx, rx) = tokio::sync::oneshot::channel();
        self.event_tx
            .send(SessionEvent::EstablishRequestWiseChannel { responder: tx })
            .await
            .map_err(|_| SessionError::SessionServiceTerminated)?;
        rx.await
            .map_err(|_| SessionError::SessionServiceTerminated)?
    }

    /// close the http-request wise channel.
    pub async fn close_request_wise_channel(
        &self,
        request_id: HttpRequestId,
    ) -> Result<(), SessionError> {
        let (tx, rx) = tokio::sync::oneshot::channel();
        self.event_tx
            .send(SessionEvent::CloseRequestWiseChannel {
                id: request_id,
                responder: tx,
            })
            .await
            .map_err(|_| SessionError::SessionServiceTerminated)?;
        rx.await
            .map_err(|_| SessionError::SessionServiceTerminated)?
    }

    /// Establish a common channel for general purpose messages.
    pub async fn establish_common_channel(
        &self,
    ) -> Result<StreamableHttpMessageReceiver, SessionError> {
        let (tx, rx) = tokio::sync::oneshot::channel();
        self.event_tx
            .send(SessionEvent::Resume {
                last_event_id: EventId {
                    http_request_id: None,
                    index: 0,
                },
                responder: tx,
            })
            .await
            .map_err(|_| SessionError::SessionServiceTerminated)?;
        rx.await
            .map_err(|_| SessionError::SessionServiceTerminated)?
    }

    /// Resume streaming response by the last event id. This is suitable for both request wise and common channel.
    pub async fn resume(
        &self,
        last_event_id: EventId,
    ) -> Result<StreamableHttpMessageReceiver, SessionError> {
        let (tx, rx) = tokio::sync::oneshot::channel();
        self.event_tx
            .send(SessionEvent::Resume {
                last_event_id,
                responder: tx,
            })
            .await
            .map_err(|_| SessionError::SessionServiceTerminated)?;
        rx.await
            .map_err(|_| SessionError::SessionServiceTerminated)?
    }

    /// Send an initialize request to the session. And wait for the initialized response.
    ///
    /// This is used to establish a session with the server.
    pub async fn initialize(
        &self,
        request: ClientJsonRpcMessage,
    ) -> Result<ServerJsonRpcMessage, SessionError> {
        let (tx, rx) = tokio::sync::oneshot::channel();
        self.event_tx
            .send(SessionEvent::InitializeRequest {
                request,
                responder: tx,
            })
            .await
            .map_err(|_| SessionError::SessionServiceTerminated)?;
        rx.await
            .map_err(|_| SessionError::SessionServiceTerminated)?
    }
}

pub type SessionTransport = WorkerTransport<LocalSessionWorker>;

impl Worker for LocalSessionWorker {
    type Error = SessionError;
    type Role = RoleServer;
    fn err_closed() -> Self::Error {
        SessionError::TransportClosed
    }
    fn err_join(e: tokio::task::JoinError) -> Self::Error {
        SessionError::TokioJoinError(e)
    }
    fn config(&self) -> crate::transport::worker::WorkerConfig {
        crate::transport::worker::WorkerConfig {
            name: Some(format!("streamable-http-session-{}", self.id)),
            channel_buffer_capacity: self.session_config.channel_capacity,
        }
    }
    #[instrument(name = "streamable_http_session", skip_all, fields(id = self.id.as_ref()))]
    async fn run(mut self, mut context: WorkerContext<Self>) -> Result<(), WorkerQuitReason> {
        enum InnerEvent {
            FromHttpService(SessionEvent),
            FromHandler(WorkerSendRequest<LocalSessionWorker>),
        }
        // waiting for initialize request
        let evt = self.event_rx.recv().await.ok_or_else(|| {
            WorkerQuitReason::fatal("transport terminated", "get initialize request")
        })?;
        let SessionEvent::InitializeRequest { request, responder } = evt else {
            return Err(WorkerQuitReason::fatal(
                "unexpected message",
                "get initialize request",
            ));
        };
        context.send_to_handler(request).await?;
        let send_initialize_response = context.recv_from_handler().await?;
        responder
            .send(Ok(send_initialize_response.message))
            .map_err(|_| {
                WorkerQuitReason::fatal(
                    "failed to send initialize response to http service",
                    "send initialize response",
                )
            })?;
        send_initialize_response
            .responder
            .send(Ok(()))
            .map_err(|_| WorkerQuitReason::HandlerTerminated)?;
        let ct = context.cancellation_token.clone();
        let keep_alive = self.session_config.keep_alive.unwrap_or(Duration::MAX);
        loop {
            let keep_alive_timeout = tokio::time::sleep(keep_alive);
            let event = tokio::select! {
                event = self.event_rx.recv() => {
                    if let Some(event) = event {
                        InnerEvent::FromHttpService(event)
                    } else {
                        return Err(WorkerQuitReason::fatal("session dropped", "waiting next session event"))
                    }
                },
                from_handler = context.recv_from_handler() => {
                    InnerEvent::FromHandler(from_handler?)
                }
                _ = ct.cancelled() => {
                    return Err(WorkerQuitReason::Cancelled)
                }
                _ = keep_alive_timeout => {
                    return Err(WorkerQuitReason::fatal("keep live timeout", "poll next session event"))
                }
            };
            match event {
                InnerEvent::FromHandler(WorkerSendRequest { message, responder }) => {
                    // catch response
                    let to_unregister = match &message {
                        crate::model::JsonRpcMessage::Response(json_rpc_response) => {
                            let request_id = json_rpc_response.id.clone();
                            Some(ResourceKey::McpRequestId(request_id))
                        }
                        crate::model::JsonRpcMessage::Error(json_rpc_error) => {
                            let request_id = json_rpc_error.id.clone();
                            Some(ResourceKey::McpRequestId(request_id))
                        }
                        _ => {
                            None
                            // no need to unregister resource
                        }
                    };
                    let handle_result = self.handle_server_message(message).await;
                    let _ = responder.send(handle_result).inspect_err(|error| {
                        tracing::warn!(?error, "failed to send message to http service handler");
                    });
                    if let Some(to_unregister) = to_unregister {
                        self.unregister_resource(&to_unregister);
                    }
                }
                InnerEvent::FromHttpService(SessionEvent::ClientMessage {
                    message: json_rpc_message,
                    http_request_id,
                }) => {
                    match &json_rpc_message {
                        crate::model::JsonRpcMessage::Request(request) => {
                            if let Some(http_request_id) = http_request_id {
                                self.register_request(request, http_request_id)
                            }
                        }
                        crate::model::JsonRpcMessage::Notification(notification) => {
                            self.catch_cancellation_notification(notification)
                        }
                        crate::model::JsonRpcMessage::BatchRequest(items) => {
                            for r in items {
                                match r {
                                    crate::model::JsonRpcBatchRequestItem::Request(request) => {
                                        if let Some(http_request_id) = http_request_id {
                                            self.register_request(request, http_request_id)
                                        }
                                    }
                                    crate::model::JsonRpcBatchRequestItem::Notification(
                                        notification,
                                    ) => self.catch_cancellation_notification(notification),
                                }
                            }
                        }
                        _ => {}
                    }
                    context.send_to_handler(json_rpc_message).await?;
                }
                InnerEvent::FromHttpService(SessionEvent::EstablishRequestWiseChannel {
                    responder,
                }) => {
                    let handle_result = self.establish_request_wise_channel().await;
                    let _ = responder.send(handle_result);
                }
                InnerEvent::FromHttpService(SessionEvent::CloseRequestWiseChannel {
                    id,
                    responder,
                }) => {
                    let _handle_result = self.tx_router.remove(&id);
                    let _ = responder.send(Ok(()));
                }
                InnerEvent::FromHttpService(SessionEvent::Resume {
                    last_event_id,
                    responder,
                }) => {
                    let handle_result = self.resume(last_event_id).await;
                    let _ = responder.send(handle_result);
                }
                InnerEvent::FromHttpService(SessionEvent::Close) => {
                    return Err(WorkerQuitReason::TransportClosed);
                }
                _ => {
                    // ignore
                }
            }
        }
    }
}

#[derive(Debug, Clone)]
pub struct SessionConfig {
    /// the capacity of the channel for the session. Default is 16.
    pub channel_capacity: usize,
    /// if set, the session will be closed after this duration of inactivity.
    pub keep_alive: Option<Duration>,
}

impl SessionConfig {
    pub const DEFAULT_CHANNEL_CAPACITY: usize = 16;
}

impl Default for SessionConfig {
    fn default() -> Self {
        Self {
            channel_capacity: Self::DEFAULT_CHANNEL_CAPACITY,
            keep_alive: None,
        }
    }
}

/// Create a new session with the given id and configuration.
///
/// This function will return a pair of [`LocalSessionHandle`] and [`LocalSessionWorker`].
///
/// You can run the [`LocalSessionWorker`] as a transport for mcp server. And use the [`LocalSessionHandle`] operate the session.
pub fn create_local_session(
    id: impl Into<SessionId>,
    config: SessionConfig,
) -> (LocalSessionHandle, LocalSessionWorker) {
    let id = id.into();
    let (event_tx, event_rx) = tokio::sync::mpsc::channel(config.channel_capacity);
    let (common_tx, _) = tokio::sync::mpsc::channel(config.channel_capacity);
    let common = CachedTx::new_common(common_tx);
    tracing::info!(session_id = ?id, "create new session");
    let handle = LocalSessionHandle {
        event_tx,
        id: id.clone(),
    };
    let session_worker = LocalSessionWorker {
        next_http_request_id: 0,
        id,
        tx_router: HashMap::new(),
        resource_router: HashMap::new(),
        common,
        event_rx,
        session_config: config.clone(),
    };
    (handle, session_worker)
}

---
File: modelcontextprotocol-rust-sdk-13e72ec/crates/rmcp/src/transport/streamable_http_server/session/never.rs
---

use futures::Stream;
use thiserror::Error;

use super::{ServerSseMessage, SessionId, SessionManager};
use crate::{
    RoleServer,
    model::{ClientJsonRpcMessage, ServerJsonRpcMessage},
    transport::Transport,
};

#[derive(Debug, Clone, Error)]
#[error("Session management is not supported")]
pub struct ErrorSessionManagementNotSupported;
#[derive(Debug, Clone, Default)]
pub struct NeverSessionManager {}
pub enum NeverTransport {}
impl Transport<RoleServer> for NeverTransport {
    type Error = ErrorSessionManagementNotSupported;

    fn send(
        &mut self,
        _item: ServerJsonRpcMessage,
    ) -> impl Future<Output = Result<(), Self::Error>> + Send + 'static {
        futures::future::ready(Err(ErrorSessionManagementNotSupported))
    }

    fn receive(&mut self) -> impl Future<Output = Option<ClientJsonRpcMessage>> {
        futures::future::ready(None)
    }

    async fn close(&mut self) -> Result<(), Self::Error> {
        Err(ErrorSessionManagementNotSupported)
    }
}

impl SessionManager for NeverSessionManager {
    type Error = ErrorSessionManagementNotSupported;
    type Transport = NeverTransport;

    fn create_session(
        &self,
    ) -> impl Future<Output = Result<(SessionId, Self::Transport), Self::Error>> + Send {
        futures::future::ready(Err(ErrorSessionManagementNotSupported))
    }

    fn initialize_session(
        &self,
        _id: &SessionId,
        _message: ClientJsonRpcMessage,
    ) -> impl Future<Output = Result<ServerJsonRpcMessage, Self::Error>> + Send {
        futures::future::ready(Err(ErrorSessionManagementNotSupported))
    }

    fn has_session(
        &self,
        _id: &SessionId,
    ) -> impl Future<Output = Result<bool, Self::Error>> + Send {
        futures::future::ready(Err(ErrorSessionManagementNotSupported))
    }

    fn close_session(
        &self,
        _id: &SessionId,
    ) -> impl Future<Output = Result<(), Self::Error>> + Send {
        futures::future::ready(Err(ErrorSessionManagementNotSupported))
    }

    fn create_stream(
        &self,
        _id: &SessionId,
        _message: ClientJsonRpcMessage,
    ) -> impl Future<
        Output = Result<impl Stream<Item = ServerSseMessage> + Send + 'static, Self::Error>,
    > + Send {
        futures::future::ready(Result::<futures::stream::Pending<_>, _>::Err(
            ErrorSessionManagementNotSupported,
        ))
    }
    fn create_standalone_stream(
        &self,
        _id: &SessionId,
    ) -> impl Future<
        Output = Result<impl Stream<Item = ServerSseMessage> + Send + 'static, Self::Error>,
    > + Send {
        futures::future::ready(Result::<futures::stream::Pending<_>, _>::Err(
            ErrorSessionManagementNotSupported,
        ))
    }
    fn resume(
        &self,
        _id: &SessionId,
        _last_event_id: String,
    ) -> impl Future<
        Output = Result<impl Stream<Item = ServerSseMessage> + Send + 'static, Self::Error>,
    > + Send {
        futures::future::ready(Result::<futures::stream::Pending<_>, _>::Err(
            ErrorSessionManagementNotSupported,
        ))
    }
    fn accept_message(
        &self,
        _id: &SessionId,
        _message: ClientJsonRpcMessage,
    ) -> impl Future<Output = Result<(), Self::Error>> + Send {
        futures::future::ready(Err(ErrorSessionManagementNotSupported))
    }
}

---
File: modelcontextprotocol-rust-sdk-13e72ec/crates/rmcp/src/transport/streamable_http_server/tower.rs
---

use std::{convert::Infallible, fmt::Display, sync::Arc, time::Duration};

use bytes::Bytes;
use futures::{StreamExt, future::BoxFuture};
use http::{Method, Request, Response, header::ALLOW};
use http_body::Body;
use http_body_util::{BodyExt, Full, combinators::UnsyncBoxBody};
use tokio_stream::wrappers::ReceiverStream;

use super::session::SessionManager;
use crate::{
    RoleServer,
    model::{ClientJsonRpcMessage, GetExtensions},
    serve_server,
    service::serve_directly,
    transport::{
        OneshotTransport, TransportAdapterIdentity,
        common::{
            http_header::{
                EVENT_STREAM_MIME_TYPE, HEADER_LAST_EVENT_ID, HEADER_SESSION_ID, JSON_MIME_TYPE,
            },
            server_side_http::{
                BoxResponse, ServerSseMessage, accepted_response, expect_json,
                internal_error_response, sse_stream_response,
            },
        },
    },
};

#[derive(Debug, Clone)]
pub struct StreamableHttpServerConfig {
    /// The ping message duration for SSE connections.
    pub sse_keep_alive: Option<Duration>,
    /// If true, the server will create a session for each request and keep it alive.
    pub stateful_mode: bool,
}

impl Default for StreamableHttpServerConfig {
    fn default() -> Self {
        Self {
            sse_keep_alive: Some(Duration::from_secs(15)),
            stateful_mode: true,
        }
    }
}

pub struct StreamableHttpService<S, M = super::session::local::LocalSessionManager> {
    pub config: StreamableHttpServerConfig,
    session_manager: Arc<M>,
    service_factory: Arc<dyn Fn() -> Result<S, std::io::Error> + Send + Sync>,
}

impl<S, M> Clone for StreamableHttpService<S, M> {
    fn clone(&self) -> Self {
        Self {
            config: self.config.clone(),
            session_manager: self.session_manager.clone(),
            service_factory: self.service_factory.clone(),
        }
    }
}

impl<RequestBody, S, M> tower_service::Service<Request<RequestBody>> for StreamableHttpService<S, M>
where
    RequestBody: Body + Send + 'static,
    S: crate::Service<RoleServer>,
    M: SessionManager,
    RequestBody::Error: Display,
    RequestBody::Data: Send + 'static,
{
    type Response = BoxResponse;
    type Error = Infallible;
    type Future = BoxFuture<'static, Result<Self::Response, Self::Error>>;
    fn call(&mut self, req: http::Request<RequestBody>) -> Self::Future {
        let service = self.clone();
        Box::pin(async move {
            let response = service.handle(req).await;
            Ok(response)
        })
    }
    fn poll_ready(
        &mut self,
        _cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), Self::Error>> {
        std::task::Poll::Ready(Ok(()))
    }
}

impl<S, M> StreamableHttpService<S, M>
where
    S: crate::Service<RoleServer> + Send + 'static,
    M: SessionManager,
{
    pub fn new(
        service_factory: impl Fn() -> Result<S, std::io::Error> + Send + Sync + 'static,
        session_manager: Arc<M>,
        config: StreamableHttpServerConfig,
    ) -> Self {
        Self {
            config,
            session_manager,
            service_factory: Arc::new(service_factory),
        }
    }
    fn get_service(&self) -> Result<S, std::io::Error> {
        (self.service_factory)()
    }
    pub async fn handle<B>(&self, request: Request<B>) -> Response<UnsyncBoxBody<Bytes, Infallible>>
    where
        B: Body + Send + 'static,
        B::Error: Display,
    {
        let method = request.method().clone();
        let result = match method {
            Method::GET => self.handle_get(request).await,
            Method::POST => self.handle_post(request).await,
            Method::DELETE => self.handle_delete(request).await,
            _ => {
                // Handle other methods or return an error
                let response = Response::builder()
                    .status(http::StatusCode::METHOD_NOT_ALLOWED)
                    .header(ALLOW, "GET, POST, DELETE")
                    .body(Full::new(Bytes::from("Method Not Allowed")).boxed_unsync())
                    .expect("valid response");
                return response;
            }
        };
        match result {
            Ok(response) => response,
            Err(response) => response,
        }
    }
    async fn handle_get<B>(&self, request: Request<B>) -> Result<BoxResponse, BoxResponse>
    where
        B: Body + Send + 'static,
        B::Error: Display,
    {
        // check accept header
        if !request
            .headers()
            .get(http::header::ACCEPT)
            .and_then(|header| header.to_str().ok())
            .is_some_and(|header| header.contains(EVENT_STREAM_MIME_TYPE))
        {
            return Ok(Response::builder()
                .status(http::StatusCode::NOT_ACCEPTABLE)
                .body(
                    Full::new(Bytes::from(
                        "Not Acceptable: Client must accept text/event-stream",
                    ))
                    .boxed_unsync(),
                )
                .expect("valid response"));
        }
        // check session id
        let session_id = request
            .headers()
            .get(HEADER_SESSION_ID)
            .and_then(|v| v.to_str().ok())
            .map(|s| s.to_owned().into());
        let Some(session_id) = session_id else {
            // unauthorized
            return Ok(Response::builder()
                .status(http::StatusCode::UNAUTHORIZED)
                .body(Full::new(Bytes::from("Unauthorized: Session ID is required")).boxed_unsync())
                .expect("valid response"));
        };
        // check if session exists
        let has_session = self
            .session_manager
            .has_session(&session_id)
            .await
            .map_err(internal_error_response("check session"))?;
        if !has_session {
            // unauthorized
            return Ok(Response::builder()
                .status(http::StatusCode::UNAUTHORIZED)
                .body(Full::new(Bytes::from("Unauthorized: Session not found")).boxed_unsync())
                .expect("valid response"));
        }
        // check if last event id is provided
        let last_event_id = request
            .headers()
            .get(HEADER_LAST_EVENT_ID)
            .and_then(|v| v.to_str().ok())
            .map(|s| s.to_owned());
        if let Some(last_event_id) = last_event_id {
            // check if session has this event id
            let stream = self
                .session_manager
                .resume(&session_id, last_event_id)
                .await
                .map_err(internal_error_response("resume session"))?;
            Ok(sse_stream_response(stream, self.config.sse_keep_alive))
        } else {
            // create standalone stream
            let stream = self
                .session_manager
                .create_standalone_stream(&session_id)
                .await
                .map_err(internal_error_response("create standalone stream"))?;
            Ok(sse_stream_response(stream, self.config.sse_keep_alive))
        }
    }

    async fn handle_post<B>(&self, request: Request<B>) -> Result<BoxResponse, BoxResponse>
    where
        B: Body + Send + 'static,
        B::Error: Display,
    {
        // check accept header
        if !request
            .headers()
            .get(http::header::ACCEPT)
            .and_then(|header| header.to_str().ok())
            .is_some_and(|header| {
                header.contains(JSON_MIME_TYPE) && header.contains(EVENT_STREAM_MIME_TYPE)
            })
        {
            return Ok(Response::builder()
                .status(http::StatusCode::NOT_ACCEPTABLE)
                .body(Full::new(Bytes::from("Not Acceptable: Client must accept both application/json and text/event-stream")).boxed_unsync())
                .expect("valid response"));
        }

        // check content type
        if !request
            .headers()
            .get(http::header::CONTENT_TYPE)
            .and_then(|header| header.to_str().ok())
            .is_some_and(|header| header.starts_with(JSON_MIME_TYPE))
        {
            return Ok(Response::builder()
                .status(http::StatusCode::UNSUPPORTED_MEDIA_TYPE)
                .body(
                    Full::new(Bytes::from(
                        "Unsupported Media Type: Content-Type must be application/json",
                    ))
                    .boxed_unsync(),
                )
                .expect("valid response"));
        }

        // json deserialize request body
        let (part, body) = request.into_parts();
        let mut message = match expect_json(body).await {
            Ok(message) => message,
            Err(response) => return Ok(response),
        };

        if self.config.stateful_mode {
            // do we have a session id?
            let session_id = part
                .headers
                .get(HEADER_SESSION_ID)
                .and_then(|v| v.to_str().ok());
            if let Some(session_id) = session_id {
                let session_id = session_id.to_owned().into();
                let has_session = self
                    .session_manager
                    .has_session(&session_id)
                    .await
                    .map_err(internal_error_response("check session"))?;
                if !has_session {
                    // unauthorized
                    return Ok(Response::builder()
                        .status(http::StatusCode::UNAUTHORIZED)
                        .body(
                            Full::new(Bytes::from("Unauthorized: Session not found"))
                                .boxed_unsync(),
                        )
                        .expect("valid response"));
                }

                // inject request part to extensions
                match &mut message {
                    ClientJsonRpcMessage::Request(req) => {
                        req.request.extensions_mut().insert(part);
                    }
                    ClientJsonRpcMessage::Notification(not) => {
                        not.notification.extensions_mut().insert(part);
                    }
                    _ => {
                        // skip
                    }
                }

                match message {
                    ClientJsonRpcMessage::Request(_) => {
                        let stream = self
                            .session_manager
                            .create_stream(&session_id, message)
                            .await
                            .map_err(internal_error_response("get session"))?;
                        Ok(sse_stream_response(stream, self.config.sse_keep_alive))
                    }
                    ClientJsonRpcMessage::Notification(_)
                    | ClientJsonRpcMessage::Response(_)
                    | ClientJsonRpcMessage::Error(_) => {
                        // handle notification
                        self.session_manager
                            .accept_message(&session_id, message)
                            .await
                            .map_err(internal_error_response("accept message"))?;
                        Ok(accepted_response())
                    }
                    _ => Ok(Response::builder()
                        .status(http::StatusCode::NOT_IMPLEMENTED)
                        .body(
                            Full::new(Bytes::from("Batch requests are not supported yet"))
                                .boxed_unsync(),
                        )
                        .expect("valid response")),
                }
            } else {
                let (session_id, transport) = self
                    .session_manager
                    .create_session()
                    .await
                    .map_err(internal_error_response("create session"))?;
                let service = self
                    .get_service()
                    .map_err(internal_error_response("get service"))?;
                // spawn a task to serve the session
                tokio::spawn({
                    let session_manager = self.session_manager.clone();
                    let session_id = session_id.clone();
                    async move {
                        let service = serve_server::<S, M::Transport, _, TransportAdapterIdentity>(
                            service, transport,
                        )
                        .await;
                        match service {
                            Ok(service) => {
                                // on service created
                                let _ = service.waiting().await;
                            }
                            Err(e) => {
                                tracing::error!("Failed to create service: {e}");
                            }
                        }
                        let _ = session_manager
                            .close_session(&session_id)
                            .await
                            .inspect_err(|e| {
                                tracing::error!("Failed to close session {session_id}: {e}");
                            });
                    }
                });
                // get initialize response
                let response = self
                    .session_manager
                    .initialize_session(&session_id, message)
                    .await
                    .map_err(internal_error_response("create stream"))?;
                let mut response = sse_stream_response(
                    futures::stream::once({
                        async move {
                            ServerSseMessage {
                                event_id: None,
                                message: response.into(),
                            }
                        }
                    }),
                    self.config.sse_keep_alive,
                );

                response.headers_mut().insert(
                    HEADER_SESSION_ID,
                    session_id
                        .parse()
                        .map_err(internal_error_response("create session id header"))?,
                );
                Ok(response)
            }
        } else {
            let service = self
                .get_service()
                .map_err(internal_error_response("get service"))?;
            match message {
                ClientJsonRpcMessage::Request(request) => {
                    let (transport, receiver) =
                        OneshotTransport::<RoleServer>::new(ClientJsonRpcMessage::Request(request));
                    let service = serve_directly(service, transport, None);
                    tokio::spawn(async move {
                        // on service created
                        let _ = service.waiting().await;
                    });
                    Ok(sse_stream_response(
                        ReceiverStream::new(receiver).map(|message| {
                            tracing::info!(?message);
                            ServerSseMessage {
                                event_id: None,
                                message: message.into(),
                            }
                        }),
                        self.config.sse_keep_alive,
                    ))
                }
                ClientJsonRpcMessage::Notification(_notification) => {
                    // ignore
                    Ok(accepted_response())
                }
                ClientJsonRpcMessage::Response(_json_rpc_response) => Ok(accepted_response()),
                ClientJsonRpcMessage::Error(_json_rpc_error) => Ok(accepted_response()),
                _ => Ok(Response::builder()
                    .status(http::StatusCode::NOT_IMPLEMENTED)
                    .body(
                        Full::new(Bytes::from("Batch requests are not supported yet"))
                            .boxed_unsync(),
                    )
                    .expect("valid response")),
            }
        }
    }

    async fn handle_delete<B>(&self, request: Request<B>) -> Result<BoxResponse, BoxResponse>
    where
        B: Body + Send + 'static,
        B::Error: Display,
    {
        // check session id
        let session_id = request
            .headers()
            .get(HEADER_SESSION_ID)
            .and_then(|v| v.to_str().ok())
            .map(|s| s.to_owned().into());
        let Some(session_id) = session_id else {
            // unauthorized
            return Ok(Response::builder()
                .status(http::StatusCode::UNAUTHORIZED)
                .body(Full::new(Bytes::from("Unauthorized: Session ID is required")).boxed_unsync())
                .expect("valid response"));
        };
        // close session
        self.session_manager
            .close_session(&session_id)
            .await
            .map_err(internal_error_response("close session"))?;
        Ok(accepted_response())
    }
}

---
File: modelcontextprotocol-rust-sdk-13e72ec/crates/rmcp/src/transport/worker.rs
---

use std::borrow::Cow;

use tokio_util::sync::CancellationToken;
use tracing::{Instrument, Level};

use super::{IntoTransport, Transport};
use crate::service::{RxJsonRpcMessage, ServiceRole, TxJsonRpcMessage};

#[derive(Debug, thiserror::Error)]
pub enum WorkerQuitReason {
    #[error("Join error {0}")]
    Join(#[from] tokio::task::JoinError),
    #[error("Transport fatal {error}, when {context}")]
    Fatal {
        error: Cow<'static, str>,
        context: Cow<'static, str>,
    },
    #[error("Transport canncelled")]
    Cancelled,
    #[error("Transport closed")]
    TransportClosed,
    #[error("Handler terminated")]
    HandlerTerminated,
}

impl WorkerQuitReason {
    pub fn fatal(msg: impl Into<Cow<'static, str>>, context: impl Into<Cow<'static, str>>) -> Self {
        Self::Fatal {
            error: msg.into(),
            context: context.into(),
        }
    }
    pub fn fatal_context<E: std::error::Error>(
        context: impl Into<Cow<'static, str>>,
    ) -> impl FnOnce(E) -> Self {
        |e| Self::Fatal {
            error: Cow::Owned(format!("{e}")),
            context: context.into(),
        }
    }
}

pub trait Worker: Sized + Send + 'static {
    type Error: std::error::Error + Send + Sync + 'static;
    type Role: ServiceRole;
    fn err_closed() -> Self::Error;
    fn err_join(e: tokio::task::JoinError) -> Self::Error;
    fn run(
        self,
        context: WorkerContext<Self>,
    ) -> impl Future<Output = Result<(), WorkerQuitReason>> + Send;
    fn config(&self) -> WorkerConfig {
        WorkerConfig::default()
    }
}

pub struct WorkerSendRequest<W: Worker> {
    pub message: TxJsonRpcMessage<W::Role>,
    pub responder: tokio::sync::oneshot::Sender<Result<(), W::Error>>,
}

pub struct WorkerTransport<W: Worker> {
    rx: tokio::sync::mpsc::Receiver<RxJsonRpcMessage<W::Role>>,
    send_service: tokio::sync::mpsc::Sender<WorkerSendRequest<W>>,
    join_handle: Option<tokio::task::JoinHandle<Result<(), WorkerQuitReason>>>,
    _drop_guard: tokio_util::sync::DropGuard,
    ct: CancellationToken,
}

pub struct WorkerConfig {
    pub name: Option<String>,
    pub channel_buffer_capacity: usize,
}

impl Default for WorkerConfig {
    fn default() -> Self {
        Self {
            name: None,
            channel_buffer_capacity: 16,
        }
    }
}
pub enum WorkerAdapter {}

impl<W: Worker> IntoTransport<W::Role, W::Error, WorkerAdapter> for W {
    fn into_transport(self) -> impl Transport<W::Role, Error = W::Error> + 'static {
        WorkerTransport::spawn(self)
    }
}

impl<W: Worker> WorkerTransport<W> {
    pub fn cancel_token(&self) -> CancellationToken {
        self.ct.clone()
    }
    pub fn spawn(worker: W) -> Self {
        Self::spawn_with_ct(worker, CancellationToken::new())
    }
    pub fn spawn_with_ct(worker: W, transport_task_ct: CancellationToken) -> Self {
        let config = worker.config();
        let worker_name = config.name;
        let (to_transport_tx, from_handler_rx) =
            tokio::sync::mpsc::channel::<WorkerSendRequest<W>>(config.channel_buffer_capacity);
        let (to_handler_tx, from_transport_rx) =
            tokio::sync::mpsc::channel::<RxJsonRpcMessage<W::Role>>(config.channel_buffer_capacity);
        let context = WorkerContext {
            to_handler_tx,
            from_handler_rx,
            cancellation_token: transport_task_ct.clone(),
        };

        let join_handle = tokio::spawn(async move {
            worker
                .run(context)
                .instrument(tracing::span!(
                    Level::TRACE,
                    "transport_worker",
                    name = worker_name,
                ))
                .await
                .inspect_err(|e| match e {
                    WorkerQuitReason::Cancelled
                    | WorkerQuitReason::TransportClosed
                    | WorkerQuitReason::HandlerTerminated => {
                        tracing::debug!("worker quit with reason: {:?}", e);
                    }
                    WorkerQuitReason::Join(e) => {
                        tracing::error!("worker quit with join error: {:?}", e);
                    }
                    WorkerQuitReason::Fatal { error, context } => {
                        tracing::error!("worker quit with fatal: {error}, when {context}");
                    }
                })
                .inspect(|_| {
                    tracing::debug!("worker quit");
                })
        });
        Self {
            rx: from_transport_rx,
            send_service: to_transport_tx,
            join_handle: Some(join_handle),
            ct: transport_task_ct.clone(),
            _drop_guard: transport_task_ct.drop_guard(),
        }
    }
}

pub struct SendRequest<W: Worker> {
    pub message: TxJsonRpcMessage<W::Role>,
    pub responder: tokio::sync::oneshot::Sender<RxJsonRpcMessage<W::Role>>,
}

pub struct WorkerContext<W: Worker> {
    pub to_handler_tx: tokio::sync::mpsc::Sender<RxJsonRpcMessage<W::Role>>,
    pub from_handler_rx: tokio::sync::mpsc::Receiver<WorkerSendRequest<W>>,
    pub cancellation_token: CancellationToken,
}

impl<W: Worker> WorkerContext<W> {
    pub async fn send_to_handler(
        &mut self,
        item: RxJsonRpcMessage<W::Role>,
    ) -> Result<(), WorkerQuitReason> {
        self.to_handler_tx
            .send(item)
            .await
            .map_err(|_| WorkerQuitReason::HandlerTerminated)
    }

    pub async fn recv_from_handler(&mut self) -> Result<WorkerSendRequest<W>, WorkerQuitReason> {
        self.from_handler_rx
            .recv()
            .await
            .ok_or(WorkerQuitReason::HandlerTerminated)
    }
}

impl<W: Worker> Transport<W::Role> for WorkerTransport<W> {
    type Error = W::Error;

    fn send(
        &mut self,
        item: TxJsonRpcMessage<W::Role>,
    ) -> impl Future<Output = Result<(), Self::Error>> + Send + 'static {
        let tx = self.send_service.clone();
        let (responder, receiver) = tokio::sync::oneshot::channel();
        let request = WorkerSendRequest {
            message: item,
            responder,
        };
        async move {
            tx.send(request).await.map_err(|_| W::err_closed())?;
            receiver.await.map_err(|_| W::err_closed())??;
            Ok(())
        }
    }
    async fn receive(&mut self) -> Option<RxJsonRpcMessage<W::Role>> {
        self.rx.recv().await
    }
    async fn close(&mut self) -> Result<(), Self::Error> {
        if let Some(handle) = self.join_handle.take() {
            self.ct.cancel();
            let _quit_reason = handle.await.map_err(W::err_join)?;
            Ok(())
        } else {
            Ok(())
        }
    }
}

---
File: modelcontextprotocol-rust-sdk-13e72ec/crates/rmcp/src/transport/ws.rs
---

// Maybe we don't really need a ws implementation?

---
File: modelcontextprotocol-rust-sdk-13e72ec/crates/rmcp/tests/common/calculator.rs
---

use rmcp::{
    ServerHandler,
    model::{ServerCapabilities, ServerInfo},
    schemars, tool,
};
#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct SumRequest {
    #[schemars(description = "the left hand side number")]
    pub a: i32,
    pub b: i32,
}
#[derive(Debug, Clone, Default)]
pub struct Calculator;
#[tool(tool_box)]
impl Calculator {
    #[tool(description = "Calculate the sum of two numbers")]
    fn sum(&self, #[tool(aggr)] SumRequest { a, b }: SumRequest) -> String {
        (a + b).to_string()
    }

    #[tool(description = "Calculate the sub of two numbers")]
    fn sub(
        &self,
        #[tool(param)]
        #[schemars(description = "the left hand side number")]
        a: i32,
        #[tool(param)]
        #[schemars(description = "the right hand side number")]
        b: i32,
    ) -> String {
        (a - b).to_string()
    }
}

#[tool(tool_box)]
impl ServerHandler for Calculator {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            instructions: Some("A simple calculator".into()),
            capabilities: ServerCapabilities::builder().enable_tools().build(),
            ..Default::default()
        }
    }
}

---
File: modelcontextprotocol-rust-sdk-13e72ec/crates/rmcp/tests/common/handlers.rs
---

use std::{
    future::Future,
    sync::{Arc, Mutex},
};

use rmcp::{
    ClientHandler, Error as McpError, RoleClient, RoleServer, ServerHandler,
    model::*,
    service::{NotificationContext, RequestContext},
};
use serde_json::json;
use tokio::sync::Notify;

#[derive(Clone)]
pub struct TestClientHandler {
    pub honor_this_server: bool,
    pub honor_all_servers: bool,
    pub receive_signal: Arc<Notify>,
    pub received_messages: Arc<Mutex<Vec<LoggingMessageNotificationParam>>>,
}

impl TestClientHandler {
    #[allow(dead_code)]
    pub fn new(honor_this_server: bool, honor_all_servers: bool) -> Self {
        Self {
            honor_this_server,
            honor_all_servers,
            receive_signal: Arc::new(Notify::new()),
            received_messages: Arc::new(Mutex::new(Vec::new())),
        }
    }

    #[allow(dead_code)]
    pub fn with_notification(
        honor_this_server: bool,
        honor_all_servers: bool,
        receive_signal: Arc<Notify>,
        received_messages: Arc<Mutex<Vec<LoggingMessageNotificationParam>>>,
    ) -> Self {
        Self {
            honor_this_server,
            honor_all_servers,
            receive_signal,
            received_messages,
        }
    }
}

impl ClientHandler for TestClientHandler {
    async fn create_message(
        &self,
        params: CreateMessageRequestParam,
        _context: RequestContext<RoleClient>,
    ) -> Result<CreateMessageResult, McpError> {
        // First validate that there's at least one User message
        if !params.messages.iter().any(|msg| msg.role == Role::User) {
            return Err(McpError::invalid_request(
                "Message sequence must contain at least one user message",
                Some(json!({"messages": params.messages})),
            ));
        }

        // Create response based on context inclusion
        let response = match params.include_context {
            Some(ContextInclusion::ThisServer) if self.honor_this_server => {
                "Test response with context: test context"
            }
            Some(ContextInclusion::AllServers) if self.honor_all_servers => {
                "Test response with context: test context"
            }
            _ => "Test response without context",
        };

        Ok(CreateMessageResult {
            message: SamplingMessage {
                role: Role::Assistant,
                content: Content::text(response.to_string()),
            },
            model: "test-model".to_string(),
            stop_reason: Some(CreateMessageResult::STOP_REASON_END_TURN.to_string()),
        })
    }

    fn on_logging_message(
        &self,
        params: LoggingMessageNotificationParam,
        _context: NotificationContext<RoleClient>,
    ) -> impl Future<Output = ()> + Send + '_ {
        let receive_signal = self.receive_signal.clone();
        let received_messages = self.received_messages.clone();

        async move {
            println!("Client: Received log message: {:?}", params);
            let mut messages = received_messages.lock().unwrap();
            messages.push(params);
            receive_signal.notify_one();
        }
    }
}

pub struct TestServer {}

impl TestServer {
    #[allow(dead_code)]
    pub fn new() -> Self {
        Self {}
    }
}

impl ServerHandler for TestServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            capabilities: ServerCapabilities::builder().enable_logging().build(),
            ..Default::default()
        }
    }

    fn set_level(
        &self,
        request: SetLevelRequestParam,
        context: RequestContext<RoleServer>,
    ) -> impl Future<Output = Result<(), McpError>> + Send + '_ {
        let peer = context.peer;
        async move {
            let (data, logger) = match request.level {
                LoggingLevel::Error => (
                    serde_json::json!({
                        "message": "Failed to process request",
                        "error_code": "E1001",
                        "error_details": "Connection timeout",
                        "timestamp": chrono::Utc::now().to_rfc3339(),
                    }),
                    Some("error_handler".to_string()),
                ),
                LoggingLevel::Debug => (
                    serde_json::json!({
                        "message": "Processing request",
                        "function": "handle_request",
                        "line": 42,
                        "context": {
                            "request_id": "req-123",
                            "user_id": "user-456"
                        },
                        "timestamp": chrono::Utc::now().to_rfc3339(),
                    }),
                    Some("debug_logger".to_string()),
                ),
                LoggingLevel::Info => (
                    serde_json::json!({
                        "message": "System status update",
                        "status": "healthy",
                        "metrics": {
                            "requests_per_second": 150,
                            "average_latency_ms": 45,
                            "error_rate": 0.01
                        },
                        "timestamp": chrono::Utc::now().to_rfc3339(),
                    }),
                    Some("monitoring".to_string()),
                ),
                _ => (
                    serde_json::json!({
                        "message": format!("Message at level {:?}", request.level),
                        "timestamp": chrono::Utc::now().to_rfc3339(),
                    }),
                    None,
                ),
            };

            if let Err(e) = peer
                .notify_logging_message(LoggingMessageNotificationParam {
                    level: request.level,
                    data,
                    logger,
                })
                .await
            {
                panic!("Failed to send notification: {}", e);
            }
            Ok(())
        }
    }
}

---
File: modelcontextprotocol-rust-sdk-13e72ec/crates/rmcp/tests/common/mod.rs
---

pub mod calculator;
pub mod handlers;

---
File: modelcontextprotocol-rust-sdk-13e72ec/crates/rmcp/tests/test_complex_schema.rs
---

use rmcp::{Error as McpError, model::*, schemars, tool};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
pub enum ChatRole {
    System,
    User,
    Assistant,
    Tool,
}

#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
pub struct ChatMessage {
    pub role: ChatRole,
    pub content: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
pub struct ChatRequest {
    pub system: Option<String>,
    pub messages: Vec<ChatMessage>,
}

#[derive(Clone, Default)]
pub struct Demo;

#[tool(tool_box)]
impl Demo {
    pub fn new() -> Self {
        Self
    }

    #[tool(description = "LLM")]
    async fn chat(
        &self,
        #[tool(aggr)] chat_request: ChatRequest,
    ) -> Result<CallToolResult, McpError> {
        let content = Content::json(chat_request)?;
        Ok(CallToolResult::success(vec![content]))
    }
}

#[test]
fn test_complex_schema() {
    let attr = Demo::chat_tool_attr();
    let input_schema = attr.input_schema;
    let enum_number = input_schema
        .get("definitions")
        .unwrap()
        .as_object()
        .unwrap()
        .get("ChatRole")
        .unwrap()
        .as_object()
        .unwrap()
        .get("enum")
        .unwrap()
        .as_array()
        .unwrap()
        .len();
    assert_eq!(enum_number, 4);
    println!("{}", serde_json::to_string_pretty(&input_schema).unwrap());
}

---
File: modelcontextprotocol-rust-sdk-13e72ec/crates/rmcp/tests/test_deserialization.rs
---

use rmcp::model::{JsonRpcResponse, ServerJsonRpcMessage, ServerResult};
#[test]
fn test_tool_list_result() {
    let json = std::fs::read("tests/test_deserialization/tool_list_result.json").unwrap();
    let result: ServerJsonRpcMessage = serde_json::from_slice(&json).unwrap();
    println!("{result:#?}");

    assert!(matches!(
        result,
        ServerJsonRpcMessage::Response(JsonRpcResponse {
            result: ServerResult::ListToolsResult(_),
            ..
        })
    ));
}

---
File: modelcontextprotocol-rust-sdk-13e72ec/crates/rmcp/tests/test_deserialization/tool_list_result.json
---

{
    "result": {
        "tools": [
            {
                "name": "add",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "a": {
                            "type": "number"
                        },
                        "b": {
                            "type": "number"
                        }
                    },
                    "required": [
                        "a",
                        "b"
                    ],
                    "additionalProperties": false,
                    "$schema": "http://json-schema.org/draft-07/schema#"
                }
            }
        ]
    },
    "jsonrpc": "2.0",
    "id": 2
}

---
File: modelcontextprotocol-rust-sdk-13e72ec/crates/rmcp/tests/test_logging.rs
---

// cargo test --features "server client" --package rmcp test_logging
mod common;

use std::sync::{Arc, Mutex};

use common::handlers::{TestClientHandler, TestServer};
use rmcp::{
    ServiceExt,
    model::{LoggingLevel, LoggingMessageNotificationParam, SetLevelRequestParam},
};
use serde_json::json;
use tokio::sync::Notify;

#[tokio::test]
async fn test_logging_spec_compliance() -> anyhow::Result<()> {
    let (server_transport, client_transport) = tokio::io::duplex(4096);
    let receive_signal = Arc::new(Notify::new());
    let received_messages = Arc::new(Mutex::new(Vec::<LoggingMessageNotificationParam>::new()));

    // Start server in a separate task
    let server_handle = tokio::spawn(async move {
        let server = TestServer::new().serve(server_transport).await?;

        // Test server can send messages before level is set
        server
            .peer()
            .notify_logging_message(LoggingMessageNotificationParam {
                level: LoggingLevel::Info,
                data: serde_json::json!({
                    "message": "Server initiated message",
                    "timestamp": chrono::Utc::now().to_rfc3339(),
                }),
                logger: Some("test_server".to_string()),
            })
            .await?;

        server.waiting().await?;
        anyhow::Ok(())
    });

    let client = TestClientHandler::with_notification(
        true,
        true,
        receive_signal.clone(),
        received_messages.clone(),
    )
    .serve(client_transport)
    .await?;

    // Wait for the initial server message
    receive_signal.notified().await;
    {
        let mut messages = received_messages.lock().unwrap();
        assert_eq!(messages.len(), 1, "Should receive server-initiated message");
        messages.clear();
    }

    // Test level filtering and message format
    for level in [
        LoggingLevel::Emergency,
        LoggingLevel::Warning,
        LoggingLevel::Debug,
    ] {
        client
            .peer()
            .set_level(SetLevelRequestParam { level })
            .await?;

        // Wait for each message response
        receive_signal.notified().await;

        let mut messages = received_messages.lock().unwrap();
        let msg = messages.last().unwrap();

        // Verify required fields
        assert_eq!(msg.level, level);
        assert!(msg.data.is_object());

        // Verify data format
        let data = msg.data.as_object().unwrap();
        assert!(data.contains_key("message"));
        assert!(data.contains_key("timestamp"));

        // Verify timestamp
        let timestamp = data["timestamp"].as_str().unwrap();
        chrono::DateTime::parse_from_rfc3339(timestamp).expect("RFC3339 timestamp");

        messages.clear();
    }

    // Important: Cancel the client before ending the test
    client.cancel().await?;

    // Wait for server to complete
    server_handle.await??;

    Ok(())
}

#[tokio::test]
async fn test_logging_user_scenarios() -> anyhow::Result<()> {
    let (server_transport, client_transport) = tokio::io::duplex(4096);
    let receive_signal = Arc::new(Notify::new());
    let received_messages = Arc::new(Mutex::new(Vec::<LoggingMessageNotificationParam>::new()));

    let server_handle = tokio::spawn(async move {
        let server = TestServer::new().serve(server_transport).await?;
        server.waiting().await?;
        anyhow::Ok(())
    });

    let client = TestClientHandler::with_notification(
        true,
        true,
        receive_signal.clone(),
        received_messages.clone(),
    )
    .serve(client_transport)
    .await?;

    // Test 1: Error reporting scenario
    client
        .peer()
        .set_level(SetLevelRequestParam {
            level: LoggingLevel::Error,
        })
        .await?;
    receive_signal.notified().await; // Wait for response
    {
        let messages = received_messages.lock().unwrap();
        let msg = &messages[0];
        let data = msg.data.as_object().unwrap();
        assert!(
            data.contains_key("error_code"),
            "Error should have an error code"
        );
        assert!(
            data.contains_key("error_details"),
            "Error should have details"
        );
        assert!(
            data.contains_key("timestamp"),
            "Should know when error occurred"
        );
    }

    // Test 2: Debug scenario
    client
        .peer()
        .set_level(SetLevelRequestParam {
            level: LoggingLevel::Debug,
        })
        .await?;
    receive_signal.notified().await; // Wait for response
    {
        let messages = received_messages.lock().unwrap();
        let msg = messages.last().unwrap();
        let data = msg.data.as_object().unwrap();
        assert!(
            data.contains_key("function"),
            "Debug should show function name"
        );
        assert!(data.contains_key("line"), "Debug should show line number");
        assert!(
            data.contains_key("context"),
            "Debug should show execution context"
        );
    }

    // Test 3: Production monitoring scenario
    client
        .peer()
        .set_level(SetLevelRequestParam {
            level: LoggingLevel::Info,
        })
        .await?;
    receive_signal.notified().await; // Wait for response
    {
        let messages = received_messages.lock().unwrap();
        let msg = messages.last().unwrap();
        let data = msg.data.as_object().unwrap();
        assert!(data.contains_key("status"), "Should show system status");
        assert!(data.contains_key("metrics"), "Should include metrics");
    }

    // Important: Cancel client and wait for server before ending
    client.cancel().await?;
    server_handle.await??;

    Ok(())
}

#[test]
fn test_logging_level_serialization() {
    // Test all levels match spec exactly
    let test_cases = [
        (LoggingLevel::Alert, "alert"),
        (LoggingLevel::Critical, "critical"),
        (LoggingLevel::Debug, "debug"),
        (LoggingLevel::Emergency, "emergency"),
        (LoggingLevel::Error, "error"),
        (LoggingLevel::Info, "info"),
        (LoggingLevel::Notice, "notice"),
        (LoggingLevel::Warning, "warning"),
    ];

    for (level, expected) in test_cases {
        let serialized = serde_json::to_string(&level).unwrap();
        // Remove quotes from serialized string
        let serialized = serialized.trim_matches('"');
        assert_eq!(
            serialized, expected,
            "LoggingLevel::{:?} should serialize to \"{}\"",
            level, expected
        );
    }

    // Test deserialization from spec strings
    for (level, spec_string) in test_cases {
        let deserialized: LoggingLevel =
            serde_json::from_str(&format!("\"{}\"", spec_string)).unwrap();
        assert_eq!(
            deserialized, level,
            "\"{}\" should deserialize to LoggingLevel::{:?}",
            spec_string, level
        );
    }
}

#[tokio::test]
async fn test_logging_edge_cases() -> anyhow::Result<()> {
    let (server_transport, client_transport) = tokio::io::duplex(4096);
    let receive_signal = Arc::new(Notify::new());
    let received_messages = Arc::new(Mutex::new(Vec::<LoggingMessageNotificationParam>::new()));

    let server_handle = tokio::spawn(async move {
        let server = TestServer::new().serve(server_transport).await?;
        server.waiting().await?;
        anyhow::Ok(())
    });

    let client = TestClientHandler::with_notification(
        true,
        true,
        receive_signal.clone(),
        received_messages.clone(),
    )
    .serve(client_transport)
    .await?;

    // Test all logging levels from spec
    for level in [
        LoggingLevel::Alert,
        LoggingLevel::Critical,
        LoggingLevel::Notice, // These weren't tested before
    ] {
        client
            .peer()
            .set_level(SetLevelRequestParam { level })
            .await?;
        receive_signal.notified().await;

        let messages = received_messages.lock().unwrap();
        let msg = messages.last().unwrap();
        assert_eq!(msg.level, level);
    }

    client.cancel().await?;
    server_handle.await??;
    Ok(())
}

#[tokio::test]
async fn test_logging_optional_fields() -> anyhow::Result<()> {
    let (server_transport, client_transport) = tokio::io::duplex(4096);
    let receive_signal = Arc::new(Notify::new());
    let received_messages = Arc::new(Mutex::new(Vec::<LoggingMessageNotificationParam>::new()));

    let server_handle = tokio::spawn(async move {
        let server = TestServer::new().serve(server_transport).await?;

        // Test message with and without optional logger field
        for (level, has_logger) in [(LoggingLevel::Info, true), (LoggingLevel::Debug, false)] {
            server
                .peer()
                .notify_logging_message(LoggingMessageNotificationParam {
                    level,
                    data: json!({"test": "data"}),
                    logger: has_logger.then(|| "test_logger".to_string()),
                })
                .await?;
        }

        server.waiting().await?;
        anyhow::Ok(())
    });

    let client = TestClientHandler::with_notification(
        true,
        true,
        receive_signal.clone(),
        received_messages.clone(),
    )
    .serve(client_transport)
    .await?;

    // Wait for the initial server message
    receive_signal.notified().await;
    {
        let mut messages = received_messages.lock().unwrap();
        assert_eq!(messages.len(), 2, "Should receive two messages");
        messages.clear();
    }

    // Test level filtering and message format
    for level in [LoggingLevel::Info, LoggingLevel::Debug] {
        client
            .peer()
            .set_level(SetLevelRequestParam { level })
            .await?;

        // Wait for each message response
        receive_signal.notified().await;

        let mut messages = received_messages.lock().unwrap();
        let msg = messages.last().unwrap();

        // Verify required fields
        assert_eq!(msg.level, level);
        assert!(msg.data.is_object());

        // Verify data format
        let data = msg.data.as_object().unwrap();
        assert!(data.contains_key("message"));
        assert!(data.contains_key("timestamp"));

        // Verify timestamp
        let timestamp = data["timestamp"].as_str().unwrap();
        chrono::DateTime::parse_from_rfc3339(timestamp).expect("RFC3339 timestamp");

        messages.clear();
    }

    // Important: Cancel the client before ending the test
    client.cancel().await?;

    // Wait for server to complete
    server_handle.await??;

    Ok(())
}

---
File: modelcontextprotocol-rust-sdk-13e72ec/crates/rmcp/tests/test_message_protocol.rs
---

//cargo test --test test_message_protocol --features "client server"

mod common;
use common::handlers::{TestClientHandler, TestServer};
use rmcp::{
    ServiceExt,
    model::*,
    service::{RequestContext, Service},
};
use tokio_util::sync::CancellationToken;

// Tests start here
#[tokio::test]
async fn test_message_roles() {
    let messages = vec![
        SamplingMessage {
            role: Role::User,
            content: Content::text("user message"),
        },
        SamplingMessage {
            role: Role::Assistant,
            content: Content::text("assistant message"),
        },
    ];

    // Verify all roles can be serialized/deserialized correctly
    let json = serde_json::to_string(&messages).unwrap();
    let deserialized: Vec<SamplingMessage> = serde_json::from_str(&json).unwrap();
    assert_eq!(messages, deserialized);
}

#[tokio::test]
async fn test_context_inclusion_integration() -> anyhow::Result<()> {
    let (server_transport, client_transport) = tokio::io::duplex(4096);

    // Start server
    let server_handle = tokio::spawn(async move {
        let server = TestServer::new().serve(server_transport).await?;
        server.waiting().await?;
        anyhow::Ok(())
    });

    // Start client that honors context requests
    let handler = TestClientHandler::new(true, true);
    let client = handler.clone().serve(client_transport).await?;

    // Test ThisServer context inclusion
    let request = ServerRequest::CreateMessageRequest(CreateMessageRequest {
        method: Default::default(),
        params: CreateMessageRequestParam {
            messages: vec![SamplingMessage {
                role: Role::User,
                content: Content::text("test message"),
            }],
            include_context: Some(ContextInclusion::ThisServer),
            model_preferences: None,
            system_prompt: None,
            temperature: None,
            max_tokens: 100,
            stop_sequences: None,
            metadata: None,
        },
        extensions: Default::default(),
    });

    let result = handler
        .handle_request(
            request.clone(),
            RequestContext {
                peer: client.peer().clone(),
                ct: CancellationToken::new(),
                id: NumberOrString::Number(1),
                meta: Default::default(),
                extensions: Default::default(),
            },
        )
        .await?;

    if let ClientResult::CreateMessageResult(result) = result {
        let text = result.message.content.as_text().unwrap().text.as_str();
        assert!(
            text.contains("test context"),
            "Response should include context for ThisServer"
        );
    } else {
        panic!("Expected CreateMessageResult");
    }

    // Test AllServers context inclusion
    let request = ServerRequest::CreateMessageRequest(CreateMessageRequest {
        method: Default::default(),
        params: CreateMessageRequestParam {
            messages: vec![SamplingMessage {
                role: Role::User,
                content: Content::text("test message"),
            }],
            include_context: Some(ContextInclusion::AllServers),
            model_preferences: None,
            system_prompt: None,
            temperature: None,
            max_tokens: 100,
            stop_sequences: None,
            metadata: None,
        },
        extensions: Default::default(),
    });

    let result = handler
        .handle_request(
            request.clone(),
            RequestContext {
                peer: client.peer().clone(),
                ct: CancellationToken::new(),
                id: NumberOrString::Number(2),
                meta: Default::default(),
                extensions: Default::default(),
            },
        )
        .await?;

    if let ClientResult::CreateMessageResult(result) = result {
        let text = result.message.content.as_text().unwrap().text.as_str();
        assert!(
            text.contains("test context"),
            "Response should include context for AllServers"
        );
    } else {
        panic!("Expected CreateMessageResult");
    }

    // Test No context inclusion
    let request = ServerRequest::CreateMessageRequest(CreateMessageRequest {
        method: Default::default(),
        params: CreateMessageRequestParam {
            messages: vec![SamplingMessage {
                role: Role::User,
                content: Content::text("test message"),
            }],
            include_context: Some(ContextInclusion::None),
            model_preferences: None,
            system_prompt: None,
            temperature: None,
            max_tokens: 100,
            stop_sequences: None,
            metadata: None,
        },
        extensions: Default::default(),
    });

    let result = handler
        .handle_request(
            request.clone(),
            RequestContext {
                peer: client.peer().clone(),
                ct: CancellationToken::new(),
                id: NumberOrString::Number(3),
                meta: Default::default(),
                extensions: Default::default(),
            },
        )
        .await?;

    if let ClientResult::CreateMessageResult(result) = result {
        let text = result.message.content.as_text().unwrap().text.as_str();
        assert!(
            !text.contains("test context"),
            "Response should not include context for None"
        );
    } else {
        panic!("Expected CreateMessageResult");
    }

    client.cancel().await?;
    server_handle.await??;
    Ok(())
}

#[tokio::test]
async fn test_context_inclusion_ignored_integration() -> anyhow::Result<()> {
    let (server_transport, client_transport) = tokio::io::duplex(4096);

    // Start server
    let server_handle = tokio::spawn(async move {
        let server = TestServer::new().serve(server_transport).await?;
        server.waiting().await?;
        anyhow::Ok(())
    });

    // Start client that ignores context requests
    let handler = TestClientHandler::new(false, false);
    let client = handler.clone().serve(client_transport).await?;

    // Test that context requests are ignored
    let request = ServerRequest::CreateMessageRequest(CreateMessageRequest {
        method: Default::default(),
        params: CreateMessageRequestParam {
            messages: vec![SamplingMessage {
                role: Role::User,
                content: Content::text("test message"),
            }],
            include_context: Some(ContextInclusion::ThisServer),
            model_preferences: None,
            system_prompt: None,
            temperature: None,
            max_tokens: 100,
            stop_sequences: None,
            metadata: None,
        },
        extensions: Default::default(),
    });

    let result = handler
        .handle_request(
            request.clone(),
            RequestContext {
                peer: client.peer().clone(),
                ct: CancellationToken::new(),
                id: NumberOrString::Number(1),
                meta: Meta::default(),
                extensions: Default::default(),
            },
        )
        .await?;

    if let ClientResult::CreateMessageResult(result) = result {
        let text = result.message.content.as_text().unwrap().text.as_str();
        assert!(
            !text.contains("test context"),
            "Context should be ignored when client chooses not to honor requests"
        );
    } else {
        panic!("Expected CreateMessageResult");
    }

    client.cancel().await?;
    server_handle.await??;
    Ok(())
}

#[tokio::test]
async fn test_message_sequence_integration() -> anyhow::Result<()> {
    let (server_transport, client_transport) = tokio::io::duplex(4096);

    // Start server
    let server_handle = tokio::spawn(async move {
        let server = TestServer::new().serve(server_transport).await?;
        server.waiting().await?;
        anyhow::Ok(())
    });

    // Start client
    let handler = TestClientHandler::new(true, true);
    let client = handler.clone().serve(client_transport).await?;

    let request = ServerRequest::CreateMessageRequest(CreateMessageRequest {
        method: Default::default(),
        params: CreateMessageRequestParam {
            messages: vec![
                SamplingMessage {
                    role: Role::User,
                    content: Content::text("first message"),
                },
                SamplingMessage {
                    role: Role::Assistant,
                    content: Content::text("second message"),
                },
            ],
            include_context: Some(ContextInclusion::ThisServer),
            model_preferences: None,
            system_prompt: None,
            temperature: None,
            max_tokens: 100,
            stop_sequences: None,
            metadata: None,
        },
        extensions: Default::default(),
    });

    let result = handler
        .handle_request(
            request.clone(),
            RequestContext {
                peer: client.peer().clone(),
                ct: CancellationToken::new(),
                id: NumberOrString::Number(1),
                meta: Meta::default(),
                extensions: Default::default(),
            },
        )
        .await?;

    if let ClientResult::CreateMessageResult(result) = result {
        let text = result.message.content.as_text().unwrap().text.as_str();
        assert!(
            text.contains("test context"),
            "Response should include context when ThisServer is specified"
        );
        assert_eq!(result.model, "test-model");
        assert_eq!(
            result.stop_reason,
            Some(CreateMessageResult::STOP_REASON_END_TURN.to_string())
        );
    } else {
        panic!("Expected CreateMessageResult");
    }

    client.cancel().await?;
    server_handle.await??;
    Ok(())
}

#[tokio::test]
async fn test_message_sequence_validation_integration() -> anyhow::Result<()> {
    let (server_transport, client_transport) = tokio::io::duplex(4096);

    let server_handle = tokio::spawn(async move {
        let server = TestServer::new().serve(server_transport).await?;
        server.waiting().await?;
        anyhow::Ok(())
    });

    let handler = TestClientHandler::new(true, true);
    let client = handler.clone().serve(client_transport).await?;

    // Test valid sequence: User -> Assistant -> User
    let request = ServerRequest::CreateMessageRequest(CreateMessageRequest {
        method: Default::default(),
        params: CreateMessageRequestParam {
            messages: vec![
                SamplingMessage {
                    role: Role::User,
                    content: Content::text("first user message"),
                },
                SamplingMessage {
                    role: Role::Assistant,
                    content: Content::text("first assistant response"),
                },
                SamplingMessage {
                    role: Role::User,
                    content: Content::text("second user message"),
                },
            ],
            include_context: None,
            model_preferences: None,
            system_prompt: None,
            temperature: None,
            max_tokens: 100,
            stop_sequences: None,
            metadata: None,
        },
        extensions: Default::default(),
    });

    let result = handler
        .handle_request(
            request.clone(),
            RequestContext {
                peer: client.peer().clone(),
                ct: CancellationToken::new(),
                id: NumberOrString::Number(1),
                meta: Meta::default(),
                extensions: Default::default(),
            },
        )
        .await?;

    assert!(matches!(result, ClientResult::CreateMessageResult(_)));

    // Test invalid: No user message
    let request = ServerRequest::CreateMessageRequest(CreateMessageRequest {
        method: Default::default(),
        params: CreateMessageRequestParam {
            messages: vec![SamplingMessage {
                role: Role::Assistant,
                content: Content::text("assistant message"),
            }],
            include_context: None,
            model_preferences: None,
            system_prompt: None,
            temperature: None,
            max_tokens: 100,
            stop_sequences: None,
            metadata: None,
        },
        extensions: Default::default(),
    });

    let result = handler
        .handle_request(
            request.clone(),
            RequestContext {
                peer: client.peer().clone(),
                ct: CancellationToken::new(),
                id: NumberOrString::Number(2),
                meta: Meta::default(),
                extensions: Default::default(),
            },
        )
        .await;

    assert!(result.is_err());

    client.cancel().await?;
    server_handle.await??;
    Ok(())
}

#[tokio::test]
async fn test_selective_context_handling_integration() -> anyhow::Result<()> {
    let (server_transport, client_transport) = tokio::io::duplex(4096);

    let server_handle = tokio::spawn(async move {
        let server = TestServer::new().serve(server_transport).await?;
        server.waiting().await?;
        anyhow::Ok(())
    });

    // Client that only honors ThisServer but ignores AllServers
    let handler = TestClientHandler::new(true, false);
    let client = handler.clone().serve(client_transport).await?;

    // Test ThisServer is honored
    let request = ServerRequest::CreateMessageRequest(CreateMessageRequest {
        method: Default::default(),
        params: CreateMessageRequestParam {
            messages: vec![SamplingMessage {
                role: Role::User,
                content: Content::text("test message"),
            }],
            include_context: Some(ContextInclusion::ThisServer),
            model_preferences: None,
            system_prompt: None,
            temperature: None,
            max_tokens: 100,
            stop_sequences: None,
            metadata: None,
        },
        extensions: Default::default(),
    });

    let result = handler
        .handle_request(
            request.clone(),
            RequestContext {
                peer: client.peer().clone(),
                ct: CancellationToken::new(),
                id: NumberOrString::Number(1),
                meta: Meta::default(),
                extensions: Default::default(),
            },
        )
        .await?;

    if let ClientResult::CreateMessageResult(result) = result {
        let text = result.message.content.as_text().unwrap().text.as_str();
        assert!(
            text.contains("test context"),
            "ThisServer context request should be honored"
        );
    }

    // Test AllServers is ignored
    let request = ServerRequest::CreateMessageRequest(CreateMessageRequest {
        method: Default::default(),
        params: CreateMessageRequestParam {
            messages: vec![SamplingMessage {
                role: Role::User,
                content: Content::text("test message"),
            }],
            include_context: Some(ContextInclusion::AllServers),
            model_preferences: None,
            system_prompt: None,
            temperature: None,
            max_tokens: 100,
            stop_sequences: None,
            metadata: None,
        },
        extensions: Default::default(),
    });

    let result = handler
        .handle_request(
            request.clone(),
            RequestContext {
                peer: client.peer().clone(),
                ct: CancellationToken::new(),
                id: NumberOrString::Number(2),
                meta: Meta::default(),
                extensions: Default::default(),
            },
        )
        .await?;

    if let ClientResult::CreateMessageResult(result) = result {
        let text = result.message.content.as_text().unwrap().text.as_str();
        assert!(
            !text.contains("test context"),
            "AllServers context request should be ignored"
        );
    }

    client.cancel().await?;
    server_handle.await??;
    Ok(())
}

#[tokio::test]
async fn test_context_inclusion() -> anyhow::Result<()> {
    let (server_transport, client_transport) = tokio::io::duplex(4096);
    let server_handle = tokio::spawn(async move {
        let server = TestServer::new().serve(server_transport).await?;
        server.waiting().await?;
        anyhow::Ok(())
    });

    let handler = TestClientHandler::new(true, true);
    let client = handler.clone().serve(client_transport).await?;

    // Test context handling
    let request = ServerRequest::CreateMessageRequest(CreateMessageRequest {
        method: Default::default(),
        params: CreateMessageRequestParam {
            messages: vec![SamplingMessage {
                role: Role::User,
                content: Content::text("test"),
            }],
            include_context: Some(ContextInclusion::ThisServer),
            model_preferences: None,
            system_prompt: None,
            temperature: None,
            max_tokens: 100,
            stop_sequences: None,
            metadata: None,
        },
        extensions: Default::default(),
    });

    let result = handler
        .handle_request(
            request.clone(),
            RequestContext {
                peer: client.peer().clone(),
                ct: CancellationToken::new(),
                id: NumberOrString::Number(1),
                meta: Meta::default(),
                extensions: Default::default(),
            },
        )
        .await?;

    if let ClientResult::CreateMessageResult(result) = result {
        let text = result.message.content.as_text().unwrap().text.as_str();
        assert!(text.contains("test context"));
    }

    client.cancel().await?;
    server_handle.await??;
    Ok(())
}

---
File: modelcontextprotocol-rust-sdk-13e72ec/crates/rmcp/tests/test_message_schema.rs
---

mod tests {
    use rmcp::model::{ClientJsonRpcMessage, ServerJsonRpcMessage};
    use schemars::schema_for;

    #[test]
    fn test_client_json_rpc_message_schema() {
        let schema = schema_for!(ClientJsonRpcMessage);
        let schema_str = serde_json::to_string_pretty(&schema).unwrap();
        let expected = std::fs::read_to_string(
            "tests/test_message_schema/client_json_rpc_message_schema.json",
        )
        .unwrap();

        // Parse both strings to JSON values for more robust comparison
        let schema_json: serde_json::Value = serde_json::from_str(&schema_str).unwrap();
        let expected_json: serde_json::Value = serde_json::from_str(&expected).unwrap();
        assert_eq!(
            schema_json, expected_json,
            "Schema generation for ClientJsonRpcMessage should match expected output"
        );
    }

    #[test]
    fn test_server_json_rpc_message_schema() {
        let schema = schema_for!(ServerJsonRpcMessage);
        let schema_str = serde_json::to_string_pretty(&schema).unwrap();
        let expected = std::fs::read_to_string(
            "tests/test_message_schema/server_json_rpc_message_schema.json",
        )
        .unwrap();

        // Parse both strings to JSON values for more robust comparison
        let schema_json: serde_json::Value = serde_json::from_str(&schema_str).unwrap();
        let expected_json: serde_json::Value = serde_json::from_str(&expected).unwrap();
        assert_eq!(
            schema_json, expected_json,
            "Schema generation for ServerJsonRpcMessage should match expected output"
        );
    }
}

---
File: modelcontextprotocol-rust-sdk-13e72ec/crates/rmcp/tests/test_message_schema/client_json_rpc_message_schema.json
---

{
  "$schema": "http://json-schema.org/draft-07/schema#",
  "title": "JsonRpcMessage_for_ClientRequest_and_ClientResult_and_ClientNotification",
  "anyOf": [
    {
      "$ref": "#/definitions/JsonRpcRequest_for_ClientRequest"
    },
    {
      "$ref": "#/definitions/JsonRpcResponse_for_ClientResult"
    },
    {
      "$ref": "#/definitions/JsonRpcNotification_for_ClientNotification"
    },
    {
      "type": "array",
      "items": {
        "$ref": "#/definitions/JsonRpcBatchRequestItem_for_ClientRequest_and_ClientNotification"
      }
    },
    {
      "type": "array",
      "items": {
        "$ref": "#/definitions/JsonRpcBatchResponseItem_for_ClientResult"
      }
    },
    {
      "$ref": "#/definitions/JsonRpcError"
    }
  ],
  "definitions": {
    "Annotated_for_RawContent": {
      "type": "object",
      "oneOf": [
        {
          "type": "object",
          "required": [
            "text",
            "type"
          ],
          "properties": {
            "text": {
              "type": "string"
            },
            "type": {
              "type": "string",
              "enum": [
                "text"
              ]
            }
          }
        },
        {
          "type": "object",
          "required": [
            "data",
            "mimeType",
            "type"
          ],
          "properties": {
            "data": {
              "description": "The base64-encoded image",
              "type": "string"
            },
            "mimeType": {
              "type": "string"
            },
            "type": {
              "type": "string",
              "enum": [
                "image"
              ]
            }
          }
        },
        {
          "type": "object",
          "required": [
            "resource",
            "type"
          ],
          "properties": {
            "resource": {
              "$ref": "#/definitions/ResourceContents"
            },
            "type": {
              "type": "string",
              "enum": [
                "resource"
              ]
            }
          }
        },
        {
          "type": "object",
          "required": [
            "data",
            "mimeType",
            "type"
          ],
          "properties": {
            "annotations": {
              "anyOf": [
                {
                  "$ref": "#/definitions/Annotations"
                },
                {
                  "type": "null"
                }
              ]
            },
            "data": {
              "type": "string"
            },
            "mimeType": {
              "type": "string"
            },
            "type": {
              "type": "string",
              "enum": [
                "audio"
              ]
            }
          }
        }
      ],
      "properties": {
        "annotations": {
          "anyOf": [
            {
              "$ref": "#/definitions/Annotations"
            },
            {
              "type": "null"
            }
          ]
        }
      }
    },
    "Annotations": {
      "type": "object",
      "properties": {
        "audience": {
          "type": [
            "array",
            "null"
          ],
          "items": {
            "$ref": "#/definitions/Role"
          }
        },
        "priority": {
          "type": [
            "number",
            "null"
          ],
          "format": "float"
        },
        "timestamp": {
          "type": [
            "string",
            "null"
          ],
          "format": "date-time"
        }
      }
    },
    "ArgumentInfo": {
      "type": "object",
      "required": [
        "name",
        "value"
      ],
      "properties": {
        "name": {
          "type": "string"
        },
        "value": {
          "type": "string"
        }
      }
    },
    "CallToolRequestMethod": {
      "type": "string",
      "format": "const",
      "const": "tools/call"
    },
    "CallToolRequestParam": {
      "type": "object",
      "required": [
        "name"
      ],
      "properties": {
        "arguments": {
          "type": [
            "object",
            "null"
          ],
          "additionalProperties": true
        },
        "name": {
          "type": "string"
        }
      }
    },
    "CancelledNotificationMethod": {
      "type": "string",
      "format": "const",
      "const": "notifications/cancelled"
    },
    "CancelledNotificationParam": {
      "type": "object",
      "required": [
        "requestId"
      ],
      "properties": {
        "reason": {
          "type": [
            "string",
            "null"
          ]
        },
        "requestId": {
          "$ref": "#/definitions/NumberOrString"
        }
      }
    },
    "ClientCapabilities": {
      "title": "Builder",
      "description": "```rust # use rmcp::model::ClientCapabilities; let cap = ClientCapabilities::builder() .enable_experimental() .enable_roots() .enable_roots_list_changed() .build(); ```",
      "type": "object",
      "properties": {
        "experimental": {
          "type": [
            "object",
            "null"
          ],
          "additionalProperties": {
            "type": "object",
            "additionalProperties": true
          }
        },
        "roots": {
          "anyOf": [
            {
              "$ref": "#/definitions/RootsCapabilities"
            },
            {
              "type": "null"
            }
          ]
        },
        "sampling": {
          "type": [
            "object",
            "null"
          ],
          "additionalProperties": true
        }
      }
    },
    "ClientResult": {
      "anyOf": [
        {
          "$ref": "#/definitions/CreateMessageResult"
        },
        {
          "$ref": "#/definitions/ListRootsResult"
        },
        {
          "$ref": "#/definitions/EmptyObject"
        }
      ]
    },
    "CompleteRequestMethod": {
      "type": "string",
      "format": "const",
      "const": "completion/complete"
    },
    "CompleteRequestParam": {
      "type": "object",
      "required": [
        "argument",
        "ref"
      ],
      "properties": {
        "argument": {
          "$ref": "#/definitions/ArgumentInfo"
        },
        "ref": {
          "$ref": "#/definitions/Reference"
        }
      }
    },
    "CreateMessageResult": {
      "type": "object",
      "required": [
        "content",
        "model",
        "role"
      ],
      "properties": {
        "content": {
          "$ref": "#/definitions/Annotated_for_RawContent"
        },
        "model": {
          "type": "string"
        },
        "role": {
          "$ref": "#/definitions/Role"
        },
        "stopReason": {
          "type": [
            "string",
            "null"
          ]
        }
      }
    },
    "EmptyObject": {
      "type": "object"
    },
    "ErrorData": {
      "description": "Error information for JSON-RPC error responses.",
      "type": "object",
      "required": [
        "code",
        "message"
      ],
      "properties": {
        "code": {
          "description": "The error type that occurred.",
          "type": "integer",
          "format": "int32"
        },
        "data": {
          "description": "Additional information about the error. The value of this member is defined by the sender (e.g. detailed error information, nested errors etc.)."
        },
        "message": {
          "description": "A short description of the error. The message SHOULD be limited to a concise single sentence.",
          "type": "string"
        }
      }
    },
    "GetPromptRequestMethod": {
      "type": "string",
      "format": "const",
      "const": "prompts/get"
    },
    "GetPromptRequestParam": {
      "type": "object",
      "required": [
        "name"
      ],
      "properties": {
        "arguments": {
          "type": [
            "object",
            "null"
          ],
          "additionalProperties": true
        },
        "name": {
          "type": "string"
        }
      }
    },
    "Implementation": {
      "type": "object",
      "required": [
        "name",
        "version"
      ],
      "properties": {
        "name": {
          "type": "string"
        },
        "version": {
          "type": "string"
        }
      }
    },
    "InitializeRequestParam": {
      "type": "object",
      "required": [
        "capabilities",
        "clientInfo",
        "protocolVersion"
      ],
      "properties": {
        "capabilities": {
          "$ref": "#/definitions/ClientCapabilities"
        },
        "clientInfo": {
          "$ref": "#/definitions/Implementation"
        },
        "protocolVersion": {
          "$ref": "#/definitions/ProtocolVersion"
        }
      }
    },
    "InitializeResultMethod": {
      "type": "string",
      "format": "const",
      "const": "initialize"
    },
    "InitializedNotificationMethod": {
      "type": "string",
      "format": "const",
      "const": "notifications/initialized"
    },
    "JsonRpcBatchRequestItem_for_ClientRequest_and_ClientNotification": {
      "anyOf": [
        {
          "$ref": "#/definitions/JsonRpcRequest_for_ClientRequest"
        },
        {
          "$ref": "#/definitions/JsonRpcNotification_for_ClientNotification"
        }
      ]
    },
    "JsonRpcBatchResponseItem_for_ClientResult": {
      "anyOf": [
        {
          "$ref": "#/definitions/JsonRpcResponse_for_ClientResult"
        },
        {
          "$ref": "#/definitions/JsonRpcError"
        }
      ]
    },
    "JsonRpcError": {
      "type": "object",
      "required": [
        "error",
        "id",
        "jsonrpc"
      ],
      "properties": {
        "error": {
          "$ref": "#/definitions/ErrorData"
        },
        "id": {
          "$ref": "#/definitions/NumberOrString"
        },
        "jsonrpc": {
          "$ref": "#/definitions/JsonRpcVersion2_0"
        }
      }
    },
    "JsonRpcNotification_for_ClientNotification": {
      "type": "object",
      "anyOf": [
        {
          "$ref": "#/definitions/Notification_for_CancelledNotificationMethod_and_CancelledNotificationParam"
        },
        {
          "$ref": "#/definitions/Notification_for_ProgressNotificationMethod_and_ProgressNotificationParam"
        },
        {
          "$ref": "#/definitions/NotificationNoParam_for_InitializedNotificationMethod"
        },
        {
          "$ref": "#/definitions/NotificationNoParam_for_RootsListChangedNotificationMethod"
        }
      ],
      "required": [
        "jsonrpc"
      ],
      "properties": {
        "jsonrpc": {
          "$ref": "#/definitions/JsonRpcVersion2_0"
        }
      }
    },
    "JsonRpcRequest_for_ClientRequest": {
      "type": "object",
      "anyOf": [
        {
          "$ref": "#/definitions/RequestNoParam_for_PingRequestMethod"
        },
        {
          "$ref": "#/definitions/Request_for_InitializeResultMethod_and_InitializeRequestParam"
        },
        {
          "$ref": "#/definitions/Request_for_CompleteRequestMethod_and_CompleteRequestParam"
        },
        {
          "$ref": "#/definitions/Request_for_SetLevelRequestMethod_and_SetLevelRequestParam"
        },
        {
          "$ref": "#/definitions/Request_for_GetPromptRequestMethod_and_GetPromptRequestParam"
        },
        {
          "$ref": "#/definitions/RequestOptionalParam_for_ListPromptsRequestMethod_and_PaginatedRequestParam"
        },
        {
          "$ref": "#/definitions/RequestOptionalParam_for_ListResourcesRequestMethod_and_PaginatedRequestParam"
        },
        {
          "$ref": "#/definitions/RequestOptionalParam_for_ListResourceTemplatesRequestMethod_and_PaginatedRequestParam"
        },
        {
          "$ref": "#/definitions/Request_for_ReadResourceRequestMethod_and_ReadResourceRequestParam"
        },
        {
          "$ref": "#/definitions/Request_for_SubscribeRequestMethod_and_SubscribeRequestParam"
        },
        {
          "$ref": "#/definitions/Request_for_UnsubscribeRequestMethod_and_UnsubscribeRequestParam"
        },
        {
          "$ref": "#/definitions/Request_for_CallToolRequestMethod_and_CallToolRequestParam"
        },
        {
          "$ref": "#/definitions/RequestOptionalParam_for_ListToolsRequestMethod_and_PaginatedRequestParam"
        }
      ],
      "required": [
        "id",
        "jsonrpc"
      ],
      "properties": {
        "id": {
          "$ref": "#/definitions/NumberOrString"
        },
        "jsonrpc": {
          "$ref": "#/definitions/JsonRpcVersion2_0"
        }
      }
    },
    "JsonRpcResponse_for_ClientResult": {
      "type": "object",
      "required": [
        "id",
        "jsonrpc",
        "result"
      ],
      "properties": {
        "id": {
          "$ref": "#/definitions/NumberOrString"
        },
        "jsonrpc": {
          "$ref": "#/definitions/JsonRpcVersion2_0"
        },
        "result": {
          "$ref": "#/definitions/ClientResult"
        }
      }
    },
    "JsonRpcVersion2_0": {
      "type": "string",
      "format": "const",
      "const": "2.0"
    },
    "ListPromptsRequestMethod": {
      "type": "string",
      "format": "const",
      "const": "prompts/list"
    },
    "ListResourceTemplatesRequestMethod": {
      "type": "string",
      "format": "const",
      "const": "resources/templates/list"
    },
    "ListResourcesRequestMethod": {
      "type": "string",
      "format": "const",
      "const": "resources/list"
    },
    "ListRootsResult": {
      "type": "object",
      "required": [
        "roots"
      ],
      "properties": {
        "roots": {
          "type": "array",
          "items": {
            "$ref": "#/definitions/Root"
          }
        }
      }
    },
    "ListToolsRequestMethod": {
      "type": "string",
      "format": "const",
      "const": "tools/list"
    },
    "LoggingLevel": {
      "type": "string",
      "enum": [
        "debug",
        "info",
        "notice",
        "warning",
        "error",
        "critical",
        "alert",
        "emergency"
      ]
    },
    "NotificationNoParam_for_InitializedNotificationMethod": {
      "type": "object",
      "required": [
        "method"
      ],
      "properties": {
        "method": {
          "$ref": "#/definitions/InitializedNotificationMethod"
        }
      }
    },
    "NotificationNoParam_for_RootsListChangedNotificationMethod": {
      "type": "object",
      "required": [
        "method"
      ],
      "properties": {
        "method": {
          "$ref": "#/definitions/RootsListChangedNotificationMethod"
        }
      }
    },
    "Notification_for_CancelledNotificationMethod_and_CancelledNotificationParam": {
      "type": "object",
      "required": [
        "method",
        "params"
      ],
      "properties": {
        "method": {
          "$ref": "#/definitions/CancelledNotificationMethod"
        },
        "params": {
          "$ref": "#/definitions/CancelledNotificationParam"
        }
      }
    },
    "Notification_for_ProgressNotificationMethod_and_ProgressNotificationParam": {
      "type": "object",
      "required": [
        "method",
        "params"
      ],
      "properties": {
        "method": {
          "$ref": "#/definitions/ProgressNotificationMethod"
        },
        "params": {
          "$ref": "#/definitions/ProgressNotificationParam"
        }
      }
    },
    "NumberOrString": {
      "oneOf": [
        {
          "type": "number"
        },
        {
          "type": "string"
        }
      ]
    },
    "PaginatedRequestParam": {
      "type": "object",
      "properties": {
        "cursor": {
          "type": [
            "string",
            "null"
          ]
        }
      }
    },
    "PingRequestMethod": {
      "type": "string",
      "format": "const",
      "const": "ping"
    },
    "ProgressNotificationMethod": {
      "type": "string",
      "format": "const",
      "const": "notifications/progress"
    },
    "ProgressNotificationParam": {
      "type": "object",
      "required": [
        "progress",
        "progressToken"
      ],
      "properties": {
        "message": {
          "description": "An optional message describing the current progress.",
          "type": [
            "string",
            "null"
          ]
        },
        "progress": {
          "description": "The progress thus far. This should increase every time progress is made, even if the total is unknown.",
          "type": "integer",
          "format": "uint32",
          "minimum": 0.0
        },
        "progressToken": {
          "$ref": "#/definitions/NumberOrString"
        },
        "total": {
          "description": "Total number of items to process (or total progress required), if known",
          "type": [
            "integer",
            "null"
          ],
          "format": "uint32",
          "minimum": 0.0
        }
      }
    },
    "ProtocolVersion": {
      "type": "string"
    },
    "ReadResourceRequestMethod": {
      "type": "string",
      "format": "const",
      "const": "resources/read"
    },
    "ReadResourceRequestParam": {
      "type": "object",
      "required": [
        "uri"
      ],
      "properties": {
        "uri": {
          "type": "string"
        }
      }
    },
    "Reference": {
      "oneOf": [
        {
          "type": "object",
          "required": [
            "type",
            "uri"
          ],
          "properties": {
            "type": {
              "type": "string",
              "enum": [
                "ref/resource"
              ]
            },
            "uri": {
              "type": "string"
            }
          }
        },
        {
          "type": "object",
          "required": [
            "name",
            "type"
          ],
          "properties": {
            "name": {
              "type": "string"
            },
            "type": {
              "type": "string",
              "enum": [
                "ref/prompt"
              ]
            }
          }
        }
      ]
    },
    "RequestNoParam_for_PingRequestMethod": {
      "type": "object",
      "required": [
        "method"
      ],
      "properties": {
        "method": {
          "$ref": "#/definitions/PingRequestMethod"
        }
      }
    },
    "RequestOptionalParam_for_ListPromptsRequestMethod_and_PaginatedRequestParam": {
      "type": "object",
      "required": [
        "method"
      ],
      "properties": {
        "method": {
          "$ref": "#/definitions/ListPromptsRequestMethod"
        },
        "params": {
          "anyOf": [
            {
              "$ref": "#/definitions/PaginatedRequestParam"
            },
            {
              "type": "null"
            }
          ]
        }
      }
    },
    "RequestOptionalParam_for_ListResourceTemplatesRequestMethod_and_PaginatedRequestParam": {
      "type": "object",
      "required": [
        "method"
      ],
      "properties": {
        "method": {
          "$ref": "#/definitions/ListResourceTemplatesRequestMethod"
        },
        "params": {
          "anyOf": [
            {
              "$ref": "#/definitions/PaginatedRequestParam"
            },
            {
              "type": "null"
            }
          ]
        }
      }
    },
    "RequestOptionalParam_for_ListResourcesRequestMethod_and_PaginatedRequestParam": {
      "type": "object",
      "required": [
        "method"
      ],
      "properties": {
        "method": {
          "$ref": "#/definitions/ListResourcesRequestMethod"
        },
        "params": {
          "anyOf": [
            {
              "$ref": "#/definitions/PaginatedRequestParam"
            },
            {
              "type": "null"
            }
          ]
        }
      }
    },
    "RequestOptionalParam_for_ListToolsRequestMethod_and_PaginatedRequestParam": {
      "type": "object",
      "required": [
        "method"
      ],
      "properties": {
        "method": {
          "$ref": "#/definitions/ListToolsRequestMethod"
        },
        "params": {
          "anyOf": [
            {
              "$ref": "#/definitions/PaginatedRequestParam"
            },
            {
              "type": "null"
            }
          ]
        }
      }
    },
    "Request_for_CallToolRequestMethod_and_CallToolRequestParam": {
      "type": "object",
      "required": [
        "method",
        "params"
      ],
      "properties": {
        "method": {
          "$ref": "#/definitions/CallToolRequestMethod"
        },
        "params": {
          "$ref": "#/definitions/CallToolRequestParam"
        }
      }
    },
    "Request_for_CompleteRequestMethod_and_CompleteRequestParam": {
      "type": "object",
      "required": [
        "method",
        "params"
      ],
      "properties": {
        "method": {
          "$ref": "#/definitions/CompleteRequestMethod"
        },
        "params": {
          "$ref": "#/definitions/CompleteRequestParam"
        }
      }
    },
    "Request_for_GetPromptRequestMethod_and_GetPromptRequestParam": {
      "type": "object",
      "required": [
        "method",
        "params"
      ],
      "properties": {
        "method": {
          "$ref": "#/definitions/GetPromptRequestMethod"
        },
        "params": {
          "$ref": "#/definitions/GetPromptRequestParam"
        }
      }
    },
    "Request_for_InitializeResultMethod_and_InitializeRequestParam": {
      "type": "object",
      "required": [
        "method",
        "params"
      ],
      "properties": {
        "method": {
          "$ref": "#/definitions/InitializeResultMethod"
        },
        "params": {
          "$ref": "#/definitions/InitializeRequestParam"
        }
      }
    },
    "Request_for_ReadResourceRequestMethod_and_ReadResourceRequestParam": {
      "type": "object",
      "required": [
        "method",
        "params"
      ],
      "properties": {
        "method": {
          "$ref": "#/definitions/ReadResourceRequestMethod"
        },
        "params": {
          "$ref": "#/definitions/ReadResourceRequestParam"
        }
      }
    },
    "Request_for_SetLevelRequestMethod_and_SetLevelRequestParam": {
      "type": "object",
      "required": [
        "method",
        "params"
      ],
      "properties": {
        "method": {
          "$ref": "#/definitions/SetLevelRequestMethod"
        },
        "params": {
          "$ref": "#/definitions/SetLevelRequestParam"
        }
      }
    },
    "Request_for_SubscribeRequestMethod_and_SubscribeRequestParam": {
      "type": "object",
      "required": [
        "method",
        "params"
      ],
      "properties": {
        "method": {
          "$ref": "#/definitions/SubscribeRequestMethod"
        },
        "params": {
          "$ref": "#/definitions/SubscribeRequestParam"
        }
      }
    },
    "Request_for_UnsubscribeRequestMethod_and_UnsubscribeRequestParam": {
      "type": "object",
      "required": [
        "method",
        "params"
      ],
      "properties": {
        "method": {
          "$ref": "#/definitions/UnsubscribeRequestMethod"
        },
        "params": {
          "$ref": "#/definitions/UnsubscribeRequestParam"
        }
      }
    },
    "ResourceContents": {
      "anyOf": [
        {
          "type": "object",
          "required": [
            "text",
            "uri"
          ],
          "properties": {
            "mime_type": {
              "type": [
                "string",
                "null"
              ]
            },
            "text": {
              "type": "string"
            },
            "uri": {
              "type": "string"
            }
          }
        },
        {
          "type": "object",
          "required": [
            "blob",
            "uri"
          ],
          "properties": {
            "blob": {
              "type": "string"
            },
            "mime_type": {
              "type": [
                "string",
                "null"
              ]
            },
            "uri": {
              "type": "string"
            }
          }
        }
      ]
    },
    "Role": {
      "type": "string",
      "enum": [
        "user",
        "assistant"
      ]
    },
    "Root": {
      "type": "object",
      "required": [
        "uri"
      ],
      "properties": {
        "name": {
          "type": [
            "string",
            "null"
          ]
        },
        "uri": {
          "type": "string"
        }
      }
    },
    "RootsCapabilities": {
      "type": "object",
      "properties": {
        "listChanged": {
          "type": [
            "boolean",
            "null"
          ]
        }
      }
    },
    "RootsListChangedNotificationMethod": {
      "type": "string",
      "format": "const",
      "const": "notifications/roots/list_changed"
    },
    "SetLevelRequestMethod": {
      "type": "string",
      "format": "const",
      "const": "logging/setLevel"
    },
    "SetLevelRequestParam": {
      "type": "object",
      "required": [
        "level"
      ],
      "properties": {
        "level": {
          "$ref": "#/definitions/LoggingLevel"
        }
      }
    },
    "SubscribeRequestMethod": {
      "type": "string",
      "format": "const",
      "const": "resources/subscribe"
    },
    "SubscribeRequestParam": {
      "type": "object",
      "required": [
        "uri"
      ],
      "properties": {
        "uri": {
          "type": "string"
        }
      }
    },
    "UnsubscribeRequestMethod": {
      "type": "string",
      "format": "const",
      "const": "resources/unsubscribe"
    },
    "UnsubscribeRequestParam": {
      "type": "object",
      "required": [
        "uri"
      ],
      "properties": {
        "uri": {
          "type": "string"
        }
      }
    }
  }
}

---
File: modelcontextprotocol-rust-sdk-13e72ec/crates/rmcp/tests/test_message_schema/server_json_rpc_message_schema.json
---

{
  "$schema": "http://json-schema.org/draft-07/schema#",
  "title": "JsonRpcMessage_for_ServerRequest_and_ServerResult_and_ServerNotification",
  "anyOf": [
    {
      "$ref": "#/definitions/JsonRpcRequest_for_ServerRequest"
    },
    {
      "$ref": "#/definitions/JsonRpcResponse_for_ServerResult"
    },
    {
      "$ref": "#/definitions/JsonRpcNotification_for_ServerNotification"
    },
    {
      "type": "array",
      "items": {
        "$ref": "#/definitions/JsonRpcBatchRequestItem_for_ServerRequest_and_ServerNotification"
      }
    },
    {
      "type": "array",
      "items": {
        "$ref": "#/definitions/JsonRpcBatchResponseItem_for_ServerResult"
      }
    },
    {
      "$ref": "#/definitions/JsonRpcError"
    }
  ],
  "definitions": {
    "Annotated_for_RawContent": {
      "type": "object",
      "oneOf": [
        {
          "type": "object",
          "required": [
            "text",
            "type"
          ],
          "properties": {
            "text": {
              "type": "string"
            },
            "type": {
              "type": "string",
              "enum": [
                "text"
              ]
            }
          }
        },
        {
          "type": "object",
          "required": [
            "data",
            "mimeType",
            "type"
          ],
          "properties": {
            "data": {
              "description": "The base64-encoded image",
              "type": "string"
            },
            "mimeType": {
              "type": "string"
            },
            "type": {
              "type": "string",
              "enum": [
                "image"
              ]
            }
          }
        },
        {
          "type": "object",
          "required": [
            "resource",
            "type"
          ],
          "properties": {
            "resource": {
              "$ref": "#/definitions/ResourceContents"
            },
            "type": {
              "type": "string",
              "enum": [
                "resource"
              ]
            }
          }
        },
        {
          "type": "object",
          "required": [
            "data",
            "mimeType",
            "type"
          ],
          "properties": {
            "annotations": {
              "anyOf": [
                {
                  "$ref": "#/definitions/Annotations"
                },
                {
                  "type": "null"
                }
              ]
            },
            "data": {
              "type": "string"
            },
            "mimeType": {
              "type": "string"
            },
            "type": {
              "type": "string",
              "enum": [
                "audio"
              ]
            }
          }
        }
      ],
      "properties": {
        "annotations": {
          "anyOf": [
            {
              "$ref": "#/definitions/Annotations"
            },
            {
              "type": "null"
            }
          ]
        }
      }
    },
    "Annotated_for_RawEmbeddedResource": {
      "type": "object",
      "required": [
        "resource"
      ],
      "properties": {
        "annotations": {
          "anyOf": [
            {
              "$ref": "#/definitions/Annotations"
            },
            {
              "type": "null"
            }
          ]
        },
        "resource": {
          "$ref": "#/definitions/ResourceContents"
        }
      }
    },
    "Annotated_for_RawResource": {
      "description": "Represents a resource in the extension with metadata",
      "type": "object",
      "required": [
        "name",
        "uri"
      ],
      "properties": {
        "annotations": {
          "anyOf": [
            {
              "$ref": "#/definitions/Annotations"
            },
            {
              "type": "null"
            }
          ]
        },
        "description": {
          "description": "Optional description of the resource",
          "type": [
            "string",
            "null"
          ]
        },
        "mimeType": {
          "description": "MIME type of the resource content (\"text\" or \"blob\")",
          "type": [
            "string",
            "null"
          ]
        },
        "name": {
          "description": "Name of the resource",
          "type": "string"
        },
        "size": {
          "description": "The size of the raw resource content, in bytes (i.e., before base64 encoding or any tokenization), if known.\n\nThis can be used by Hosts to display file sizes and estimate context window us",
          "type": [
            "integer",
            "null"
          ],
          "format": "uint32",
          "minimum": 0.0
        },
        "uri": {
          "description": "URI representing the resource location (e.g., \"file:///path/to/file\" or \"str:///content\")",
          "type": "string"
        }
      }
    },
    "Annotated_for_RawResourceTemplate": {
      "type": "object",
      "required": [
        "name",
        "uriTemplate"
      ],
      "properties": {
        "annotations": {
          "anyOf": [
            {
              "$ref": "#/definitions/Annotations"
            },
            {
              "type": "null"
            }
          ]
        },
        "description": {
          "type": [
            "string",
            "null"
          ]
        },
        "mimeType": {
          "type": [
            "string",
            "null"
          ]
        },
        "name": {
          "type": "string"
        },
        "uriTemplate": {
          "type": "string"
        }
      }
    },
    "Annotations": {
      "type": "object",
      "properties": {
        "audience": {
          "type": [
            "array",
            "null"
          ],
          "items": {
            "$ref": "#/definitions/Role"
          }
        },
        "priority": {
          "type": [
            "number",
            "null"
          ],
          "format": "float"
        },
        "timestamp": {
          "type": [
            "string",
            "null"
          ],
          "format": "date-time"
        }
      }
    },
    "CallToolResult": {
      "type": "object",
      "required": [
        "content"
      ],
      "properties": {
        "content": {
          "type": "array",
          "items": {
            "$ref": "#/definitions/Annotated_for_RawContent"
          }
        },
        "isError": {
          "type": [
            "boolean",
            "null"
          ]
        }
      }
    },
    "CancelledNotificationMethod": {
      "type": "string",
      "format": "const",
      "const": "notifications/cancelled"
    },
    "CancelledNotificationParam": {
      "type": "object",
      "required": [
        "requestId"
      ],
      "properties": {
        "reason": {
          "type": [
            "string",
            "null"
          ]
        },
        "requestId": {
          "$ref": "#/definitions/NumberOrString"
        }
      }
    },
    "CompleteResult": {
      "type": "object",
      "required": [
        "completion"
      ],
      "properties": {
        "completion": {
          "$ref": "#/definitions/CompletionInfo"
        }
      }
    },
    "CompletionInfo": {
      "type": "object",
      "required": [
        "values"
      ],
      "properties": {
        "hasMore": {
          "type": [
            "boolean",
            "null"
          ]
        },
        "total": {
          "type": [
            "integer",
            "null"
          ],
          "format": "uint32",
          "minimum": 0.0
        },
        "values": {
          "type": "array",
          "items": {
            "type": "string"
          }
        }
      }
    },
    "ContextInclusion": {
      "type": "string",
      "enum": [
        "allServers",
        "none",
        "thisServer"
      ]
    },
    "CreateMessageRequestMethod": {
      "type": "string",
      "format": "const",
      "const": "sampling/createMessage"
    },
    "CreateMessageRequestParam": {
      "type": "object",
      "required": [
        "maxTokens",
        "messages"
      ],
      "properties": {
        "includeContext": {
          "anyOf": [
            {
              "$ref": "#/definitions/ContextInclusion"
            },
            {
              "type": "null"
            }
          ]
        },
        "maxTokens": {
          "type": "integer",
          "format": "uint32",
          "minimum": 0.0
        },
        "messages": {
          "type": "array",
          "items": {
            "$ref": "#/definitions/SamplingMessage"
          }
        },
        "metadata": true,
        "modelPreferences": {
          "anyOf": [
            {
              "$ref": "#/definitions/ModelPreferences"
            },
            {
              "type": "null"
            }
          ]
        },
        "stopSequences": {
          "type": [
            "array",
            "null"
          ],
          "items": {
            "type": "string"
          }
        },
        "systemPrompt": {
          "type": [
            "string",
            "null"
          ]
        },
        "temperature": {
          "type": [
            "number",
            "null"
          ],
          "format": "float"
        }
      }
    },
    "EmptyObject": {
      "type": "object"
    },
    "ErrorData": {
      "description": "Error information for JSON-RPC error responses.",
      "type": "object",
      "required": [
        "code",
        "message"
      ],
      "properties": {
        "code": {
          "description": "The error type that occurred.",
          "type": "integer",
          "format": "int32"
        },
        "data": {
          "description": "Additional information about the error. The value of this member is defined by the sender (e.g. detailed error information, nested errors etc.)."
        },
        "message": {
          "description": "A short description of the error. The message SHOULD be limited to a concise single sentence.",
          "type": "string"
        }
      }
    },
    "GetPromptResult": {
      "type": "object",
      "required": [
        "messages"
      ],
      "properties": {
        "description": {
          "type": [
            "string",
            "null"
          ]
        },
        "messages": {
          "type": "array",
          "items": {
            "$ref": "#/definitions/PromptMessage"
          }
        }
      }
    },
    "Implementation": {
      "type": "object",
      "required": [
        "name",
        "version"
      ],
      "properties": {
        "name": {
          "type": "string"
        },
        "version": {
          "type": "string"
        }
      }
    },
    "InitializeResult": {
      "type": "object",
      "required": [
        "capabilities",
        "protocolVersion",
        "serverInfo"
      ],
      "properties": {
        "capabilities": {
          "$ref": "#/definitions/ServerCapabilities"
        },
        "instructions": {
          "type": [
            "string",
            "null"
          ]
        },
        "protocolVersion": {
          "$ref": "#/definitions/ProtocolVersion"
        },
        "serverInfo": {
          "$ref": "#/definitions/Implementation"
        }
      }
    },
    "JsonRpcBatchRequestItem_for_ServerRequest_and_ServerNotification": {
      "anyOf": [
        {
          "$ref": "#/definitions/JsonRpcRequest_for_ServerRequest"
        },
        {
          "$ref": "#/definitions/JsonRpcNotification_for_ServerNotification"
        }
      ]
    },
    "JsonRpcBatchResponseItem_for_ServerResult": {
      "anyOf": [
        {
          "$ref": "#/definitions/JsonRpcResponse_for_ServerResult"
        },
        {
          "$ref": "#/definitions/JsonRpcError"
        }
      ]
    },
    "JsonRpcError": {
      "type": "object",
      "required": [
        "error",
        "id",
        "jsonrpc"
      ],
      "properties": {
        "error": {
          "$ref": "#/definitions/ErrorData"
        },
        "id": {
          "$ref": "#/definitions/NumberOrString"
        },
        "jsonrpc": {
          "$ref": "#/definitions/JsonRpcVersion2_0"
        }
      }
    },
    "JsonRpcNotification_for_ServerNotification": {
      "type": "object",
      "anyOf": [
        {
          "$ref": "#/definitions/Notification_for_CancelledNotificationMethod_and_CancelledNotificationParam"
        },
        {
          "$ref": "#/definitions/Notification_for_ProgressNotificationMethod_and_ProgressNotificationParam"
        },
        {
          "$ref": "#/definitions/Notification_for_LoggingMessageNotificationMethod_and_LoggingMessageNotificationParam"
        },
        {
          "$ref": "#/definitions/Notification_for_ResourceUpdatedNotificationMethod_and_ResourceUpdatedNotificationParam"
        },
        {
          "$ref": "#/definitions/NotificationNoParam_for_ResourceListChangedNotificationMethod"
        },
        {
          "$ref": "#/definitions/NotificationNoParam_for_ToolListChangedNotificationMethod"
        },
        {
          "$ref": "#/definitions/NotificationNoParam_for_PromptListChangedNotificationMethod"
        }
      ],
      "required": [
        "jsonrpc"
      ],
      "properties": {
        "jsonrpc": {
          "$ref": "#/definitions/JsonRpcVersion2_0"
        }
      }
    },
    "JsonRpcRequest_for_ServerRequest": {
      "type": "object",
      "anyOf": [
        {
          "$ref": "#/definitions/RequestNoParam_for_PingRequestMethod"
        },
        {
          "$ref": "#/definitions/Request_for_CreateMessageRequestMethod_and_CreateMessageRequestParam"
        },
        {
          "$ref": "#/definitions/RequestNoParam_for_ListRootsRequestMethod"
        }
      ],
      "required": [
        "id",
        "jsonrpc"
      ],
      "properties": {
        "id": {
          "$ref": "#/definitions/NumberOrString"
        },
        "jsonrpc": {
          "$ref": "#/definitions/JsonRpcVersion2_0"
        }
      }
    },
    "JsonRpcResponse_for_ServerResult": {
      "type": "object",
      "required": [
        "id",
        "jsonrpc",
        "result"
      ],
      "properties": {
        "id": {
          "$ref": "#/definitions/NumberOrString"
        },
        "jsonrpc": {
          "$ref": "#/definitions/JsonRpcVersion2_0"
        },
        "result": {
          "$ref": "#/definitions/ServerResult"
        }
      }
    },
    "JsonRpcVersion2_0": {
      "type": "string",
      "format": "const",
      "const": "2.0"
    },
    "ListPromptsResult": {
      "type": "object",
      "required": [
        "prompts"
      ],
      "properties": {
        "nextCursor": {
          "type": [
            "string",
            "null"
          ]
        },
        "prompts": {
          "type": "array",
          "items": {
            "$ref": "#/definitions/Prompt"
          }
        }
      }
    },
    "ListResourceTemplatesResult": {
      "type": "object",
      "required": [
        "resourceTemplates"
      ],
      "properties": {
        "nextCursor": {
          "type": [
            "string",
            "null"
          ]
        },
        "resourceTemplates": {
          "type": "array",
          "items": {
            "$ref": "#/definitions/Annotated_for_RawResourceTemplate"
          }
        }
      }
    },
    "ListResourcesResult": {
      "type": "object",
      "required": [
        "resources"
      ],
      "properties": {
        "nextCursor": {
          "type": [
            "string",
            "null"
          ]
        },
        "resources": {
          "type": "array",
          "items": {
            "$ref": "#/definitions/Annotated_for_RawResource"
          }
        }
      }
    },
    "ListRootsRequestMethod": {
      "type": "string",
      "format": "const",
      "const": "roots/list"
    },
    "ListToolsResult": {
      "type": "object",
      "required": [
        "tools"
      ],
      "properties": {
        "nextCursor": {
          "type": [
            "string",
            "null"
          ]
        },
        "tools": {
          "type": "array",
          "items": {
            "$ref": "#/definitions/Tool"
          }
        }
      }
    },
    "LoggingLevel": {
      "type": "string",
      "enum": [
        "debug",
        "info",
        "notice",
        "warning",
        "error",
        "critical",
        "alert",
        "emergency"
      ]
    },
    "LoggingMessageNotificationMethod": {
      "type": "string",
      "format": "const",
      "const": "notifications/message"
    },
    "LoggingMessageNotificationParam": {
      "type": "object",
      "required": [
        "data",
        "level"
      ],
      "properties": {
        "data": true,
        "level": {
          "$ref": "#/definitions/LoggingLevel"
        },
        "logger": {
          "type": [
            "string",
            "null"
          ]
        }
      }
    },
    "ModelHint": {
      "type": "object",
      "properties": {
        "name": {
          "type": [
            "string",
            "null"
          ]
        }
      }
    },
    "ModelPreferences": {
      "type": "object",
      "properties": {
        "costPriority": {
          "type": [
            "number",
            "null"
          ],
          "format": "float"
        },
        "hints": {
          "type": [
            "array",
            "null"
          ],
          "items": {
            "$ref": "#/definitions/ModelHint"
          }
        },
        "intelligencePriority": {
          "type": [
            "number",
            "null"
          ],
          "format": "float"
        },
        "speedPriority": {
          "type": [
            "number",
            "null"
          ],
          "format": "float"
        }
      }
    },
    "NotificationNoParam_for_PromptListChangedNotificationMethod": {
      "type": "object",
      "required": [
        "method"
      ],
      "properties": {
        "method": {
          "$ref": "#/definitions/PromptListChangedNotificationMethod"
        }
      }
    },
    "NotificationNoParam_for_ResourceListChangedNotificationMethod": {
      "type": "object",
      "required": [
        "method"
      ],
      "properties": {
        "method": {
          "$ref": "#/definitions/ResourceListChangedNotificationMethod"
        }
      }
    },
    "NotificationNoParam_for_ToolListChangedNotificationMethod": {
      "type": "object",
      "required": [
        "method"
      ],
      "properties": {
        "method": {
          "$ref": "#/definitions/ToolListChangedNotificationMethod"
        }
      }
    },
    "Notification_for_CancelledNotificationMethod_and_CancelledNotificationParam": {
      "type": "object",
      "required": [
        "method",
        "params"
      ],
      "properties": {
        "method": {
          "$ref": "#/definitions/CancelledNotificationMethod"
        },
        "params": {
          "$ref": "#/definitions/CancelledNotificationParam"
        }
      }
    },
    "Notification_for_LoggingMessageNotificationMethod_and_LoggingMessageNotificationParam": {
      "type": "object",
      "required": [
        "method",
        "params"
      ],
      "properties": {
        "method": {
          "$ref": "#/definitions/LoggingMessageNotificationMethod"
        },
        "params": {
          "$ref": "#/definitions/LoggingMessageNotificationParam"
        }
      }
    },
    "Notification_for_ProgressNotificationMethod_and_ProgressNotificationParam": {
      "type": "object",
      "required": [
        "method",
        "params"
      ],
      "properties": {
        "method": {
          "$ref": "#/definitions/ProgressNotificationMethod"
        },
        "params": {
          "$ref": "#/definitions/ProgressNotificationParam"
        }
      }
    },
    "Notification_for_ResourceUpdatedNotificationMethod_and_ResourceUpdatedNotificationParam": {
      "type": "object",
      "required": [
        "method",
        "params"
      ],
      "properties": {
        "method": {
          "$ref": "#/definitions/ResourceUpdatedNotificationMethod"
        },
        "params": {
          "$ref": "#/definitions/ResourceUpdatedNotificationParam"
        }
      }
    },
    "NumberOrString": {
      "oneOf": [
        {
          "type": "number"
        },
        {
          "type": "string"
        }
      ]
    },
    "PingRequestMethod": {
      "type": "string",
      "format": "const",
      "const": "ping"
    },
    "ProgressNotificationMethod": {
      "type": "string",
      "format": "const",
      "const": "notifications/progress"
    },
    "ProgressNotificationParam": {
      "type": "object",
      "required": [
        "progress",
        "progressToken"
      ],
      "properties": {
        "message": {
          "description": "An optional message describing the current progress.",
          "type": [
            "string",
            "null"
          ]
        },
        "progress": {
          "description": "The progress thus far. This should increase every time progress is made, even if the total is unknown.",
          "type": "integer",
          "format": "uint32",
          "minimum": 0.0
        },
        "progressToken": {
          "$ref": "#/definitions/NumberOrString"
        },
        "total": {
          "description": "Total number of items to process (or total progress required), if known",
          "type": [
            "integer",
            "null"
          ],
          "format": "uint32",
          "minimum": 0.0
        }
      }
    },
    "Prompt": {
      "description": "A prompt that can be used to generate text from a model",
      "type": "object",
      "required": [
        "name"
      ],
      "properties": {
        "arguments": {
          "description": "Optional arguments that can be passed to customize the prompt",
          "type": [
            "array",
            "null"
          ],
          "items": {
            "$ref": "#/definitions/PromptArgument"
          }
        },
        "description": {
          "description": "Optional description of what the prompt does",
          "type": [
            "string",
            "null"
          ]
        },
        "name": {
          "description": "The name of the prompt",
          "type": "string"
        }
      }
    },
    "PromptArgument": {
      "description": "Represents a prompt argument that can be passed to customize the prompt",
      "type": "object",
      "required": [
        "name"
      ],
      "properties": {
        "description": {
          "description": "A description of what the argument is used for",
          "type": [
            "string",
            "null"
          ]
        },
        "name": {
          "description": "The name of the argument",
          "type": "string"
        },
        "required": {
          "description": "Whether this argument is required",
          "type": [
            "boolean",
            "null"
          ]
        }
      }
    },
    "PromptListChangedNotificationMethod": {
      "type": "string",
      "format": "const",
      "const": "notifications/prompts/list_changed"
    },
    "PromptMessage": {
      "description": "A message in a prompt conversation",
      "type": "object",
      "required": [
        "content",
        "role"
      ],
      "properties": {
        "content": {
          "description": "The content of the message",
          "allOf": [
            {
              "$ref": "#/definitions/PromptMessageContent"
            }
          ]
        },
        "role": {
          "description": "The role of the message sender",
          "allOf": [
            {
              "$ref": "#/definitions/PromptMessageRole"
            }
          ]
        }
      }
    },
    "PromptMessageContent": {
      "description": "Content types that can be included in prompt messages",
      "oneOf": [
        {
          "description": "Plain text content",
          "type": "object",
          "required": [
            "text",
            "type"
          ],
          "properties": {
            "text": {
              "type": "string"
            },
            "type": {
              "type": "string",
              "enum": [
                "text"
              ]
            }
          }
        },
        {
          "description": "Image content with base64-encoded data",
          "type": "object",
          "required": [
            "data",
            "mimeType",
            "type"
          ],
          "properties": {
            "annotations": {
              "anyOf": [
                {
                  "$ref": "#/definitions/Annotations"
                },
                {
                  "type": "null"
                }
              ]
            },
            "data": {
              "description": "The base64-encoded image",
              "type": "string"
            },
            "mimeType": {
              "type": "string"
            },
            "type": {
              "type": "string",
              "enum": [
                "image"
              ]
            }
          }
        },
        {
          "description": "Embedded server-side resource",
          "type": "object",
          "required": [
            "resource",
            "type"
          ],
          "properties": {
            "resource": {
              "$ref": "#/definitions/Annotated_for_RawEmbeddedResource"
            },
            "type": {
              "type": "string",
              "enum": [
                "resource"
              ]
            }
          }
        }
      ]
    },
    "PromptMessageRole": {
      "description": "Represents the role of a message sender in a prompt conversation",
      "type": "string",
      "enum": [
        "user",
        "assistant"
      ]
    },
    "PromptsCapability": {
      "type": "object",
      "properties": {
        "listChanged": {
          "type": [
            "boolean",
            "null"
          ]
        }
      }
    },
    "ProtocolVersion": {
      "type": "string"
    },
    "ReadResourceResult": {
      "type": "object",
      "required": [
        "contents"
      ],
      "properties": {
        "contents": {
          "type": "array",
          "items": {
            "$ref": "#/definitions/ResourceContents"
          }
        }
      }
    },
    "RequestNoParam_for_ListRootsRequestMethod": {
      "type": "object",
      "required": [
        "method"
      ],
      "properties": {
        "method": {
          "$ref": "#/definitions/ListRootsRequestMethod"
        }
      }
    },
    "RequestNoParam_for_PingRequestMethod": {
      "type": "object",
      "required": [
        "method"
      ],
      "properties": {
        "method": {
          "$ref": "#/definitions/PingRequestMethod"
        }
      }
    },
    "Request_for_CreateMessageRequestMethod_and_CreateMessageRequestParam": {
      "type": "object",
      "required": [
        "method",
        "params"
      ],
      "properties": {
        "method": {
          "$ref": "#/definitions/CreateMessageRequestMethod"
        },
        "params": {
          "$ref": "#/definitions/CreateMessageRequestParam"
        }
      }
    },
    "ResourceContents": {
      "anyOf": [
        {
          "type": "object",
          "required": [
            "text",
            "uri"
          ],
          "properties": {
            "mime_type": {
              "type": [
                "string",
                "null"
              ]
            },
            "text": {
              "type": "string"
            },
            "uri": {
              "type": "string"
            }
          }
        },
        {
          "type": "object",
          "required": [
            "blob",
            "uri"
          ],
          "properties": {
            "blob": {
              "type": "string"
            },
            "mime_type": {
              "type": [
                "string",
                "null"
              ]
            },
            "uri": {
              "type": "string"
            }
          }
        }
      ]
    },
    "ResourceListChangedNotificationMethod": {
      "type": "string",
      "format": "const",
      "const": "notifications/resources/list_changed"
    },
    "ResourceUpdatedNotificationMethod": {
      "type": "string",
      "format": "const",
      "const": "notifications/resources/updated"
    },
    "ResourceUpdatedNotificationParam": {
      "type": "object",
      "required": [
        "uri"
      ],
      "properties": {
        "uri": {
          "type": "string"
        }
      }
    },
    "ResourcesCapability": {
      "type": "object",
      "properties": {
        "listChanged": {
          "type": [
            "boolean",
            "null"
          ]
        },
        "subscribe": {
          "type": [
            "boolean",
            "null"
          ]
        }
      }
    },
    "Role": {
      "type": "string",
      "enum": [
        "user",
        "assistant"
      ]
    },
    "SamplingMessage": {
      "type": "object",
      "required": [
        "content",
        "role"
      ],
      "properties": {
        "content": {
          "$ref": "#/definitions/Annotated_for_RawContent"
        },
        "role": {
          "$ref": "#/definitions/Role"
        }
      }
    },
    "ServerCapabilities": {
      "title": "Builder",
      "description": "```rust # use rmcp::model::ServerCapabilities; let cap = ServerCapabilities::builder() .enable_logging() .enable_experimental() .enable_prompts() .enable_resources() .enable_tools() .enable_tool_list_changed() .build(); ```",
      "type": "object",
      "properties": {
        "completions": {
          "type": [
            "object",
            "null"
          ],
          "additionalProperties": true
        },
        "experimental": {
          "type": [
            "object",
            "null"
          ],
          "additionalProperties": {
            "type": "object",
            "additionalProperties": true
          }
        },
        "logging": {
          "type": [
            "object",
            "null"
          ],
          "additionalProperties": true
        },
        "prompts": {
          "anyOf": [
            {
              "$ref": "#/definitions/PromptsCapability"
            },
            {
              "type": "null"
            }
          ]
        },
        "resources": {
          "anyOf": [
            {
              "$ref": "#/definitions/ResourcesCapability"
            },
            {
              "type": "null"
            }
          ]
        },
        "tools": {
          "anyOf": [
            {
              "$ref": "#/definitions/ToolsCapability"
            },
            {
              "type": "null"
            }
          ]
        }
      }
    },
    "ServerResult": {
      "anyOf": [
        {
          "$ref": "#/definitions/InitializeResult"
        },
        {
          "$ref": "#/definitions/CompleteResult"
        },
        {
          "$ref": "#/definitions/GetPromptResult"
        },
        {
          "$ref": "#/definitions/ListPromptsResult"
        },
        {
          "$ref": "#/definitions/ListResourcesResult"
        },
        {
          "$ref": "#/definitions/ListResourceTemplatesResult"
        },
        {
          "$ref": "#/definitions/ReadResourceResult"
        },
        {
          "$ref": "#/definitions/CallToolResult"
        },
        {
          "$ref": "#/definitions/ListToolsResult"
        },
        {
          "$ref": "#/definitions/EmptyObject"
        }
      ]
    },
    "Tool": {
      "description": "A tool that can be used by a model.",
      "type": "object",
      "required": [
        "inputSchema",
        "name"
      ],
      "properties": {
        "annotations": {
          "description": "Optional additional tool information.",
          "anyOf": [
            {
              "$ref": "#/definitions/ToolAnnotations"
            },
            {
              "type": "null"
            }
          ]
        },
        "description": {
          "description": "A description of what the tool does",
          "type": [
            "string",
            "null"
          ]
        },
        "inputSchema": {
          "description": "A JSON Schema object defining the expected parameters for the tool",
          "type": "object",
          "additionalProperties": true
        },
        "name": {
          "description": "The name of the tool",
          "type": "string"
        }
      }
    },
    "ToolAnnotations": {
      "description": "Additional properties describing a Tool to clients.\n\nNOTE: all properties in ToolAnnotations are **hints**. They are not guaranteed to provide a faithful description of tool behavior (including descriptive properties like `title`).\n\nClients should never make tool use decisions based on ToolAnnotations received from untrusted servers.",
      "type": "object",
      "properties": {
        "destructiveHint": {
          "description": "If true, the tool may perform destructive updates to its environment. If false, the tool performs only additive updates.\n\n(This property is meaningful only when `readOnlyHint == false`)\n\nDefault: true A human-readable description of the tool's purpose.",
          "type": [
            "boolean",
            "null"
          ]
        },
        "idempotentHint": {
          "description": "If true, calling the tool repeatedly with the same arguments will have no additional effect on the its environment.\n\n(This property is meaningful only when `readOnlyHint == false`)\n\nDefault: false.",
          "type": [
            "boolean",
            "null"
          ]
        },
        "openWorldHint": {
          "description": "If true, this tool may interact with an \"open world\" of external entities. If false, the tool's domain of interaction is closed. For example, the world of a web search tool is open, whereas that of a memory tool is not.\n\nDefault: true",
          "type": [
            "boolean",
            "null"
          ]
        },
        "readOnlyHint": {
          "description": "If true, the tool does not modify its environment.\n\nDefault: false",
          "type": [
            "boolean",
            "null"
          ]
        },
        "title": {
          "description": "A human-readable title for the tool.",
          "type": [
            "string",
            "null"
          ]
        }
      }
    },
    "ToolListChangedNotificationMethod": {
      "type": "string",
      "format": "const",
      "const": "notifications/tools/list_changed"
    },
    "ToolsCapability": {
      "type": "object",
      "properties": {
        "listChanged": {
          "type": [
            "boolean",
            "null"
          ]
        }
      }
    }
  }
}

---
File: modelcontextprotocol-rust-sdk-13e72ec/crates/rmcp/tests/test_notification.rs
---

use std::sync::Arc;

use rmcp::{
    ClientHandler, ServerHandler, ServiceExt,
    model::{
        ResourceUpdatedNotificationParam, ServerCapabilities, ServerInfo, SubscribeRequestParam,
    },
};
use tokio::sync::Notify;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

pub struct Server {}

impl ServerHandler for Server {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            capabilities: ServerCapabilities::builder()
                .enable_resources()
                .enable_resources_subscribe()
                .enable_resources_list_changed()
                .build(),
            ..Default::default()
        }
    }

    async fn subscribe(
        &self,
        request: rmcp::model::SubscribeRequestParam,
        context: rmcp::service::RequestContext<rmcp::RoleServer>,
    ) -> Result<(), rmcp::Error> {
        let uri = request.uri;
        let peer = context.peer;

        tokio::spawn(async move {
            let span = tracing::info_span!("subscribe", uri = %uri);
            let _enter = span.enter();

            if let Err(e) = peer
                .notify_resource_updated(ResourceUpdatedNotificationParam { uri: uri.clone() })
                .await
            {
                panic!("Failed to send notification: {}", e);
            }
        });

        Ok(())
    }
}

pub struct Client {
    receive_signal: Arc<Notify>,
}

impl ClientHandler for Client {
    async fn on_resource_updated(
        &self,
        params: rmcp::model::ResourceUpdatedNotificationParam,
        _context: rmcp::service::NotificationContext<rmcp::RoleClient>,
    ) {
        let uri = params.uri;
        tracing::info!("Resource updated: {}", uri);
        self.receive_signal.notify_one();
    }
}

#[tokio::test]
async fn test_server_notification() -> anyhow::Result<()> {
    let _ = tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "debug".to_string().into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .try_init();
    let (server_transport, client_transport) = tokio::io::duplex(4096);
    tokio::spawn(async move {
        let server = Server {}.serve(server_transport).await?;
        server.waiting().await?;
        anyhow::Ok(())
    });
    let receive_signal = Arc::new(Notify::new());
    let client = Client {
        receive_signal: receive_signal.clone(),
    }
    .serve(client_transport)
    .await?;
    client
        .subscribe(SubscribeRequestParam {
            uri: "test://test-resource".to_owned(),
        })
        .await?;
    receive_signal.notified().await;
    client.cancel().await?;
    Ok(())
}

---
File: modelcontextprotocol-rust-sdk-13e72ec/crates/rmcp/tests/test_tool_macro_annotations.rs
---

#[cfg(test)]
mod tests {
    use rmcp::{ServerHandler, tool};

    #[derive(Debug, Clone, Default)]
    pub struct AnnotatedServer {}

    impl AnnotatedServer {
        // Tool with inline comments for documentation
        /// Direct annotation test tool
        /// This is used to test tool annotations
        #[tool(
            name = "direct-annotated-tool",
            annotations = {
                title: "Annotated Tool",
                readOnlyHint: true
            }
        )]
        pub async fn direct_annotated_tool(&self, #[tool(param)] input: String) -> String {
            format!("Direct: {}", input)
        }
    }

    impl ServerHandler for AnnotatedServer {
        async fn call_tool(
            &self,
            request: rmcp::model::CallToolRequestParam,
            context: rmcp::service::RequestContext<rmcp::RoleServer>,
        ) -> Result<rmcp::model::CallToolResult, rmcp::Error> {
            let tcc = rmcp::handler::server::tool::ToolCallContext::new(self, request, context);
            match tcc.name() {
                "direct-annotated-tool" => Self::direct_annotated_tool_tool_call(tcc).await,
                _ => Err(rmcp::Error::invalid_params("method not found", None)),
            }
        }
    }

    #[test]
    fn test_direct_tool_attributes() {
        // Get the tool definition
        let tool = AnnotatedServer::direct_annotated_tool_tool_attr();

        // Verify basic properties
        assert_eq!(tool.name, "direct-annotated-tool");

        // Verify description is extracted from doc comments
        assert!(tool.description.is_some());
        assert!(
            tool.description
                .as_ref()
                .unwrap()
                .contains("Direct annotation test tool")
        );

        let annotations = tool.annotations.unwrap();
        assert_eq!(annotations.title.as_ref().unwrap(), "Annotated Tool");
        assert_eq!(annotations.read_only_hint, Some(true));
    }
}

---
File: modelcontextprotocol-rust-sdk-13e72ec/crates/rmcp/tests/test_tool_macros.rs
---

//cargo test --test test_tool_macros --features "client server"

use std::sync::Arc;

use rmcp::{
    ClientHandler, ServerHandler, ServiceExt,
    handler::server::tool::ToolCallContext,
    model::{CallToolRequestParam, ClientInfo},
    tool,
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, JsonSchema)]
pub struct GetWeatherRequest {
    pub city: String,
    pub date: String,
}

impl ServerHandler for Server {
    async fn call_tool(
        &self,
        request: rmcp::model::CallToolRequestParam,
        context: rmcp::service::RequestContext<rmcp::RoleServer>,
    ) -> Result<rmcp::model::CallToolResult, rmcp::Error> {
        let tcc = ToolCallContext::new(self, request, context);
        match tcc.name() {
            "get-weather" => Self::get_weather_tool_call(tcc).await,
            _ => Err(rmcp::Error::invalid_params("method not found", None)),
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct Server {}

impl Server {
    /// This tool is used to get the weather of a city.
    #[tool(name = "get-weather", description = "Get the weather of a city.", vis = )]
    pub async fn get_weather(&self, #[tool(param)] city: String) -> String {
        drop(city);
        "rain".to_string()
    }
    #[tool(description = "Empty Parameter")]
    async fn empty_param(&self) {}

    #[tool(description = "Optional Parameter")]
    async fn optional_param(&self, #[tool(param)] city: Option<String>) -> String {
        city.unwrap_or_default()
    }
}

// define generic service trait
pub trait DataService: Send + Sync + 'static {
    fn get_data(&self) -> String;
}

// mock service for test
#[derive(Clone)]
struct MockDataService;
impl DataService for MockDataService {
    fn get_data(&self) -> String {
        "mock data".to_string()
    }
}

// define generic server
#[derive(Debug, Clone)]
pub struct GenericServer<DS: DataService> {
    data_service: Arc<DS>,
}

#[tool(tool_box)]
impl<DS: DataService> GenericServer<DS> {
    pub fn new(data_service: DS) -> Self {
        Self {
            data_service: Arc::new(data_service),
        }
    }

    #[tool(description = "Get data from the service")]
    async fn get_data(&self) -> String {
        self.data_service.get_data()
    }
}
#[tool(tool_box)]
impl<DS: DataService> ServerHandler for GenericServer<DS> {}

#[tokio::test]
async fn test_tool_macros() {
    let server = Server::default();
    let _attr = Server::get_weather_tool_attr();
    let _get_weather_call_fn = Server::get_weather_tool_call;
    let _get_weather_fn = Server::get_weather;
    server.get_weather("harbin".into()).await;
}

#[tokio::test]
async fn test_tool_macros_with_empty_param() {
    let _attr = Server::empty_param_tool_attr();
    println!("{_attr:?}");
    assert_eq!(_attr.input_schema.get("type").unwrap(), "object");
    assert!(_attr.input_schema.get("properties").is_none());
}

#[tokio::test]
async fn test_tool_macros_with_generics() {
    let mock_service = MockDataService;
    let server = GenericServer::new(mock_service);
    let _attr = GenericServer::<MockDataService>::get_data_tool_attr();
    let _get_data_call_fn = GenericServer::<MockDataService>::get_data_tool_call;
    let _get_data_fn = GenericServer::<MockDataService>::get_data;
    assert_eq!(server.get_data().await, "mock data");
}

#[tokio::test]
async fn test_tool_macros_with_optional_param() {
    let _attr = Server::optional_param_tool_attr();
    // println!("{_attr:?}");
    let attr_type = _attr
        .input_schema
        .get("properties")
        .unwrap()
        .get("city")
        .unwrap()
        .get("type")
        .unwrap();
    println!("_attr.input_schema: {:?}", attr_type);
    assert_eq!(attr_type.as_str().unwrap(), "string");
}

impl GetWeatherRequest {}

// Struct defined for testing optional field schema generation
#[derive(Debug, Deserialize, Serialize, JsonSchema)]
pub struct OptionalFieldTestSchema {
    #[schemars(description = "An optional description field")]
    pub description: Option<String>,
}

// Struct defined for testing optional i64 field schema generation and null handling
#[derive(Debug, Deserialize, Serialize, JsonSchema)]
pub struct OptionalI64TestSchema {
    #[schemars(description = "An optional i64 field")]
    pub count: Option<i64>,
    pub mandatory_field: String, // Added to ensure non-empty object schema
}

// Dummy struct to host the test tool method
#[derive(Debug, Clone, Default)]
pub struct OptionalSchemaTester {}

impl OptionalSchemaTester {
    // Dummy tool function using the test schema as an aggregated parameter
    #[tool(description = "A tool to test optional schema generation")]
    async fn test_optional_aggr(&self, #[tool(aggr)] _req: OptionalFieldTestSchema) {
        // Implementation doesn't matter for schema testing
        // Return type changed to () to satisfy IntoCallToolResult
    }

    // Tool function to test optional i64 handling
    #[tool(description = "A tool to test optional i64 schema generation")]
    async fn test_optional_i64_aggr(&self, #[tool(aggr)] req: OptionalI64TestSchema) -> String {
        match req.count {
            Some(c) => format!("Received count: {}", c),
            None => "Received null count".to_string(),
        }
    }
}

// Implement ServerHandler to route tool calls for OptionalSchemaTester
impl ServerHandler for OptionalSchemaTester {
    async fn call_tool(
        &self,
        request: rmcp::model::CallToolRequestParam,
        context: rmcp::service::RequestContext<rmcp::RoleServer>,
    ) -> Result<rmcp::model::CallToolResult, rmcp::Error> {
        let tcc = ToolCallContext::new(self, request, context);
        match tcc.name() {
            "test_optional_aggr" => Self::test_optional_aggr_tool_call(tcc).await,
            "test_optional_i64_aggr" => Self::test_optional_i64_aggr_tool_call(tcc).await,
            _ => Err(rmcp::Error::invalid_params("method not found", None)),
        }
    }
}

#[test]
fn test_optional_field_schema_generation_via_macro() {
    // tests https://github.com/modelcontextprotocol/rust-sdk/issues/135

    // Get the attributes generated by the #[tool] macro helper
    let tool_attr = OptionalSchemaTester::test_optional_aggr_tool_attr();

    // Print the actual generated schema for debugging
    println!(
        "Actual input schema generated by macro: {:#?}",
        tool_attr.input_schema
    );

    // Verify the schema generated for the aggregated OptionalFieldTestSchema
    // by the macro infrastructure (which should now use OpenAPI 3 settings)
    let input_schema_map = &*tool_attr.input_schema; // Dereference Arc<JsonObject>

    // Check the schema for the 'description' property within the input schema
    let properties = input_schema_map
        .get("properties")
        .expect("Schema should have properties")
        .as_object()
        .unwrap();
    let description_schema = properties
        .get("description")
        .expect("Properties should include description")
        .as_object()
        .unwrap();

    // Assert that the format is now `type: "string", nullable: true`
    assert_eq!(
        description_schema.get("type").map(|v| v.as_str().unwrap()),
        Some("string"),
        "Schema for Option<String> generated by macro should be type: \"string\""
    );
    assert_eq!(
        description_schema
            .get("nullable")
            .map(|v| v.as_bool().unwrap()),
        Some(true),
        "Schema for Option<String> generated by macro should have nullable: true"
    );
    // We still check the description is correct
    assert_eq!(
        description_schema
            .get("description")
            .map(|v| v.as_str().unwrap()),
        Some("An optional description field")
    );

    // Ensure the old 'type: [T, null]' format is NOT used
    let type_value = description_schema.get("type").unwrap();
    assert!(
        !type_value.is_array(),
        "Schema type should not be an array [T, null]"
    );
}

// Define a dummy client handler
#[derive(Debug, Clone, Default)]
struct DummyClientHandler {}

impl ClientHandler for DummyClientHandler {
    fn get_info(&self) -> ClientInfo {
        ClientInfo::default()
    }
}

#[tokio::test]
async fn test_optional_i64_field_with_null_input() -> anyhow::Result<()> {
    let (server_transport, client_transport) = tokio::io::duplex(4096);

    // Server setup
    let server = OptionalSchemaTester::default();
    let server_handle = tokio::spawn(async move {
        server.serve(server_transport).await?.waiting().await?;
        anyhow::Ok(())
    });

    // Create a simple client handler that just forwards tool calls
    let client_handler = DummyClientHandler::default();
    let client = client_handler.serve(client_transport).await?;

    // Test null case
    let result = client
        .call_tool(CallToolRequestParam {
            name: "test_optional_i64_aggr".into(),
            arguments: Some(
                serde_json::json!({
                    "count": null,
                    "mandatory_field": "test_null"
                })
                .as_object()
                .unwrap()
                .clone(),
            ),
        })
        .await?;

    let result_text = result
        .content
        .first()
        .and_then(|content| content.raw.as_text())
        .map(|text| text.text.as_str())
        .expect("Expected text content");

    assert_eq!(
        result_text, "Received null count",
        "Null case should return expected message"
    );

    // Test Some case
    let some_result = client
        .call_tool(CallToolRequestParam {
            name: "test_optional_i64_aggr".into(),
            arguments: Some(
                serde_json::json!({
                    "count": 42,
                    "mandatory_field": "test_some"
                })
                .as_object()
                .unwrap()
                .clone(),
            ),
        })
        .await?;

    let some_result_text = some_result
        .content
        .first()
        .and_then(|content| content.raw.as_text())
        .map(|text| text.text.as_str())
        .expect("Expected text content");

    assert_eq!(
        some_result_text, "Received count: 42",
        "Some case should return expected message"
    );

    client.cancel().await?;
    server_handle.await??;
    Ok(())
}

---
File: modelcontextprotocol-rust-sdk-13e72ec/crates/rmcp/tests/test_with_js.rs
---

use rmcp::{
    ServiceExt,
    service::QuitReason,
    transport::{
        ConfigureCommandExt, SseServer, StreamableHttpClientTransport, StreamableHttpServerConfig,
        TokioChildProcess,
        streamable_http_server::{
            session::local::LocalSessionManager, tower::StreamableHttpService,
        },
    },
};
use tokio_util::sync::CancellationToken;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};
mod common;
use common::calculator::Calculator;

const SSE_BIND_ADDRESS: &str = "127.0.0.1:8000";
const STREAMABLE_HTTP_BIND_ADDRESS: &str = "127.0.0.1:8001";
const STREAMABLE_HTTP_JS_BIND_ADDRESS: &str = "127.0.0.1:8002";

#[tokio::test]
async fn test_with_js_client() -> anyhow::Result<()> {
    let _ = tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "debug".to_string().into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .try_init();
    tokio::process::Command::new("npm")
        .arg("install")
        .current_dir("tests/test_with_js")
        .spawn()?
        .wait()
        .await?;

    let ct = SseServer::serve(SSE_BIND_ADDRESS.parse()?)
        .await?
        .with_service(Calculator::default);

    let exit_status = tokio::process::Command::new("node")
        .arg("tests/test_with_js/client.js")
        .spawn()?
        .wait()
        .await?;
    assert!(exit_status.success());
    ct.cancel();
    Ok(())
}

#[tokio::test]
async fn test_with_js_server() -> anyhow::Result<()> {
    let _ = tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "debug".to_string().into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .try_init();
    tokio::process::Command::new("npm")
        .arg("install")
        .current_dir("tests/test_with_js")
        .spawn()?
        .wait()
        .await?;
    let transport =
        TokioChildProcess::new(tokio::process::Command::new("node").configure(|cmd| {
            cmd.arg("tests/test_with_js/server.js");
        }))?;

    let client = ().serve(transport).await?;
    let resources = client.list_all_resources().await?;
    tracing::info!("{:#?}", resources);
    let tools = client.list_all_tools().await?;
    tracing::info!("{:#?}", tools);

    client.cancel().await?;
    Ok(())
}

#[tokio::test]
async fn test_with_js_streamable_http_client() -> anyhow::Result<()> {
    let _ = tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "debug".to_string().into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .try_init();
    tokio::process::Command::new("npm")
        .arg("install")
        .current_dir("tests/test_with_js")
        .spawn()?
        .wait()
        .await?;

    let service: StreamableHttpService<Calculator, LocalSessionManager> =
        StreamableHttpService::new(
            || Ok(Calculator),
            Default::default(),
            StreamableHttpServerConfig {
                stateful_mode: true,
                sse_keep_alive: None,
            },
        );
    let router = axum::Router::new().nest_service("/mcp", service);
    let tcp_listener = tokio::net::TcpListener::bind(STREAMABLE_HTTP_BIND_ADDRESS).await?;
    let ct = CancellationToken::new();
    let handle = tokio::spawn({
        let ct = ct.clone();
        async move {
            let _ = axum::serve(tcp_listener, router)
                .with_graceful_shutdown(async move { ct.cancelled_owned().await })
                .await;
        }
    });
    let exit_status = tokio::process::Command::new("node")
        .arg("tests/test_with_js/streamable_client.js")
        .spawn()?
        .wait()
        .await?;
    assert!(exit_status.success());
    ct.cancel();
    handle.await?;
    Ok(())
}

#[tokio::test]
async fn test_with_js_streamable_http_server() -> anyhow::Result<()> {
    let _ = tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "debug".to_string().into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .try_init();
    tokio::process::Command::new("npm")
        .arg("install")
        .current_dir("tests/test_with_js")
        .spawn()?
        .wait()
        .await?;

    let transport = StreamableHttpClientTransport::from_uri(format!(
        "http://{STREAMABLE_HTTP_JS_BIND_ADDRESS}/mcp"
    ));

    let mut server = tokio::process::Command::new("node")
        .arg("tests/test_with_js/streamable_server.js")
        .spawn()?;

    // waiting for server up
    tokio::time::sleep(tokio::time::Duration::from_millis(1000)).await;

    let client = ().serve(transport).await?;
    let resources = client.list_all_resources().await?;
    tracing::info!("{:#?}", resources);
    let tools = client.list_all_tools().await?;
    tracing::info!("{:#?}", tools);
    let quit_reason = client.cancel().await?;
    server.kill().await?;
    assert!(matches!(quit_reason, QuitReason::Cancelled));
    Ok(())
}

---
File: modelcontextprotocol-rust-sdk-13e72ec/crates/rmcp/tests/test_with_js/.gitignore
---

/node_modules
package-lock.json

---
File: modelcontextprotocol-rust-sdk-13e72ec/crates/rmcp/tests/test_with_js/client.js
---

import { Client } from "@modelcontextprotocol/sdk/client/index.js";
import { SSEClientTransport } from "@modelcontextprotocol/sdk/client/sse.js";

const transport = new SSEClientTransport(new URL(`http:

const client = new Client(
  {
    name: "example-client",
    version: "1.0.0"
  },
  {
    capabilities: {
      prompts: {},
      resources: {},
      tools: {}
    }
  }
);
await client.connect(transport);
const tools = await client.listTools();
console.log(tools);
const resources = await client.listResources();
console.log(resources);
const templates = await client.listResourceTemplates();
console.log(templates);
const prompts = await client.listPrompts();
console.log(prompts);
await client.close();
await transport.close();

---
File: modelcontextprotocol-rust-sdk-13e72ec/crates/rmcp/tests/test_with_js/package.json
---

{
  "dependencies": {
    "@modelcontextprotocol/sdk": "^1.10",
    "eventsource-parser": "^3.0.1",
    "express": "^5.1.0"
  },
  "type": "module",
  "name": "test_with_ts",
  "version": "1.0.0",
  "main": "index.js",
  "scripts": {
    "test": "echo \"Error: no test specified\" && exit 1"
  },
  "author": "",
  "license": "ISC",
  "description": ""
}

---
File: modelcontextprotocol-rust-sdk-13e72ec/crates/rmcp/tests/test_with_js/server.js
---

import { McpServer, ResourceTemplate } from "@modelcontextprotocol/sdk/server/mcp.js";
import { StdioServerTransport } from "@modelcontextprotocol/sdk/server/stdio.js";
import { z } from "zod";

const server = new McpServer({
  name: "Demo",
  version: "1.0.0"
});

server.resource(
  "greeting",
  new ResourceTemplate("greeting:
  async (uri, { name }) => ({
    contents: [{
      uri: uri.href,
      text: `Hello, ${name}`
    }]
  })
);

server.tool(
  "add",
  { a: z.number(), b: z.number() },
  async ({ a, b }) => ({
    "content": [
      {
        "type": "text",
        "text": `${a + b}`
      }
    ]
  })
);

const transport = new StdioServerTransport();
await server.connect(transport);

---
File: modelcontextprotocol-rust-sdk-13e72ec/crates/rmcp/tests/test_with_js/streamable_client.js
---

import { Client } from "@modelcontextprotocol/sdk/client/index.js";
import { StreamableHTTPClientTransport } from "@modelcontextprotocol/sdk/client/streamableHttp.js";

const transport = new StreamableHTTPClientTransport(new URL(`http:

const client = new Client(
  {
    name: "example-client",
    version: "1.0.0"
  },
  {
    capabilities: {
      prompts: {},
      resources: {},
      tools: {}
    }
  }
);
await client.connect(transport);
const tools = await client.listTools();
console.log(tools);
const resources = await client.listResources();
console.log(resources);
const templates = await client.listResourceTemplates();
console.log(templates);
const prompts = await client.listPrompts();
console.log(prompts);
await client.close();

---
File: modelcontextprotocol-rust-sdk-13e72ec/crates/rmcp/tests/test_with_js/streamable_server.js
---

import { McpServer, ResourceTemplate } from "@modelcontextprotocol/sdk/server/mcp.js";
import { StreamableHTTPServerTransport } from "@modelcontextprotocol/sdk/server/streamableHttp.js";
import { isInitializeRequest } from "@modelcontextprotocol/sdk/types.js"
import { randomUUID } from "node:crypto"
import { z } from "zod";
import express from "express"

const app = express();
app.use(express.json());


const transports = {};


app.post('/mcp', async (req, res) => {
  const sessionId = req.headers['mcp-session-id'];
  let transport;

  if (sessionId && transports[sessionId]) {
    transport = transports[sessionId];
  } else if (!sessionId && isInitializeRequest(req.body)) {
    transport = new StreamableHTTPServerTransport({
      sessionIdGenerator: () => randomUUID().toString(),
      onsessioninitialized: (sessionId) => {
        transports[sessionId] = transport;
      }
    });

    transport.onclose = () => {
      if (transport.sessionId) {
        delete transports[transport.sessionId];
      }
    };
    const server = new McpServer({
      name: "example-server",
      version: "1.0.0"
    });

    server.resource(
      "greeting",
      new ResourceTemplate("greeting:
      async (uri, { name }) => ({
        contents: [{
          uri: uri.href,
          text: `Hello, ${name}`
        }]
      })
    );

    server.tool(
      "add",
      { a: z.number(), b: z.number() },
      async ({ a, b }) => ({
        "content": [
          {
            "type": "text",
            "text": `${a + b}`
          }
        ]
      })
    );

    await server.connect(transport);
  } else {
    res.status(400).json({
      jsonrpc: '2.0',
      error: {
        code: -32000,
        message: 'Bad Request: No valid session ID provided',
      },
      id: null,
    });
    return;
  }

  await transport.handleRequest(req, res, req.body);
});


const handleSessionRequest = async (req, res) => {
  const sessionId = req.headers['mcp-session-id'];
  if (!sessionId || !transports[sessionId]) {
    res.status(400).send('Invalid or missing session ID');
    return;
  }

  const transport = transports[sessionId];
  await transport.handleRequest(req, res);
};


app.get('/mcp', handleSessionRequest);


app.delete('/mcp', handleSessionRequest);
console.log("Listening on port 8002");
app.listen(8002);

---
File: modelcontextprotocol-rust-sdk-13e72ec/crates/rmcp/tests/test_with_python.rs
---

use axum::Router;
use rmcp::{
    ServiceExt,
    transport::{ConfigureCommandExt, SseServer, TokioChildProcess, sse_server::SseServerConfig},
};
use tokio::time::timeout;
use tokio_util::sync::CancellationToken;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};
mod common;
use common::calculator::Calculator;

async fn init() -> anyhow::Result<()> {
    let _ = tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "debug".to_string().into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .try_init();
    tokio::process::Command::new("uv")
        .args(["pip", "install", "-r", "pyproject.toml"])
        .current_dir("tests/test_with_python")
        .spawn()?
        .wait()
        .await?;
    Ok(())
}

#[tokio::test]
async fn test_with_python_client() -> anyhow::Result<()> {
    init().await?;

    const BIND_ADDRESS: &str = "127.0.0.1:8000";

    let ct = SseServer::serve(BIND_ADDRESS.parse()?)
        .await?
        .with_service(Calculator::default);

    let status = tokio::process::Command::new("uv")
        .arg("run")
        .arg("tests/test_with_python/client.py")
        .arg(format!("http://{BIND_ADDRESS}/sse"))
        .spawn()?
        .wait()
        .await?;
    assert!(status.success());
    ct.cancel();
    Ok(())
}

/// Test the SSE server in a nested Axum router.
#[tokio::test]
async fn test_nested_with_python_client() -> anyhow::Result<()> {
    init().await?;

    const BIND_ADDRESS: &str = "127.0.0.1:8001";

    // Create an SSE router
    let sse_config = SseServerConfig {
        bind: BIND_ADDRESS.parse()?,
        sse_path: "/sse".to_string(),
        post_path: "/message".to_string(),
        ct: CancellationToken::new(),
        sse_keep_alive: None,
    };

    let listener = tokio::net::TcpListener::bind(&sse_config.bind).await?;

    let (sse_server, sse_router) = SseServer::new(sse_config);
    let ct = sse_server.with_service(Calculator::default);

    let main_router = Router::new().nest("/nested", sse_router);

    let server_ct = ct.clone();
    let server = axum::serve(listener, main_router).with_graceful_shutdown(async move {
        server_ct.cancelled().await;
        tracing::info!("sse server cancelled");
    });

    tokio::spawn(async move {
        let _ = server.await;
        tracing::info!("sse server shutting down");
    });

    // Spawn the process with timeout, as failure to access the '/message' URL
    // causes the client to never exit.
    let status = timeout(
        tokio::time::Duration::from_secs(5),
        tokio::process::Command::new("uv")
            .arg("run")
            .arg("tests/test_with_python/client.py")
            .arg(format!("http://{BIND_ADDRESS}/nested/sse"))
            .spawn()?
            .wait(),
    )
    .await?;
    assert!(status?.success());
    ct.cancel();
    Ok(())
}

#[tokio::test]
async fn test_with_python_server() -> anyhow::Result<()> {
    init().await?;

    let transport = TokioChildProcess::new(tokio::process::Command::new("uv").configure(|cmd| {
        cmd.arg("run").arg("tests/test_with_python/server.py");
    }))?;

    let client = ().serve(transport).await?;
    let resources = client.list_all_resources().await?;
    tracing::info!("{:#?}", resources);
    let tools = client.list_all_tools().await?;
    tracing::info!("{:#?}", tools);
    client.cancel().await?;
    Ok(())
}

---
File: modelcontextprotocol-rust-sdk-13e72ec/crates/rmcp/tests/test_with_python/client.py
---

from mcp import ClientSession, StdioServerParameters, types
from mcp.client.sse import sse_client
import sys

async def run():
    url = sys.argv[1]
    async with sse_client(url) as (read, write):
        async with ClientSession(
            read, write
        ) as session:
            await session.initialize()

            prompts = await session.list_prompts()
            print(prompts)
            resources = await session.list_resources()
            print(resources)

            tools = await session.list_tools()
            print(tools)

if __name__ == "__main__":
    import asyncio

    asyncio.run(run())

---
File: modelcontextprotocol-rust-sdk-13e72ec/crates/rmcp/tests/test_with_python/pyproject.toml
---

[build-system]
requires = ["setuptools>=42", "wheel"]
build-backend = "setuptools.build_meta"

[project]
name = "test_with_python"
version = "0.1.0"
description = "Test Python client for RMCP"
dependencies = [
  "mcp",
]

[tool.setuptools]
py-modules = ["client", "server"]

---
File: modelcontextprotocol-rust-sdk-13e72ec/crates/rmcp/tests/test_with_python/server.py
---

from mcp.server.fastmcp import FastMCP

mcp = FastMCP("Demo")

@mcp.tool()
def add(a: int, b: int) -> int:
    return a + b



@mcp.resource("greeting://{name}")
def get_greeting(name: str) -> str:
    return f"Hello, {name}!"



if __name__ == "__main__":
    mcp.run()

---
File: modelcontextprotocol-rust-sdk-13e72ec/docs/DEVCONTAINER.md
---

## Development with Dev Container and GitHub Codespaces

This repository provides a Dev Container to easily set up a development environment. Using Dev Container allows you to work in a consistent development environment with pre-configured dependencies and tools, whether locally or in the cloud with GitHub Codespaces.

### Prerequisites

**For Local Development:**

* [Docker Desktop](https://www.docker.com/products/docker-desktop/) or any other compatible container runtime (e.g., Podman, OrbStack) installed.
* [Visual Studio Code](https://code.visualstudio.com/) with the [Remote - Containers extension](https://marketplace.visualstudio.com/items?itemName=ms-vscode-remote.remote-containers) installed.

**For GitHub Codespaces:**

* A GitHub account.

### Starting Dev Container

**Using Visual Studio Code (Local):**

1.  Clone the repository.
2.  Open the repository in Visual Studio Code.
3.  Open the command palette in Visual Studio Code (`Ctrl + Shift + P` or `Cmd + Shift + P`) and execute `Dev Containers: Reopen in Container`.

**Using GitHub Codespaces (Cloud):**

1.  Navigate to the repository on GitHub.
2.  Click the "<> Code" button.
3.  Select the "Codespaces" tab.
4.  Click "Create codespace on main" (or your desired branch).

### Dev Container Configuration

Dev Container settings are configured in `.devcontainer/devcontainer.json`. In this file, you can set the Docker image to use, extensions to install, port forwarding, and more. This configuration is used both for local development and GitHub Codespaces.

### Development

Once the Dev Container is started, you can proceed with development as usual. The container already has the necessary tools and libraries installed. In GitHub Codespaces, you will have a fully configured VS Code in your browser or desktop application.

### Stopping Dev Container

**Using Visual Studio Code (Local):**

To stop the Dev Container, open the command palette in Visual Studio Code and execute `Remote: Close Remote Connection`.

**Using GitHub Codespaces (Cloud):**

GitHub Codespaces will automatically stop after a period of inactivity. You can also manually stop the codespace from the Codespaces menu in GitHub.

### More Information

* [Visual Studio Code Dev Containers](https://code.visualstudio.com/docs/remote/containers)
* [Dev Container Specification](https://containers.dev/implementors/json_reference/)
* [GitHub Codespaces](https://github.com/features/codespaces)

This document describes the basic usage of Dev Container and GitHub Codespaces. Add project-specific settings and procedures as needed.

---
File: modelcontextprotocol-rust-sdk-13e72ec/docs/OAUTH_SUPPORT.md
---

# Model Context Protocol OAuth Authorization

This document describes the OAuth 2.1 authorization implementation for Model Context Protocol (MCP), following the [MCP 2025-03-26 Authorization Specification](https://spec.modelcontextprotocol.io/specification/2025-03-26/basic/authorization/).

## Features

- Full support for OAuth 2.1 authorization flow
- PKCE support for enhanced security
- Authorization server metadata discovery
- Dynamic client registration
- Automatic token refresh
- Authorized SSE transport implementation
- Authorized HTTP Client implementation
## Usage Guide

### 1. Enable Features

Enable the auth feature in Cargo.toml:

```toml
[dependencies]
rmcp = { version = "0.1", features = ["auth", "transport-sse-client"] }
```

### 2. Use OAuthState

```rust ignore
    // Initialize oauth state machine
    let mut oauth_state = OAuthState::new(&server_url, None)
        .await
        .context("Failed to initialize oauth state machine")?;
    oauth_state
        .start_authorization(&["mcp", "profile", "email"], MCP_REDIRECT_URI)
        .await
        .context("Failed to start authorization")?;

```

### 3. Get authorization url and do callback

```rust ignore
    // Get authorization URL and guide user to open it
    let auth_url = oauth_state.get_authorization_url().await?;
    println!("Please open the following URL in your browser for authorization:\n{}", auth_url);
    // Handle callback - In real applications, this is typically done in a callback server
    let auth_code = "Authorization code obtained from browser after user authorization";
    let credentials = oauth_state.handle_callback(auth_code).await?;
    println!("Authorization successful, access token: {}", credentials.access_token);

```

### 4. Use Authorized SSE Transport and create client

```rust ignore
    let transport =
        match create_authorized_transport(MCP_SSE_URL.to_string(), oauth_state, Some(retry_config))
            .await
        {
            Ok(t) => t,
            Err(e) => {
                tracing::error!("Failed to create authorized transport: {}", e);
                return Err(anyhow::anyhow!("Connection failed: {}", e));
            }
        };

    // Create client and connect to MCP server
    let client_service = ClientInfo::default();
    let client = client_service.serve(transport).await?;
```

### 5. May you can use Authorized HTTP Client after authorized

```rust ignore
    let client = oauth_state.to_authorized_http_client().await?;
```

## Complete Example
client: Please refer to `examples/clients/src/oauth_client.rs` for a complete usage example.
server: Please refer to `examples/servers/src/mcp_oauth_server.rs` for a complete usage example.
### Running the Example in server
```bash
# Run example
cargo run --example mcp_oauth_server
```

### Running the Example in client

```bash
# Run example
cargo run --example oauth-client
```

## Authorization Flow Description

1. **Metadata Discovery**: Client attempts to get authorization server metadata from `/.well-known/oauth-authorization-server`
2. **Client Registration**: If supported, client dynamically registers itself
3. **Authorization Request**: Build authorization URL with PKCE and guide user to access
4. **Authorization Code Exchange**: After user authorization, exchange authorization code for access token
5. **Token Usage**: Use access token for API calls
6. **Token Refresh**: Automatically use refresh token to get new access token when current one expires

## Security Considerations

- All tokens are securely stored in memory
- PKCE implementation prevents authorization code interception attacks
- Automatic token refresh support reduces user intervention
- Only accepts HTTPS connections or secure local callback URIs

## Troubleshooting

If you encounter authorization issues, check the following:

1. Ensure server supports OAuth 2.1 authorization
2. Verify callback URI matches server's allowed redirect URIs
3. Check network connection and firewall settings
4. Verify server supports metadata discovery or dynamic client registration

## References

- [MCP Authorization Specification](https://spec.modelcontextprotocol.io/specification/2025-03-26/basic/authorization/)
- [OAuth 2.1 Specification Draft](https://oauth.net/2.1/)
- [RFC 8414: OAuth 2.0 Authorization Server Metadata](https://datatracker.ietf.org/doc/html/rfc8414)
- [RFC 7591: OAuth 2.0 Dynamic Client Registration Protocol](https://datatracker.ietf.org/doc/html/rfc7591)

---
File: modelcontextprotocol-rust-sdk-13e72ec/docs/readme/README.zh-cn.md
---

# RMCP
[![Crates.io Version](https://img.shields.io/crates/v/rmcp)](https://crates.io/crates/rmcp)
![Release status](https://github.commodelcontextprotocol/rust-sdk/actions/workflows/release.yml/badge.svg)
[![docs.rs](https://img.shields.io/docsrs/rmcp)](https://docs.rs/rmcp/latest/rmcp)

一个基于tokio异步运行时的官方Model Context Protocol SDK实现。

## 使用

### 导入
```toml
rmcp = { version = "0.1", features = ["server"] }
## 或者开发者频道
rmcp = { git = "https://github.com/modelcontextprotocol/rust-sdk", branch = "main" }
```

### 快速上手
一行代码启动客户端：
```rust
use rmcp::{ServiceExt, transport::{TokioChildProcess, ConfigureCommandExt}};
use tokio::process::Command;

let client = ().serve(TokioChildProcess::new(Command::new("npx").configure(|cmd| {
    cmd.arg("-y").arg("@modelcontextprotocol/server-everything");
}))?).await?;
```

#### 1. 构建传输层

```rust, ignore
use tokio::io::{stdin, stdout};
let transport = (stdin(), stdout());
```

传输层类型必须实现 [`IntoTransport`](crate::transport::IntoTransport) trait, 这个特性允许分割成一个sink和一个stream。

对于客户端, Sink 的 Item 是 [`ClientJsonRpcMessage`](crate::model::ClientJsonRpcMessage)， Stream 的 Item 是 [`ServerJsonRpcMessage`](crate::model::ServerJsonRpcMessage)

对于服务端, Sink 的 Item 是 [`ServerJsonRpcMessage`](crate::model::ServerJsonRpcMessage)， Stream 的 Item 是 [`ClientJsonRpcMessage`](crate::model::ClientJsonRpcMessage)

##### 这些类型自动实现了 [`IntoTransport`](crate::transport::IntoTransport) trait
1. 已经同时实现了 [`Sink`](futures::Sink) 和 [`Stream`](futures::Stream) trait的类型。
2. 由sink `Tx` 和 stream `Rx`组成的元组: `(Tx, Rx)`。
3. 同时实现了 [`tokio::io::AsyncRead`] 和 [`tokio::io::AsyncWrite`] trait的类型。
4. 由 [`tokio::io::AsyncRead`] `R `和 [`tokio::io::AsyncWrite`] `W` 组成的元组:  `(R, W)`。

例如，你可以看到我们如何轻松地通过TCP流或http升级构建传输层。 [examples](examples/README.md)

#### 2. 构建服务
你可以通过 [`ServerHandler`](crates/rmcp/src/handler/server.rs) 或 [`ClientHandler`](crates/rmcp/src/handler/client.rs) 轻松构建服务

```rust, ignore
let service = common::counter::Counter::new();
```

#### 3. 把他们组装到一起
```rust, ignore
// 这里会自动完成初始化流程
let server = service.serve(transport).await?;
```

#### 4. 与服务交互
一旦服务初始化完成，你可以发送请求或通知：

```rust, ignore
// 请求
let roots = server.list_roots().await?;

// 或发送通知
server.notify_cancelled(...).await?;
```

#### 5. 等待服务关闭
```rust, ignore
let quit_reason = server.waiting().await?;
// 或取消它
let quit_reason = server.cancel().await?;
```

### 使用宏来声明工具
使用 `toolbox` 和 `tool` 宏来快速创建工具。

请看这个[文件](examples/servers/src/common/calculator.rs)。
```rust, ignore
use rmcp::{ServerHandler, model::ServerInfo, schemars, tool};

use super::counter::Counter;

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct SumRequest {
    #[schemars(description = "the left hand side number")]
    pub a: i32,
    #[schemars(description = "the right hand side number")]
    pub b: i32,
}
#[derive(Debug, Clone)]
pub struct Calculator;

// create a static toolbox to store the tool attributes
#[tool(tool_box)]
impl Calculator {
    // async function
    #[tool(description = "Calculate the sum of two numbers")]
    async fn sum(&self, #[tool(aggr)] SumRequest { a, b }: SumRequest) -> String {
        (a + b).to_string()
    }

    // sync function
    #[tool(description = "Calculate the sum of two numbers")]
    fn sub(
        &self,
        #[tool(param)]
        // this macro will transfer the schemars and serde's attributes
        #[schemars(description = "the left hand side number")]
        a: i32,
        #[tool(param)]
        #[schemars(description = "the right hand side number")]
        b: i32,
    ) -> String {
        (a - b).to_string()
    }
}

// impl call_tool and list_tool by querying static toolbox
#[tool(tool_box)]
impl ServerHandler for Calculator {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            instructions: Some("A simple calculator".into()),
            ..Default::default()
        }
    }
}

```
你要做的唯一事情就是确保函数的返回类型实现了 `IntoCallToolResult`。

你可以为返回类型实现 `IntoContents`，那么返回值将自动标记为成功。

如果返回类型是 `Result<T, E>`，其中 `T` 与 `E` 都实现了 `IntoContents`，那也是可以的。

### 管理多个服务
在很多情况下你需要在一个集合中管理多个服务，你可以调用 `into_dyn` 来将服务转换为相同类型。
```rust, ignore
let service = service.into_dyn();
```


### 示例
查看 [examples](examples/README.md)

### 功能特性
- `client`: 使用客户端sdk
- `server`: 使用服务端sdk
- `macros`: 宏默认
#### 传输层
- `transport-io`: 服务端标准输入输出传输
- `transport-sse-server`: 服务端SSE传输
- `transport-child-process`: 客户端标准输入输出传输
- `transport-sse`: 客户端SSE传输

## 相关资源
- [MCP Specification](https://spec.modelcontextprotocol.io/specification/2024-11-05/)

- [Schema](https://github.com/modelcontextprotocol/specification/blob/main/schema/2024-11-05/schema.ts)

---
File: modelcontextprotocol-rust-sdk-13e72ec/examples/README.md
---

# Quick Start With Claude Desktop

1. **Build the Server (Counter Example)**

   ```sh
   cargo build --release --example servers_counter_stdio
   ```

   This builds a standard input/output MCP server binary.

2. **Add or update this section in your** `PATH-TO/claude_desktop_config.json`

   Windows

   ```json
   {
     "mcpServers": {
       "counter": {
         "command": "PATH-TO/rust-sdk/target/release/examples/servers_counter_stdio.exe",
         "args": []
       }
     }
   }
   ```

   MacOS/Linux

   ```json
   {
     "mcpServers": {
       "counter": {
         "command": "PATH-TO/rust-sdk/target/release/examples/servers_counter_stdio",
         "args": []
       }
     }
   }
   ```

3. **Ensure that the MCP UI elements appear in Claude Desktop**
   The MCP UI elements will only show up in Claude for Desktop if at least one server is properly configured. It may require to restart Claude for Desktop.

4. **Once Claude Desktop is running, try chatting:**

   ```text
   counter.say_hello
   ```

   Or test other tools like:

   ```texts
   counter.increment
   counter.get_value
   counter.sum {"a": 3, "b": 4}
   ```

# Client Examples

see [clients/README.md](clients/README.md)

# Server Examples

see [servers/README.md](servers/README.md)

# Transport Examples

- [Tcp](transport/src/tcp.rs)
- [Transport on http upgrade](transport/src/http_upgrade.rs)
- [Unix Socket](transport/src/unix_socket.rs)
- [Websocket](transport/src/websocket.rs)

# Integration

- [Rig](examples/rig-integration) A stream chatbot with rig
- [Simple Chat Client](examples/simple-chat-client) A simple chat client implementation using the Model Context Protocol (MCP) SDK.

# WASI

- [WASI-P2 runtime](wasi) How it works with wasip2

## Use Mcp Inspector

```sh
npx @modelcontextprotocol/inspector
```

---
File: modelcontextprotocol-rust-sdk-13e72ec/examples/clients/Cargo.toml
---

[package]
name = "mcp-client-examples"
version = "0.1.5"
edition = "2024"
publish = false

[dependencies]
rmcp = { path = "../../crates/rmcp", features = [
    "client",
    "transport-sse-client",
    "reqwest",
    "transport-streamable-http-client",
    "transport-child-process",
    "tower",
    "auth"
] }
tokio = { version = "1", features = ["full"] }
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter"] }
rand = "0.9"
futures = "0.3"
anyhow = "1.0"
url = "2.4"
tower = "0.5"
axum = "0.8"
reqwest = "0.12"

[[example]]
name = "clients_sse"
path = "src/sse.rs"

[[example]]
name = "clients_git_stdio"
path = "src/git_stdio.rs"

[[example]]
name = "clients_streamable_http"
path = "src/streamable_http.rs"

[[example]]
name = "clients_everything_stdio"
path = "src/everything_stdio.rs"

[[example]]
name = "clients_collection"
path = "src/collection.rs"

[[example]]
name = "clients_oauth_client"
path = "src/auth/oauth_client.rs"

---
File: modelcontextprotocol-rust-sdk-13e72ec/examples/clients/README.md
---

# MCP Client Examples

This directory contains Model Context Protocol (MCP) client examples implemented in Rust. These examples demonstrate how to communicate with MCP servers using different transport methods and how to use various client APIs.

## Example List

### SSE Client (`sse.rs`)

A client that communicates with an MCP server using Server-Sent Events (SSE) transport.

- Connects to an MCP server running at `http://localhost:8000/sse`
- Retrieves server information and list of available tools
- Calls a tool named "increment"

### Git Standard I/O Client (`git_stdio.rs`)

A client that communicates with a Git-related MCP server using standard input/output.

- Launches the `uvx mcp-server-git` command as a child process
- Retrieves server information and list of available tools
- Calls the `git_status` tool to check the Git status of the current directory

### Streamable HTTP Client (`streamable_http.rs`)

A client that communicates with an MCP server using HTTP streaming transport.
- Connects to an MCP server running at `http://localhost:8000`
- Retrieves server information and list of available tools
- Calls a tool named "increment"

### Full-Featured Standard I/O Client (`everything_stdio.rs`)

An example demonstrating all MCP client capabilities.

- Launches `npx -y @modelcontextprotocol/server-everything` as a child process
- Retrieves server information and list of available tools
- Calls various tools, including "echo" and "longRunningOperation"
- Lists and reads available resources
- Lists and retrieves simple and complex prompts
- Lists available resource templates

### Client Collection (`collection.rs`)

An example showing how to manage multiple MCP clients.

- Creates 10 clients connected to Git servers
- Stores these clients in a HashMap
- Performs the same sequence of operations on each client
- Uses `into_dyn()` to convert services to dynamic services

### OAuth Client (`auth/oauth_client.rs`)

A client demonstrating how to authenticate with an MCP server using OAuth.

- Starts a local HTTP server to handle OAuth callbacks
- Initializes the OAuth state machine and begins the authorization flow
- Displays the authorization URL and waits for user authorization
- Establishes an authorized connection to the MCP server using the acquired access token
- Demonstrates how to use the authorized connection to retrieve available tools and prompts

## How to Run

Each example can be run using Cargo:

```bash
# Run the SSE client example
cargo run --example clients_sse

# Run the Git standard I/O client example
cargo run --example clients_git_stdio

# Run the streamable HTTP client example
cargo run --example clients_streamable_http

# Run the full-featured standard I/O client example
cargo run --example clients_everything_stdio

# Run the client collection example
cargo run --example clients_collection

# Run the OAuth client example
cargo run --example clients_oauth_client
```

## Dependencies

These examples use the following main dependencies:

- `rmcp`: Rust implementation of the MCP client library
- `tokio`: Asynchronous runtime
- `serde` and `serde_json`: For JSON serialization and deserialization
- `tracing` and `tracing-subscriber`: For logging, not must, only for logging
- `anyhow`: Error handling, not must, only for error handling
- `axum`: For the OAuth callback HTTP server (used only in the OAuth example)
- `reqwest`: HTTP client library (used for OAuth and streamable HTTP transport)

---
File: modelcontextprotocol-rust-sdk-13e72ec/examples/clients/src/auth/callback.html
---

<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>OAuth Success</title>
    <style>
        :root {
            --primary-color: #4CAF50;
            --background-color: #f8f9fa;
            --text-color: #333;
            --border-color: #e0e0e0;
        }

        body {
            font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, "Helvetica Neue", Arial, sans-serif;
            margin: 0;
            padding: 0;
            min-height: 100vh;
            display: flex;
            align-items: center;
            justify-content: center;
            background-color: var(--background-color);
            color: var(--text-color);
        }

        .container {
            background: white;
            padding: 2rem;
            border-radius: 12px;
            box-shadow: 0 4px 6px rgba(0, 0, 0, 0.1);
            text-align: center;
            max-width: 500px;
            width: 90%;
            margin: 1rem;
        }

        .icon {
            font-size: 4rem;
            color: var(--primary-color);
            margin-bottom: 1rem;
            animation: scaleIn 0.5s ease-out;
        }

        h1 {
            color: var(--primary-color);
            margin: 0 0 1rem 0;
            font-size: 1.8rem;
        }

        p {
            margin: 0;
            line-height: 1.6;
            color: #666;
        }

        .close-button {
            margin-top: 1.5rem;
            padding: 0.75rem 1.5rem;
            background-color: var(--primary-color);
            color: white;
            border: none;
            border-radius: 6px;
            cursor: pointer;
            font-size: 1rem;
            transition: background-color 0.2s;
        }

        .close-button:hover {
            background-color: #45a049;
        }

        @keyframes scaleIn {
            from {
                transform: scale(0);
                opacity: 0;
            }
            to {
                transform: scale(1);
                opacity: 1;
            }
        }
    </style>
</head>
<body>
    <div class="container">
        <div class="icon">âœ“</div>
        <h1>Authorization Success</h1>
        <p>You have successfully authorized the MCP client. You can now close this window and return to the application.</p>
        <button class="close-button" onclick="window.close()">Close Window</button>
    </div>
</body>
</html>

---
File: modelcontextprotocol-rust-sdk-13e72ec/examples/clients/src/auth/oauth_client.rs
---

use std::{net::SocketAddr, sync::Arc};

use anyhow::{Context, Result};
use axum::{
    Router,
    extract::{Query, State},
    response::Html,
    routing::get,
};
use rmcp::{
    ServiceExt,
    model::ClientInfo,
    transport::{
        SseClientTransport,
        auth::{AuthClient, OAuthState},
        sse_client::SseClientConfig,
    },
};
use serde::Deserialize;
use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader, BufWriter},
    sync::{Mutex, oneshot},
};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

const MCP_SERVER_URL: &str = "http://localhost:3000/mcp";
const MCP_REDIRECT_URI: &str = "http://localhost:8080/callback";
const MCP_SSE_URL: &str = "http://localhost:3000/mcp/sse";
const CALLBACK_PORT: u16 = 8080;
const CALLBACK_HTML: &str = include_str!("callback.html");

#[derive(Clone)]
struct AppState {
    code_receiver: Arc<Mutex<Option<oneshot::Sender<String>>>>,
}

#[derive(Debug, Deserialize)]
struct CallbackParams {
    code: String,
    #[allow(dead_code)]
    state: Option<String>,
}

async fn callback_handler(
    Query(params): Query<CallbackParams>,
    State(state): State<AppState>,
) -> Html<String> {
    tracing::info!("Received callback with code: {}", params.code);

    // Send the code to the main thread
    if let Some(sender) = state.code_receiver.lock().await.take() {
        let _ = sender.send(params.code);
    }
    // Return success page
    Html(CALLBACK_HTML.to_string())
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "debug".to_string().into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();
    // it is a http server for handling callback
    // Create channel for receiving authorization code
    let (code_sender, code_receiver) = oneshot::channel::<String>();

    // Create app state
    let app_state = AppState {
        code_receiver: Arc::new(Mutex::new(Some(code_sender))),
    };

    // Start HTTP server for handling callbacks
    let app = Router::new()
        .route("/callback", get(callback_handler))
        .with_state(app_state);

    let addr = SocketAddr::from(([127, 0, 0, 1], CALLBACK_PORT));
    tracing::info!("Starting callback server at: http://{}", addr);

    // Start server in a separate task
    tokio::spawn(async move {
        let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
        let result = axum::serve(listener, app).await;

        if let Err(e) = result {
            tracing::error!("Callback server error: {}", e);
        }
    });

    // Get server URL
    let server_url = MCP_SERVER_URL.to_string();
    tracing::info!("Using MCP server URL: {}", server_url);

    // Initialize oauth state machine
    let mut oauth_state = OAuthState::new(&server_url, None)
        .await
        .context("Failed to initialize oauth state machine")?;
    oauth_state
        .start_authorization(&["mcp", "profile", "email"], MCP_REDIRECT_URI)
        .await
        .context("Failed to start authorization")?;

    // Output authorization URL to user
    let mut output = BufWriter::new(tokio::io::stdout());
    output.write_all(b"\n=== MCP OAuth Client ===\n\n").await?;
    output
        .write_all(b"Please open the following URL in your browser to authorize:\n\n")
        .await?;
    output
        .write_all(oauth_state.get_authorization_url().await?.as_bytes())
        .await?;
    output
        .write_all(b"\n\nWaiting for browser callback, please do not close this window...\n")
        .await?;
    output.flush().await?;

    // Wait for authorization code
    tracing::info!("Waiting for authorization code...");
    let auth_code = code_receiver
        .await
        .context("Failed to get authorization code")?;
    tracing::info!("Received authorization code: {}", auth_code);
    // Exchange code for access token
    tracing::info!("Exchanging authorization code for access token...");
    oauth_state
        .handle_callback(&auth_code)
        .await
        .context("Failed to handle callback")?;
    tracing::info!("Successfully obtained access token");

    output
        .write_all(b"\nAuthorization successful! Access token obtained.\n\n")
        .await?;
    output.flush().await?;

    // Create authorized transport, this transport is authorized by the oauth state machine
    tracing::info!("Establishing authorized connection to MCP server...");
    let am = oauth_state
        .into_authorization_manager()
        .ok_or_else(|| anyhow::anyhow!("Failed to get authorization manager"))?;
    let client = AuthClient::new(reqwest::Client::default(), am);
    let transport = SseClientTransport::start_with_client(
        client,
        SseClientConfig {
            sse_endpoint: MCP_SSE_URL.into(),
            ..Default::default()
        },
    )
    .await?;

    // Create client and connect to MCP server
    let client_service = ClientInfo::default();
    let client = client_service.serve(transport).await?;
    tracing::info!("Successfully connected to MCP server");

    // Test API requests
    output
        .write_all(b"Fetching available tools from server...\n")
        .await?;
    output.flush().await?;

    match client.peer().list_all_tools().await {
        Ok(tools) => {
            output
                .write_all(format!("Available tools: {}\n\n", tools.len()).as_bytes())
                .await?;
            for tool in tools {
                output
                    .write_all(
                        format!(
                            "- {} ({})\n",
                            tool.name,
                            tool.description.unwrap_or_default()
                        )
                        .as_bytes(),
                    )
                    .await?;
            }
        }
        Err(e) => {
            output
                .write_all(format!("Error fetching tools: {}\n", e).as_bytes())
                .await?;
        }
    }

    output
        .write_all(b"\nFetching available prompts from server...\n")
        .await?;
    output.flush().await?;

    match client.peer().list_all_prompts().await {
        Ok(prompts) => {
            output
                .write_all(format!("Available prompts: {}\n\n", prompts.len()).as_bytes())
                .await?;
            for prompt in prompts {
                output
                    .write_all(format!("- {}\n", prompt.name).as_bytes())
                    .await?;
            }
        }
        Err(e) => {
            output
                .write_all(format!("Error fetching prompts: {}\n", e).as_bytes())
                .await?;
        }
    }

    output
        .write_all(b"\nConnection established successfully. You are now authenticated with the MCP server.\n")
        .await?;
    output.flush().await?;

    // Keep the program running, wait for user input to exit
    output.write_all(b"\nPress Enter to exit...\n").await?;
    output.flush().await?;

    let mut input = String::new();
    let mut reader = BufReader::new(tokio::io::stdin());
    reader.read_line(&mut input).await?;

    Ok(())
}

---
File: modelcontextprotocol-rust-sdk-13e72ec/examples/clients/src/collection.rs
---

/// This example show how to store multiple clients in a map and call tools on them.
/// into_dyn() is used to convert the service to a dynamic service.
/// For example, you can use this to call tools on a service that is running in a different process.
/// or a service that is running in a different machine.
use std::collections::HashMap;

use anyhow::Result;
use rmcp::{
    model::CallToolRequestParam,
    service::ServiceExt,
    transport::{ConfigureCommandExt, TokioChildProcess},
};
use tokio::process::Command;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| format!("info,{}=debug", env!("CARGO_CRATE_NAME")).into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    let mut clients_map = HashMap::new();
    for idx in 0..10 {
        let service = ()
            .into_dyn()
            .serve(TokioChildProcess::new(Command::new("uvx").configure(
                |cmd| {
                    cmd.arg("mcp-client-git");
                },
            ))?)
            .await?;
        clients_map.insert(idx, service);
    }

    for (_, service) in clients_map.iter() {
        // Initialize
        let _server_info = service.peer_info();

        // List tools
        let _tools = service.list_tools(Default::default()).await?;

        // Call tool 'git_status' with arguments = {"repo_path": "."}
        let _tool_result = service
            .call_tool(CallToolRequestParam {
                name: "git_status".into(),
                arguments: serde_json::json!({ "repo_path": "." }).as_object().cloned(),
            })
            .await?;
    }
    for (_, service) in clients_map {
        service.cancel().await?;
    }
    Ok(())
}

---
File: modelcontextprotocol-rust-sdk-13e72ec/examples/clients/src/everything_stdio.rs
---

use anyhow::Result;
use rmcp::{
    ServiceExt,
    model::{CallToolRequestParam, GetPromptRequestParam, ReadResourceRequestParam},
    object,
    transport::{ConfigureCommandExt, TokioChildProcess},
};
use tokio::process::Command;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| format!("info,{}=debug", env!("CARGO_CRATE_NAME")).into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    // Start server
    let service = ()
        .serve(TokioChildProcess::new(Command::new("npx").configure(
            |cmd| {
                cmd.arg("-y").arg("@modelcontextprotocol/server-everything");
            },
        ))?)
        .await?;

    // Initialize
    let server_info = service.peer_info();
    tracing::info!("Connected to server: {server_info:#?}");

    // List tools
    let tools = service.list_all_tools().await?;
    tracing::info!("Available tools: {tools:#?}");

    // Call tool echo
    let tool_result = service
        .call_tool(CallToolRequestParam {
            name: "echo".into(),
            arguments: Some(object!({ "message": "hi from rmcp" })),
        })
        .await?;
    tracing::info!("Tool result for echo: {tool_result:#?}");

    // Call tool longRunningOperation
    let tool_result = service
        .call_tool(CallToolRequestParam {
            name: "longRunningOperation".into(),
            arguments: Some(object!({ "duration": 3, "steps": 1 })),
        })
        .await?;
    tracing::info!("Tool result for longRunningOperation: {tool_result:#?}");

    // List resources
    let resources = service.list_all_resources().await?;
    tracing::info!("Available resources: {resources:#?}");

    // Read resource
    let resource = service
        .read_resource(ReadResourceRequestParam {
            uri: "test://static/resource/3".into(),
        })
        .await?;
    tracing::info!("Resource: {resource:#?}");

    // List prompts
    let prompts = service.list_all_prompts().await?;
    tracing::info!("Available prompts: {prompts:#?}");

    // Get simple prompt
    let prompt = service
        .get_prompt(GetPromptRequestParam {
            name: "simple_prompt".into(),
            arguments: None,
        })
        .await?;
    tracing::info!("Prompt - simple: {prompt:#?}");

    // Get complex prompt (returns text & image)
    let prompt = service
        .get_prompt(GetPromptRequestParam {
            name: "complex_prompt".into(),
            arguments: Some(object!({ "temperature": "0.5", "style": "formal" })),
        })
        .await?;
    tracing::info!("Prompt - complex: {prompt:#?}");

    // List resource templates
    let resource_templates = service.list_all_resource_templates().await?;
    tracing::info!("Available resource templates: {resource_templates:#?}");

    service.cancel().await?;

    Ok(())
}

---
File: modelcontextprotocol-rust-sdk-13e72ec/examples/clients/src/git_stdio.rs
---

use anyhow::Result;
use rmcp::{
    model::CallToolRequestParam,
    service::ServiceExt,
    transport::{ConfigureCommandExt, TokioChildProcess},
};
use tokio::process::Command;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| format!("info,{}=debug", env!("CARGO_CRATE_NAME")).into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();
    let service = ()
        .serve(TokioChildProcess::new(Command::new("uvx").configure(
            |cmd| {
                cmd.arg("mcp-server-git");
            },
        ))?)
        .await?;

    // or serve_client((), TokioChildProcess::new(cmd)?).await?;

    // Initialize
    let server_info = service.peer_info();
    tracing::info!("Connected to server: {server_info:#?}");

    // List tools
    let tools = service.list_tools(Default::default()).await?;
    tracing::info!("Available tools: {tools:#?}");

    // Call tool 'git_status' with arguments = {"repo_path": "."}
    let tool_result = service
        .call_tool(CallToolRequestParam {
            name: "git_status".into(),
            arguments: serde_json::json!({ "repo_path": "." }).as_object().cloned(),
        })
        .await?;
    tracing::info!("Tool result: {tool_result:#?}");
    service.cancel().await?;
    Ok(())
}

---
File: modelcontextprotocol-rust-sdk-13e72ec/examples/clients/src/sse.rs
---

use anyhow::Result;
use rmcp::{
    ServiceExt,
    model::{CallToolRequestParam, ClientCapabilities, ClientInfo, Implementation},
    transport::SseClientTransport,
};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| format!("info,{}=debug", env!("CARGO_CRATE_NAME")).into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();
    let transport = SseClientTransport::start("http://localhost:8000/sse").await?;
    let client_info = ClientInfo {
        protocol_version: Default::default(),
        capabilities: ClientCapabilities::default(),
        client_info: Implementation {
            name: "test sse client".to_string(),
            version: "0.0.1".to_string(),
        },
    };
    let client = client_info.serve(transport).await.inspect_err(|e| {
        tracing::error!("client error: {:?}", e);
    })?;

    // Initialize
    let server_info = client.peer_info();
    tracing::info!("Connected to server: {server_info:#?}");

    // List tools
    let tools = client.list_tools(Default::default()).await?;
    tracing::info!("Available tools: {tools:#?}");

    let tool_result = client
        .call_tool(CallToolRequestParam {
            name: "increment".into(),
            arguments: serde_json::json!({}).as_object().cloned(),
        })
        .await?;
    tracing::info!("Tool result: {tool_result:#?}");
    client.cancel().await?;
    Ok(())
}

---
File: modelcontextprotocol-rust-sdk-13e72ec/examples/clients/src/streamable_http.rs
---

use anyhow::Result;
use rmcp::{
    ServiceExt,
    model::{CallToolRequestParam, ClientCapabilities, ClientInfo, Implementation},
    transport::StreamableHttpClientTransport,
};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| format!("info,{}=debug", env!("CARGO_CRATE_NAME")).into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();
    let transport = StreamableHttpClientTransport::from_uri("http://localhost:8000/mcp");
    let client_info = ClientInfo {
        protocol_version: Default::default(),
        capabilities: ClientCapabilities::default(),
        client_info: Implementation {
            name: "test sse client".to_string(),
            version: "0.0.1".to_string(),
        },
    };
    let client = client_info.serve(transport).await.inspect_err(|e| {
        tracing::error!("client error: {:?}", e);
    })?;

    // Initialize
    let server_info = client.peer_info();
    tracing::info!("Connected to server: {server_info:#?}");

    // List tools
    let tools = client.list_tools(Default::default()).await?;
    tracing::info!("Available tools: {tools:#?}");

    let tool_result = client
        .call_tool(CallToolRequestParam {
            name: "increment".into(),
            arguments: serde_json::json!({}).as_object().cloned(),
        })
        .await?;
    tracing::info!("Tool result: {tool_result:#?}");
    client.cancel().await?;
    Ok(())
}

---
File: modelcontextprotocol-rust-sdk-13e72ec/examples/rig-integration/Cargo.toml
---

[package]
name = "rig-integration"
edition = { workspace = true }
version = { workspace = true }
authors = { workspace = true }
license = { workspace = true }
repository = { workspace = true }
description = { workspace = true }
keywords = { workspace = true }
homepage = { workspace = true }
categories = { workspace = true }
readme = { workspace = true }

[dependencies]
rig-core = "0.13.0"
tokio = { version = "1", features = ["full"] }
rmcp = { path = "../../crates/rmcp", features = [
    "client",
    "reqwest",
    "transport-child-process",
    "transport-sse-client",
] }
anyhow = "1.0"
serde_json = "1"
serde = { version = "1", features = ["derive"] }
toml = "0.8"
futures = "0.3"
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = [
    "env-filter",
    "std",
    "fmt",
] }
tracing-appender = "0.2"

---
File: modelcontextprotocol-rust-sdk-13e72ec/examples/rig-integration/config.toml
---

deepseek_key = ""
cohere_key = ""

[mcp]

[[mcp.server]]
name = "git"
protocol = "stdio"
command = "uvx"
args = ["mcp-server-git"]

---
File: modelcontextprotocol-rust-sdk-13e72ec/examples/rig-integration/src/chat.rs
---

use futures::StreamExt;
use rig::{
    agent::Agent,
    completion::{AssistantContent, CompletionModel},
    message::Message,
    streaming::StreamingChat,
};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader, BufWriter};

pub async fn cli_chatbot<M>(chatbot: Agent<M>) -> anyhow::Result<()>
where
    M: CompletionModel,
{
    let mut chat_log = vec![];

    let mut output = BufWriter::new(tokio::io::stdout());
    let mut input = BufReader::new(tokio::io::stdin());
    output.write_all(b"Enter :q to quit\n").await?;
    loop {
        output.write_all(b"\x1b[32muser>\x1b[0m ").await?;
        // Flush stdout to ensure the prompt appears before input
        output.flush().await?;
        let mut input_buf = String::new();
        input.read_line(&mut input_buf).await?;
        // Remove the newline character from the input
        let input = input_buf.trim();
        // Check for a command to exit
        if input == ":q" {
            break;
        }
        match chatbot.stream_chat(input, chat_log.clone()).await {
            Ok(mut response) => {
                tracing::info!(%input);
                chat_log.push(Message::user(input));
                stream_output_agent_start(&mut output).await?;
                let mut message_buf = String::new();
                while let Some(message) = response.next().await {
                    match message {
                        Ok(AssistantContent::Text(text)) => {
                            message_buf.push_str(&text.text);
                            output_agent(text.text, &mut output).await?;
                        }
                        Ok(AssistantContent::ToolCall(tool_call)) => {
                            let name = tool_call.function.name;
                            let arguments = tool_call.function.arguments;
                            chat_log.push(Message::assistant(format!(
                                "Calling tool: {name} with args: {arguments}"
                            )));
                            let result = chatbot.tools.call(&name, arguments.to_string()).await;
                            match result {
                                Ok(tool_call_result) => {
                                    stream_output_agent_finished(&mut output).await?;
                                    stream_output_toolcall(&tool_call_result, &mut output).await?;
                                    stream_output_agent_start(&mut output).await?;
                                    chat_log.push(Message::user(tool_call_result));
                                }
                                Err(e) => {
                                    output_error(e, &mut output).await?;
                                }
                            }
                        }
                        Err(error) => {
                            output_error(error, &mut output).await?;
                        }
                    }
                }
                chat_log.push(Message::assistant(message_buf));
                stream_output_agent_finished(&mut output).await?;
            }
            Err(error) => {
                output_error(error, &mut output).await?;
            }
        }
    }

    Ok(())
}

pub async fn output_error(
    e: impl std::fmt::Display,
    output: &mut BufWriter<tokio::io::Stdout>,
) -> std::io::Result<()> {
    output
        .write_all(b"\x1b[1;31m\xE2\x9D\x8C ERROR: \x1b[0m")
        .await?;
    output.write_all(e.to_string().as_bytes()).await?;
    output.write_all(b"\n").await?;
    output.flush().await?;
    Ok(())
}

pub async fn output_agent(
    content: impl std::fmt::Display,
    output: &mut BufWriter<tokio::io::Stdout>,
) -> std::io::Result<()> {
    output.write_all(content.to_string().as_bytes()).await?;
    output.flush().await?;
    Ok(())
}

pub async fn stream_output_toolcall(
    content: impl std::fmt::Display,
    output: &mut BufWriter<tokio::io::Stdout>,
) -> std::io::Result<()> {
    output
        .write_all(b"\x1b[1;33m\xF0\x9F\x9B\xA0 Tool Call: \x1b[0m")
        .await?;
    output.write_all(content.to_string().as_bytes()).await?;
    output.write_all(b"\n").await?;
    output.flush().await?;
    Ok(())
}

pub async fn stream_output_agent_start(
    output: &mut BufWriter<tokio::io::Stdout>,
) -> std::io::Result<()> {
    output
        .write_all(b"\x1b[1;34m\xF0\x9F\xA4\x96 Agent: \x1b[0m")
        .await?;
    output.flush().await?;
    Ok(())
}

pub async fn stream_output_agent_finished(
    output: &mut BufWriter<tokio::io::Stdout>,
) -> std::io::Result<()> {
    output.write_all(b"\n").await?;
    output.flush().await?;
    Ok(())
}

---
File: modelcontextprotocol-rust-sdk-13e72ec/examples/rig-integration/src/config.rs
---

use std::path::Path;

use serde::{Deserialize, Serialize};

pub mod mcp;

#[derive(Debug, Deserialize, Serialize)]
pub struct Config {
    pub mcp: mcp::McpConfig,
    pub deepseek_key: Option<String>,
    pub cohere_key: Option<String>,
}

impl Config {
    pub async fn retrieve(path: impl AsRef<Path>) -> anyhow::Result<Self> {
        let content = tokio::fs::read_to_string(path).await?;
        let config: Self = toml::from_str(&content)?;
        Ok(config)
    }
}

---
File: modelcontextprotocol-rust-sdk-13e72ec/examples/rig-integration/src/config/mcp.rs
---

use std::{collections::HashMap, process::Stdio};

use rmcp::{RoleClient, ServiceExt, service::RunningService, transport::ConfigureCommandExt};
use serde::{Deserialize, Serialize};

use crate::mcp_adaptor::McpManager;
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct McpServerConfig {
    name: String,
    #[serde(flatten)]
    transport: McpServerTransportConfig,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(tag = "protocol", rename_all = "lowercase")]
pub enum McpServerTransportConfig {
    Sse {
        url: String,
    },
    Stdio {
        command: String,
        #[serde(default)]
        args: Vec<String>,
        #[serde(default)]
        envs: HashMap<String, String>,
    },
}

#[derive(Debug, Serialize, Deserialize)]
pub struct McpConfig {
    server: Vec<McpServerConfig>,
}

impl McpConfig {
    pub async fn create_manager(&self) -> anyhow::Result<McpManager> {
        let mut clients = HashMap::new();
        let mut task_set = tokio::task::JoinSet::<anyhow::Result<_>>::new();
        for server in &self.server {
            let server = server.clone();
            task_set.spawn(async move {
                let client = server.transport.start().await?;
                anyhow::Result::Ok((server.name.clone(), client))
            });
        }
        let start_up_result = task_set.join_all().await;
        for result in start_up_result {
            match result {
                Ok((name, client)) => {
                    clients.insert(name, client);
                }
                Err(e) => {
                    eprintln!("Failed to start server: {:?}", e);
                }
            }
        }
        Ok(McpManager { clients })
    }
}

impl McpServerTransportConfig {
    pub async fn start(&self) -> anyhow::Result<RunningService<RoleClient, ()>> {
        let client = match self {
            McpServerTransportConfig::Sse { url } => {
                let transport = rmcp::transport::SseClientTransport::start(url.to_string()).await?;
                ().serve(transport).await?
            }
            McpServerTransportConfig::Stdio {
                command,
                args,
                envs,
            } => {
                let transport = rmcp::transport::TokioChildProcess::new(
                    tokio::process::Command::new(command).configure(|cmd| {
                        cmd.args(args).envs(envs).stderr(Stdio::null());
                    }),
                )?;
                ().serve(transport).await?
            }
        };
        Ok(client)
    }
}

---
File: modelcontextprotocol-rust-sdk-13e72ec/examples/rig-integration/src/main.rs
---

use rig::{
    client::{CompletionClient, ProviderClient},
    embeddings::EmbeddingsBuilder,
    providers::{cohere, deepseek},
    vector_store::in_memory_store::InMemoryVectorStore,
};
use tracing_appender::rolling::{RollingFileAppender, Rotation};
pub mod chat;
pub mod config;
pub mod mcp_adaptor;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let file_appender = RollingFileAppender::new(
        Rotation::DAILY,
        "logs",
        format!("{}.log", env!("CARGO_CRATE_NAME")),
    );
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive(tracing::Level::INFO.into()),
        )
        .with_writer(file_appender)
        .with_file(false)
        .with_ansi(false)
        .init();

    let config = config::Config::retrieve("config.toml").await?;
    let openai_client = {
        if let Some(key) = config.deepseek_key {
            deepseek::Client::new(&key)
        } else {
            deepseek::Client::from_env()
        }
    };
    let cohere_client = {
        if let Some(key) = config.cohere_key {
            cohere::Client::new(&key)
        } else {
            cohere::Client::from_env()
        }
    };
    let mcp_manager = config.mcp.create_manager().await?;
    tracing::info!(
        "MCP Manager created, {} servers started",
        mcp_manager.clients.len()
    );
    let tool_set = mcp_manager.get_tool_set().await?;
    let embedding_model =
        cohere_client.embedding_model(cohere::EMBED_MULTILINGUAL_V3, "search_document");
    let embeddings = EmbeddingsBuilder::new(embedding_model.clone())
        .documents(tool_set.schemas()?)?
        .build()
        .await?;
    let store = InMemoryVectorStore::from_documents_with_id_f(embeddings, |f| {
        tracing::info!("store tool {}", f.name);
        f.name.clone()
    });
    let index = store.index(embedding_model);
    let dpsk = openai_client
        .agent(deepseek::DEEPSEEK_CHAT)
        .dynamic_tools(4, index, tool_set)
        .build();

    chat::cli_chatbot(dpsk).await?;

    Ok(())
}

---
File: modelcontextprotocol-rust-sdk-13e72ec/examples/rig-integration/src/mcp_adaptor.rs
---

use std::collections::HashMap;

use rig::tool::{ToolDyn as RigTool, ToolEmbeddingDyn, ToolSet};
use rmcp::{
    RoleClient,
    model::{CallToolRequestParam, CallToolResult, Tool as McpTool},
    service::{RunningService, ServerSink},
};

pub struct McpToolAdaptor {
    tool: McpTool,
    server: ServerSink,
}

impl RigTool for McpToolAdaptor {
    fn name(&self) -> String {
        self.tool.name.to_string()
    }

    fn definition(
        &self,
        _prompt: String,
    ) -> std::pin::Pin<Box<dyn Future<Output = rig::completion::ToolDefinition> + Send + Sync + '_>>
    {
        Box::pin(std::future::ready(rig::completion::ToolDefinition {
            name: self.name(),
            description: self
                .tool
                .description
                .as_deref()
                .unwrap_or_default()
                .to_string(),
            parameters: self.tool.schema_as_json_value(),
        }))
    }

    fn call(
        &self,
        args: String,
    ) -> std::pin::Pin<
        Box<dyn Future<Output = Result<String, rig::tool::ToolError>> + Send + Sync + '_>,
    > {
        let server = self.server.clone();
        Box::pin(async move {
            let call_mcp_tool_result = server
                .call_tool(CallToolRequestParam {
                    name: self.tool.name.clone(),
                    arguments: serde_json::from_str(&args)
                        .map_err(rig::tool::ToolError::JsonError)?,
                })
                .await
                .inspect(|result| tracing::info!(?result))
                .inspect_err(|error| tracing::error!(%error))
                .map_err(|e| rig::tool::ToolError::ToolCallError(Box::new(e)))?;

            Ok(convert_mcp_call_tool_result_to_string(call_mcp_tool_result))
        })
    }
}

impl ToolEmbeddingDyn for McpToolAdaptor {
    fn context(&self) -> serde_json::Result<serde_json::Value> {
        serde_json::to_value(self.tool.clone())
    }

    fn embedding_docs(&self) -> Vec<String> {
        vec![
            self.tool
                .description
                .as_deref()
                .unwrap_or_default()
                .to_string(),
        ]
    }
}

pub struct McpManager {
    pub clients: HashMap<String, RunningService<RoleClient, ()>>,
}

impl McpManager {
    pub async fn get_tool_set(&self) -> anyhow::Result<ToolSet> {
        let mut tool_set = ToolSet::default();
        let mut task = tokio::task::JoinSet::<anyhow::Result<_>>::new();
        for client in self.clients.values() {
            let server = client.peer().clone();
            task.spawn(get_tool_set(server));
        }
        let results = task.join_all().await;
        for result in results {
            match result {
                Err(e) => {
                    tracing::error!(error = %e, "Failed to get tool set");
                }
                Ok(tools) => {
                    tool_set.add_tools(tools);
                }
            }
        }
        Ok(tool_set)
    }
}

pub fn convert_mcp_call_tool_result_to_string(result: CallToolResult) -> String {
    serde_json::to_string(&result).unwrap()
}

pub async fn get_tool_set(server: ServerSink) -> anyhow::Result<ToolSet> {
    let tools = server.list_all_tools().await?;
    let mut tool_builder = ToolSet::builder();
    for tool in tools {
        tracing::info!("get tool: {}", tool.name);
        let adaptor = McpToolAdaptor {
            tool: tool.clone(),
            server: server.clone(),
        };
        tool_builder = tool_builder.dynamic_tool(adaptor);
    }
    let tool_set = tool_builder.build();
    Ok(tool_set)
}

---
File: modelcontextprotocol-rust-sdk-13e72ec/examples/servers/Cargo.toml
---

[package]
name = "mcp-server-examples"
version = "0.1.5"
edition = "2024"
publish = false

[dependencies]
rmcp = { path = "../../crates/rmcp", features = [
    "server",
    "transport-sse-server",
    "transport-io",
    "transport-streamable-http-server",
    "auth",
] }
tokio = { version = "1", features = [
    "macros",
    "rt",
    "rt-multi-thread",
    "io-std",
    "signal",
] }
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
anyhow = "1.0"
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = [
    "env-filter",
    "std",
    "fmt",
] }
futures = "0.3"
rand = { version = "0.9", features = ["std"] }
axum = { version = "0.8", features = ["macros"] }
schemars = { version = "0.8", optional = true }
reqwest = { version = "0.12", features = ["json"] }
chrono = "0.4"
uuid = { version = "1.6", features = ["v4", "serde"] }
serde_urlencoded = "0.7"
askama = { version = "0.14" }
tower-http = { version = "0.6", features = ["cors"] }
hyper = { version = "1" }
hyper-util = { version = "0", features = ["server"] }

[dev-dependencies]
tokio-stream = { version = "0.1" }
tokio-util = { version = "0.7", features = ["codec"] }

[[example]]
name = "servers_counter_stdio"
path = "src/counter_stdio.rs"

[[example]]
name = "servers_counter_sse"
path = "src/counter_sse.rs"

[[example]]
name = "servers_counter_sse_directly"
path = "src/counter_sse_directly.rs"

[[example]]
name = "servers_memory_stdio"
path = "src/memory_stdio.rs"

[[example]]
name = "servers_counter_streamhttp"
path = "src/counter_streamhttp.rs"

[[example]]
name = "servers_complex_auth_sse"
path = "src/complex_auth_sse.rs"

[[example]]
name = "servers_simple_auth_sse"
path = "src/simple_auth_sse.rs"

[[example]]
name = "counter_hyper_streamable_http"
path = "src/counter_hyper_streamable_http.rs"

---
File: modelcontextprotocol-rust-sdk-13e72ec/examples/servers/README.md
---

# MCP Server Examples

This directory contains Model Context Protocol (MCP) server examples implemented in Rust. These examples demonstrate how to create MCP servers using different transport methods and how to implement various server capabilities including tools, resources, prompts, and authentication.

## Example List

### Counter Standard I/O Server (`counter_stdio.rs`)

A basic MCP server that communicates using standard input/output transport.

- Provides a simple counter tool with increment, decrement, and get_value operations
- Demonstrates basic tool implementation and stdio transport

### Counter SSE Server (`counter_sse.rs`)

A server that provides counter functionality using Server-Sent Events (SSE) transport.

- Runs on `http://127.0.0.1:8000/sse` by default
- Provides the same counter tools as the stdio version
- Demonstrates SSE transport setup with graceful shutdown
- Can be accessed via web browsers or SSE-compatible clients

### Counter SSE Direct Server (`counter_sse_directly.rs`)

A minimal SSE server implementation showing direct SSE server usage.

- Simplified version of the SSE server
- Demonstrates basic SSE server configuration
- Provides counter functionality with minimal setup

### Memory Standard I/O Server (`memory_stdio.rs`)

A minimal server example using stdio transport.

- Lightweight server implementation
- Demonstrates basic server setup patterns
- Good starting point for custom server development

### Counter Streamable HTTP Server (`counter_streamhttp.rs`)

A server using streamable HTTP transport for MCP communication, with axum.

- Runs on HTTP with streaming capabilities
- Provides counter tools via HTTP streaming
- Demonstrates streamable HTTP transport configuration

### Counter Streamable HTTP Server with Hyper (`counter_hyper_streamable_http.rs`)

A server using streamable HTTP transport for MCP communication, with hyper.
- Runs on HTTP with streaming capabilities
- Provides counter tools via HTTP streaming
- Demonstrates streamable HTTP transport configuration

### Complex OAuth SSE Server (`complex_auth_sse.rs`)

A comprehensive example demonstrating OAuth 2.0 integration with MCP servers.

- Full OAuth 2.0 authorization server implementation
- Client registration and token management
- User authorization flow with web interface
- Token validation middleware
- Integrated with MCP SSE transport
- Demonstrates enterprise-grade authentication patterns

### Simple OAuth SSE Server (`simple_auth_sse.rs`)

A simplified OAuth example showing basic token-based authentication.

- Basic token store and validation
- Authorization middleware for SSE endpoints
- Token generation API
- Simplified authentication flow
- Good starting point for adding authentication to MCP servers

## How to Run

Each example can be run using Cargo:

```bash
# Run the counter standard I/O server
cargo run --example servers_counter_stdio

# Run the counter SSE server
cargo run --example servers_counter_sse

# Run the counter SSE direct server
cargo run --example servers_counter_sse_directly

# Run the memory standard I/O server
cargo run --example servers_memory_stdio

# Run the counter streamable HTTP server
cargo run --example servers_counter_streamhttp

# Run the complex OAuth SSE server
cargo run --example servers_complex_auth_sse

# Run the simple OAuth SSE server
cargo run --example servers_simple_auth_sse
```

## Testing with MCP Inspector

Many of these servers can be tested using the MCP Inspector tool:
See [inspector](https://github.com/modelcontextprotocol/inspector)

## Dependencies

These examples use the following main dependencies:

- `rmcp`: Rust implementation of the MCP server library
- `tokio`: Asynchronous runtime
- `serde` and `serde_json`: For JSON serialization and deserialization
- `tracing` and `tracing-subscriber`: For logging
- `anyhow`: Error handling
- `axum`: Web framework for HTTP-based transports
- `tokio-util`: Utilities for async programming
- `askama`: Template engine (used in OAuth examples)
- `tower-http`: HTTP middleware (used for CORS in OAuth examples)
- `uuid`: UUID generation (used in OAuth examples)
- `chrono`: Date and time handling (used in OAuth examples)
- `rand`: Random number generation (used in OAuth examples)

## Common Module

The `common/` directory contains shared code used across examples:

- `counter.rs`: Counter tool implementation with MCP server traits
- `calculator.rs`: Calculator tool examples
- `generic_service.rs`: Generic service implementations

This modular approach allows for code reuse and demonstrates how to structure larger MCP server applications.

---
File: modelcontextprotocol-rust-sdk-13e72ec/examples/servers/src/common/calculator.rs
---

use rmcp::{
    ServerHandler,
    handler::server::wrapper::Json,
    model::{ServerCapabilities, ServerInfo},
    schemars, tool,
};

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct SumRequest {
    #[schemars(description = "the left hand side number")]
    pub a: i32,
    pub b: i32,
}
#[derive(Debug, Clone)]
pub struct Calculator;
#[tool(tool_box)]
impl Calculator {
    #[tool(description = "Calculate the sum of two numbers")]
    fn sum(&self, #[tool(aggr)] SumRequest { a, b }: SumRequest) -> String {
        (a + b).to_string()
    }

    #[tool(description = "Calculate the difference of two numbers")]
    fn sub(
        &self,
        #[tool(param)]
        #[schemars(description = "the left hand side number")]
        a: i32,
        #[tool(param)]
        #[schemars(description = "the right hand side number")]
        b: i32,
    ) -> Json<i32> {
        Json(a - b)
    }
}

#[tool(tool_box)]
impl ServerHandler for Calculator {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            instructions: Some("A simple calculator".into()),
            capabilities: ServerCapabilities::builder().enable_tools().build(),
            ..Default::default()
        }
    }
}

---
File: modelcontextprotocol-rust-sdk-13e72ec/examples/servers/src/common/counter.rs
---

use std::sync::Arc;

use rmcp::{
    Error as McpError, RoleServer, ServerHandler, const_string, model::*, schemars,
    service::RequestContext, tool,
};
use serde_json::json;
use tokio::sync::Mutex;

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct StructRequest {
    pub a: i32,
    pub b: i32,
}

#[derive(Clone)]
pub struct Counter {
    counter: Arc<Mutex<i32>>,
}

#[tool(tool_box)]
impl Counter {
    #[allow(dead_code)]
    pub fn new() -> Self {
        Self {
            counter: Arc::new(Mutex::new(0)),
        }
    }

    fn _create_resource_text(&self, uri: &str, name: &str) -> Resource {
        RawResource::new(uri, name.to_string()).no_annotation()
    }

    #[tool(description = "Increment the counter by 1")]
    async fn increment(&self) -> Result<CallToolResult, McpError> {
        let mut counter = self.counter.lock().await;
        *counter += 1;
        Ok(CallToolResult::success(vec![Content::text(
            counter.to_string(),
        )]))
    }

    #[tool(description = "Decrement the counter by 1")]
    async fn decrement(&self) -> Result<CallToolResult, McpError> {
        let mut counter = self.counter.lock().await;
        *counter -= 1;
        Ok(CallToolResult::success(vec![Content::text(
            counter.to_string(),
        )]))
    }

    #[tool(description = "Get the current counter value")]
    async fn get_value(&self) -> Result<CallToolResult, McpError> {
        let counter = self.counter.lock().await;
        Ok(CallToolResult::success(vec![Content::text(
            counter.to_string(),
        )]))
    }

    #[tool(description = "Say hello to the client")]
    fn say_hello(&self) -> Result<CallToolResult, McpError> {
        Ok(CallToolResult::success(vec![Content::text("hello")]))
    }

    #[tool(description = "Repeat what you say")]
    fn echo(
        &self,
        #[tool(param)]
        #[schemars(description = "Repeat what you say")]
        saying: String,
    ) -> Result<CallToolResult, McpError> {
        Ok(CallToolResult::success(vec![Content::text(saying)]))
    }

    #[tool(description = "Calculate the sum of two numbers")]
    fn sum(
        &self,
        #[tool(aggr)] StructRequest { a, b }: StructRequest,
    ) -> Result<CallToolResult, McpError> {
        Ok(CallToolResult::success(vec![Content::text(
            (a + b).to_string(),
        )]))
    }
}
const_string!(Echo = "echo");
#[tool(tool_box)]
impl ServerHandler for Counter {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            protocol_version: ProtocolVersion::V_2024_11_05,
            capabilities: ServerCapabilities::builder()
                .enable_prompts()
                .enable_resources()
                .enable_tools()
                .build(),
            server_info: Implementation::from_build_env(),
            instructions: Some("This server provides a counter tool that can increment and decrement values. The counter starts at 0 and can be modified using the 'increment' and 'decrement' tools. Use 'get_value' to check the current count.".to_string()),
        }
    }

    async fn list_resources(
        &self,
        _request: Option<PaginatedRequestParam>,
        _: RequestContext<RoleServer>,
    ) -> Result<ListResourcesResult, McpError> {
        Ok(ListResourcesResult {
            resources: vec![
                self._create_resource_text("str:////Users/to/some/path/", "cwd"),
                self._create_resource_text("memo://insights", "memo-name"),
            ],
            next_cursor: None,
        })
    }

    async fn read_resource(
        &self,
        ReadResourceRequestParam { uri }: ReadResourceRequestParam,
        _: RequestContext<RoleServer>,
    ) -> Result<ReadResourceResult, McpError> {
        match uri.as_str() {
            "str:////Users/to/some/path/" => {
                let cwd = "/Users/to/some/path/";
                Ok(ReadResourceResult {
                    contents: vec![ResourceContents::text(cwd, uri)],
                })
            }
            "memo://insights" => {
                let memo = "Business Intelligence Memo\n\nAnalysis has revealed 5 key insights ...";
                Ok(ReadResourceResult {
                    contents: vec![ResourceContents::text(memo, uri)],
                })
            }
            _ => Err(McpError::resource_not_found(
                "resource_not_found",
                Some(json!({
                    "uri": uri
                })),
            )),
        }
    }

    async fn list_prompts(
        &self,
        _request: Option<PaginatedRequestParam>,
        _: RequestContext<RoleServer>,
    ) -> Result<ListPromptsResult, McpError> {
        Ok(ListPromptsResult {
            next_cursor: None,
            prompts: vec![Prompt::new(
                "example_prompt",
                Some("This is an example prompt that takes one required argument, message"),
                Some(vec![PromptArgument {
                    name: "message".to_string(),
                    description: Some("A message to put in the prompt".to_string()),
                    required: Some(true),
                }]),
            )],
        })
    }

    async fn get_prompt(
        &self,
        GetPromptRequestParam { name, arguments }: GetPromptRequestParam,
        _: RequestContext<RoleServer>,
    ) -> Result<GetPromptResult, McpError> {
        match name.as_str() {
            "example_prompt" => {
                let message = arguments
                    .and_then(|json| json.get("message")?.as_str().map(|s| s.to_string()))
                    .ok_or_else(|| {
                        McpError::invalid_params("No message provided to example_prompt", None)
                    })?;

                let prompt =
                    format!("This is an example prompt with your message here: '{message}'");
                Ok(GetPromptResult {
                    description: None,
                    messages: vec![PromptMessage {
                        role: PromptMessageRole::User,
                        content: PromptMessageContent::text(prompt),
                    }],
                })
            }
            _ => Err(McpError::invalid_params("prompt not found", None)),
        }
    }

    async fn list_resource_templates(
        &self,
        _request: Option<PaginatedRequestParam>,
        _: RequestContext<RoleServer>,
    ) -> Result<ListResourceTemplatesResult, McpError> {
        Ok(ListResourceTemplatesResult {
            next_cursor: None,
            resource_templates: Vec::new(),
        })
    }

    async fn initialize(
        &self,
        _request: InitializeRequestParam,
        context: RequestContext<RoleServer>,
    ) -> Result<InitializeResult, McpError> {
        if let Some(http_request_part) = context.extensions.get::<axum::http::request::Parts>() {
            let initialize_headers = &http_request_part.headers;
            let initialize_uri = &http_request_part.uri;
            tracing::info!(?initialize_headers, %initialize_uri, "initialize from http server");
        }
        Ok(self.get_info())
    }
}

---
File: modelcontextprotocol-rust-sdk-13e72ec/examples/servers/src/common/generic_service.rs
---

use std::sync::Arc;

use rmcp::{
    ServerHandler,
    model::{ServerCapabilities, ServerInfo},
    schemars, tool,
};

#[allow(dead_code)]
pub trait DataService: Send + Sync + 'static {
    fn get_data(&self) -> String;
    fn set_data(&mut self, data: String);
}

#[derive(Debug, Clone)]
pub struct MemoryDataService {
    data: String,
}

impl MemoryDataService {
    #[allow(dead_code)]
    pub fn new(initial_data: impl Into<String>) -> Self {
        Self {
            data: initial_data.into(),
        }
    }
}

impl DataService for MemoryDataService {
    fn get_data(&self) -> String {
        self.data.clone()
    }

    fn set_data(&mut self, data: String) {
        self.data = data;
    }
}

#[derive(Debug, Clone)]
pub struct GenericService<DS: DataService> {
    #[allow(dead_code)]
    data_service: Arc<DS>,
}

#[tool(tool_box)]
impl<DS: DataService> GenericService<DS> {
    #[allow(dead_code)]
    pub fn new(data_service: DS) -> Self {
        Self {
            data_service: Arc::new(data_service),
        }
    }

    #[tool(description = "get memory from service")]
    pub async fn get_data(&self) -> String {
        self.data_service.get_data()
    }

    #[tool(description = "set memory to service")]
    pub async fn set_data(&self, #[tool(param)] data: String) -> String {
        let new_data = data.clone();
        format!("Current memory: {}", new_data)
    }
}

#[tool(tool_box)]
impl<DS: DataService> ServerHandler for GenericService<DS> {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            instructions: Some("generic data service".into()),
            capabilities: ServerCapabilities::builder().enable_tools().build(),
            ..Default::default()
        }
    }
}

---
File: modelcontextprotocol-rust-sdk-13e72ec/examples/servers/src/common/mod.rs
---

pub mod calculator;
pub mod counter;
pub mod generic_service;

---
File: modelcontextprotocol-rust-sdk-13e72ec/examples/servers/src/complex_auth_sse.rs
---

use std::{collections::HashMap, net::SocketAddr, sync::Arc, time::Duration};

use anyhow::Result;
use askama::Template;
use axum::{
    Json, Router,
    body::Body,
    extract::{Form, Query, State},
    http::{Request, StatusCode},
    middleware::{self, Next},
    response::{Html, IntoResponse, Redirect, Response},
    routing::{get, post},
};
use rand::{Rng, distr::Alphanumeric};
use rmcp::transport::{
    SseServer,
    auth::{
        AuthorizationMetadata, ClientRegistrationRequest, ClientRegistrationResponse,
        OAuthClientConfig,
    },
    sse_server::SseServerConfig,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tokio::sync::RwLock;
use tokio_util::sync::CancellationToken;
use tower_http::cors::{Any, CorsLayer};
use tracing::{debug, error, info, warn};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};
use uuid::Uuid;
// Import Counter tool for MCP service
mod common;
use common::counter::Counter;

const BIND_ADDRESS: &str = "127.0.0.1:3000";
const INDEX_HTML: &str = include_str!("html/mcp_oauth_index.html");

// A easy way to manage MCP OAuth Store for managing tokens and sessions
#[derive(Clone, Debug)]
struct McpOAuthStore {
    clients: Arc<RwLock<HashMap<String, OAuthClientConfig>>>,
    auth_sessions: Arc<RwLock<HashMap<String, AuthSession>>>,
    access_tokens: Arc<RwLock<HashMap<String, McpAccessToken>>>,
}

impl McpOAuthStore {
    fn new() -> Self {
        let mut clients = HashMap::new();
        clients.insert(
            "mcp-client".to_string(),
            OAuthClientConfig {
                client_id: "mcp-client".to_string(),
                client_secret: Some("mcp-client-secret".to_string()),
                scopes: vec!["profile".to_string(), "email".to_string()],
                redirect_uri: "http://localhost:8080/callback".to_string(),
            },
        );

        Self {
            clients: Arc::new(RwLock::new(clients)),
            auth_sessions: Arc::new(RwLock::new(HashMap::new())),
            access_tokens: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    async fn validate_client(
        &self,
        client_id: &str,
        redirect_uri: &str,
    ) -> Option<OAuthClientConfig> {
        let clients = self.clients.read().await;
        if let Some(client) = clients.get(client_id) {
            if client.redirect_uri.contains(&redirect_uri.to_string()) {
                return Some(client.clone());
            }
        }
        None
    }

    async fn create_auth_session(
        &self,
        client_id: String,
        scope: Option<String>,
        state: Option<String>,
        session_id: String,
    ) -> String {
        let session = AuthSession {
            client_id,
            scope,
            _state: state,
            _created_at: chrono::Utc::now(),
            auth_token: None,
        };

        self.auth_sessions
            .write()
            .await
            .insert(session_id.clone(), session);
        session_id
    }

    async fn update_auth_session_token(
        &self,
        session_id: &str,
        token: AuthToken,
    ) -> Result<(), String> {
        let mut sessions = self.auth_sessions.write().await;
        if let Some(session) = sessions.get_mut(session_id) {
            session.auth_token = Some(token);
            Ok(())
        } else {
            Err("Session not found".to_string())
        }
    }

    async fn create_mcp_token(&self, session_id: &str) -> Result<McpAccessToken, String> {
        let sessions = self.auth_sessions.read().await;
        if let Some(session) = sessions.get(session_id) {
            if let Some(auth_token) = &session.auth_token {
                let access_token = format!("mcp-token-{}", Uuid::new_v4());
                let token = McpAccessToken {
                    access_token: access_token.clone(),
                    token_type: "Bearer".to_string(),
                    expires_in: 3600,
                    refresh_token: format!("mcp-refresh-{}", Uuid::new_v4()),
                    scope: session.scope.clone(),
                    auth_token: auth_token.clone(),
                    client_id: session.client_id.clone(),
                };

                self.access_tokens
                    .write()
                    .await
                    .insert(access_token.clone(), token.clone());
                Ok(token)
            } else {
                Err("No third-party token available for session".to_string())
            }
        } else {
            Err("Session not found".to_string())
        }
    }

    async fn validate_token(&self, token: &str) -> Option<McpAccessToken> {
        self.access_tokens.read().await.get(token).cloned()
    }
}

// a simple session record for auth session
#[derive(Clone, Debug)]
struct AuthSession {
    client_id: String,
    scope: Option<String>,
    _state: Option<String>,
    _created_at: chrono::DateTime<chrono::Utc>,
    auth_token: Option<AuthToken>,
}

// a simple token record for auth token
// not used oauth2 token for avoid include oauth2 crate in this example
#[derive(Clone, Debug, Serialize, Deserialize)]
struct AuthToken {
    access_token: String,
    token_type: String,
    expires_in: u64,
    refresh_token: String,
    scope: Option<String>,
}

// a simple token record for mcp token ,
// not used oauth2 token for avoid include oauth2 crate in this example
#[derive(Clone, Debug, Serialize)]
struct McpAccessToken {
    access_token: String,
    token_type: String,
    expires_in: u64,
    refresh_token: String,
    scope: Option<String>,
    auth_token: AuthToken,
    client_id: String,
}

#[derive(Debug, Deserialize)]
struct AuthorizeQuery {
    #[allow(dead_code)]
    response_type: String,
    client_id: String,
    redirect_uri: String,
    scope: Option<String>,
    state: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
struct TokenRequest {
    grant_type: String,
    #[serde(default)]
    code: String,
    #[serde(default)]
    client_id: String,
    #[serde(default)]
    client_secret: String,
    #[serde(default)]
    redirect_uri: String,
    #[serde(default)]
    code_verifier: Option<String>,
    #[serde(default)]
    refresh_token: String,
}

#[derive(Debug, Deserialize, Serialize)]
struct UserInfo {
    sub: String,
    name: String,
    email: String,
    username: String,
}

fn generate_random_string(length: usize) -> String {
    rand::rng()
        .sample_iter(&Alphanumeric)
        .take(length)
        .map(char::from)
        .collect()
}

// Root path handler
async fn index() -> Html<&'static str> {
    Html(INDEX_HTML)
}

#[derive(Template)]
#[template(path = "mcp_oauth_authorize.html")]
struct OAuthAuthorizeTemplate {
    client_id: String,
    redirect_uri: String,
    scope: String,
    state: String,
    scopes: String,
}

// Initial OAuth authorize endpoint
async fn oauth_authorize(
    Query(params): Query<AuthorizeQuery>,
    State(state): State<Arc<McpOAuthStore>>,
) -> impl IntoResponse {
    debug!("doing oauth_authorize");
    if let Some(_client) = state
        .validate_client(&params.client_id, &params.redirect_uri)
        .await
    {
        let template = OAuthAuthorizeTemplate {
            client_id: params.client_id,
            redirect_uri: params.redirect_uri,
            scope: params.scope.clone().unwrap_or_default(),
            state: params.state.clone().unwrap_or_default(),
            scopes: params
                .scope
                .clone()
                .unwrap_or_else(|| "Basic scope".to_string()),
        };

        Html(template.render().unwrap()).into_response()
    } else {
        (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({
                "error": "invalid_request",
                "error_description": "invalid client id or redirect uri"
            })),
        )
            .into_response()
    }
}

// handle approval of authorization
#[derive(Debug, Deserialize)]
struct ApprovalForm {
    client_id: String,
    redirect_uri: String,
    scope: String,
    state: String,
    approved: String,
}

async fn oauth_approve(
    State(state): State<Arc<McpOAuthStore>>,
    Form(form): Form<ApprovalForm>,
) -> impl IntoResponse {
    if form.approved != "true" {
        // user rejected the authorization request
        let redirect_url = format!(
            "{}?error=access_denied&error_description={}{}",
            form.redirect_uri,
            "user rejected the authorization request",
            if form.state.is_empty() {
                "".to_string()
            } else {
                format!("&state={}", form.state)
            }
        );
        return Redirect::to(&redirect_url).into_response();
    }

    // user approved the authorization request, generate authorization code
    let session_id = Uuid::new_v4().to_string();
    let auth_code = format!("mcp-code-{}", session_id);

    // create new session record authorization information
    let session_id = state
        .create_auth_session(
            form.client_id.clone(),
            Some(form.scope.clone()),
            Some(form.state.clone()),
            session_id.clone(),
        )
        .await;

    // create token
    let created_token = AuthToken {
        access_token: format!("tp-token-{}", Uuid::new_v4()),
        token_type: "Bearer".to_string(),
        expires_in: 3600,
        refresh_token: format!("tp-refresh-{}", Uuid::new_v4()),
        scope: Some(form.scope),
    };

    // update session token
    if let Err(e) = state
        .update_auth_session_token(&session_id, created_token)
        .await
    {
        error!("Failed to update session token: {}", e);
    }

    // redirect back to client, with authorization code
    let redirect_url = format!(
        "{}?code={}{}",
        form.redirect_uri,
        auth_code,
        if form.state.is_empty() {
            "".to_string()
        } else {
            format!("&state={}", form.state)
        }
    );

    info!("authorization approved, redirecting to: {}", redirect_url);
    Redirect::to(&redirect_url).into_response()
}

// Handle token request from the MCP client
async fn oauth_token(
    State(state): State<Arc<McpOAuthStore>>,
    request: axum::http::Request<Body>,
) -> impl IntoResponse {
    info!("Received token request");

    let bytes = match axum::body::to_bytes(request.into_body(), usize::MAX).await {
        Ok(bytes) => bytes,
        Err(e) => {
            error!("can't read request body: {}", e);
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({
                    "error": "invalid_request",
                    "error_description": "can't read request body"
                })),
            )
                .into_response();
        }
    };

    let body_str = String::from_utf8_lossy(&bytes);
    info!("request body: {}", body_str);

    let token_req = match serde_urlencoded::from_bytes::<TokenRequest>(&bytes) {
        Ok(form) => {
            info!("successfully parsed form data: {:?}", form);
            form
        }
        Err(e) => {
            error!("can't parse form data: {}", e);
            return (
                StatusCode::UNPROCESSABLE_ENTITY,
                Json(serde_json::json!({
                    "error": "invalid_request",
                    "error_description": format!("can't parse form data: {}", e)
                })),
            )
                .into_response();
        }
    };
    if token_req.grant_type == "refresh_token" {
        warn!("this easy server only support authorization_code now");
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({
                "error": "unsupported_grant_type",
                "error_description": "only authorization_code is supported"
            })),
        )
            .into_response();
    }
    if token_req.grant_type != "authorization_code" {
        info!("unsupported grant type: {}", token_req.grant_type);
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({
                "error": "unsupported_grant_type",
                "error_description": "only authorization_code is supported"
            })),
        )
            .into_response();
    }

    // get session_id from code
    if !token_req.code.starts_with("mcp-code-") {
        info!("invalid authorization code: {}", token_req.code);
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({
                "error": "invalid_grant",
                "error_description": "invalid authorization code"
            })),
        )
            .into_response();
    }

    // handle empty client_id
    let client_id = if token_req.client_id.is_empty() {
        "mcp-client".to_string()
    } else {
        token_req.client_id.clone()
    };

    // validate client
    match state
        .validate_client(&client_id, &token_req.redirect_uri)
        .await
    {
        Some(_) => {
            let session_id = token_req.code.replace("mcp-code-", "");
            info!("got session id: {}", session_id);

            // create mcp access token
            match state.create_mcp_token(&session_id).await {
                Ok(token) => {
                    info!("successfully created access token");
                    (
                        StatusCode::OK,
                        Json(serde_json::json!({
                            "access_token": token.access_token,
                            "token_type": token.token_type,
                            "expires_in": token.expires_in,
                            "refresh_token": token.refresh_token,
                            "scope": token.scope,
                        })),
                    )
                        .into_response()
                }
                Err(e) => {
                    error!("failed to create access token: {}", e);
                    (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        Json(serde_json::json!({
                            "error": "server_error",
                            "error_description": format!("failed to create access token: {}", e)
                        })),
                    )
                        .into_response()
                }
            }
        }
        None => {
            info!(
                "invalid client id or redirect uri: {} / {}",
                client_id, token_req.redirect_uri
            );
            (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({
                    "error": "invalid_client",
                    "error_description": "invalid client id or redirect uri"
                })),
            )
                .into_response()
        }
    }
}

// Auth middleware for SSE connections
async fn validate_token_middleware(
    State(token_store): State<Arc<McpOAuthStore>>,
    request: Request<axum::body::Body>,
    next: Next,
) -> Response {
    debug!("validate_token_middleware");
    // Extract the access token from the Authorization header
    let auth_header = request.headers().get("Authorization");
    let token = match auth_header {
        Some(header) => {
            let header_str = header.to_str().unwrap_or("");
            if let Some(stripped) = header_str.strip_prefix("Bearer ") {
                stripped.to_string()
            } else {
                return StatusCode::UNAUTHORIZED.into_response();
            }
        }
        None => {
            return StatusCode::UNAUTHORIZED.into_response();
        }
    };

    // Validate the token
    match token_store.validate_token(&token).await {
        Some(_) => next.run(request).await,
        None => StatusCode::UNAUTHORIZED.into_response(),
    }
}

// handle oauth server metadata request
async fn oauth_authorization_server() -> impl IntoResponse {
    let mut additional_fields = HashMap::new();
    additional_fields.insert(
        "response_types_supported".into(),
        Value::Array(vec![Value::String("code".into())]),
    );
    additional_fields.insert(
        "code_challenge_methods_supported".into(),
        Value::Array(vec![Value::String("S256".into())]),
    );
    let metadata = AuthorizationMetadata {
        authorization_endpoint: format!("http://{}/oauth/authorize", BIND_ADDRESS),
        token_endpoint: format!("http://{}/oauth/token", BIND_ADDRESS),
        scopes_supported: Some(vec!["profile".to_string(), "email".to_string()]),
        registration_endpoint: format!("http://{}/oauth/register", BIND_ADDRESS),
        issuer: Some(BIND_ADDRESS.to_string()),
        jwks_uri: Some(format!("http://{}/oauth/jwks", BIND_ADDRESS)),
        additional_fields,
    };
    debug!("metadata: {:?}", metadata);
    (StatusCode::OK, Json(metadata))
}

// handle client registration request
async fn oauth_register(
    State(state): State<Arc<McpOAuthStore>>,
    Json(req): Json<ClientRegistrationRequest>,
) -> impl IntoResponse {
    debug!("register request: {:?}", req);
    if req.redirect_uris.is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({
                "error": "invalid_request",
                "error_description": "at least one redirect uri is required"
            })),
        )
            .into_response();
    }

    // generate client id and secret
    let client_id = format!("client-{}", Uuid::new_v4());
    let client_secret = generate_random_string(32);

    let client = OAuthClientConfig {
        client_id: client_id.clone(),
        client_secret: Some(client_secret.clone()),
        redirect_uri: req.redirect_uris[0].clone(),
        scopes: vec![],
    };

    state
        .clients
        .write()
        .await
        .insert(client_id.clone(), client);

    // return client information
    let response = ClientRegistrationResponse {
        client_id,
        client_secret: Some(client_secret),
        client_name: req.client_name,
        redirect_uris: req.redirect_uris,
        additional_fields: HashMap::new(),
    };

    (StatusCode::CREATED, Json(response)).into_response()
}

// Log all HTTP requests
async fn log_request(request: Request<Body>, next: Next) -> Response {
    let method = request.method().clone();
    let uri = request.uri().clone();
    let version = request.version();

    // Log headers
    let headers = request.headers().clone();
    let mut header_log = String::new();
    for (key, value) in headers.iter() {
        let value_str = value.to_str().unwrap_or("<binary>");
        header_log.push_str(&format!("\n  {}: {}", key, value_str));
    }

    // Try to get request body for form submissions
    let content_type = headers
        .get("content-type")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");

    let request_info = if content_type.contains("application/x-www-form-urlencoded")
        || content_type.contains("application/json")
    {
        format!(
            "{} {} {:?}{}\nContent-Type: {}",
            method, uri, version, header_log, content_type
        )
    } else {
        format!("{} {} {:?}{}", method, uri, version, header_log)
    };

    info!("REQUEST: {}", request_info);

    // Call the actual handler
    let response = next.run(request).await;

    // Log response status
    let status = response.status();
    info!("RESPONSE: {} for {} {}", status, method, uri);

    response
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "debug".to_string().into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    // Create the OAuth store
    let oauth_store = Arc::new(McpOAuthStore::new());

    // Set up port
    let addr = BIND_ADDRESS.parse::<SocketAddr>()?;

    // Create SSE server configuration for MCP
    let sse_config = SseServerConfig {
        bind: addr,
        sse_path: "/mcp/sse".to_string(),
        post_path: "/mcp/message".to_string(),
        ct: CancellationToken::new(),
        sse_keep_alive: Some(Duration::from_secs(15)),
    };

    // Create SSE server
    let (sse_server, sse_router) = SseServer::new(sse_config);

    // Create protected SSE routes (require authorization)
    let protected_sse_router = sse_router.layer(middleware::from_fn_with_state(
        oauth_store.clone(),
        validate_token_middleware,
    ));

    // Create CORS layer for the oauth authorization server endpoint
    let cors_layer = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    // Create a sub-router for the oauth authorization server endpoint with CORS
    let oauth_server_router = Router::new()
        .route(
            "/.well-known/oauth-authorization-server",
            get(oauth_authorization_server).options(oauth_authorization_server),
        )
        .route("/oauth/token", post(oauth_token).options(oauth_token))
        .route(
            "/oauth/register",
            post(oauth_register).options(oauth_register),
        )
        .layer(cors_layer)
        .with_state(oauth_store.clone());

    // Create HTTP router with request logging middleware
    let app = Router::new()
        .route("/", get(index))
        .route("/mcp", get(index))
        .route("/oauth/authorize", get(oauth_authorize))
        .route("/oauth/approve", post(oauth_approve))
        .merge(oauth_server_router) // Merge the CORS-enabled oauth server router
        // .merge(protected_sse_router)
        .with_state(oauth_store.clone())
        .layer(middleware::from_fn(log_request));

    let app = app.merge(protected_sse_router);
    // Register token validation middleware for SSE
    let cancel_token = sse_server.config.ct.clone();
    // Handle Ctrl+C
    let cancel_token2 = sse_server.config.ct.clone();
    // Start SSE server with Counter service
    sse_server.with_service(Counter::new);

    // Start HTTP server
    info!("MCP OAuth Server started on {}", addr);
    let listener = tokio::net::TcpListener::bind(addr).await?;
    let server = axum::serve(listener, app).with_graceful_shutdown(async move {
        cancel_token.cancelled().await;
        info!("Server is shutting down");
    });

    tokio::spawn(async move {
        match tokio::signal::ctrl_c().await {
            Ok(()) => {
                info!("Received Ctrl+C, shutting down");
                cancel_token2.cancel();
            }
            Err(e) => error!("Failed to listen for Ctrl+C: {}", e),
        }
    });

    if let Err(e) = server.await {
        error!("Server error: {}", e);
    }

    Ok(())
}

---
File: modelcontextprotocol-rust-sdk-13e72ec/examples/servers/src/counter_hyper_streamable_http.rs
---

mod common;
use common::counter::Counter;
use hyper_util::{
    rt::{TokioExecutor, TokioIo},
    server::conn::auto::Builder,
    service::TowerToHyperService,
};
use rmcp::transport::streamable_http_server::{
    StreamableHttpService, session::local::LocalSessionManager,
};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let service = TowerToHyperService::new(StreamableHttpService::new(
        || Ok(Counter::new()),
        LocalSessionManager::default().into(),
        Default::default(),
    ));
    let listener = tokio::net::TcpListener::bind("[::1]:8080").await?;
    loop {
        let io = tokio::select! {
            _ = tokio::signal::ctrl_c() => break,
            accept = listener.accept() => {
                TokioIo::new(accept?.0)
            }
        };
        let service = service.clone();
        tokio::spawn(async move {
            let _result = Builder::new(TokioExecutor::default())
                .serve_connection(io, service)
                .await;
        });
    }
    Ok(())
}

---
File: modelcontextprotocol-rust-sdk-13e72ec/examples/servers/src/counter_sse.rs
---

use rmcp::transport::sse_server::{SseServer, SseServerConfig};
use tracing_subscriber::{
    layer::SubscriberExt,
    util::SubscriberInitExt,
    {self},
};
mod common;
use common::counter::Counter;

const BIND_ADDRESS: &str = "127.0.0.1:8000";

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "debug".to_string().into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    let config = SseServerConfig {
        bind: BIND_ADDRESS.parse()?,
        sse_path: "/sse".to_string(),
        post_path: "/message".to_string(),
        ct: tokio_util::sync::CancellationToken::new(),
        sse_keep_alive: None,
    };

    let (sse_server, router) = SseServer::new(config);

    // Do something with the router, e.g., add routes or middleware

    let listener = tokio::net::TcpListener::bind(sse_server.config.bind).await?;

    let ct = sse_server.config.ct.child_token();

    let server = axum::serve(listener, router).with_graceful_shutdown(async move {
        ct.cancelled().await;
        tracing::info!("sse server cancelled");
    });

    tokio::spawn(async move {
        if let Err(e) = server.await {
            tracing::error!(error = %e, "sse server shutdown with error");
        }
    });

    let ct = sse_server.with_service(Counter::new);

    tokio::signal::ctrl_c().await?;
    ct.cancel();
    Ok(())
}

---
File: modelcontextprotocol-rust-sdk-13e72ec/examples/servers/src/counter_sse_directly.rs
---

use rmcp::transport::sse_server::SseServer;
use tracing_subscriber::{
    layer::SubscriberExt,
    util::SubscriberInitExt,
    {self},
};
mod common;
use common::counter::Counter;

const BIND_ADDRESS: &str = "127.0.0.1:8000";

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "debug".to_string().into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    let ct = SseServer::serve(BIND_ADDRESS.parse()?)
        .await?
        .with_service_directly(Counter::new);

    tokio::signal::ctrl_c().await?;
    ct.cancel();
    Ok(())
}

---
File: modelcontextprotocol-rust-sdk-13e72ec/examples/servers/src/counter_stdio.rs
---

use anyhow::Result;
use common::counter::Counter;
use rmcp::{ServiceExt, transport::stdio};
use tracing_subscriber::{self, EnvFilter};
mod common;
/// npx @modelcontextprotocol/inspector cargo run -p mcp-server-examples --example std_io
#[tokio::main]
async fn main() -> Result<()> {
    // Initialize the tracing subscriber with file and stdout logging
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env().add_directive(tracing::Level::DEBUG.into()))
        .with_writer(std::io::stderr)
        .with_ansi(false)
        .init();

    tracing::info!("Starting MCP server");

    // Create an instance of our counter router
    let service = Counter::new().serve(stdio()).await.inspect_err(|e| {
        tracing::error!("serving error: {:?}", e);
    })?;

    service.waiting().await?;
    Ok(())
}

---
File: modelcontextprotocol-rust-sdk-13e72ec/examples/servers/src/counter_streamhttp.rs
---

use rmcp::transport::streamable_http_server::{
    StreamableHttpService, session::local::LocalSessionManager,
};
use tracing_subscriber::{
    layer::SubscriberExt,
    util::SubscriberInitExt,
    {self},
};
mod common;
use common::counter::Counter;

const BIND_ADDRESS: &str = "127.0.0.1:8000";

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "debug".to_string().into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    let service = StreamableHttpService::new(
        || Ok(Counter::new()),
        LocalSessionManager::default().into(),
        Default::default(),
    );

    let router = axum::Router::new().nest_service("/mcp", service);
    let tcp_listener = tokio::net::TcpListener::bind(BIND_ADDRESS).await?;
    let _ = axum::serve(tcp_listener, router)
        .with_graceful_shutdown(async { tokio::signal::ctrl_c().await.unwrap() })
        .await;
    Ok(())
}

---
File: modelcontextprotocol-rust-sdk-13e72ec/examples/servers/src/html/mcp_oauth_index.html
---

<!DOCTYPE html>
<html>
<head>
    <title>MCP OAuth Server</title>
    <style>
        body { font-family: Arial, sans-serif; margin: 40px auto; max-width: 800px; line-height: 1.6; }
        h1, h2 { color: #333; }
        code { background: #f4f4f4; padding: 2px 5px; border-radius: 3px; }
        .endpoint { background: #f9f9f9; padding: 15px; border-radius: 5px; margin-bottom: 15px; }
        .flow { background: #e8f5e9; padding: 15px; border-radius: 5px; margin-bottom: 15px; }
    </style>
</head>
<body>
    <h1>MCP OAuth Server</h1>
    <p>This is an MCP server with OAuth 2.0 integration to a third-party authorization server.</p>
    <h2>Available Endpoints:</h2>
    <div class="endpoint">
        <h3>Authorization Endpoint</h3>
        <p><code>GET /oauth/authorize</code></p>
        <p>Parameters:</p>
        <ul>
            <li><code>response_type</code> - Must be "code"</li>
            <li><code>client_id</code> - Client identifier (e.g., "mcp-client")</li>
            <li><code>redirect_uri</code> - URI to redirect after authorization</li>
            <li><code>scope</code> - Optional requested scope</li>
            <li><code>state</code> - Optional state value for CSRF prevention</li>
        </ul>
    </div>
    <div class="endpoint">
        <h3>Token Endpoint</h3>
        <p><code>POST /oauth/token</code></p>
        <p>Parameters:</p>
        <ul>
            <li><code>grant_type</code> - Must be "authorization_code"</li>
            <li><code>code</code> - The authorization code</li>
            <li><code>client_id</code> - Client identifier</li>
            <li><code>client_secret</code> - Client secret</li>
            <li><code>redirect_uri</code> - Redirect URI used in authorization request</li>
        </ul>
    </div>
    <div class="endpoint">
        <h3>MCP SSE Endpoints</h3>
        <p><code>/mcp/sse</code> - SSE connection endpoint (requires OAuth token)</p>
        <p><code>/mcp/message</code> - Message endpoint (requires OAuth token)</p>
    </div>
    <div class="flow">
        <h2>OAuth Flow:</h2>
        <ol>
            <li>MCP Client initiates OAuth flow with this MCP Server</li>
            <li>MCP Server redirects to Third-Party OAuth Server</li>
            <li>User authenticates with Third-Party Server</li>
            <li>Third-Party Server redirects back to MCP Server with auth code</li>
            <li>MCP Server exchanges the code for a third-party access token</li>
            <li>MCP Server generates its own token bound to the third-party session</li>
            <li>MCP Server completes the OAuth flow with the MCP Client</li>
        </ol>
    </div>
</body>
</html>

---
File: modelcontextprotocol-rust-sdk-13e72ec/examples/servers/src/html/sse_auth_index.html
---

<!DOCTYPE html>
<html>
<head>
    <title>RMCP Authorized SSE Server</title>
    <style>
        body { font-family: Arial, sans-serif; margin: 40px; line-height: 1.6; }
        h1 { color: #333; }
        .endpoint { background: #f4f4f4; padding: 10px; border-radius: 5px; }
        pre { background: #eee; padding: 10px; border-radius: 5px; }
    </style>
</head>
<body>
    <h1>RMCP Authorized SSE Server</h1>
    <p>This is a Server-Sent Events server example that requires OAuth authorization.</p>
    <h2>Available Endpoints:</h2>
    <ul>
        <li class="endpoint"><code>/api/health</code> - Health check</li>
        <li class="endpoint"><code>/api/token/{token_id}</code> - Get test token (available: demo, test)</li>
        <li class="endpoint"><code>/sse</code> - SSE connection endpoint (requires authorization)</li>
        <li class="endpoint"><code>/message</code> - Message sending endpoint (requires authorization)</li>
    </ul>
    <h2>Usage:</h2>
    <pre>
        # Get a token
        curl http://127.0.0.1:8000/api/token/demo

        # Connect to SSE using the token
        curl -H "Authorization: Bearer demo-token" http://127.0.0.1:8000/sse
    </pre>
</body>
</html>

---
File: modelcontextprotocol-rust-sdk-13e72ec/examples/servers/src/memory_stdio.rs
---

use std::error::Error;
mod common;
use common::generic_service::{GenericService, MemoryDataService};
use rmcp::serve_server;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let memory_service = MemoryDataService::new("initial data");

    let generic_service = GenericService::new(memory_service);

    println!("start server, connect to standard input/output");

    let io = (tokio::io::stdin(), tokio::io::stdout());

    serve_server(generic_service, io).await?;
    Ok(())
}

---
File: modelcontextprotocol-rust-sdk-13e72ec/examples/servers/src/simple_auth_sse.rs
---

/// This example shows how to use the RMCP SSE server with OAuth authorization.
/// Use the inspector to view this server https://github.com/modelcontextprotocol/inspector
/// The default index page is available at http://127.0.0.1:8000/
/// # Get a token
/// curl http://127.0.0.1:8000/api/token/demo
/// # Connect to SSE using the token
/// curl -H "Authorization: Bearer demo-token" http://127.0.0.1:8000/sse
use std::{net::SocketAddr, sync::Arc, time::Duration};

use anyhow::Result;
use axum::{
    Json, Router,
    extract::{Path, State},
    http::{HeaderMap, Request, StatusCode},
    middleware::{self, Next},
    response::{Html, Response},
    routing::get,
};
use rmcp::transport::{SseServer, sse_server::SseServerConfig};
use tokio_util::sync::CancellationToken;
mod common;
use common::counter::Counter;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

const BIND_ADDRESS: &str = "127.0.0.1:8000";
const INDEX_HTML: &str = include_str!("html/sse_auth_index.html");
// A simple token store
struct TokenStore {
    valid_tokens: Vec<String>,
}

impl TokenStore {
    fn new() -> Self {
        // For demonstration purposes, use more secure token management in production
        Self {
            valid_tokens: vec!["demo-token".to_string(), "test-token".to_string()],
        }
    }

    fn is_valid(&self, token: &str) -> bool {
        self.valid_tokens.contains(&token.to_string())
    }
}

// Extract authorization token
fn extract_token(headers: &HeaderMap) -> Option<String> {
    headers
        .get("Authorization")
        .and_then(|value| value.to_str().ok())
        .and_then(|auth_header| {
            auth_header
                .strip_prefix("Bearer ")
                .map(|stripped| stripped.to_string())
        })
}

// Authorization middleware
async fn auth_middleware(
    State(token_store): State<Arc<TokenStore>>,
    headers: HeaderMap,
    request: Request<axum::body::Body>,
    next: Next,
) -> Result<Response, StatusCode> {
    match extract_token(&headers) {
        Some(token) if token_store.is_valid(&token) => {
            // Token is valid, proceed with the request
            Ok(next.run(request).await)
        }
        _ => {
            // Token is invalid, return 401 error
            Err(StatusCode::UNAUTHORIZED)
        }
    }
}

// Root path handler
async fn index() -> Html<&'static str> {
    Html(INDEX_HTML)
}

// Health check endpoint
async fn health_check() -> &'static str {
    "OK"
}

// Token generation endpoint (simplified example)
async fn get_token(Path(token_id): Path<String>) -> Result<Json<serde_json::Value>, StatusCode> {
    // In a real application, you should authenticate the user and generate a real token
    if token_id == "demo" || token_id == "test" {
        let token = format!("{}-token", token_id);
        Ok(Json(serde_json::json!({
            "access_token": token,
            "token_type": "Bearer",
            "expires_in": 3600
        })))
    } else {
        Err(StatusCode::UNAUTHORIZED)
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "debug".to_string().into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    // Create token store
    let token_store = Arc::new(TokenStore::new());

    // Set up port
    let addr = BIND_ADDRESS.parse::<SocketAddr>()?;

    // Create SSE server configuration
    let sse_config = SseServerConfig {
        bind: addr,
        sse_path: "/sse".to_string(),
        post_path: "/message".to_string(),
        ct: CancellationToken::new(),
        sse_keep_alive: Some(Duration::from_secs(15)),
    };

    // Create SSE server
    let (sse_server, sse_router) = SseServer::new(sse_config);

    // Create API routes
    let api_routes = Router::new()
        .route("/health", get(health_check))
        .route("/token/{token_id}", get(get_token));

    // Create protected SSE routes (require authorization)
    let protected_sse_router = sse_router.layer(middleware::from_fn_with_state(
        token_store.clone(),
        auth_middleware,
    ));

    // Create main router, public endpoints don't require authorization
    let app = Router::new()
        .route("/", get(index))
        .nest("/api", api_routes)
        .merge(protected_sse_router)
        .with_state(());

    // Start server and register service
    let listener = tokio::net::TcpListener::bind(addr).await?;
    let ct = sse_server.config.ct.clone();

    // Start SSE server with Counter service
    sse_server.with_service(Counter::new);

    // Handle signals for graceful shutdown
    let cancel_token = ct.clone();
    tokio::spawn(async move {
        match tokio::signal::ctrl_c().await {
            Ok(()) => {
                println!("Received Ctrl+C, shutting down server...");
                cancel_token.cancel();
            }
            Err(err) => {
                eprintln!("Unable to listen for Ctrl+C signal: {}", err);
            }
        }
    });

    // Start HTTP server
    tracing::info!("Server started on {}", addr);
    let server = axum::serve(listener, app).with_graceful_shutdown(async move {
        // Wait for cancellation signal
        ct.cancelled().await;
        println!("Server is shutting down...");
    });

    if let Err(e) = server.await {
        eprintln!("Server error: {}", e);
    }

    println!("Server has been shut down");
    Ok(())
}

---
File: modelcontextprotocol-rust-sdk-13e72ec/examples/servers/templates/mcp_oauth_authorize.html
---

<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>MCP OAuth</title>
    <style>
        :root {
            --primary-color: #4285f4;
            --secondary-color: #f1f1f1;
            --text-color: #333;
            --border-color: #ddd;
        }

        body {
            font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, "Helvetica Neue", Arial, sans-serif;
            margin: 0;
            padding: 0;
            min-height: 100vh;
            display: flex;
            align-items: center;
            justify-content: center;
            background-color: #f8f9fa;
            color: var(--text-color);
        }

        .container {
            background: white;
            padding: 2rem;
            border-radius: 12px;
            box-shadow: 0 4px 6px rgba(0, 0, 0, 0.1);
            max-width: 600px;
            width: 90%;
            margin: 1rem;
        }

        h1 {
            margin: 0 0 1.5rem 0;
            font-size: 1.8rem;
            text-align: center;
        }

        .client-info {
            background: var(--secondary-color);
            padding: 1rem;
            border-radius: 8px;
            margin-bottom: 1.5rem;
        }

        .btn-group {
            display: flex;
            gap: 1rem;
            justify-content: center;
        }

        .btn {
            padding: 0.75rem 1.5rem;
            border-radius: 6px;
            cursor: pointer;
            font-size: 1rem;
            transition: all 0.2s;
            border: none;
        }

        .btn-primary {
            background-color: var(--primary-color);
            color: white;
        }

        .btn-primary:hover {
            background-color: #3367d6;
        }

        .btn-secondary {
            background-color: var(--secondary-color);
            color: var(--text-color);
            border: 1px solid var(--border-color);
        }

        .btn-secondary:hover {
            background-color: #e0e0e0;
        }
    </style>
</head>
<body>
    <div class="container">
        <h1>MCP OAuth</h1>
        <div class="client-info">
            <p><strong>{{ client_id }}</strong> requests access to your account.</p>
            <p>requested scopes: {{ scopes }}</p>
        </div>
        <form action="/oauth/approve" method="post">
            <input type="hidden" name="client_id" value="{{ client_id }}">
            <input type="hidden" name="redirect_uri" value="{{ redirect_uri }}">
            <input type="hidden" name="scope" value="{{ scope }}">
            <input type="hidden" name="state" value="{{ state }}">
            <div class="btn-group">
                <button type="submit" name="approved" value="true" class="btn btn-primary">Approve</button>
                <button type="submit" name="approved" value="false" class="btn btn-secondary">Reject</button>
            </div>
        </form>
    </div>
</body>
</html>

---
File: modelcontextprotocol-rust-sdk-13e72ec/examples/simple-chat-client/Cargo.toml
---

[package]
name = "simple-chat-client"
version = "0.1.0"
edition = "2021"

[dependencies]
tokio = { version = "1", features = ["full"] }
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
reqwest = { version = "0.12", features = ["json"] }
anyhow = "1.0"
thiserror = "2.0"
async-trait = "0.1"
futures = "0.3"
toml = "0.8"
rmcp = { workspace = true, features = [
    "client",
    "transport-child-process",
    "transport-sse-client",
    "reqwest"
], no-default-features = true }
clap = { version = "4.0", features = ["derive"] }

---
File: modelcontextprotocol-rust-sdk-13e72ec/examples/simple-chat-client/README.md
---

# Simple Chat Client

A simple chat client implementation using the Model Context Protocol (MCP) SDK. It just a example for developers to understand how to use the MCP SDK. This example use the easiest way to start a MCP server, and call the tool directly. No need embedding or complex third library or function call(because some models can't support function call).Just add tool in system prompt, and the client will call the tool automatically.


## Usage

After configuring the config file, you can run the example:
```bash
./simple_chat --help                                                       # show help info
./simple_chat config > config.toml                                         # output default config to file
./simple_chat --config my_config.toml chat                                 # start chat with specified config
./simple_chat --config my_config.toml --model gpt-4o-mini chat             # start chat with specified model
```

---
File: modelcontextprotocol-rust-sdk-13e72ec/examples/simple-chat-client/src/bin/simple_chat.rs
---

use std::{process::exit, sync::Arc};

use anyhow::Result;
use clap::{Parser, Subcommand};
use simple_chat_client::{
    chat::ChatSession,
    client::OpenAIClient,
    config::Config,
    tool::{Tool, ToolSet, get_mcp_tools},
};

#[derive(Parser)]
#[command(author, version, about = "Simple Chat Client")]
struct Cli {
    /// Config file path
    #[arg(short, long, value_name = "FILE")]
    config: Option<String>,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Output default config template
    Config,

    /// Start chat
    Chat {
        /// Specify the model name
        #[arg(short, long)]
        model: Option<String>,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Config => {
            println!("{}", include_str!("../config.toml"));
            return Ok(());
        }
        Commands::Chat { model } => {
            // load config
            let config_path = cli.config;
            let mut config = match config_path {
                Some(path) => Config::load(&path).await?,
                None => {
                    println!("No config file provided, using default config");
                    exit(-1);
                }
            };

            // if command line specify model, override config file setting
            if let Some(model_name) = model {
                config.model_name = Some(model_name);
            }

            // create openai client
            let api_key = config
                .openai_key
                .clone()
                .unwrap_or_else(|| std::env::var("OPENAI_API_KEY").expect("need set api key"));
            let url = config.chat_url.clone();
            println!("use api address: {:?}", url);
            let openai_client = Arc::new(OpenAIClient::new(api_key, url, config.proxy));

            // create tool set
            let mut tool_set = ToolSet::default();

            // load MCP
            if config.mcp.is_some() {
                let mcp_clients = config.create_mcp_clients().await?;

                for (name, client) in mcp_clients {
                    println!("load MCP tool: {}", name);
                    let server = client.peer().clone();
                    let tools = get_mcp_tools(server).await?;

                    for tool in tools {
                        println!("add tool: {}", tool.name());
                        tool_set.add_tool(tool);
                    }
                }
            }

            // create chat session
            let mut session = ChatSession::new(
                openai_client,
                tool_set,
                config
                    .model_name
                    .unwrap_or_else(|| "gpt-4o-mini".to_string()),
            );

            let support_tool = config.support_tool.unwrap_or(true);
            let mut system_prompt;
            // if not support tool call, add tool call format guidance
            if !support_tool {
                // build system prompt
                system_prompt =
            "you are a assistant, you can help user to complete various tasks. you have the following tools to use:\n".to_string();

                // add tool info to system prompt
                for tool in session.get_tools() {
                    system_prompt.push_str(&format!(
                        "\ntool name: {}\ndescription: {}\nparameters: {}\n",
                        tool.name(),
                        tool.description(),
                        serde_json::to_string_pretty(&tool.parameters())
                            .expect("failed to serialize tool parameters")
                    ));
                }

                // add tool call format guidance
                system_prompt.push_str(
                    "\nif you need to call tool, please use the following format:\n\
            Tool: <tool name>\n\
            Inputs: <inputs>\n",
                );
                println!("system prompt: {}", system_prompt);
            } else {
                system_prompt =
                    "you are a assistant, you can help user to complete various tasks.".to_string();
            }

            // add system prompt
            session.add_system_prompt(system_prompt);

            // start chat
            session.chat(support_tool).await?;
        }
    }

    Ok(())
}

---
File: modelcontextprotocol-rust-sdk-13e72ec/examples/simple-chat-client/src/chat.rs
---

use std::{
    io::{self, Write},
    sync::Arc,
};

use anyhow::Result;
use serde_json;

use crate::{
    client::ChatClient,
    model::{CompletionRequest, Message, ToolFunction},
    tool::{Tool as ToolTrait, ToolSet},
};

pub struct ChatSession {
    client: Arc<dyn ChatClient>,
    tool_set: ToolSet,
    model: String,
    messages: Vec<Message>,
}

impl ChatSession {
    pub fn new(client: Arc<dyn ChatClient>, tool_set: ToolSet, model: String) -> Self {
        Self {
            client,
            tool_set,
            model,
            messages: Vec::new(),
        }
    }

    pub fn add_system_prompt(&mut self, prompt: impl ToString) {
        self.messages.push(Message::system(prompt));
    }

    pub fn get_tools(&self) -> Vec<Arc<dyn ToolTrait>> {
        self.tool_set.tools()
    }

    pub async fn analyze_tool_call(&mut self, response: &Message) {
        let mut tool_calls_func = Vec::new();
        if let Some(tool_calls) = response.tool_calls.as_ref() {
            for tool_call in tool_calls {
                if tool_call._type == "function" {
                    tool_calls_func.push(tool_call.function.clone());
                }
            }
        } else {
            // check if message contains tool call
            if response.content.contains("Tool:") {
                let lines: Vec<&str> = response.content.split('\n').collect();
                // simple parse tool call
                let mut tool_name = None;
                let mut args_text = Vec::new();
                let mut parsing_args = false;

                for line in lines {
                    if line.starts_with("Tool:") {
                        tool_name = line.strip_prefix("Tool:").map(|s| s.trim().to_string());
                        parsing_args = false;
                    } else if line.starts_with("Inputs:") {
                        parsing_args = true;
                    } else if parsing_args {
                        args_text.push(line.trim());
                    }
                }
                if let Some(name) = tool_name {
                    tool_calls_func.push(ToolFunction {
                        name,
                        arguments: args_text.join("\n"),
                    });
                }
            }
        }
        // call tool
        for tool_call in tool_calls_func {
            println!("tool call: {:?}", tool_call);
            let tool = self.tool_set.get_tool(&tool_call.name);
            if let Some(tool) = tool {
                // call tool
                let args = serde_json::from_str::<serde_json::Value>(&tool_call.arguments)
                    .unwrap_or_default();
                match tool.call(args).await {
                    Ok(result) => {
                        if result.is_error.is_some_and(|b| b) {
                            self.messages
                                .push(Message::user("tool call failed, mcp call error"));
                        } else {
                            result.content.iter().for_each(|content| {
                                if let Some(content_text) = content.as_text() {
                                    let json_result = serde_json::from_str::<serde_json::Value>(
                                        &content_text.text,
                                    )
                                    .unwrap_or_default();
                                    let pretty_result =
                                        serde_json::to_string_pretty(&json_result).unwrap();
                                    println!("call tool result: {}", pretty_result);
                                    self.messages.push(Message::user(format!(
                                        "call tool result: {}",
                                        pretty_result
                                    )));
                                }
                            });
                        }
                    }
                    Err(e) => {
                        println!("tool call failed: {}", e);
                        self.messages
                            .push(Message::user(format!("tool call failed: {}", e)));
                    }
                }
            } else {
                println!("tool not found: {}", tool_call.name);
            }
        }
    }
    pub async fn chat(&mut self, support_tool: bool) -> Result<()> {
        println!("welcome to use simple chat client, use 'exit' to quit");

        loop {
            print!("> ");
            io::stdout().flush()?;

            let mut input = String::new();
            io::stdin().read_line(&mut input)?;
            input = input.trim().to_string();

            if input.is_empty() {
                continue;
            }

            if input == "exit" {
                break;
            }

            self.messages.push(Message::user(&input));
            let tool_definitions = if support_tool {
                // prepare tool list
                let tools = self.tool_set.tools();
                if !tools.is_empty() {
                    Some(
                        tools
                            .iter()
                            .map(|tool| crate::model::Tool {
                                name: tool.name(),
                                description: tool.description(),
                                parameters: tool.parameters(),
                            })
                            .collect(),
                    )
                } else {
                    None
                }
            } else {
                None
            };

            // create request
            let request = CompletionRequest {
                model: self.model.clone(),
                messages: self.messages.clone(),
                temperature: Some(0.7),
                tools: tool_definitions,
            };

            // send request
            let response = self.client.complete(request).await?;
            // get choice
            let choice = response.choices.first().unwrap();
            println!("AI > {}", choice.message.content);
            // analyze tool call
            self.analyze_tool_call(&choice.message).await;
        }

        Ok(())
    }
}

---
File: modelcontextprotocol-rust-sdk-13e72ec/examples/simple-chat-client/src/client.rs
---

use anyhow::Result;
use async_trait::async_trait;
use reqwest::Client as HttpClient;

use crate::model::{CompletionRequest, CompletionResponse};

#[async_trait]
pub trait ChatClient: Send + Sync {
    async fn complete(&self, request: CompletionRequest) -> Result<CompletionResponse>;
}

pub struct OpenAIClient {
    api_key: String,
    client: HttpClient,
    base_url: String,
}

impl OpenAIClient {
    pub fn new(api_key: String, url: Option<String>, proxy: Option<bool>) -> Self {
        let base_url = url.unwrap_or("https://api.openai.com/v1/chat/completions".to_string());
        let proxy = proxy.unwrap_or(false);
        let client = if proxy {
            HttpClient::new()
        } else {
            HttpClient::builder()
                .no_proxy()
                .build()
                .unwrap_or_else(|_| HttpClient::new())
        };

        Self {
            api_key,
            client,
            base_url,
        }
    }

    pub fn with_base_url(mut self, base_url: impl Into<String>) -> Self {
        self.base_url = base_url.into();
        self
    }
}

#[async_trait]
impl ChatClient for OpenAIClient {
    async fn complete(&self, request: CompletionRequest) -> Result<CompletionResponse> {
        let response = self
            .client
            .post(&self.base_url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .await?;

        if !response.status().is_success() {
            let error_text = response.text().await?;
            println!("API error: {}", error_text);
            return Err(anyhow::anyhow!("API Error: {}", error_text));
        }
        let text_data = response.text().await?;
        println!("Received response: {}", text_data);
        let completion: CompletionResponse = serde_json::from_str(&text_data)
            .map_err(anyhow::Error::from)
            .unwrap();
        Ok(completion)
    }
}

---
File: modelcontextprotocol-rust-sdk-13e72ec/examples/simple-chat-client/src/config.rs
---

use std::{collections::HashMap, path::Path, process::Stdio};

use anyhow::Result;
use rmcp::{RoleClient, ServiceExt, service::RunningService, transport::ConfigureCommandExt};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct Config {
    pub openai_key: Option<String>,
    pub chat_url: Option<String>,
    pub mcp: Option<McpConfig>,
    pub model_name: Option<String>,
    pub proxy: Option<bool>,
    pub support_tool: Option<bool>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct McpConfig {
    pub server: Vec<McpServerConfig>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct McpServerConfig {
    pub name: String,
    #[serde(flatten)]
    pub transport: McpServerTransportConfig,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(tag = "protocol", rename_all = "lowercase")]
pub enum McpServerTransportConfig {
    Sse {
        url: String,
    },
    Stdio {
        command: String,
        #[serde(default)]
        args: Vec<String>,
        #[serde(default)]
        envs: HashMap<String, String>,
    },
}

impl McpServerTransportConfig {
    pub async fn start(&self) -> Result<RunningService<RoleClient, ()>> {
        let client = match self {
            McpServerTransportConfig::Sse { url } => {
                let transport =
                    rmcp::transport::sse_client::SseClientTransport::start(url.to_owned()).await?;
                ().serve(transport).await?
            }
            McpServerTransportConfig::Stdio {
                command,
                args,
                envs,
            } => {
                let transport = rmcp::transport::child_process::TokioChildProcess::new(
                    tokio::process::Command::new(command).configure(|cmd| {
                        cmd.args(args)
                            .envs(envs)
                            .stderr(Stdio::inherit())
                            .stdout(Stdio::inherit());
                    }),
                )?;
                ().serve(transport).await?
            }
        };
        Ok(client)
    }
}

impl Config {
    pub async fn load(path: impl AsRef<Path>) -> Result<Self> {
        let content = tokio::fs::read_to_string(path).await?;
        let config: Self = toml::from_str(&content)?;
        Ok(config)
    }

    pub async fn create_mcp_clients(
        &self,
    ) -> Result<HashMap<String, RunningService<RoleClient, ()>>> {
        let mut clients = HashMap::new();

        if let Some(mcp_config) = &self.mcp {
            for server in &mcp_config.server {
                let client = server.transport.start().await?;
                clients.insert(server.name.clone(), client);
            }
        }

        Ok(clients)
    }
}

---
File: modelcontextprotocol-rust-sdk-13e72ec/examples/simple-chat-client/src/config.toml
---

openai_key = "key"
chat_url = "url"
model_name = "model_name"
proxy = false
support_tool = true # if support tool call

[mcp]
[[mcp.server]]
name = "MCP server name"
protocol = "stdio"
command = "MCP server path"
args = [" "]

---
File: modelcontextprotocol-rust-sdk-13e72ec/examples/simple-chat-client/src/error.rs
---

use std::fmt;

use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct McpError {
    pub message: String,
}

impl fmt::Display for McpError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for McpError {}

impl McpError {
    pub fn new(message: impl ToString) -> Self {
        Self {
            message: message.to_string(),
        }
    }
}

---
File: modelcontextprotocol-rust-sdk-13e72ec/examples/simple-chat-client/src/lib.rs
---

pub mod chat;
pub mod client;
pub mod config;
pub mod error;
pub mod model;
pub mod tool;

---
File: modelcontextprotocol-rust-sdk-13e72ec/examples/simple-chat-client/src/model.rs
---

use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Message {
    pub role: String,
    pub content: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<Vec<ToolCall>>,
}

impl Message {
    pub fn system(content: impl ToString) -> Self {
        Self {
            role: "system".to_string(),
            content: content.to_string(),
            tool_calls: None,
        }
    }

    pub fn user(content: impl ToString) -> Self {
        Self {
            role: "user".to_string(),
            content: content.to_string(),
            tool_calls: None,
        }
    }

    pub fn assistant(content: impl ToString) -> Self {
        Self {
            role: "assistant".to_string(),
            content: content.to_string(),
            tool_calls: None,
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CompletionRequest {
    pub model: String,
    pub messages: Vec<Message>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tools: Option<Vec<Tool>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Tool {
    pub name: String,
    pub description: String,
    pub parameters: serde_json::Value,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CompletionResponse {
    pub id: String,
    pub object: String,
    pub created: u64,
    pub model: String,
    pub choices: Vec<Choice>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Choice {
    pub index: u32,
    pub message: Message,
    pub finish_reason: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ToolCall {
    pub id: String,
    #[serde(rename = "type")]
    pub _type: String,
    pub function: ToolFunction,
}
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ToolFunction {
    pub name: String,
    pub arguments: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ToolResult {
    pub success: bool,
    pub contents: Vec<Content>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Content {
    pub content_type: String,
    pub body: String,
}

impl Content {
    pub fn text(content: impl ToString) -> Self {
        Self {
            content_type: "text/plain".to_string(),
            body: content.to_string(),
        }
    }
}

---
File: modelcontextprotocol-rust-sdk-13e72ec/examples/simple-chat-client/src/tool.rs
---

use std::{collections::HashMap, sync::Arc};

use anyhow::Result;
use async_trait::async_trait;
use rmcp::{
    model::{CallToolRequestParam, CallToolResult, Tool as McpTool},
    service::ServerSink,
};
use serde_json::Value;

use crate::{
    error::McpError,
    model::{Content, ToolResult},
};

#[async_trait]
pub trait Tool: Send + Sync {
    fn name(&self) -> String;
    fn description(&self) -> String;
    fn parameters(&self) -> Value;
    async fn call(&self, args: Value) -> Result<CallToolResult>;
}

pub struct McpToolAdapter {
    tool: McpTool,
    server: ServerSink,
}

impl McpToolAdapter {
    pub fn new(tool: McpTool, server: ServerSink) -> Self {
        Self { tool, server }
    }
}

#[async_trait]
impl Tool for McpToolAdapter {
    fn name(&self) -> String {
        self.tool.name.clone().to_string()
    }

    fn description(&self) -> String {
        self.tool
            .description
            .clone()
            .unwrap_or_default()
            .to_string()
    }

    fn parameters(&self) -> Value {
        serde_json::to_value(&self.tool.input_schema).unwrap_or(serde_json::json!({}))
    }

    async fn call(&self, args: Value) -> Result<CallToolResult> {
        let arguments = match args {
            Value::Object(map) => Some(map),
            _ => None,
        };
        println!("arguments: {:?}", arguments);
        let call_result = self
            .server
            .call_tool(CallToolRequestParam {
                name: self.tool.name.clone(),
                arguments,
            })
            .await?;

        Ok(call_result)
    }
}
#[derive(Default)]
pub struct ToolSet {
    tools: HashMap<String, Arc<dyn Tool>>,
}

impl ToolSet {
    pub fn add_tool<T: Tool + 'static>(&mut self, tool: T) {
        self.tools.insert(tool.name(), Arc::new(tool));
    }

    pub fn get_tool(&self, name: &str) -> Option<Arc<dyn Tool>> {
        self.tools.get(name).cloned()
    }

    pub fn tools(&self) -> Vec<Arc<dyn Tool>> {
        self.tools.values().cloned().collect()
    }
}

pub async fn get_mcp_tools(server: ServerSink) -> Result<Vec<McpToolAdapter>> {
    let tools = server.list_all_tools().await?;
    Ok(tools
        .into_iter()
        .map(|tool| McpToolAdapter::new(tool, server.clone()))
        .collect())
}

pub trait IntoCallToolResult {
    fn into_call_tool_result(self) -> Result<ToolResult, McpError>;
}

impl<T> IntoCallToolResult for Result<T, McpError>
where
    T: serde::Serialize,
{
    fn into_call_tool_result(self) -> Result<ToolResult, McpError> {
        match self {
            Ok(response) => {
                let content = Content {
                    content_type: "application/json".to_string(),
                    body: serde_json::to_string(&response).unwrap_or_default(),
                };
                Ok(ToolResult {
                    success: true,
                    contents: vec![content],
                })
            }
            Err(error) => {
                let content = Content {
                    content_type: "application/json".to_string(),
                    body: serde_json::to_string(&error).unwrap_or_default(),
                };
                Ok(ToolResult {
                    success: false,
                    contents: vec![content],
                })
            }
        }
    }
}

---
File: modelcontextprotocol-rust-sdk-13e72ec/examples/transport/Cargo.toml
---

[package]
name = "transport"
edition = { workspace = true }
version = { workspace = true }
authors = { workspace = true }
license = { workspace = true }
repository = { workspace = true }
description = { workspace = true }
keywords = { workspace = true }
homepage = { workspace = true }
categories = { workspace = true }
readme = { workspace = true }

[package.metadata.docs.rs]
all-features = true

[dependencies]
rmcp = { path = "../../crates/rmcp", features = ["server", "client"] }
tokio = { version = "1", features = [
    "macros",
    "rt",
    "rt-multi-thread",
    "io-std",
    "net",
    "fs",
    "time",
] }
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
anyhow = "1.0"
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = [
    "env-filter",
    "std",
    "fmt",
] }
futures = "0.3"
rand = { version = "0.9" }
schemars = { version = "0.8", optional = true }
hyper = { version = "1", features = ["client", "server", "http1"] }
hyper-util = { version = "0.1", features = ["tokio"] }
tokio-tungstenite = "0.27.0"
reqwest = { version = "0.12" }
pin-project-lite = "0.2"

[[example]]
name = "tcp"
path = "src/tcp.rs"


[[example]]
name = "http_upgrade"
path = "src/http_upgrade.rs"

[[example]]
name = "unix_socket"
path = "src/unix_socket.rs"

[[example]]
name = "websocket"
path = "src/websocket.rs"

[[example]]
name = "named-pipe"
path = "src/named-pipe.rs"

---
File: modelcontextprotocol-rust-sdk-13e72ec/examples/transport/src/common/calculator.rs
---

use rmcp::{ServerHandler, model::ServerInfo, schemars, tool, tool_box};

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct SumRequest {
    #[schemars(description = "the left hand side number")]
    pub a: i32,
    pub b: i32,
}
#[derive(Debug, Clone)]
pub struct Calculator;
impl Calculator {
    #[tool(description = "Calculate the sum of two numbers")]
    fn sum(&self, #[tool(aggr)] SumRequest { a, b }: SumRequest) -> String {
        (a + b).to_string()
    }

    #[tool(description = "Calculate the sub of two numbers")]
    fn sub(
        &self,
        #[tool(param)]
        #[schemars(description = "the left hand side number")]
        a: i32,
        #[tool(param)]
        #[schemars(description = "the right hand side number")]
        b: i32,
    ) -> String {
        (a - b).to_string()
    }

    tool_box!(Calculator { sum, sub });
}

impl ServerHandler for Calculator {
    tool_box!(@derive);
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            instructions: Some("A simple calculator".into()),
            ..Default::default()
        }
    }
}

---
File: modelcontextprotocol-rust-sdk-13e72ec/examples/transport/src/common/mod.rs
---

pub mod calculator;

---
File: modelcontextprotocol-rust-sdk-13e72ec/examples/transport/src/http_upgrade.rs
---

use common::calculator::Calculator;
use hyper::{
    Request, StatusCode,
    body::Incoming,
    header::{HeaderValue, UPGRADE},
};
use hyper_util::rt::TokioIo;
use rmcp::{RoleClient, ServiceExt, service::RunningService};
use tracing_subscriber::EnvFilter;
mod common;
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env().add_directive(tracing::Level::INFO.into()))
        .init();
    start_server().await?;
    let client = http_client("127.0.0.1:8001").await?;
    let tools = client.list_all_tools().await?;
    client.cancel().await?;
    tracing::info!("{:#?}", tools);
    Ok(())
}

async fn http_server(req: Request<Incoming>) -> Result<hyper::Response<String>, hyper::Error> {
    tokio::spawn(async move {
        let upgraded = hyper::upgrade::on(req).await?;
        let service = Calculator.serve(TokioIo::new(upgraded)).await?;
        service.waiting().await?;
        anyhow::Result::<()>::Ok(())
    });
    let mut response = hyper::Response::new(String::new());
    *response.status_mut() = StatusCode::SWITCHING_PROTOCOLS;
    response
        .headers_mut()
        .insert(UPGRADE, HeaderValue::from_static("mcp"));
    Ok(response)
}

async fn http_client(uri: &str) -> anyhow::Result<RunningService<RoleClient, ()>> {
    let tcp_stream = tokio::net::TcpStream::connect(uri).await?;
    let (mut s, c) =
        hyper::client::conn::http1::handshake::<_, String>(TokioIo::new(tcp_stream)).await?;
    tokio::spawn(c.with_upgrades());
    let mut req = Request::new(String::new());
    req.headers_mut()
        .insert(UPGRADE, HeaderValue::from_static("mcp"));
    let response = s.send_request(req).await?;
    let upgraded = hyper::upgrade::on(response).await?;
    let client = ().serve(TokioIo::new(upgraded)).await?;
    Ok(client)
}

async fn start_server() -> anyhow::Result<()> {
    let tcp_listener = tokio::net::TcpListener::bind("127.0.0.1:8001").await?;
    let service = hyper::service::service_fn(http_server);
    tokio::spawn(async move {
        while let Ok((stream, addr)) = tcp_listener.accept().await {
            tracing::info!("accepted connection from: {}", addr);
            let conn = hyper::server::conn::http1::Builder::new()
                .serve_connection(TokioIo::new(stream), service)
                .with_upgrades();
            tokio::spawn(conn);
        }
    });

    Ok(())
}

---
File: modelcontextprotocol-rust-sdk-13e72ec/examples/transport/src/named-pipe.rs
---

mod common;

#[cfg(target_family = "windows")]
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    use common::calculator::Calculator;
    use rmcp::{serve_client, serve_server};
    use tokio::net::windows::named_pipe::{ClientOptions, ServerOptions};
    const PIPE_NAME: &str = r"\\.\pipe\rmcp_example";

    async fn server(name: &str) -> anyhow::Result<()> {
        let mut server = ServerOptions::new()
            .first_pipe_instance(true)
            .create(name)?;
        while let Ok(_) = server.connect().await {
            let stream = server;
            server = ServerOptions::new().create(name)?;
            tokio::spawn(async move {
                match serve_server(Calculator, stream).await {
                    Ok(server) => {
                        println!("Server initialized successfully");
                        if let Err(e) = server.waiting().await {
                            println!("Error while server waiting: {}", e);
                        }
                    }
                    Err(e) => println!("Server initialization failed: {}", e),
                }

                anyhow::Ok(())
            });
        }
        Ok(())
    }

    async fn client() -> anyhow::Result<()> {
        println!("Client connecting to {}", PIPE_NAME);
        let stream = ClientOptions::new().open(PIPE_NAME)?;

        let client = serve_client((), stream).await?;
        println!("Client connected and initialized successfully");

        // List available tools
        let tools = client.peer().list_tools(Default::default()).await?;
        println!("Available tools: {:?}", tools);

        // Call the sum tool
        if let Some(sum_tool) = tools.tools.iter().find(|t| t.name.contains("sum")) {
            println!("Calling sum tool: {}", sum_tool.name);
            let result = client
                .peer()
                .call_tool(rmcp::model::CallToolRequestParam {
                    name: sum_tool.name.clone(),
                    arguments: Some(rmcp::object!({
                        "a": 10,
                        "b": 20
                    })),
                })
                .await?;

            println!("Result: {:?}", result);
        }

        Ok(())
    }
    tokio::spawn(server(PIPE_NAME));
    let mut clients = vec![];

    for _ in 0..100 {
        clients.push(client());
    }
    for client in clients {
        client.await?;
    }
    Ok(())
}

#[cfg(not(target_family = "windows"))]
fn main() {
    println!("Unix socket example is not supported on this platform.");
}

---
File: modelcontextprotocol-rust-sdk-13e72ec/examples/transport/src/tcp.rs
---

use common::calculator::Calculator;
use rmcp::{serve_client, serve_server};

mod common;
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tokio::spawn(server());
    client().await?;
    Ok(())
}

async fn server() -> anyhow::Result<()> {
    let tcp_listener = tokio::net::TcpListener::bind("127.0.0.1:8001").await?;
    while let Ok((stream, _)) = tcp_listener.accept().await {
        tokio::spawn(async move {
            let server = serve_server(Calculator, stream).await?;
            server.waiting().await?;
            anyhow::Ok(())
        });
    }
    Ok(())
}

async fn client() -> anyhow::Result<()> {
    let stream = tokio::net::TcpSocket::new_v4()?
        .connect("127.0.0.1:8001".parse()?)
        .await?;
    let client = serve_client((), stream).await?;
    let tools = client.peer().list_tools(Default::default()).await?;
    println!("{:?}", tools);
    Ok(())
}

---
File: modelcontextprotocol-rust-sdk-13e72ec/examples/transport/src/unix_socket.rs
---

mod common;

#[cfg(target_family = "unix")]
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    use std::fs;

    use common::calculator::Calculator;
    use rmcp::{serve_client, serve_server};
    use tokio::net::{UnixListener, UnixStream};

    const SOCKET_PATH: &str = "/tmp/rmcp_example.sock";
    async fn server(unix_listener: UnixListener) -> anyhow::Result<()> {
        while let Ok((stream, addr)) = unix_listener.accept().await {
            println!("Client connected: {:?}", addr);
            tokio::spawn(async move {
                match serve_server(Calculator, stream).await {
                    Ok(server) => {
                        println!("Server initialized successfully");
                        if let Err(e) = server.waiting().await {
                            println!("Error while server waiting: {}", e);
                        }
                    }
                    Err(e) => println!("Server initialization failed: {}", e),
                }

                anyhow::Ok(())
            });
        }
        Ok(())
    }

    async fn client() -> anyhow::Result<()> {
        println!("Client connecting to {}", SOCKET_PATH);
        let stream = UnixStream::connect(SOCKET_PATH).await?;

        let client = serve_client((), stream).await?;
        println!("Client connected and initialized successfully");

        // List available tools
        let tools = client.peer().list_tools(Default::default()).await?;
        println!("Available tools: {:?}", tools);

        // Call the sum tool
        if let Some(sum_tool) = tools.tools.iter().find(|t| t.name.contains("sum")) {
            println!("Calling sum tool: {}", sum_tool.name);
            let result = client
                .peer()
                .call_tool(rmcp::model::CallToolRequestParam {
                    name: sum_tool.name.clone(),
                    arguments: Some(rmcp::object!({
                        "a": 10,
                        "b": 20
                    })),
                })
                .await?;

            println!("Result: {:?}", result);
        }

        Ok(())
    }

    // Remove any existing socket file
    let _ = fs::remove_file(SOCKET_PATH);
    match UnixListener::bind(SOCKET_PATH) {
        Ok(unix_listener) => {
            println!("Server successfully listening on {}", SOCKET_PATH);
            tokio::spawn(server(unix_listener));
        }
        Err(e) => {
            println!("Unable to bind to {}: {}", SOCKET_PATH, e);
        }
    }

    client().await?;

    // Clean up socket file
    let _ = fs::remove_file(SOCKET_PATH);

    Ok(())
}

#[cfg(not(target_family = "unix"))]
fn main() {
    println!("Unix socket example is not supported on this platform.");
}

---
File: modelcontextprotocol-rust-sdk-13e72ec/examples/transport/src/websocket.rs
---

use std::marker::PhantomData;

use common::calculator::Calculator;
use futures::{Sink, Stream};
use rmcp::{
    RoleClient, RoleServer, ServiceExt,
    service::{RunningService, RxJsonRpcMessage, ServiceRole, TxJsonRpcMessage},
};
use tokio_tungstenite::tungstenite;
use tracing_subscriber::EnvFilter;
mod common;
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env().add_directive(tracing::Level::INFO.into()))
        .init();
    start_server().await?;
    let client = http_client("ws://127.0.0.1:8001").await?;
    let tools = client.list_all_tools().await?;
    client.cancel().await?;
    tracing::info!("{:#?}", tools);
    Ok(())
}

async fn http_client(uri: &str) -> anyhow::Result<RunningService<RoleClient, ()>> {
    let (stream, response) = tokio_tungstenite::connect_async(uri).await?;
    if response.status() != tungstenite::http::StatusCode::SWITCHING_PROTOCOLS {
        return Err(anyhow::anyhow!("failed to upgrade connection"));
    }
    let transport = WebsocketTransport::new_client(stream);
    let client = ().serve(transport).await?;
    Ok(client)
}

async fn start_server() -> anyhow::Result<()> {
    let tcp_listener = tokio::net::TcpListener::bind("127.0.0.1:8001").await?;
    tokio::spawn(async move {
        while let Ok((stream, addr)) = tcp_listener.accept().await {
            tracing::info!("accepted connection from: {}", addr);
            tokio::spawn(async move {
                let ws_stream = tokio_tungstenite::accept_async(stream).await?;
                let transport = WebsocketTransport::new_server(ws_stream);
                let server = Calculator.serve(transport).await?;
                server.waiting().await?;
                Ok::<(), anyhow::Error>(())
            });
        }
    });
    Ok(())
}

pin_project_lite::pin_project! {
    pub struct WebsocketTransport<R, S, E> {
        #[pin]
        stream: S,
        marker: PhantomData<(fn() -> E, fn() -> R)>
    }
}

impl<R, S, E> WebsocketTransport<R, S, E> {
    pub fn new(stream: S) -> Self {
        Self {
            stream,
            marker: PhantomData,
        }
    }
}

impl<S, E> WebsocketTransport<RoleClient, S, E> {
    pub fn new_client(stream: S) -> Self {
        Self {
            stream,
            marker: PhantomData,
        }
    }
}

impl<S, E> WebsocketTransport<RoleServer, S, E> {
    pub fn new_server(stream: S) -> Self {
        Self {
            stream,
            marker: PhantomData,
        }
    }
}

impl<R, S, E> Stream for WebsocketTransport<R, S, E>
where
    S: Stream<Item = Result<tungstenite::Message, E>>,
    R: ServiceRole,
    E: std::error::Error,
{
    type Item = RxJsonRpcMessage<R>;

    fn poll_next(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Option<Self::Item>> {
        let this = self.as_mut().project();
        match this.stream.poll_next(cx) {
            std::task::Poll::Ready(Some(Ok(message))) => {
                let message = match message {
                    tungstenite::Message::Text(json) => json,
                    _ => return self.poll_next(cx),
                };
                let message = match serde_json::from_str::<RxJsonRpcMessage<R>>(&message) {
                    Ok(message) => message,
                    Err(e) => {
                        tracing::warn!(error = %e, "serde_json parse error");
                        return self.poll_next(cx);
                    }
                };
                std::task::Poll::Ready(Some(message))
            }
            std::task::Poll::Ready(Some(Err(e))) => {
                tracing::warn!(error = %e, "websocket error");
                self.poll_next(cx)
            }
            std::task::Poll::Ready(None) => std::task::Poll::Ready(None),
            std::task::Poll::Pending => std::task::Poll::Pending,
        }
    }
}

impl<R, S, E> Sink<TxJsonRpcMessage<R>> for WebsocketTransport<R, S, E>
where
    S: Sink<tungstenite::Message, Error = E>,
    R: ServiceRole,
{
    type Error = E;

    fn poll_ready(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), Self::Error>> {
        let this = self.project();
        this.stream.poll_ready(cx)
    }

    fn start_send(
        self: std::pin::Pin<&mut Self>,
        item: TxJsonRpcMessage<R>,
    ) -> Result<(), Self::Error> {
        let this = self.project();
        let message = tungstenite::Message::Text(
            serde_json::to_string(&item)
                .expect("jsonrpc should be valid json")
                .into(),
        );
        this.stream.start_send(message)
    }

    fn poll_flush(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), Self::Error>> {
        let this = self.project();
        this.stream.poll_flush(cx)
    }

    fn poll_close(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), Self::Error>> {
        let this = self.project();
        this.stream.poll_close(cx)
    }
}

---
File: modelcontextprotocol-rust-sdk-13e72ec/examples/wasi/Cargo.toml
---

[package]
name = "wasi-mcp-example"
edition = { workspace = true }
version = { workspace = true }
authors = { workspace = true }
license = { workspace = true }
repository = { workspace = true }
description = { workspace = true }
keywords = { workspace = true }
homepage = { workspace = true }
categories = { workspace = true }
readme = { workspace = true }

[lib]
crate-type = ["cdylib"]

[dependencies]
wasi = { version = "0.14.2"}
tokio = { version = "1", features = ["rt", "io-util", "sync", "macros", "time"] }
rmcp= { path = "../../crates/rmcp", features = ["server", "macros"] }
serde = { version  = "1", features = ["derive"]}
tracing-subscriber = { version = "0.3", features = [
    "env-filter",
    "std",
    "fmt",
] }
tracing = "0.1"

---
File: modelcontextprotocol-rust-sdk-13e72ec/examples/wasi/README.md
---

# Example for WASI-p2

Build:

```sh
cargo build -p wasi-mcp-example --target wasm32-wasip2
```

Run:

```
npx @modelcontextprotocol/inspector wasmtime target/wasm32-wasip2/debug/wasi_mcp_example.wasm
```

*Note:* Change `wasmtime` to a different installed run time, if needed.

The printed URL of the MCP inspector can be opened and a connection to the module established via `STDIO`.

---
File: modelcontextprotocol-rust-sdk-13e72ec/examples/wasi/config.toml
---

[build]
target = "wasm32-wasip2"

---
File: modelcontextprotocol-rust-sdk-13e72ec/examples/wasi/src/calculator.rs
---

use rmcp::{
    ServerHandler,
    model::{ServerCapabilities, ServerInfo},
    schemars, tool, tool_box,
};

#[derive(Debug, rmcp::serde::Deserialize, schemars::JsonSchema)]
pub struct SumRequest {
    #[schemars(description = "the left hand side number")]
    pub a: i32,
    pub b: i32,
}
#[derive(Debug, Clone)]
pub struct Calculator;
impl Calculator {
    #[tool(description = "Calculate the sum of two numbers")]
    fn sum(&self, #[tool(aggr)] SumRequest { a, b }: SumRequest) -> String {
        (a + b).to_string()
    }

    #[tool(description = "Calculate the sub of two numbers")]
    fn sub(
        &self,
        #[tool(param)]
        #[schemars(description = "the left hand side number")]
        a: i32,
        #[tool(param)]
        #[schemars(description = "the right hand side number")]
        b: i32,
    ) -> String {
        (a - b).to_string()
    }

    tool_box!(Calculator { sum, sub });
}

impl ServerHandler for Calculator {
    tool_box!(@derive);
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            instructions: Some("A simple calculator".into()),
            capabilities: ServerCapabilities::builder().enable_tools().build(),
            ..Default::default()
        }
    }
}

---
File: modelcontextprotocol-rust-sdk-13e72ec/examples/wasi/src/lib.rs
---

pub mod calculator;
use std::task::{Poll, Waker};

use rmcp::ServiceExt;
use tokio::io::{AsyncRead, AsyncWrite};
use tracing_subscriber::EnvFilter;
use wasi::{
    cli::{
        stdin::{InputStream, get_stdin},
        stdout::{OutputStream, get_stdout},
    },
    io::streams::Pollable,
};

pub fn wasi_io() -> (AsyncInputStream, AsyncOutputStream) {
    let input = AsyncInputStream { inner: get_stdin() };
    let output = AsyncOutputStream {
        inner: get_stdout(),
    };
    (input, output)
}

pub struct AsyncInputStream {
    inner: InputStream,
}

impl AsyncRead for AsyncInputStream {
    fn poll_read(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        buf: &mut tokio::io::ReadBuf<'_>,
    ) -> std::task::Poll<std::io::Result<()>> {
        let bytes = self
            .inner
            .read(buf.remaining() as u64)
            .map_err(std::io::Error::other)?;
        if bytes.is_empty() {
            let pollable = self.inner.subscribe();
            let waker = cx.waker().clone();
            runtime_poll(waker, pollable);
            return Poll::Pending;
        }
        buf.put_slice(&bytes);
        std::task::Poll::Ready(Ok(()))
    }
}

pub struct AsyncOutputStream {
    inner: OutputStream,
}
fn runtime_poll(waker: Waker, pollable: Pollable) {
    tokio::task::spawn(async move {
        loop {
            if pollable.ready() {
                waker.wake();
                break;
            } else {
                tokio::task::yield_now().await;
            }
        }
    });
}
impl AsyncWrite for AsyncOutputStream {
    fn poll_write(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        buf: &[u8],
    ) -> Poll<Result<usize, std::io::Error>> {
        let writable_len = self.inner.check_write().map_err(std::io::Error::other)?;
        if writable_len == 0 {
            let pollable = self.inner.subscribe();
            let waker = cx.waker().clone();
            runtime_poll(waker, pollable);
            return Poll::Pending;
        }
        let bytes_to_write = buf.len().min(writable_len as usize);
        self.inner
            .write(&buf[0..bytes_to_write])
            .map_err(std::io::Error::other)?;
        Poll::Ready(Ok(bytes_to_write))
    }

    fn poll_flush(
        self: std::pin::Pin<&mut Self>,
        _cx: &mut std::task::Context<'_>,
    ) -> Poll<Result<(), std::io::Error>> {
        self.inner.flush().map_err(std::io::Error::other)?;
        Poll::Ready(Ok(()))
    }

    fn poll_shutdown(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> Poll<Result<(), std::io::Error>> {
        self.poll_flush(cx)
    }
}

struct TokioCliRunner;

impl wasi::exports::cli::run::Guest for TokioCliRunner {
    fn run() -> Result<(), ()> {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        rt.block_on(async move {
            tracing_subscriber::fmt()
                .with_env_filter(
                    EnvFilter::from_default_env().add_directive(tracing::Level::DEBUG.into()),
                )
                .with_writer(std::io::stderr)
                .with_ansi(false)
                .init();
            let server = calculator::Calculator.serve(wasi_io()).await.unwrap();
            server.waiting().await.unwrap();
        });
        Ok(())
    }
}
wasi::cli::command::export!(TokioCliRunner);

---
File: modelcontextprotocol-rust-sdk-13e72ec/rust-toolchain.toml
---

[toolchain]
channel = "1.85"
components = ["rustc", "rust-std", "cargo", "clippy", "rustfmt", "rust-docs"]

---
File: modelcontextprotocol-rust-sdk-13e72ec/rustfmt.toml
---

newline_style = "Unix"
unstable_features = true # Cargo fmt now needs to be called with `cargo +nightly fmt`
group_imports = "StdExternalCrate" # Create 3 groups: std, external crates, and self.
imports_granularity = "Crate" # Merge imports from the same crate into a single use statement
style_edition = "2024"
max_width = 100
