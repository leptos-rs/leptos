use super::world::{AppWorld, HOST};
use anyhow::Result;
use fantoccini::Locator;
use std::result::Result::Ok;
use tokio::{self, time};

pub async fn goto_path(world: &mut AppWorld, path: &str) -> Result<()> {
    let url = format!("{}{}", HOST, path);
    let client = &world.client;

    client.goto(&url).await?;
    let _: () = client.wait().for_url(url::Url::parse(&url)?).await?;

    Ok(())
}

pub async fn fill_todo(world: &mut AppWorld, text: &str) -> Result<()> {
    let client = &world.client;
    let form = client.form(Locator::Css("div form")).await?;
    form.set_by_name("title", text).await?;

    Ok(())
}

pub async fn click_add_button(world: &mut AppWorld) -> Result<()> {
    let client = &world.client;
    let element = client
        .wait()
        .for_element(Locator::Css("input[value='Add']"))
        .await?;

    element.click().await?;

    Ok(())
}

pub async fn empty_todo_list(world: &mut AppWorld) -> Result<()> {
    let client = &world.client;
    let todos = client.find_all(Locator::Css("li")).await?;

    for _todo in todos {
        let _ = delete_last_todo(world).await?;
    }

    Ok(())
}

pub async fn delete_last_todo(world: &mut AppWorld) -> Result<()> {
    let client = &world.client;
    let element = client
        .wait()
        .for_element(Locator::Css("li:last-child input[value='X']"))
        .await
        .expect("Last todo not found");

    element.click().await.expect("Failed to delete todo");
    time::sleep(time::Duration::from_millis(250)).await;

    Ok(())
}
