use crate::fixtures::{check, world::AppWorld};
use anyhow::{Ok, Result};
use cucumber::{gherkin::Step, then};

#[then(regex = r"^I see the result is empty$")]
async fn i_see_the_result_is_empty(world: &mut AppWorld) -> Result<()> {
    let client = &world.client;
    check::result_text_is(client, "").await?;
    Ok(())
}

#[then(regex = r"^I see the result is the string (.*)$")]
async fn i_see_the_result_is_the_string(
    world: &mut AppWorld,
    text: String,
) -> Result<()> {
    let client = &world.client;
    check::result_text_is(client, &text).await?;
    Ok(())
}

#[then(regex = r"^I see the navbar$")]
async fn i_see_the_navbar(world: &mut AppWorld) -> Result<()> {
    let client = &world.client;
    check::element_exists(client, "nav").await?;
    Ok(())
}

#[then(regex = r"^I counted ([0-9]+) log message$")]
#[then(regex = r"^I counted ([0-9]+) log messages$")]
#[then(regex = r"^I see ([0-9]+) log message$")]
#[then(regex = r"^I see ([0-9]+) log messages$")]
async fn i_counted_log_messages(
    world: &mut AppWorld,
    count: usize,
) -> Result<()> {
    let client = &world.client;
    check::count_log_messages(client, count).await?;
    Ok(())
}

#[then(regex = r"^I find the following being the most recent log messages$")]
async fn i_find_the_following_being_the_most_recent_log_messages(
    world: &mut AppWorld,
    step: &Step,
) -> Result<()> {
    let client = &world.client;

    let expected = step
        .table
        .as_ref()
        .expect("the table must be present")
        .rows
        .iter()
        .map(|row| row[0].as_str())
        .collect::<Vec<_>>();

    check::last_log_messages(client, &expected).await?;

    Ok(())
}
