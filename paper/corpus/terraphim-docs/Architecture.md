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
        VSCODE[VS Code Extension<br/>TypeScript]
        NODEJS[Node.js Bindings<br/>NAPI Integration]
    end

    subgraph "API Layer"
        HTTP[HTTP API Server<br/>terraphim_server]
        MCP[MCP Server<br/>terraphim_mcp_server]
    end

    subgraph "Service Layer"
        Service[Core Service<br/>terraphim_service]
        Middleware[Search Orchestration<br/>terraphim_middleware]
    end

    subgraph "Knowledge & Processing Layer"
        KG[Knowledge Graph<br/>terraphim_rolegraph]
        Automata[Text Processing<br/>terraphim_automata]
        Config[Configuration<br/>terraphim_config]
        Settings[Settings Management<br/>terraphim_settings]
        MarkdownParser[Markdown Parser<br/>terraphim-markdown-parser]
    end

    subgraph "Data Layer"
        Persistence[Storage Abstraction<br/>terraphim_persistence]
        Types[Shared Types<br/>terraphim_types]
        BuildArgs[Build Configuration<br/>terraphim_build_args]
    end

    subgraph "Haystack Services"
        RipgrepHaystack[Ripgrep Haystack<br/>Local Files]
        AtomicHaystack[Atomic Haystack<br/>Atomic Server]
        QueryRsHaystack[QueryRs Haystack<br/>Rust Docs & Reddit]
        ClickUpHaystack[ClickUp Haystack<br/>Task Management]
        MCPHaystack[MCP Haystack<br/>AI Tools]
        PerplexityHaystack[Perplexity Haystack<br/>AI Web Search]
        GoogleDocsHaystack[Google Docs Haystack<br/>Google Docs API]
        AtlassianHaystack[Atlassian Haystack<br/>Jira & Confluence]
        DiscourseHaystack[Discourse Haystack<br/>Forum Integration]
        JMAPHaystack[JMAP Haystack<br/>Email Integration]
    end

    subgraph "External Integrations"
        AtomicClient[Atomic Client<br/>terraphim_atomic_client]
        OnePassword[1Password CLI<br/>terraphim_onepassword_cli]
        Update[Update System<br/>terraphim_update]
    end

    subgraph "External Systems"
        LLM[LLM Providers<br/>OpenRouter, Ollama]
        AtomicDB[(Atomic Data<br/>Protocol)]
        LocalFS[(Local Files<br/>Documents)]
        GitHub[GitHub Repositories]
        S3[S3 Storage<br/>staging-storage.terraphim.io]
        GoogleAPI[Google APIs<br/>Docs, Drive]
        AtlassianAPI[Atlassian APIs<br/>Jira, Confluence]
        DiscourseAPI[Discourse API<br/>Forum Data]
        EmailAPI[Email APIs<br/>JMAP Protocol]
    end

    UI --> HTTP
    WebUI --> HTTP
    TUI --> Service
    VSCODE --> MCP
    NODEJS --> Service

    HTTP --> Service
    MCP --> Service

    Service --> Middleware
    Service --> KG
    Service --> Config
    Service --> Settings

    Middleware --> Automata
    Middleware --> Persistence
    Middleware --> RipgrepHaystack
    Middleware --> AtomicHaystack
    Middleware --> QueryRsHaystack
    Middleware --> ClickUpHaystack
    Middleware --> MCPHaystack
    Middleware --> PerplexityHaystack

    KG --> Automata
    KG --> Types

    Config --> Types
    Config --> Settings

    Persistence --> Types
    Persistence --> AtomicClient

    Automata --> MarkdownParser

    Service --> OnePassword
    Service --> Update

    RipgrepHaystack --> LocalFS
    AtomicHaystack --> AtomicDB
    QueryRsHaystack --> GitHub
    ClickUpHaystack --> ExternalAPIs
    MCPHaystack --> ExternalAPIs
    PerplexityHaystack --> ExternalAPIs
    GoogleDocsHaystack --> GoogleAPI
    AtlassianHaystack --> AtlassianAPI
    DiscourseHaystack --> DiscourseAPI
    JMAPHaystack --> EmailAPI

    AtomicClient --> AtomicDB
    Service --> LLM

    classDef frontend fill:#e1f5fe
    classDef api fill:#f3e5f5
    classDef service fill:#e8f5e8
    classDef knowledge fill:#fff3e0
    classDef data fill:#fce4ec
    classDef haystack fill:#e8f5e8
    classDef integration fill:#f3e5f5
    classDef external fill:#f1f8e9

    class UI,WebUI,TUI,VSCODE,NODEJS frontend
    class HTTP,MCP api
    class Service,Middleware service
    class KG,Automata,Config,Settings,MarkdownParser knowledge
    class Persistence,Types,BuildArgs data
    class RipgrepHaystack,AtomicHaystack,QueryRsHaystack,ClickUpHaystack,MCPHaystack,PerplexityHaystack,GoogleDocsHaystack,AtlassianHaystack,DiscourseHaystack,JMAPHaystack haystack
    class AtomicClient,OnePassword,Update integration
    class LLM,AtomicDB,LocalFS,GitHub,S3,GoogleAPI,AtlassianAPI,DiscourseAPI,EmailAPI external
