<script lang="ts">
  // import { Tabs, Tab } from 'svelma';
  import {Route} from 'tinro'; 
  import { Button, Field, Icon, Input } from 'svelma';
  import { Select } from 'svelma';
  import FetchRole from './FetchRole.svelte';
  import { Switch } from 'svelma';
  import {Agent} from '@tomic/lib';
  let data;
  let isWiki=false;
  let fetchUrl = 'https://raw.githubusercontent.com/terraphim/terraphim-cloud-fastapi/main/data/ref_arch.json';
  let postUrl = 'http://localhost:8000/article/';
  let atomicServerUrl='http://localhost:9883/';
  let agentSecret;
  const setAtomicServer = async () => {
    console.log("Updating atomic server configuration");
    const agent = Agent.fromSecret(agentSecret);
    $store.setServerUrl(atomicServerUrl);
    console.log("Server set.Setting agent");
    $store.setAgent(agent);
  };

  const handleClickUrl = async () => {
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

  let syncWorker: Worker | undefined = undefined;

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
  import { store } from '@tomic/svelte';
  import { getResource, getValue } from '@tomic/svelte';
  import { urls } from '@tomic/lib';
  
  // const resource = $store.getResourceLoading('http://localhost:9883/config/y3zx5wtm0bq');
  const resource1 = getResource('http://localhost:9883/config/y3zx5wtm0bq');
  
  const name = getValue<string>(resource1, urls.properties.name);
  const roles = getValue<string[]>(resource1, "http://localhost:9883/property/role");
  $: console.log("Print name", $name);
  $: console.log("Print roles", $roles);
</script>
<div class="box">
  <!-- <Tab label="Atomic"> -->
    <Route path="/atomic">
    <Field>
      <Input
        bind:value={atomicServerUrl}
      />
      </Field>
      <Field grouped>
      <Input
        type="password"
        placeholder="secret"
        icon="fas fa-lock"
        expanded
        bind:value={agentSecret}
      />
      </Field>
      <Field grouped>
        <Button type="is-success" class="is-right" iconPack="fa" iconLeft="check" on:click={setAtomicServer} on:submit={setAtomicServer}>Save</Button>
    </Field>
  </Route>
  <!-- <Tab label="JSON"> -->
    <Route path="/json">
    <Field grouped>
      <Input
        type="search"
        placeholder="Fetch JSON"
        icon="search"
        bind:value={fetchUrl}
      />
      </Field>
      <Field grouped>
      <Input
        type="search"
        placeholder="Post JSON"
        icon="search"
        bind:value={postUrl}
      />
        <Switch bind:checked={isWiki}>WikiPage</Switch>
        </Field>
        <Field grouped>
        <Button
          type="is-primary"
          on:click={handleClickUrl}
          on:submit={handleClickUrl}>Fetch</Button
        >
    </Field>
  </Route>
</div>
<hr>
<Field grouped position="is-right">
  <Select >
    {#each $roles??[] as role_value}
      <FetchRole subject={role_value} />
    {/each}
  </Select>
</Field>
<nav class="navbar ">
  <div class="navbar-brand">
  <a class="navbar-item" href="/">
    <!-- FIXME: replace home icon with terraphim -->
    <span class="icon" style="color: #333;">
      <i class="fas fa-home">
      </i>
    </span>
  </a>
  <a class="navbar-item" href="/fetch/json">Fetch JSON</a>
  <a class="navbar-item" href="/fetch/atomic">Set Atomic Server</a>
</div>
</nav>