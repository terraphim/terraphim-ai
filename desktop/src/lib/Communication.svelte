<script>
  import { emit, listen } from "@tauri-apps/api/event";
  import { invoke } from "@tauri-apps/api/tauri";
  import { onDestroy, onMount } from "svelte";

  export let onMessage;
  let unlisten;

  onMount(async () => {
    unlisten = await listen("rust-event", onMessage);
  });
  onDestroy(() => {
    if (unlisten) {
      unlisten();
    }
  });

  function log() {
    invoke("log_operation", {
      event: "tauri-click",
      payload: "this payload is optional because we used Option in Rust",
    });
  }

  function performRequest() {
    invoke("perform_request", {
      endpoint: "dummy endpoint arg",
      body: {
        id: 5,
        name: "test",
      },
    })
      .then(onMessage)
      .catch(onMessage);
  }

  function emitEvent() {
    emit("js-event", "this is the payload string");
  }
</script>

<div>
  <button class="button" id="log" on:click={log}>Call Log API</button>
  <button class="button" id="request" on:click={performRequest}>
    Call Request (async) API
  </button>
  <button class="button" id="event" on:click={emitEvent}>
    Send event to Rust
  </button>
</div>
