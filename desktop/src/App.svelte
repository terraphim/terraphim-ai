<script lang="ts">
  import "@fortawesome/fontawesome-free/css/all.css";
  import { Route } from "tinro";
  import FetchTabs from "./lib/Fetchers/FetchTabs.svelte";
  import Search from "./lib/Search/Search.svelte";
  import {
    default as ThemeSwitcher,
    default as configStore,
  } from "./lib/ThemeSwitcher.svelte";
  import { theme } from "./lib/stores";
  let visible = "is-hidden";
  function toggleVissible() {
    visible = "";
  }
</script>

<svelte:head>
  <meta
    name="color-scheme"
    content={$theme == "spacelab" ? "lumen darkly" : $theme}
  />
  <link
    rel="stylesheet"
    href={`/assets/bulmaswatch/${$theme}/bulmaswatch.min.css`}
  />
</svelte:head>

<div class="is-full-height">
  <main class="main-content">
    <ThemeSwitcher />
    <Route path="/">
      <Search />
    </Route>
    <br />

    <Route path="/fetch/*"><FetchTabs /></Route>
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
            <a class="navbar-item" href="/fetch/json">Configuration</a>
            <a class="navbar-item" href="/contacts">Contacts</a>
          </div>
        </nav>
      </Route>
    </div>
  </footer>
</div>

<style>
  :root {
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
  footer {
    flex-shrink: 0;
    text-align: center;
    padding: 1em;
  }
</style>