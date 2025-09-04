/**
 * Configuration for the Novel editor autocomplete system
 */

export interface AutocompleteConfig {
  /** Enable/disable autocomplete functionality */
  enabled: boolean;
  /** Character that triggers autocomplete suggestions */
  trigger: string;
  /** Alternative trigger characters for different suggestion types */
  altTriggers?: string[];
  /** Minimum number of characters before showing suggestions */
  minLength: number;
  /** Maximum number of suggestions to display */
  maxSuggestions: number;
  /** Debounce delay in milliseconds before making requests */
  debounceDelay: number;
  /** Allow spaces in suggestion queries */
  allowSpaces: boolean;
  /** Enable snippet/description display */
  showSnippets: boolean;
  /** Timeout for autocomplete requests in milliseconds */
  requestTimeout: number;
  /** Maximum number of retry attempts for failed requests */
  maxRetries: number;
  /** Retry delay multiplier for exponential backoff */
  retryDelay: number;
}

export const DEFAULT_AUTOCOMPLETE_CONFIG: AutocompleteConfig = {
  enabled: true,
  trigger: '/',
  altTriggers: ['@', '#'],
  minLength: 1,
  maxSuggestions: 8,
  debounceDelay: 300,
  allowSpaces: false,
  showSnippets: true,
  requestTimeout: 5000,
  maxRetries: 3,
  retryDelay: 1000,
};

export const AUTOCOMPLETE_PRESETS = {
  /** Fast, minimal autocomplete for quick suggestions */
  minimal: {
    ...DEFAULT_AUTOCOMPLETE_CONFIG,
    minLength: 2,
    maxSuggestions: 5,
    debounceDelay: 200,
    showSnippets: false,
  } as AutocompleteConfig,

  /** Comprehensive autocomplete with full features */
  comprehensive: {
    ...DEFAULT_AUTOCOMPLETE_CONFIG,
    minLength: 1,
    maxSuggestions: 12,
    debounceDelay: 400,
    showSnippets: true,
    allowSpaces: true,
  } as AutocompleteConfig,

  /** Fast autocomplete for development/testing */
  development: {
    ...DEFAULT_AUTOCOMPLETE_CONFIG,
    minLength: 1,
    maxSuggestions: 6,
    debounceDelay: 100,
    showSnippets: true,
    requestTimeout: 3000,
    maxRetries: 1,
  } as AutocompleteConfig,

  /** Conservative autocomplete for production */
  production: {
    ...DEFAULT_AUTOCOMPLETE_CONFIG,
    minLength: 2,
    maxSuggestions: 8,
    debounceDelay: 500,
    showSnippets: true,
    requestTimeout: 10000,
    maxRetries: 5,
    retryDelay: 2000,
  } as AutocompleteConfig,
};

/**
 * Get autocomplete configuration based on environment or preference
 */
export function getAutocompleteConfig(
  preset?: keyof typeof AUTOCOMPLETE_PRESETS,
  overrides?: Partial<AutocompleteConfig>
): AutocompleteConfig {
  const baseConfig = preset
    ? AUTOCOMPLETE_PRESETS[preset]
    : DEFAULT_AUTOCOMPLETE_CONFIG;

  return {
    ...baseConfig,
    ...overrides,
  };
}

/**
 * Validate autocomplete configuration
 */
export function validateAutocompleteConfig(config: Partial<AutocompleteConfig>): string[] {
  const errors: string[] = [];

  if (config.minLength !== undefined && config.minLength < 0) {
    errors.push('minLength must be non-negative');
  }

  if (config.maxSuggestions !== undefined && config.maxSuggestions < 1) {
    errors.push('maxSuggestions must be at least 1');
  }

  if (config.debounceDelay !== undefined && config.debounceDelay < 0) {
    errors.push('debounceDelay must be non-negative');
  }

  if (config.requestTimeout !== undefined && config.requestTimeout < 1000) {
    errors.push('requestTimeout must be at least 1000ms');
  }

  if (config.maxRetries !== undefined && config.maxRetries < 0) {
    errors.push('maxRetries must be non-negative');
  }

  if (config.retryDelay !== undefined && config.retryDelay < 100) {
    errors.push('retryDelay must be at least 100ms');
  }

  if (config.trigger && config.trigger.length !== 1) {
    errors.push('trigger must be a single character');
  }

  return errors;
}

/**
 * Environment-specific configuration
 */
export function getEnvironmentConfig(): AutocompleteConfig {
  // Detect environment
  const isDevelopment = typeof window !== 'undefined' &&
    (window.location.hostname === 'localhost' || window.location.hostname === '127.0.0.1');

  const isTauri = typeof window !== 'undefined' &&
    '__TAURI__' in window;

  if (isDevelopment) {
    return getAutocompleteConfig('development');
  }

  if (isTauri) {
    // Tauri apps can be more responsive
    return getAutocompleteConfig('comprehensive', {
      debounceDelay: 200,
      requestTimeout: 3000,
    });
  }

  // Default to production config for web deployments
  return getAutocompleteConfig('production');
}

/**
 * Keyboard shortcuts for autocomplete
 */
export const AUTOCOMPLETE_SHORTCUTS = {
  TRIGGER: 'Type trigger character (default: /)',
  NAVIGATE_UP: '↑ Arrow Up',
  NAVIGATE_DOWN: '↓ Arrow Down',
  SELECT: 'Tab or Enter',
  CANCEL: 'Esc',
  FORCE_REFRESH: 'Ctrl+Space (in editor)',
};

/**
 * Suggestion types and their configurations
 */
export interface SuggestionType {
  id: string;
  name: string;
  icon: string;
  description: string;
  trigger?: string;
  color?: string;
}

export const SUGGESTION_TYPES: Record<string, SuggestionType> = {
  'knowledge-graph': {
    id: 'knowledge-graph',
    name: 'Knowledge Graph',
    icon: '🔗',
    description: 'Terms from your knowledge graph',
    color: '#3b82f6',
  },
  'document': {
    id: 'document',
    name: 'Document',
    icon: '📄',
    description: 'Document titles and content',
    color: '#10b981',
  },
  'role': {
    id: 'role',
    name: 'Role',
    icon: '👤',
    description: 'Role-specific suggestions',
    trigger: '@',
    color: '#8b5cf6',
  },
  'command': {
    id: 'command',
    name: 'Command',
    icon: '⚡',
    description: 'Editor commands and actions',
    trigger: '/',
    color: '#f59e0b',
  },
  'tag': {
    id: 'tag',
    name: 'Tag',
    icon: '🏷️',
    description: 'Content tags and categories',
    trigger: '#',
    color: '#ef4444',
  },
};
