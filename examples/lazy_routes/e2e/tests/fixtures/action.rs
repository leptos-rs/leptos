use super::{find, world::HOST};
use anyhow::Result;
use fantoccini::Client;
use std::result::Result::Ok;

pub async fn goto_path(client: &Client, path: &str) -> Result<()> {
    let url = format!("{}{}", HOST, path);
    client.goto(&url).await?;

    Ok(())
}

pub async fn click_link(client: &Client, text: &str) -> Result<()> {
    let link = find::link_with_text(&client, &text).await?;
    link.click().await?;
    Ok(())
}

pub async fn click_button(client: &Client, id: &str) -> Result<()> {
    let btn = find::element_by_id(&client, &id).await?;
    btn.click().await?;
    Ok(())
}
