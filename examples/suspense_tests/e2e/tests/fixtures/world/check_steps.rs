use crate::fixtures::{check, world::AppWorld};
use anyhow::{Ok, Result};
use cucumber::{then, gherkin::Step};

#[then(regex = r"^I see the page title is (.*)$")]
async fn i_see_the_page_title_is(
    world: &mut AppWorld,
    text: String,
) -> Result<()> {
    let client = &world.client;
    check::page_title_is(client, &text).await?;

    Ok(())
}

#[then(regex = r"^I see the one second message is (.*)$")]
async fn i_see_the_one_second_message_is(
    world: &mut AppWorld,
    text: String,
) -> Result<()> {
    let client = &world.client;
    check::loaded_one_message_is(client, &text).await?;

    Ok(())
}

#[then(regex = r"^I see the two second message is (.*)$")]
async fn i_see_the_two_second_message_is(
    world: &mut AppWorld,
    text: String,
) -> Result<()> {
    let client = &world.client;
    check::loaded_two_message_is(client, &text).await?;

    Ok(())
}

#[then(regex = r"^I see the following message is (.*)$")]
async fn i_see_the_following_message_is(
    world: &mut AppWorld,
    text: String,
) -> Result<()> {
    let client = &world.client;
    check::following_message_is(client, &text).await?;

    Ok(())
}

#[then(regex = r"^I see the inside message is (.*)$")]
async fn i_see_the_inside_message_is(
    world: &mut AppWorld,
    text: String,
) -> Result<()> {
    let client = &world.client;
    check::inside_message_is(client, &text).await?;

    Ok(())
}

#[then(expr = "I see the first count is {int}")]
#[then(expr = "I see the count is {int}")]
async fn i_see_the_first_count_is(
    world: &mut AppWorld,
    expected: u32,
) -> Result<()> {
    let client = &world.client;
    check::first_count_is(client, expected).await?;

    Ok(())
}

#[then(expr = "I see the second count is {int}")]
async fn i_see_the_second_count_is(
    world: &mut AppWorld,
    expected: u32,
) -> Result<()> {
    let client = &world.client;
    check::second_count_is(client, expected).await?;

    Ok(())
}

#[then(expr = "I see the following counters under section")]
#[then(expr = "the following counters under section")]
async fn i_see_the_following_counters_under_section(
    world: &mut AppWorld,
    step: &Step,
) -> Result<()> {
    // FIXME ideally check the mode; for now leave it because effort
    let client = &world.client;
    if let Some(table) = step.table.as_ref() {
        let expected = table.rows
            .iter()
            .skip(1)
            .map(|row| (row[0].as_str(), row[1].parse::<u32>().unwrap()))
            .collect::<Vec<_>>();
        check::instrumented_counts(client, &expected).await?;
    }

    Ok(())
}
