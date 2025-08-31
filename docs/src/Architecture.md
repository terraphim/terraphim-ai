# Terraphim AI Architecture

This document provides a comprehensive architectural overview of Terraphim AI, a privacy-first AI assistant that operates locally, providing semantic search across multiple knowledge repositories.

## Overall System Architecture

The system follows a multi-layered architecture with clear separation of concerns:

```mermaid
graph TB
    subgraph "Frontend Layer"
        UI[Svelte Desktop UI<br/>with Tauri]
        WebUI[Web Interface<br/>Alternative]
        TUI[Terminal UI<br/>terraphim_tui]
    end

    subgraph "API Layer"
        HTTP[HTTP API Server<br/>terraphim_server]
        MCP[MCP Server<br/>AI Integration]
    end

    subgraph "Service Layer"
        Service[Core Service<br/>terraphim_service]
        Middleware[Search Orchestration<br/>terraphim_middleware]
    end

    subgraph "Knowledge & Processing Layer"
        KG[Knowledge Graph<br/>terraphim_rolegraph]
        Automata[Text Processing<br/>terraphim_automata]
        Config[Configuration<br/>terraphim_config]
    end

    subgraph "Data Layer"
        Persistence[Storage Abstraction<br/>terraphim_persistence]
        Haystacks[Data Sources<br/>Ripgrep, Atomic, MCP]
    end

    subgraph "External Systems"
        LLM[LLM Providers<br/>OpenRouter, Ollama]
        AtomicDB[(Atomic Data<br/>Protocol)]
        LocalFS[(Local Files<br/>Documents)]
        APIs[External APIs<br/>Reddit, ClickUp]
    end

    UI --> HTTP
    WebUI --> HTTP
    TUI --> Service

    HTTP --> Service
    MCP --> Service

    Service --> Middleware
    Service --> KG
    Service --> Config

    Middleware --> Automata
    Middleware --> Persistence

    KG --> Automata

    Persistence --> Haystacks

    Haystacks --> AtomicDB
    Haystacks --> LocalFS
    Haystacks --> APIs

    Service --> LLM

    classDef frontend fill:#e1f5fe
    classDef api fill:#f3e5f5
    classDef service fill:#e8f5e8
    classDef knowledge fill:#fff3e0
    classDef data fill:#fce4ec
    classDef external fill:#f1f8e9

    class UI,WebUI,TUI frontend
    class HTTP,MCP api
    class Service,Middleware service
    class KG,Automata,Config knowledge
    class Persistence,Haystacks data
    class LLM,AtomicDB,LocalFS,APIs external
```

## Core Components Architecture

### Search and Knowledge Processing Flow

```mermaid
flowchart TD
    subgraph "User Interaction"
        Query[Search Query<br/>from UI]
        Results[Search Results<br/>to UI]
    end

    subgraph "Search Processing"
        Parse[Query Parsing<br/>& Normalization]
        Expand[Semantic Expansion<br/>via Knowledge Graph]
        Execute[Multi-Haystack<br/>Search Execution]
    end

    subgraph "Knowledge Graph System"
        Thesaurus[Thesaurus<br/>Concept Mapping]
        Automata[FST Automata<br/>Fast Text Matching]
        Graph[Role Graph<br/>Document Relationships]
    end

    subgraph "Relevance Scoring"
        TitleScorer[Title Scorer<br/>Basic Matching]
        BM25[BM25 Family<br/>Statistical Relevance]
        TerraphimGraph[Terraphim Graph<br/>Semantic Ranking]
    end

    subgraph "Data Sources"
        Ripgrep[Local Files<br/>ripgrep]
        Atomic[Atomic Server<br/>Structured Data]
        QueryRs[Rust Docs<br/>& Reddit]
        MCP[MCP Tools<br/>AI Integration]
    end

    Query --> Parse
    Parse --> Expand
    Expand --> Execute

    Expand --> Thesaurus
    Thesaurus --> Automata
    Automata --> Graph

    Execute --> Ripgrep
    Execute --> Atomic
    Execute --> QueryRs
    Execute --> MCP

    Ripgrep --> TitleScorer
    Atomic --> BM25
    QueryRs --> TerraphimGraph
    MCP --> TerraphimGraph

    TitleScorer --> Results
    BM25 --> Results
    TerraphimGraph --> Results

    classDef userLayer fill:#e3f2fd
    classDef processLayer fill:#e8f5e8
    classDef knowledgeLayer fill:#fff3e0
    classDef scoreLayer fill:#f3e5f5
    classDef dataLayer fill:#fce4ec

    class Query,Results userLayer
    class Parse,Expand,Execute processLayer
    class Thesaurus,Automata,Graph knowledgeLayer
    class TitleScorer,BM25,TerraphimGraph scoreLayer
    class Ripgrep,Atomic,QueryRs,MCP dataLayer
```

