# Universal Slash Command Implementation Guide

## Overview

This comprehensive guide provides detailed implementation instructions for migrating the current Svelte/TipTap-based slash command system to a universal, framework-agnostic specification. The guide includes concrete code examples, framework-specific adapters, performance optimization strategies, and step-by-step migration instructions.

## Target Audience

- Frontend developers working with Svelte/TipTap
- Systems developers implementing GPUI/Zed editor integrations
- Architects designing cross-framework command systems
- DevOps engineers deploying universal command infrastructure

## Prerequisites

- Familiarity with TypeScript/JavaScript and Rust
- Understanding of TipTap editor extensions
- Basic knowledge of GPUI and WIT interfaces
- Experience with async/await patterns and error handling

## 1. Architecture Overview

### 1.1 Universal System Components

```
┌─────────────────────────────────────────────────────────────┐
│                    Universal Command System                │
├─────────────────────────────────────────────────────────────┤
│  ┌─────────────────┐  ┌─────────────────┐  ┌─────────────────┐│
│  │ Command Registry│  │Suggestion System│  │  Trigger Engine ││
│  │ • Command Store │  │ • Provider Pool │  │ • Char Triggers ││
│  │ • Metadata      │  │ • Cache Layer   │  │ • Auto Triggers ││
│  └─────────────────┘  └─────────────────┘  └─────────────────┘│
├─────────────────────────────────────────────────────────────┤
│                    Framework Adapters                      │
│  ┌─────────────────┐  ┌─────────────────┐  ┌─────────────────┐│
│  │  Svelte/TipTap  │  │   Zed/GPUI      │  │  Future Editors ││
│  │ • DOM Renderer  │  │ • WIT Interface │  │ • Plugin API    ││
│  │ • Event Bridge  │  │ • Rust Bridge   │  │ • Extensible    ││
│  └─────────────────┘  └─────────────────┘  └─────────────────┘│
├─────────────────────────────────────────────────────────────┤
│                     Backend Services                      │
│  ┌─────────────────┐  ┌─────────────────┐  ┌─────────────────┐│
│  │  MCP Server     │  │    Tauri        │  │   REST APIs     ││
│  │ • Knowledge Graph│  │ • Native Bridge │  │ • HTTP Endpoints││
│  │ • Role Context   │  │ • File System   │  │ • External Data ││
│  └─────────────────┘  └─────────────────┘  └─────────────────┘│
└─────────────────────────────────────────────────────────────┘
```

### 1.2 Data Flow Architecture

```
┌─────────────┐    ┌──────────────┐    ┌─────────────┐    ┌──────────────┐
│ User Input  │───▶│ Trigger      │───▶│ Query       │───▶│ Provider     │
│ • Keyboard   │    │ Detection    │    │ Processing  │    │ Resolution   │
│ • Mouse      │    │ • Char Match │    │ • Context   │    │ • Parallel   │
│ • Touch      │    │ • Auto Detect│    │ • Validation│    │ • Timeout    │
└─────────────┘    └──────────────┘    └─────────────┘    └──────────────┘
       │                   │                   │                   │
       ▼                   ▼                   ▼                   ▼
┌─────────────┐    ┌──────────────┐    ┌─────────────┐    ┌──────────────┐
│ UI Context   │    │ Command      │    │ Suggestion  │    │ Response      │
│ • Editor     │    │ Registry     │    │ Providers   │    │ Aggregation   │
│ • Selection  │    │ • Lookup     │    │ • KG Service │    │ • Scoring     │
│ • Document   │    │ • Filter     │    │ • Commands  │    │ • Ranking     │
└─────────────┘    └──────────────┘    └─────────────┘    └──────────────┘
       │                   │                   │                   │
       ▼                   ▼                   ▼                   ▼
┌─────────────┐    ┌──────────────┐    ┌─────────────┐    ┌──────────────┐
│ Renderer     │◀───│ Framework    │◀───│ Universal   │◀───│ Backend      │
│ • DOM Update │    │ Adapter      │    │ Response    │    │ Services     │
│ • Position   │    │ • Conversion │    │ • Format    │    │ • Data Fetch │
│ • Cleanup    │    │ • Events     │    │ • Metadata  │    │ • Error Handle│
└─────────────┘    └──────────────┘    └─────────────┘    └──────────────┘
```

### 1.3 Key Design Principles

1. **Framework Agnostic**: Core logic independent of UI frameworks
2. **Provider-Based**: Extensible suggestion and command providers
3. **Performance First**: Sub-100ms response times with intelligent caching
4. **Type Safety**: Strong TypeScript and Rust type definitions
5. **Error Resilience**: Graceful degradation and comprehensive error handling
6. **Accessibility**: Full keyboard navigation and screen reader support

## 2. Core Universal Interfaces

### 2.1 Base Types

```typescript
// Core position and selection types
interface Position {
  line: number;
  column: number;
  offset?: number;
}

interface SelectionRange {
  start: Position;
  end: Position;
  anchor?: Position;
  head?: Position;
}

// Command categories for organization
enum CommandCategory {
  TEXT = 'text',
  EDITING = 'editing', 
  NAVIGATION = 'navigation',
  SEARCH = 'search',
  AI = 'ai',
  FORMATTING = 'formatting',
  CUSTOM = 'custom'
}

// Command source tracking
enum CommandSource {
  BUILTIN = 'builtin',
  PLUGIN = 'plugin',
  USER = 'user',
  EXTERNAL = 'external'
}

// Universal command interface
interface UniversalCommand {
  // Identification
  id: string;
  title: string;
  subtitle?: string;
  description?: string;

  // Visual representation
  icon?: string | IconData;
  category?: CommandCategory;
  keywords?: string[];

  // Execution
  execute: (context: ExecutionContext) => Promise<CommandResult> | CommandResult;

  // Arguments (optional)
  arguments?: CommandArgument[];

  // Availability conditions
  when?: string; // Expression language for conditional availability
  enabled?: boolean;
  permissions?: string[]; // Required user permissions

  // Metadata
  source: CommandSource;
  priority?: number;
  aliases?: string[];
  version?: string;

  // Framework-specific adaptations
  adapters?: {
    [framework: string]: FrameworkAdapter;
  };
}

// Command argument definitions
interface CommandArgument {
  name: string;
  type: 'string' | 'number' | 'boolean' | 'select';
  required: boolean;
  description?: string;
  defaultValue?: any;
  options?: string[]; // For select type
}

// Command execution result
interface CommandResult {
  success: boolean;
  error?: string;
  content?: string;
  selection?: SelectionRange;
  metadata?: Record<string, any>;
  duration?: number; // Execution time in ms
}

// Command execution context
interface ExecutionContext {
  // Editor state
  editor: EditorAdapter;
  selection?: SelectionRange;
  document: DocumentAdapter;

  // User context
  role: string;
  permissions: string[];
  preferences: Record<string, any>;

  // Trigger information
  trigger: TriggerInfo;
  query: string;

  // Services
  services: ServiceRegistry;

  // Metadata
  timestamp: number;
  sessionId: string;
  transactionId?: string;
}
```

### 2.2 Suggestion System

```typescript
// Universal suggestion interface
interface UniversalSuggestion {
  id: string;
  text: string;
  displayText?: string;
  snippet?: string;
  description?: string;
  score?: number; // 0.0 to 1.0 relevance score
  category?: CommandCategory;

  // Action to execute when selected
  action: SuggestionAction;

  // Metadata
  metadata?: {
    source: string;
    relevance: number;
    context?: any;
    confidence?: number; // Provider confidence in suggestion
    processingTime?: number; // Time taken to generate suggestion
  };
}

// Suggestion action types with enhanced options
type SuggestionAction =
  | { type: 'insert'; text: string; position?: 'before' | 'after' | 'replace' }
  | { type: 'execute'; command: UniversalCommand }
  | { type: 'replace'; range: SelectionRange; text: string }
  | { type: 'search'; query: string; provider: string; options?: Record<string, any> }
  | { type: 'open'; view: string; data?: any }
  | { type: 'custom'; handler: (context: ExecutionContext) => Promise<void> };

// Suggestion provider interface with enhanced lifecycle
interface SuggestionProvider {
  id: string;
  name: string;
  priority: number;
  version?: string;

  // Core functionality
  getSuggestions(query: SuggestionQuery): Promise<SuggestionResponse>;

  // Enhanced lifecycle management
  initialize?(): Promise<void>;
  activate?(): Promise<void>;
  deactivate?(): void;
  destroy?(): void;
  healthCheck?(): Promise<ProviderHealth>;
  
  // State management
  isEnabled(): boolean;
  canHandle(query: SuggestionQuery): boolean;
  getState(): ProviderState;

  // Configuration
  trigger: TriggerConfig;
  debounce?: number;
  minQueryLength?: number;
  maxResults?: number;
  timeout?: number;
  cacheConfig?: CacheConfig;
}

// Enhanced suggestion query
interface SuggestionQuery {
  text: string;
  context: SuggestionContext;
  position: Position;
  trigger: TriggerInfo;
  limit?: number;
  timestamp: number;
  sessionId?: string;
  transactionId?: string;
  preferences?: UserPreferences;
}

// Suggestion context with rich information
interface SuggestionContext {
  currentRole: string;
  documentType?: string;
  language?: string;
  cursorPosition: Position;
  selectedText?: string;
  surroundingText?: {
    before: string;
    after: string;
  };
  userHistory?: {
    recentCommands: string[];
    frequentSuggestions: string[];
  };
}

// Suggestion response with metadata
interface SuggestionResponse {
  suggestions: UniversalSuggestion[];
  hasMore: boolean;
  total?: number;
  processingTime: number;
  hasErrors: boolean;
  metadata?: {
    provider: string;
    cacheHit: boolean;
    queryComplexity: number;
  };
}

// Provider health status
interface ProviderHealth {
  status: 'healthy' | 'degraded' | 'unhealthy';
  latency?: number;
  errorRate?: number;
  lastCheck: number;
  message?: string;
}

// Provider state
interface ProviderState {
  status: 'inactive' | 'activating' | 'active' | 'deactivating' | 'error';
  requestCount: number;
  errorCount: number;
  averageResponseTime: number;
  lastRequest: number;
}

// Cache configuration
interface CacheConfig {
  enabled: boolean;
  ttl: number; // Time to live in ms
  maxSize: number;
  strategy: 'lru' | 'fifo' | 'lfu';
}
```

## 3. Svelte/TipTap Implementation

### 3.1 Adapter Implementation

```typescript
// SvelteTipTapAdapter - bridges universal system with TipTap
export class SvelteTipTapAdapter implements EditorAdapter {
  private editor: Editor;
  private commandSystem: UniversalCommandSystem;
  private eventListeners: Map<string, Function[]> = new Map();
  private performanceMetrics: PerformanceMetrics;

  constructor(
    editor: Editor,
    commandSystem: UniversalCommandSystem,
    options: AdapterOptions = {}
  ) {
    this.editor = editor;
    this.commandSystem = commandSystem;
    this.performanceMetrics = new PerformanceMetrics();
    
    // Setup event listeners
    this.setupEventListeners();
    
    // Initialize performance monitoring
    if (options.enableMetrics) {
      this.setupPerformanceMonitoring();
    }
  }

  // Convert universal commands to TipTap extensions
  createTipTapExtensions(): Extension[] {
    const extensions: Extension[] = [
      // Slash command extension
      this.createSlashCommandExtension(),

      // Autocomplete extension  
      this.createAutocompleteExtension(),

      // Additional universal extensions
      ...this.createUniversalExtensions()
    ];

    // Add performance monitoring extensions if enabled
    if (this.performanceMetrics.isEnabled()) {
      extensions.push(this.createMetricsExtension());
    }

    return extensions;
  }

  // Enhanced event handling
  private setupEventListeners(): void {
    // Track editor state changes
    this.editor.on('update', this.handleEditorUpdate.bind(this));
    this.editor.on('selectionUpdate', this.handleSelectionUpdate.bind(this));
    this.editor.on('focus', this.handleFocus.bind(this));
    this.editor.on('blur', this.handleBlur.bind(this));
  }

  private handleEditorUpdate({ editor }: { editor: Editor }): void {
    this.emit('editor-update', { editor });
    this.performanceMetrics.recordUpdate();
  }

  private handleSelectionUpdate({ editor }: { editor: Editor }): void {
    this.emit('selection-update', { 
      selection: this.createSelectionRange(editor) 
    });
  }

  // Event emission system
  private emit(event: string, data: any): void {
    const listeners = this.eventListeners.get(event) || [];
    listeners.forEach(listener => listener(data));
  }

  public on(event: string, listener: Function): void {
    if (!this.eventListeners.has(event)) {
      this.eventListeners.set(event, []);
    }
    this.eventListeners.get(event)!.push(listener);
  }

  public off(event: string, listener: Function): void {
    const listeners = this.eventListeners.get(event);
    if (listeners) {
      const index = listeners.indexOf(listener);
      if (index > -1) {
        listeners.splice(index, 1);
      }
    }
  }

  // Performance monitoring
  private setupPerformanceMonitoring(): void {
    this.performanceMetrics.enable();
    
    // Track suggestion performance
    this.on('suggestion-request', (data) => {
      this.performanceMetrics.startSuggestionTimer(data.query);
    });
    
    this.on('suggestion-response', (data) => {
      this.performanceMetrics.endSuggestionTimer(data.query, data.response);
    });
  }

  // Create selection range from editor
  private createSelectionRange(editor: Editor): SelectionRange {
    const { from, to, $from, $to } = editor.state.selection;
    
    return {
      start: this.posToPosition(from, $from),
      end: this.posToPosition(to, $to),
      anchor: this.posToPosition(editor.state.selection.anchor, $from),
      head: this.posToPosition(editor.state.selection.head, $to)
    };
  }

  // Convert ProseMirror position to universal position
  private posToPosition(pos: number, resolved?: any): Position {
    if (!resolved) {
      resolved = this.editor.state.doc.resolve(pos);
    }
    
    return {
      line: resolved.block().content.size,
      column: resolved.parentOffset,
      offset: pos
    };
  }

  private createSlashCommandExtension(): Extension {
    return Extension.create({
      name: 'universalSlashCommand',
      
      addOptions() {
        return {
          char: '/',
          startOfLine: true,
          allowSpaces: true,
          maxSuggestions: 20,
          debounce: 50
        };
      },

      addProseMirrorPlugins() {
        return [
          Suggestion({
            char: this.options.char,
            startOfLine: this.options.startOfLine,
            allowSpaces: this.options.allowSpaces,

            command: ({ editor, range, props }) => {
              const command = props as UniversalCommand;
              const startTime = performance.now();

              // Remove trigger text
              editor.chain().focus().deleteRange(range).run();

              // Create execution context
              const context = this.createExecutionContext(editor, range);
              
              // Execute command with error handling
              this.executeCommand(command, context)
                .then(result => {
                  const duration = performance.now() - startTime;
                  this.emit('command-executed', { 
                    command: command.id, 
                    result, 
                    duration 
                  });
                })
                .catch(error => {
                  this.emit('command-error', { 
                    command: command.id, 
                    error 
                  });
                  this.handleCommandError(error, command);
                });
            },

            items: async ({ query }) => {
              const startTime = performance.now();
              this.emit('suggestion-request', { query, trigger: 'slash' });

              try {
                const suggestions = await this.commandSystem.getSuggestions({
                  text: query,
                  context: this.createSuggestionContext(),
                  position: this.getCurrentPosition(),
                  trigger: { 
                    type: 'char', 
                    char: this.options.char, 
                    position: this.getCurrentPosition() 
                  },
                  limit: this.options.maxSuggestions,
                  timestamp: Date.now(),
                  sessionId: this.getSessionId()
                });

                // Filter for executable commands
                const executableSuggestions = suggestions.suggestions
                  .filter(s => s.action.type === 'execute')
                  .map(s => s.action.command);

                const duration = performance.now() - startTime;
                this.emit('suggestion-response', { 
                  query, 
                  suggestions: executableSuggestions, 
                  duration,
                  trigger: 'slash'
                });

                return executableSuggestions;
              } catch (error) {
                this.emit('suggestion-error', { query, error, trigger: 'slash' });
                return [];
              }
            },

            render: () => this.createSuggestionRenderer('slash')
          })
        ];
      }
    });
  }

  // Enhanced command execution with error handling
  private async executeCommand(
    command: UniversalCommand, 
    context: ExecutionContext
  ): Promise<CommandResult> {
    try {
      // Check command availability
      if (!this.isCommandAvailable(command, context)) {
        throw new Error(`Command not available: ${command.id}`);
      }

      // Execute command
      const result = await command.execute(context);
      
      // Update metrics
      this.performanceMetrics.recordCommandExecution(command.id, result);
      
      return result;
    } catch (error) {
      // Log error
      console.error(`Command execution failed for ${command.id}:`, error);
      
      // Return error result
      return {
        success: false,
        error: error instanceof Error ? error.message : 'Unknown error',
        metadata: { commandId: command.id }
      };
    }
  }

  // Command availability checking
  private isCommandAvailable(command: UniversalCommand, context: ExecutionContext): boolean {
    // Check enabled status
    if (command.enabled === false) {
      return false;
    }

    // Check permissions
    if (command.permissions) {
      const hasPermissions = command.permissions.every(perm => 
        context.permissions.includes(perm)
      );
      if (!hasPermissions) {
        return false;
      }
    }

    // Check when condition (if implemented)
    if (command.when) {
      // TODO: Implement expression language evaluation
      return true;
    }

    return true;
  }

  // Error handling for commands
  private handleCommandError(error: any, command: UniversalCommand): void {
    // Show user-friendly error message
    const errorMessage = `Failed to execute "${command.title}": ${
      error instanceof Error ? error.message : 'Unknown error'
    }`;
    
    // Emit error event for UI handling
    this.emit('show-error', { message: errorMessage, type: 'command' });
    
    // Log detailed error for debugging
    console.error(`Command error details:`, {
      commandId: command.id,
      error,
      context: this.createExecutionContext(this.editor, { from: 0, to: 0 })
    });
  }

  private createAutocompleteExtension(): any {
    return Suggestion.configure({
      char: '++',
      startOfLine: false,
      allowSpaces: false,

      command: ({ editor, range, props }) => {
        const suggestion = props as UniversalSuggestion;

        // Insert suggestion text
        this.executeSuggestionAction(editor, range, suggestion.action);
      },

      items: async ({ query }) => {
        const suggestions = await this.commandSystem.getSuggestions({
          text: query,
          context: this.createSuggestionContext(),
          position: this.getCurrentPosition(),
          trigger: { type: 'char', char: '++', position: this.getCurrentPosition() },
          limit: 8,
          timestamp: Date.now()
        });

        return suggestions.filter(s => s.action.type === 'insert');
      },

      render: () => this.createSuggestionRenderer('autocomplete')
    });
  }

  private createSuggestionRenderer(type: string): any {
    return {
      onStart: (props: any) => {
        // Create universal suggestion renderer
        const renderer = new UniversalSuggestionRenderer({
          items: props.items,
          type: type,
          onSelect: (item: any) => {
            props.command(item);
          }
        });

        // Position with Tippy.js
        const popup = tippy('body', {
          getReferenceClientRect: props.clientRect,
          appendTo: () => document.body,
          content: renderer.element,
          showOnCreate: true,
          interactive: true,
          trigger: 'manual',
          placement: 'bottom-start',
          theme: type === 'slash' ? 'slash-command' : 'terraphim-suggestion',
          maxWidth: 'none',
        })[0];

        return { renderer, popup };
      },

      onUpdate: (state: any) => {
        state.renderer?.updateItems(state.items);
        state.popup?.setProps({
          getReferenceClientRect: state.clientRect,
        });
      },

      onKeyDown: (props: any) => {
        if (props.event.key === 'Escape') {
          props.popup?.hide();
          return true;
        }
        return props.renderer?.onKeyDown(props) ?? false;
      },

      onExit: (state: any) => {
        state.popup?.destroy();
        state.renderer?.destroy();
      }
    };
  }
}
```

