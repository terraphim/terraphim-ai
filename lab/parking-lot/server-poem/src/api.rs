use poem::web::Data;
use poem_openapi::{param::Query, payload::PlainText, OpenApi};
use poem_openapi::{payload::Json, ApiResponse};
use serde_json::value::Index;
use ulid::Ulid;

use crate::types::{ApiTags, Article, SearchQuery};
use anyhow::{Context, Result};
use poem_openapi::{types::ToJSON, types::Type, NewType, Object};
use terraphim_types::{Article, IndexedArticle, RoleGraph};
use tokio::sync::Mutex;

// This would be the correct approach, however
// IndexArticle is not `Type` and so we can't implement `NewType` for `ApiIndexedArticle`
// That is because `IndexedArticle` is not `Send` and `Sync`.
// We tried `Arc<Mutex<IndexedArticle>>` but that didn't work either because
// `Mutex` is not `Type` either and we cannot implement that because of orphan rules
// Therefore we manually serialize it with to_string()

// use std::sync::Arc;

// #[derive(Debug, Clone)]
// pub struct ApiIndexedArticle(pub IndexedArticle);

// impl ToJSON for ApiIndexedArticle {
//     /// Convert this value to [`Value`].
//     fn to_json(&self) -> Option<Value>;

//     /// Convert this value to JSON string.
//     fn to_json_string(&self) -> String {
//         serde_json::to_string(&self.to_json()).unwrap_or_default()
//     }
// }

#[derive(ApiResponse, Debug, PartialEq, Eq)]
pub enum QueryResponse {
    /// Return the found articles.
    #[oai(status = 200)]
    Ok(PlainText<String>),
    /// Return when the specified query didn't match any articles.
    #[oai(status = 404)]
    NotFound,
}

#[derive(ApiResponse)]
enum CreateArticleResponse {
    /// Returns when the article is successfully created.
    #[oai(status = 200)]
    Ok(Json<String>),
}

pub(crate) struct Api {
    /// RoleGraph for ingesting articles
    pub(crate) rolegraph: Mutex<RoleGraph>,
}

#[OpenApi]
impl Api {
    #[oai(path = "/hello", method = "get")]
    async fn index(&self, name: Query<Option<String>>) -> PlainText<String> {
        match name.0 {
            Some(name) => PlainText(format!("hello, {name}!")),
            None => PlainText("hello!".to_string()),
        }
    }

    #[oai(path = "/articles", method = "post", tag = "ApiTags::Article")]
    async fn create_article(&self, article: Json<Article>) -> CreateArticleResponse {
        log::warn!("create_article");

        log::warn!("create article");
        let id = Ulid::new().to_string();
        let article: Article = article.0;

        let mut rolegraph = self.rolegraph.lock().await;
        rolegraph.parse_article(id.clone(), article);

        log::warn!("send response");
        CreateArticleResponse::Ok(Json(id))
    }

    #[oai(path = "/search", method = "post", tag = "ApiTags::Search")]
    async fn graph_search(&self, search_query: Json<SearchQuery>) -> QueryResponse {
        let rolegraph = self.rolegraph.lock().await;
        let articles: Vec<(&String, IndexedArticle)> =
            match rolegraph.query(&search_query.search_term) {
                Ok(docs) => docs,
                Err(e) => {
                    log::error!("Error: {}", e);
                    return QueryResponse::NotFound;
                }
            };

        match articles.len() {
            0 => QueryResponse::NotFound,
            _ => {
                let docs: Vec<String> = match articles
                    .into_iter()
                    .map(|(_id, doc)| doc.to_json_string())
                    .collect()
                {
                    Ok(docs) => docs,
                    Err(e) => {
                        log::error!("Error converting an individual article into JSON: {}", e);
                        return QueryResponse::NotFound;
                    }
                };
                // let docs: Vec<String> = articles.into_iter().map(|(_id, doc) | doc.to_string()).collect();
                // let docs: Vec<IndexedArticle> = articles.into_iter().map(|(_id, doc) | doc).collect();
                let json: String = match serde_json::to_string(&docs) {
                    Ok(json) => json,
                    Err(e) => {
                        log::error!("Error converting vector of articles to JSON: {}", e);
                        return QueryResponse::NotFound;
                    }
                };
                QueryResponse::Ok(PlainText(json))
            }
        }
    }

    #[oai(path = "/config", method = "get")]
    async fn config(&self, name: Query<Option<String>>) -> PlainText<String> {
        match name.0 {
            Some(name) => PlainText(format!("hello, {name}!")),
            None => PlainText("hello!".to_string()),
        }
    }
}

// fn serialize_into_json(articles: Vec<(&String, IndexedArticle)>) -> QueryResponse {
//     let docs: Result<Vec<String>, _> = articles.into_iter().map(|(_id, doc) | doc.to_json_string()).collect();
//     QueryResponse::Ok(Json(docs.unwrap()))
// }

// #[cfg(test)]
// mod test {
//     use super::*;
//     use terraphim_rolegraph::{Article, IndexedArticle, RoleGraph};

