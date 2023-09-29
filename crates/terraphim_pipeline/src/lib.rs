use ahash::{AHashMap};
use memoize::memoize;
use itertools::Itertools;
use regex::Regex;
use std::collections::hash_map::Entry;


use terraphim_automata::load_automata;
use terraphim_automata::matcher::{find_matches_ids, Dictionary};
use unicode_segmentation::UnicodeSegmentation;
use log::{ warn};
// use tracing::{debug, error, info, span, warn, Level};


// Reference to external storage of documents, traditional indexes use document, aka article or entity.
#[derive(Debug, Clone)]
pub struct Document {
    id: String,
    // matched to edges
    matched_to: Vec<Edge>,
    rank: u64,
    //normalized rank
    normalized_rank: f32,
}


//TODO: create top_k_nodes function where
// sort nodes by rank
// TODO create top_k_edges function where
//sort edges by rank
// TODO create top_k_documents function where
// sort document id by rank


#[derive(Debug, Clone)]
pub struct RoleGraph {
    // role filter
    role: String,
    nodes: AHashMap<u64, Node>,
    edges: AHashMap<u64, Edge>,
    documents: AHashMap<String, Document>,
    automata_url: String,
    dict_hash: AHashMap<String, Dictionary>,

}
impl RoleGraph {
    pub fn new(role: String, automata_url: &str) -> Self {
        let dict_hash = load_automata(automata_url).unwrap();
        Self {
            role,
            nodes: AHashMap::new(),
            edges: AHashMap::new(),
            documents: AHashMap::new(),
            automata_url: automata_url.to_string(),
            dict_hash: dict_hash,

        }
    }
    //  Query the graph using a query string, returns a list of document ids ranked and weighted by weighted mean average of node rank, edge rank and document rank

    // node rank is a weight for edge and edge rank is a weight for document_id
    // create hashmap of output with document_id, rank to dedupe documents in output
    // normalise output rank from 1 to number of records
    // pre-sort document_id by rank using BtreeMap
    //  overall weighted average is calculated a weighted average of node rank and edge rank and document rank
    //  weighted average  can be calculated: sum of (weight*rank)/sum of weights for each node, edge and document.
    //  rank is a number of co-occurences normalised over number of documents (entities), see cleora train function
    // YAGNI: at the moment I don't need it, so parked
    pub fn normalise(&mut self){
        let node_len=self.nodes.len() as u32;
        warn!("Node Length {}", node_len);
        let edge_len=self.edges.len() as u32;
        warn!("Edge Length {}", edge_len);
        let document_count=self.documents.len() as u32 ;
        warn!("document Length {}", document_count);
        let normalizer=f32::from_bits(node_len+edge_len+document_count);
        let weight_node=f32::from_bits(node_len)/normalizer;
        let weight_edge=f32::from_bits(edge_len)/normalizer;
        let weight_document=f32::from_bits(document_count)/normalizer;
        warn!("Weight Node {}", weight_node);
        warn!("Weight Edge {}", weight_edge);
        warn!("Weight document {}", weight_document);
        // for each node for each edge for each document
        // for (document_id,rank) in self.documents.iter(){
        //     let weighted_rank=(weight_node*node_rank as f32)+(weight_edge*edge_rank as f32)+(weight_document*rank as f32)/(weight_node+weight_edge+weight_document);
        //     warn!("document id {} Weighted Rank {}", document_id, weighted_rank);
        //     sorted_vector_by_rank_weighted.push((document_id, weighted_rank));
        // }


    }


