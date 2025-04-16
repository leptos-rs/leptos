use fantoccini::{elements::Element, Client, Locator};

pub async fn input(client: &Client) -> Element {
    let textbox = client
        .wait()
        .for_element(Locator::Css("input"))
        .await
        .expect("websocket textbox not found");

    textbox
}

pub async fn label(client: &Client) -> Element {
    let label = client
        .wait()
        .for_element(Locator::Css("p"))
        .await
        .expect("");

    label
}