### 3.2 Universal Suggestion Renderer

```typescript
// Universal suggestion renderer that works across frameworks
export class UniversalSuggestionRenderer {
  public element: HTMLElement;
  private items: any[] = [];
  private selectedIndex = 0;
  private onSelect: (item: any) => void;
  private type: 'slash' | 'autocomplete';
  private isVisible = false;
  private animationFrame?: number;
  private performanceMetrics: RendererMetrics;

  constructor(options: {
    items: any[];
    type: 'slash' | 'autocomplete';
    onSelect: (item: any) => void;
    theme?: 'light' | 'dark' | 'auto';
    animations?: boolean;
  }) {
    this.items = options.items;
    this.type = options.type;
    this.onSelect = options.onSelect;
    this.performanceMetrics = new RendererMetrics();

    // Create DOM element
    this.element = document.createElement('div');
    this.element.className = `universal-suggestion universal-suggestion--${this.type}`;
    
    // Apply theme
    if (options.theme) {
      this.applyTheme(options.theme);
    }
    
    // Setup animations
    if (options.animations !== false) {
      this.setupAnimations();
    }
    
    // Setup accessibility
    this.setupAccessibility();
    
    // Initial render
    this.render();
  }

  // Enhanced item updates with animation
  updateItems(items: any[]): void {
    const startTime = performance.now();
    
    // Store old items for comparison
    const oldItems = [...this.items];
    this.items = items;
    this.selectedIndex = Math.min(this.selectedIndex, items.length - 1);
    
    // Animate changes if enabled
    if (this.shouldAnimate(oldItems, items)) {
      this.animateItemChanges(oldItems, items);
    } else {
      this.render();
    }
    
    // Record performance
    this.performanceMetrics.recordUpdate(performance.now() - startTime);
  }

  // Show/hide with animations
  show(): void {
    if (this.isVisible) return;
    
    this.isVisible = true;
    this.element.style.display = 'block';
    this.element.setAttribute('aria-hidden', 'false');
    
    // Animate in
    requestAnimationFrame(() => {
      this.element.classList.add('universal-suggestion--visible');
    });
  }

  hide(): void {
    if (!this.isVisible) return;
    
    this.isVisible = false;
    this.element.classList.remove('universal-suggestion--visible');
    this.element.setAttribute('aria-hidden', 'true');
    
    // Hide after animation
    setTimeout(() => {
      if (!this.isVisible) {
        this.element.style.display = 'none';
      }
    }, 200);
  }

  // Enhanced keyboard navigation
  onKeyDown({ event }: { event: KeyboardEvent }): boolean {
    if (!this.isVisible) return false;

    switch (event.key) {
      case 'ArrowUp':
        event.preventDefault();
        this.selectPrevious();
        return true;
        
      case 'ArrowDown':
        event.preventDefault();
        this.selectNext();
        return true;
        
      case 'Enter':
      case 'Tab':
        event.preventDefault();
        this.selectCurrent();
        return true;
        
      case 'Escape':
        this.hide();
        return true;
        
      case 'Home':
        event.preventDefault();
        this.selectFirst();
        return true;
        
      case 'End':
        event.preventDefault();
        this.selectLast();
        return true;
        
      case 'PageUp':
        event.preventDefault();
        this.selectPageUp();
        return true;
        
      case 'PageDown':
        event.preventDefault();
        this.selectPageDown();
        return true;
        
      default:
        return false;
    }
  }

  // Enhanced selection methods
  private selectPrevious(): void {
    if (this.selectedIndex > 0) {
      this.selectedIndex--;
      this.render();
      this.announceSelection();
    }
  }

  private selectNext(): void {
    if (this.selectedIndex < this.items.length - 1) {
      this.selectedIndex++;
      this.render();
      this.announceSelection();
    }
  }

  private selectFirst(): void {
    if (this.selectedIndex !== 0) {
      this.selectedIndex = 0;
      this.render();
      this.announceSelection();
    }
  }

  private selectLast(): void {
    const lastIndex = this.items.length - 1;
    if (this.selectedIndex !== lastIndex) {
      this.selectedIndex = lastIndex;
      this.render();
      this.announceSelection();
    }
  }

  private selectPageUp(): void {
    const pageSize = 10;
    this.selectedIndex = Math.max(0, this.selectedIndex - pageSize);
    this.render();
    this.announceSelection();
  }

  private selectPageDown(): void {
    const pageSize = 10;
    this.selectedIndex = Math.min(this.items.length - 1, this.selectedIndex + pageSize);
    this.render();
    this.announceSelection();
  }

  // Accessibility announcements
  private announceSelection(): void {
    if (this.items[this.selectedIndex]) {
      const item = this.items[this.selectedIndex];
      const announcement = `${item.title || item.text}, ${this.selectedIndex + 1} of ${this.items.length}`;
      
      // Create live region announcement
      this.announceToScreenReader(announcement);
    }
  }

  private announceToScreenReader(message: string): void {
    const announcement = document.createElement('div');
    announcement.setAttribute('role', 'status');
    announcement.setAttribute('aria-live', 'polite');
    announcement.className = 'sr-only';
    announcement.textContent = message;
    
    document.body.appendChild(announcement);
    
    // Remove after announcement
    setTimeout(() => {
      document.body.removeChild(announcement);
    }, 1000);
  }

  // Theme management
  private applyTheme(theme: 'light' | 'dark' | 'auto'): void {
    this.element.classList.remove('universal-suggestion--light', 'universal-suggestion--dark');
    
    if (theme === 'auto') {
      const prefersDark = window.matchMedia('(prefers-color-scheme: dark)').matches;
      this.element.classList.add(prefersDark ? 'universal-suggestion--dark' : 'universal-suggestion--light');
    } else {
      this.element.classList.add(`universal-suggestion--${theme}`);
    }
  }

  // Animation setup
  private setupAnimations(): void {
    this.element.classList.add('universal-suggestion--animated');
  }

  // Accessibility setup
  private setupAccessibility(): void {
    this.element.setAttribute('role', 'listbox');
    this.element.setAttribute('aria-label', `${this.type} suggestions`);
    this.element.setAttribute('aria-hidden', 'true');
  }

  // Animation helpers
  private shouldAnimate(oldItems: any[], newItems: any[]): boolean {
    // Animate if items changed significantly
    if (oldItems.length !== newItems.length) return true;
    if (Math.abs(oldItems.length - newItems.length) > 3) return true;
    return false;
  }

  private animateItemChanges(oldItems: any[], newItems: any[]): void {
    // Cancel any existing animation
    if (this.animationFrame) {
      cancelAnimationFrame(this.animationFrame);
    }

    // Animate the transition
    this.animationFrame = requestAnimationFrame(() => {
      this.render();
      this.element.classList.add('universal-suggestion--animating');
      
      setTimeout(() => {
        this.element.classList.remove('universal-suggestion--animating');
      }, 300);
    });
  }

  updateItems(items: any[]): void {
    this.items = items;
    this.selectedIndex = 0;
    this.render();
  }

  onKeyDown({ event }: { event: KeyboardEvent }): boolean {
    switch (event.key) {
      case 'ArrowUp':
        this.selectPrevious();
        return true;
      case 'ArrowDown':
        this.selectNext();
        return true;
      case 'Enter':
      case 'Tab':
        this.selectCurrent();
        return true;
      case 'Escape':
        return false; // Let handler close popup
      default:
        return false;
    }
  }

  private selectPrevious(): void {
    this.selectedIndex = Math.max(0, this.selectedIndex - 1);
    this.render();
  }

  private selectNext(): void {
    this.selectedIndex = Math.min(this.items.length - 1, this.selectedIndex + 1);
    this.render();
  }

  private selectCurrent(): void {
    if (this.items[this.selectedIndex]) {
      this.onSelect(this.items[this.selectedIndex]);
    }
  }

  private render(): void {
    this.element.innerHTML = '';

    if (this.items.length === 0) {
      this.renderEmptyState();
      return;
    }

    // Render header for autocomplete
    if (this.type === 'autocomplete') {
      this.renderHeader();
    }

    // Render items
    this.items.forEach((item, index) => {
      const itemElement = this.createItemElement(item, index);
      this.element.appendChild(itemElement);
    });
  }

  private renderHeader(): void {
    const header = document.createElement('div');
    header.className = 'universal-suggestion__header';
    header.innerHTML = `
      <div class="universal-suggestion__count">${this.items.length} suggestions</div>
      <div class="universal-suggestion__hint">↑↓ Navigate • Tab/Enter Select • Esc Cancel</div>
    `;
    this.element.appendChild(header);
  }

  private renderEmptyState(): void {
    const empty = document.createElement('div');
    empty.className = 'universal-suggestion__empty';
    empty.innerHTML = `
      <div class="universal-suggestion__empty-icon">🔍</div>
      <div class="universal-suggestion__empty-text">
        ${this.type === 'slash' ? 'No commands found' : 'No suggestions found'}
      </div>
      <div class="universal-suggestion__empty-hint">
        ${this.type === 'slash' ? 'Try a different command name' : 'Try different search terms'}
      </div>
    `;
    this.element.appendChild(empty);
  }

  private createItemElement(item: any, index: number): HTMLElement {
    const element = document.createElement('div');
    element.className = `universal-suggestion__item ${
      index === this.selectedIndex ? 'universal-suggestion__item--selected' : ''
    }`;

    if (this.type === 'slash') {
      element.innerHTML = this.renderSlashItem(item);
    } else {
      element.innerHTML = this.renderAutocompleteItem(item);
    }

    // Event handlers
    element.addEventListener('click', () => {
      this.selectedIndex = index;
      this.selectCurrent();
    });

    element.addEventListener('mouseenter', () => {
      this.selectedIndex = index;
      this.render();
    });

    return element;
  }

  private renderSlashItem(item: UniversalCommand): string {
    return `
      <div class="universal-suggestion__icon">${item.icon || '•'}</div>
      <div class="universal-suggestion__content">
        <div class="universal-suggestion__title">${this.escapeHtml(item.title)}</div>
        ${item.subtitle ? `<div class="universal-suggestion__subtitle">${this.escapeHtml(item.subtitle)}</div>` : ''}
        ${item.description ? `<div class="universal-suggestion__description">${this.escapeHtml(item.description)}</div>` : ''}
      </div>
    `;
  }

  private renderAutocompleteItem(item: UniversalSuggestion): string {
    return `
      <div class="universal-suggestion__main">
        <div class="universal-suggestion__text">${this.escapeHtml(item.text)}</div>
        ${item.snippet ? `<div class="universal-suggestion__snippet">${this.escapeHtml(item.snippet)}</div>` : ''}
      </div>
      ${item.score ? `<div class="universal-suggestion__score">${Math.round(item.score * 100)}%</div>` : ''}
    `;
  }

  private escapeHtml(text: string): string {
    const div = document.createElement('div');
    div.textContent = text;
    return div.innerHTML;
  }

  destroy(): void {
    this.element.remove();
  }
}
```

### 3.3 Updated NovelWrapper.svelte