```

## Haystack Services Architecture

The Terraphim AI system supports multiple haystack services, each designed to integrate with different data sources and provide specialized search capabilities:

### Core Haystack Services

#### 1. **Ripgrep Haystack** (`Ripgrep`)
- **Purpose**: Local filesystem search using ripgrep
- **Data Source**: Local markdown and text files
- **Features**: Full-text search, regex support, file filtering
- **Use Cases**: Personal knowledge base, documentation search
- **Performance**: Fast local search with minimal overhead

#### 2. **Atomic Haystack** (`Atomic`)
- **Purpose**: Integration with Atomic Data Protocol
- **Data Source**: Atomic Server (localhost:9883)
- **Features**: Structured data search, real-time updates
- **Use Cases**: Collaborative knowledge management, structured data
- **Authentication**: Base64 encoded secrets

#### 3. **QueryRs Haystack** (`QueryRs`)
- **Purpose**: Rust documentation and Reddit integration
- **Data Source**: Rust docs, Reddit posts, external APIs
- **Features**: API-based search, content aggregation
- **Use Cases**: Developer documentation, community content
- **Integration**: RESTful API calls

#### 4. **ClickUp Haystack** (`ClickUp`)
- **Purpose**: Task and project management integration
- **Data Source**: ClickUp API
- **Features**: Task search, project filtering, team collaboration
- **Use Cases**: Project management, task tracking
- **Authentication**: API key-based

#### 5. **MCP Haystack** (`Mcp`)
- **Purpose**: Model Context Protocol server integration
- **Data Source**: MCP-compatible AI tools
- **Features**: AI-powered search, tool integration
- **Use Cases**: AI assistant integration, tool orchestration
- **Protocol**: MCP standard compliance

#### 6. **Perplexity Haystack** (`Perplexity`)
- **Purpose**: AI-powered web search
- **Data Source**: Perplexity API
- **Features**: Real-time web search, AI-enhanced results
- **Use Cases**: Current information, web research
- **Authentication**: API key-based

### Extended Haystack Services

#### 7. **Google Docs Haystack** (`GoogleDocs`)
- **Purpose**: Google Workspace integration
- **Data Source**: Google Docs, Google Drive
- **Features**: Document conversion to markdown, collaborative editing
- **Use Cases**: Enterprise document management, team collaboration
- **Authentication**: OAuth 2.0 with refresh tokens

#### 8. **Atlassian Haystack** (`Atlassian`)
- **Purpose**: Jira and Confluence integration
- **Data Source**: Jira issues, Confluence pages
- **Features**: Issue tracking, knowledge base search
- **Use Cases**: Software development, project documentation
- **Authentication**: API token-based

#### 9. **Discourse Haystack** (`Discourse`)
- **Purpose**: Forum and community integration
- **Data Source**: Discourse forum posts and topics
- **Features**: Community content search, discussion tracking
- **Use Cases**: Community management, support forums
- **Authentication**: API key-based

#### 10. **JMAP Haystack** (`JMAP`)
- **Purpose**: Email integration via JMAP protocol
- **Data Source**: Email servers (IMAP/SMTP)
- **Features**: Email search, attachment handling
- **Use Cases**: Email management, communication search
- **Protocol**: JMAP standard compliance

### Haystack Configuration

Each haystack is configured with:
- **Service Type**: Defines the underlying service implementation
- **Location**: Path or URL for the data source
- **Read-only Flag**: Prevents modification of source data
- **Authentication**: Service-specific credentials
- **Extra Parameters**: Additional configuration options

### Relevance Scoring Integration

All haystack services integrate with Terraphim's relevance scoring system:
- **TitleScorer**: Basic title-based matching
- **BM25 Family**: Statistical relevance (BM25, BM25F, BM25Plus)
- **TerraphimGraph**: Knowledge graph-based semantic ranking

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
        ClickUp[ClickUp API<br/>Task Management]
        MCP[MCP Tools<br/>AI Integration]
        Perplexity[Perplexity API<br/>AI Web Search]
        GoogleDocs[Google Docs<br/>Workspace Integration]
        Atlassian[Jira & Confluence<br/>Atlassian APIs]
        Discourse[Discourse Forums<br/>Community Content]
        JMAP[Email Servers<br/>JMAP Protocol]
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
    Execute --> ClickUp
    Execute --> MCP
    Execute --> Perplexity
    Execute --> GoogleDocs
    Execute --> Atlassian
    Execute --> Discourse
    Execute --> JMAP

    Ripgrep --> TitleScorer
    Atomic --> BM25
    QueryRs --> TerraphimGraph
    ClickUp --> BM25F
    MCP --> TerraphimGraph
    Perplexity --> BM25Plus
    GoogleDocs --> TitleScorer
    Atlassian --> BM25
    Discourse --> TerraphimGraph
    JMAP --> TitleScorer

    TitleScorer --> Results
    BM25 --> Results
    BM25F --> Results
    BM25Plus --> Results
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
    class Ripgrep,Atomic,QueryRs,ClickUp,MCP,Perplexity,GoogleDocs,Atlassian,Discourse,JMAP dataLayer
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
        Update[terraphim_update<br/>Update System]
        BuildArgs[terraphim_build_args<br/>Build Configuration]
    end

    subgraph "Haystack Layer"
        HaystackCore[haystack_core<br/>Core Haystack Types]
        HaystackAtlassian[haystack_atlassian<br/>Jira & Confluence]
        HaystackDiscourse[haystack_discourse<br/>Forum Integration]
        HaystackGoogleDocs[haystack_googledocs<br/>Google Workspace]
        HaystackJMAP[haystack_jmap<br/>Email Integration]
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
    Service --> Update

    Middleware --> HaystackCore
    HaystackCore --> HaystackAtlassian
    HaystackCore --> HaystackDiscourse
    HaystackCore --> HaystackGoogleDocs
    HaystackCore --> HaystackJMAP

    classDef appLayer fill:#e1f5fe
    classDef serviceLayer fill:#e8f5e8
    classDef domainLayer fill:#fff3e0
    classDef infraLayer fill:#fce4ec
    classDef haystackLayer fill:#e8f5e8

    class Server,Desktop,TUI appLayer
    class Service,Middleware,MCP serviceLayer
    class RoleGraph,Automata,Config domainLayer
    class Persistence,Settings,Types,AtomicClient,MarkdownParser,OnePassword,Update,BuildArgs infraLayer
    class HaystackCore,HaystackAtlassian,HaystackDiscourse,HaystackGoogleDocs,HaystackJMAP haystackLayer
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
        ClickUpConfig[ClickUp<br/>Task Management]
        MCPConfig[MCP<br/>AI Tools]
        PerplexityConfig[Perplexity<br/>AI Web Search]
        GoogleDocsConfig[Google Docs<br/>Workspace Integration]
        AtlassianConfig[Atlassian<br/>Jira & Confluence]
        DiscourseConfig[Discourse<br/>Forum Integration]
        JMAPConfig[JMAP<br/>Email Integration]
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
    Haystacks --> ClickUpConfig
    Haystacks --> MCPConfig
    Haystacks --> PerplexityConfig
    Haystacks --> GoogleDocsConfig
    Haystacks --> AtlassianConfig
    Haystacks --> DiscourseConfig
    Haystacks --> JMAPConfig

    Role --> OpenRouter
    Role --> Ollama
    Role --> GenericLLM

    classDef configLayer fill:#e3f2fd
    classDef roleLayer fill:#e8f5e8
    classDef haystackLayer fill:#fff3e0
    classDef llmLayer fill:#f3e5f5

    class GlobalConfig,RoleConfigs,UserSettings configLayer
    class Role,Haystacks,KnowledgeGraph,RelevanceFunction,Theme roleLayer
    class RipgrepConfig,AtomicConfig,QueryRsConfig,ClickUpConfig,MCPConfig,PerplexityConfig,GoogleDocsConfig,AtlassianConfig,DiscourseConfig,JMAPConfig haystackLayer
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

## Comprehensive System Summary

The Terraphim AI architecture represents a complete ecosystem for privacy-first, locally-operated AI assistance with advanced semantic search capabilities. The system is built around several key architectural principles:

### **Multi-Layer Architecture**
- **Frontend Layer**: Multiple interfaces (Desktop Tauri, Web UI, Terminal UI, VS Code Extension, Node.js bindings)
- **API Layer**: HTTP server and MCP server for different integration patterns
- **Service Layer**: Core business logic and search orchestration
- **Knowledge Layer**: Graph processing, text matching, and configuration management
- **Data Layer**: Storage abstraction and shared type system
- **Haystack Layer**: 10+ specialized data source integrations
- **Integration Layer**: External service clients and utilities

### **Comprehensive Haystack Ecosystem**
The system supports 10 different haystack types, each optimized for specific data sources:

**Core Haystacks** (6):
- **Ripgrep**: Local filesystem search
- **Atomic**: Atomic Data Protocol integration
- **QueryRs**: Rust documentation and Reddit
- **ClickUp**: Task and project management
- **MCP**: Model Context Protocol tools
- **Perplexity**: AI-powered web search

**Extended Haystacks** (4):
- **Google Docs**: Google Workspace integration
- **Atlassian**: Jira and Confluence
- **Discourse**: Forum and community content
- **JMAP**: Email integration

### **Advanced Relevance Scoring**
Multiple scoring algorithms for optimal search results:
- **TitleScorer**: Basic title-based matching
- **BM25 Family**: Statistical relevance (BM25, BM25F, BM25Plus)
- **TerraphimGraph**: Knowledge graph-based semantic ranking

### **Privacy-First Design**
- Local processing capabilities
- Optional cloud features
- Secure credential management
- Role-based access control

### **Extensibility and Modularity**
- Modular crate architecture
- Plugin-based haystack system
- Configurable role-based access
- Multiple deployment options

This architecture provides a comprehensive view of the Terraphim AI system, showing how components interact to deliver a privacy-first, locally-operated AI assistant with advanced semantic search capabilities across multiple data sources and use cases.
