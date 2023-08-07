use super::{find, world::HOST};
use anyhow::Result;
use fantoccini::Client;
use std::result::Result::Ok;
use tokio::{self, time};

pub async fn goto_path(client: &Client, path: &str) -> Result<()> {
    let url = format!("{}{}", HOST, path);
    client.goto(&url).await?;

    Ok(())
}

pub async fn add_todo(client: &Client, text: &str) -> Result<()> {
    fill_todo(client, text).await?;
    click_add_button(client).await?;
    Ok(())
}

pub async fn fill_todo(client: &Client, text: &str) -> Result<()> {
    let textbox = find::todo_input(client).await;
    textbox.send_keys(text).await?;

    Ok(())
}

pub async fn click_add_button(client: &Client) -> Result<()> {
    let add_button = find::add_button(client).await;
    add_button.click().await?;

    Ok(())
}

pub async fn empty_todo_list(client: &Client) -> Result<()> {
    let todos = find::todos(client).await;

    for _todo in todos {
        let _ = delete_first_todo(client).await?;
    }

    Ok(())
}

pub async fn delete_first_todo(client: &Client) -> Result<()> {
    if let Some(element) = find::first_delete_button(client).await {
        element.click().await.expect("Failed to delete todo");
        time::sleep(time::Duration::from_millis(250)).await;
    }

    Ok(())
}

pub async fn delete_todo(client: &Client, text: &str) -> Result<()> {
    if let Some(element) = find::delete_button(client, text).await {
        element.click().await?;
        time::sleep(time::Duration::from_millis(250)).await;
    }

    Ok(())
}
