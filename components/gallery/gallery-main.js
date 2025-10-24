/**
 * Gallery Main Component
 * Main content area for displaying component documentation
 */

class GalleryMain extends HTMLElement {
  constructor() {
    super();
    this.attachShadow({ mode: 'open' });
    this.currentPath = '/';
  }

  connectedCallback() {
    this.render();
  }

  /**
   * Update content based on route
   * @param {string} path - Route path
   */
  updateContent(path) {
    this.currentPath = path;
    this.loadContent(path);
  }

  /**
   * Load content for the given path
   * @param {string} path - Route path
   */
  async loadContent(path) {
    const content = this.shadowRoot.querySelector('.content');

    // Show loading state
    content.innerHTML = `
      <div class="loading-container">
        <div class="loading-spinner"></div>
        <p>Loading...</p>
      </div>
    `;

    try {
      if (path === '/' || path === '') {
        this.renderWelcome();
      } else {
        await this.renderComponentPage(path);
      }
    } catch (error) {
      console.error('Failed to load content:', error);
      this.renderError(error.message);
    }
  }

  /**
   * Render welcome page
   */
  renderWelcome() {
    const content = this.shadowRoot.querySelector('.content');
    content.innerHTML = `
      <div class="welcome">
        <h1>Welcome to Terraphim Gallery</h1>
        <p class="subtitle">A showcase of pure vanilla Web Components</p>

        <div class="welcome-grid">
          <div class="welcome-card">
            <div class="card-icon">📦</div>
            <h3>Base Components</h3>
            <p>Foundation components and utilities for building Terraphim applications</p>
          </div>

          <div class="welcome-card">
            <div class="card-icon">🎨</div>
            <h3>Gallery Components</h3>
            <p>Components that make up this gallery and documentation system</p>
          </div>

          <div class="welcome-card">
            <div class="card-icon">🚀</div>
            <h3>Getting Started</h3>
            <p>Learn how to use Terraphim components in your projects</p>
          </div>
        </div>

        <div class="features">
          <h2>Features</h2>
          <ul>
            <li>Pure vanilla JavaScript - no build tools required</li>
            <li>Shadow DOM encapsulation for style isolation</li>
            <li>Custom Elements API for component definition</li>
            <li>Accessibility-first design with ARIA support</li>
            <li>Responsive design for mobile, tablet, and desktop</li>
            <li>Dark mode support with theme switching</li>
          </ul>
        </div>
      </div>
    `;
  }

  /**
   * Render component documentation page
   * @param {string} path - Component path
   */
  async renderComponentPage(path) {
    const content = this.shadowRoot.querySelector('.content');

    // For Phase 1, show placeholder
    content.innerHTML = `
      <div class="component-page">
        <h1>Component Documentation</h1>
        <p class="subtitle">Path: <code>${path}</code></p>

        <div class="info-box">
          <strong>Phase 1 Implementation</strong>
          <p>Full component documentation will be available in Phase 2.</p>
          <p>The navigation and routing infrastructure is now complete.</p>
        </div>

        <h2>Coming Soon</h2>
        <ul>
          <li>Live component examples</li>
          <li>API documentation</li>
          <li>Code snippets with syntax highlighting</li>
          <li>Interactive property editors</li>
          <li>Download and installation instructions</li>
        </ul>
      </div>
    `;
  }

  /**
   * Render error state
   * @param {string} message - Error message
   */
  renderError(message) {
    const content = this.shadowRoot.querySelector('.content');
    content.innerHTML = `
      <div class="error-container">
        <div class="error-icon">⚠️</div>
        <h2>Error Loading Content</h2>
        <p>${message}</p>
        <button onclick="window.location.hash = '/'">Go Home</button>
      </div>
    `;
  }

