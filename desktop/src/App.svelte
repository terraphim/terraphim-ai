<script lang="ts">
  import logo from '/public/assets/terraphim_gray.png';
  import { writable } from 'svelte/store';
  import Shortcuts from './lib/Shortcuts.svelte';
  import '@fortawesome/fontawesome-free/css/all.css';
  import Search from './lib/Search/Search.svelte';
  import ThemeSwitcher from './lib/ThemeSwitcher.svelte';
  import { theme } from './lib/stores';
  import configStore from './lib/ThemeSwitcher.svelte';
  import { Route, router, active } from 'tinro';
  import FetchTabs from './lib/Fetchers/FetchTabs.svelte';
  import { initStore } from '@tomic/svelte';
  import { Store, Agent, properties, importJsonAdString} from '@tomic/lib';
  import { onMount } from 'svelte';

  // onMount(() => {
  //   const secret = prompt('Enter your secret');
  //   if (!secret) {
  //     console.log('No secret entered');
  //     return;
  //   }
  //   const agent = Agent.fromSecret(secret);
  //   // This is where you configure your atomic data store.
  //   const store = new Store({
  //     serverUrl: 'http://localhost:9883',
  //     agent,
  //   });
  //   initStore(store);
  // })
  let visible = "is-hidden";
  function toggleVissible() {
        visible = ""
    }
</script>

<svelte:head>
  <meta
    name="color-scheme"
    content={$theme == 'spacelab' ? 'lumen darkly' : $theme}
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
  <ul>
    {#if Array.isArray(configStore)}
      {#each configStore as item}
        <li>{item.name} {item.theme}</li>
      {/each}
    {/if}
  </ul>

  <!-- <nav class="navbar ">

    <a class="navbar-item " href="/" use:active exact>Home</a>
    <a class="navbar-item " href="/fetch" use:active>Fetchers</a>
  </nav> -->
  <Route path="/fetch/*"><FetchTabs /></Route>
</main>


<footer on:mouseover={toggleVissible}>
  <div class="{visible}">
  <Route path="/">
    
  
    
    <nav class="navbar ">
      <div class="navbar-brand">
        <a class="navbar-item" href="/">
          <span class="icon" style="color: #333;">
            <i class="fas fa-home">
            </i>
          </span>
        </a>
      <a class="navbar-item" href="/fetch/json">Fetch</a>
      <a class="navbar-item" href="/contacts">Contacts</a>
      </div>
  </nav>

</Route>
</div>
</footer>
</div>
<style>
  :root {
    font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, Oxygen,
      Ubuntu, Cantarell, 'Open Sans', 'Helvetica Neue', sans-serif;
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
