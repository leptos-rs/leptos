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

#[given(regex = r"^I select the mode (.*)$")]
async fn i_select_the_mode(world: &mut AppWorld, text: String) -> Result<()> {
    let client = &world.client;
    action::click_link(client, &text).await?;

    Ok(())
}

#[given(regex = r"^I select the component (.*)$")]
#[when(regex = "^I select the component (.*)$")]
async fn i_select_the_component(
    world: &mut AppWorld,
    text: String,
) -> Result<()> {
    let client = &world.client;
    action::click_link(client, &text).await?;

    Ok(())
}

#[when(expr = "I click the first count {int} times")]
#[when(expr = "I click the count {int} times")]
async fn i_click_the_first_button_n_times(
    world: &mut AppWorld,
    times: u32,
) -> Result<()> {
    let client = &world.client;

    for _ in 1..=times {
        action::click_first_button(client).await?;
    }

    Ok(())
}

#[when(expr = "I click the second count {int} times")]
async fn i_click_the_second_button_n_times(
    world: &mut AppWorld,
    times: u32,
) -> Result<()> {
    let client = &world.client;

    for _ in 1..=times {
        action::click_second_button(client).await?;
    }

    Ok(())
}