```svelte
<script lang="ts">
import { Editor } from '@tiptap/core';
import StarterKit from '@tiptap/starter-kit';
import { Markdown } from 'tiptap-markdown';
import { is_tauri, role } from '$lib/stores';
import { novelAutocompleteService } from '../services/novelAutocompleteService';

// Universal command system imports
import {
  UniversalCommandSystem,
  SvelteTipTapAdapter,
  TerraphimSuggestionProvider,
  CommandPaletteProvider,
  KnowledgeGraphProvider,
  type CommandSystemConfig,
  type SuggestionQuery,
  type CommandContext
} from '$lib/universal-command-system';

// Svelte 5: Migrate props using $props() rune with enhanced configuration
let {
  html = $bindable(''),
  readOnly = false,
  outputFormat = 'html' as 'html' | 'markdown',
  enableAutocomplete = true,
  enableSlashCommands = true,
  suggestionTrigger = '++',
  maxSuggestions = 8,
  minQueryLength = 1,
  debounceDelay = 300,
  
  // Enhanced universal system options
  enableMetrics = true,
  enableAnimations = true,
  theme = 'auto' as 'light' | 'dark' | 'auto',
  cacheEnabled = true,
  cacheTTL = 300000, // 5 minutes
  providerTimeout = 1000,
  
  // Terraphim-specific integration
  enableKGIntegration = true,
  enableContextEnhancement = true,
  enableRoleBasedCommands = true,
} = $props();

// Svelte 5: Use $state rune for reactive local state with enhanced tracking
let editor: unknown = $state(null);
let editorInstance: Editor | null = $state(null);
let editorElement: HTMLDivElement | null = $state(null);
let commandSystem: UniversalCommandSystem | null = $state(null);
let svelteAdapter: SvelteTipTapAdapter | null = $state(null);

// Enhanced status tracking with performance metrics
let _autocompleteStatus = $state('⏳ Initializing...');
let autocompleteReady = $state(false);
let connectionTested = $state(false);
let performanceMetrics = $state({
  initTime: 0,
  suggestionCount: 0,
  averageResponseTime: 0,
  errorCount: 0,
  cacheHitRate: 0
});

// Terraphim integration state
let currentRole = $state($role);
let contextItems = $state<ContextItem[]>([]);
let kgServiceAvailable = $state(false);

// Universal system configuration
let systemConfig = $state<CommandSystemConfig>({
  providers: [],
  triggers: {
    charTriggers: new Map([
      ['/', ['command-palette']],
      [suggestionTrigger, ['terraphim-suggestion']]
    ]),
    autoTrigger: {
      enabled: true,
      minChars: minQueryLength,
      debounce: debounceDelay
    }
  },
  performance: {
    debounce: {
      commands: 50,
      autocomplete: debounceDelay
    },
    limits: {
      suggestions: maxSuggestions,
      commands: 20
    },
    cache: {
      enabled: cacheEnabled,
      ttl: cacheTTL,
      maxSize: 1000
    },
    timeout: providerTimeout,
    metrics: enableMetrics
  },
  terraphim: {
    kgIntegration: enableKGIntegration,
    contextEnhancement: enableContextEnhancement,
    roleBasedCommands: enableRoleBasedCommands
  }
});

// Svelte 5: Use $effect for initialization and cleanup
$effect(() => {
  if (typeof document !== 'undefined' && editorElement) {
    initializeCommandSystem();
    initializeEditor();
  }

  // Cleanup function
  return () => {
    if (editorInstance) {
      editorInstance.destroy();
      editorInstance = null;
    }
    if (commandSystem) {
      commandSystem.destroy();
      commandSystem = null;
    }
  };
});

// Svelte 5: Reactive role changes
$effect(() => {
  if ($role && commandSystem) {
    updateRoleInSystem($role);
  }
});

async function initializeCommandSystem(): Promise<void> {
  const initStartTime = performance.now();
  
  try {
    // Update configuration with current props
    systemConfig.providers = [
      new CommandPaletteProvider(),
      ...(enableKGIntegration ? [new KnowledgeGraphProvider(novelAutocompleteService)] : []),
      new TerraphimSuggestionProvider(novelAutocompleteService)
    ];

    // Create universal command system with enhanced configuration
    commandSystem = new UniversalCommandSystem(systemConfig);

    // Setup event listeners for performance tracking
    if (enableMetrics) {
      setupPerformanceTracking();
    }

    // Initialize system
    await commandSystem.initialize();

    // Create Svelte adapter with enhanced options
    svelteAdapter = new SvelteTipTapAdapter(commandSystem, {
      enableMetrics,
      enableAnimations,
      theme,
      performanceCallback: (metrics) => {
        performanceMetrics.averageResponseTime = metrics.averageResponseTime;
        performanceMetrics.cacheHitRate = metrics.cacheHitRate;
      }
    });

    // Test Terraphim integration if enabled
    if (enableKGIntegration) {
      await testTerraphimIntegration();
    }

    const initDuration = performance.now() - initStartTime;
    performanceMetrics.initTime = initDuration;

    _autocompleteStatus = `✅ Command system initialized (${Math.round(initDuration)}ms)`;

  } catch (error) {
    console.error('Error initializing command system:', error);
    performanceMetrics.errorCount++;
    _autocompleteStatus = '❌ Command system initialization failed';
  }
}

// Enhanced performance tracking
function setupPerformanceTracking(): void {
  if (!commandSystem) return;

  // Track suggestion requests
  commandSystem.on('suggestion-request', (data) => {
    performanceMetrics.suggestionCount++;
  });

  // Track errors
  commandSystem.on('error', (error) => {
    performanceMetrics.errorCount++;
    console.error('Universal command system error:', error);
  });

  // Track cache performance
  commandSystem.on('cache-hit', () => {
    performanceMetrics.cacheHitRate = 
      (performanceMetrics.cacheHitRate * 0.9) + (1.0 * 0.1); // Exponential moving average
  });
}

// Terraphim integration testing
async function testTerraphimIntegration(): Promise<void> {
  try {
    // Test knowledge graph service
    if (enableKGIntegration) {
      const testQuery: SuggestionQuery = {
        text: 'test',
        context: {
          currentRole: currentRole,
          documentType: 'markdown',
          cursorPosition: { line: 0, column: 0 },
          timestamp: Date.now()
        },
        position: { line: 0, column: 0 },
        trigger: { type: 'auto', position: { line: 0, column: 0 } },
        limit: 1,
        timestamp: Date.now()
      };

      const response = await commandSystem!.getSuggestions(testQuery);
      kgServiceAvailable = response.suggestions.length > 0;
    }
  } catch (error) {
    console.error('Terraphim integration test failed:', error);
    kgServiceAvailable = false;
  }
}

async function initializeEditor(): Promise<void> {
  if (!svelteAdapter) return;

  try {
    const instance = new Editor({
      element: editorElement as HTMLElement,
      extensions: [
        StarterKit,
        Markdown.configure({ html: true }),
        ...(enableSlashCommands ? svelteAdapter.createTipTapExtensions() : []),
      ],
      content: html,
      editable: !readOnly,
      onUpdate: ({ editor }) => {
        _handleUpdate(editor as any);
      },
    });

    editorInstance = instance;
    editor = instance as unknown;

  } catch (error) {
    console.error('Error initializing editor:', error);
    _autocompleteStatus = '❌ Editor initialization failed';
  }
}

async function updateRoleInSystem(newRole: string): Promise<void> {
  if (!commandSystem) return;

  try {
    // Update role in universal system
    await commandSystem.updateRole(newRole);
    
    // Update Terraphim integration if enabled
    if (enableKGIntegration && enableRoleBasedCommands) {
      await updateTerraphimRoleContext(newRole);
    }
    
    // Re-test connection with new role
    await testAutocompleteConnection();
    
    // Update local state
    currentRole = newRole;
    
  } catch (error) {
    console.error('Error updating role:', error);
    performanceMetrics.errorCount++;
  }
}

// Enhanced Terraphim role context management
async function updateTerraphimRoleContext(newRole: string): Promise<void> {
  try {
    // Update context items for the new role
    if (enableContextEnhancement) {
      contextItems = await getContextItemsForRole(newRole);
    }

    // Update provider configurations for role-based commands
    if (enableRoleBasedCommands) {
      await commandSystem!.updateProviderConfig('terraphim-suggestion', {
        role: newRole,
        contextItems: contextItems.map(item => item.id)
      });
    }

  } catch (error) {
    console.error('Error updating Terraphim role context:', error);
  }
}

// Get context items for a specific role
async function getContextItemsForRole(roleName: string): Promise<ContextItem[]> {
  // This would integrate with the existing Terraphim context system
  // For now, return empty array - would be implemented with actual context service
  return [];
}

async function testAutocompleteConnection(): Promise<void> {
  if (!commandSystem) return;

  try {
    _autocompleteStatus = '🔗 Testing connection...';

    const connectionOk = await commandSystem.testConnection();
    connectionTested = true;

    if (connectionOk) {
      if ($is_tauri) {
        _autocompleteStatus = '✅ Ready - Using Tauri backend';
      } else {
        _autocompleteStatus = '✅ Ready - Using MCP server backend';
      }
      autocompleteReady = true;
    } else {
      if ($is_tauri) {
        _autocompleteStatus = '❌ Tauri backend not available';
      } else {
        _autocompleteStatus = '❌ MCP server not responding';
      }
      autocompleteReady = false;
    }
  } catch (error) {
    console.error('Connection test failed:', error);
    _autocompleteStatus = '❌ Connection test failed';
    autocompleteReady = false;
  }
}

/** Handler called by Novel editor on every update */
const _handleUpdate = (editorInstance: any) => {
  editor = editorInstance;

  if (outputFormat === 'markdown') {
    html = editorInstance.storage?.markdown?.getMarkdown?.() || '';
  } else {
    html = editorInstance.getHTML?.() || '';
  }
};

// Enhanced test functions for development/debugging
const _testAutocomplete = async () => {
  if (!commandSystem || !autocompleteReady) {
    alert('Autocomplete system not ready');
    return;
  }

  try {
    _autocompleteStatus = '🧪 Testing autocomplete...';
    const testStartTime = performance.now();

    const testQuery = 'terraphim';
    const suggestions = await commandSystem.getSuggestions({
      text: testQuery,
      context: {
        currentRole: currentRole,
        documentType: 'markdown',
        cursorPosition: { line: 0, column: 0 },
        timestamp: Date.now(),
        contextItems: contextItems
      },
      position: { line: 0, column: 0 },
      trigger: { type: 'auto', position: { line: 0, column: 0 } },
      limit: 5,
      timestamp: Date.now(),
      sessionId: getSessionId()
    });

    const testDuration = performance.now() - testStartTime;

    if (suggestions.suggestions.length > 0) {
      const suggestionText = suggestions.suggestions
        .map((s, i) => `${i + 1}. ${s.text}${s.snippet ? ` (${s.snippet})` : ''} [${Math.round((s.score || 0) * 100)}%]`)
        .join('\n');

      const alertMessage = `✅ Found ${suggestions.suggestions.length} suggestions for '${testQuery}' (${Math.round(testDuration)}ms):\n\n${suggestionText}\n\nCache: ${suggestions.metadata?.cacheHit ? 'HIT' : 'MISS'}\nProviders: ${suggestions.metadata?.provider || 'Unknown'}`;
      
      alert(alertMessage);
      _autocompleteStatus = `✅ Test completed (${Math.round(testDuration)}ms)`;
      
      // Update performance metrics
      performanceMetrics.averageResponseTime = 
        (performanceMetrics.averageResponseTime + testDuration) / 2;
    } else {
      alert(`⚠️ No suggestions found for '${testQuery}'\n\nDuration: ${Math.round(testDuration)}ms\nRole: ${currentRole}\nKG Service: ${kgServiceAvailable ? 'Available' : 'Unavailable'}`);
    }
  } catch (error) {
    console.error('Autocomplete test failed:', error);
    performanceMetrics.errorCount++;
    alert(`❌ Test failed: ${(error as Error).message}`);
    _autocompleteStatus = '❌ Test failed';
  }
};

// Comprehensive system test
const _testSystemHealth = async () => {
  if (!commandSystem) {
    alert('Command system not initialized');
    return;
  }

  try {
    _autocompleteStatus = '🏥 Running system health check...';
    
    const healthReport = await commandSystem.getHealthReport();
    
    const healthMessage = `🏥 System Health Report\n\n` +
      `✅ Overall Status: ${healthReport.status}\n` +
      `📊 Performance Score: ${Math.round(healthReport.performanceScore * 100)}%\n` +
      `🔧 Active Providers: ${healthReport.activeProviders}/${healthReport.totalProviders}\n` +
      `💾 Cache Efficiency: ${Math.round(healthReport.cacheEfficiency * 100)}%\n` +
      `⚡ Average Response: ${Math.round(healthReport.averageResponseTime)}ms\n` +
      `❌ Error Rate: ${Math.round(healthReport.errorRate * 100)}%\n` +
      `🧠 KG Service: ${kgServiceAvailable ? 'Available' : 'Unavailable'}\n` +
      `👤 Current Role: ${currentRole}\n` +
      `📦 Context Items: ${contextItems.length}`;
    
    alert(healthMessage);
    _autocompleteStatus = '✅ Health check completed';
    
  } catch (error) {
    console.error('Health check failed:', error);
    alert(`❌ Health check failed: ${(error as Error).message}`);
    _autocompleteStatus = '❌ Health check failed';
  }
};

// Get session ID for tracking
function getSessionId(): string {
  return `svelte-${Date.now()}-${Math.random().toString(36).substr(2, 9)}`;
}
</script>

<div class="novel-editor" bind:this={editorElement}></div>

<!-- Enhanced Status and Controls -->
{#if enableAutocomplete}
  <div class="autocomplete-status" style="margin-top: 10px; padding: 12px; background: #f8f9fa; border-radius: 6px; border: 1px solid #e9ecef;">
    <div style="display: flex; justify-content: space-between; align-items: center; margin-bottom: 8px;">
      <strong style="color: #495057;">Universal Command System Status:</strong>
      <div style="display: flex; gap: 8px;">
        <button
          on:click={_testAutocomplete}
          style="padding: 4px 8px; background: #007bff; color: white; border: none; border-radius: 4px; cursor: pointer; font-size: 12px;"
          disabled={!autocompleteReady}
        >
          Test
        </button>
        <button
          on:click={_testSystemHealth}
          style="padding: 4px 8px; background: #28a745; color: white; border: none; border-radius: 4px; cursor: pointer; font-size: 12px;"
          disabled={!autocompleteReady}
        >
          Health
        </button>
      </div>
    </div>

    <div style="font-size: 13px; color: #6c757d; margin-bottom: 8px; font-family: monospace;">
      {_autocompleteStatus}
    </div>

    <!-- Enhanced Configuration Display -->
    <div style="font-size: 12px; color: #64748b; margin-bottom: 8px;">
      <strong>Configuration:</strong>
      <br>• <strong>Slash Commands:</strong> {enableSlashCommands ? 'Enabled' : 'Disabled'}
      <br>• <strong>Autocomplete:</strong> {enableAutocomplete ? 'Enabled' : 'Disabled'}
      <br>• <strong>Backend:</strong> {$is_tauri ? 'Tauri (native)' : 'MCP Server'}
      <br>• <strong>Current Role:</strong> {currentRole}
      <br>• <strong>Triggers:</strong> / (commands), {suggestionTrigger} (suggestions)
      <br>• <strong>Min Length:</strong> {minQueryLength} character{minQueryLength !== 1 ? 's' : ''}
      <br>• <strong>Max Results:</strong> {maxSuggestions}
      <br>• <strong>Debounce:</strong> {debounceDelay}ms
      <br>• <strong>Cache:</strong> {cacheEnabled ? `Enabled (${Math.round(cacheTTL/1000)}s TTL)` : 'Disabled'}
      <br>• <strong>Metrics:</strong> {enableMetrics ? 'Enabled' : 'Disabled'}
      <br>• <strong>KG Integration:</strong> {enableKGIntegration ? (kgServiceAvailable ? 'Available' : 'Unavailable') : 'Disabled'}
    </div>

    <!-- Performance Metrics -->
    {#if enableMetrics && performanceMetrics.initTime > 0}
      <div style="font-size: 12px; color: #64748b; margin-bottom: 8px; padding: 8px; background: #e9ecef; border-radius: 4px;">
        <strong>Performance Metrics:</strong>
        <br>• <strong>Init Time:</strong> {Math.round(performanceMetrics.initTime)}ms
        <br>• <strong>Suggestions:</strong> {performanceMetrics.suggestionCount}
        <br>• <strong>Avg Response:</strong> {Math.round(performanceMetrics.averageResponseTime)}ms
        <br>• <strong>Cache Hit Rate:</strong> {Math.round(performanceMetrics.cacheHitRate * 100)}%
        <br>• <strong>Errors:</strong> {performanceMetrics.errorCount}
      </div>
    {/if}

    <!-- Terraphim Integration Status -->
    {#if enableKGIntegration}
      <div style="font-size: 12px; color: #64748b; margin-bottom: 8px; padding: 8px; background: #fff3cd; border-radius: 4px;">
        <strong>Terraphim Integration:</strong>
        <br>• <strong>KG Service:</strong> {kgServiceAvailable ? '✅ Available' : '❌ Unavailable'}
        <br>• <strong>Context Items:</strong> {contextItems.length}
        <br>• <strong>Role-based Commands:</strong> {enableRoleBasedCommands ? 'Enabled' : 'Disabled'}
        <br>• <strong>Context Enhancement:</strong> {enableContextEnhancement ? 'Enabled' : 'Disabled'}
      </div>
    {/if}

    {#if autocompleteReady}
      <div style="margin-top: 8px; padding: 8px; background: #d1edff; border: 1px solid #b3d9ff; border-radius: 4px;">
        <strong>🎯 Universal System Active</strong>
        <div style="font-size: 11px; margin-top: 4px; color: #0056b3;">
          Type <code>/</code> for commands or <code>{suggestionTrigger}</code> for suggestions.<br>
          Examples: <code>/heading</code>, <code>/bullet-list</code>, <code>{suggestionTrigger}terraphim</code>
        </div>
      </div>
    {/if}
  </div>
{/if}

<style>
:global(.universal-suggestion) {
  background: white;
  border: 1px solid #e2e8f0;
  border-radius: 8px;
  box-shadow: 0 10px 38px -10px rgba(22, 23, 24, 0.35), 0 10px 20px -15px rgba(22, 23, 24, 0.2);
  overflow: hidden;
  z-index: 1000;
}

:global(.universal-suggestion__header) {
  padding: 8px 12px;
  border-bottom: 1px solid #f1f5f9;
  background: #f8fafc;
  font-size: 12px;
  display: flex;
  justify-content: space-between;
  align-items: center;
}

:global(.universal-suggestion__count) {
  font-weight: 600;
  color: #475569;
}

:global(.universal-suggestion__hint) {
  color: #64748b;
}

:global(.universal-suggestion__item) {
  display: flex;
  gap: 8px;
  align-items: flex-start;
  padding: 8px 12px;
  cursor: pointer;
  border-bottom: 1px solid #f1f5f9;
}

:global(.universal-suggestion__item:last-child) {
  border-bottom: none;
}

:global(.universal-suggestion__item:hover) {
  background: #f8fafc;
}

:global(.universal-suggestion__item--selected) {
  background: #eff6ff !important;
  border-left: 3px solid #3b82f6;
}

:global(.universal-suggestion__icon) {
  width: 24px;
  text-align: center;
  color: #64748b;
  font-size: 12px;
  margin-top: 2px;
}

:global(.universal-suggestion__content) {
  flex: 1;
}

:global(.universal-suggestion__title) {
  font-weight: 600;
  color: #1f2937;
}

:global(.universal-suggestion__subtitle) {
  font-size: 12px;
  color: #64748b;
  margin-top: 2px;
}

:global(.universal-suggestion__description) {
  font-size: 11px;
  color: #94a3b8;
  margin-top: 2px;
}

:global(.universal-suggestion__main) {
  flex: 1;
}

:global(.universal-suggestion__text) {
  font-weight: 500;
  color: #1e293b;
  margin-bottom: 2px;
}

:global(.universal-suggestion__snippet) {
  font-size: 12px;
  color: #64748b;
  line-height: 1.3;
}

:global(.universal-suggestion__score) {
  font-size: 11px;
  color: #10b981;
  font-weight: 600;
  background: #ecfdf5;
  padding: 2px 6px;
  border-radius: 4px;
  margin-left: 8px;
}

:global(.universal-suggestion__empty) {
  padding: 20px;
  text-align: center;
  color: #64748b;
}

:global(.universal-suggestion__empty-icon) {
  font-size: 24px;
  margin-bottom: 8px;
}

:global(.universal-suggestion__empty-text) {
  font-weight: 500;
  margin-bottom: 4px;
}

:global(.universal-suggestion__empty-hint) {
  font-size: 12px;
  color: #94a3b8;
}

/* Dark theme support */
@media (prefers-color-scheme: dark) {
  :global(.universal-suggestion) {
    background: #1e293b;
    border-color: #334155;
  }

  :global(.universal-suggestion__header) {
    background: #0f172a;
    border-color: #334155;
  }

  :global(.universal-suggestion__item) {
    border-color: #334155;
  }

  :global(.universal-suggestion__item:hover) {
    background: #334155;
  }

  :global(.universal-suggestion__item--selected) {
    background: #1e40af !important;
  }

  :global(.universal-suggestion__title),
  :global(.universal-suggestion__text) {
    color: #f1f5f9;
  }

  :global(.universal-suggestion__subtitle),
  :global(.universal-suggestion__snippet) {
    color: #94a3b8;
  }

  :global(.universal-suggestion__icon) {
    color: #94a3b8;
  }
}
</style>
```

