# Terraphim AI Assistant

Terraphim is a privacy-first AI assistant that operates locally on user hardware, providing semantic search across multiple knowledge repositories without compromising data privacy.

## Core Features
- **Local-first**: Runs entirely on user's infrastructure
- **Multi-source search**: Integrates personal files, team repositories, and public sources (StackOverflow, GitHub)
- **Knowledge graphs**: Creates structured graphs from document collections (haystacks)
- **Role-based contexts**: Different AI personas (developer, system operator, etc.) with specialized knowledge
- **Multiple interfaces**: Web UI, desktop app (Tauri), terminal interface (TUI)

## Key Concepts
- **Haystack**: A data source that can be searched (folder, Notion workspace, email)
- **Knowledge Graph**: Structured representation of information with entities and relationships
- **Profile**: Endpoint for persisting user data (S3, sled, rocksdb)
- **Role**: AI assistant configuration with specialized behavior and knowledge
- **Rolegraph**: Knowledge graph structure for document ingestion and result ranking

## Target Users
- Developers seeking code-related information across repositories
- Knowledge workers managing multiple information sources
- Privacy-conscious users wanting local AI assistance
- Teams needing unified search across documentation systems