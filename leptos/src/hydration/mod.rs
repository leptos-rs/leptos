#![allow(clippy::needless_lifetimes)]

use crate::prelude::*;
use leptos_config::LeptosOptions;
use leptos_macro::{component, view};

/// Inserts auto-reloading code used in `cargo-leptos`.
///
/// This should be included in the `<head>` of your application shell during development.
#[component]
pub fn AutoReload(
    /// Whether the file-watching feature should be disabled.
    #[prop(optional)]
    disable_watch: bool,
    /// Configuration options for this project.
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

        let script = format!(
            "(function (reload_port, protocol) {{ {} {} }})({reload_port:?}, \
             {protocol})",
            leptos_hot_reload::HOT_RELOAD_JS,
            include_str!("reload_script.js")
        );
        view! { <script nonce=nonce>{script}</script> }
    })
}

/// Inserts hydration scripts that add interactivity to your server-rendered HTML.
///
/// This should be included in the `<head>` of your application shell.
#[component]
pub fn HydrationScripts(
    /// Configuration options for this project.
    options: LeptosOptions,
    /// Should be `true` to hydrate in `islands` mode.
    #[prop(optional)]
    islands: bool,
    /// A base url, not including a trailing slash
    #[prop(optional, into)]
    root: Option<String>,
) -> impl IntoView {
    let mut js_file_name = options.output_name.to_string();
    let mut wasm_file_name = options.output_name.to_string();
    if options.hash_files {
        let hash_path = std::env::current_exe()
            .map(|path| {
                path.parent().map(|p| p.to_path_buf()).unwrap_or_default()
            })
            .unwrap_or_default()
            .join(options.hash_file.as_ref());
        if hash_path.exists() {
            let hashes = std::fs::read_to_string(&hash_path)
                .expect("failed to read hash file");
            for line in hashes.lines() {
                let line = line.trim();
                if !line.is_empty() {
                    if let Some((file, hash)) = line.split_once(':') {
                        if file == "js" {
                            js_file_name.push_str(&format!(".{}", hash.trim()));
                        } else if file == "wasm" {
                            wasm_file_name
                                .push_str(&format!(".{}", hash.trim()));
                        }
                    }
                }
            }
        }
    } else if std::option_env!("LEPTOS_OUTPUT_NAME").is_none() {
        wasm_file_name.push_str("_bg");
    }

    let pkg_path = &options.site_pkg_dir;
    #[cfg(feature = "nonce")]
    let nonce = crate::nonce::use_nonce();
    #[cfg(not(feature = "nonce"))]
    let nonce = None::<String>;
    let script = if islands {
        if let Some(sc) = Owner::current_shared_context() {
            sc.set_is_hydrating(false);
        }
        include_str!("./island_script.js")
    } else {
        include_str!("./hydration_script.js")
    };

    let root = root.unwrap_or_default();
    view! {
        <link rel="modulepreload" href=format!("{root}/{pkg_path}/{js_file_name}.js") nonce=nonce.clone()/>
        <link
            rel="preload"
            href=format!("{root}/{pkg_path}/{wasm_file_name}.wasm")
            r#as="fetch"
            r#type="application/wasm"
            crossorigin=nonce.clone().unwrap_or_default()
        />
        <script type="module" nonce=nonce>
            {format!("{script}({root:?}, {pkg_path:?}, {js_file_name:?}, {wasm_file_name:?})")}
        </script>
    }
}
