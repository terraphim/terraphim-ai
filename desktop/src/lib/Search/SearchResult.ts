export interface Document {
  id: string;
  title: string;
  description?: string;
  url?: string;
  body?: string;
  tags?: string[];
  rank?: number;
  matched_edges?: any[];
  nodes?: any[];
}

export interface SearchResponse {
  status: string;
  results: Document[];
}

export interface ChartNode {
  name: string;
  size: number;
  color: string;
  children?: ChartNode[];
}

export interface ChartData {
  name: string;
  children: ChartNode[];
}

export interface RankEntry {
  edge_weight: number;
  document_id: string;
}

export interface ApiRankedNode {
  id: string;
  name: string;
  normalized_term: string;
  value: number;
  total_documents: number;
  parent: string | null;
  children: ApiRankedNode[];
  expanded: boolean;
  ranks: RankEntry[];
}

export interface NodeResponse {
  status: string;
  name: string;
  nodes: ApiRankedNode[];
}
