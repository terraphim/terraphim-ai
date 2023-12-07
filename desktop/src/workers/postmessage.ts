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