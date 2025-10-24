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
