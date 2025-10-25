/**
 * @fileoverview TerraphimSearchInput - Search input with autocomplete functionality
 * Web Component implementing ARIA combobox pattern with keyboard navigation
 * Supports query parsing with AND/OR operators and integrates with knowledge graph
 */

import { TerraphimElement } from '../base/terraphim-element.js';
import { SearchAPI } from './search-api.js';
import { parseSearchInput, getCurrentTerm, endsWithOperator } from './search-utils.js';

/**
 * @typedef {Object} AutocompleteSuggestion
 * @property {string} term - Suggested term
 * @property {number} [score] - Relevance score
 * @property {string} [description] - Optional description
 * @property {boolean} [isFromKG] - Whether term is from knowledge graph
 */

/**
 * TerraphimSearchInput - Search input component with autocomplete
 *
 * @element terraphim-search-input
 *
 * @fires {CustomEvent} search-submit - When Enter is pressed on complete query
 * @fires {CustomEvent} term-added - When autocomplete term is selected
 * @fires {CustomEvent} input-changed - When input value changes
 * @fires {CustomEvent} dropdown-opened - When autocomplete dropdown opens
 * @fires {CustomEvent} dropdown-closed - When autocomplete dropdown closes
 *
 * @attr {string} value - Current input value
 * @attr {string} placeholder - Placeholder text
 * @attr {string} role - Role name for autocomplete context
 * @attr {boolean} disabled - Disable input
 *
 * @example
 * <terraphim-search-input
 *   placeholder="Search..."
 *   role="engineer"
 * ></terraphim-search-input>
 *
 * <script>
 *   const input = document.querySelector('terraphim-search-input');
 *   input.addEventListener('term-added', (e) => {
 *     console.log('Term added:', e.detail.term);
 *   });
 * </script>
 */
export class TerraphimSearchInput extends TerraphimElement {
  static get observedAttributes() {
    return ['value', 'placeholder', 'role', 'disabled'];
  }

  static get properties() {
    return {
      value: { type: String, reflect: true, default: '' },
      placeholder: { type: String, reflect: true, default: 'Search...' },
      role: { type: String, reflect: true, default: '' },
      disabled: { type: Boolean, reflect: true, default: false },
    };
  }

  constructor() {
    super();

    /**
     * Array of autocomplete suggestions
     * @type {AutocompleteSuggestion[]}
     */
    this._suggestions = [];

    /**
     * Currently selected suggestion index
     * @type {number}
     */
    this._selectedIndex = -1;

    /**
     * Whether dropdown is visible
     * @type {boolean}
     */
    this._dropdownVisible = false;

    /**
     * Whether autocomplete is loading
     * @type {boolean}
     */
    this._loading = false;

    /**
     * Debounce timer for autocomplete requests
     * @type {number|null}
     */
    this._debounceTimer = null;

    /**
     * Current cursor position in input
     * @type {number}
     */
    this._cursorPosition = 0;

    /**
     * SearchAPI instance
     * @type {SearchAPI}
     */
    this._api = new SearchAPI();

    /**
     * Abort controller for canceling requests
     * @type {AbortController|null}
     */
    this._abortController = null;

    this.attachShadow({ mode: 'open' });
  }

  /**
   * Get current suggestions
   * @returns {AutocompleteSuggestion[]}
   */
  get suggestions() {
    return this._suggestions;
  }

  /**
   * Set suggestions and update display
   * @param {AutocompleteSuggestion[]} value
   */
  set suggestions(value) {
    this._suggestions = Array.isArray(value) ? value : [];
    this._dropdownVisible = this._suggestions.length > 0;
    this.requestUpdate();
  }

  /**
   * Lifecycle hook: setup after connection
   */
  onConnected() {
    // Initial render will be called automatically
  }

  /**
   * Lifecycle hook: cleanup before disconnection
   */
  onDisconnected() {
    this._clearDebounce();
    this._cancelAutocomplete();
    this._api.cleanup();
  }

  /**
   * Focus the input element
   */
  focus() {
    const input = this.$('input');
    if (input) input.focus();
  }

  /**
   * Blur the input element
   */
  blur() {
    const input = this.$('input');
    if (input) input.blur();
  }