    pub fn query(&self, query_string: &str)->Vec<(&String, Document)> {
        warn!("performing query");
        // FIXME: handle case when no matches found with empty non empty vector - otherwise all ranks will blow up
        let nodes = find_matches_ids(query_string, &self.dict_hash).unwrap_or(Vec::from([1]));
        
        let mut results_map= AHashMap::new();
        for node_id in nodes.iter() {
            // warn!("Matched node {:?}", node_id);
            let node = self.nodes.get(node_id).unwrap();
            let node_rank=node.rank;
            // warn!("Node Rank {}", node_rank);
            // warn!("Node connected to Edges {:?}", node.connected_with);
            for each_edge_key in node.connected_with.iter() {
                let each_edge = self.edges.get(each_edge_key).unwrap();
                warn!("Edge Details{:?}", each_edge);
                let edge_rank=each_edge.rank;
                for (document_id, rank) in each_edge.doc_hash.iter() {
                    let total_rank= node_rank + edge_rank + rank;
                    match results_map.entry(document_id){
                        Entry::Vacant(_) => {
                            let document= Document{
                                id: document_id.to_string(),
                                matched_to: vec![each_edge.clone()],
                                rank: total_rank,
                                normalized_rank: 0.0,
                            };
                            
                            results_map.insert(document_id, document);
                        }
                        Entry::Occupied(entry) => {
                            let document = entry.into_mut();       
                            document.rank += 1;
                            document.matched_to.push(each_edge.clone());
                            document.matched_to.dedup_by_key(|k| k.id.clone());
                        }
                    }

                }
            }

        }
            // warn!("Results Map {:#?}", results_map);
            let mut  hash_vec = results_map.into_iter().collect::<Vec<_>>();
            hash_vec.sort_by(|a, b| b.1.rank.cmp(&a.1.rank));
            hash_vec
  
    }
    pub fn parse_document_to_pair(&mut self, document_id: String,text:&str){
        let matches = find_matches_ids(text, &self.dict_hash).unwrap();
        for (a, b) in matches.into_iter().tuple_windows() {
            self.add_or_update_document(document_id.clone(), a, b);
        }

    }
    pub fn add_or_update_document(&mut self, document_id: String, x: u64, y: u64) {
        let edge = magic_pair(x, y);
        let edge = self.init_or_update_edge(edge, document_id);
        self.init_or_update_node(x, &edge);
        self.init_or_update_node(y, &edge);
    }
    fn init_or_update_node(&mut self, node_id: u64, edge: &Edge) {
        match self.nodes.entry(node_id) {
            Entry::Vacant(_) => {
                let node = Node::new(node_id, edge.clone());
                self.nodes.insert(node.id, node);
                
            }
            Entry::Occupied(entry) => {
                let node = entry.into_mut();
                node.rank += 1;
                node.connected_with.push(edge.id);
            }
        };
    }
    fn init_or_update_edge(&mut self, edge_key: u64, document_id: String) -> Edge {
        let edge = match self.edges.entry(edge_key) {
            Entry::Vacant(_) => {
                let edge = Edge::new(edge_key, document_id);
                self.edges.insert(edge.id, edge.clone());
                edge
            }
            Entry::Occupied(entry) => {
                let edge = entry.into_mut();
                *edge.doc_hash.entry(document_id).or_insert(1) += 1;
                let edge_read = edge.clone();
                edge_read
            }
        };
        edge
    }
}
#[derive(Debug, Clone)]
pub struct Edge {
    // id of the node
    id: u64,
    rank: u64,
    // hashmap document_id, rank
    doc_hash: AHashMap<String, u64>,
}
impl Edge {
    pub fn new(id: u64, document_id: String) -> Self {
        let mut doc_hash = AHashMap::new();
        doc_hash.insert(document_id, 1);
        Self {
            id,
            rank: 1,
            doc_hash: doc_hash,
        }
    }
}
// Node represent single concept
#[derive(Debug, Clone)]
pub struct Node {
    id: u64,
    // number of co-occureneces
    rank: u64,
    connected_with: Vec<u64>,
}
impl Node {
    fn new(id: u64, edge: Edge) -> Self {
        let mut connected_with = Vec::new();
        connected_with.push(edge.id);
        Self {
            id,
            rank: 1,
            connected_with: connected_with,
        }
    }
    // pub fn sort_edges_by_value(&self) {
    //     // let count_b: BTreeMap<&u64, &Edge> =
    //     // self.connected_with.iter().map(|(k, v)| (v, k)).collect();
    //     // for (k, v) in self.connected_with.iter().map(|(k, v)| (v.rank, k)) {
    //     // warn!("k {:?} v {:?}", k, v);
    //     // }
    //     warn!("Connected with {:?}", self.connected_with);
    // }
}

#[macro_use]
extern crate lazy_static;
lazy_static! {
    static ref RE: Regex = Regex::new(r"[?!|]\s+").unwrap();
}
pub fn split_paragraphs(paragraphs: &str) -> Vec<&str> {
    let sentences = UnicodeSegmentation::split_sentence_bounds(paragraphs);
    let parts = sentences.flat_map(|sentence| RE.split(sentence.trim_end_matches(char::is_whitespace)));
    parts.map(|part| part.trim()).filter(|part|!part.is_empty()).collect()
}



/// Combining two numbers into a unique one: pairing functions.
/// It uses "elegant pairing" (https://odino.org/combining-two-numbers-into-a-unique-one-pairing-functions/).
/// also using memoize macro with Ahash hasher
#[memoize(CustomHasher: ahash::AHashMap)]
pub fn magic_pair(x: u64, y: u64) -> u64 {
    if x >= y {
        x * x + x + y
    } else {
        y * y + x
    }
}

// Magic unpair
// func unpair(z int) (int, int) {
//   q := int(math.Floor(math.Sqrt(float64(z))))
//     l := z - q * q

//   if l < q {
//       return l, q
//   }

