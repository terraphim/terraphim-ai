import { Extension } from '@tiptap/core';
import { PluginKey } from 'prosemirror-state';
import { Suggestion } from '@tiptap/suggestion';

// Define the SuggestionOptions type based on the Suggestion function parameters
interface SuggestionOptions {
  pluginKey?: any;
  editor: any;
  char?: string;
  allowSpaces?: boolean;
  allowToIncludeChar?: boolean;
  allowedPrefixes?: string[];
  startOfLine?: boolean;
  decorationTag?: string;
  decorationClass?: string;
  decorationContent?: string;
  decorationEmptyClass?: string;
  command?: (props: any) => any;
  items?: (props: any) => Promise<any[]>;
  render?: () => any;
  allow?: (props: any) => boolean;
  findSuggestionMatch?: any;
}
import { novelAutocompleteService, type NovelAutocompleteSuggestion } from '../services/novelAutocompleteService';
import tippy, { type Instance, type Props } from 'tippy.js';

export interface TerraphimSuggestionOptions {
  /**
   * Character that triggers the autocomplete
   */
  trigger: string;
  /**
   * PluginKey for this suggestion instance
   */
  pluginKey: PluginKey;
  /**
   * Allow spaces in suggestions
   */
  allowSpaces: boolean;
  /**
   * Maximum number of suggestions to show
   */
  limit: number;
  /**
   * Minimum characters before triggering
   */
  minLength: number;
  /**
   * Debounce delay in milliseconds
   */
  debounce: number;
}

declare module '@tiptap/core' {
  interface Commands<ReturnType> {
    terraphimSuggestion: {
      /**
       * Insert a suggestion
       */
      insertSuggestion: (suggestion: NovelAutocompleteSuggestion) => ReturnType;
    };
  }
}

/**
 * Custom TipTap extension for Terraphim-based autocomplete suggestions
 *
 * This extension integrates with the novelAutocompleteService to provide
 * knowledge graph-based suggestions directly in the Novel editor.
 */