  /**
   * Clear the input value
   */
  clear() {
    this.value = '';
    this._suggestions = [];
    this._selectedIndex = -1;
    this._dropdownVisible = false;
    this.requestUpdate();
    this.emit('input-changed', { value: '' });
  }

  /**
   * Set input value programmatically
   * @param {string} value - New value
   */
  setValue(value) {
    this.value = value;
    this.requestUpdate();
  }

  /**
   * Render the component
   */
  render() {
    const hasDropdown = this._dropdownVisible && this._suggestions.length > 0;
    const ariaExpanded = hasDropdown ? 'true' : 'false';
    const ariaActiveDescendant = this._selectedIndex >= 0
      ? `suggestion-${this._selectedIndex}`
      : '';

    this.setHTML(this.shadowRoot, `
      <style>
        :host {
          display: block;
          width: 100%;
          position: relative;
        }

        .input-container {
          position: relative;
          width: 100%;
        }

        .search-input {
          width: 100%;
          padding: 0.75rem 3rem 0.75rem 1rem;
          font-size: 1rem;
          line-height: 1.5;
          border: 1px solid #dbdbdb;
          border-radius: 4px;
          background: white;
          transition: border-color 0.2s ease, box-shadow 0.2s ease;
        }

        .search-input:focus {
          outline: none;
          border-color: #3273dc;
          box-shadow: 0 0 0 0.125em rgba(50, 115, 220, 0.25);
        }

        .search-input:disabled {
          background-color: #f5f5f5;
          cursor: not-allowed;
          opacity: 0.6;
        }

        .search-icon {
          position: absolute;
          right: 1rem;
          top: 50%;
          transform: translateY(-50%);
          color: #7a7a7a;
          pointer-events: none;
          width: 1.25rem;
          height: 1.25rem;
        }

        .loading-spinner {
          position: absolute;
          right: 1rem;
          top: 50%;
          transform: translateY(-50%);
          width: 1.25rem;
          height: 1.25rem;
          border: 2px solid #f3f3f3;
          border-top: 2px solid #3273dc;
          border-radius: 50%;
          animation: spin 0.8s linear infinite;
        }

        @keyframes spin {
          0% { transform: translateY(-50%) rotate(0deg); }
          100% { transform: translateY(-50%) rotate(360deg); }
        }

        /* Dropdown styles */
        .dropdown {
          position: absolute;
          top: 100%;
          left: 0;
          right: 0;
          z-index: 1000;
          max-height: 300px;
          overflow-y: auto;
          background: white;
          border: 1px solid #dbdbdb;
          border-top: none;
          border-radius: 0 0 4px 4px;
          box-shadow: 0 2px 8px rgba(10, 10, 10, 0.1);
          margin-top: -1px;
          display: ${hasDropdown ? 'block' : 'none'};
        }

        .suggestions-list {
          list-style: none;
          margin: 0;
          padding: 0;
        }

        .suggestion-item {
          padding: 0.75rem 1rem;
          cursor: pointer;
          transition: background-color 0.15s ease;
          border-bottom: 1px solid #f5f5f5;
          display: flex;
          align-items: center;
          gap: 0.5rem;
        }

        .suggestion-item:last-child {
          border-bottom: none;
        }

        .suggestion-item:hover,
        .suggestion-item.selected {
          background-color: #f5f5f5;
        }

        .suggestion-item.selected {
          background-color: #e8f4fd;
        }

        .kg-indicator {
          width: 8px;
          height: 8px;
          border-radius: 50%;
          background-color: #3273dc;
          flex-shrink: 0;
          margin-right: 0.25rem;
        }

        .suggestion-text {
          flex: 1;
          font-size: 0.95rem;
        }

        .suggestion-highlight {
          font-weight: 600;
          color: #3273dc;
        }

        .empty-state {
          padding: 1rem;
          text-align: center;
          color: #999;
          font-size: 0.875rem;
        }

        /* Screen reader only */
        .sr-only {
          position: absolute;
          width: 1px;
          height: 1px;
          padding: 0;
          margin: -1px;
          overflow: hidden;
          clip: rect(0, 0, 0, 0);
          white-space: nowrap;
          border-width: 0;
        }
      </style>

      <div class="input-container">
        <input
          type="text"
          class="search-input"
          value="${this._escapeHtml(this.value)}"
          placeholder="${this._escapeHtml(this.placeholder)}"
          ?disabled="${this.disabled}"
          role="combobox"
          aria-label="Search input with autocomplete"
          aria-autocomplete="list"
          aria-expanded="${ariaExpanded}"
          aria-owns="suggestions-listbox"
          aria-activedescendant="${ariaActiveDescendant}"
          aria-controls="suggestions-listbox"
        />

        ${this._loading ? `
          <div class="loading-spinner" aria-hidden="true"></div>
        ` : `
          <svg class="search-icon" aria-hidden="true" viewBox="0 0 24 24" fill="none" stroke="currentColor">
            <circle cx="11" cy="11" r="8" stroke-width="2"/>
            <path d="M21 21l-4.35-4.35" stroke-width="2" stroke-linecap="round"/>
          </svg>
        `}

        <div class="dropdown" role="region" aria-live="polite">
          <ul
            id="suggestions-listbox"
            class="suggestions-list"
            role="listbox"
            aria-label="Autocomplete suggestions"
          >
            ${this._renderSuggestions()}
          </ul>
        </div>

        <!-- Screen reader announcements -->
        <div class="sr-only" role="status" aria-live="polite" aria-atomic="true">
          ${this._getScreenReaderAnnouncement()}
        </div>
      </div>
    `);

    this._attachEventListeners();
  }

