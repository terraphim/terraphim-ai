import { Extension } from '@tiptap/core';
import { PluginKey } from 'prosemirror-state';
import { Suggestion } from '@tiptap/suggestion';
import type { SuggestionOptions } from '@tiptap/suggestion';
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
      trigger: '++',
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
 * Suggestion dropdown renderer component
 */
class TerraphimSuggestionRenderer {
  public element: HTMLElement;
  private items: NovelAutocompleteSuggestion[] = [];
  private selectedIndex = 0;
  private command: (props: { id: string; [key: string]: any }) => void;

  constructor(options: {
    items: NovelAutocompleteSuggestion[];
    command: (props: { id: string; [key: string]: any }) => void;
  }) {
    this.items = options.items;
    this.command = options.command;

    this.element = document.createElement('div');
    this.element.className = 'terraphim-suggestion-dropdown';
    this.render();
  }

  updateItems(items: NovelAutocompleteSuggestion[]) {
    this.items = items;
    this.selectedIndex = 0;
    this.render();
  }

  onKeyDown({ event }: { event: KeyboardEvent }): boolean {
    if (event.key === 'ArrowUp') {
      this.selectPrevious();
      return true;
    }

    if (event.key === 'ArrowDown') {
      this.selectNext();
      return true;
    }

    if (event.key === 'Enter' || event.key === 'Tab') {
      this.selectItem(this.selectedIndex);
      return true;
    }

    return false;
  }

  selectPrevious() {
    this.selectedIndex = Math.max(0, this.selectedIndex - 1);
    this.render();
  }

  selectNext() {
    this.selectedIndex = Math.min(this.items.length - 1, this.selectedIndex + 1);
    this.render();
  }

  selectItem(index: number) {
    const item = this.items[index];
    if (item) {
      // Pass the item as props object with id property as expected by TipTap
      this.command({ id: item.text, ...item });
    }
  }

  private render() {
    this.element.innerHTML = '';

    if (this.items.length === 0) {
      this.element.innerHTML = `
        <div class="terraphim-suggestion-item terraphim-suggestion-empty">
          <div class="terraphim-suggestion-text">No suggestions found</div>
          <div class="terraphim-suggestion-hint">Try a different search term</div>
        </div>
      `;
      return;
    }

    const header = document.createElement('div');
    header.className = 'terraphim-suggestion-header';
    header.innerHTML = `
      <div class="terraphim-suggestion-count">${this.items.length} suggestions</div>
      <div class="terraphim-suggestion-hint">↑↓ Navigate • Tab/Enter Select • Esc Cancel</div>
    `;
    this.element.appendChild(header);

    this.items.forEach((item, index) => {
      const itemEl = document.createElement('div');
      itemEl.className = `terraphim-suggestion-item ${
        index === this.selectedIndex ? 'terraphim-suggestion-selected' : ''
      }`;

      itemEl.innerHTML = `
        <div class="terraphim-suggestion-main">
          <div class="terraphim-suggestion-text">${this.escapeHtml(item.text)}</div>
          ${item.snippet ? `<div class="terraphim-suggestion-snippet">${this.escapeHtml(item.snippet)}</div>` : ''}
        </div>
        ${item.score ? `<div class="terraphim-suggestion-score">${Math.round(item.score * 100)}%</div>` : ''}
      `;

      itemEl.addEventListener('click', () => this.selectItem(index));
      itemEl.addEventListener('mouseenter', () => {
        this.selectedIndex = index;
        this.render();
      });

      this.element.appendChild(itemEl);
    });
  }

  private escapeHtml(text: string): string {
    const div = document.createElement('div');
    div.textContent = text;
    return div.innerHTML;
  }

  destroy() {
    this.element.remove();
  }
}

// CSS styles for the suggestion dropdown
export const terraphimSuggestionStyles = `
.terraphim-suggestion-dropdown {
  background: white;
  border: 1px solid #e2e8f0;
  border-radius: 8px;
  box-shadow: 0 10px 38px -10px rgba(22, 23, 24, 0.35), 0 10px 20px -15px rgba(22, 23, 24, 0.2);
  max-height: 300px;
  min-width: 300px;
  overflow-y: auto;
  z-index: 1000;
}

.terraphim-suggestion-header {
  padding: 8px 12px;
  border-bottom: 1px solid #f1f5f9;
  background: #f8fafc;
  font-size: 12px;
  display: flex;
  justify-content: space-between;
  align-items: center;
}

.terraphim-suggestion-count {
  font-weight: 600;
  color: #475569;
}

.terraphim-suggestion-hint {
  color: #64748b;
}

.terraphim-suggestion-item {
  padding: 8px 12px;
  cursor: pointer;
  display: flex;
  justify-content: space-between;
  align-items: flex-start;
  border-bottom: 1px solid #f1f5f9;
}

.terraphim-suggestion-item:hover {
  background: #f8fafc;
}

.terraphim-suggestion-selected {
  background: #eff6ff !important;
  border-left: 3px solid #3b82f6;
}

.terraphim-suggestion-empty {
  color: #64748b;
  font-style: italic;
  text-align: center;
  cursor: default;
}

.terraphim-suggestion-main {
  flex: 1;
}

.terraphim-suggestion-text {
  font-weight: 500;
  color: #1e293b;
  margin-bottom: 2px;
}

.terraphim-suggestion-snippet {
  font-size: 12px;
  color: #64748b;
  line-height: 1.3;
}

.terraphim-suggestion-score {
  font-size: 11px;
  color: #10b981;
  font-weight: 600;
  background: #ecfdf5;
  padding: 2px 6px;
  border-radius: 4px;
  margin-left: 8px;
}

/* Dark theme support */
@media (prefers-color-scheme: dark) {
  .terraphim-suggestion-dropdown {
    background: #1e293b;
    border-color: #334155;
  }

  .terraphim-suggestion-header {
    background: #0f172a;
    border-color: #334155;
  }

  .terraphim-suggestion-item:hover {
    background: #334155;
  }

  .terraphim-suggestion-selected {
    background: #1e40af !important;
  }

  .terraphim-suggestion-text {
    color: #f1f5f9;
  }

  .terraphim-suggestion-snippet {
    color: #94a3b8;
  }
}

/* Tippy.js theme */
.tippy-box[data-theme~='terraphim-suggestion'] {
  background: transparent;
  box-shadow: none;
}

.tippy-box[data-theme~='terraphim-suggestion'] > .tippy-backdrop {
  background: transparent;
}

.tippy-box[data-theme~='terraphim-suggestion'] > .tippy-arrow {
  display: none;
}
`;
