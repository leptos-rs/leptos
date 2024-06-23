#[cfg(feature = "ssr")]
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};

#[cfg(feature = "ssr")]
pub fn story(path: &str) -> String {
    format!("https://node-hnapi.herokuapp.com/{path}")
}

#[cfg(feature = "ssr")]
pub fn user(path: &str) -> String {
    format!("https://hacker-news.firebaseio.com/v0/user/{path}.json")
}

#[cfg(feature = "ssr")]
pub async fn fetch_api<T>(path: &str) -> Option<T>
where
    T: Serialize + DeserializeOwned,
{
    use leptos::logging;
    reqwest::get(path)
        .await
        .map_err(|e| logging::error!("{e}"))
        .ok()?
        .json()
        .await
        .ok()
}

#[derive(Debug, Deserialize, Serialize, PartialEq, Eq, Clone)]
pub struct Story {
    pub id: usize,
    pub title: String,
    pub points: Option<i32>,
    pub user: Option<String>,
    pub time: usize,
    pub time_ago: String,
    #[serde(alias = "type")]
    pub story_type: String,
    pub url: String,
    #[serde(default)]
    pub domain: String,
    #[serde(default)]
    pub comments: Option<Vec<Comment>>,
    pub comments_count: Option<usize>,
}

#[derive(Debug, Deserialize, Serialize, PartialEq, Eq, Clone)]
pub struct Comment {
    pub id: usize,
    pub level: usize,
    pub user: Option<String>,
    pub time: usize,
    pub time_ago: String,
    pub content: Option<String>,
    pub comments: Vec<Comment>,
}

#[derive(Debug, Deserialize, Serialize, PartialEq, Eq, Clone)]
pub struct User {
    pub created: usize,
    pub id: String,
    pub karma: i32,
    pub about: Option<String>,
}
