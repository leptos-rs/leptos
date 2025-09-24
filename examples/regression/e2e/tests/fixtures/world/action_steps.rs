use crate::fixtures::{action, world::AppWorld};
use anyhow::{Ok, Result};
use cucumber::{gherkin::Step, given, when};

#[given("I see the app")]
#[when("I open the app")]
async fn i_open_the_app(world: &mut AppWorld) -> Result<()> {
    let client = &world.client;
    action::goto_path(client, "").await?;

    Ok(())
}

#[given(regex = "^I can access regression test (.*)$")]
#[when(regex = "^I select the link (.*)$")]
async fn i_select_the_link(world: &mut AppWorld, text: String) -> Result<()> {
    let client = &world.client;
    action::click_link(client, &text).await?;

    Ok(())
}

#[given(expr = "I select the following links")]
#[when(expr = "I select the following links")]
async fn i_select_the_following_links(
    world: &mut AppWorld,
    step: &Step,
) -> Result<()> {
    let client = &world.client;

    if let Some(table) = step.table.as_ref() {
        for row in table.rows.iter() {
            action::click_link(client, &row[0]).await?;
        }
    }

    Ok(())
}

#[given(regex = "^I (refresh|reload) the (browser|page)$")]
#[when(regex = "^I (refresh|reload) the (browser|page)$")]
async fn i_refresh_the_browser(world: &mut AppWorld) -> Result<()> {
    let client = &world.client;
    client.refresh().await?;

    Ok(())
}

#[given(regex = "^I press the back button$")]
#[when(regex = "^I press the back button$")]
async fn i_go_back(world: &mut AppWorld) -> Result<()> {
    let client = &world.client;
    client.back().await?;

    Ok(())
}
