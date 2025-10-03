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

pub async fn select_option_is_selected(
    client: &Client,
    id: &str,
) -> Result<()> {
    let el = find::element_by_id(client, id)
        .await
        .expect(&format!("could not find element with id `{id}`"));
    let selected = el.prop("selected").await?;
    assert_eq!(selected.as_deref(), Some("true"));
    Ok(())
}

pub async fn element_value_is(
    client: &Client,
    id: &str,
    expected: &str,
) -> Result<()> {
    let el = find::element_by_id(client, id)
        .await
        .expect(&format!("could not find element with id `{id}`"));
    let value = el.prop("value").await?;
    assert_eq!(value.as_deref(), Some(expected));
    Ok(())
}

pub async fn path_is(client: &Client, expected_path: &str) -> Result<()> {
    let url = client
        .current_url()
        .await
        .expect("could not access current URL");
    let path = url.path();
    assert_eq!(expected_path, path);
    Ok(())
}