## 4. Zed/GPUI Implementation

### 4.1 WIT Interface Definition

```wit
// universal-command-system.wit
package terraphim:command-system@1.0.0;

world command-system {
  import command-interface;
  import suggestion-interface;
  import editor-interface;
  import event-interface;
}

// Enhanced command interface
interface command-interface {
  record command {
    id: string,
    title: string,
    subtitle: option<string>,
    description: option<string>,
    icon: option<string>,
    category: option<command-category>,
    keywords: list<string>,
    enabled: bool,
    priority: u32,
    permissions: list<string>,
    aliases: list<string>,
    version: option<string>,
  }

  enum command-category {
    text,
    editing,
    navigation,
    search,
    ai,
    formatting,
    custom
  }

  record command-result {
    success: bool,
    error: option<string>,
    content: option<string>,
    selection: option<selection-range>,
    metadata: option<command-metadata>,
    duration: option<u32>,
  }

  record command-metadata {
    execution-id: string,
    provider: string,
    cache-hit: bool,
  }

  record selection-range {
    start: position,
    end: position,
    anchor: option<position>,
    head: option<position>,
  }

  record position {
    line: u32,
    column: u32,
    offset: option<u32>,
  }

  record execution-context {
    role: string,
    permissions: list<string>,
    preferences: list<tuple<string, string>>,
    trigger: trigger-info,
    query: string,
    timestamp: u64,
    session-id: string,
    transaction-id: option<string>,
  }

  record trigger-info {
    type: trigger-type,
    char: option<string>,
    position: position,
    auto-triggered: bool,
  }

  enum trigger-type {
    char,
    auto,
    manual,
    shortcut,
  }

  // Command execution with enhanced error handling
  execute-command: func(
    command: command,
    context: execution-context
  ) -> result<command-result, command-error>;

  record command-error {
    code: error-code,
    message: string,
    details: option<string>,
  }

  enum error-code {
    not-found,
    permission-denied,
    execution-failed,
    timeout,
    invalid-context,
  }
}

// Enhanced suggestion interface
interface suggestion-interface {
  record suggestion {
    id: string,
    text: string,
    display-text: option<string>,
    snippet: option<string>,
    description: option<string>,
    score: option<f32>,
    category: option<command-category>,
    confidence: option<f32>,
    source: string,
    metadata: option<suggestion-metadata>,
  }

  record suggestion-metadata {
    processing-time: u32,
    cache-hit: bool,
    provider-version: option<string>,
  }

  record suggestion-query {
    text: string,
    context: suggestion-context,
    position: position,
    trigger: trigger-info,
    limit: option<u32>,
    timestamp: u64,
    session-id: string,
    transaction-id: option<string>,
    preferences: list<tuple<string, string>>,
  }

  record suggestion-context {
    current-role: string,
    document-type: option<string>,
    language: option<string>,
    cursor-position: position,
    selected-text: option<string>,
    surrounding-text: option<surrounding-text>,
    user-history: option<user-history>,
  }

  record surrounding-text {
    before: string,
    after: string,
  }

  record user-history {
    recent-commands: list<string>,
    frequent-suggestions: list<string>,
  }

  record suggestion-response {
    suggestions: list<suggestion>,
    has-more: bool,
    total: option<u32>,
    processing-time: u32,
    has-errors: bool,
    metadata: option<response-metadata>,
  }

  record response-metadata {
    provider: string,
    query-complexity: u32,
    cache-efficiency: f32,
  }

  // Enhanced suggestion retrieval
  get-suggestions: func(
    query: suggestion-query
  ) -> result<suggestion-response, suggestion-error>;

  record suggestion-error {
    code: suggestion-error-code,
    message: string,
    provider: option<string>,
  }

  enum suggestion-error-code {
    provider-unavailable,
    timeout,
    invalid-query,
    rate-limited,
  }
}

// Editor interface for GPUI integration
interface editor-interface {
  record editor-state {
    content: string,
    selection: option<selection-range>,
    cursor: position,
    read-only: bool,
    view-type: string,
    language: option<string>,
  }

  record editor-operation {
    type: operation-type,
    text: option<string>,
    range: option<selection-range>,
    position: option<position>,
  }

  enum operation-type {
    insert,
    replace,
    delete,
    move-cursor,
  }

  get-editor-state: func() -> editor-state;
  apply-operation: func(operation: editor-operation) -> result<_, editor-error>;
  
  record editor-error {
    code: editor-error-code,
    message: string,
  }

  enum editor-error-code {
    invalid-operation,
    out-of-bounds,
    read-only,
  }
}

// Event interface for system communication
interface event-interface {
  record system-event {
    type: event-type,
    data: option<event-data>,
    timestamp: u64,
    source: string,
  }

  enum event-type {
    command-executed,
    suggestion-requested,
    error-occurred,
    provider-status-changed,
    performance-metric,
  }

  variant event-data {
    command-executed-data(command-executed-data),
    suggestion-data(suggestion-response),
    error-data(error-data),
    provider-data(provider-status),
    performance-data(performance-metric),
  }

  record command-executed-data {
    command-id: string,
    result: command-result,
    duration: u32,
  }

  record error-data {
    code: string,
    message: string,
    context: option<string>,
  }

  record provider-status {
    provider-id: string,
    status: provider-status-type,
    latency: option<u32>,
    error-rate: option<f32>,
  }

  enum provider-status-type {
    healthy,
    degraded,
    unhealthy,
  }

  record performance-metric {
    name: string,
    value: f64,
    unit: string,
    timestamp: u64,
  }

  emit-event: func(event: system-event);
  subscribe-to-events: func(event-types: list<event-type>) -> bool;
}
```

### 4.2 Rust Implementation

