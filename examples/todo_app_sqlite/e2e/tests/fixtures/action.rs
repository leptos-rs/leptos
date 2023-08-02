use super::{
    find,
    world::{AppWorld, HOST},
};
use anyhow::Result;
use std::result::Result::Ok;
use tokio::{self, time};

pub async fn goto_path(world: &mut AppWorld, path: &str) -> Result<()> {
    let url = format!("{}{}", HOST, path);
    let client = &world.client;

    client.goto(&url).await?;
    let _: () = client.wait().for_url(url::Url::parse(&url)?).await?;

    Ok(())
}

pub async fn add_todo(world: &mut AppWorld, text: &str) -> Result<()> {
    fill_todo(world, text).await?;
    click_add_button(world).await?;
    Ok(())
}

pub async fn fill_todo(world: &mut AppWorld, text: &str) -> Result<()> {
    let textbox = find::todo_input(world).await;
    textbox.send_keys(text).await?;

    Ok(())
}

pub async fn click_add_button(world: &mut AppWorld) -> Result<()> {
    let add_button = find::add_button(world).await;
    add_button.click().await?;

    Ok(())
}

pub async fn empty_todo_list(world: &mut AppWorld) -> Result<()> {
    let todos = find::todos(world).await;

    for _todo in todos {
        let _ = delete_last_todo(world).await?;
    }

    Ok(())
}

pub async fn delete_last_todo(world: &mut AppWorld) -> Result<()> {
    if let Some(element) = find::last_delete_button(world).await {
        element.click().await.expect("Failed to delete todo");
        time::sleep(time::Duration::from_millis(500)).await;
    }

    Ok(())
}

pub async fn delete_todo(world: &mut AppWorld, text: &str) -> Result<()> {
    if let Some(element) = find::delete_button(world, text).await {
        element.click().await?;
        time::sleep(time::Duration::from_millis(500)).await;
    }

    Ok(())
}