  /**
   * Render suggestions list
   * @private
   * @returns {string}
   */
  _renderSuggestions() {
    if (this._suggestions.length === 0) {
      return '<li class="empty-state">No suggestions found</li>';
    }

    const currentTerm = getCurrentTerm(this.value, this._cursorPosition).toLowerCase();

    return this._suggestions.map((suggestion, index) => {
      const isSelected = index === this._selectedIndex;
      const term = suggestion.term || suggestion;
      const isFromKG = suggestion.isFromKG || false;
      const highlightedText = this._highlightMatch(term, currentTerm);

      return `
        <li
          id="suggestion-${index}"
          class="suggestion-item ${isSelected ? 'selected' : ''}"
          role="option"
          aria-selected="${isSelected}"
          data-index="${index}"
          data-term="${this._escapeHtml(term)}"
          data-from-kg="${isFromKG}"
        >
          ${isFromKG ? '<span class="kg-indicator" aria-label="From knowledge graph"></span>' : ''}
          <span class="suggestion-text">${highlightedText}</span>
        </li>
      `;
    }).join('');
  }

  /**
   * Highlight matching text in suggestion
   * @private
   * @param {string} text - Full suggestion text
   * @param {string} query - Search query
   * @returns {string} HTML with highlighted match
   */
  _highlightMatch(text, query) {
    if (!query || query.length === 0) {
      return this._escapeHtml(text);
    }

    const lowerText = text.toLowerCase();
    const lowerQuery = query.toLowerCase();
    const startIndex = lowerText.indexOf(lowerQuery);

    if (startIndex === -1) {
      return this._escapeHtml(text);
    }

    const before = text.slice(0, startIndex);
    const match = text.slice(startIndex, startIndex + query.length);
    const after = text.slice(startIndex + query.length);

    return `${this._escapeHtml(before)}<span class="suggestion-highlight">${this._escapeHtml(match)}</span>${this._escapeHtml(after)}`;
  }

  /**
   * Get screen reader announcement text
   * @private
   * @returns {string}
   */
  _getScreenReaderAnnouncement() {
    if (!this._dropdownVisible) return '';

    const count = this._suggestions.length;
    if (count === 0) {
      return 'No suggestions available';
    }

    return `${count} suggestion${count === 1 ? '' : 's'} available. Use up and down arrow keys to navigate, Enter or Tab to select.`;
  }

