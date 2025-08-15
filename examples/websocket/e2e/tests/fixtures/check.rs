use anyhow::{Ok, Result};
use fantoccini::{Client, Locator};
use pretty_assertions::assert_eq;

pub async fn text_on_element(
    client: &Client,
    selector: &str,
    expected_text: &str,
) -> Result<()> {
    let element = client
        .wait()
        .for_element(Locator::Css(selector))
        .await
        .unwrap_or_else(|_| {
            panic!("Element not found by Css selector `{}`", selector)
        });

    let actual = element.text().await?;
    assert_eq!(&actual, expected_text);

    Ok(())
}
