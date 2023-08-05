use super::{action, check, world::AppWorld};
use anyhow::{Ok, Result};
use cucumber::{given, then, when};

#[given(regex = "^I add a todo as (.*)$")]
#[when(regex = "^I add a todo as (.*)$")]
async fn i_add_a_todo_titled(world: &mut AppWorld, text: String) -> Result<()> {
    action::add_todo(world, text.as_str()).await?;

    Ok(())
}

#[given(regex = "^I set the todo as (.*)$")]
async fn i_set_the_todo_as(world: &mut AppWorld, text: String) -> Result<()> {
    action::fill_todo(world, &text).await?;

    Ok(())
}

#[when(regex = "I click the Add button$")]
async fn i_click_the_button(world: &mut AppWorld) -> Result<()> {
    action::click_add_button(world).await?;

    Ok(())
}

#[then(regex = "^I see the todo named (.*)$")]
async fn i_see_the_todo_is_present(
    world: &mut AppWorld,
    text: String,
) -> Result<()> {
    check::todo_present(world, text.as_str(), true).await?;

    Ok(())
}

#[then("I see the pending todo")]
async fn i_see_the_pending_todo(world: &mut AppWorld) -> Result<()> {
    check::todo_is_pending(world).await?;

    Ok(())
}
