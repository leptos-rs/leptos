use crate::fixtures::{action, world::AppWorld};
use anyhow::{Ok, Result};
use cucumber::{given, when};

#[given("I see the app")]
#[when("I open the app")]
async fn i_open_the_app(world: &mut AppWorld) -> Result<()> {
    let client = &world.client;
    action::goto_path(client, "").await?;

    Ok(())
}

#[given(regex = "^I add a text as (.*)$")]
async fn i_add_a_text(world: &mut AppWorld, text: String) -> Result<()> {
    let client = &world.client;
    action::fill_input(client, text.as_str()).await?;

    Ok(())
}
