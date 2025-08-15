use crate::fixtures::{check, world::AppWorld};
use anyhow::{Ok, Result};
use cucumber::then;

#[then(regex = r"^I see the result is empty$")]
async fn i_see_the_result_is_empty(world: &mut AppWorld) -> Result<()> {
    let client = &world.client;
    check::result_text_is(client, "").await?;
    Ok(())
}

#[then(regex = r"^I see the result is the string (.*)$")]
async fn i_see_the_result_is_the_string(
    world: &mut AppWorld,
    text: String,
) -> Result<()> {
    let client = &world.client;
    check::result_text_is(client, &text).await?;
    Ok(())
}

#[then(regex = r"^I see the navbar$")]
async fn i_see_the_navbar(world: &mut AppWorld) -> Result<()> {
    let client = &world.client;
    check::element_exists(client, "nav").await?;
    Ok(())
}
