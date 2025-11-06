# Summary: lessons-learned.md

## Purpose
Captures critical technical insights, development patterns, and lessons from Terraphim AI implementation. Serves as knowledge repository for repeatable patterns and anti-patterns to avoid.

## Key Functionality
- **Pattern Discovery**: Documents successful implementation patterns for reuse
- **Anti-patterns**: Records mistakes and their solutions for future reference
- **Technology Insights**: Lessons about tools, frameworks, deployment strategies
- **Decision Context**: Explains why certain technical approaches were chosen
- **Best Practices**: Establishes project-specific development guidelines

## Critical Lessons

**Pattern 1: Pattern Discovery Through Reading Existing Code**
- **Context**: Deployment infrastructure implementation
- **Learning**: Always read existing scripts before creating new infrastructure
- **Example**: Reading `deploy-to-bigbox.sh` revealed correct Caddy+rsync pattern (not Docker/nginx)
- **When to Apply**: Any new feature deployment, integration with existing infrastructure

**Pattern 2: Vanilla JavaScript over Framework for Simple UIs**
- **Context**: TruthForge UI implementation
- **Learning**: No build step = instant deployment; check project patterns before choosing technology
- **Implementation**: Class-based separation (TruthForgeClient, TruthForgeUI), progressive enhancement
- **Benefits**: Static files work immediately, no compilation required

**Pattern 3: Rsync + Caddy Deployment Pattern**
- **Context**: Bigbox infrastructure deployment
- **Learning**: Project uses rsync for file copying, Caddy for reverse proxy (not Docker/nginx)
- **Steps**: Copy files → Configure Caddy → Update endpoints → Start backend → Verify
- **Example**: `alpha.truthforge.terraphim.cloud` deployment

**Pattern 4: 1Password CLI for Secret Management**
- **Context**: Production secret injection
- **Learning**: Use `op run --env-file=.env` in systemd services, never commit secrets
- **Benefits**: Centralized management, audit trail, automatic rotation
- **Implementation**: `.env` file contains vault references (`op://Shared/Key/field`)

**Pattern 5: Test-Driven Security Implementation**
- **Context**: Security vulnerability fixes
- **Learning**: Write tests first for security issues, then implement fixes
- **Categories**: Prompt injection, command injection, memory safety, network validation
- **Coverage**: 99 comprehensive tests across multiple attack vectors

## Technical Insights

**UI Development**:
- WebSocket client reusability from shared libraries (agent-workflows/shared/)
- Protocol-aware URL configuration for file:// vs http:// protocols
- Fallback mechanisms for graceful degradation

**Security**:
- Defense-in-depth patterns with multiple validation layers
- Unicode obfuscation detection critical for prompt sanitizers
- Concurrent security testing required for production readiness
- Regex catastrophic backtracking prevention in validation

**Deployment**:
- Phase-based deployment makes debugging easier (copy, configure, update, verify)
- Caddy reverse proxy with automatic HTTPS and zero-downtime reloads
- Systemd services with proper EnvironmentFile for secret loading

**Testing**:
- Browser automation (Playwright) for E2E validation
- Protocol validation prevents future regressions
- Comprehensive test suites build confidence in changes

## Anti-Patterns to Avoid
- Assuming Docker deployment without checking existing patterns
- Creating new infrastructure without reading existing scripts
- Using frameworks when vanilla JS suffices for simple UIs
- Storing secrets in .env files or environment variables
- Skipping security tests for "simple" changes
- Using blocking operations in async functions

## When to Apply These Lessons
- **Pattern Discovery**: Beginning any new feature or integration
- **Vanilla JS**: Building simple UIs, demos, or examples
- **Deployment Pattern**: Any production deployment or service configuration
- **Secret Management**: All production deployments with sensitive data
- **Security Testing**: Any code handling user input, system commands, or network operations
