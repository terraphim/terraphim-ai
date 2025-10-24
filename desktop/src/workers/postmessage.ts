<<<<<<< HEAD
export interface FetcherRequestData {
  url: string;
  postUrl: string;
  isWiki: boolean;
}

export interface FetcherResponseData {
  text: string | null;
  error?: string;
}

export type WorkerRequestMessage = {
  msg: "fetcher";
  data: FetcherRequestData;
};

export type WorkerResponseMessage = {
  msg: "response1";
  data: FetcherResponseData;
};

export type WorkerMessage = WorkerRequestMessage | WorkerResponseMessage;
=======
export interface PostMessageDataRequest {
	text: string;
}

export interface PostMessageDataResponse {
	text: string;
}

export type PostMessageRequest = 'request1' | 'start' | 'stop';
export type PostMessageResponse = 'response1' | 'response2';

export interface PostMessage<T extends PostMessageDataRequest | PostMessageDataResponse> {
	msg: PostMessageRequest | PostMessageResponse;
	data?: T;
}
>>>>>>> origin/main
