import TerraphimElement from '../base/terraphim-element.js';

/**
 * TerraphimGraph - Knowledge graph visualization component (stub)
 *
 * This is a placeholder component for the graph visualization feature.
 * Full implementation will be added in Phase 3.
 */
class TerraphimGraph extends TerraphimElement {
  connectedCallback() {
    super.connectedCallback();
    this.render();
  }

  render() {
    this.shadowRoot.innerHTML = `
      ${this.getStyles()}
      <div class="stub-container">
        <div class="stub-content">
          <span class="stub-icon">🕸️</span>
          <h1 class="title">Knowledge Graph</h1>
          <p class="subtitle">Visualize your knowledge connections</p>
          <p class="message">
            This feature is under development. The graph visualization will allow you to:
          </p>
          <ul class="feature-list">
            <li>Explore knowledge graph relationships</li>
            <li>Visualize concept connections</li>
            <li>Navigate between related topics</li>
            <li>Discover new insights</li>
          </ul>
          <p class="coming-soon">Coming soon in Phase 3!</p>
        </div>
      </div>
    `;
  }

  getStyles() {
    return `
      <style>
        :host {
          display: block;
          width: 100%;
          height: 100%;
        }

        .stub-container {
          display: flex;
          align-items: center;
          justify-content: center;
          min-height: 400px;
          padding: 2rem;
        }

        .stub-content {
          max-width: 600px;
          text-align: center;
        }

        .stub-icon {
          font-size: 4rem;
          display: block;
          margin-bottom: 1rem;
        }

        .title {
          font-size: 2rem;
          font-weight: 600;
          color: #363636;
          margin: 0 0 0.5rem 0;
        }

        .subtitle {
          font-size: 1.25rem;
          color: #7a7a7a;
          margin: 0 0 1.5rem 0;
        }

        .message {
          font-size: 1rem;
          color: #4a4a4a;
          margin: 1rem 0;
        }

        .feature-list {
          text-align: left;
          margin: 1.5rem auto;
          max-width: 400px;
          list-style: none;
          padding: 0;
        }

        .feature-list li {
          padding: 0.5rem 0;
          padding-left: 1.5rem;
          position: relative;
        }

        .feature-list li::before {
          content: '✓';
          position: absolute;
          left: 0;
          color: #48c774;
          font-weight: bold;
        }

        .coming-soon {
          margin-top: 2rem;
          font-size: 1.1rem;
          color: #3273dc;
          font-weight: 500;
        }
      </style>
    `;
  }
}

customElements.define('terraphim-graph', TerraphimGraph);

export default TerraphimGraph;
