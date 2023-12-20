<script lang="ts">
  import { Tag, Taglist } from 'svelma';
  import { fade } from 'svelte/transition';
  import ArticleModal from './ArticleModal.svelte';
  import type { SearchResult } from './SearchResult';
  import configStore from '../ThemeSwitcher.svelte';
  import { role, is_tauri, serverUrl } from '../stores';
  export let item: SearchResult;
  let showModal = false;

  const onTitleClick = () => {
    showModal = true;
  };
  
  if (configStore[$role]!== undefined){
    console.log("Have attribute",configStore[$role]);
    if (configStore[$role].hasOwnProperty('enableLogseq')) {
      // The attribute exists
      // Do something here
      console.log("enable logseq True");
    } else {
      // The attribute does not exist
      // Do something else here
    }
  }{
    console.log("Didn't make it");
  }
</script>

<div class="box">
  <article class="media">
    <div class="media-content">
      <div class="content">
        <div class="level-right">
          <Taglist>
            {#if item.tags}
            {#each Object.entries(item.tags) as [tag, url]}
            <a href="https://terraphim.github.io/terraphim-project/#/page/{tag}" target="_blank"><Tag rounded>{tag}</Tag></a>
            {/each}
            {/if}
          </Taglist>
        </div>
        <div transition:fade>
          <button on:click={onTitleClick}>
            <h2 class="title">
              {item.title}
            </h2>
          </button>
          <small>Description: {item.description}</small>
          <small />
          <br />
        </div>
      </div>
      <div class="level-right">
        <nav class="level is-mobile" transition:fade>
          <div class="level-right">
            <a
              href={item.url}
              target="_blank"
              class="level-item"
              aria-label="URL"
            >
              <span class="icon is-medium">
                <i class="fas fa-link" />
              </span>
            </a>
        
            <a href="logseq://x-callback-url/quickCapture?title={item.title}&url={item.url}" class="level-item" aria-label="download/save">
              <span class="icon is-medium">
                <i class="fas fa-download" aria-hidden="true" />
              </span>
            </a>
            <a href="#" class="level-item" aria-label="like">
              <span class="icon is-medium">
                <i class="fas fa-plus" aria-hidden="true" />
              </span>
            </a>
            <a href="#" class="level-item" aria-label="like">
              <span class="icon is-medium">
                <i class="fas fa-bookmark" aria-hidden="true"/>
              </span>
            </a>
          </div>
        </nav>
      </div>
    </div>
  </article>
</div>
<ArticleModal bind:active={showModal} {item} />

<style lang="scss">
  button {
    background: none;
    border: none;
    padding: 0;
    font: inherit;
    cursor: pointer;
    outline: inherit;
    display: block;
  }
  .title {
    font-size: 1.3em;
    margin-bottom: 0px;

    &:hover,
    &:focus {
      text-decoration: underline;
    }
  }
</style>
