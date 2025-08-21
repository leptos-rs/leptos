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

#[then(regex = r"^I see ([\d\w]+) is selected$")]
async fn i_see_the_select(world: &mut AppWorld, id: String) -> Result<()> {
    let client = &world.client;
    check::select_option_is_selected(client, &id).await?;
    Ok(())
}

#[then(regex = r"^I see the value of (\w+) is (.*)$")]
async fn i_see_the_value(
    world: &mut AppWorld,
    id: String,
    value: String,
) -> Result<()> {
    let client = &world.client;
    check::element_value_is(client, &id, &value).await?;
    Ok(())
}
