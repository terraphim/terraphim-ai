<script>
import { emit, listen } from '@tauri-apps/api/event';
import { invoke } from '@tauri-apps/api/tauri';

let { onMessage } = $props();
let unlisten = $state();

$effect(() => {
	listen('rust-event', onMessage).then(fn => unlisten = fn);
	return () => {
		if (unlisten) {
			unlisten();
		}
	};
});

function _log() {
	invoke('log_operation', {
		event: 'tauri-click',
		payload: 'this payload is optional because we used Option in Rust',
	});
}

function _performRequest() {
	invoke('perform_request', {
		endpoint: 'dummy endpoint arg',
		body: {
			id: 5,
			name: 'test',
		},
	})
		.then(onMessage)
		.catch(onMessage);
}

function _emitEvent() {
	emit('js-event', 'this is the payload string');
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
