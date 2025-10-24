/**
 * @fileoverview TerraphimTermChips - Visual display of search terms with operators
 * Web Component for displaying selected search terms as interactive chips
 * Supports term removal, operator toggle (AND/OR), and keyboard navigation
 */

import { TerraphimElement } from '../base/terraphim-element.js';

/**
 * @typedef {Object} TermChip
 * @property {string} value - Term text
 * @property {boolean} isFromKG - Whether term is from knowledge graph
 */

/**
 * TerraphimTermChips - Display search terms as removable chips
 *
 * @element terraphim-term-chips
 *
 * @fires {CustomEvent} term-removed - When a term chip is removed
 * @fires {CustomEvent} operator-changed - When operator is toggled
 * @fires {CustomEvent} clear-all - When all terms are cleared
 *
 * @attr {string} operator - Current logical operator ('AND' or 'OR')
 * @prop {TermChip[]} terms - Array of term objects
 *
 * @example
 * <terraphim-term-chips operator="AND"></terraphim-term-chips>
 *
 * <script>
 *   const chips = document.querySelector('terraphim-term-chips');
 *   chips.terms = [
 *     { value: 'rust', isFromKG: true },
 *     { value: 'async', isFromKG: true }
 *   ];
 *
 *   chips.addEventListener('term-removed', (e) => {
 *     console.log('Removed:', e.detail.term);
 *   });
 * </script>
 */
export class TerraphimTermChips extends TerraphimElement {
  static get observedAttributes() {
    return ['operator'];
  }

  static get properties() {
    return {
      operator: { type: String, reflect: true, default: 'AND' },
    };
  }

  constructor() {
    super();

    /**
     * Array of term chips
     * @type {TermChip[]}
     */
    this._terms = [];

    this.attachShadow({ mode: 'open' });
  }

  /**
   * Get terms array
   * @returns {TermChip[]}
   */
  get terms() {
    return this._terms;
  }

  /**
   * Set terms array and trigger re-render
   * @param {TermChip[]} value
   */
  set terms(value) {
    if (!Array.isArray(value)) {
      console.warn('terms must be an array');
      return;
    }

    this._terms = value;
    this.requestUpdate();
  }

  /**
   * Add a term to the chips
   * @param {string} term - Term text
   * @param {boolean} [isFromKG=false] - Whether term is from knowledge graph
   */
  addTerm(term, isFromKG = false) {
    // Check if term already exists
    if (this._terms.some(t => t.value === term)) {
      return;
    }

    this._terms = [...this._terms, { value: term, isFromKG }];
    this.requestUpdate();
  }

  /**
   * Remove a term by value
   * @param {string} term - Term to remove
   */
  removeTerm(term) {
    const oldTerms = this._terms;
    this._terms = this._terms.filter(t => t.value !== term);

    if (oldTerms.length !== this._terms.length) {
      this.emit('term-removed', { term });
      this.requestUpdate();
    }
  }

  /**
   * Clear all terms
   */
  clearAll() {
    if (this._terms.length === 0) return;

    this._terms = [];
    this.emit('clear-all');
    this.requestUpdate();
  }

  /**
   * Toggle operator between AND and OR
   */
  toggleOperator() {
    const newOperator = this.operator === 'AND' ? 'OR' : 'AND';
    this.operator = newOperator;
    this.emit('operator-changed', { operator: newOperator });
  }

