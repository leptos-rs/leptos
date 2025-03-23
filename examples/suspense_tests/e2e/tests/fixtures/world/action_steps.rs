use crate::fixtures::{action, world::AppWorld};
use anyhow::{Ok, Result};
use cucumber::{gherkin::Step, given, when};

#[given("I see the app")]
#[when("I open the app")]
async fn i_open_the_app(world: &mut AppWorld) -> Result<()> {
    let client = &world.client;
    action::goto_path(client, "").await?;

    Ok(())
}

#[given(regex = r"^I select the mode (.*)$")]
#[given(regex = r"^I select the component (.*)$")]
#[when(regex = "^I select the component (.*)$")]
#[given(regex = "^I select the link (.*)$")]
#[when(regex = "^I select the link (.*)$")]
#[when(regex = "^I click on the link (.*)$")]
#[when(regex = "^I go check the (.*)$")]
async fn i_select_the_link(world: &mut AppWorld, text: String) -> Result<()> {
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

#[given(regex = "^I (refresh|reload) the (browser|page)$")]
#[when(regex = "^I (refresh|reload) the (browser|page)$")]
async fn i_refresh_the_browser(world: &mut AppWorld) -> Result<()> {
    let client = &world.client;
    client.refresh().await?;

    Ok(())
}

#[when(expr = "I click on Reset Counters")]
async fn i_click_on_reset_counters(world: &mut AppWorld) -> Result<()> {
    let client = &world.client;
    action::click_reset_counters_button(client).await?;

    Ok(())
}

#[given(expr = "I click on Reset CSR Counters")]
#[when(expr = "I click on Reset CSR Counters")]
async fn i_click_on_reset_csr_counters(world: &mut AppWorld) -> Result<()> {
    let client = &world.client;
    action::click_reset_csr_counters_button(client).await?;

    Ok(())
}

#[when(expr = "I access the instrumented counters via SSR")]
async fn i_access_the_instrumented_counters_page_via_ssr(
    world: &mut AppWorld,
) -> Result<()> {
    let client = &world.client;
    action::click_link(client, "Instrumented").await?;
    action::click_link(client, "Counters").await?;
    client.refresh().await?;

    Ok(())
}

#[when(expr = "I access the instrumented counters via CSR")]
async fn i_access_the_instrumented_counters_page_via_csr(
    world: &mut AppWorld,
) -> Result<()> {
    let client = &world.client;
    action::click_link(client, "Instrumented").await?;
    action::click_link(client, "Counters").await?;

    Ok(())
}

#[given(expr = "I select the following links")]
#[when(expr = "I select the following links")]
async fn i_select_the_following_links(
    world: &mut AppWorld,
    step: &Step,
) -> Result<()> {
    let client = &world.client;

    if let Some(table) = step.table.as_ref() {
        for row in table.rows.iter() {
            action::click_link(client, &row[0]).await?;
        }
    }

    Ok(())
}