export const TerraphimSuggestion = Extension.create<TerraphimSuggestionOptions>({
  name: 'terraphimSuggestion',

  addOptions() {
    return {
      trigger: '/',
      pluginKey: new PluginKey('terraphimSuggestion'),
      allowSpaces: false,
      limit: 8,
      minLength: 1,
      debounce: 300,
    };
  },

  addCommands() {
    return {
      insertSuggestion:
        (suggestion: NovelAutocompleteSuggestion) =>
        ({ commands, chain }) => {
          return chain()
            .insertContent(suggestion.text)
            .run();
        },
    };
  },

  addProseMirrorPlugins() {
    const suggestion: Partial<SuggestionOptions> = {
      editor: this.editor,
      char: this.options.trigger,
      pluginKey: this.options.pluginKey,
      allowSpaces: this.options.allowSpaces,
      startOfLine: false,

      command: ({ editor, range, props }) => {
        const suggestion = props as NovelAutocompleteSuggestion;

        // Insert the suggestion text, removing the trigger character
        editor
          .chain()
          .focus()
          .insertContentAt(range, suggestion.text + ' ')
          .run();
      },

      items: async ({ query, editor }): Promise<NovelAutocompleteSuggestion[]> => {
        // Debounce the search
        return new Promise((resolve) => {
          setTimeout(async () => {
            if (query.length < this.options.minLength) {
              resolve([]);
              return;
            }

            try {
              const suggestions = await novelAutocompleteService.getSuggestionsWithSnippets(
                query,
                this.options.limit
              );

              console.log(`TerraphimSuggestion: Found ${suggestions.length} suggestions for "${query}"`);
              resolve(suggestions);
            } catch (error) {
              console.error('TerraphimSuggestion: Error getting suggestions:', error);
              resolve([]);
            }
          }, this.options.debounce);
        });
      },

      render: () => {
        let component: TerraphimSuggestionRenderer;
        let popup: Instance<Props>;

        return {
          onStart: (props) => {
            component = new TerraphimSuggestionRenderer({
              items: props.items as NovelAutocompleteSuggestion[],
              command: props.command,
            });

            if (!props.clientRect) {
              return;
            }

            popup = tippy('body', {
              getReferenceClientRect: props.clientRect as () => DOMRect,
              appendTo: () => document.body,
              content: component.element,
              showOnCreate: true,
              interactive: true,
              trigger: 'manual',
              placement: 'bottom-start',
              theme: 'terraphim-suggestion',
              maxWidth: 'none',
            })[0];
          },

          onUpdate(props) {
            component?.updateItems(props.items as NovelAutocompleteSuggestion[]);

            if (!props.clientRect) {
              return;
            }

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
      },
    };

    return [Suggestion(suggestion)];
  },
});

/**
 * Custom renderer for Terraphim suggestions
 */
class TerraphimSuggestionRenderer {
  public element: HTMLElement;
  private items: NovelAutocompleteSuggestion[] = [];
  private selectedIndex: number = 0;
  private command: (props: { editor: any; range: any; props: NovelAutocompleteSuggestion }) => void;

  constructor({ items, command }: { items: NovelAutocompleteSuggestion[]; command: any }) {
    this.items = items;
    this.command = command;
    this.element = this.createElement();
  }

  private createElement(): HTMLElement {
    const element = document.createElement('div');
    element.className = 'terraphim-suggestion-list';
    this.updateItems(this.items);
    return element;
  }

  updateItems(items: NovelAutocompleteSuggestion[]) {
    this.items = items;
    this.selectedIndex = 0;
    this.render();
  }

  private render() {
    if (this.items.length === 0) {
      this.element.innerHTML = '<div class="terraphim-suggestion-item no-results">No suggestions found</div>';
      return;
    }

    this.element.innerHTML = this.items
      .map((item, index) => {
        const isSelected = index === this.selectedIndex;
        return `
          <div class="terraphim-suggestion-item ${isSelected ? 'selected' : ''}" data-index="${index}">
            <div class="terraphim-suggestion-text">${this.escapeHtml(item.text)}</div>
            ${item.snippet ? `<div class="terraphim-suggestion-snippet">${this.escapeHtml(item.snippet)}</div>` : ''}
            ${item.score ? `<div class="terraphim-suggestion-score">${item.score.toFixed(2)}</div>` : ''}
          </div>
        `;
      })
      .join('');

    // Add click handlers
    this.element.querySelectorAll('.terraphim-suggestion-item').forEach((item, index) => {
      item.addEventListener('click', () => {
        this.selectItem(index);
      });
    });
  }

  private selectItem(index: number) {
    if (index >= 0 && index < this.items.length) {
      this.selectedIndex = index;
      this.render();
      this.command({
        editor: null,
        range: null,
        props: this.items[index],
      });
    }
  }

  onKeyDown(props: { event: KeyboardEvent }): boolean {
    if (this.items.length === 0) {
      return false;
    }

    switch (props.event.key) {
      case 'ArrowDown':
        props.event.preventDefault();
        this.selectedIndex = (this.selectedIndex + 1) % this.items.length;
        this.render();
        return true;

      case 'ArrowUp':
        props.event.preventDefault();
        this.selectedIndex = this.selectedIndex === 0 ? this.items.length - 1 : this.selectedIndex - 1;
        this.render();
        return true;

      case 'Enter':
      case 'Tab':
        props.event.preventDefault();
        this.selectItem(this.selectedIndex);
        return true;

      case 'Escape':
        props.event.preventDefault();
        return true;

      default:
        return false;
    }
  }

  private escapeHtml(text: string): string {
    const div = document.createElement('div');
    div.textContent = text;
    return div.innerHTML;
  }

  destroy() {
    // Cleanup if needed
  }
}

/**
 * CSS styles for the Terraphim suggestion popup
 */
export const terraphimSuggestionStyles = `
  .terraphim-suggestion-list {
    background: white;
    border: 1px solid #e1e5e9;
    border-radius: 8px;
    box-shadow: 0 4px 12px rgba(0, 0, 0, 0.15);
    max-height: 300px;
    overflow-y: auto;
    padding: 4px 0;
    min-width: 200px;
    max-width: 400px;
    font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif;
    font-size: 14px;
    z-index: 1000;
  }

  .terraphim-suggestion-item {
    padding: 8px 12px;
    cursor: pointer;
    border-bottom: 1px solid #f1f3f4;
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  .terraphim-suggestion-item:last-child {
    border-bottom: none;
  }

  .terraphim-suggestion-item:hover,
  .terraphim-suggestion-item.selected {
    background-color: #f8f9fa;
  }

  .terraphim-suggestion-item.selected {
    background-color: #e3f2fd;
  }

  .terraphim-suggestion-text {
    font-weight: 500;
    color: #1a1a1a;
    line-height: 1.4;
  }

  .terraphim-suggestion-snippet {
    font-size: 12px;
    color: #666;
    line-height: 1.3;
    max-width: 100%;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .terraphim-suggestion-score {
    font-size: 11px;
    color: #999;
    font-weight: 500;
  }

  .terraphim-suggestion-item.no-results {
    color: #666;
    font-style: italic;
    cursor: default;
  }

  .terraphim-suggestion-item.no-results:hover {
    background-color: transparent;
  }

  /* Dark theme support */
  @media (prefers-color-scheme: dark) {
    .terraphim-suggestion-list {
      background: #2d3748;
      border-color: #4a5568;
      color: #e2e8f0;
    }

    .terraphim-suggestion-item {
      border-bottom-color: #4a5568;
    }

    .terraphim-suggestion-item:hover,
    .terraphim-suggestion-item.selected {
      background-color: #4a5568;
    }

    .terraphim-suggestion-item.selected {
      background-color: #2b6cb0;
    }

    .terraphim-suggestion-text {
      color: #e2e8f0;
    }

    .terraphim-suggestion-snippet {
      color: #a0aec0;
    }

    .terraphim-suggestion-score {
      color: #718096;
    }

    .terraphim-suggestion-item.no-results {
      color: #a0aec0;
    }
  }
`;