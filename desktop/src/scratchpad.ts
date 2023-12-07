// import type {
//     PostMessage,
//     PostMessageDataRequest,
//     PostMessageDataResponse,
// } from './workers/postmessage';

import { onMount } from 'svelte';

  // const onWorkerMessage = ({
  //   data: { msg, data },
  // }: MessageEvent<PostMessage<PostMessageDataResponse>>) => {
  //   console.log(msg, data);
  // };

  // let syncWorker: Worker | undefined = undefined;

  // const loadWorker = async () => {
  //   const SyncWorker = await import('$workers/fetcher.worker?worker');
  //   syncWorker = new SyncWorker.default();

  //   syncWorker.onmessage = onWorkerMessage;

  //   const message: PostMessage<PostMessageDataRequest> = {
  //     msg: 'request1',
  //     data: { text: 'Hello World v2 🤪' },
  //   };
  //   syncWorker.postMessage(message);
  // };

  // onMount(loadWorker);

// import { writable } from 'svelte/store'

// export default function (url) {
//     const loading = writable(false)
//     const error = writable(false)
//     const data = writable({})

//     async function get() {
//         loading.set(true)
//         error.set(false)
//         try {
//             const response = await fetch(url)
//             data.set(await response.json())
//         } catch (e) {
//             error.set(e)
//         }
//         loading.set(false)
//     }

//     get()

//     return [data, loading, error, get]
}