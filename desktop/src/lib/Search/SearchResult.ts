export interface Document {
  id: string;
  url: string;
  title: string;
  body: string;
  description?: string;
  stub?: string;
  tags?: string[];
  rank?: number;
}

export interface SearchResponse {
  status: string;
  results: Document[];
  total: number;
}
