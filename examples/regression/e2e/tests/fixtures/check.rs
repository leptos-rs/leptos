use crate::fixtures::find;
use anyhow::{Ok, Result};
use fantoccini::Client;
use pretty_assertions::assert_eq;

pub async fn result_text_is(
    client: &Client,
    expected_text: &str,
) -> Result<()> {
    let actual = find::text_at_id(client, "result").await?;
    assert_eq!(&actual, expected_text);
    Ok(())
}