## Crate Dependency Architecture

```mermaid
graph TD
    subgraph "Application Layer"
        Server[terraphim_server<br/>HTTP API + Main Binary]
        Desktop[Desktop App<br/>Svelte + Tauri]
        TUI[terraphim_tui<br/>Terminal Interface]
    end

    subgraph "Service Layer"
        Service[terraphim_service<br/>Core Business Logic]
        Middleware[terraphim_middleware<br/>Search Orchestration]
        MCP[terraphim_mcp_server<br/>AI Tool Integration]
    end

    subgraph "Domain Layer"
        RoleGraph[terraphim_rolegraph<br/>Knowledge Graph]
        Automata[terraphim_automata<br/>Text Processing]
        Config[terraphim_config<br/>Configuration]
    end

    subgraph "Infrastructure Layer"
        Persistence[terraphim_persistence<br/>Storage Abstraction]
        Settings[terraphim_settings<br/>System Settings]
        Types[terraphim_types<br/>Shared Types]
        AtomicClient[terraphim_atomic_client<br/>Atomic Data Client]
        MarkdownParser[terraphim-markdown-parser<br/>Markdown Processing]
        OnePassword[terraphim_onepassword_cli<br/>1Password Integration]
    end

    Server --> Service
    Desktop --> Server
    TUI --> Service

    Service --> Middleware
    Service --> Config
    Service --> RoleGraph
    MCP --> Service

    Middleware --> Automata
    Middleware --> Persistence

    RoleGraph --> Automata
    RoleGraph --> Types

    Config --> Types
    Config --> Settings

    Persistence --> AtomicClient
    Persistence --> Types

    Automata --> MarkdownParser

    Service --> OnePassword

    classDef appLayer fill:#e1f5fe
    classDef serviceLayer fill:#e8f5e8
    classDef domainLayer fill:#fff3e0
    classDef infraLayer fill:#fce4ec

    class Server,Desktop,TUI appLayer
    class Service,Middleware,MCP serviceLayer
    class RoleGraph,Automata,Config domainLayer
    class Persistence,Settings,Types,AtomicClient,MarkdownParser,OnePassword infraLayer
```

## Configuration and Role System

```mermaid
graph TB
    subgraph "Configuration Hierarchy"
        GlobalConfig[Global Configuration<br/>terraphim_server/default/]
        RoleConfigs[Role-Specific Configs<br/>Engineer, System Operator]
        UserSettings[User Settings<br/>settings.toml]
    end

    subgraph "Role Components"
        Role[Role Definition]
        Haystacks[Data Sources<br/>Configuration]
        KnowledgeGraph[Knowledge Graph<br/>Settings]
        RelevanceFunction[Relevance Function<br/>Selection]
        Theme[UI Theme<br/>Configuration]
    end

    subgraph "Haystack Types"
        RipgrepConfig[Ripgrep<br/>Local Files]
        AtomicConfig[Atomic Server<br/>Structured Data]
        QueryRsConfig[QueryRs<br/>Documentation]
        MCPConfig[MCP<br/>AI Tools]
        ClickUpConfig[ClickUp<br/>Task Management]
    end

    subgraph "LLM Integration"
        OpenRouter[OpenRouter<br/>Cloud Models]
        Ollama[Ollama<br/>Local Models]
        GenericLLM[Generic LLM<br/>Interface]
    end

    GlobalConfig --> RoleConfigs
    RoleConfigs --> UserSettings

    RoleConfigs --> Role
    Role --> Haystacks
    Role --> KnowledgeGraph
    Role --> RelevanceFunction
    Role --> Theme

    Haystacks --> RipgrepConfig
    Haystacks --> AtomicConfig
    Haystacks --> QueryRsConfig
    Haystacks --> MCPConfig
    Haystacks --> ClickUpConfig

    Role --> OpenRouter
    Role --> Ollama
    Role --> GenericLLM

    classDef configLayer fill:#e3f2fd
    classDef roleLayer fill:#e8f5e8
    classDef haystackLayer fill:#fff3e0
    classDef llmLayer fill:#f3e5f5

    class GlobalConfig,RoleConfigs,UserSettings configLayer
    class Role,Haystacks,KnowledgeGraph,RelevanceFunction,Theme roleLayer
    class RipgrepConfig,AtomicConfig,QueryRsConfig,MCPConfig,ClickUpConfig haystackLayer
    class OpenRouter,Ollama,GenericLLM llmLayer
```

