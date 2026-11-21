use super::{find, world::HOST};
use anyhow::Result;
use fantoccini::{Client, Locator};
use std::result::Result::Ok;

pub async fn goto_path(client: &Client, path: &str) -> Result<()> {
    let url = format!("{}{}", HOST, path);
    client.goto(&url).await?;
    wait_for_hydration(client).await?;

    Ok(())
}

pub async fn wait_for_hydration(client: &Client) -> Result<()> {
    client
        .wait()
        .for_element(Locator::Css("#hydrated[data-hydrated='true']"))
        .await?;

    Ok(())
}

pub async fn click_link(client: &Client, text: &str) -> Result<()> {
    let link = find::link_with_text(&client, &text).await?;
    link.click().await?;
    Ok(())
}

pub async fn click_button(client: &Client, id: &str) -> Result<()> {
    let btn = find::element_by_id(&client, &id).await?;
    btn.click().await?;
    Ok(())
}

/// Simulates a network failure for lazily loaded WASM chunks by shimming
/// `window.fetch` to reject requests for split `.wasm` files while blocking is
/// enabled. The main `/pkg/regression.wasm` bundle and non-`.wasm` requests
/// pass through.
pub async fn set_wasm_chunks_blocked(
    client: &Client,
    blocked: bool,
) -> Result<()> {
    client
        .execute(
            r#"
            const [blocked] = arguments;
            if (!window.__realFetch) {
                window.__realFetch = window.fetch;
                window.fetch = function (input, init) {
                    const url =
                        typeof input === "string" ? input : input.url || String(input);
                    const path = url.split("?")[0];
                    if (
                        window.__blockWasmChunks &&
                        path.endsWith(".wasm") &&
                        !path.endsWith("/regression.wasm")
                    ) {
                        return Promise.reject(
                            new TypeError("simulated network failure: " + url)
                        );
                    }
                    return window.__realFetch.apply(this, arguments);
                };
            }
            window.__blockWasmChunks = blocked;
            "#,
            vec![serde_json::Value::Bool(blocked)],
        )
        .await?;
    Ok(())
}
