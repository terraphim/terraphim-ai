<<<<<<< HEAD
import type {
  FetcherRequestData,
  FetcherResponseData,
  WorkerRequestMessage,
  WorkerResponseMessage,
} from "./postmessage";

async function postJSON(url: string, data: unknown): Promise<void> {
  const response = await fetch(url, {
    method: "POST",
    headers: {
      "Content-Type": "application/json",
    },
    body: JSON.stringify(data),
  });

  if (!response.ok) {
    const message = await response.text().catch(() => response.statusText);
    throw new Error(`Failed to post document: ${response.status} ${message}`);
  }
}

async function fetcher(
  url: string,
  postUrl: string,
  isWiki: boolean
): Promise<FetcherResponseData> {
  let fetchUrl = url;
  if (isWiki) {
    fetchUrl = `${url}?action=raw`;
  }

  try {
    const response = await fetch(fetchUrl);
    if (!response.ok) {
      const message = await response.text().catch(() => response.statusText);
      throw new Error(`Fetch failed: ${response.status} ${message}`);
    }

    const payload = await response.json();
    if (isWiki) {
      return {
        text: "Fetched wiki content; no posting performed",
      };
    }

    if (Array.isArray(payload)) {
      for (const document of payload) {
        await postJSON(postUrl, document);
      }
      return { text: `Posted ${payload.length} documents` };
    } else if (typeof payload === "object" && payload !== null) {
      await postJSON(postUrl, payload);
      return { text: "Posted single document" };
    }

    return {
      text: null,
      error: "Fetched data was not a JSON object or array",
    };
  } catch (error) {
    const message = error instanceof Error ? error.message : String(error);
    console.error(error);
    return { text: null, error: message };
  }
}

self.onmessage = async ({
  data,
}: MessageEvent<WorkerRequestMessage>): Promise<void> => {
  if (data.msg !== "fetcher" || !data.data) {
    return;
  }

  const { url, postUrl, isWiki } = data.data as FetcherRequestData;
  const result = await fetcher(url, postUrl, isWiki);

  const message: WorkerResponseMessage = {
    msg: "response1",
    data: result,
  };
  self.postMessage(message);
=======
import type { PostMessage, PostMessageDataRequest } from './postmessage';

let result: Error | null = null;
async function postJSON(url: string, data: Record<string, unknown>) {
	try {
		const response = await fetch(url, {
			method: 'POST',
			headers: {
				'Content-Type': 'application/json',
				// 'Content-Type': 'application/x-www-form-urlencoded',
			},
			body: JSON.stringify(data),
		});
		const _json = await response.json();
	} catch (e) {
		result = e;
	}
}
async function fetcher(url: string, postUrl: string, isWiki = false) {
	let fetched: Response | null = null;
	if (isWiki) {
		url = `${url}?action=raw`;
	}

	try {
		const response = await fetch(url);
		fetched = await response.json();
		if (isWiki) {
		} else {
			const obj = fetched;
			// loop over list of document in fetched object
			for (let i = 0; i < obj.length; i++) {
				const document = obj[i];
				postJSON(postUrl, document);
			}
		}
	} catch (e) {
		result = e;
	}
}
const handleMessage = ({ data: { data } }: MessageEvent<PostMessage<PostMessageDataRequest>>) => {
	fetcher(data.url, data.postUrl, data.isWiki);

	const message: PostMessage<PostMessageDataRequest> = {
		msg: 'response1',
		data: { text: result },
	};
	postMessage(message);
>>>>>>> origin/main
};

self.onmessage = handleMessage;
