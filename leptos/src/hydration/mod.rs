#![allow(clippy::needless_lifetimes)]

use crate::prelude::*;
use leptos_config::LeptosOptions;
use leptos_macro::{component, view};
use tachys::view::RenderHtml;

#[component]
pub fn AutoReload<'a>(
    #[prop(optional)] disable_watch: bool,
    #[prop(optional)] nonce: Option<&'a str>,
    options: LeptosOptions,
) -> impl RenderHtml<Dom> + 'a {
    (!disable_watch && std::env::var("LEPTOS_WATCH").is_ok()).then(|| {
        let reload_port = match options.reload_external_port {
            Some(val) => val,
            None => options.reload_port,
        };
        let protocol = match options.reload_ws_protocol {
            leptos_config::ReloadWSProtocol::WS => "'ws://'",
            leptos_config::ReloadWSProtocol::WSS => "'wss://'",
        };

        let script = include_str!("reload_script.js");
        view! {
            <script crossorigin=nonce>
                {format!("{script}({reload_port:?}, {protocol})")}
            </script>
        }
    })
}

#[component]
pub fn HydrationScripts(
    options: LeptosOptions,
    #[prop(optional)] islands: bool,
) -> impl RenderHtml<Dom> {
    let pkg_path = &options.site_pkg_dir;
    let output_name = &options.output_name;
    let mut wasm_output_name = output_name.clone();
    if std::option_env!("LEPTOS_OUTPUT_NAME").is_none() {
        wasm_output_name.push_str("_bg");
    }
    let nonce = None::<String>; // use_nonce(); // TODO
    let script = if islands {
        include_str!("./island_script.js")
    } else {
        include_str!("./hydration_script.js")
    };

    view! {
        <link rel="modulepreload" href=format!("/{pkg_path}/{output_name}.js") nonce=nonce.clone()/>
        <link rel="preload" href=format!("/{pkg_path}/{wasm_output_name}.wasm") r#as="fetch" r#type="application/wasm" crossorigin=nonce.clone().unwrap_or_default()/>
        <script type="module" nonce=nonce>
            {format!("{script}({pkg_path:?}, {output_name:?}, {wasm_output_name:?})")}
        </script>
    }
}
