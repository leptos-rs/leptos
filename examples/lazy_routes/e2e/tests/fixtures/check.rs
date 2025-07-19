use crate::fixtures::find;
use anyhow::{Ok, Result};
use fantoccini::Client;
use pretty_assertions::assert_eq;

pub async fn page_name_is(client: &Client, expected_text: &str) -> Result<()> {
    let actual = find::text_at_id(client, "page").await?;
    assert_eq!(&actual, expected_text);
    Ok(())
}

pub async fn result_is(client: &Client, expected_text: &str) -> Result<()> {
    let actual = find::text_at_id(client, "result").await?;
    assert_eq!(&actual, expected_text);
    Ok(())
}

pub async fn navigating_appears(client: &Client) -> Result<()> {
    let actual = find::text_at_id(client, "navigating").await?;
    assert_eq!(&actual, "Navigating...");
    Ok(())
}

pub async fn element_exists(client: &Client, id: &str) -> Result<()> {
    find::element_by_id(client, id)
        .await
        .expect(&format!("could not find element with id `{id}`"));
    Ok(())
}
