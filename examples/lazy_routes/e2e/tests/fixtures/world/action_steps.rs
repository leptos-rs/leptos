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

#[when(regex = "^I open the app at (.*)$")]
async fn i_open_the_app_at(world: &mut AppWorld, url: String) -> Result<()> {
    let client = &world.client;
    action::goto_path(client, &url).await?;

    Ok(())
}

#[when(regex = "^I select the link (.*)$")]
async fn i_select_the_link(world: &mut AppWorld, text: String) -> Result<()> {
    let client = &world.client;
    action::click_link(client, &text).await?;

    Ok(())
}

#[when(regex = "^I click the button (.*)$")]
async fn i_click_the_button(world: &mut AppWorld, id: String) -> Result<()> {
    let client = &world.client;
    action::click_button(client, &id).await?;

    Ok(())
}

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

#[when("I wait for a second")]
async fn i_wait_for_a_second(world: &mut AppWorld) -> Result<()> {
    tokio::time::sleep(std::time::Duration::from_secs(1)).await;

    Ok(())
}

#[given(regex = "^I (refresh|reload) the (browser|page)$")]
#[when(regex = "^I (refresh|reload) the (browser|page)$")]
async fn i_refresh_the_browser(world: &mut AppWorld) -> Result<()> {
    let client = &world.client;
    client.refresh().await?;

    Ok(())
}