//     #[test]
//     fn test_serialization() {
//         let articles: Vec<(&String, IndexedArticle)>  = vec![];
//         let response = serialize_into_json(articles);
//         // assert_eq!(response, QueryResponse::Ok(Json(vec![r#"{"title":"test","body":"test","tags":["test"]}"#])));
//         assert_eq!(response, QueryResponse::Ok(Json(vec![])));
//     }

//     #[test]
//     fn test_serialization_complex() {
//         let role = "system operator".to_string();
//         // let automata_url = "https://system-operator.s3.eu-west-2.amazonaws.com/term_to_id.json";
//         let automata_url = "./data/term_to_id.json";
//         let mut rolegraph = RoleGraph::new(role, automata_url);

//         let doc = Article {
//             id: "1".to_string(),
//             title: "Life cycle concepts".to_string(),
//             body: Some("I am a text with the word Life cycle concepts and bar and Trained operators and maintainers, some bingo words Paradigm Map and project planning, then again: some bingo words Paradigm Map and project planning, then repeats: Trained operators and maintainers, project direction".to_string()),
//             description: None,
//         };

//         rolegraph.parse_article(
//             "asdf".to_string(),
//             doc
//         );

//         let indexed_docs = rolegraph.query("Life cycle concepts");
//         let response = serialize_into_json(indexed_docs);
// curl -X 'POST' \
//   'http://localhost:3000/api/search' \
//   -H 'accept: application/json; charset=utf-8' \
//   -H 'Content-Type: application/json; charset=utf-8' \
//   -d '{
//   "search_term": "trained operators and maintainers",
//   "skip": 0,
//   "limit": 0,
//   "role": "string"
// }'
// ["{\"id\":\"01HFE81F635G4XYZWEKRKC6EXE\",\"matched_to\":[{\"id\":1788657,\"rank\":1,\"doc_hash\":{\"01HFE81F635G4XYZWEKRKC6EXE\":2,\"01HFD8P9V4A1ZZFJ486Z2WE5TY\":1}},{\"id\":1790137,\"rank\":1,\"doc_hash\":{\"01HFD8P9V4A1ZZFJ486Z2WE5TY\":2,\"01HFE81F635G4XYZWEKRKC6EXE\":3}},{\"id\":1788810,\"rank\":1,\"doc_hash\":{\"01HFE81F635G4XYZWEKRKC6EXE\":2,\"01HFD8P9V4A1ZZFJ486Z2WE5TY\":1}},{\"id\":1790137,\"rank\":1,\"doc_hash\":{\"01HFD8P9V4A1ZZFJ486Z2WE5TY\":2,\"01HFE81F635G4XYZWEKRKC6EXE\":3}},{\"id\":1788657,\"rank\":1,\"doc_hash\":{\"01HFE81F635G4XYZWEKRKC6EXE\":2,\"01HFD8P9V4A1ZZFJ486Z2WE5TY\":1}},{\"id\":1790137,\"rank\":1,\"doc_hash\":{\"01HFD8P9V4A1ZZFJ486Z2WE5TY\":2,\"01HFE81F635G4XYZWEKRKC6EXE\":3}},{\"id\":1788810,\"rank\":1,\"doc_hash\":{\"01HFE81F635G4XYZWEKRKC6EXE\":2,\"01HFD8P9V4A1ZZFJ486Z2WE5TY\":1}},{\"id\":1790137,\"rank\":1,\"doc_hash\":{\"01HFD8P9V4A1ZZFJ486Z2WE5TY\":2,\"01HFE81F635G4XYZWEKRKC6EXE\":3}}],\"rank\":18,\"normalized_rank\":0.0}","{\"id\":\"01HFD8P9V4A1ZZFJ486Z2WE5TY\",\"matched_to\":[{\"id\":1788657,\"rank\":1,\"doc_hash\":{\"01HFE81F635G4XYZWEKRKC6EXE\":2,\"01HFD8P9V4A1ZZFJ486Z2WE5TY\":1}},{\"id\":1790137,\"rank\":1,\"doc_hash\":{\"01HFD8P9V4A1ZZFJ486Z2WE5TY\":2,\"01HFE81F635G4XYZWEKRKC6EXE\":3}},{\"id\":1788810,\"rank\":1,\"doc_hash\":{\"01HFE81F635G4XYZWEKRKC6EXE\":2,\"01HFD8P9V4A1ZZFJ486Z2WE5TY\":1}},{\"id\":1790137,\"rank\":1,\"doc_hash\":{\"01HFD8P9V4A1ZZFJ486Z2WE5TY\":2,\"01HFE81F635G4XYZWEKRKC6EXE\":3}},{\"id\":1788657,\"rank\":1,\"doc_hash\":{\"01HFE81F635G4XYZWEKRKC6EXE\":2,\"01HFD8

// let json = r#"{"id":"asdf","matched_to":[{"id":1185920,"rank":1,"doc_hash":{"asdf":1}},{"id":1788657,"rank":1,"doc_hash":{"asdf":1}}],"rank":7,"normalized_rank":0.0}"#;

//         let indexed_docs = indexed_docs.into_iter().map(|(_id, doc) | doc.to_json_string()).collect();
//     let expected = QueryResponse::Ok(Json(indexed_docs));
//         println!("{:?}", response);
//         assert_eq!(response, expected);

//     }
// }
