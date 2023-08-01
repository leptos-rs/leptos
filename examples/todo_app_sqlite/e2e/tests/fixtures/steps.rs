use super::{
    actions::{click_add_button, empty_todo_list, fill_todo, goto_path},
    checks::check_text_on_element,
    world::AppWorld,
};
use anyhow::{Ok, Result};
use cucumber::{given, then, when};

#[when("I empty the todo list")]
async fn the_todo_list_is_empty(world: &mut AppWorld) -> Result<()> {
    empty_todo_list(world).await?;

    Ok(())
}

#[given("I see the app")]
#[given("I open the app")]
#[when("I open the app")]
async fn i_open_the_app(world: &mut AppWorld) -> Result<()> {
    goto_path(world, "").await?;

    Ok(())
}

#[given(regex = "^I set the todo as (.*)$")]
async fn i_set_the_todo_as(world: &mut AppWorld, text: String) -> Result<()> {
    fill_todo(world, &text).await?;

    Ok(())
}

#[when(regex = "I click the Add button$")]
async fn i_click_the_button(world: &mut AppWorld) -> Result<()> {
    click_add_button(world).await?;

    Ok(())
}

#[then(regex = "^I see the page title is (.*)$")]
async fn i_see_the_page_title_is(
    world: &mut AppWorld,
    text: String,
) -> Result<()> {
    check_text_on_element(world, "h1", &text).await?;

    Ok(())
}

#[then(regex = "^I see the label of the input is (.*)$")]
async fn i_see_the_label_of_the_input_is(
    world: &mut AppWorld,
    text: String,
) -> Result<()> {
    check_text_on_element(world, "label", &text).await?;

    Ok(())
}

#[then(regex = "^I see the last todo is (.*)$")]
async fn i_see_the_last_todo_is(
    world: &mut AppWorld,
    text: String,
) -> Result<()> {
    check_text_on_element(world, "li:last-child", &text).await?;

    Ok(())
}

#[then(regex = "^I see the empty list message is (.*)$")]
async fn i_see_the_no_todo_message_is(
    world: &mut AppWorld,
    text: String,
) -> Result<()> {
    check_text_on_element(world, "ul p", &text).await?;

    Ok(())
}
