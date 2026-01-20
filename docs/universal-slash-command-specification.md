# Universal Slash Command Specification

## Executive Summary

This specification defines a framework-agnostic universal slash command system that translates the current Svelte/TipTap implementation to work across multiple editor frameworks including Zed (GPUI), Novel, and future editors. The specification maintains the rich functionality of the current implementation while providing extensible patterns for cross-framework compatibility.

### Current State Analysis

Based on disciplined research of the current Svelte implementation:

**NovelWrapper.svelte** (`/desktop/src/lib/Editor/NovelWrapper.svelte`):
- **Core Architecture**: TipTap-based editor with ProseMirror extensions
- **Configuration**: Extensive props for triggers, debounce, limits, and backend options
- **Backend Integration**: Dual Tauri/MCP support with connection testing and status management
- **State Management**: Svelte 5 runes (`$state`, `$effect`, `$props`) for reactive patterns
- **Output Formats**: Support for both HTML and Markdown output

**SlashCommand.ts** (`/desktop/src/lib/Editor/SlashCommand.ts`):
- **Command Definition**: Static array of `CommandItem` objects with title, icon, subtitle, and run function
- **Trigger System**: Single-character trigger ('/') with `startOfLine: true` requirement
- **Rendering**: Custom DOM-based renderer with Tippy.js for positioning
- **Navigation**: Arrow keys, Enter/Tab selection, Escape cancellation
- **Styling**: Comprehensive CSS with dark theme support

**TerraphimSuggestion.ts** (`/desktop/src/lib/Editor/TerraphimSuggestion.ts`):
- **Async Integration**: Full async suggestion fetching with debouncing
- **Backend Service**: Integration with `novelAutocompleteService` for knowledge graph suggestions
- **Rich UI**: Headers, snippets, scores, and comprehensive feedback
- **Error Handling**: Graceful fallbacks and empty state handling

### Zed Editor Capabilities Analysis

