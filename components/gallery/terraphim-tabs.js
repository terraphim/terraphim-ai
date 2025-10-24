/**
 * @fileoverview TerraphimTabs - Tab navigation component
 * Provides tab switching with keyboard navigation
 */

import { TerraphimElement } from '../base/terraphim-element.js';
import { galleryState } from './terraphim-gallery.js';

/**
 * TerraphimTabs Component
 * Tab navigation with active state management
 */
export class TerraphimTabs extends TerraphimElement {
  static get properties() {
    return {
      tabs: { type: Array, default: () => [] },
      activeTab: { type: String }
    };
  }

  constructor() {
    super();
    this.attachShadow({ mode: 'open' });
  }

  onConnected() {
    this.bindState(galleryState, 'currentTab', 'activeTab', { immediate: true });

    // Keyboard navigation
    this.listenTo(document, 'keydown', (e) => {
      if (!this.tabs || this.tabs.length === 0) return;

      const currentIndex = this.tabs.findIndex(t => t.id === this.activeTab);
      if (currentIndex === -1) return;

      let newIndex = currentIndex;

      // Arrow Left: previous tab
      if (e.key === 'ArrowLeft' && e.altKey) {
        e.preventDefault();
        newIndex = (currentIndex - 1 + this.tabs.length) % this.tabs.length;
      }
      // Arrow Right: next tab
      else if (e.key === 'ArrowRight' && e.altKey) {
        e.preventDefault();
        newIndex = (currentIndex + 1) % this.tabs.length;
      }

      if (newIndex !== currentIndex) {
        this.selectTab(this.tabs[newIndex].id);
      }
    });
  }

  selectTab(tabId) {
    galleryState.set('currentTab', tabId);
    this.emit('tab-changed', { tabId });
  }

  render() {
    if (!this.tabs || this.tabs.length === 0) {
      this.setHTML(this.shadowRoot, '<div>No tabs available</div>');
      return;
    }

    this.setHTML(this.shadowRoot, `
      <style>
        :host {
          display: block;
          width: 100%;
        }

        .tabs-container {
          border-bottom: 2px solid var(--color-border, #e0e0e0);
        }

        .tabs-list {
          display: flex;
          gap: 0;
          list-style: none;
          margin: 0;
          padding: 0;
        }

        .tab-item {
          margin: 0;
        }

        .tab-button {
          position: relative;
          padding: 12px 24px;
          background: transparent;
          border: none;
          border-bottom: 2px solid transparent;
          color: var(--color-text-secondary, #666);
          font-size: 14px;
          font-weight: 500;
          cursor: pointer;
          transition: all 0.2s;
          margin-bottom: -2px;
        }

        .tab-button:hover {
          color: var(--color-text, #333);
          background: rgba(52, 152, 219, 0.05);
        }

        .tab-button.active {
          color: var(--color-primary, #3498db);
          border-bottom-color: var(--color-primary, #3498db);
        }

        .tab-button:focus {
          outline: 2px solid var(--color-primary, #3498db);
          outline-offset: -2px;
        }

        .tab-icon {
          margin-right: 6px;
        }

        .keyboard-hint {
          font-size: 11px;
          color: var(--color-text-secondary, #999);
          margin-top: 8px;
          text-align: center;
        }
      </style>

      <div class="tabs-container">
        <ul class="tabs-list" role="tablist">
          ${this.tabs.map(tab => {
            const isActive = this.activeTab === tab.id;
            return `
              <li class="tab-item" role="presentation">
                <button
                  class="tab-button ${isActive ? 'active' : ''}"
                  role="tab"
                  aria-selected="${isActive}"
                  data-tab-id="${tab.id}"
                >
                  ${tab.icon ? `<span class="tab-icon">${tab.icon}</span>` : ''}
                  ${tab.label}
                </button>
              </li>
            `;
          }).join('')}
        </ul>
      </div>

      <div class="keyboard-hint">
        Use Alt + ← → to navigate tabs
      </div>
    `);

    // Add event listeners
    this.$$('.tab-button').forEach(button => {
      button.addEventListener('click', () => {
        const tabId = button.dataset.tabId;
        this.selectTab(tabId);
      });
    });
  }
}

customElements.define('terraphim-tabs', TerraphimTabs);