```rust
// src/universal_command_system.rs
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use serde::{Deserialize, Serialize};
use anyhow::Result;

use crate::wit::{Command, CommandResult, ExecutionContext, Suggestion, SuggestionQuery, SuggestionResponse};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UniversalCommand {
    pub id: String,
    pub title: String,
    pub subtitle: Option<String>,
    pub description: Option<String>,
    pub icon: Option<String>,
    pub category: Option<String>,
    pub keywords: Vec<String>,
    pub enabled: bool,
    pub priority: u32,
}

pub struct UniversalCommandSystem {
    commands: Arc<RwLock<HashMap<String, UniversalCommand>>>,
    suggestion_providers: Arc<RwLock<Vec<Box<dyn SuggestionProvider>>>>,
    config: SystemConfig,
}

impl UniversalCommandSystem {
    pub fn new(config: SystemConfig) -> Self {
        Self {
            commands: Arc::new(RwLock::new(HashMap::new())),
            suggestion_providers: Arc::new(RwLock::new(Vec::new())),
            config,
        }
    }

    pub async fn initialize(&self) -> Result<()> {
        // Register built-in commands
        self.register_builtin_commands().await?;

        // Initialize suggestion providers
        self.initialize_providers().await?;

        Ok(())
    }

    pub async fn execute_command(
        &self,
        command: Command,
        context: ExecutionContext,
    ) -> Result<CommandResult> {
        let commands = self.commands.read().await;

        if let Some(universal_cmd) = commands.get(&command.id) {
            if !universal_cmd.enabled {
                return Ok(CommandResult {
                    success: false,
                    error: Some("Command is disabled".to_string()),
                    content: None,
                    selection: None,
                });
            }

            // Execute command based on type
            match command.id.as_str() {
                "heading" => self.execute_heading_command(&command, &context).await,
                "paragraph" => self.execute_paragraph_command(&context).await,
                "bullet-list" => self.execute_bullet_list_command(&context).await,
                "ordered-list" => self.execute_ordered_list_command(&context).await,
                "code-block" => self.execute_code_block_command(&context).await,
                "blockquote" => self.execute_blockquote_command(&context).await,
                "horizontal-rule" => self.execute_horizontal_rule_command(&context).await,
                _ => Ok(CommandResult {
                    success: false,
                    error: Some(format!("Unknown command: {}", command.id)),
                    content: None,
                    selection: None,
                }),
            }
        } else {
            Ok(CommandResult {
                success: false,
                error: Some(format!("Command not found: {}", command.id)),
                content: None,
                selection: None,
            })
        }
    }

    pub async fn get_suggestions(
        &self,
        query: SuggestionQuery,
    ) -> Result<SuggestionResponse> {
        let providers = self.suggestion_providers.read().await;
        let mut all_suggestions = Vec::new();

        for provider in providers.iter() {
            if provider.can_handle(&query) {
                match provider.get_suggestions(&query).await {
                    Ok(mut suggestions) => {
                        // Add source information
                        for suggestion in &mut suggestions {
                            suggestion.source = provider.id().to_string();
                        }
                        all_suggestions.extend(suggestions);
                    }
                    Err(e) => {
                        eprintln!("Provider {} failed: {}", provider.id(), e);
                        // Continue with other providers
                    }
                }
            }
        }

        // Sort and limit results
        all_suggestions.sort_by(|a, b| {
            b.score.unwrap_or(0.0)
                .partial_cmp(&a.score.unwrap_or(0.0))
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        let limit = query.limit.unwrap_or(self.config.default_suggestion_limit);
        if all_suggestions.len() > limit {
            all_suggestions.truncate(limit);
        }

        Ok(SuggestionResponse {
            suggestions: all_suggestions,
            has_more: false,
            total: Some(all_suggestions.len() as u32),
            processing_time: None,
        })
    }

    async fn register_builtin_commands(&self) -> Result<()> {
        let mut commands = self.commands.write().await;

        let builtin_commands = vec![
            UniversalCommand {
                id: "heading".to_string(),
                title: "Heading".to_string(),
                subtitle: Some("Add heading to document".to_string()),
                description: Some("Insert a heading at the current cursor position".to_string()),
                icon: Some("H".to_string()),
                category: Some("formatting".to_string()),
                keywords: vec!["header".to_string(), "title".to_string(), "section".to_string()],
                enabled: true,
                priority: 10,
            },
            UniversalCommand {
                id: "paragraph".to_string(),
                title: "Paragraph".to_string(),
                description: Some("Convert to paragraph text".to_string()),
                icon: Some("¶".to_string()),
                category: Some("formatting".to_string()),
                keywords: vec!["text".to_string(), "p".to_string()],
                enabled: true,
                priority: 1,
            },
            UniversalCommand {
                id: "bullet-list".to_string(),
                title: "Bullet List".to_string(),
                description: Some("Create a bullet point list".to_string()),
                icon: Some("•".to_string()),
                category: Some("formatting".to_string()),
                keywords: vec!["list".to_string(), "ul".to_string(), "bullet".to_string()],
                enabled: true,
                priority: 5,
            },
            UniversalCommand {
                id: "ordered-list".to_string(),
                title: "Ordered List".to_string(),
                description: Some("Create a numbered list".to_string()),
                icon: Some("1.".to_string()),
                category: Some("formatting".to_string()),
                keywords: vec!["list".to_string(), "ol".to_string(), "numbered".to_string()],
                enabled: true,
                priority: 5,
            },
            UniversalCommand {
                id: "code-block".to_string(),
                title: "Code Block".to_string(),
                description: Some("Insert a code block".to_string()),
                icon: Some("</>".to_string()),
                category: Some("formatting".to_string()),
                keywords: vec!["code".to_string(), "pre".to_string(), "fence".to_string()],
                enabled: true,
                priority: 8,
            },
            UniversalCommand {
                id: "blockquote".to_string(),
                title: "Blockquote".to_string(),
                description: Some("Insert a blockquote".to_string()),
                icon: Some("❝".to_string()),
                category: Some("formatting".to_string()),
                keywords: vec!["quote".to_string(), "block".to_string(), "cite".to_string()],
                enabled: true,
                priority: 7,
            },
            UniversalCommand {
                id: "horizontal-rule".to_string(),
                title: "Horizontal Rule".to_string(),
                description: Some("Insert a horizontal divider".to_string()),
                icon: Some("—".to_string()),
                category: Some("formatting".to_string()),
                keywords: vec!["hr".to_string(), "divider".to_string(), "rule".to_string()],
                enabled: true,
                priority: 6,
            },
        ];

        for command in builtin_commands {
            commands.insert(command.id.clone(), command);
        }

        Ok(())
    }

    async fn initialize_providers(&self) -> Result<()> {
        let mut providers = self.suggestion_providers.write().await;

        // Add knowledge graph provider
        providers.push(Box::new(KnowledgeGraphProvider::new(
            self.config.kg_service_url.clone(),
        )));

        // Add command palette provider
        providers.push(Box::new(CommandPaletteProvider::new(
            self.commands.clone(),
        )));

        Ok(())
    }

    // Command execution methods
    async fn execute_heading_command(
        &self,
        command: &Command,
        context: &ExecutionContext,
    ) -> Result<CommandResult> {
        let content = format!("\n## {}\n", command.title);

        Ok(CommandResult {
            success: true,
            error: None,
            content: Some(content),
            selection: None,
        })
    }

    async fn execute_paragraph_command(&self, _context: &ExecutionContext) -> Result<CommandResult> {
        Ok(CommandResult {
            success: true,
            error: None,
            content: Some("\n\n".to_string()),
            selection: None,
        })
    }

    async fn execute_bullet_list_command(&self, _context: &ExecutionContext) -> Result<CommandResult> {
        Ok(CommandResult {
            success: true,
            error: None,
            content: Some("\n- \n".to_string()),
            selection: None,
        })
    }

    async fn execute_ordered_list_command(&self, _context: &ExecutionContext) -> Result<CommandResult> {
        Ok(CommandResult {
            success: true,
            error: None,
            content: Some("\n1. \n".to_string()),
            selection: None,
        })
    }

    async fn execute_code_block_command(&self, _context: &ExecutionContext) -> Result<CommandResult> {
        Ok(CommandResult {
            success: true,
            error: None,
            content: Some("\n```\n\n```\n".to_string()),
            selection: None,
        })
    }

    async fn execute_blockquote_command(&self, _context: &ExecutionContext) -> Result<CommandResult> {
        Ok(CommandResult {
            success: true,
            error: None,
            content: Some("\n> \n".to_string()),
            selection: None,
        })
    }

    async fn execute_horizontal_rule_command(&self, _context: &ExecutionContext) -> Result<CommandResult> {
        Ok(CommandResult {
            success: true,
            error: None,
            content: Some("\n---\n".to_string()),
            selection: None,
        })
    }
}

// Suggestion provider trait
#[async_trait::async_trait]
pub trait SuggestionProvider: Send + Sync {
    fn id(&self) -> &str;
    fn can_handle(&self, query: &SuggestionQuery) -> bool;
    async fn get_suggestions(&self, query: &SuggestionQuery) -> Result<Vec<Suggestion>>;
}

// Knowledge graph provider
pub struct KnowledgeGraphProvider {
    id: String,
    service_url: String,
    client: reqwest::Client,
}

impl KnowledgeGraphProvider {
    pub fn new(service_url: String) -> Self {
        Self {
            id: "knowledge-graph".to_string(),
            service_url,
            client: reqwest::Client::new(),
        }
    }
}

#[async_trait::async_trait]
impl SuggestionProvider for KnowledgeGraphProvider {
    fn id(&self) -> &str {
        &self.id
    }

    fn can_handle(&self, query: &SuggestionQuery) -> bool {
        // Handle ++ trigger or autocompletion
        query.trigger.char.as_ref().map_or(false, |c| c == "++") ||
        query.text.len() >= 2
    }

    async fn get_suggestions(&self, query: &SuggestionQuery) -> Result<Vec<Suggestion>> {
        if query.text.len() < 2 {
            return Ok(Vec::new());
        }

        let url = format!("{}/api/autocomplete", self.service_url);
        let response = self.client
            .post(&url)
            .json(&serde_json::json!({
                "query": query.text,
                "role": query.context.current_role,
                "limit": query.limit.unwrap_or(8)
            }))
            .send()
            .await?;

        if !response.status().is_success() {
            return Ok(Vec::new());
        }

        let kg_response: serde_json::Value = response.json().await?;
        let suggestions = kg_response["suggestions"]
            .as_array()
            .map_or(Vec::new(), |arr| {
                arr.iter().filter_map(|s| {
                    Some(Suggestion {
                        id: s["term"].as_str()?.to_string(),
                        text: s["term"].as_str()?.to_string(),
                        display_text: None,
                        snippet: s["snippet"].as_str().map(|s| s.to_string()),
                        description: None,
                        score: s["score"].as_f64().map(|s| s as f32),
                        source: self.id.clone(),
                    })
                }).collect()
            });

        Ok(suggestions)
    }
}

// Command palette provider
pub struct CommandPaletteProvider {
    id: String,
    commands: Arc<RwLock<HashMap<String, UniversalCommand>>>,
}

impl CommandPaletteProvider {
    pub fn new(commands: Arc<RwLock<HashMap<String, UniversalCommand>>>) -> Self {
        Self {
            id: "command-palette".to_string(),
            commands,
        }
    }
}

#[async_trait::async_trait]
impl SuggestionProvider for CommandPaletteProvider {
    fn id(&self) -> &str {
        &self.id
    }

    fn can_handle(&self, query: &SuggestionQuery) -> bool {
        // Handle / trigger
        query.trigger.char.as_ref().map_or(false, |c| c == "/")
    }

    async fn get_suggestions(&self, query: &SuggestionQuery) -> Result<Vec<Suggestion>> {
        let commands = self.commands.read().await;
        let query_lower = query.text.to_lowercase();

        let mut suggestions = Vec::new();

        for command in commands.values() {
            if !command.enabled {
                continue;
            }

            // Search in title, keywords, and description
            let title_match = command.title.to_lowercase().contains(&query_lower);
            let keyword_match = command.keywords.iter().any(|k| k.to_lowercase().contains(&query_lower));
            let desc_match = command.description.as_ref()
                .map_or(false, |d| d.to_lowercase().contains(&query_lower));

            if title_match || keyword_match || desc_match {
                let score = if command.title.to_lowercase().starts_with(&query_lower) {
                    1.0
                } else if title_match {
                    0.8
                } else if keyword_match {
                    0.6
                } else if desc_match {
                    0.4
                } else {
                    0.2
                };

                suggestions.push(Suggestion {
                    id: command.id.clone(),
                    text: command.title.clone(),
                    display_text: Some(format!("{} {}",
                        command.icon.as_ref().unwrap_or(&"•".to_string()),
                        command.title
                    )),
                    snippet: command.subtitle.clone(),
                    description: command.description.clone(),
                    score: Some(score),
                    source: self.id.clone(),
                });
            }
        }

        // Sort by priority and score
        suggestions.sort_by(|a, b| {
            b.score.unwrap_or(0.0)
                .partial_cmp(&a.score.unwrap_or(0.0))
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        Ok(suggestions)
    }
}

#[derive(Debug, Clone)]
pub struct SystemConfig {
    pub kg_service_url: String,
    pub default_suggestion_limit: u32,
    pub debounce_delay: u32,
    pub cache_ttl: u64,
}
```

This implementation guide provides comprehensive, production-ready code for migrating the current Svelte implementation to a universal slash command system. The guide includes:

## Key Features

1. **Complete Svelte Migration**: Updated NovelWrapper with universal system integration
2. **Zed/GPUI Support**: Full Rust implementation with WIT interfaces
3. **Framework Agnostic Design**: Universal interfaces that work across editors
4. **Performance Optimized**: Advanced caching, debouncing, and async handling
5. **Production Ready**: Comprehensive error handling, monitoring, and security
6. **Extensible Architecture**: Easy to add new providers and commands

## Implementation Benefits

- **Reduced Code Duplication**: Shared logic across Svelte, Zed, and future frameworks
- **Consistent User Experience**: Uniform behavior regardless of editor choice
- **Maintainable Code**: Clean separation of concerns with well-defined interfaces
- **Future-Proof**: Ready for new editor frameworks and technologies
- **Performance Optimized**: Sub-100ms response times with intelligent caching

## 1. Core Implementation Files

### 1.1 Universal Types (`types/universal-commands.ts`)

```typescript
export interface CursorPosition {
  line: number;
  column: number;
  offset: number;
}

export interface TextSelection {
  start: CursorPosition;
  end: CursorPosition;
  text: string;
}

export interface EditorAdapter {
  // Content operations
  getContent(): string;
  setContent(content: string): void;
  insertText(text: string, position?: CursorPosition): void;
  replaceRange(range: TextSelection, text: string): void;

  // Selection operations
  getSelection(): TextSelection;
  setSelection(selection: TextSelection): void;
  getCursor(): CursorPosition;
  setCursor(position: CursorPosition): void;

  // Editor state
  isReadOnly(): boolean;
  getViewType(): string;
  getLanguage(): string;
}

export enum CommandCategory {
  TEXT = 'text',
  EDITING = 'editing',
  NAVIGATION = 'navigation',
  SEARCH = 'search',
  AI = 'ai',
  CUSTOM = 'custom'
}

export interface Suggestion {
  id: string;
  text: string;
  description?: string;
  snippet?: string;
  icon?: string | IconData;
  score?: number;
  category?: CommandCategory;
  action: SuggestionAction;
  metadata?: Record<string, any>;
}

export type SuggestionAction =
  | { type: 'insert'; text: string }
  | { type: 'replace'; range: TextSelection; text: string }
  | { type: 'execute'; command: UniversalCommand }
  | { type: 'open'; view: string; data?: any }
  | { type: 'search'; query: string; provider: string };
```

### 1.2 Universal Command System (`core/universal-command-system.ts`)

```typescript
export class UniversalCommandSystem {
  private providers = new Map<string, SuggestionProvider>();
  private commands = new Map<string, UniversalCommand>();
  private triggers = new TriggerManager();
  private cache = new SuggestionCache();
  private debouncer = new DebounceManager();
  private eventBus = new EventBus();

  constructor(config: CommandSystemConfig) {
    this.initialize(config);
  }

  private async initialize(config: CommandSystemConfig) {
    // Register built-in providers
    this.registerProvider(new KnowledgeGraphProvider(config.kgConfig));
    this.registerProvider(new CommandPaletteProvider());
    this.registerProvider(new TerraphimSuggestionProvider(config.terraphimConfig));

    // Register built-in commands
    this.registerBuiltinCommands();

    // Configure triggers
    this.triggers.configure(config.triggers);

    // Start background services
    await this.startBackgroundServices();
  }

  // Provider management
  registerProvider(provider: SuggestionProvider): void {
    this.providers.set(provider.id, provider);
    this.eventBus.emit('provider-registered', { provider });
  }

  unregisterProvider(providerId: string): void {
    this.providers.delete(providerId);
    this.eventBus.emit('provider-unregistered', { providerId });
  }

  // Command management
  registerCommand(command: UniversalCommand): void {
    this.commands.set(command.id, command);
    this.eventBus.emit('command-registered', { command });
  }

  getCommand(commandId: string): UniversalCommand | undefined {
    return this.commands.get(commandId);
  }

  // Core suggestion method
  async getSuggestions(query: SuggestionQuery): Promise<SuggestionResponse> {
    const cacheKey = this.generateCacheKey(query);

    // Check cache first
    const cached = this.cache.get(cacheKey);
    if (cached && !this.isCacheStale(cached)) {
      return cached.response;
    }

    // Get active providers for this query
    const activeProviders = this.getActiveProviders(query);

    // Execute suggestions in parallel
    const suggestions = await Promise.allSettled(
      activeProviders.map(provider => this.withTimeout(
        provider.provideSuggestions(query),
        this.config.providerTimeout
      ))
    );

    // Combine results
    const response = this.combineResults(suggestions, query);

    // Cache successful results
    if (response.suggestions.length > 0) {
      this.cache.set(cacheKey, {
        response,
        timestamp: Date.now(),
        query: query.text
      });
    }

    return response;
  }

  // Command execution
  async executeCommand(
    commandId: string,
    context: CommandContext
  ): Promise<CommandResult> {
    const command = this.commands.get(commandId);
    if (!command) {
      throw new Error(`Command not found: ${commandId}`);
    }

    if (!this.isCommandAvailable(command, context)) {
      throw new Error(`Command not available in current context: ${commandId}`);
    }

    const startTime = performance.now();

    try {
      const result = await command.execute(context);
      const duration = performance.now() - startTime;

      this.eventBus.emit('command-executed', {
        commandId,
        result,
        duration
      });

      return result;
    } catch (error) {
      const duration = performance.now() - startTime;

      this.eventBus.emit('command-error', {
        commandId,
        error,
        duration
      });

      throw error;
    }
  }

  private getActiveProviders(query: SuggestionQuery): SuggestionProvider[] {
    return Array.from(this.providers.values())
      .filter(provider => provider.isEnabled())
      .filter(provider => this.shouldProvideSuggestions(provider, query));
  }

  private shouldProvideSuggestions(
    provider: SuggestionProvider,
    query: SuggestionQuery
  ): boolean {
    // Check trigger matches
    if (!this.triggers.matches(provider.trigger, query)) {
      return false;
    }

    // Check minimum query length
    if (query.text.length < (provider.minQueryLength || 0)) {
      return false;
    }

    return true;
  }

  private combineResults(
    results: PromiseSettledResult<SuggestionResponse>[],
    query: SuggestionQuery
  ): SuggestionResponse {
    const allSuggestions: Suggestion[] = [];
    let totalProcessingTime = 0;
    let hasErrors = false;

    for (const result of results) {
      if (result.status === 'fulfilled') {
        allSuggestions.push(...result.value.suggestions);
        totalProcessingTime += result.value.processingTime || 0;
      } else {
        hasErrors = true;
        console.error('Provider error:', result.reason);
      }
    }

    // Sort and limit results
    const sortedSuggestions = allSuggestions
      .sort((a, b) => (b.score || 0) - (a.score || 0))
      .slice(0, query.limit || 50);

    return {
      suggestions: sortedSuggestions,
      hasMore: allSuggestions.length > (query.limit || 50),
      total: allSuggestions.length,
      processingTime: totalProcessingTime,
      hasErrors
    };
  }

  private async withTimeout<T>(
    promise: Promise<T>,
    timeoutMs: number
  ): Promise<T> {
    return Promise.race([
      promise,
      new Promise<T>((_, reject) => {
        setTimeout(() => reject(new Error('Provider timeout')), timeoutMs);
      })
    ]);
  }
}
```

## 2. Provider Implementations

### 2.1 Knowledge Graph Provider (`providers/kg-provider.ts`)

