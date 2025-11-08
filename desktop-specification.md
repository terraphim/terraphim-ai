# Terraphim AI Desktop Application - Current Specification

## Overview

Terraphim AI Desktop is a cross-platform desktop application built with Tauri (Rust backend + Svelte frontend) that provides a privacy-first AI assistant with semantic search, knowledge management, and multi-agent workflows.

## Architecture

### Frontend (Svelte + TypeScript)
- **Framework**: Svelte 5.2.8 with TypeScript
- **Routing**: Tinro for client-side routing
- **UI Framework**: Bulma CSS with Bulmaswatch themes
- **Build Tool**: Vite 5.3.4
- **State Management**: Svelte stores with TypeScript integration

### Backend (Tauri + Rust)
- **Framework**: Tauri 1.7.1 with Rust Edition 2021
- **Runtime**: Tokio async runtime
- **Architecture**: Multi-crate workspace sharing core business logic
- **Features**: System tray, global shortcuts, file system access

### Core Components

#### 1. Main Application Structure
```
desktop/
├── src/                    # Svelte frontend
│   ├── App.svelte         # Main application shell
│   ├── lib/               # Component library
│   │   ├── Chat/          # Chat interface components
│   │   ├── Search/        # Search interface components
│   │   ├── Config/        # Configuration components
│   │   └── ThemeSwitcher.svelte
│   └── assets/            # Static assets
├── src-tauri/             # Rust backend
│   ├── src/
│   │   ├── main.rs        # Application entry point
│   │   ├── cmd.rs         # Tauri command handlers
│   │   └── bindings.rs    # TypeScript bindings
│   ├── Cargo.toml         # Rust dependencies
│   └── icons/             # Application icons
└── tests/                 # E2E and integration tests
```

#### 2. User Interface Components

**Main Navigation**:
- Search interface with semantic search capabilities
- Chat interface for AI conversations
- Graph visualization for knowledge graphs
- Configuration wizard and JSON editor

**Key Features**:
- Responsive design with mobile/tablet/desktop layouts
- Theme switching (light/dark modes)
- System tray integration with role switching
- Global shortcuts for show/hide functionality
- Modal dialogs for configuration and context management

#### 3. Backend Services

**Tauri Commands** (40+ endpoints):
- Search and knowledge graph operations
- Configuration management
- Conversation management (in-memory and persistent)
- 1Password integration
- MCP server capabilities
- Document management and autocomplete

**Core Integrations**:
- **MCP Server**: Can run as embedded server or standalone via CLI
- **1Password CLI**: Secret management and configuration
- **Knowledge Graph**: Role-based semantic search
- **Multi-Agent Workflows**: AI agent orchestration

#### 4. Data Management

**Configuration**:
- JSON-based configuration with schema validation
- Role-based configurations with switching
- Device settings with environment variable support

**Persistence**:
- Multi-backend support (SQLite, RocksDB, Redis, S3, Azure)
- User data folder initialization with bundled content
- Conversation persistence with search capabilities

**Security**:
- Local-first data storage for privacy
- 1Password integration for secure secret management
- MCP authentication system with API key validation

## Current Features

### 1. Search System
- Semantic search across multiple data sources
- Autocomplete with Aho-Corasick automata
- Context management and result filtering
- Integration with knowledge graphs

### 2. Chat Interface
- Real-time AI conversations with context
- Multiple conversation management
- Persistent conversation storage
- Context editing and management

### 3. Knowledge Graph Visualization
- Interactive D3.js graph visualization
- Role-based graph filtering
- Term context management
- Search integration

### 4. Configuration Management
- Wizard-based initial setup
- JSON configuration editor with validation
- Role switching and management
- Theme and preference settings

### 5. System Integration
- System tray with quick role switching
- Global shortcuts for window management
- Auto-start capabilities
- Cross-platform file associations

## Technical Specifications

### Dependencies

**Frontend Dependencies**:
- Svelte 5.2.8 (UI framework)
- Bulma 1.0.2 (CSS framework)
- D3.js 7.9.0 (Data visualization)
- TipTap 2.22.1 (Rich text editor)
- @tomic/lib 0.40.0 (Atomic server integration)

**Backend Dependencies**:
- Tauri 1.7.1 (Desktop framework)
- Tokio 1.36.0 (Async runtime)
- Serde 1.0.197 (Serialization)
- Tracing 0.1.40 (Logging)
- Multiple workspace crates for business logic

### Build System
- **Frontend**: Vite with Svelte plugin
- **Backend**: Cargo with Tauri CLI
- **Testing**: Vitest (unit), Playwright (E2E)
- **CI/CD**: GitHub Actions with matrix builds

### Platform Support
- **Linux**: Primary development platform
- **macOS**: Full support with universal binaries
- **Windows**: Support with Windows subsystems

## Performance Characteristics

### Memory Usage
- Frontend: ~50-100MB (Svelte + Vite dev mode)
- Backend: ~100-200MB (Rust runtime + services)
- Total: ~150-300MB typical usage

### Startup Time
- Cold start: 2-3 seconds
- Warm start: <1 second
- MCP server mode: <1 second

### File Sizes
- Linux binary: ~25MB
- macOS bundle: ~50MB
- Windows installer: ~60MB

## Testing Infrastructure

### Test Coverage
- **Unit Tests**: Vitest with 85%+ coverage
- **E2E Tests**: Playwright with 50+ test scenarios
- **Integration Tests**: WebDriver and API testing
- **Performance Tests**: Benchmarking and stress testing

### Test Categories
- Smoke tests for basic functionality
- Atomic server integration tests
- Role graph validation tests
- Performance and stress tests
- Visual regression tests

## Security Features

### Data Privacy
- Local-first data storage
- No external data transmission by default
- User-controlled encryption keys
- GDPR compliance design

### Authentication
- MCP API key authentication with SHA256 hashing
- 1Password integration for secure credential storage
- Role-based access control

### Secure Development
- No mocks in security-critical tests
- Comprehensive input validation
- Memory-safe Rust backend
- Regular security audits

## Current Limitations

### Technical Debt
- Tauri 1.x (upgrade to 2.x planned)
- Mixed dependency management (yarn/npm)
- Complex test setup with multiple frameworks

### Performance
- Large bundle size for simple use cases
- Memory usage with large knowledge graphs
- Startup time for complex configurations

### User Experience
- Limited offline capabilities
- Complex initial setup process
- Mobile responsiveness needs improvement

## Integration Points

### External Services
- **OpenRouter**: Cloud AI model provider
- **Ollama**: Local AI model runner
- **Atomic Server**: Optional knowledge storage
- **1Password**: Credential management

### Internal Services
- **MCP Server**: Model Context Protocol
- **Knowledge Graph**: Semantic search engine
- **Multi-Agent System**: AI workflow orchestration
- **Configuration Service**: Dynamic settings management

This specification serves as the foundation for evaluating migration strategies and architectural improvements for the Terraphim AI Desktop application.