## Knowledge Graph Processing Pipeline

```mermaid
flowchart LR
    subgraph "Input Sources"
        Documents[Documents<br/>Markdown/Text]
        URLs[Remote URLs<br/>Thesaurus]
        ManualInput[Manual Input<br/>Concept Mapping]
    end

    subgraph "Processing Pipeline"
        Extract[Concept Extraction<br/>NLP Processing]
        Build[Thesaurus Building<br/>JSON Format]
        Automata[FST Construction<br/>Fast Matching]
        Graph[Graph Building<br/>Node/Edge Creation]
    end

    subgraph "Storage & Usage"
        Cache[Automata Cache<br/>Performance]
        RoleGraph[Role-Specific Graph<br/>Personalized Search]
        SearchEnhancement[Search Enhancement<br/>Semantic Expansion]
    end

    Documents --> Extract
    URLs --> Build
    ManualInput --> Build

    Extract --> Build
    Build --> Automata
    Automata --> Graph

    Automata --> Cache
    Graph --> RoleGraph
    RoleGraph --> SearchEnhancement

    classDef inputLayer fill:#e8f5e8
    classDef processLayer fill:#fff3e0
    classDef storageLayer fill:#f3e5f5

    class Documents,URLs,ManualInput inputLayer
    class Extract,Build,Automata,Graph processLayer
    class Cache,RoleGraph,SearchEnhancement storageLayer
```

## Desktop Application Architecture

```mermaid
graph TB
    subgraph "Frontend (Svelte)"
        SearchUI[Search Interface<br/>Real-time Results]
        ConfigWizard[Configuration Wizard<br/>Role Management]
        GraphViz[Graph Visualization<br/>Knowledge Explorer]
        ChatInterface[Chat Interface<br/>AI Integration]
    end

    subgraph "Tauri Bridge"
        Commands[Tauri Commands<br/>Rust Backend]
        Events[Event System<br/>Real-time Updates]
        FileSystem[File System Access<br/>Native APIs]
    end

    subgraph "Backend Service"
        APILayer[HTTP API<br/>RESTful Endpoints]
        ServiceLayer[Business Logic<br/>Search & Config]
        EventBus[Event Bus<br/>State Management]
    end

    subgraph "State Management"
        ConfigStore[Configuration Store<br/>Role Settings]
        SearchStore[Search State<br/>Query History]
        UIStore[UI State<br/>Theme & Layout]
    end

    SearchUI --> Commands
    ConfigWizard --> Commands
    GraphViz --> Commands
    ChatInterface --> Commands

    Commands --> APILayer
    Events --> EventBus
    FileSystem --> ServiceLayer

    APILayer --> ServiceLayer
    ServiceLayer --> EventBus

    EventBus --> ConfigStore
    EventBus --> SearchStore
    EventBus --> UIStore

    ConfigStore --> ConfigWizard
    SearchStore --> SearchUI
    UIStore --> SearchUI

    classDef frontendLayer fill:#e1f5fe
    classDef bridgeLayer fill:#e8f5e8
    classDef backendLayer fill:#fff3e0
    classDef stateLayer fill:#f3e5f5

    class SearchUI,ConfigWizard,GraphViz,ChatInterface frontendLayer
    class Commands,Events,FileSystem bridgeLayer
    class APILayer,ServiceLayer,EventBus backendLayer
    class ConfigStore,SearchStore,UIStore stateLayer
```

## Data Flow Architecture

```mermaid
sequenceDiagram
    participant UI as User Interface
    participant API as HTTP API Server
    participant Service as terraphim_service
    participant Middleware as terraphim_middleware
    participant KG as Knowledge Graph
    participant Automata as FST Automata
    participant Haystack as Data Sources

    UI->>API: Search Request
    API->>Service: Process Query
    Service->>KG: Get Semantic Expansion
    KG->>Automata: Match Concepts
    Automata-->>KG: Matched Terms
    KG-->>Service: Expanded Query
    Service->>Middleware: Execute Search
    Middleware->>Haystack: Query Data Sources
    Haystack-->>Middleware: Raw Results
    Middleware->>Service: Ranked Results
    Service->>API: Formatted Response
    API-->>UI: Search Results

    Note over Service,Middleware: Relevance scoring applied
    Note over KG,Automata: Thesaurus-based expansion
    Note over Middleware,Haystack: Multi-source aggregation
```

