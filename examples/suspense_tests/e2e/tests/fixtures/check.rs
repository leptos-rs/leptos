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
