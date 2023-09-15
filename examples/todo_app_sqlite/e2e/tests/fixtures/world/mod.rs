pub mod action_steps;
pub mod check_steps;

use anyhow::Result;
use cucumber::World;
use fantoccini::{
    error::NewSessionError, wd::Capabilities, Client, ClientBuilder,
};

pub const HOST: &str = "http://127.0.0.1:3000";

#[derive(Debug, World)]
#[world(init = Self::new)]
pub struct AppWorld {
    pub client: Client,
}

impl AppWorld {
    async fn new() -> Result<Self, anyhow::Error> {
        let webdriver_client = build_client().await?;

        Ok(Self {
            client: webdriver_client,
        })
    }
}

async fn build_client() -> Result<Client, NewSessionError> {
    let mut cap = Capabilities::new();
    let arg = serde_json::from_str("{\"args\": [\"-headless\"]}").unwrap();
    cap.insert("goog:chromeOptions".to_string(), arg);

    let client = ClientBuilder::native()
        .capabilities(cap)
        .connect("http://localhost:4444")
        .await?;

    Ok(client)
}
