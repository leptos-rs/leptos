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

#[given(regex = "^I add a todo as (.*)$")]
#[when(regex = "^I add a todo as (.*)$")]
async fn i_add_a_todo_titled(world: &mut AppWorld, text: String) -> Result<()> {
    let client = &world.client;
    action::add_todo(client, text.as_str()).await?;

    Ok(())
}

#[given(regex = "^I set the todo as (.*)$")]
async fn i_set_the_todo_as(world: &mut AppWorld, text: String) -> Result<()> {
    let client = &world.client;
    action::fill_todo(client, &text).await?;

    Ok(())
}

#[when(regex = "I click the Add button$")]
async fn i_click_the_button(world: &mut AppWorld) -> Result<()> {
    let client = &world.client;
    action::click_add_button(client).await?;

    Ok(())
}

#[when(regex = "^I delete the todo named (.*)$")]
async fn i_delete_the_todo_named(
    world: &mut AppWorld,
    text: String,
) -> Result<()> {
    let client = &world.client;
    action::delete_todo(client, text.as_str()).await?;

    Ok(())
}

#[given("the todo list is empty")]
#[when("I empty the todo list")]
async fn i_empty_the_todo_list(world: &mut AppWorld) -> Result<()> {
    let client = &world.client;
    action::empty_todo_list(client).await?;

    Ok(())
}
