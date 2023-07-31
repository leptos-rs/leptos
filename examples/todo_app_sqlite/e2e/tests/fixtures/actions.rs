use super::world::{AppWorld, HOST};
use anyhow::{Ok, Result};
use fantoccini::Locator;

pub async fn goto_path(world: &mut AppWorld, path: &str) -> Result<()> {
    let url = format!("{}{}", HOST, path);
    let client = &world.client;

    client.goto(&url).await?;
    let _: () = client.wait().for_url(url::Url::parse(&url)?).await?;

    Ok(())
}

pub async fn fill_todo(world: &mut AppWorld, text: &str) -> Result<()> {
    let client = &world.client;
    let form = client.form(Locator::Css("form")).await?;
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
