use anyhow::{Ok, Result};
use fantoccini::{elements::Element, Client, Locator};

pub async fn page_title(client: &Client) -> Result<String> {
    let selector = "h1";
    let element = client
        .wait()
        .for_element(Locator::Css(selector))
        .await
        .expect(
            format!("Page title not found by Css selector `{}`", selector)
                .as_str(),
        );

    let text = element.text().await?;

    Ok(text)
}

pub async fn loaded_one_message(client: &Client) -> Result<String> {
    let text = component_message(client, "loaded-1").await?;

    Ok(text)
}

pub async fn loaded_two_message(client: &Client) -> Result<String> {
    let text = component_message(client, "loaded-2").await?;

    Ok(text)
}

pub async fn following_message(client: &Client) -> Result<String> {
    let text = component_message(client, "following-message").await?;

    Ok(text)
}

pub async fn inside_message(client: &Client) -> Result<String> {
    let text = component_message(client, "inside-message").await?;

    Ok(text)
}

pub async fn first_count(client: &Client) -> Result<u32> {
    let element = first_button(client).await?;
    let text = element.text().await?;
    let count = text.parse::<u32>().unwrap();

    Ok(count)
}

pub async fn first_button(client: &Client) -> Result<Element> {
    let counter_button = client
        .wait()
        .for_element(Locator::Css("button"))
        .await
        .expect("First button not found");

    Ok(counter_button)
}

pub async fn second_count(client: &Client) -> Result<u32> {
    let element = second_button(client).await?;
    let text = element.text().await?;
    let count = text.parse::<u32>().unwrap();

    Ok(count)
}

pub async fn second_button(client: &Client) -> Result<Element> {
    let counter_button = client
        .wait()
        .for_element(Locator::Id("second-count"))
        .await
        .expect("Second button not found");

    Ok(counter_button)
}

pub async fn instrumented_count(
    client: &Client,
    selector: &str,
) -> Result<u32> {
    let element = client
        .wait()
        .for_element(Locator::Id(selector))
        .await
        .expect(format!("Element #{selector} not found.").as_str());
    let text = element.text().await?;
    let count = text.parse::<u32>().expect(
        format!("Element #{selector} does not contain a number.").as_str(),
    );
    Ok(count)
}

pub async fn reset_counter(client: &Client) -> Result<Element> {
    let reset_button = client
        .wait()
        .for_element(Locator::Id("reset-counters"))
        .await
        .expect("Reset counter input not found");

    Ok(reset_button)
}

pub async fn reset_csr_counter(client: &Client) -> Result<Element> {
    let reset_button = client
        .wait()
        .for_element(Locator::Id("reset-csr-counters"))
        .await
        .expect("Reset CSR counter input not found");

    Ok(reset_button)
}

async fn component_message(client: &Client, id: &str) -> Result<String> {
    let element =
        client.wait().for_element(Locator::Id(id)).await.expect(
            format!("loaded message not found by id `{}`", id).as_str(),
        );

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
