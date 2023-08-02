use super::{find, world::AppWorld};
use anyhow::{Ok, Result};
use fantoccini::Locator;
use pretty_assertions::assert_eq;

pub async fn text_on_element(
    world: &mut AppWorld,
    selector: &str,
    expected_text: &str,
) -> Result<()> {
    let client = &world.client;
    let element = client
        .wait()
        .for_element(Locator::Css(selector))
        .await
        .expect(
            format!("Element not found by Css selector `{}`", selector)
                .as_str(),
        );

    let actual = element.text().await?;
    assert_eq!(&actual, expected_text);

    Ok(())
}

pub async fn todo_present(
    world: &mut AppWorld,
    text: &str,
    expected: bool,
) -> Result<()> {
    let todo_present = is_todo_present(world, text).await;

    assert_eq!(todo_present, expected);

    Ok(())
}

async fn is_todo_present(world: &mut AppWorld, text: &str) -> bool {
    let todos = find::todos(world).await;

    for todo in todos {
        let todo_title = todo.text().await.expect("Todo title not found");
        if todo_title == text {
            return true;
        }
    }

    false
}
