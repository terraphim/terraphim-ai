/**
 * @fileoverview TerraphimSidebar - Navigation sidebar for component gallery
 * Displays category tree with counts and active highlighting
 */

import { TerraphimElement } from '../base/terraphim-element.js';
import { galleryState } from './terraphim-gallery.js';

/**
 * TerraphimSidebar Component
 * Category navigation sidebar with collapsible sections
 */
export class TerraphimSidebar extends TerraphimElement {
  static get properties() {
    return {
      selectedCategory: { type: String },
      components: { type: Array }
    };
  }

  constructor() {
    super();
    this.attachShadow({ mode: 'open' });
    this.categories = {
      'all': 'All Components',
      'base': 'Base Components',
      'gallery': 'Gallery Components',
      'examples': 'Examples'
    };
  }

  onConnected() {
    this.bindState(galleryState, 'selectedCategory', 'selectedCategory', { immediate: true });
    this.bindState(galleryState, 'components', 'components', { immediate: true });
  }

  getCategoryCount(category) {
    if (category === 'all') {
      return this.components.length;
    }
    return this.components.filter(c => c.category === category).length;
  }

  selectCategory(category) {
    galleryState.set('selectedCategory', category);
    galleryState.set('currentComponent', null);
  }

  render() {
    this.setHTML(this.shadowRoot, `
      <style>
        :host {
          display: block;
          width: 100%;
          height: 100%;
        }

        .sidebar-container {
          padding: 20px 0;
        }

        .sidebar-header {
          padding: 0 20px 15px;
          border-bottom: 1px solid var(--color-border, #e0e0e0);
          margin-bottom: 15px;
        }

        .sidebar-title {
          font-size: 14px;
          font-weight: 600;
          color: var(--color-text-secondary, #666);
          text-transform: uppercase;
          letter-spacing: 0.5px;
        }

        .category-list {
          list-style: none;
          padding: 0;
          margin: 0;
        }

        .category-item {
          margin: 0;
        }

        .category-link {
          display: flex;
          align-items: center;
          justify-content: space-between;
          padding: 10px 20px;
          color: var(--color-text, #333);
          text-decoration: none;
          cursor: pointer;
          transition: all 0.2s;
          border-left: 3px solid transparent;
        }

        .category-link:hover {
          background: rgba(52, 152, 219, 0.1);
        }

        .category-link.active {
          background: rgba(52, 152, 219, 0.15);
          border-left-color: var(--color-primary, #3498db);
          font-weight: 500;
        }

        .category-name {
          font-size: 14px;
        }

        .category-badge {
          background: var(--color-border, #e0e0e0);
          color: var(--color-text-secondary, #666);
          padding: 2px 8px;
          border-radius: 10px;
          font-size: 12px;
          font-weight: 500;
          min-width: 24px;
          text-align: center;
        }

        .category-link.active .category-badge {
          background: var(--color-primary, #3498db);
          color: white;
        }
      </style>

      <div class="sidebar-container">
        <div class="sidebar-header">
          <div class="sidebar-title">Categories</div>
        </div>

        <ul class="category-list">
          ${Object.entries(this.categories).map(([key, label]) => {
            const count = this.getCategoryCount(key);
            const isActive = this.selectedCategory === key;

            return `
              <li class="category-item">
                <a
                  class="category-link ${isActive ? 'active' : ''}"
                  data-category="${key}"
                >
                  <span class="category-name">${label}</span>
                  <span class="category-badge">${count}</span>
                </a>
              </li>
            `;
          }).join('')}
        </ul>
      </div>
    `);

    // Add event listeners
    this.$$('.category-link').forEach(link => {
      link.addEventListener('click', (e) => {
        e.preventDefault();
        const category = link.dataset.category;
        this.selectCategory(category);
      });
    });
  }
}

customElements.define('terraphim-sidebar', TerraphimSidebar);