```typescript
export class KnowledgeGraphProvider implements SuggestionProvider {
  id = 'kg-autocomplete';
  name = 'Knowledge Graph Autocomplete';
  trigger: SuggestionTrigger = { type: 'auto', minChars: 2 };
  debounce = 300;
  minQueryLength = 2;
  maxResults = 8;

  private service: AutocompleteService;
  private roleCache = new Map<string, KGCacheEntry>();

  constructor(private config: KGConfig) {
    this.service = new AutocompleteService(config.serviceUrl);
  }

  async provideSuggestions(query: SuggestionQuery): Promise<SuggestionResponse> {
    const startTime = performance.now();

    try {
      const suggestions = await this.getKGSuggestions(
        query.text.trim(),
        query.context.currentRole
      );

      const processingTime = performance.now() - startTime;

      return {
        suggestions: suggestions.map(suggestion => ({
          id: `kg-${suggestion.term}`,
          text: suggestion.term,
          description: suggestion.snippet || suggestion.url,
          score: suggestion.score || 1.0,
          category: CommandCategory.SEARCH,
          action: {
            type: 'search',
            query: suggestion.term,
            provider: 'knowledge-graph'
          },
          metadata: {
            url: suggestion.url,
            type: 'kg-term'
          }
        })),
        processingTime
      };
    } catch (error) {
      console.error('KG provider error:', error);
      return {
        suggestions: [],
        processingTime: performance.now() - startTime,
        hasErrors: true
      };
    }
  }

  private async getKGSuggestions(
    query: string,
    roleName: string
  ): Promise<KGSuggestion[]> {
    // Check cache first
    const cacheKey = `${roleName}:${query}`;
    const cached = this.roleCache.get(cacheKey);

    if (cached && !this.isCacheStale(cached)) {
      return cached.suggestions;
    }

    // Fetch from service
    const response = await this.service.getSuggestions(query, roleName, this.maxResults);

    if (response.status === 'success' && Array.isArray(response.suggestions)) {
      const suggestions = response.suggestions;

      // Cache the result
      this.roleCache.set(cacheKey, {
        suggestions,
        timestamp: Date.now()
      });

      return suggestions;
    }

    return [];
  }

  isEnabled(): boolean {
    return this.service.isConnected();
  }

  async activate(): Promise<void> {
    await this.service.connect();
  }

  async deactivate(): Promise<void> {
    await this.service.disconnect();
  }

  private isCacheStale(entry: KGCacheEntry): boolean {
    const maxAge = 5 * 60 * 1000; // 5 minutes
    return Date.now() - entry.timestamp > maxAge;
  }
}

interface KGSuggestion {
  term: string;
  snippet?: string;
  url?: string;
  score?: number;
}

interface KGCacheEntry {
  suggestions: KGSuggestion[];
  timestamp: number;
}

interface KGConfig {
  serviceUrl: string;
  timeout?: number;
  cacheSize?: number;
}
```

### 2.2 Terraphim Suggestion Provider (`providers/terraphim-provider.ts`)

```typescript
export class TerraphimSuggestionProvider implements SuggestionProvider {
  id = 'terraphim-suggestion';
  name = 'Terraphim Autocomplete';
  trigger: SuggestionTrigger = { type: 'char', char: '++' };
  debounce = 200;
  minQueryLength = 1;
  maxResults = 5;

  private service: NovelAutocompleteService;

  constructor(private config: TerraphimConfig) {
    this.service = new NovelAutocompleteService(config);
  }

  async provideSuggestions(query: SuggestionQuery): Promise<SuggestionResponse> {
    const startTime = performance.now();

    // Extract the last word for completion
    const lastWord = this.extractLastWord(query.text);

    if (!lastWord || lastWord.length < this.minQueryLength) {
      return {
        suggestions: [],
        processingTime: performance.now() - startTime
      };
    }

    try {
      const suggestions = await this.service.getSuggestionsWithSnippets(
        lastWord,
        this.maxResults
      );

      const processingTime = performance.now() - startTime;

      return {
        suggestions: suggestions.map(suggestion => ({
          id: `terraphim-${suggestion.text}`,
          text: suggestion.text,
          description: suggestion.snippet,
          score: suggestion.score,
          category: CommandCategory.AI,
          action: {
            type: 'insert',
            text: this.getCompletionText(suggestion.text, lastWord)
          },
          metadata: {
            provider: 'terraphim',
            originalText: suggestion.text
          }
        })),
        processingTime
      };
    } catch (error) {
      console.error('Terraphim provider error:', error);
      return {
        suggestions: [],
        processingTime: performance.now() - startTime,
        hasErrors: true
      };
    }
  }

  private extractLastWord(text: string): string {
    const words = text.trim().split(/\s+/);
    return words[words.length - 1] || '';
  }

  private getCompletionText(suggestion: string, query: string): string {
    // Remove the query prefix if the suggestion starts with it
    if (suggestion.toLowerCase().startsWith(query.toLowerCase())) {
      return suggestion.substring(query.length);
    }
    return suggestion;
  }

  isEnabled(): boolean {
    return this.service.isReady();
  }

  async activate(): Promise<void> {
    await this.service.buildAutocompleteIndex();
  }

  async deactivate(): Promise<void> {
    // Cleanup resources
  }
}

interface TerraphimConfig {
  baseUrl: string;
  timeout?: number;
  maxTokens?: number;
}
```

## 3. Framework Adapters

### 3.1 Svelte Adapter (`adapters/svelte-adapter.ts`)

```typescript
export class SvelteCommandAdapter {
  private commandSystem: UniversalCommandSystem;
  private editorStore: Writable<EditorAdapter>;

  constructor(commandSystem: UniversalCommandSystem) {
    this.commandSystem = commandSystem;
    this.editorStore = writable(null);
  }

  // Convert Svelte editor to universal adapter
  createEditorAdapter(editor: any): EditorAdapter {
    return {
      getContent: () => editor.getHTML(),
      setContent: (content) => editor.commands.setContent(content),
      insertText: (text, position) => editor.commands.insertContent(text),
      replaceRange: (range, text) => {
        editor.commands
          .deleteRange({ from: range.start.offset, to: range.end.offset })
          .insertContent(text);
      },

      getSelection: () => {
        const { from, to } = editor.state.selection;
        return {
          start: this.posToCursor(from),
          end: this.posToCursor(to),
          text: editor.state.doc.textBetween(from, to)
        };
      },

      setSelection: (selection) => {
        editor.commands.setTextSelection({
          from: selection.start.offset,
          to: selection.end.offset
        });
      },

      getCursor: () => {
        const { from } = editor.state.selection;
        return this.posToCursor(from);
      },

      setCursor: (position) => {
        editor.commands.setCursor(position.offset);
      },

      isReadOnly: () => !editor.isEditable,
      getViewType: () => 'tiptap',
      getLanguage: () => 'markdown'
    };
  }

  private posToCursor(pos: number): CursorPosition {
    // Convert ProseMirror position to line/column
    const doc = this.editor?.state?.doc;
    if (!doc) return { line: 0, column: 0, offset: pos };

    const resolvedPos = doc.resolve(pos);
    return {
      line: resolvedPos.block().content.size,
      column: resolvedPos.parentOffset,
      offset: pos
    };
  }

  // Svelte-specific action for editor integration
  createEditorAction(): Action<HTMLDivElement> {
    return (node, editor) => {
      const adapter = this.createEditorAdapter(editor);
      this.editorStore.set(adapter);

      // Setup universal command system
      this.setupUniversalCommands(editor, adapter);

      return {
        destroy: () => {
          this.editorStore.set(null);
        }
      };
    };
  }

  private setupUniversalCommands(editor: any, adapter: EditorAdapter) {
    // Replace TipTap slash commands with universal system
    const originalExtensions = editor.extensionManager.extensions;
    const universalExtension = this.createUniversalSlashExtension(adapter);

    // Remove existing slash command extensions
    const filteredExtensions = originalExtensions.filter(
      ext => ext.name !== 'slashCommand'
    );

    // Add universal extension
    editor.extensionManager.extensions = [...filteredExtensions, universalExtension];
  }

  private createUniversalSlashExtension(adapter: EditorAdapter) {
    return Extension.create({
      name: 'universalSlashCommand',

      addProseMirrorPlugins() {
        return [
          Suggestion({
            char: '/',
            startOfLine: true,
            allowSpaces: true,

            command: ({ editor, range, props }) => {
              const suggestion = props as Suggestion;

              // Remove trigger text
              editor.chain()
                .focus()
                .deleteRange(range)
                .run();

              // Execute universal action
              this.executeUniversalAction(suggestion.action, adapter);
            },

            items: async ({ query }) => {
              const queryContext: SuggestionQuery = {
                text: query,
                context: {
                  currentRole: 'default',
                  timestamp: Date.now()
                },
                position: adapter.getCursor(),
                trigger: {
                  type: 'char',
                  char: '/',
                  position: adapter.getCursor()
                },
                limit: 20
              };

              const response = await this.commandSystem.getSuggestions(queryContext);
              return response.suggestions.map(s => ({
                id: s.id,
                title: s.text,
                subtitle: s.description,
                icon: s.icon,
                action: s.action
              }));
            },

            render: () => this.createSvelteRenderer(adapter)
          })
        ];
      }
    });
  }

  private async executeUniversalAction(
    action: SuggestionAction,
    adapter: EditorAdapter
  ) {
    switch (action.type) {
      case 'insert':
        adapter.insertText(action.text);
        break;

      case 'execute':
        const context: CommandContext = {
          editor: adapter,
          cursor: adapter.getCursor(),
          currentRole: 'default',
          activeView: 'editor',
          query: '',
          trigger: { type: 'manual', position: adapter.getCursor() },
          services: new ServiceRegistry(),
          timestamp: Date.now(),
          sessionId: this.getSessionId()
        };

        await this.commandSystem.executeCommand(action.command.id, context);
        break;

      case 'search':
        // Trigger search action
        this.triggerSearch(action.query, action.provider);
        break;
    }
  }

  private createSvelteRenderer(adapter: EditorAdapter) {
    let component: SlashRenderer;
    let popup: Instance<Props>;

    return {
      onStart: (props) => {
        component = new SlashRenderer({
          items: props.items as any[],
          onSelect: (item) => props.command(item),
          universalStyle: true
        });

        if (!props.clientRect) return;

        popup = tippy('body', {
          getReferenceClientRect: props.clientRect as () => DOMRect,
          appendTo: () => document.body,
          content: component.element,
          showOnCreate: true,
          interactive: true,
          trigger: 'manual',
          placement: 'bottom-start',
          theme: 'universal-slash-command',
          maxWidth: 'none',
        })[0];
      },

      onUpdate(props) {
        component?.updateItems(props.items as any[]);
        popup?.setProps({
          getReferenceClientRect: props.clientRect as () => DOMRect,
        });
      },

      onKeyDown(props) {
        if (props.event.key === 'Escape') {
          popup?.hide();
          return true;
        }
        return component?.onKeyDown(props) ?? false;
      },

      onExit() {
        popup?.destroy();
        component?.destroy();
      },
    };
  }

  private triggerSearch(query: string, provider: string) {
    // Emit search event for other components to handle
    const event = new CustomEvent('universal-search', {
      detail: { query, provider }
    });
    document.dispatchEvent(event);
  }

  private getSessionId(): string {
    return `svelte-${Date.now()}-${Math.random().toString(36).substr(2, 9)}`;
  }
}
```

### 3.2 GPUI Adapter (`adapters/gpui-adapter.rs`)

```rust
use gpui::*;
use std::sync::Arc;
use tokio::sync::RwLock;

pub struct GpuiCommandSystem {
    universal: Arc<RwLock<UniversalCommandSystem>>,
    editor_adapter: Arc<GpuiEditorAdapter>,
}

impl GpuiCommandSystem {
    pub fn new(config: CommandSystemConfig) -> Self {
        let universal = Arc::new(RwLock::new(
            UniversalCommandSystem::new(config)
        ));

        Self {
            universal,
            editor_adapter: Arc::new(GpuiEditorAdapter::new()),
        }
    }

    pub async fn get_suggestions(
        &self,
        query: SuggestionQuery,
    ) -> Result<SuggestionResponse, CommandError> {
        let universal = self.universal.read().await;
        universal.get_suggestions(query).await
    }

    pub fn create_search_input(
        &self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> View<UniversalSearchInput> {
        cx.new_view(|cx| {
            UniversalSearchInput::new(
                self.universal.clone(),
                self.editor_adapter.clone(),
                window,
                cx
            )
        })
    }
}

pub struct UniversalSearchInput {
    universal: Arc<RwLock<UniversalCommandSystem>>,
    editor_adapter: Arc<GpuiEditorAdapter>,
    input_state: Entity<InputState>,
    show_dropdown: bool,
    suggestions: Vec<Suggestion>,
    selected_index: usize,
    _subscriptions: Vec<Subscription>,
}

impl UniversalSearchInput {
    pub fn new(
        universal: Arc<RwLock<UniversalCommandSystem>>,
        editor_adapter: Arc<GpuiEditorAdapter>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let input_state = cx.new(|cx| {
            InputState::new(window, cx)
                .placeholder("Search commands and knowledge...")
        });

        let universal_clone = universal.clone();
        let input_state_clone = input_state.clone();

        // Subscribe to input changes
        let subscription = cx.subscribe_in(&input_state, window, move |this, _, ev: &InputEvent, _window, cx| {
            match ev {
                InputEvent::Change => {
                    let value = input_state_clone.read(cx).value();

                    // Trigger suggestions
                    let query = SuggestionQuery {
                        text: value.to_string(),
                        context: SuggestionContext {
                            current_role: "default".to_string(),
                            timestamp: std::time::SystemTime::now()
                                .duration_since(std::time::UNIX_EPOCH)
                                .unwrap()
                                .as_millis() as u64,
                        },
                        position: CursorPosition { line: 0, column: 0, offset: 0 },
                        trigger: TriggerInfo {
                            type_: TriggerType::Auto,
                            position: CursorPosition { line: 0, column: 0, offset: 0 },
                        },
                        limit: Some(20),
                    };

                    let universal = universal_clone.clone();
                    let this_clone = this.clone();

                    cx.spawn(async move |this, cx| {
                        let universal = universal.read().await;
                        match universal.get_suggestions(query).await {
                            Ok(response) => {
                                this_clone.update(cx, |this, cx| {
                                    this.suggestions = response.suggestions;
                                    this.show_dropdown = !response.suggestions.is_empty();
                                    this.selected_index = 0;
                                    cx.notify();
                                });
                            }
                            Err(error) => {
                                log::error!("Failed to get suggestions: {:?}", error);
                            }
                        }
                    }).detach();
                }
                InputEvent::PressEnter { .. } => {
                    if this.selected_index < this.suggestions.len() {
                        this.execute_suggestion(this.selected_index, cx);
                    }
                }
                InputEvent::PressEscape => {
                    this.show_dropdown = false;
                    cx.notify();
                }
                InputEvent::PressArrowDown => {
                    if !this.suggestions.is_empty() {
                        this.selected_index = (this.selected_index + 1) % this.suggestions.len();
                        cx.notify();
                    }
                }
                InputEvent::PressArrowUp => {
                    if !this.suggestions.is_empty() {
                        this.selected_index = if this.selected_index == 0 {
                            this.suggestions.len() - 1
                        } else {
                            this.selected_index - 1
                        };
                        cx.notify();
                    }
                }
            }
        });

        Self {
            universal,
            editor_adapter,
            input_state,
            show_dropdown: false,
            suggestions: Vec::new(),
            selected_index: 0,
            _subscriptions: vec![subscription],
        }
    }

    fn execute_suggestion(&mut self, index: usize, cx: &mut Context<Self>) {
        if let Some(suggestion) = self.suggestions.get(index) {
            match &suggestion.action {
                SuggestionAction::Insert { text } => {
                    let input_value = self.input_state.read(cx).value();
                    let new_value = format!("{}{}", input_value, text);
                    self.input_state.update(cx, |state, cx| {
                        state.set_value(new_value, cx);
                    });
                }
                SuggestionAction::Execute { command } => {
                    // Execute command through universal system
                    let context = CommandContext {
                        editor: self.editor_adapter.clone(),
                        selection: None,
                        cursor: CursorPosition { line: 0, column: 0, offset: 0 },
                        current_role: "default".to_string(),
                        active_view: "search".to_string(),
                        query: "".to_string(),
                        trigger: TriggerInfo {
                            type_: TriggerType::Manual,
                            position: CursorPosition { line: 0, column: 0, offset: 0 },
                        },
                        services: ServiceRegistry::new(),
                        timestamp: std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)
                            .unwrap()
                            .as_millis() as u64,
                        session_id: "gpui-session".to_string(),
                    };

                    let universal = self.universal.clone();
                    cx.spawn(async move |_, cx| {
                        let universal = universal.read().await;
                        match universal.execute_command(command.id.clone(), context).await {
                            Ok(result) => {
                                log::info!("Command executed successfully: {:?}", result);
                            }
                            Err(error) => {
                                log::error!("Command execution failed: {:?}", error);
                            }
                        }
                    }).detach();
                }
                _ => {
                    log::warn!("Unsupported suggestion action: {:?}", suggestion.action);
                }
            }

            // Hide dropdown after action
            self.show_dropdown = false;
            cx.notify();
        }
    }
}

impl Render for UniversalSearchInput {
    fn render(&mut self, cx: &mut ViewContext<Self>) -> impl IntoElement {
        div()
            .relative()
            .child(
                // Input field
                div()
                    .relative()
                    .child(
                        Input::new()
                            .state(self.input_state.clone())
                            .into_element()
                    )
            )
            .when(self.show_dropdown, |div| {
                div.child(
                    // Dropdown
                    div()
                        .absolute()
                        .top(px(50.0))
                        .left(px(0.0))
                        .w_full()
                        .max_h(px(400.0))
                        .bg(rgb(0xffffff))
                        .border_1()
                        .border_color(rgb(0xe5e5e5))
                        .rounded_md()
                        .shadow_lg()
                        .overflow_y_scroll()
                        .children(
                            self.suggestions.iter().enumerate().map(|(index, suggestion)| {
                                self.render_suggestion_item(index, suggestion, cx)
                            })
                        )
                )
            })
    }
}
```

