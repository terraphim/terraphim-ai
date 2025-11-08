<script lang="ts">
import { invoke } from '@tauri-apps/api/core';
import { Agent } from '@tomic/lib';
import { store } from '@tomic/svelte';
import { JSONEditor } from 'svelte-jsoneditor';

import { configStore, is_tauri } from '$lib/stores';
import { CONFIG } from '../../config';
import FetchRole from './FetchRole.svelte';

let _content = {
	json: $configStore,
};
function _handleChange(updatedContent) {
	console.log('contents changed:', updatedContent);
	console.log('is tauri', $is_tauri);
	configStore.update((config) => {
		config = updatedContent.json;
		return config;
	});
	if (is_tauri) {
		console.log('Updating config on server');
		invoke('update_config', { configNew: updatedContent.json })
			.then((res) => {
				console.log(`Message: ${res}`);
			})
			.catch((e) => console.error(e));
	} else {
		// post to server using /api/config
		const configURL = `${CONFIG.ServerURL}/config/`;
		fetch(configURL, {
			method: 'POST',
			headers: {
				'Content-Type': 'application/json',
			},
			body: JSON.stringify(updatedContent.json),
		});
	}
	_content = updatedContent;
	_content;
}
let isWiki = false;
let fetchUrl =
	'https://raw.githubusercontent.com/terraphim/terraphim-cloud-fastapi/main/data/ref_arch.json';
let postUrl = 'http://localhost:8000/documents/';
let atomicServerUrl = 'http://localhost:9883/';
let agentSecret: string | undefined;
const _setAtomicServer = async () => {
	console.log('Updating atomic server configuration');
	const agent = Agent.fromSecret(agentSecret);
	store.setServerUrl(atomicServerUrl);
	console.log('Server set.Setting agent');
	// Type assertion needed due to different @tomic/lib versions between dependencies
	store.setAgent(agent as any);
};

const _handleClickUrl = async () => {
	loadWorker();
};

// import fetchStore from './fetch.js';
// const [data, loading, error, get] = fetchStore(url)
import type {
	PostMessage,
	PostMessageDataRequest,
	PostMessageDataResponse,
} from '$workers/postmessage.ts';

const onWorkerMessage = ({
	data: { msg, data },
}: MessageEvent<PostMessage<PostMessageDataResponse>>) => {
	console.log(msg, data);
};

let syncWorker: Worker | undefined;

const loadWorker = async () => {
	const SyncWorker = await import('$workers/fetcher.worker?worker');
	syncWorker = new SyncWorker.default();

	syncWorker.onmessage = onWorkerMessage;

	const message: PostMessage<PostMessageDataRequest> = {
		msg: 'fetcher',
		data: {
			url: fetchUrl,
			postUrl: postUrl,
			isWiki: isWiki,
		},
	};
	syncWorker.postMessage(message);
};

import { urls } from '@tomic/lib';
// This functiolity related to atomic server
import { getResource, getValue } from '@tomic/svelte';

// const resource = $store.getResourceLoading('http://localhost:9883/config/y3zx5wtm0bq');
const resource1 = getResource('http://localhost:9883/config/y3zx5wtm0bq');

const _name = getValue<string>(resource1, urls.properties.name);
const _roles = getValue<string[]>(resource1, 'http://localhost:9883/property/role');
// FIXME: update roles to configStore
$: console.log('Print name', $_name);
$: console.log('Print roles', $_roles);
</script>

<div class="box">
  <!-- <Tab label="Atomic"> -->
  <div class="tab-content">
    <div class="field">
      <div class="control">
        <input class="input" type="text" bind:value={atomicServerUrl} />
      </div>
    </div>
    <div class="field is-grouped">
      <div class="control has-icons-left is-expanded">
        <input
          class="input"
          type="password"
          placeholder="secret"
          bind:value={agentSecret}
        />
        <span class="icon is-left">
          <i class="fas fa-lock"></i>
        </span>
      </div>
    </div>
    <div class="field is-grouped">
      <div class="control">
        <button
          class="button is-success is-right"
          on:click={_setAtomicServer}
          on:submit={_setAtomicServer}
        >
          <span class="icon">
            <i class="fa fa-check"></i>
          </span>
          <span>Save</span>
        </button>
      </div>
    </div>
  </div>
  <!-- <Tab label="JSON"> -->
  <div class="tab-content">
    <div class="field is-grouped">
      <div class="control has-icons-left is-expanded">
        <input
          class="input"
          type="search"
          placeholder="Fetch JSON"
          bind:value={fetchUrl}
        />
        <span class="icon is-left">
          <i class="fas fa-search"></i>
        </span>
      </div>
    </div>
    <div class="field is-grouped">
      <div class="control has-icons-left is-expanded">
        <input
          class="input"
          type="search"
          placeholder="Post JSON"
          bind:value={postUrl}
        />
        <span class="icon is-left">
          <i class="fas fa-search"></i>
        </span>
      </div>
      <div class="control">
        <label class="checkbox">
          <input type="checkbox" bind:checked={isWiki} />
          WikiPage
        </label>
      </div>
    </div>
    <div class="field is-grouped">
      <div class="control">
        <button
          class="button is-primary"
          on:click={_handleClickUrl}
          on:submit={_handleClickUrl}
        >
          Fetch
        </button>
      </div>
    </div>
  </div>
  <div class="tab-content">
    <p>
      <i
        >The best editing experience is to configure Atomic Server, in the
        meantime use editor below. You will need to refresh page via Command R
        or Ctrl-R to see changes</i
      >
    </p>
    <div class="editor">
      <JSONEditor content={_content} onChange={_handleChange} />
    </div>
  </div>
</div>
<hr />
<div class="field is-grouped is-grouped-right">
  <div class="control">
    <div class="select">
      <select>
        {#each $_roles ?? [] as role_value}
          <FetchRole subject={role_value} />
        {/each}
      </select>
    </div>
  </div>
</div>
<nav class="navbar">
  <div class="navbar-brand">
    <a class="navbar-item" href="/">
      <!-- FIXME: replace home icon with terraphim -->
      <span class="icon" style="color: #333;">
        <i class="fas fa-home"> </i>
      </span>
    </a>
    <a class="navbar-item" href="/fetch/json">Fetch JSON Data</a>
    <a class="navbar-item" href="/fetch/atomic">Set Atomic Server</a>
    <a class="navbar-item" href="/fetch/editor">Edit JSON config</a>
  </div>
</nav>
