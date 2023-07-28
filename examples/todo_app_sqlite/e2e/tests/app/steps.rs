use super::{
    actions::goto_path, checks::check_text_on_element, world::AppWorld,
};
use anyhow::{Ok, Result};
use cucumber::{given, then, when};

#[given("I see the app")]
#[when("I open the app")]
async fn i_open_the_app(world: &mut AppWorld) -> Result<()> {
    goto_path(world, "").await?;

    Ok(())
}

#[then(regex = "^I see the page title is (.*)$")]
async fn i_see_the_page_title_is(
    world: &mut AppWorld,
    title: String,
) -> Result<()> {
    check_text_on_element(world, "h1", &title).await?;

    Ok(())
}
