/**
 * @fileoverview TerraphimComponentCard - Component preview card
 * Displays component information with view/code actions
 */

import { TerraphimElement } from '../base/terraphim-element.js';
import { galleryState } from './terraphim-gallery.js';

/**
 * TerraphimComponentCard Component
 * Preview card for components with metadata display
 */
export class TerraphimComponentCard extends TerraphimElement {
  static get properties() {
    return {
      component: { type: Object },
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

  handleViewDemo() {
    galleryState.set('currentComponent', this.component);
    galleryState.set('currentTab', 'demo');
    this.emit('component-selected', this.component);
  }

  handleViewCode() {
    galleryState.set('currentComponent', this.component);
    galleryState.set('currentTab', 'code');
    this.emit('component-selected', this.component);
  }

  render() {
    if (!this.component) {
      this.setHTML(this.shadowRoot, '<div>No component data</div>');
      return;
    }

    const isGrid = this.view === 'grid';

    this.setHTML(this.shadowRoot, `
      <style>
        :host {
          display: block;
        }

        .card {
          background: var(--color-bg, #ffffff);
          border: 1px solid var(--color-border, #e0e0e0);
          border-radius: 8px;
          overflow: hidden;
          transition: all 0.2s;
          display: ${isGrid ? 'flex' : 'flex'};
          flex-direction: ${isGrid ? 'column' : 'row'};
          height: ${isGrid ? 'auto' : '140px'};
        }

        .card:hover {
          box-shadow: 0 4px 12px rgba(0, 0, 0, 0.1);
          transform: translateY(-2px);
          border-color: var(--color-primary, #3498db);
        }

        .card-preview {
          background: var(--color-bg-secondary, #f5f5f5);
          padding: ${isGrid ? '30px' : '20px'};
          display: flex;
          align-items: center;
          justify-content: center;
          min-height: ${isGrid ? '120px' : '100%'};
          flex: ${isGrid ? '0 0 auto' : '0 0 140px'};
          border-bottom: ${isGrid ? '1px solid var(--color-border, #e0e0e0)' : 'none'};
          border-right: ${isGrid ? 'none' : '1px solid var(--color-border, #e0e0e0)'};
        }

        .preview-icon {
          font-size: ${isGrid ? '48px' : '36px'};
          opacity: 0.5;
        }

        .card-content {
          padding: ${isGrid ? '20px' : '16px 20px'};
          flex: 1;
          display: flex;
          flex-direction: column;
        }

        .card-header {
          margin-bottom: 8px;
        }

        .card-title {
          font-size: ${isGrid ? '16px' : '15px'};
          font-weight: 600;
          color: var(--color-text, #333);
          margin: 0 0 4px 0;
        }

        .card-category {
          display: inline-block;
          font-size: 11px;
          color: var(--color-text-secondary, #666);
          background: var(--color-bg-secondary, #f5f5f5);
          padding: 2px 8px;
          border-radius: 10px;
          text-transform: capitalize;
        }

        .card-description {
          font-size: 13px;
          color: var(--color-text-secondary, #666);
          line-height: 1.5;
          margin: 8px 0;
          flex: 1;
          overflow: hidden;
          display: -webkit-box;
          -webkit-line-clamp: ${isGrid ? '3' : '2'};
          -webkit-box-orient: vertical;
        }

        .card-tags {
          display: flex;
          flex-wrap: wrap;
          gap: 6px;
          margin: 8px 0;
        }

        .tag {
          font-size: 11px;
          color: var(--color-primary, #3498db);
          background: rgba(52, 152, 219, 0.1);
          padding: 2px 8px;
          border-radius: 3px;
        }

        .card-actions {
          display: flex;
          gap: 8px;
          margin-top: ${isGrid ? '12px' : 'auto'};
        }

        .btn {
          flex: 1;
          padding: 8px 16px;
          border: 1px solid var(--color-border, #e0e0e0);
          border-radius: 4px;
          background: transparent;
          color: var(--color-text, #333);
          font-size: 13px;
          cursor: pointer;
          transition: all 0.2s;
        }

        .btn:hover {
          background: var(--color-primary, #3498db);
          color: white;
          border-color: var(--color-primary, #3498db);
        }

        .btn-primary {
          background: var(--color-primary, #3498db);
          color: white;
          border-color: var(--color-primary, #3498db);
        }

        .btn-primary:hover {
          background: var(--color-primary-hover, #2980b9);
          border-color: var(--color-primary-hover, #2980b9);
        }
      </style>

      <div class="card">
        <div class="card-preview">
          <div class="preview-icon">📦</div>
        </div>

        <div class="card-content">
          <div class="card-header">
            <h3 class="card-title">${this.component.name}</h3>
            <span class="card-category">${this.component.category || 'component'}</span>
          </div>

          <p class="card-description">
            ${this.component.description || 'No description available'}
          </p>

          ${this.component.tags && this.component.tags.length > 0 ? `
            <div class="card-tags">
              ${this.component.tags.slice(0, 3).map(tag => `
                <span class="tag">${tag}</span>
              `).join('')}
            </div>
          ` : ''}

          <div class="card-actions">
            <button class="btn btn-primary" id="viewDemoBtn">View Demo</button>
            <button class="btn" id="viewCodeBtn">View Code</button>
          </div>
        </div>
      </div>
    `);

    // Add event listeners
    const viewDemoBtn = this.$('#viewDemoBtn');
    if (viewDemoBtn) {
      viewDemoBtn.addEventListener('click', () => this.handleViewDemo());
    }

    const viewCodeBtn = this.$('#viewCodeBtn');
    if (viewCodeBtn) {
      viewCodeBtn.addEventListener('click', () => this.handleViewCode());
    }
  }
}

customElements.define('terraphim-component-card', TerraphimComponentCard);
