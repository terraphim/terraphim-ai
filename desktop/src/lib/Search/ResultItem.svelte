<script lang="ts">
  import { Taglist, Tag } from "svelma";
  import { fade } from "svelte/transition";
  import ArticleModal from "./ArticleModal.svelte";
  import type { Document } from "./SearchResult";
  import configStore from "../ThemeSwitcher.svelte";
  import { role } from "../stores";

  export let document: Document;
  let showModal = false;

  const onTitleClick = () => {
    showModal = true;
  };

  if (configStore[$role] !== undefined) {
    console.log("Have attribute", configStore[$role]);
    if (configStore[$role].hasOwnProperty("enableLogseq")) {
      console.log("enable logseq True");
    } else {
      console.log("Didn't make it");
    }
  }
</script>

<div class="box">
  <article class="media">
    <div class="media-content">
      <div class="content">
        <div class="level-right">
          {#if document.tags}
          <Taglist>
              {#each document.tags as tag}
              <!-- FIXME: link shall be config parameter for KG -->
                <a
                  href="https://terraphim.github.io/terraphim-project/#/page/{tag}"
                  target="_blank"><Tag rounded>{tag}</Tag></a
                >
              {/each}
          </Taglist>
          {/if}
        </div>
          <div class="level-right">
          <Taglist>
            <Tag rounded>Rank {document.rank}</Tag>
          </Taglist>
        </div>
        <div transition:fade>
          <button on:click={onTitleClick}>
            <h2 class="title">
              {document.title}
            </h2>
          </button>
          <small
            >Description: {document.description ||
              "No description available"}</small
          >
          <br />
        </div>
      </div>
      <div class="level-right">
        <nav class="level is-mobile" transition:fade>
          <div class="level-right">
            {#if "url" in document}
              <a
                href={document.url}
                target="_blank"
                class="level-item"
                aria-label="URL"
              >
                <span class="icon is-medium">
                  <i class="fas fa-link" />
                </span>
              </a>
            {/if}
            <a href="#" class="level-item" aria-label="like">
              <span class="icon is-medium">
                <i class="fas fa-plus" aria-hidden="true" />
              </span>
            </a>
            <a
            href={`vscode://${encodeURIComponent(document.title)}.md?${encodeURIComponent(document.body)}`}
            class="level-item"
            aria-label="Open in VSCode"
          >
            <span class="icon is-medium">
              <i class="fas fa-code" aria-hidden="true" />
            </span>
          </a>
          </div>
        </nav>
      </div>
    </div>
  </article>
</div>
<ArticleModal bind:active={showModal} item={document} />

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
