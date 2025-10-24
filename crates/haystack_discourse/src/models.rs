use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize)]
pub struct Post {
    pub id: u64,
    pub title: String,
    pub url: String,
    pub excerpt: String,
    pub body: Option<String>,
    pub created_at: String,
    pub username: String,
}

#[derive(Debug, Deserialize)]
pub struct SearchResponse {
    pub posts: Vec<PostResponse>,
    pub topics: Vec<TopicResponse>,
}

#[derive(Debug, Deserialize)]
pub struct PostResponse {
    pub id: u64,
    pub blurb: String,
    pub topic_id: u64,
    pub post_number: u64,
    pub username: String,
    pub created_at: String,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct TopicResponse {
    pub id: u64,
    pub title: String,
    pub slug: String,
    pub posts_count: u64,
    pub fancy_title: String,
}

#[derive(Debug, Deserialize)]
pub struct PostDetailsResponse {
    pub post: PostDetails,
}

#[derive(Debug, Deserialize)]
pub struct PostDetails {
    pub cooked: String, // HTML content of the post
}
