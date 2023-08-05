use super::{action, check, world::AppWorld};
use anyhow::{Ok, Result};
use cucumber::{given, then, when};

#[when(regex = "^I delete the todo named (.*)$")]
async fn i_delete_the_todo_named(
    world: &mut AppWorld,
    text: String,
) -> Result<()> {
    action::delete_todo(world, text.as_str()).await?;

    Ok(())
}

#[given("the todo list is empty")]
#[when("I empty the todo list")]
async fn i_empty_the_todo_list(world: &mut AppWorld) -> Result<()> {
    action::empty_todo_list(world).await?;

    Ok(())
}

#[then(regex = "^I see the empty list message is (.*)$")]
async fn i_see_the_empty_list_message_is(
    world: &mut AppWorld,
    text: String,
) -> Result<()> {
    check::text_on_element(world, "ul p", &text).await?;

    Ok(())
}

#[then(regex = "^I do not see the todo named (.*)$")]
async fn i_do_not_see_the_todo_is_present(
    world: &mut AppWorld,
    text: String,
) -> Result<()> {
    check::todo_present(world, text.as_str(), false).await?;

    Ok(())
}
