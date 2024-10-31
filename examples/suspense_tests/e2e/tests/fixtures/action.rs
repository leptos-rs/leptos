use super::{find, world::HOST};
use anyhow::Result;
use fantoccini::{Client, Locator};
use std::result::Result::Ok;

pub async fn goto_path(client: &Client, path: &str) -> Result<()> {
    let url = format!("{}{}", HOST, path);
    client.goto(&url).await?;

    Ok(())
}

pub async fn click_link(client: &Client, text: &str) -> Result<()> {
    let link = client
        .wait()
        .for_element(Locator::LinkText(text))
        .await
        .expect(format!("Link not found by `{}`", text).as_str());

    link.click().await?;

    Ok(())
}

pub async fn click_first_button(client: &Client) -> Result<()> {
    let counter_button = find::first_button(client).await?;

    counter_button.click().await?;

    Ok(())
}

pub async fn click_second_button(client: &Client) -> Result<()> {
    let counter_button = find::second_button(client).await?;

    counter_button.click().await?;

    Ok(())
}

pub async fn click_reset_counters_button(client: &Client) -> Result<()> {
    let reset_counter = find::reset_counter(client).await?;

    reset_counter.click().await?;

    Ok(())
}

pub async fn click_reset_csr_counters_button(client: &Client) -> Result<()> {
    let reset_counter = find::reset_csr_counter(client).await?;

    reset_counter.click().await?;

    Ok(())
}
