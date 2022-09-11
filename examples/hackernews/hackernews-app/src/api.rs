use anyhow::Result;
use serde::{de::DeserializeOwned, Deserialize, Serialize};

pub fn story(path: &str) -> String {
    format!("https://node-hnapi.herokuapp.com/{path}")
}

pub fn user(path: &str) -> String {
    format!("https://hacker-news.firebaseio.com/v0/user/{path}.json")
}

#[cfg(not(feature = "ssr"))]
pub async fn fetch_api<T>(path: &str) -> Result<T, ()>
where
    T: DeserializeOwned,
{
    gloo_net::http::Request::get(path)
        .send()
        .await
        .map_err(|e| log::error!("{e}"))?
        .json::<T>()
        .await
        .map_err(|e| log::error!("{e}"))
}

#[cfg(feature = "ssr")]
pub async fn fetch_api<T>(path: &str) -> Result<T, ()>
where
    T: DeserializeOwned,
{
    reqwest::get(path)
        .await
        .map_err(|e| log::error!("{e}"))?
        .json::<T>()
        .await
        .map_err(|e| log::error!("{e}"))
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
    pub comments: Option<Vec<Comment>>,
    pub comments_count: Option<usize>,
}

#[derive(Debug, Deserialize, Serialize, PartialEq, Eq, Clone)]
pub struct Comment {
    pub id: usize,
    pub level: usize,
    pub user: String,
    pub time: usize,
    pub time_ago: String,
    pub content: String,
    pub comments: Vec<Comment>,
}

#[derive(Debug, Deserialize, Serialize, PartialEq, Eq, Clone)]
pub struct User {
    pub created: usize,
    pub id: String,
    pub karma: i32,
    pub about: Option<String>,
}
