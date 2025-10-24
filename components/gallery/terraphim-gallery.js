/**
 * @fileoverview TerraphimGallery - Main component gallery container
 * Manages layout, state, and orchestrates all gallery subcomponents
 */

import { TerraphimElement } from '../base/terraphim-element.js';
import { createGlobalState } from '../base/terraphim-state.js';

/**
 * Create gallery state instance
 */
export const galleryState = createGlobalState({
  view: 'grid',
  theme: 'light',
  searchQuery: '',
  selectedCategory: 'all',
  selectedTags: [],
  components: [],
  currentComponent: null,
  currentTab: 'demo'
}, {
  persist: true,
  storagePrefix: 'terraphim-gallery'
});

/**
 * TerraphimGallery Component
 * Main container for component gallery with sidebar and content area
 */
export class TerraphimGallery extends TerraphimElement {
  static get properties() {
    return {
      view: { type: String, default: 'grid' },
      theme: { type: String, default: 'light' },
      sidebarCollapsed: { type: Boolean, default: false }
    };
  }

  constructor() {
    super();
    this.attachShadow({ mode: 'open' });
  }

  onConnected() {
    // Bind to gallery state
    this.bindState(galleryState, 'view', 'view', { immediate: true });
    this.bindState(galleryState, 'theme', 'theme', { immediate: true });

    // Load component metadata
    this.loadComponents();
  }

  async loadComponents() {
    try {
      // Load metadata files
      const metaFiles = [
        'terraphim-element.meta.json',
        'terraphim-state.meta.json',
        'state-helpers.meta.json'
      ];

      const components = [];
      for (const file of metaFiles) {
        try {
          const response = await fetch(`./data/${file}`);
          if (response.ok) {
            const meta = await response.json();
            components.push(meta);
          }
        } catch (err) {
          console.warn(`Failed to load ${file}:`, err);
        }
      }

      galleryState.set('components', components);
    } catch (error) {
      console.error('Failed to load components:', error);
    }
  }

  toggleSidebar() {
    this.sidebarCollapsed = !this.sidebarCollapsed;
    this.requestUpdate();
  }

  render() {
    const sidebarClass = this.sidebarCollapsed ? 'collapsed' : '';

    this.setHTML(this.shadowRoot, `
      <style>
        :host {
          display: block;
          width: 100%;
          height: 100vh;
          --sidebar-width: 280px;
          --header-height: 60px;
          --color-bg: ${this.theme === 'dark' ? '#1a1a1a' : '#ffffff'};
          --color-bg-secondary: ${this.theme === 'dark' ? '#2a2a2a' : '#f5f5f5'};
          --color-text: ${this.theme === 'dark' ? '#e0e0e0' : '#333333'};
          --color-text-secondary: ${this.theme === 'dark' ? '#999999' : '#666666'};
          --color-border: ${this.theme === 'dark' ? '#3a3a3a' : '#e0e0e0'};
          --color-primary: #3498db;
          --color-primary-hover: #2980b9;
        }

        * {
          box-sizing: border-box;
        }

        .gallery-container {
          display: flex;
          width: 100%;
          height: 100%;
          background: var(--color-bg);
          color: var(--color-text);
          font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif;
        }

        .sidebar {
          width: var(--sidebar-width);
          height: 100%;
          border-right: 1px solid var(--color-border);
          background: var(--color-bg-secondary);
          transition: transform 0.3s ease;
          overflow-y: auto;
        }

        .sidebar.collapsed {
          transform: translateX(calc(-1 * var(--sidebar-width)));
        }

        .main-content {
          flex: 1;
          display: flex;
          flex-direction: column;
          overflow: hidden;
        }

        .header {
          height: var(--header-height);
          border-bottom: 1px solid var(--color-border);
          display: flex;
          align-items: center;
          padding: 0 20px;
          gap: 15px;
          background: var(--color-bg);
        }

        .toggle-sidebar-btn {
          padding: 8px 12px;
          background: transparent;
          border: 1px solid var(--color-border);
          border-radius: 4px;
          cursor: pointer;
          color: var(--color-text);
          font-size: 18px;
          transition: all 0.2s;
        }

        .toggle-sidebar-btn:hover {
          background: var(--color-bg-secondary);
        }

        .header-title {
          font-size: 20px;
          font-weight: 600;
          flex: 1;
        }

        .content-area {
          flex: 1;
          overflow-y: auto;
          padding: 30px;
        }

        /* Responsive */
        @media (max-width: 768px) {
          .sidebar {
            position: absolute;
            z-index: 100;
            box-shadow: 2px 0 8px rgba(0, 0, 0, 0.1);
          }

          .sidebar:not(.collapsed) {
            transform: translateX(0);
          }
        }
      </style>

      <div class="gallery-container">
        <aside class="sidebar ${sidebarClass}">
          <slot name="sidebar"></slot>
        </aside>

        <div class="main-content">
          <header class="header">
            <button class="toggle-sidebar-btn" id="toggleBtn">☰</button>
            <div class="header-title">Terraphim Component Gallery</div>
            <slot name="header-actions"></slot>
          </header>

          <div class="content-area">
            <slot name="content"></slot>
          </div>
        </div>
      </div>
    `);

    // Add event listener
    const toggleBtn = this.$('#toggleBtn');
    if (toggleBtn) {
      toggleBtn.addEventListener('click', () => this.toggleSidebar());
    }
  }
}

customElements.define('terraphim-gallery', TerraphimGallery);
