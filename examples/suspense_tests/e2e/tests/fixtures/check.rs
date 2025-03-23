use crate::fixtures::find;
use anyhow::{Ok, Result};
use fantoccini::Client;
use pretty_assertions::assert_eq;

pub async fn page_title_is(client: &Client, expected_text: &str) -> Result<()> {
    let actual = find::page_title(client).await?;
    assert_eq!(&actual, expected_text);

    Ok(())
}

pub async fn loaded_one_message_is(
    client: &Client,
    expected_text: &str,
) -> Result<()> {
    let actual = find::loaded_one_message(client).await?;
    assert_eq!(&actual, expected_text);

    Ok(())
}

pub async fn loaded_two_message_is(
    client: &Client,
    expected_text: &str,
) -> Result<()> {
    let actual = find::loaded_two_message(client).await?;
    assert_eq!(&actual, expected_text);

    Ok(())
}

pub async fn inside_message_is(
    client: &Client,
    expected_text: &str,
) -> Result<()> {
    let actual = find::inside_message(client).await?;
    assert_eq!(&actual, expected_text);

    Ok(())
}

pub async fn following_message_is(
    client: &Client,
    expected_text: &str,
) -> Result<()> {
    let actual = find::following_message(client).await?;
    assert_eq!(&actual, expected_text);

    Ok(())
}

pub async fn first_count_is(client: &Client, expected: u32) -> Result<()> {
    let actual = find::first_count(client).await?;
    assert_eq!(actual, expected);

    Ok(())
}

pub async fn second_count_is(client: &Client, expected: u32) -> Result<()> {
    let actual = find::second_count(client).await?;
    assert_eq!(actual, expected);

    Ok(())
}

pub async fn instrumented_counts(
    client: &Client,
    expected: &[(&str, u32)],
) -> Result<()> {
    let mut actual = Vec::<(&str, u32)>::new();

    for (selector, _) in expected.iter() {
        actual
            .push((selector, find::instrumented_count(client, selector).await?))
    }

    assert_eq!(actual, expected);

    Ok(())
}

pub async fn link_text_is_aria_current(
    client: &Client,
    text: &str,
) -> Result<()> {
    let link = find::link_with_text(client, text).await?;

    link.attr("aria-current")
        .await?
        .expect(format!("aria-current missing for {text}").as_str());

    Ok(())
}

pub async fn link_text_is_not_aria_current(
    client: &Client,
    text: &str,
) -> Result<()> {
    let link = find::link_with_text(client, text).await?;

    link.attr("aria-current")
        .await?
        .map(|_| anyhow::bail!("aria-current mistakenly set for {text}"))
        .unwrap_or(Ok(()))
}