**Extension System** ([Zed Extensions Documentation](https://zed.dev/blog/zed-decoded-extensions)):
- **Technology Stack**: WIT (WebAssembly Interface Types) + Wasm + Wasmtime
- **Pattern**: WIT definitions compiled to Rust types for Wasm compatibility
- **Async Support**: Full async interaction capabilities between editor and extensions

**Slash Command Support** ([Slash Command Extensions](https://zed.dev/docs/extensions/slash-commands)):
- **Registration**: TOML-based configuration in `extension.toml`
- **Implementation**: `run_slash_command` method with structured input/output
- **Auto-completion**: `complete_slash_command_argument` for argument completion
- **Output Format**: Rich text with optional sections for "crease rendering"

### Key Objectives

- **Framework Agnostic**: Define interfaces that work across Svelte, GPUI, Zed, and other editors
- **Performance Optimized**: Maintain sub-100ms response times for autocomplete and command execution
- **Extensible**: Support custom commands, providers, and suggestion sources
- **Accessible**: Ensure keyboard navigation and screen reader compatibility
- **Type Safe**: Provide strong typing for command definitions and responses
- **Backward Compatible**: Maintain existing functionality during migration

## 1. Core Interfaces

### 1.1 Universal Command Definition

```typescript
interface UniversalCommand {
  // Core identification
  id: string;
  title: string;
  subtitle?: string;
  description?: string;

  // Visual representation
  icon?: string | IconData;
  category?: CommandCategory;
  keywords?: string[];

  // Execution
  execute: (context: CommandContext) => Promise<CommandResult> | CommandResult;

  // Availability control
  when?: string; // VSCode-style when clause
  enabled?: boolean;

  // Metadata
  source: CommandSource;
  priority?: number;
  aliases?: string[];
}
```

### 1.2 Command Context

```typescript
interface CommandContext {
  // Editor state
  editor: EditorAdapter;
  selection?: TextSelection;
  cursor: CursorPosition;

  // Application state
  currentRole: string;
  activeView: string;

  // User input
  query: string;
  trigger: TriggerInfo;

  // Services
  services: ServiceRegistry;

  // Metadata
  timestamp: number;
  sessionId: string;
}
```

### 1.3 Command Result

```typescript
interface CommandResult {
  success: boolean;
  error?: CommandError;

  // Content changes
  content?: ContentChange;
  selection?: TextSelection;

  // UI changes
  view?: ViewChange;
  notification?: Notification;

  // Metadata
  duration?: number;
  telemetry?: TelemetryData;
}
```

### 1.4 Suggestion System

```typescript
interface SuggestionProvider {
  id: string;
  name: string;

  // Suggestion logic
  provideSuggestions: (
    query: SuggestionQuery
  ) => Promise<SuggestionResponse>;

  // Configuration
  trigger: SuggestionTrigger;
  debounce?: number;
  minQueryLength?: number;
  maxResults?: number;

  // State management
  isEnabled(): boolean;
  activate(): Promise<void>;
  deactivate(): Promise<void>;
}

interface SuggestionQuery {
  text: string;
  context: SuggestionContext;
  position: CursorPosition;
  trigger: TriggerInfo;
  limit?: number;
}

interface SuggestionResponse {
  suggestions: Suggestion[];
  hasMore?: boolean;
  total?: number;
  processingTime?: number;
}
```

## 2. Suggestion System Architecture

### 2.1 Provider Registration

```typescript
class SuggestionRegistry {
  private providers: Map<string, SuggestionProvider> = new Map();
  private activeProviders: Set<string> = new Set();

  register(provider: SuggestionProvider): void;
  unregister(providerId: string): void;
  activate(providerId: string): Promise<void>;
  deactivate(providerId: string): Promise<void>;

  async getSuggestions(query: SuggestionQuery): Promise<SuggestionResponse>;
  getActiveProviders(): SuggestionProvider[];
}
```

### 2.2 Built-in Providers

#### Knowledge Graph Provider
```typescript
class KnowledgeGraphProvider implements SuggestionProvider {
  id = 'kg-autocomplete';
  name = 'Knowledge Graph Autocomplete';
  trigger = { type: 'auto', minChars: 2 };
  debounce = 300;
  minQueryLength = 2;
  maxResults = 8;

  async provideSuggestions(query: SuggestionQuery): Promise<SuggestionResponse> {
    // Implementation based on current KGSearchInput logic
    const suggestions = await this.getKGSuggestions(
      query.text,
      query.context.currentRole
    );

    return {
      suggestions: suggestions.map(this.mapToUniversalSuggestion),
      processingTime: Date.now() - query.context.timestamp
    };
  }
}
```

#### Command Palette Provider
```typescript
class CommandPaletteProvider implements SuggestionProvider {
  id = 'command-palette';
  name = 'Command Palette';
  trigger = { type: 'manual', chars: ['/', '>'] };

  async provideSuggestions(query: SuggestionQuery): Promise<SuggestionResponse> {
    const commands = await this.getMatchingCommands(query.text);

    return {
      suggestions: commands.map(cmd => ({
        id: cmd.id,
        text: cmd.title,
        description: cmd.description,
        icon: cmd.icon,
        category: cmd.category,
        action: { type: 'execute', data: cmd }
      }))
    };
  }
}
```

#### Terraphim Suggestion Provider
```typescript
class TerraphimSuggestionProvider implements SuggestionProvider {
  id = 'terraphim-suggestion';
  name = 'Terraphim Autocomplete';
  trigger = { type: 'char', char: '++' };
  debounce = 200;
  minQueryLength = 1;

  async provideSuggestions(query: SuggestionQuery): Promise<SuggestionResponse> {
    // Based on current TerraphimSuggestion implementation
    const response = await this.fetchSuggestions(
      query.text,
      query.context.currentRole,
      5 // limit
    );

    return {
      suggestions: response.suggestions.map(sugg => ({
        id: sugg.id,
        text: sugg.term,
        snippet: sugg.snippet,
        score: sugg.score,
        action: { type: 'insert', text: sugg.completion }
      }))
    };
  }
}
```

## 3. Trigger System

### 3.1 Trigger Types

```typescript
type TriggerType =
  | 'char'        // Specific character sequence (e.g., "++", "/")
  | 'manual'      // Explicit invocation (Ctrl+P, Cmd+P)
  | 'auto'        // Automatic based on typing
  | 'context'     // Based on cursor context
  | 'debounced';  // Delayed automatic

interface TriggerInfo {
  type: TriggerType;
  char?: string;
  delay?: number;
  context?: string;
  position: CursorPosition;
}
```

### 3.2 Trigger Configuration

```typescript
interface TriggerConfig {
  // Character-based triggers
  charTriggers: Map<string, ProviderId[]>;

  // Manual invocation
  manualKeybindings: Keybinding[];

  // Auto-trigger settings
  autoTrigger: {
    enabled: boolean;
    minChars: number;
    debounce: number;
    excludePatterns: string[];
  };

  // Context triggers
  contextTriggers: ContextRule[];
}
```

### 3.3 Debouncing Strategy

```typescript
class DebounceManager {
  private timers: Map<string, NodeJS.Timeout> = new Map();

  debounce<T>(
    key: string,
    fn: () => Promise<T>,
    delay: number
  ): Promise<T> {
    return new Promise((resolve, reject) => {
      // Clear existing timer
      if (this.timers.has(key)) {
        clearTimeout(this.timers.get(key)!);
      }

      // Set new timer
      const timer = setTimeout(async () => {
        try {
          const result = await fn();
          this.timers.delete(key);
          resolve(result);
        } catch (error) {
          this.timers.delete(key);
          reject(error);
        }
      }, delay);

      this.timers.set(key, timer);
    });
  }
}
```

## 4. Error Handling & Fallbacks

### 4.1 Error Types

```typescript
enum CommandErrorType {
  PROVIDER_ERROR = 'provider_error',
  NETWORK_ERROR = 'network_error',
  TIMEOUT_ERROR = 'timeout_error',
  INVALID_QUERY = 'invalid_query',
  PERMISSION_DENIED = 'permission_denied',
  NOT_AVAILABLE = 'not_available'
}

interface CommandError {
  type: CommandErrorType;
  message: string;
  details?: any;
  recoverable: boolean;
  suggestions?: string[];
}
```

### 4.2 Fallback Strategy

```typescript
interface FallbackConfig {
  primaryProvider: string;
  fallbackProviders: string[];
  retryAttempts: number;
  retryDelay: number;
  timeoutMs: number;
}

class ResilientSuggestionService {
  async getSuggestionsWithFallback(
    query: SuggestionQuery,
    config: FallbackConfig
  ): Promise<SuggestionResponse> {
    const providers = [config.primaryProvider, ...config.fallbackProviders];

    for (const providerId of providers) {
      try {
        const provider = this.registry.getProvider(providerId);
        if (provider && provider.isEnabled()) {
          const response = await this.withTimeout(
            provider.provideSuggestions(query),
            config.timeoutMs
          );

          if (response.suggestions.length > 0) {
            return response;
          }
        }
      } catch (error) {
        console.warn(`Provider ${providerId} failed:`, error);
        // Continue to next provider
      }
    }

    // Return empty response if all providers fail
    return { suggestions: [] };
  }
}
```

## 5. Performance Requirements

### 5.1 Response Time Targets

| Operation | Target | Maximum |
|-----------|--------|---------|
| Command palette open | 50ms | 100ms |
| Autocomplete suggestions | 100ms | 200ms |
| Command execution | 200ms | 500ms |
| Large suggestion lists | 150ms | 300ms |

### 5.2 Caching Strategy

```typescript
interface SuggestionCache {
  // LRU cache for recent queries
  recentQueries: LRUCache<string, SuggestionResponse>;

  // Pre-computed command index
  commandIndex: Map<string, UniversalCommand[]>;

  // Provider-specific caches
  providerCaches: Map<string, ProviderCache>;
}

class PerformanceOptimizer {
  private cache: SuggestionCache;
  private metrics: PerformanceMetrics;

  async getOptimizedSuggestions(
    query: SuggestionQuery
  ): Promise<SuggestionResponse> {
    // Check cache first
    const cacheKey = this.getCacheKey(query);
    const cached = this.cache.recentQueries.get(cacheKey);

    if (cached && !this.isCacheStale(cached)) {
      this.metrics.recordCacheHit();
      return cached;
    }

    // Measure performance
    const startTime = performance.now();
    const response = await this.registry.getSuggestions(query);
    const duration = performance.now() - startTime;

    // Cache successful responses
    if (response.suggestions.length > 0) {
      this.cache.recentQueries.set(cacheKey, {
        ...response,
        timestamp: Date.now()
      });
    }

    this.metrics.recordQuery(duration, response.suggestions.length);
    return response;
  }
}
```

### 5.3 Virtual Scrolling

For large suggestion sets (>50 items):

```typescript
interface VirtualScrollConfig {
  itemHeight: number;
  visibleItems: number;
  bufferSize: number;
  overscan: number;
}

class VirtualSuggestionList {
  renderItems(
    suggestions: Suggestion[],
    scrollTop: number,
    containerHeight: number
  ): RenderedItem[] {
    const startIndex = Math.floor(scrollTop / this.config.itemHeight);
    const endIndex = Math.min(
      startIndex + this.config.visibleItems + this.config.overscan,
      suggestions.length
    );

    return suggestions
      .slice(startIndex, endIndex)
      .map((suggestion, index) => ({
        suggestion,
        index: startIndex + index,
        top: (startIndex + index) * this.config.itemHeight
      }));
  }
}
```

## 6. Testing & Validation

### 6.1 Test Categories

```typescript
interface TestSuite {
  unit: UnitTestSuite;
  integration: IntegrationTestSuite;
  performance: PerformanceTestSuite;
  accessibility: AccessibilityTestSuite;
}

interface PerformanceTestSuite {
  benchmarks: Benchmark[];
  loadTests: LoadTest[];
  memoryTests: MemoryTest[];
}
```

### 6.2 Test Requirements

#### Unit Tests
- Command registration and execution
- Suggestion provider logic
- Trigger detection and handling
- Error handling scenarios

#### Integration Tests
- Cross-framework compatibility
- Service integration (MCP, Tauri, REST API)
- State management consistency
- Event handling

#### Performance Tests
- Autocomplete response time <100ms
- Memory usage under load
- Virtual scrolling performance
- Cache effectiveness

#### Accessibility Tests
- Keyboard navigation completeness
- Screen reader compatibility
- Focus management
- ARIA label correctness

### 6.3 Validation Framework

```typescript
class ValidationFramework {
  async validateImplementation(
    implementation: CommandSystemImplementation
  ): Promise<ValidationReport> {
    const results = await Promise.all([
      this.validateInterfaceCompliance(implementation),
      this.validatePerformanceRequirements(implementation),
      this.validateAccessibilityRequirements(implementation),
      this.validateErrorHandling(implementation)
    ]);

    return this.combineResults(results);
  }
}
```

## 7. Implementation Guides

### 7.1 Svelte to Universal Migration

#### Current Svelte Pattern
```svelte
<!-- Current implementation -->
<script>
import { SlashCommand } from './SlashCommand';

let editor = $state(null);
</script>

<Editor
  extensions={[
    SlashCommand.configure({
      trigger: '/',
      items: DEFAULT_ITEMS
    })
  ]}
/>
```

#### Universal Pattern
```typescript
// Universal implementation
const commandSystem = new UniversalCommandSystem({
  providers: [
    new CommandPaletteProvider(),
    new KnowledgeGraphProvider(),
    new TerraphimSuggestionProvider()
  ],
  triggers: {
    charTriggers: new Map([
      ['/', ['command-palette']],
      ['++', ['terraphim-suggestion']]
    ]),
    autoTrigger: {
      enabled: true,
      minChars: 2,
      debounce: 300
    }
  }
});

// Framework adapter for Svelte
const svelteAdapter = new SvelteCommandAdapter(commandSystem);
```

### 7.2 GPUI Integration

```rust
// GPUI implementation
pub struct UniversalCommandSystem {
    providers: Vec<Box<dyn SuggestionProvider>>,
    registry: SuggestionRegistry,
    debouncer: DebounceManager,
}

impl UniversalCommandSystem {
    pub fn new() -> Self {
        Self {
            providers: vec![
                Box::new(KnowledgeGraphProvider::new()),
                Box::new(CommandPaletteProvider::new()),
            ],
            registry: SuggestionRegistry::new(),
            debouncer: DebounceManager::new(),
        }
    }

    pub async fn get_suggestions(
        &self,
        query: SuggestionQuery,
    ) -> Result<SuggestionResponse, CommandError> {
        self.debouncer.debounce(
            "suggestions",
            || self.registry.get_suggestions(query),
            300,
        ).await
    }
}
```

### 7.3 Zed Editor Integration

```rust
// Zed editor plugin
pub struct TerraphimCommandPlugin;

impl ZedPlugin for TerraphimCommandPlugin {
    fn new() -> Self {
        Self
    }

    fn initialize(&mut self, workspace: &mut Workspace) {
        // Register universal command system
        let command_system = UniversalCommandSystem::new();

        // Register Zed-specific adapters
        workspace.register_action(Box::new(ShowCommandPalette::new(command_system)));
        workspace.register_action(Box::new(TriggerAutocomplete::new(command_system)));
    }
}
```

## 8. Migration Strategy

### 8.1 Phase 1: Interface Definition (Week 1-2)
- Define universal interfaces and types
- Create validation framework
- Establish testing patterns

### 8.2 Phase 2: Core Implementation (Week 3-4)
- Implement suggestion registry and providers
- Create trigger system with debouncing
- Build error handling and fallback mechanisms

### 8.3 Phase 3: Framework Adapters (Week 5-6)
- Svelte adapter (migrate existing implementation)
- GPUI adapter (new implementation)
- Zed editor adapter (plugin development)

### 8.4 Phase 4: Testing & Optimization (Week 7-8)
- Comprehensive testing suite
- Performance optimization
- Accessibility validation

### 8.5 Phase 5: Documentation & Deployment (Week 9-10)
- Implementation guides
- Migration documentation
- Production deployment

## 9. Conclusion

This universal slash command specification provides a comprehensive framework for implementing consistent, performant command palettes and autocomplete systems across multiple editor platforms. By abstracting the current Svelte implementation into universal interfaces, we enable:

- **Code Reuse**: Shared logic across frameworks
- **Consistent UX**: Uniform behavior regardless of editor
- **Performance**: Optimized caching and debouncing strategies
- **Extensibility**: Easy addition of new providers and commands
- **Maintainability**: Centralized business logic with framework-specific adapters

The specification maintains the rich functionality of the current implementation while providing a clear migration path to universal interfaces that support future editor integrations.

---

**Document Version**: 1.0
**Last Updated**: 2025-01-22
**Next Review**: 2025-02-22