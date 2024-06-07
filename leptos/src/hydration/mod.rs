#![allow(clippy::needless_lifetimes)]

use crate::prelude::*;
use leptos_config::LeptosOptions;
use leptos_macro::{component, view};

#[component]
pub fn AutoReload(
    #[prop(optional)] disable_watch: bool,
    options: LeptosOptions,
) -> impl IntoView {
    (!disable_watch && std::env::var("LEPTOS_WATCH").is_ok()).then(|| {
        #[cfg(feature = "nonce")]
        let nonce = crate::nonce::use_nonce();
        #[cfg(not(feature = "nonce"))]
        let nonce = None::<()>;

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
            <script nonce=nonce>
                {format!("{script}({reload_port:?}, {protocol})")}
            </script>
        }
    })
}

#[component]
pub fn HydrationScripts(
    options: LeptosOptions,
    #[prop(optional)] islands: bool,
) -> impl IntoView {
    let pkg_path = &options.site_pkg_dir;
    let output_name = &options.output_name;
    let mut wasm_output_name = output_name.clone();
    if std::option_env!("LEPTOS_OUTPUT_NAME").is_none() {
        wasm_output_name.push_str("_bg");
    }
    #[cfg(feature = "nonce")]
    let nonce = crate::nonce::use_nonce();
    #[cfg(not(feature = "nonce"))]
    let nonce = None::<()>;
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
