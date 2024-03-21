use fantoccini::{elements::Element, Client, Locator};

pub async fn todo_input(client: &Client) -> Element {
    let textbox = client
        .wait()
        .for_element(Locator::Css("input[name='title"))
        .await
        .expect("Todo textbox not found");

    textbox
}

pub async fn add_button(client: &Client) -> Element {
    let button = client
        .wait()
        .for_element(Locator::Css("input[value='Add']"))
        .await
        .expect("");

    button
}

pub async fn first_delete_button(client: &Client) -> Option<Element> {
    if let Ok(element) = client
        .wait()
        .for_element(Locator::Css("li:first-child input[value='X']"))
        .await
    {
        return Some(element);
    }

    None
}

pub async fn delete_button(client: &Client, text: &str) -> Option<Element> {
    let selector = format!("//*[text()='{text}']//input[@value='X']");
    if let Ok(element) =
        client.wait().for_element(Locator::XPath(&selector)).await
    {
        return Some(element);
    }

    None
}

pub async fn pending_todo(client: &Client) -> Option<Element> {
    if let Ok(element) =
        client.wait().for_element(Locator::Css(".pending")).await
    {
        return Some(element);
    }

    None
}

pub async fn todos(client: &Client) -> Vec<Element> {
    let todos = client
        .find_all(Locator::Css("li"))
        .await
        .expect("Todo List not found");

    todos
}
