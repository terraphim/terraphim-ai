/**
 * @fileoverview TerraphimSearch - Search functionality for component gallery
 * Real-time debounced search with keyboard shortcuts
 */

import { TerraphimElement } from '../base/terraphim-element.js';
import { galleryState } from './terraphim-gallery.js';
import { createDebouncedSetter } from '../base/state-helpers.js';

/**
 * TerraphimSearch Component
 * Search input with debouncing and keyboard shortcuts
 */
export class TerraphimSearch extends TerraphimElement {
  static get properties() {
    return {
      searchQuery: { type: String, default: '' },
      placeholder: { type: String, default: 'Search components...' }
    };
  }

  constructor() {
    super();
    this.attachShadow({ mode: 'open' });
    this.debouncedSet = createDebouncedSetter(galleryState, 300);
  }

  onConnected() {
    this.bindState(galleryState, 'searchQuery', 'searchQuery', { immediate: true });

    // Keyboard shortcut: Cmd/Ctrl + K to focus
    this.listenTo(document, 'keydown', (e) => {
      if ((e.metaKey || e.ctrlKey) && e.key === 'k') {
        e.preventDefault();
        this.$('#searchInput')?.focus();
      }
    });
  }

  handleInput(e) {
    const value = e.target.value;
    this.searchQuery = value;
    this.debouncedSet('searchQuery', value);
    this.requestUpdate();
  }

  handleClear() {
    this.searchQuery = '';
    galleryState.set('searchQuery', '');
    this.$('#searchInput').value = '';
    this.$('#searchInput').focus();
  }

  render() {
    this.setHTML(this.shadowRoot, `
      <style>
        :host {
          display: block;
          width: 100%;
        }

        .search-container {
          position: relative;
          width: 100%;
        }

        .search-input {
          width: 100%;
          padding: 10px 40px 10px 40px;
          border: 1px solid var(--color-border, #e0e0e0);
          border-radius: 6px;
          background: var(--color-bg, #ffffff);
          color: var(--color-text, #333);
          font-size: 14px;
          font-family: inherit;
          outline: none;
          transition: all 0.2s;
        }

        .search-input:focus {
          border-color: var(--color-primary, #3498db);
          box-shadow: 0 0 0 3px rgba(52, 152, 219, 0.1);
        }

        .search-input::placeholder {
          color: var(--color-text-secondary, #999);
        }

        .search-icon {
          position: absolute;
          left: 12px;
          top: 50%;
          transform: translateY(-50%);
          color: var(--color-text-secondary, #999);
          pointer-events: none;
          font-size: 16px;
        }

        .clear-btn {
          position: absolute;
          right: 8px;
          top: 50%;
          transform: translateY(-50%);
          background: transparent;
          border: none;
          color: var(--color-text-secondary, #999);
          cursor: pointer;
          padding: 4px 8px;
          border-radius: 3px;
          font-size: 18px;
          line-height: 1;
          opacity: ${this.searchQuery ? '1' : '0'};
          pointer-events: ${this.searchQuery ? 'auto' : 'none'};
          transition: all 0.2s;
        }

        .clear-btn:hover {
          background: var(--color-bg-secondary, #f5f5f5);
          color: var(--color-text, #333);
        }

        .shortcut-hint {
          position: absolute;
          right: 40px;
          top: 50%;
          transform: translateY(-50%);
          font-size: 11px;
          color: var(--color-text-secondary, #999);
          pointer-events: none;
          display: ${this.searchQuery ? 'none' : 'block'};
        }

        .kbd {
          background: var(--color-bg-secondary, #f5f5f5);
          border: 1px solid var(--color-border, #e0e0e0);
          border-radius: 3px;
          padding: 2px 6px;
          font-family: monospace;
          font-size: 10px;
        }
      </style>

      <div class="search-container">
        <span class="search-icon">🔍</span>
        <input
          type="text"
          id="searchInput"
          class="search-input"
          placeholder="${this.placeholder}"
          value="${this.searchQuery}"
          autocomplete="off"
          spellcheck="false"
        />
        <span class="shortcut-hint">
          <span class="kbd">⌘K</span>
        </span>
        <button class="clear-btn" id="clearBtn" title="Clear search">×</button>
      </div>
    `);

    // Add event listeners
    const input = this.$('#searchInput');
    if (input) {
      input.addEventListener('input', (e) => this.handleInput(e));
    }

    const clearBtn = this.$('#clearBtn');
    if (clearBtn) {
      clearBtn.addEventListener('click', () => this.handleClear());
    }
  }
}

customElements.define('terraphim-search', TerraphimSearch);
