use crate::app::world::AppWorld;
use anyhow::Result;
use fantoccini::Locator;

pub async fn check_text_on_element(
    world: &mut AppWorld,
    selector: &str,
    expected_text: &str,
) -> Result<()> {
    let client = &world.client;
    let element = client.wait().for_element(Locator::Css(selector)).await?;

    let actual = element.text().await?;
    assert_eq!(&actual, expected_text);

    Ok(())
}
