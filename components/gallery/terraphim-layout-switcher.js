/**
 * @fileoverview TerraphimLayoutSwitcher - Grid/list view toggle
 * Switches between grid and list card layouts
 */

import { TerraphimElement } from '../base/terraphim-element.js';
import { galleryState } from './terraphim-gallery.js';

/**
 * TerraphimLayoutSwitcher Component
 * Toggle between grid and list view layouts
 */
export class TerraphimLayoutSwitcher extends TerraphimElement {
  static get properties() {
    return {
      view: { type: String, default: 'grid' }
    };
  }

  constructor() {
    super();
    this.attachShadow({ mode: 'open' });
  }

  onConnected() {
    this.bindState(galleryState, 'view', 'view', { immediate: true });
  }

  setView(view) {
    galleryState.set('view', view);
  }

  render() {
    this.setHTML(this.shadowRoot, `
      <style>
        :host {
          display: inline-block;
        }

        .layout-switcher {
          display: inline-flex;
          gap: 0;
          background: var(--color-bg-secondary, #f5f5f5);
          border: 1px solid var(--color-border, #e0e0e0);
          border-radius: 4px;
          padding: 2px;
        }

        .layout-button {
          padding: 6px 12px;
          background: transparent;
          border: none;
          color: var(--color-text-secondary, #666);
          cursor: pointer;
          transition: all 0.2s;
          border-radius: 3px;
          font-size: 16px;
          display: flex;
          align-items: center;
          justify-content: center;
        }

        .layout-button:hover {
          color: var(--color-text, #333);
        }

        .layout-button.active {
          background: var(--color-primary, #3498db);
          color: white;
        }

        .layout-button:focus {
          outline: 2px solid var(--color-primary, #3498db);
          outline-offset: 2px;
        }
      </style>

      <div class="layout-switcher" role="group" aria-label="Layout view">
        <button
          class="layout-button ${this.view === 'grid' ? 'active' : ''}"
          id="gridBtn"
          aria-label="Grid view"
          title="Grid view"
          aria-pressed="${this.view === 'grid'}"
        >
          ▦
        </button>
        <button
          class="layout-button ${this.view === 'list' ? 'active' : ''}"
          id="listBtn"
          aria-label="List view"
          title="List view"
          aria-pressed="${this.view === 'list'}"
        >
          ☰
        </button>
      </div>
    `);

    // Add event listeners
    const gridBtn = this.$('#gridBtn');
    if (gridBtn) {
      gridBtn.addEventListener('click', () => this.setView('grid'));
    }

    const listBtn = this.$('#listBtn');
    if (listBtn) {
      listBtn.addEventListener('click', () => this.setView('list'));
    }
  }
}

customElements.define('terraphim-layout-switcher', TerraphimLayoutSwitcher);
