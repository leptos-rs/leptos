use crate::fixtures::{check, world::AppWorld};
use anyhow::{Ok, Result};
use cucumber::then;

#[then(regex = r"^I see the navigating indicator")]
async fn i_see_the_nav(world: &mut AppWorld) -> Result<()> {
    let client = &world.client;
    check::navigating_appears(client).await?;
    Ok(())
}

#[then(regex = r"^I see the page is (.*)$")]
async fn i_see_the_page_is(world: &mut AppWorld, text: String) -> Result<()> {
    let client = &world.client;
    check::page_name_is(client, &text).await?;
    Ok(())
}

#[then(regex = r"^I see the result is (.*)$")]
async fn i_see_the_result_is(world: &mut AppWorld, text: String) -> Result<()> {
    let client = &world.client;
    check::result_is(client, &text).await?;
    Ok(())
}

#[then(regex = r"^I see the navbar$")]
async fn i_see_the_navbar(world: &mut AppWorld) -> Result<()> {
    let client = &world.client;
    check::element_exists(client, "nav").await?;
    Ok(())
}
