use crate::fixtures::{check, world::AppWorld};
use anyhow::{Ok, Result};
use cucumber::then;

#[then(regex = "^I see the page title is (.*)$")]
async fn i_see_the_page_title_is(
    world: &mut AppWorld,
    text: String,
) -> Result<()> {
    let client = &world.client;
    check::text_on_element(client, "h1", &text).await?;

    Ok(())
}

#[then(regex = "^I see the label of the input is (.*)$")]
async fn i_see_the_label_of_the_input_is(
    world: &mut AppWorld,
    text: String,
) -> Result<()> {
    let client = &world.client;
    check::text_on_element(client, "label", &text).await?;

    Ok(())
}

#[then(regex = "^I see the todo named (.*)$")]
async fn i_see_the_todo_is_present(
    world: &mut AppWorld,
    text: String,
) -> Result<()> {
    let client = &world.client;
    check::todo_present(client, text.as_str(), true).await?;

    Ok(())
}

#[then("I see the pending todo")]
async fn i_see_the_pending_todo(world: &mut AppWorld) -> Result<()> {
    let client = &world.client;

    check::todo_is_pending(client).await?;

    Ok(())
}

#[then(regex = "^I see the empty list message is (.*)$")]
async fn i_see_the_empty_list_message_is(
    world: &mut AppWorld,
    text: String,
) -> Result<()> {
    let client = &world.client;
    check::text_on_element(client, "ul p", &text).await?;

    Ok(())
}

#[then(regex = "^I do not see the todo named (.*)$")]
async fn i_do_not_see_the_todo_is_present(
    world: &mut AppWorld,
    text: String,
) -> Result<()> {
    let client = &world.client;
    check::todo_present(client, text.as_str(), false).await?;

    Ok(())
}
