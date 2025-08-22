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

pub async fn element_exists(client: &Client, id: &str) -> Result<()> {
    find::element_by_id(client, id)
        .await
        .expect(&format!("could not find element with id `{id}`"));
    Ok(())
}

pub async fn count_log_messages(client: &Client, count: usize) -> Result<()> {
    let elements = find::log_message_elements(client).await?;
    assert_eq!(elements.len(), count);
    Ok(())
}

pub async fn last_log_messages(
    client: &Client,
    expected: &[&str],
) -> Result<()> {
    let elements = find::log_message_elements(client).await?;
    let elements_len = elements.len();
    let expected_len = expected.len();
    assert!(
        elements_len >= expected_len,
        "the messages available is not equal or greater than what is being expected",
    );

    let mut result = Vec::new();
    for element in elements.into_iter().skip(elements_len - expected_len) {
        result.push(element.text().await?);
    }
    assert_eq!(result, expected);
    Ok(())
}
