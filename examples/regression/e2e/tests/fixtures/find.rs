use anyhow::{Ok, Result};
use fantoccini::{elements::Element, Client, Locator};

pub async fn text_at_id(client: &Client, id: &str) -> Result<String> {
    let element = element_by_id(client, id)
        .await
        .expect(format!("no such element with id `{}`", id).as_str());
    let text = element.text().await?;
    Ok(text)
}

pub async fn link_with_text(client: &Client, text: &str) -> Result<Element> {
    let link = client
        .wait()
        .for_element(Locator::LinkText(text))
        .await
        .expect(format!("Link not found by `{}`", text).as_str());
    Ok(link)
}

pub async fn element_by_id(client: &Client, id: &str) -> Result<Element> {
    Ok(client.wait().for_element(Locator::Id(id)).await?)
}
