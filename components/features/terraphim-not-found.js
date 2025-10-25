import TerraphimElement from '../base/terraphim-element.js';

/**
 * TerraphimNotFound - 404 error page component
 *
 * Displays a friendly 404 error page when no route matches.
 */
class TerraphimNotFound extends TerraphimElement {
  connectedCallback() {
    super.connectedCallback();
    this.render();
    this._setupEventListeners();
  }

  _setupEventListeners() {
    const homeButton = this.shadowRoot.querySelector('.home-button');
    if (homeButton) {
      homeButton.addEventListener('click', this._handleHomeClick.bind(this));
    }

    const backButton = this.shadowRoot.querySelector('.back-button');
    if (backButton) {
      backButton.addEventListener('click', this._handleBackClick.bind(this));
    }
  }

  _handleHomeClick(event) {
    event.preventDefault();
    const router = window.terraphimRouter;
    if (router) {
      router.navigate('/');
    }
  }

  _handleBackClick(event) {
    event.preventDefault();
    window.history.back();
  }

  render() {
    this.shadowRoot.innerHTML = `
      ${this.getStyles()}
      <div class="not-found-container">
        <div class="not-found-content">
          <div class="error-code">404</div>
          <h1 class="title">Page Not Found</h1>
          <p class="message">
            The page you're looking for doesn't exist or has been moved.
          </p>
          <div class="actions">
            <button class="home-button button is-primary">
              <span class="icon">
                <i class="fas fa-home"></i>
              </span>
              <span>Go Home</span>
            </button>
            <button class="back-button button">
              <span class="icon">
                <i class="fas fa-arrow-left"></i>
              </span>
              <span>Go Back</span>
            </button>
          </div>
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

        .not-found-container {
          display: flex;
          align-items: center;
          justify-content: center;
          min-height: 400px;
          padding: 2rem;
        }

        .not-found-content {
          max-width: 600px;
          text-align: center;
        }

        .error-code {
          font-size: 6rem;
          font-weight: 700;
          color: #f14668;
          line-height: 1;
          margin-bottom: 1rem;
          text-shadow: 2px 2px 4px rgba(0, 0, 0, 0.1);
        }

        .title {
          font-size: 2rem;
          font-weight: 600;
          color: #363636;
          margin: 0 0 1rem 0;
        }

        .message {
          font-size: 1.125rem;
          color: #4a4a4a;
          margin: 0 0 2rem 0;
          line-height: 1.6;
        }

        .actions {
          display: flex;
          gap: 1rem;
          justify-content: center;
          flex-wrap: wrap;
        }

        .button {
          display: inline-flex;
          align-items: center;
          justify-content: center;
          gap: 0.5rem;
          padding: 0.75rem 1.5rem;
          border: 1px solid #dbdbdb;
          border-radius: 4px;
          background: white;
          color: #363636;
          font-size: 1rem;
          cursor: pointer;
          transition: all 0.2s;
          text-decoration: none;
        }

        .button:hover {
          border-color: #b5b5b5;
          background: #f5f5f5;
        }

        .button.is-primary {
          background: #3273dc;
          color: white;
          border-color: transparent;
        }

        .button.is-primary:hover {
          background: #2366d1;
        }

        .button:active {
          transform: translateY(1px);
        }

        .icon {
          display: inline-flex;
          align-items: center;
          justify-content: center;
        }

        @media screen and (max-width: 768px) {
          .error-code {
            font-size: 4rem;
          }

          .title {
            font-size: 1.5rem;
          }

          .message {
            font-size: 1rem;
          }

          .actions {
            flex-direction: column;
          }

          .button {
            width: 100%;
          }
        }
      </style>
    `;
  }
}

customElements.define('terraphim-not-found', TerraphimNotFound);

export default TerraphimNotFound;
