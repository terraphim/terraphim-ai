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

export interface NodeRank {
  edge_weight: number;
  document_id: string;
}

export interface Node {
  id: string;
  normalized_term: string;
  ranks: NodeRank[];
  total_documents: number;
}

export interface NodeResponse {
  status: string;
  nodes: Node[];
}