  /**
   * Attach event listeners to rendered elements
   * @private
   */
  _attachEventListeners() {
    const input = this.$('input');
    if (!input) return;

    // Input event - handle text changes
    input.addEventListener('input', (e) => {
      this.value = e.target.value;
      this._cursorPosition = e.target.selectionStart || 0;
      this._handleInput(e.target.value);
      this.emit('input-changed', { value: e.target.value });
    });

    // Keydown event - handle keyboard navigation
    input.addEventListener('keydown', (e) => {
      this._handleKeydown(e);
    });

    // Focus event
    input.addEventListener('focus', () => {
      // Re-trigger autocomplete if there's text
      if (this.value.trim().length >= 2) {
        this._triggerAutocomplete(this.value);
      }
    });

    // Blur event - close dropdown after a delay to allow click events
    input.addEventListener('blur', () => {
      setTimeout(() => {
        this._closeDropdown();
      }, 200);
    });

    // Suggestion item clicks
    this.$$('.suggestion-item').forEach(item => {
      item.addEventListener('click', () => {
        const index = parseInt(item.getAttribute('data-index'), 10);
        this._selectSuggestion(index);
      });

      item.addEventListener('mouseenter', () => {
        const index = parseInt(item.getAttribute('data-index'), 10);
        this._selectedIndex = index;
        this.requestUpdate();
      });
    });
  }

  /**
   * Handle input text changes
   * @private
   * @param {string} value - Input value
   */
  _handleInput(value) {
    // Clear previous debounce timer
    this._clearDebounce();

    // If input is empty, clear suggestions
    if (!value || value.trim().length === 0) {
      this._suggestions = [];
      this._selectedIndex = -1;
      this._closeDropdown();
      return;
    }

    // Get the current term being typed
    const currentTerm = getCurrentTerm(value, this._cursorPosition);

    // Only trigger autocomplete for terms >= 2 characters
    if (currentTerm.length < 2) {
      this._suggestions = [];
      this._selectedIndex = -1;
      this._closeDropdown();
      return;
    }

    // Debounce autocomplete request (300ms)
    this._debounceTimer = setTimeout(() => {
      this._triggerAutocomplete(currentTerm);
    }, 300);
  }

  /**
   * Handle keyboard events
   * @private
   * @param {KeyboardEvent} event
   */
  _handleKeydown(event) {
    const hasDropdown = this._dropdownVisible && this._suggestions.length > 0;

    // Handle dropdown navigation
    if (hasDropdown) {
      switch (event.key) {
        case 'ArrowDown':
          event.preventDefault();
          this._moveSelection(1);
          break;

        case 'ArrowUp':
          event.preventDefault();
          this._moveSelection(-1);
          break;

        case 'Enter':
          // If a suggestion is selected, apply it
          if (this._selectedIndex >= 0) {
            event.preventDefault();
            this._selectSuggestion(this._selectedIndex);
          } else {
            // Otherwise, submit the search
            event.preventDefault();
            this._submitSearch();
          }
          break;

        case 'Tab':
          // Apply suggestion if one is selected
          if (this._selectedIndex >= 0) {
            event.preventDefault();
            this._selectSuggestion(this._selectedIndex);
          }
          break;

        case 'Escape':
          event.preventDefault();
          this._closeDropdown();
          break;

        default:
          break;
      }
    } else if (event.key === 'Enter') {
      // Submit search when Enter is pressed without dropdown
      event.preventDefault();
      this._submitSearch();
    }
  }

  /**
   * Move selection up or down in suggestions list
   * @private
   * @param {number} direction - 1 for down, -1 for up
   */
  _moveSelection(direction) {
    if (this._suggestions.length === 0) return;

    this._selectedIndex += direction;

    // Wrap around
    if (this._selectedIndex < 0) {
      this._selectedIndex = this._suggestions.length - 1;
    } else if (this._selectedIndex >= this._suggestions.length) {
      this._selectedIndex = 0;
    }

    this.requestUpdate();

    // Scroll selected item into view
    this._scrollToSelected();
  }

  /**
   * Scroll selected suggestion into view
   * @private
   */
  _scrollToSelected() {
    requestAnimationFrame(() => {
      const selectedItem = this.$(`#suggestion-${this._selectedIndex}`);
      if (selectedItem) {
        selectedItem.scrollIntoView({
          block: 'nearest',
          behavior: 'smooth'
        });
      }
    });
  }

