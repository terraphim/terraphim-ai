use tokio::io::AsyncBufRead;
use std::process::{ExitStatus, Stdio};
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader as TokioBufferedReader};

use tokio::process::{Child, Command};
use tokio::time::Duration;
use tokio_stream::wrappers::UnboundedReceiverStream;
use tokio::sync::{Mutex, mpsc};
use tokio_stream::StreamExt;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::collections::HashSet;
use std::fs::{self, File};

use terraphim_types::{Article,  ConfigState, SearchQuery};
use terraphim_pipeline::{IndexedDocument}; 

mod lib;
use lib::{Message,json_decode, Data};

fn calculate_hash<T: Hash>(t: &T) -> String {
    let mut s = DefaultHasher::new();
    t.hash(&mut s);
    format!("{:x}", s.finish())
}

pub async fn run_ripgrep_and_broadcast(needle: String, haystack: String) {

    let mut child = Command::new("/home/alex/.cargo/bin/rg").arg(&needle).arg(&haystack).arg("--json").arg("--trim").arg("-C3").arg("-i")
    .stdout(Stdio::piped())
    .spawn()
    .unwrap();
    let mut stdout = child.stdout.take().expect("Stdout is not available"); 
    let read = async move { 
        let mut data = String::new(); 
        stdout.read_to_string(&mut data).await.map(|_| data) 
    }; 
    let output = read.await;
    let msgs = json_decode(&output.unwrap());
    // // println!("stdout: {:#?}", msgs);

    
    let mut config_state= ConfigState::new().await.expect("Failed to load config state");
    
    let mut article = Article::default();

    let mut existing_paths: HashSet<String> = HashSet::new();

    for each_msg in msgs.iter() {
        match each_msg {
            Message::Begin(begin_msg) => {
                println!("stdout: {:#?}", each_msg);
                article = Article::default();

                // get path
                let path: Option<Data> = begin_msg.path.clone();
                let path_text = match path {
                    Some(Data::Text{text}) => text,
                    _ => {
                        println!("Error: path is not text");
                        continue
                    }
                };

                if existing_paths.contains(&path_text) {
                    continue
                }
                existing_paths.insert(path_text.clone());

                let id = calculate_hash(&path_text);
                article.id = Some(id.clone());
                article.title = path_text.clone();
                article.url = path_text;

            },
            Message::Match(match_msg) => {
                println!("stdout: {:#?}", article);
                let path = match_msg.path.clone();
                let path = path.unwrap();
                let path_text = match path {
                    Data::Text{text} => text,
                    _ => {
                        println!("Error: path is not text");
                        continue
                    }
                };
                let body = fs::read_to_string(path_text).unwrap();
                article.body = body;

                let lines = match &match_msg.lines {
                    Data::Text{text} => text,
                    _ => {
                        println!("Error: lines is not text");
                        continue
                    }
                };
                match article.description {
                    Some(description) => {
                        article.description = Some(description + " " + &lines);
                    },
                    None => {
                        article.description = Some(lines.clone());
                    }
                }
            },
            Message::Context(context_msg) => {
                // let article = Article::new(context_msg.clone());
                println!("stdout: {:#?}", article);

                let article_url = article.url.clone();
                let path = context_msg.path.clone();
                let path = path.unwrap();
                let path_text = match path {
                    Data::Text{text} => text,
                    _ => {
                        println!("Error: path is not text");
                        continue
                    }
                };

                // We got a context for a different article
                if article_url != path_text {
                    continue
                }

                let lines = match &context_msg.lines {
                    Data::Text{text} => text,
                    _ => {
                        println!("Error: lines is not text");
                        continue
                    }
                };
                match article.description {
                    Some(description) => {
                        article.description = Some(description + " " + &lines);
                    },
                    None => {
                        article.description = Some(lines.clone());
                    }
                }
            },
            Message::End(end_msg) => {
                println!("stdout: {:#?}", each_msg);
                // The `End` message could be received before the `Begin` message
                // causing the article to be empty
                let id = match article.id {
                    Some(ref id) => id,
                    None => {
                        continue
                    }
                };
                config_state.index_article(article.clone()).await.expect("Failed to index article");
            },
            _ => {
            }
        }
        
    }

    
    let role_name = "System Operator".to_string();
    println!("{:#?}", role_name);
    println!("Searching articles with query: {needle} {role_name}");
    let search_query = SearchQuery {
        search_term: needle,
        role: Some(role_name),
        skip: Some(0),
        limit: Some(10),
    };
    
    let docs= config_state.search_articles(search_query).await.expect("Failed to search articles");
    
    // let docs: Vec<IndexedDocument> = documents.into_iter().map(|(_id, doc) | doc).collect();
    println!("Found articles: {docs:?}");
    // send the results to the stream as well (only for testing)
    for doc in docs.iter() {
        println!("Found articles: {:#?}", doc);
    }
 
}

#[tokio::main]    
async fn main() -> Result<(), Box<dyn std::error::Error>>{
    let needle = "trained operators and maintainers".to_string();
    let haystack = "../../../INCOSE-Systems-Engineering-Handbook".to_string();
    run_ripgrep_and_broadcast(needle, haystack).await;
    Ok(())
}
