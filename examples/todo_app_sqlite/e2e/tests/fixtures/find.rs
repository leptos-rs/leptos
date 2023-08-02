use super::world::AppWorld;
use fantoccini::{elements::Element, Locator};

pub async fn todo_input(world: &mut AppWorld) -> Element {
    let client = &world.client;
    let textbox = client
        .wait()
        .for_element(Locator::Css("input[name='title"))
        .await
        .expect("Todo textbox not found");

    textbox
}

pub async fn add_button(world: &mut AppWorld) -> Element {
    let client = &world.client;
    let button = client
        .wait()
        .for_element(Locator::Css("input[value='Add']"))
        .await
        .expect("");

    button
}

pub async fn last_delete_button(world: &mut AppWorld) -> Option<Element> {
    let client = &world.client;
    if let Ok(element) = client
        .wait()
        .for_element(Locator::Css("li:last-child input[value='X']"))
        .await
    {
        return Some(element);
    }

    None
}

pub async fn delete_button(
    world: &mut AppWorld,
    text: &str,
) -> Option<Element> {
    let client = &world.client;
    let selector = format!("//*[text()='{text}']//input[@value='X']");
    if let Ok(element) =
        client.wait().for_element(Locator::XPath(&selector)).await
    {
        return Some(element);
    }

    None
}

pub async fn todos(world: &mut AppWorld) -> Vec<Element> {
    let client = &world.client;
    let todos = client
        .find_all(Locator::Css("li"))
        .await
        .expect("Todo List not found");

    todos
}
