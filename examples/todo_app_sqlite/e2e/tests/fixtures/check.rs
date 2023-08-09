use super::find;
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
        .expect(
            format!("Element not found by Css selector `{}`", selector)
                .as_str(),
        );

    let actual = element.text().await?;
    assert_eq!(&actual, expected_text);

    Ok(())
}

pub async fn todo_present(
    client: &Client,
    text: &str,
    expected: bool,
) -> Result<()> {
    let todo_present = is_todo_present(client, text).await;

    assert_eq!(todo_present, expected);

    Ok(())
}

async fn is_todo_present(client: &Client, text: &str) -> bool {
    let todos = find::todos(client).await;

    for todo in todos {
        let todo_title = todo.text().await.expect("Todo title not found");
        if todo_title == text {
            return true;
        }
    }

    false
}

pub async fn todo_is_pending(client: &Client) -> Result<()> {
    if let None = find::pending_todo(client).await {
        assert!(false, "Pending todo not found");
    }

    Ok(())
}
