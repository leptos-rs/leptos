use super::{find, world::HOST};
use anyhow::Result;
use fantoccini::Client;
use std::result::Result::Ok;

pub async fn goto_path(client: &Client, path: &str) -> Result<()> {
    let url = format!("{}{}", HOST, path);
    client.goto(&url).await?;

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
/// `window.fetch` to reject requests for `.wasm` files while blocking is
/// enabled. The main bundle is unaffected: it has already been loaded by the
/// time this runs, and everything that isn't a `.wasm` request passes through.
pub async fn set_wasm_chunks_blocked(client: &Client, blocked: bool) -> Result<()> {
    client
        .execute(
            r#"
            const [blocked] = arguments;
            if (!window.__realFetch) {
                window.__realFetch = window.fetch;
                window.fetch = function (input, init) {
                    const url =
                        typeof input === "string" ? input : input.url || String(input);
                    if (window.__blockWasmChunks && url.split("?")[0].endsWith(".wasm")) {
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