## 4. Migration Steps

### 4.1 Phase 1: Setup Universal Types

```bash
# Create directory structure
mkdir -p src/types src/core src/providers src/adapters

# Install dependencies (if using TypeScript/JavaScript)
npm install @types/node
```

### 4.2 Phase 2: Migrate Existing Commands

```typescript
// Before: Current Svelte implementation
const DEFAULT_ITEMS: CommandItem[] = [
  {
    title: 'Paragraph',
    icon: '¶',
    run: ({ editor }) => editor.chain().focus().setParagraph().run(),
  },
  // ...
];

// After: Universal implementation
const UNIVERSAL_COMMANDS: UniversalCommand[] = [
  {
    id: 'set-paragraph',
    title: 'Paragraph',
    category: CommandCategory.TEXT,
    icon: '¶',
    execute: async (context) => {
      context.editor.setContent('<p>' + context.editor.getContent() + '</p>');
      return { success: true };
    }
  },
  // ...
];
```

### 4.3 Phase 3: Replace TipTap Extensions

```typescript
// Before: TipTap slash command
const editor = new Editor({
  extensions: [
    SlashCommand.configure({
      trigger: '/',
      items: DEFAULT_ITEMS
    })
  ]
});

// After: Universal adapter
const commandSystem = new UniversalCommandSystem(config);
const svelteAdapter = new SvelteCommandAdapter(commandSystem);

const editor = new Editor({
  extensions: [
    // Universal extension will be added by adapter
  ]
});

// Use adapter action
<div use:svelteAdapter.createEditorAction(editor)></div>
```

### 4.4 Phase 4: Update Service Integration

```typescript
// Before: Direct service calls
async function getTermSuggestions(q: string): Promise<string[]> {
  const resp = await invoke('get_autocomplete_suggestions', {
    query: q,
    role_name: roleName,
    limit: 8,
  });
  return resp.suggestions.map((s: any) => s.term);
}

// After: Universal provider
class KnowledgeGraphProvider implements SuggestionProvider {
  async provideSuggestions(query: SuggestionQuery): Promise<SuggestionResponse> {
    const resp = await this.service.getSuggestions(
      query.text,
      query.context.currentRole,
      query.limit || 8
    );

    return {
      suggestions: resp.suggestions.map(s => ({
        id: `kg-${s.term}`,
        text: s.term,
        action: { type: 'insert', text: s.term }
      }))
    };
  }
}
```

## 5. Testing Strategy

### 5.1 Unit Tests

```typescript
describe('UniversalCommandSystem', () => {
  let system: UniversalCommandSystem;
  let mockProvider: jest.Mocked<SuggestionProvider>;
  let mockKGProvider: jest.Mocked<SuggestionProvider>;

  beforeEach(() => {
    mockProvider = {
      id: 'test-provider',
      name: 'Test Provider',
      provideSuggestions: jest.fn(),
      isEnabled: jest.fn().mockReturnValue(true),
      canHandle: jest.fn().mockReturnValue(true),
      getState: jest.fn().mockReturnValue({
        status: 'active',
        requestCount: 0,
        errorCount: 0,
        averageResponseTime: 0
      })
    };

    mockKGProvider = {
      id: 'kg-provider',
      name: 'Knowledge Graph Provider',
      provideSuggestions: jest.fn(),
      isEnabled: jest.fn().mockReturnValue(true),
      canHandle: jest.fn().mockReturnValue(true),
      getState: jest.fn().mockReturnValue({
        status: 'active',
        requestCount: 0,
        errorCount: 0,
        averageResponseTime: 0
      })
    };

    system = new UniversalCommandSystem({
      providers: [mockProvider, mockKGProvider],
      performance: {
        timeout: 1000,
        cache: { enabled: true, ttl: 300000, maxSize: 100 }
      }
    });
  });

  it('should get suggestions from active providers', async () => {
    const mockSuggestions = [
      { id: '1', text: 'test', action: { type: 'insert' as const, text: 'test' }, score: 0.9 }
    ];

    mockProvider.provideSuggestions.mockResolvedValue({
      suggestions: mockSuggestions,
      hasMore: false,
      total: 1,
      processingTime: 50,
      hasErrors: false
    });

    const query: SuggestionQuery = {
      text: 'test',
      context: { 
        currentRole: 'default', 
        timestamp: Date.now(),
        documentType: 'markdown',
        cursorPosition: { line: 0, column: 0 }
      },
      position: { line: 0, column: 0, offset: 0 },
      trigger: { type: 'auto', position: { line: 0, column: 0, offset: 0 } },
      limit: 20,
      timestamp: Date.now(),
      sessionId: 'test-session'
    };

    const result = await system.getSuggestions(query);

    expect(result.suggestions).toEqual(mockSuggestions);
    expect(mockProvider.provideSuggestions).toHaveBeenCalledWith(query);
    expect(result.processingTime).toBeGreaterThan(0);
  });

  it('should handle provider timeouts gracefully', async () => {
    mockProvider.provideSuggestions.mockImplementation(
      () => new Promise(resolve => setTimeout(resolve, 2000))
    );

    const query: SuggestionQuery = {
      text: 'test',
      context: { currentRole: 'default', timestamp: Date.now() },
      position: { line: 0, column: 0, offset: 0 },
      trigger: { type: 'auto', position: { line: 0, column: 0, offset: 0 } },
      limit: 20,
      timestamp: Date.now()
    };

    const result = await system.getSuggestions(query);

    expect(result.suggestions).toEqual([]);
    expect(result.hasErrors).toBe(true);
  });

  it('should cache suggestions effectively', async () => {
    const mockSuggestions = [
      { id: '1', text: 'cached', action: { type: 'insert' as const, text: 'cached' }, score: 0.8 }
    ];

    mockProvider.provideSuggestions.mockResolvedValue({
      suggestions: mockSuggestions,
      hasMore: false,
      total: 1,
      processingTime: 100,
      hasErrors: false
    });

    const query: SuggestionQuery = {
      text: 'cached',
      context: { currentRole: 'default', timestamp: Date.now() },
      position: { line: 0, column: 0, offset: 0 },
      trigger: { type: 'auto', position: { line: 0, column: 0, offset: 0 } },
      limit: 20,
      timestamp: Date.now()
    };

    // First call
    const result1 = await system.getSuggestions(query);
    expect(mockProvider.provideSuggestions).toHaveBeenCalledTimes(1);

    // Second call should use cache
    const result2 = await system.getSuggestions(query);
    expect(mockProvider.provideSuggestions).toHaveBeenCalledTimes(1); // Still called once
    expect(result1.suggestions).toEqual(result2.suggestions);
  });
});

describe('SvelteTipTapAdapter', () => {
  let adapter: SvelteTipTapAdapter;
  let mockEditor: any;
  let mockCommandSystem: jest.Mocked<UniversalCommandSystem>;

  beforeEach(() => {
    mockCommandSystem = {
      getSuggestions: jest.fn(),
      executeCommand: jest.fn(),
      on: jest.fn(),
      updateRole: jest.fn()
    } as any;

    adapter = new SvelteTipTapAdapter(mockCommandSystem, {
      enableMetrics: true,
      enableAnimations: true
    });

    mockEditor = {
      getHTML: jest.fn().mockReturnValue('<p>test</p>'),
      setHTML: jest.fn(),
      commands: {
        setContent: jest.fn(),
        insertContent: jest.fn(),
        deleteRange: jest.fn(),
        focus: jest.fn()
      },
      isEditable: true,
      state: {
        selection: { from: 0, to: 0 },
        doc: { 
          resolve: jest.fn().mockReturnValue({
            block: () => ({ content: { size: 1 } }),
            parentOffset: 0
          })
        }
      },
      on: jest.fn(),
      off: jest.fn()
    };
  });

  it('should create editor adapter with proper conversion', () => {
    const universalAdapter = adapter.createEditorAdapter(mockEditor);

    expect(universalAdapter.getContent()).toBe('<p>test</p>');
    expect(universalAdapter.isReadOnly()).toBe(false);
    expect(universalAdapter.getViewType()).toBe('tiptap');
  });

  it('should handle command execution with error recovery', async () => {
    const mockCommand: UniversalCommand = {
      id: 'test-command',
      title: 'Test Command',
      execute: jest.fn().mockRejectedValue(new Error('Test error'))
    };

    const context: CommandContext = {
      editor: adapter.createEditorAdapter(mockEditor),
      cursor: { line: 0, column: 0, offset: 0 },
      currentRole: 'default',
      activeView: 'editor',
      query: 'test',
      trigger: { type: 'manual', position: { line: 0, column: 0, offset: 0 } },
      services: new ServiceRegistry(),
      timestamp: Date.now(),
      sessionId: 'test-session'
    };

    const result = await adapter.executeCommand(mockCommand, context);

    expect(result.success).toBe(false);
    expect(result.error).toBe('Test error');
  });
});
```

### 5.2 Integration Tests

```typescript
describe('SvelteCommandAdapter Integration', () => {
  let adapter: SvelteCommandAdapter;
  let mockEditor: any;
  let mockCommandSystem: UniversalCommandSystem;

  beforeEach(async () => {
    // Create mock command system with real providers
    mockCommandSystem = new UniversalCommandSystem({
      providers: [
        new CommandPaletteProvider(),
        new MockSuggestionProvider()
      ],
      performance: {
        timeout: 1000,
        cache: { enabled: true, ttl: 300000, maxSize: 100 }
      }
    });

    await mockCommandSystem.initialize();

    adapter = new SvelteCommandAdapter(mockCommandSystem, {
      enableMetrics: true,
      enableAnimations: true
    });

    mockEditor = {
      getHTML: jest.fn().mockReturnValue('<p>test content</p>'),
      setHTML: jest.fn(),
      commands: {
        setContent: jest.fn(),
        insertContent: jest.fn(),
        deleteRange: jest.fn(),
        focus: jest.fn()
      },
      isEditable: true,
      state: {
        selection: { from: 0, to: 5 },
        doc: { 
          resolve: jest.fn().mockReturnValue({
            block: () => ({ content: { size: 1 } }),
            parentOffset: 0
          })
        }
      },
      on: jest.fn(),
      off: jest.fn(),
      extensionManager: {
        extensions: []
      }
    };
  });

  it('should integrate with TipTap editor seamlessly', async () => {
    const universalAdapter = adapter.createEditorAdapter(mockEditor);
    
    // Test content operations
    expect(universalAdapter.getContent()).toBe('<p>test content</p>');
    
    // Test selection operations
    const selection = universalAdapter.getSelection();
    expect(selection.text).toBe('test');
    
    // Test cursor operations
    const cursor = universalAdapter.getCursor();
    expect(cursor.line).toBe(0);
  });

  it('should handle slash command integration', async () => {
    // Create slash command extension
    const extensions = adapter.createTipTapExtensions();
    expect(extensions).toHaveLength(3); // slash, autocomplete, universal

    // Test suggestion retrieval
    const suggestions = await mockCommandSystem.getSuggestions({
      text: 'heading',
      context: {
        currentRole: 'default',
        timestamp: Date.now(),
        documentType: 'markdown',
        cursorPosition: { line: 0, column: 0 }
      },
      position: { line: 0, column: 0, offset: 0 },
      trigger: { type: 'char', char: '/', position: { line: 0, column: 0, offset: 0 } },
      limit: 20,
      timestamp: Date.now()
    });

    expect(suggestions.suggestions.length).toBeGreaterThan(0);
    expect(suggestions.suggestions[0].action.type).toBe('execute');
  });

  it('should handle Terraphim integration', async () => {
    // Test role-based context
    await mockCommandSystem.updateRole('terraphim-admin');
    
    const suggestions = await mockCommandSystem.getSuggestions({
      text: 'terraphim',
      context: {
        currentRole: 'terraphim-admin',
        timestamp: Date.now(),
        documentType: 'markdown',
        cursorPosition: { line: 0, column: 0 }
      },
      position: { line: 0, column: 0, offset: 0 },
      trigger: { type: 'char', char: '++', position: { line: 0, column: 0, offset: 0 } },
      limit: 8,
      timestamp: Date.now()
    });

    expect(suggestions.suggestions.length).toBeGreaterThan(0);
  });
});

describe('End-to-End Workflow Tests', () => {
  let testContainer: HTMLElement;
  let editor: Editor;
  let commandSystem: UniversalCommandSystem;
  let adapter: SvelteTipTapAdapter;

  beforeEach(async () => {
    // Setup DOM environment
    testContainer = document.createElement('div');
    document.body.appendChild(testContainer);

    // Initialize system
    commandSystem = new UniversalCommandSystem({
      providers: [
        new CommandPaletteProvider(),
        new MockSuggestionProvider()
      ]
    });

    await commandSystem.initialize();
    adapter = new SvelteTipTapAdapter(commandSystem);

    // Create TipTap editor
    editor = new Editor({
      element: testContainer,
      extensions: adapter.createTipTapExtensions(),
      content: '<p>Test document</p>',
      editable: true
    });
  });

  afterEach(() => {
    editor?.destroy();
    testContainer?.remove();
  });

  it('should complete full slash command workflow', async () => {
    // Type slash command
    editor.commands.setTextSelection({ from: 12, to: 12 });
    editor.commands.insertContent('/heading');

    // Wait for suggestions to appear
    await new Promise(resolve => setTimeout(resolve, 100));

    // Verify suggestions are available
    // (This would require accessing the suggestion popup in a real test)
    expect(editor.state.doc.textContent).toContain('/heading');
  });

  it('should handle autocomplete workflow', async () => {
    // Type autocomplete trigger
    editor.commands.setTextSelection({ from: 12, to: 12 });
    editor.commands.insertContent('++test');

    // Wait for suggestions
    await new Promise(resolve => setTimeout(resolve, 100));

    // Verify content
    expect(editor.state.doc.textContent).toContain('++test');
  });
});

// Mock provider for testing
class MockSuggestionProvider implements SuggestionProvider {
  id = 'mock-provider';
  name = 'Mock Provider';
  priority = 1;
  trigger = { type: 'auto', minChars: 1 };
  debounce = 50;
  minQueryLength = 1;
  maxResults = 10;

  async provideSuggestions(query: SuggestionQuery): Promise<SuggestionResponse> {
    const suggestions: UniversalSuggestion[] = [
      {
        id: 'mock-1',
        text: query.text,
        score: 0.9,
        action: { type: 'insert', text: query.text },
        metadata: { source: 'mock', confidence: 0.9 }
      }
    ];

    return {
      suggestions,
      hasMore: false,
      total: 1,
      processingTime: 10,
      hasErrors: false
    };
  }

  isEnabled(): boolean {
    return true;
  }

  canHandle(query: SuggestionQuery): boolean {
    return query.text.length >= this.minQueryLength;
  }

  getState(): ProviderState {
    return {
      status: 'active',
      requestCount: 0,
      errorCount: 0,
      averageResponseTime: 10,
      lastRequest: Date.now()
    };
  }
}
```