//   return q, l - q
// }
#[memoize(CustomHasher: ahash::AHashMap)]
pub fn magic_unpair(z: u64) -> (u64, u64) {
    let q = (z as f32).sqrt().floor() as u64;
    let l = z - q * q;
    if l < q {
        return (l, q);
    } else {
        return (q, l - q);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use itertools::Itertools;
    use terraphim_automata::load_automata;
    use terraphim_automata::matcher::{find_matches, find_matches_ids, replace_matches, Dictionary};

    use ulid::Ulid;

    #[test]
    fn test_split_paragraphs() {
        let paragraph = "This is the first sentence.\n\n This is the second sentence. This is the second sentence? This is the second sentence| This is the second sentence!\n\nThis is the third sentence. Mr. John Johnson Jr. was born in the U.S.A but earned his Ph.D. in Israel before joining Nike Inc. as an engineer. He also worked at craigslist.org as a business analyst.";
        let sentences = split_paragraphs(paragraph);
        assert_eq!(sentences.len(), 9);
        assert_eq!(sentences[0], "This is the first sentence.");
        assert_eq!(sentences[1], "This is the second sentence.");
        assert_eq!(sentences[2], "This is the second sentence?");
        assert_eq!(sentences[3], "This is the second sentence");
        assert_eq!(sentences[4], "This is the second sentence!");
        assert_eq!(sentences[5], "This is the third sentence.");
        assert_eq!(sentences[6], "Mr.");
        assert_eq!(sentences[7],"John Johnson Jr. was born in the U.S.A but earned his Ph.D. in Israel before joining Nike Inc. as an engineer.");
        assert_eq!(sentences[8], "He also worked at craigslist.org as a business analyst.");
    }
    
    #[test]
    fn test_find_matches() {
        let query = "I am a text with the word Life cycle concepts and bar and Trained operators and maintainers, project direction, some bingo words Paradigm Map and project planning, then again: some bingo words Paradigm Map and project planning, then repeats: Trained operators and maintainers, project direction";
        let dict_hash = load_automata("https://system-operator.s3.eu-west-2.amazonaws.com/term_to_id.json").unwrap();
        let matches = find_matches(query, dict_hash.clone(), false).unwrap();
        assert_eq!(matches.len(), 7);
    }
    
    #[test]
    fn test_replace_matches() {
        let query = "I am a text with the word Life cycle concepts and bar and Trained operators and maintainers, project direction, some bingo words Paradigm Map and project planning, then again: some bingo words Paradigm Map and project planning, then repeats: Trained operators and maintainers, project direction";
        let dict_hash = load_automata("https://system-operator.s3.eu-west-2.amazonaws.com/term_to_id.json").unwrap();
        let matches = replace_matches(query, dict_hash.clone()).unwrap();
        assert_eq!(matches.len(), 171);
    }
    
    #[test]
    fn test_find_matches_ids() {
        let query = "I am a text with the word Life cycle concepts and bar and Trained operators and maintainers, project direction, some bingo words Paradigm Map and project planning, then again: some bingo words Paradigm Map and project planning, then repeats: Trained operators and maintainers, project direction";
        let dict_hash = load_automata("https://system-operator.s3.eu-west-2.amazonaws.com/term_to_id.json").unwrap();
        let matches = find_matches_ids(query, &dict_hash).unwrap();
        assert_eq!(matches.len(), 7);
    }
    
    #[test]
    fn test_rolegraph() {
        let role = "system operator".to_string();
        let automata_url = "https://system-operator.s3.eu-west-2.amazonaws.com/term_to_id.json";
        let dict_hash = load_automata(automata_url).unwrap();
        let mut rolegraph = RoleGraph::new(role, automata_url);
        let article_id = Ulid::new().to_string();
        let query = "I am a text with the word Life cycle concepts and bar and Trained operators and maintainers, project direction, some bingo words Paradigm Map and project planning, then again: some bingo words Paradigm Map and project planning, then repeats: Trained operators and maintainers, project direction";
        let matches = find_matches_ids(query, &dict_hash).unwrap();
        for (a, b) in matches.into_iter().tuple_windows() {
            rolegraph.add_or_update_document(article_id.clone(), a, b);
        }
        let article_id2= Ulid::new().to_string();
        let query2 = "I am a text with the word Life cycle concepts and bar and maintainers, some bingo words Paradigm Map and project planning, then again: some bingo words Paradigm Map and project planning, then repeats: Trained operators and maintainers, project direction";
        let matches2 = find_matches_ids(query2, &dict_hash).unwrap();
        for (a, b) in matches2.into_iter().tuple_windows() {
            rolegraph.add_or_update_document(article_id2.clone(), a, b);
        }
        let article_id3= Ulid::new().to_string();
        let query3 = "I am a text with the word Life cycle concepts and bar and maintainers, some bingo words Paradigm Map and project planning, then again: some bingo words Paradigm Map and project planning, then repeats: Trained operators and maintainers, project direction";
        let matches3 = find_matches_ids(query3, &dict_hash).unwrap();
        for (a, b) in matches3.into_iter().tuple_windows() {
            rolegraph.add_or_update_document(article_id3.clone(), a, b);
        }
        let article_id4= "ArticleID4".to_string();
        let query4 = "I am a text with the word Life cycle concepts and bar and maintainers, some bingo words, then again: some bingo words Paradigm Map and project planning, then repeats: Trained operators and maintainers, project direction";
        rolegraph.parse_document_to_pair(article_id4,query4);
        warn!("Query graph");
        let results_map= rolegraph.query("Life cycle concepts and project direction");
        assert_eq!(results_map.len(),4);
    }
}