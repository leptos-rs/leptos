use crate::fixtures::{check, world::AppWorld};
use anyhow::{Ok, Result};
use cucumber::then;
use std::time::Duration;
use tokio::time::sleep;

#[then(regex = "^I see the page title is (.*)$")]
async fn i_see_the_page_title_is(
    world: &mut AppWorld,
    text: String,
) -> Result<()> {
    let client = &world.client;
    check::text_on_element(client, "h1", &text).await?;

    Ok(())
}

#[then(regex = "^I see the label of the input is (.*)$")]
async fn i_see_the_label_of_the_input_is(
    world: &mut AppWorld,
    text: String,
) -> Result<()> {
    sleep(Duration::from_millis(500)).await;
    let client = &world.client;
    check::text_on_element(client, "p", &text).await?;

    Ok(())
}
