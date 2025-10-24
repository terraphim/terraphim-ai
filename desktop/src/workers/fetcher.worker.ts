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
};

self.onmessage = handleMessage;