  /**
   * Render component template
   */
  render() {
    this.shadowRoot.innerHTML = `
      <style>
        :host {
          display: block;
          background: var(--color-background, #ffffff);
          overflow-y: auto;
          height: 100%;
        }

        .main-container {
          max-width: var(--content-max-width, 1200px);
          margin: 0 auto;
          padding: 2rem;
        }

        .content {
          min-height: 400px;
        }

        /* Welcome page styles */
        .welcome {
          max-width: 800px;
          margin: 0 auto;
        }

        .welcome h1 {
          font-size: 2.5rem;
          margin-bottom: 0.5rem;
          color: var(--color-text-primary, #2c3e50);
        }

        .subtitle {
          font-size: 1.125rem;
          color: var(--color-text-secondary, #7f8c8d);
          margin-bottom: 2rem;
        }

        .welcome-grid {
          display: grid;
          grid-template-columns: repeat(auto-fit, minmax(250px, 1fr));
          gap: 1.5rem;
          margin: 2rem 0;
        }

        .welcome-card {
          padding: 1.5rem;
          background: var(--color-surface, #f8f9fa);
          border: 1px solid var(--color-border, #e1e4e8);
          border-radius: 12px;
          transition: all 0.2s ease-in-out;
        }

        .welcome-card:hover {
          transform: translateY(-2px);
          box-shadow: var(--shadow-md);
        }

        .card-icon {
          font-size: 2.5rem;
          margin-bottom: 1rem;
        }

        .welcome-card h3 {
          font-size: 1.25rem;
          margin-bottom: 0.5rem;
          color: var(--color-text-primary, #2c3e50);
        }

        .welcome-card p {
          font-size: 0.875rem;
          color: var(--color-text-secondary, #7f8c8d);
          line-height: 1.5;
        }

        .features {
          margin-top: 3rem;
        }

        .features h2 {
          font-size: 1.875rem;
          margin-bottom: 1rem;
          color: var(--color-text-primary, #2c3e50);
        }

        .features ul {
          list-style: none;
          padding: 0;
        }

        .features li {
          padding: 0.75rem 0;
          padding-left: 1.5rem;
          position: relative;
          color: var(--color-text-secondary, #7f8c8d);
        }

        .features li::before {
          content: '✓';
          position: absolute;
          left: 0;
          color: var(--color-success, #27ae60);
          font-weight: bold;
        }

        /* Component page styles */
        .component-page h1 {
          font-size: 2.25rem;
          margin-bottom: 0.5rem;
          color: var(--color-text-primary, #2c3e50);
        }

        .info-box {
          padding: 1rem;
          background: var(--color-surface, #f8f9fa);
          border-left: 4px solid var(--color-info, #3498db);
          border-radius: 4px;
          margin: 1.5rem 0;
        }

        .info-box strong {
          color: var(--color-text-primary, #2c3e50);
        }

        code {
          font-family: var(--font-family-mono, monospace);
          background: var(--color-code-background, #f6f8fa);
          color: var(--color-code-text, #24292e);
          padding: 0.125rem 0.375rem;
          border-radius: 4px;
          font-size: 0.875em;
        }

        /* Loading state */
        .loading-container {
          display: flex;
          flex-direction: column;
          align-items: center;
          justify-content: center;
          padding: 4rem 2rem;
          color: var(--color-text-secondary, #7f8c8d);
        }

        .loading-spinner {
          width: 40px;
          height: 40px;
          border: 3px solid var(--color-border, #e1e4e8);
          border-top-color: var(--color-accent, #3498db);
          border-radius: 50%;
          animation: spin 0.8s linear infinite;
          margin-bottom: 1rem;
        }

        @keyframes spin {
          to { transform: rotate(360deg); }
        }

        /* Error state */
        .error-container {
          display: flex;
          flex-direction: column;
          align-items: center;
          justify-content: center;
          padding: 4rem 2rem;
          text-align: center;
        }

        .error-icon {
          font-size: 3rem;
          margin-bottom: 1rem;
        }

        .error-container h2 {
          color: var(--color-error, #e74c3c);
          margin-bottom: 0.5rem;
        }

        .error-container button {
          margin-top: 1rem;
          padding: 0.5rem 1rem;
          background: var(--color-accent, #3498db);
          color: var(--color-text-inverse, #ffffff);
          border: none;
          border-radius: 6px;
          font-weight: 500;
          cursor: pointer;
          transition: background 0.2s ease-in-out;
        }

        .error-container button:hover {
          background: var(--color-accent-hover, #2980b9);
        }

        /* Mobile styles */
        @media (max-width: 767px) {
          .main-container {
            padding: 1rem;
          }

          .welcome h1 {
            font-size: 1.875rem;
          }

          .welcome-grid {
            grid-template-columns: 1fr;
          }
        }
      </style>

      <main class="main-container">
        <div class="content">
          <!-- Content will be inserted here -->
        </div>
      </main>
    `;

    // Load initial content
    this.loadContent(this.currentPath);
  }
}

customElements.define('gallery-main', GalleryMain);