  /**
   * Render the component
   */
  render() {
    const hasTerms = this._terms.length > 0;

    this.setHTML(this.shadowRoot, `
      <style>
        :host {
          display: block;
          width: 100%;
        }

        .chips-container {
          display: ${hasTerms ? 'flex' : 'none'};
          flex-wrap: wrap;
          gap: 0.5rem;
          padding: 0.75rem;
          background: rgba(0, 0, 0, 0.02);
          border: 1px solid #e0e0e0;
          border-radius: 4px;
          min-height: 50px;
          align-items: center;
        }

        .term-chip {
          display: inline-flex;
          align-items: center;
          gap: 0.5rem;
          padding: 0.375rem 0.75rem;
          background: #f5f5f5;
          border: 1px solid #ddd;
          border-radius: 16px;
          font-size: 0.875rem;
          cursor: pointer;
          transition: all 0.2s ease;
          position: relative;
        }

        .term-chip:hover {
          background: #e8e8e8;
          transform: scale(1.05);
        }

        .term-chip:focus {
          outline: 2px solid #3273dc;
          outline-offset: 2px;
        }

        .term-chip.from-kg {
          background: #3273dc;
          color: white;
          border-color: #2366d1;
        }

        .term-chip.from-kg:hover {
          background: #2366d1;
        }

        .term-value {
          font-weight: 500;
        }

        .remove-btn {
          background: none;
          border: none;
          color: inherit;
          font-size: 1.25rem;
          font-weight: bold;
          cursor: pointer;
          padding: 0;
          margin: 0;
          width: 1.25rem;
          height: 1.25rem;
          display: flex;
          align-items: center;
          justify-content: center;
          border-radius: 50%;
          transition: background-color 0.2s ease;
        }

        .remove-btn:hover {
          background: rgba(0, 0, 0, 0.1);
        }

        .term-chip.from-kg .remove-btn:hover {
          background: rgba(255, 255, 255, 0.2);
        }

        .operator-chip {
          display: inline-flex;
          align-items: center;
          padding: 0.375rem 0.75rem;
          background: #f5f5f5;
          border: 1px solid #ddd;
          border-radius: 16px;
          font-size: 0.875rem;
          font-weight: 600;
          color: #666;
          cursor: pointer;
          transition: all 0.2s ease;
        }

        .operator-chip:hover {
          background: #e0e0e0;
          color: #333;
        }

        .operator-chip:focus {
          outline: 2px solid #3273dc;
          outline-offset: 2px;
        }

        .controls {
          display: flex;
          gap: 0.5rem;
          margin-left: auto;
        }

        .clear-btn {
          padding: 0.25rem 0.75rem;
          background: #f5f5f5;
          border: 1px solid #ddd;
          border-radius: 4px;
          font-size: 0.75rem;
          cursor: pointer;
          transition: background-color 0.2s ease;
          white-space: nowrap;
        }

        .clear-btn:hover {
          background: #e0e0e0;
        }

        .clear-btn:focus {
          outline: 2px solid #3273dc;
          outline-offset: 2px;
        }

        .empty-state {
          display: ${hasTerms ? 'none' : 'block'};
          color: #999;
          font-size: 0.875rem;
          padding: 0.5rem;
          text-align: center;
        }

        /* Keyboard navigation */
        .term-chip[tabindex="0"] {
          cursor: pointer;
        }
      </style>

      <div class="chips-container" role="list" aria-label="Search terms">
        ${this._renderTerms()}
        ${this._renderControls()}
      </div>

      <div class="empty-state">
        No search terms selected. Use operators AND/OR to combine multiple terms.
      </div>
    `);

    // Attach event listeners after render
    this._attachEventListeners();
  }

  /**
   * Render term chips
   * @private
   * @returns {string}
   */
  _renderTerms() {
    return this._terms.map((term, index) => {
      const isLast = index === this._terms.length - 1;
      const chipClass = term.isFromKG ? 'term-chip from-kg' : 'term-chip';

      return `
        <div
          class="${chipClass}"
          role="listitem"
          tabindex="0"
          data-term="${this._escapeHtml(term.value)}"
          aria-label="Search term: ${this._escapeHtml(term.value)}${term.isFromKG ? ' (from knowledge graph)' : ''}. Press Enter to remove."
        >
          <span class="term-value">${this._escapeHtml(term.value)}</span>
          <button
            class="remove-btn"
            aria-label="Remove term: ${this._escapeHtml(term.value)}"
            title="Remove term"
          >×</button>
        </div>
        ${!isLast ? this._renderOperator() : ''}
      `;
    }).join('');
  }

  /**
   * Render operator chip between terms
   * @private
   * @returns {string}
   */
  _renderOperator() {
    return `
      <button
        class="operator-chip"
        role="button"
        tabindex="0"
        aria-label="Logical operator: ${this.operator}. Press Enter to toggle."
        title="Click to toggle between AND/OR"
      >
        ${this.operator}
      </button>
    `;
  }

  /**
   * Render control buttons
   * @private
   * @returns {string}
   */
  _renderControls() {
    if (this._terms.length === 0) return '';

    return `
      <div class="controls">
        <button
          class="clear-btn"
          role="button"
          tabindex="0"
          aria-label="Clear all search terms"
        >
          Clear all
        </button>
      </div>
    `;
  }

  /**
   * Attach event listeners to rendered elements
   * @private
   */
  _attachEventListeners() {
    // Remove term buttons
    this.$$('.remove-btn').forEach(btn => {
      btn.addEventListener('click', (e) => {
        e.stopPropagation();
        const chip = btn.closest('.term-chip');
        const term = chip.getAttribute('data-term');
        this.removeTerm(term);
      });
    });

    // Term chips (click or Enter key)
    this.$$('.term-chip').forEach(chip => {
      chip.addEventListener('click', () => {
        const term = chip.getAttribute('data-term');
        this.removeTerm(term);
      });

      chip.addEventListener('keydown', (e) => {
        if (e.key === 'Enter' || e.key === ' ') {
          e.preventDefault();
          const term = chip.getAttribute('data-term');
          this.removeTerm(term);
        }
      });
    });

    // Operator chips
    this.$$('.operator-chip').forEach(chip => {
      chip.addEventListener('click', () => {
        this.toggleOperator();
      });

      chip.addEventListener('keydown', (e) => {
        if (e.key === 'Enter' || e.key === ' ') {
          e.preventDefault();
          this.toggleOperator();
        }
      });
    });

    // Clear all button
    const clearBtn = this.$('.clear-btn');
    if (clearBtn) {
      clearBtn.addEventListener('click', () => {
        this.clearAll();
      });

      clearBtn.addEventListener('keydown', (e) => {
        if (e.key === 'Enter' || e.key === ' ') {
          e.preventDefault();
          this.clearAll();
        }
      });
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
if (!customElements.get('terraphim-term-chips')) {
  customElements.define('terraphim-term-chips', TerraphimTermChips);
}