## MCP (Model Context Protocol) Integration

```mermaid
graph TD
    subgraph "MCP Server Components"
        MCPServer[MCP Server<br/>terraphim_mcp_server]
        Tools[Available Tools<br/>Autocomplete, Search, Graph]
        Transport[Transport Layer<br/>stdio, SSE/HTTP]
    end

    subgraph "Core Functions Exposed"
        AutoComplete[Autocomplete Functions<br/>Terms & Snippets]
        TextProcessing[Text Processing<br/>Find & Replace]
        GraphOps[Graph Operations<br/>Connectivity & Paths]
        ThesaurusOps[Thesaurus Management<br/>Load & Process]
    end

    subgraph "AI Development Tools"
        IDEs[Code Editors<br/>VS Code, Cursor]
        AIAssistants[AI Assistants<br/>Claude, GPT]
        DevTools[Development Tools<br/>Custom Integrations]
    end

    subgraph "Core System Integration"
        AutomataCore[terraphim_automata<br/>Core Functions]
        RoleGraphCore[terraphim_rolegraph<br/>Graph Operations]
    end

    MCPServer --> AutoComplete
    MCPServer --> TextProcessing
    MCPServer --> GraphOps
    MCPServer --> ThesaurusOps

    AutoComplete --> AutomataCore
    TextProcessing --> AutomataCore
    GraphOps --> RoleGraphCore
    ThesaurusOps --> AutomataCore

    Transport --> IDEs
    Transport --> AIAssistants
    Transport --> DevTools

    IDEs --> MCPServer
    AIAssistants --> MCPServer
    DevTools --> MCPServer

    classDef mcpLayer fill:#e1f5fe
    classDef functionLayer fill:#e8f5e8
    classDef toolLayer fill:#fff3e0
    classDef coreLayer fill:#f3e5f5

    class MCPServer,Tools,Transport mcpLayer
    class AutoComplete,TextProcessing,GraphOps,ThesaurusOps functionLayer
    class IDEs,AIAssistants,DevTools toolLayer
    class AutomataCore,RoleGraphCore coreLayer
```

## Performance and Scalability Considerations

### Memory Management
- **Automata Caching**: FST structures cached in memory for fast access
- **Bounded Channels**: Backpressure management for async operations
- **Lazy Loading**: Knowledge graphs loaded on demand

### Concurrency Architecture
- **tokio Runtime**: Async/await pattern throughout
- **Structured Concurrency**: Scoped tasks with proper cancellation
- **Channel-based Communication**: mpsc, broadcast, oneshot patterns

### Optimization Strategies
- **Multi-source Search**: Parallel haystack querying
- **Relevance Function Selection**: Performance vs. accuracy tradeoffs
- **Progressive Timeouts**: Quick health checks, longer searches
- **Resource Limits**: Configurable limits for responsive UI

## Security Architecture

### Privacy-First Design
- **Local Processing**: All AI operations can run locally
- **No Data Transmission**: Optional cloud features only
- **Secure Storage**: Encrypted configuration options
- **Access Control**: Role-based permission system

### Authentication & Authorization
- **Atomic Server**: Base64 encoded secrets
- **API Keys**: Secure storage for external services
- **1Password Integration**: Credential management
- **OAuth Support**: Token-based authentication

## Deployment Architecture

### Desktop Deployment
- **Tauri Application**: Native desktop with web frontend
- **Embedded Server**: HTTP server runs in background
- **Auto-updates**: Built-in update mechanism
- **Cross-platform**: Windows, macOS, Linux support

### Development Environment
- **Rust Workspace**: Multi-crate project structure
- **Hot Reload**: Frontend development with Vite
- **Testing Strategy**: Unit, integration, and E2E tests
- **CI/CD Pipeline**: Automated builds and releases

This architecture provides a comprehensive view of the Terraphim AI system, showing how components interact to deliver a privacy-first, locally-operated AI assistant with advanced semantic search capabilities.
