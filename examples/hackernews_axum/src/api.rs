use leptos::Serializable;
use serde::{Deserialize, Serialize};

pub fn story(path: &str) -> String {
    format!("https://node-hnapi.herokuapp.com/{path}")
}

pub fn user(path: &str) -> String {
    format!("https://hacker-news.firebaseio.com/v0/user/{path}.json")
}

#[cfg(not(feature = "ssr"))]
pub async fn fetch_api<T>(path: &str) -> Option<T>
where
    T: Serializable,
{
    let abort_controller = web_sys::AbortController::new().ok();
    let abort_signal = abort_controller.as_ref().map(|a| a.signal());

    // abort in-flight requests if e.g., we've navigated away from this page
    leptos::on_cleanup(move || {
        if let Some(abort_controller) = abort_controller {
            abort_controller.abort()
        }
    });

    let json = gloo_net::http::Request::get(path)
        .abort_signal(abort_signal.as_ref())
        .send()
        .await
        .map_err(|e| log::error!("{e}"))
        .ok()?
        .text()
        .await
        .ok()?;

    T::de(&json).ok()
}

#[cfg(feature = "ssr")]
pub async fn fetch_api<T>(path: &str) -> Option<T>
where
    T: Serializable,
{
    let json = reqwest::get(path)
        .await
        .map_err(|e| log::error!("{e}"))
        .ok()?
        .text()
        .await
        .ok()?;
    T::de(&json).map_err(|e| log::error!("{e}")).ok()
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