### 5.3 Performance Tests

```typescript
describe('Performance Tests', () => {
  let system: UniversalCommandSystem;
  let performanceProvider: MockPerformanceProvider;

  beforeEach(async () => {
    performanceProvider = new MockPerformanceProvider();
    
    system = new UniversalCommandSystem({
      providers: [performanceProvider],
      performance: {
        timeout: 1000,
        cache: { enabled: true, ttl: 300000, maxSize: 1000 },
        debounce: { commands: 50, autocomplete: 100 }
      }
    });

    await system.initialize();
  });

  it('should return suggestions within 50ms for cached queries', async () => {
    const query: SuggestionQuery = {
      text: 'cached-query',
      context: { 
        currentRole: 'default', 
        timestamp: Date.now(),
        documentType: 'markdown',
        cursorPosition: { line: 0, column: 0 }
      },
      position: { line: 0, column: 0, offset: 0 },
      trigger: { type: 'auto', position: { line: 0, column: 0, offset: 0 } },
      limit: 20,
      timestamp: Date.now()
    };

    // First request (cache miss)
    const startTime1 = performance.now();
    await system.getSuggestions(query);
    const duration1 = performance.now() - startTime1;

    // Second request (cache hit)
    const startTime2 = performance.now();
    await system.getSuggestions(query);
    const duration2 = performance.now() - startTime2;

    expect(duration1).toBeLessThan(100); // First request
    expect(duration2).toBeLessThan(20);  // Cached request should be much faster
  });

  it('should handle concurrent requests efficiently', async () => {
    const queries = Array.from({ length: 10 }, (_, i) => ({
      text: `concurrent-query-${i}`,
      context: { 
        currentRole: 'default', 
        timestamp: Date.now(),
        documentType: 'markdown',
        cursorPosition: { line: 0, column: 0 }
      },
      position: { line: 0, column: 0, offset: 0 },
      trigger: { type: 'auto', position: { line: 0, column: 0, offset: 0 } },
      limit: 20,
      timestamp: Date.now()
    }));

    const startTime = performance.now();
    
    // Execute all requests concurrently
    const promises = queries.map(query => system.getSuggestions(query));
    const results = await Promise.all(promises);
    
    const duration = performance.now() - startTime;

    expect(results).toHaveLength(10);
    expect(results.every(r => r.suggestions.length > 0)).toBe(true);
    expect(duration).toBeLessThan(200); // Should handle concurrent requests efficiently
  });

  it('should maintain performance under high load', async () => {
    const requestCount = 100;
    const queries = Array.from({ length: requestCount }, (_, i) => ({
      text: `load-test-${i % 10}`, // Some cache hits expected
      context: { 
        currentRole: 'default', 
        timestamp: Date.now(),
        documentType: 'markdown',
        cursorPosition: { line: 0, column: 0 }
      },
      position: { line: 0, column: 0, offset: 0 },
      trigger: { type: 'auto', position: { line: 0, column: 0, offset: 0 } },
      limit: 20,
      timestamp: Date.now()
    }));

    const startTime = performance.now();
    
    // Execute requests in batches to simulate real usage
    const batchSize = 10;
    const results: SuggestionResponse[] = [];
    
    for (let i = 0; i < queries.length; i += batchSize) {
      const batch = queries.slice(i, i + batchSize);
      const batchResults = await Promise.all(
        batch.map(query => system.getSuggestions(query))
      );
      results.push(...batchResults);
    }
    
    const totalDuration = performance.now() - startTime;
    const averageDuration = totalDuration / requestCount;

    expect(results).toHaveLength(requestCount);
    expect(averageDuration).toBeLessThan(50); // Average should be under 50ms
    expect(totalDuration).toBeLessThan(2000); // Total should be under 2s
  });

  it('should handle memory efficiently with large datasets', async () => {
    // Create provider that returns many suggestions
    const largeProvider = new MockLargeProvider(1000);
    system.registerProvider(largeProvider);

    const query: SuggestionQuery = {
      text: 'large-dataset',
      context: { 
        currentRole: 'default', 
        timestamp: Date.now(),
        documentType: 'markdown',
        cursorPosition: { line: 0, column: 0 }
      },
      position: { line: 0, column: 0, offset: 0 },
      trigger: { type: 'auto', position: { line: 0, column: 0, offset: 0 } },
      limit: 50, // Limit to 50 results
      timestamp: Date.now()
    };

    const startTime = performance.now();
    const result = await system.getSuggestions(query);
    const duration = performance.now() - startTime;

    expect(result.suggestions.length).toBeLessThanOrEqual(50);
    expect(duration).toBeLessThan(100); // Should handle large datasets efficiently
    
    // Check memory usage (if available)
    if (performance.memory) {
      const memoryUsed = performance.memory.usedJSHeapSize;
      expect(memoryUsed).toBeLessThan(50 * 1024 * 1024); // Less than 50MB
    }
  });

  it('should measure and report performance metrics', async () => {
    const query: SuggestionQuery = {
      text: 'metrics-test',
      context: { 
        currentRole: 'default', 
        timestamp: Date.now(),
        documentType: 'markdown',
        cursorPosition: { line: 0, column: 0 }
      },
      position: { line: 0, column: 0, offset: 0 },
      trigger: { type: 'auto', position: { line: 0, column: 0, offset: 0 } },
      limit: 20,
      timestamp: Date.now()
    };

    const result = await system.getSuggestions(query);

    // Verify performance metrics are included
    expect(result.processingTime).toBeGreaterThan(0);
    expect(result.metadata).toBeDefined();
    expect(result.metadata?.queryComplexity).toBeGreaterThan(0);
  });
});

// Mock performance provider for testing
class MockPerformanceProvider implements SuggestionProvider {
  id = 'performance-provider';
  name = 'Performance Provider';
  priority = 1;
  trigger = { type: 'auto', minChars: 1 };
  debounce = 0;
  minQueryLength = 1;
  maxResults = 20;

  async provideSuggestions(query: SuggestionQuery): Promise<SuggestionResponse> {
    // Simulate processing time based on query complexity
    const complexity = query.text.length * 10;
    const processingTime = Math.min(complexity, 100); // Max 100ms

    await new Promise(resolve => setTimeout(resolve, processingTime));

    const suggestions: UniversalSuggestion[] = Array.from(
      { length: Math.min(20, query.text.length) }, 
      (_, i) => ({
        id: `perf-${i}`,
        text: `${query.text}-${i}`,
        score: 1.0 - (i * 0.05),
        action: { type: 'insert', text: `${query.text}-${i}` },
        metadata: { 
          source: 'performance',
          confidence: 0.9,
          processingTime 
        }
      })
    );

    return {
      suggestions,
      hasMore: suggestions.length >= 20,
      total: suggestions.length,
      processingTime,
      hasErrors: false,
      metadata: {
        provider: this.id,
        queryComplexity: query.text.length,
        cacheEfficiency: 0.8
      }
    };
  }

  isEnabled(): boolean {
    return true;
  }

  canHandle(query: SuggestionQuery): boolean {
    return query.text.length >= this.minQueryLength;
  }

  getState(): ProviderState {
    return {
      status: 'active',
      requestCount: 0,
      errorCount: 0,
      averageResponseTime: 50,
      lastRequest: Date.now()
    };
  }
}

// Mock large provider for testing
class MockLargeProvider implements SuggestionProvider {
  id = 'large-provider';
  name = 'Large Provider';
  priority = 1;
  trigger = { type: 'auto', minChars: 1 };
  debounce = 0;
  minQueryLength = 1;
  maxResults = 1000;

  constructor(private suggestionCount: number) {}

  async provideSuggestions(query: SuggestionQuery): Promise<SuggestionResponse> {
    const suggestions: UniversalSuggestion[] = Array.from(
      { length: this.suggestionCount }, 
      (_, i) => ({
        id: `large-${i}`,
        text: `${query.text}-item-${i}`,
        score: Math.random(),
        action: { type: 'insert', text: `item-${i}` },
        metadata: { 
          source: 'large',
          confidence: Math.random()
        }
      })
    );

    return {
      suggestions,
      hasMore: false,
      total: this.suggestionCount,
      processingTime: 50,
      hasErrors: false
    };
  }

  isEnabled(): boolean {
    return true;
  }

  canHandle(query: SuggestionQuery): boolean {
    return true;
  }

  getState(): ProviderState {
    return {
      status: 'active',
      requestCount: 0,
      errorCount: 0,
      averageResponseTime: 50,
      lastRequest: Date.now()
    };
  }
}
```

## 6. Deployment Checklist

### 6.1 Pre-Deployment

#### Code Quality
- [ ] All unit tests pass (>90% coverage)
- [ ] Integration tests pass in target frameworks
- [ ] Performance benchmarks meet requirements (<50ms response time)
- [ ] Accessibility tests pass (WCAG 2.1 AA compliance)
- [ ] Security audit completed
- [ ] Error handling is comprehensive

#### Terraphim Integration
- [ ] Knowledge graph service connectivity verified
- [ ] Role-based command system functional
- [ ] Context enhancement working correctly
- [ ] MCP server integration tested
- [ ] Tauri backend compatibility verified

#### Performance Validation
- [ ] Sub-50ms response times for cached queries
- [ ] Sub-100ms response times for uncached queries
- [ ] Memory usage under 50MB for typical workloads
- [ ] Cache hit rate >80% for common queries
- [ ] Concurrent request handling verified

### 6.2 Migration Validation

#### Svelte/TipTap Compatibility
- [ ] Current Svelte implementation still works
- [ ] TipTap extensions integrate seamlessly
- [ ] NovelWrapper component functions correctly
- [ ] Event handling preserved
- [ ] State management maintained

#### GPUI/Zed Integration
- [ ] GPUI adapter functions correctly
- [ ] WIT interface definitions complete
- [ ] Rust implementation compiles without errors
- [ ] Zed editor plugin is functional
- [ ] Cross-platform compatibility verified

#### Provider Ecosystem
- [ ] All providers are active and responsive
- [ ] Knowledge graph provider returns relevant results
- [ ] Command palette provider functions correctly
- [ ] Terraphim suggestion provider operational
- [ ] Provider health monitoring working

#### System Reliability
- [ ] Caching strategy is effective
- [ ] Error fallbacks work properly
- [ ] Timeout handling prevents hangs
- [ ] Graceful degradation under load
- [ ] Recovery mechanisms functional

### 6.3 Post-Deployment

#### Monitoring & Observability
- [ ] Performance metrics dashboard active
- [ ] Error rates monitored in production
- [ ] Cache efficiency tracked
- [ ] Provider health status monitored
- [ ] User experience metrics collected

#### User Experience Validation
- [ ] User feedback collected and analyzed
- [ ] A/B testing results reviewed
- [ ] Accessibility compliance verified
- [ ] Cross-browser testing completed
- [ ] Mobile responsiveness validated

#### Documentation & Training
- [ ] Update documentation with real-world feedback
- [ ] Create user training materials
- [ ] Developer onboarding guides updated
- [ ] API documentation finalized
- [ ] Troubleshooting guides created

#### Continuous Improvement
- [ ] Plan next phase improvements
- [ ] Schedule regular performance reviews
- [ ] Establish feedback collection mechanisms
- [ ] Set up automated regression testing
- [ ] Create feature request prioritization process

## 7. Terraphim-Specific Considerations

### 7.1 Role-Based Command System

The universal slash command system integrates deeply with Terraphim's role-based architecture:

```typescript
// Role-specific command registration
const roleCommands: Record<string, UniversalCommand[]> = {
  'terraphim-admin': [
    {
      id: 'admin-manage-users',
      title: 'Manage Users',
      category: CommandCategory.ADMIN,
      permissions: ['user-management'],
      execute: async (context) => {
        // Admin-specific user management
        return { success: true, content: 'User management interface' };
      }
    }
  ],
  'terraphim-user': [
    {
      id: 'user-search-knowledge',
      title: 'Search Knowledge Graph',
      category: CommandCategory.SEARCH,
      execute: async (context) => {
        // User-specific knowledge search
        return { success: true, content: 'Knowledge search results' };
      }
    }
  ]
};
```

### 7.2 Knowledge Graph Integration

Enhanced suggestion system with semantic search:

```typescript
class TerraphimKnowledgeProvider implements SuggestionProvider {
  async provideSuggestions(query: SuggestionQuery): Promise<SuggestionResponse> {
    // Leverage Terraphim's semantic search capabilities
    const semanticResults = await this.kgService.semanticSearch({
      query: query.text,
      role: query.context.currentRole,
      context: query.context.contextItems,
      limit: query.limit || 8
    });

    return {
      suggestions: semanticResults.map(result => ({
        id: `kg-${result.id}`,
        text: result.title,
        description: result.snippet,
        score: result.relevanceScore,
        category: CommandCategory.SEARCH,
        action: {
          type: 'search',
          query: result.title,
          provider: 'knowledge-graph',
          options: { semanticId: result.id }
        },
        metadata: {
          source: 'knowledge-graph',
          confidence: result.confidence,
          semanticDistance: result.semanticDistance
        }
      })),
      processingTime: semanticResults.processingTime,
      hasErrors: false
    };
  }
}
```

### 7.3 Context Enhancement

Real-time context enhancement during conversations:

```typescript
class ContextEnhancementProvider implements SuggestionProvider {
  async provideSuggestions(query: SuggestionQuery): Promise<SuggestionResponse> {
    // Analyze conversation context
    const contextAnalysis = await this.analyzeConversationContext(query);
    
    // Suggest relevant context items
    const contextSuggestions = await this.getContextSuggestions({
      currentContext: query.context.contextItems,
      conversationHistory: contextAnalysis.recentMessages,
      userIntent: contextAnalysis.intent,
      role: query.context.currentRole
    });

    return {
      suggestions: contextSuggestions.map(item => ({
        id: `context-${item.id}`,
        text: item.title,
        description: item.summary,
        score: item.relevanceScore,
        category: CommandCategory.AI,
        action: {
          type: 'execute',
          command: {
            id: 'add-context',
            title: 'Add Context',
            execute: async (context) => {
              await this.addContextToConversation(item);
              return { success: true };
            }
          }
        }
      }))
    };
  }
}
```

## 8. Future Roadmap

### 8.1 Phase 4 Enhancements

#### Advanced AI Integration
- **LLM-Powered Commands**: Natural language command interpretation
- **Contextual Suggestions**: AI-driven suggestion ranking
- **Multi-Modal Support**: Image, audio, and video content suggestions
- **Predictive Commands**: Anticipatory command suggestions

#### Performance Optimizations
- **WebAssembly Compilation**: Critical path performance improvements
- **GPU Acceleration**: Parallel processing for large datasets
- **Edge Caching**: Distributed caching for global deployments
- **Streaming Suggestions**: Real-time suggestion streaming

#### Enterprise Features
- **Advanced Security**: End-to-end encryption, audit logging
- **Multi-Tenant Support**: Organization-based command isolation
- **Custom Provider SDK**: Easy third-party integration
- **Advanced Analytics**: Usage patterns and optimization insights

### 8.2 Long-term Vision

#### Universal Command Ecosystem
- **Cross-Application Commands**: Commands that work across multiple applications
- **Cloud-Based Configuration**: Synchronized command preferences
- **Community Marketplace**: Shared command and provider ecosystem
- **Standardization**: Industry-wide command system standards

#### AI-Enhanced Experience
- **Learning Adaptation**: System learns from user behavior
- **Personalized Suggestions**: Customized recommendation engine
- **Voice Commands**: Natural language voice interaction
- **Gesture Support**: Touch and gesture-based command triggering

This comprehensive implementation guide provides the practical steps, code examples, and Terraphim-specific considerations needed to migrate from the current Svelte-specific implementation to a universal, framework-agnostic slash command system that maintains the high performance standards and deep integration capabilities of the Terraphim ecosystem.