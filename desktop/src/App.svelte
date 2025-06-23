<script lang="ts">
  import { Route } from "tinro";
  import FetchTabs from "./lib/Fetchers/FetchTabs.svelte";
  import Search from "./lib/Search/Search.svelte";
  import {
    default as ThemeSwitcher,
  } from "./lib/ThemeSwitcher.svelte";
  import { theme } from "./lib/stores";
  import ConfigWizard from "./lib/ConfigWizard.svelte";
  import ConfigJsonEditor from "./lib/ConfigJsonEditor.svelte";

  let visible = "is-hidden";
  function toggleVissible() {
    visible = "";
  }
</script>

<svelte:head>
  <meta name="color-scheme" content={$theme} />
  <title>Terraphim</title>
</svelte:head>

<div class="is-full-height">
  <main class="main-content">
    <div class="top-controls">
      <div class="role-selector">
        <ThemeSwitcher />
      </div>
    </div>
    <div class="search-area">
      <Route path="/"><Search /></Route>
    </div>
    <br />

    <Route path="/fetch/*"><FetchTabs /></Route>
    <Route path="/config/wizard"><ConfigWizard/></Route>
    <Route path="/config/json"><ConfigJsonEditor/></Route>
  </main>

  <footer on:mouseover={toggleVissible} on:focus={toggleVissible}>
    <div class={visible}>
      <Route path="/">
        <nav class="navbar">
          <div class="navbar-brand">
            <a class="navbar-item" href="/">
              <span class="icon" style="color: #333;">
                <i class="fas fa-home"> </i>
              </span>
            </a>
            <a class="navbar-item" href="/config/wizard">Wizard</a>
            <a class="navbar-item" href="/config/json">JSON&nbsp;Editor</a>
            <a class="navbar-item" href="/contacts">Contacts</a>
          </div>
        </nav>
      </Route>
    </div>
  </footer>
</div>

<style>
  @import "@fortawesome/fontawesome-free/css/all.css";
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
    justify-content: flex-end;
    margin-bottom: 1rem;
    padding-top: 0.5rem;
  }
  .role-selector {
    min-width: 200px;
  }
  .search-area {
    margin-top: 0;
  }
  footer {
    flex-shrink: 0;
    text-align: center;
    padding: 1em;
  }
</style>