  /**
   * Select a suggestion by index
   * @private
   * @param {number} index - Suggestion index
   */
  _selectSuggestion(index) {
    if (index < 0 || index >= this._suggestions.length) return;

    const suggestion = this._suggestions[index];
    const term = suggestion.term || suggestion;
    const isFromKG = suggestion.isFromKG || false;

    // Replace current term with selected suggestion
    this._replaceCurrentTerm(term);

    // Emit term-added event
    this.emit('term-added', {
      term,
      isFromKG,
      position: this._cursorPosition
    });

    // Close dropdown
    this._closeDropdown();

    // Focus back on input
    this.focus();
  }

  /**
   * Replace the current term with the selected suggestion
   * @private
   * @param {string} selectedTerm - Term to insert
   */
  _replaceCurrentTerm(selectedTerm) {
    const input = this.$('input');
    if (!input) return;

    const beforeCursor = this.value.slice(0, this._cursorPosition);
    const afterCursor = this.value.slice(this._cursorPosition);

    // Find the start of the current term
    const words = beforeCursor.split(/\s+/);
    const currentTerm = words[words.length - 1];
    const startIndex = beforeCursor.lastIndexOf(currentTerm);

    // Build new value
    const newValue = this.value.slice(0, startIndex) + selectedTerm + afterCursor;

    // Update value
    this.value = newValue;

    // Set cursor position after the inserted term
    const newCursorPosition = startIndex + selectedTerm.length;
    this._cursorPosition = newCursorPosition;

    // Update input and cursor
    requestAnimationFrame(() => {
      input.value = newValue;
      input.setSelectionRange(newCursorPosition, newCursorPosition);
    });

    this.emit('input-changed', { value: newValue });
  }

  /**
   * Submit search
   * @private
   */
  _submitSearch() {
    const parsed = parseSearchInput(this.value);

    this.emit('search-submit', {
      query: this.value,
      parsed,
      role: this.role
    });
  }

  /**
   * Trigger autocomplete API call
   * @private
   * @param {string} query - Search query
   */
  async _triggerAutocomplete(query) {
    if (!query || query.trim().length < 2) return;

    // Cancel any pending request
    this._cancelAutocomplete();

    this._loading = true;
    this.requestUpdate();

    try {
      const suggestions = await this._api.getAutocompleteSuggestions(query, {
        role: this.role,
        limit: 8
      });

      // Map suggestions to include KG indicator
      this._suggestions = suggestions.map(suggestion => {
        if (typeof suggestion === 'string') {
          return { term: suggestion, isFromKG: false };
        }
        return suggestion;
      });

      this._selectedIndex = -1;
      this._loading = false;

      if (this._suggestions.length > 0) {
        this._openDropdown();
      } else {
        this._closeDropdown();
      }

      this.requestUpdate();
    } catch (error) {
      console.error('Autocomplete error:', error);
      this._loading = false;
      this._suggestions = [];
      this._closeDropdown();
      this.requestUpdate();
    }
  }

  /**
   * Cancel pending autocomplete request
   * @private
   */
  _cancelAutocomplete() {
    if (this._abortController) {
      this._abortController.abort();
      this._abortController = null;
    }
  }

  /**
   * Open autocomplete dropdown
   * @private
   */
  _openDropdown() {
    if (this._dropdownVisible) return;

    this._dropdownVisible = true;
    this.emit('dropdown-opened');
    this.requestUpdate();
  }

  /**
   * Close autocomplete dropdown
   * @private
   */
  _closeDropdown() {
    if (!this._dropdownVisible) return;

    this._dropdownVisible = false;
    this._selectedIndex = -1;
    this.emit('dropdown-closed');
    this.requestUpdate();
  }

  /**
   * Clear debounce timer
   * @private
   */
  _clearDebounce() {
    if (this._debounceTimer) {
      clearTimeout(this._debounceTimer);
      this._debounceTimer = null;
    }
  }

  /**
   * Escape HTML special characters
   * @private
   * @param {string} str - String to escape
   * @returns {string} Escaped string
   */
  _escapeHtml(str) {
    const div = document.createElement('div');
    div.textContent = str;
    return div.innerHTML;
  }
}

// Register the custom element
if (!customElements.get('terraphim-search-input')) {
  customElements.define('terraphim-search-input', TerraphimSearchInput);
}
