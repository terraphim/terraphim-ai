<script lang="ts">
import { active, Route, router } from 'tinro';
import '@fortawesome/fontawesome-free/css/all.css';
import logo from '/assets/terraphim_gray.png';
import Chat from './lib/Chat/Chat.svelte';
import ConfigJsonEditor from './lib/ConfigJsonEditor.svelte';
import ConfigWizard from './lib/ConfigWizard.svelte';
import RoleGraphVisualization from './lib/RoleGraphVisualization.svelte';
import Search from './lib/Search/Search.svelte';
import { theme } from './lib/stores';
import ThemeSwitcher from './lib/ThemeSwitcher.svelte';

// Svelte 5: Use $state rune for reactive local state
let _visible = $state('is-hidden');

function _toggleVissible() {
	_visible = '';
}

function goBack() {
	// Try to go back in browser history, fallback to home
	if (window.history.length > 1) {
		window.history.back();
	} else {
		router.goto('/');
	}
}
</script>

<svelte:head>
  <meta name="color-scheme" content={$theme} />
  <title>Terraphim</title>
</svelte:head>

<div class="is-full-height">
  <main class="main-content">
    <div class="top-controls">
      <div class="main-navigation">
        <div class="navigation-row">
  <button
    class="logo-back-button"
    on:click={goBack}
    on:keydown={(e) => {
      if (e.key === 'Enter' || e.key === ' ') {
        e.preventDefault();
        goBack();
      }
    }}
    title="Go back"
    aria-label="Go back"
  >
            <img src={logo} alt="Terraphim" class="logo-image" />
          </button>
          <div class="tabs is-boxed">
            <ul>
              <li>
                <a href="/" use:active data-exact data-testid="search-tab">
                  <span class="icon is-small"><i class="fas fa-search"></i></span>
                  <span>Search</span>
                </a>
              </li>
              <li>
                <a href="/chat" use:active data-testid="chat-tab">
                  <span class="icon is-small"><i class="fas fa-comments"></i></span>
                  <span>Chat</span>
                </a>
              </li>
              <li>
                <a href="/graph" use:active data-testid="graph-tab">
                  <span class="icon is-small"><i class="fas fa-project-diagram"></i></span>
                  <span>Graph</span>
                </a>
              </li>
            </ul>
          </div>
        </div>
      </div>
      <div class="role-selector">
        <ThemeSwitcher />
      </div>
    </div>
    <div class="main-area">
      <Route path="/"><Search /></Route>
      <Route path="/chat"><Chat /></Route>
      <Route path="/graph"><RoleGraphVisualization /></Route>
    </div>
    <br />

    <Route path="/config/wizard"><ConfigWizard /></Route>
    <Route path="/config/json"><ConfigJsonEditor /></Route>
  </main>

  <footer on:mouseover={_toggleVissible} on:focus={_toggleVissible}>
    <div class={_visible}>
      <Route path="/">
        <nav class="navbar">
          <div class="navbar-brand">
            <a class="navbar-item" href="/" aria-label="Go to home search">
              <span class="icon" style="color: #333;">
                <i class="fas fa-home"> </i>
              </span>
            </a>
            <a class="navbar-item" href="/config/wizard">Wizard</a>
            <a class="navbar-item" href="/config/json">JSON&nbsp;Editor</a>
            <a class="navbar-item" href="/graph">Graph</a>
            <a class="navbar-item" href="/chat">Chat</a>
          </div>
        </nav>
      </Route>
    </div>
  </footer>
</div>

<style>
  :global(body) {
    font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, Oxygen,
      Ubuntu, Cantarell, "Open Sans", "Helvetica Neue", sans-serif;
  }
  .is-full-height {
    min-height: 100vh;
    flex-direction: column;
    display: flex;
  }
  .main-content {
    flex: 1;
    padding-left: 1em;
    padding-right: 1em;
  }
  .top-controls {
    display: flex;
    justify-content: space-between;
    align-items: flex-start;
    margin-bottom: 1rem;
    padding-top: 0.5rem;
  }
  .main-navigation {
    flex: 1;
  }
  .navigation-row {
    display: flex;
    align-items: center;
    gap: 0;
  }
  .navigation-row .tabs {
    flex: 1;
    margin-bottom: 0;
  }

  .logo-back-button {
    background: none;
    border: none;
    padding: 0.5rem;
    margin-right: 1rem;
    cursor: pointer;
    border-radius: 4px;
    transition: all 0.2s ease;
    display: flex;
    align-items: center;
    justify-content: center;
    flex-shrink: 0;
  }

  .logo-back-button:hover {
    background-color: rgba(0, 0, 0, 0.05);
    transform: translateY(-1px);
    box-shadow: 0 2px 4px rgba(0, 0, 0, 0.1);
  }

  .logo-back-button:active {
    transform: translateY(0);
    box-shadow: 0 1px 2px rgba(0, 0, 0, 0.1);
  }

  .logo-image {
    height: 32px;
    width: auto;
    object-fit: contain;
  }

  /* Responsive design */
  @media (max-width: 768px) {
    .logo-back-button {
      margin-right: 0.5rem;
      padding: 0.25rem;
    }
    .logo-image {
      height: 28px;
    }
  }
  .role-selector {
    min-width: 200px;
    margin-left: 1rem;
  }
  .main-area {
    margin-top: 0;
  }

  /* Active navigation tab styles */
  :global(.tabs li:has(a.active)) {
    border-bottom-color: #3273dc;
  }
  :global(.tabs a.active) {
    color: #3273dc !important;
    border-bottom-color: #3273dc !important;
  }

  /* Fallback for browsers that don't support :has() selector */
  @supports not (selector(:has(*))) {
    :global(.tabs a.active) {
      background-color: #f5f5f5;
      border-bottom: 3px solid #3273dc;
    }
  }
  footer {
    flex-shrink: 0;
    text-align: center;
    padding: 1em;
  }
</style>
