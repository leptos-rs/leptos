use super::{find, world::HOST};
use anyhow::Result;
use fantoccini::Client;
use std::result::Result::Ok;

pub async fn goto_path(client: &Client, path: &str) -> Result<()> {
    let url = format!("{}{}", HOST, path);
    client.goto(&url).await?;

    Ok(())
}

pub async fn fill_input(client: &Client, text: &str) -> Result<()> {
    let textbox = find::input(client).await;
    textbox.send_keys(text).await?;

    Ok(())
}
