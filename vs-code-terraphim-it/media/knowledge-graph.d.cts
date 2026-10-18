export function exportKnowledgeGraph(source: string, assetDirectory: string): Promise<string>;

export function checkpointProjection(source: string, assetDirectory: string): Promise<{presentationSource:string;body:string}>;
