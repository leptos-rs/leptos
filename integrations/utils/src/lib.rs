use futures::{Stream, StreamExt};
use leptos::{nonce::use_nonce, use_context, RuntimeId};
use leptos_config::LeptosOptions;
use leptos_meta::MetaContext;
use std::{borrow::Cow, collections::HashMap, env, fs};

extern crate tracing;

#[tracing::instrument(level = "trace", fields(error), skip_all)]
fn autoreload(nonce_str: &str, options: &LeptosOptions) -> String {
    let reload_port = match options.reload_external_port {
        Some(val) => val,
        None => options.reload_port,
    };
    let protocol = match options.reload_ws_protocol {
        leptos_config::ReloadWSProtocol::WS => "'ws://'",
        leptos_config::ReloadWSProtocol::WSS => "'wss://'",
    };
    match std::env::var("LEPTOS_WATCH").is_ok() {
        true => format!(
            r#"
                <script crossorigin=""{nonce_str}>(function () {{
                    {}
                    let host = window.location.hostname;
                    let ws = new WebSocket({protocol} + host + ':{reload_port}/live_reload');
                    ws.onmessage = (ev) => {{
                        let msg = JSON.parse(ev.data);
                        if (msg.all) window.location.reload();
                        if (msg.css) {{
                            let found = false;
                            document.querySelectorAll("link").forEach((link) => {{
                                if (link.getAttribute('href').includes(msg.css)) {{
                                    let newHref = '/' + msg.css + '?version=' + new Date().getMilliseconds();
                                    link.setAttribute('href', newHref);
                                    found = true;
                                }}
                            }});
                            if (!found) console.warn(`CSS hot-reload: Could not find a <link href=/\"${{msg.css}}\"> element`);
                        }};
                        if(msg.view) {{
                            patch(msg.view);
                        }}
                    }};
                    ws.onclose = () => console.warn('Live-reload stopped. Manual reload necessary.');
                }})()
                </script>
                "#,
            leptos_hot_reload::HOT_RELOAD_JS
        ),
        false => "".to_string(),
    }
}

#[tracing::instrument(level = "trace", fields(error), skip_all)]
pub fn html_parts_separated(
    options: &LeptosOptions,
    meta: Option<&MetaContext>,
) -> (String, &'static str) {
    // First check runtime env, then build time, then default:
    let pkg_path = match std::env::var("CDN_PKG_PATH").ok().map(Cow::from) {
        Some(path) => path,
        None => match option_env!("CDN_PKG_PATH").map(Cow::from) {
            Some(path) => path,
            None => format!("/{}", options.site_pkg_dir).into(),
        },
    };
    let output_name = &options.output_name;
    let nonce = use_nonce();
    let nonce = nonce
        .as_ref()
        .map(|nonce| format!(" nonce=\"{nonce}\""))
        .unwrap_or_default();

    // Because wasm-pack adds _bg to the end of the WASM filename, and we want to maintain compatibility with it's default options
    // we add _bg to the wasm files if cargo-leptos doesn't set the env var LEPTOS_OUTPUT_NAME at compile time
    // Otherwise we need to add _bg because wasm_pack always does.
    let mut wasm_output_name = output_name.clone();
    if std::option_env!("LEPTOS_OUTPUT_NAME").is_none() {
        wasm_output_name.push_str("_bg");
    }

    let leptos_autoreload = autoreload(&nonce, options);

    let html_metadata =
        meta.and_then(|mc| mc.html.as_string()).unwrap_or_default();
    let head = meta
        .as_ref()
        .map(|meta| meta.dehydrate())
        .unwrap_or_default();
    let import_callback = if cfg!(feature = "experimental-islands") {
        /* r#"() => {
          for (let e of document.querySelectorAll("leptos-island")) {
            let l = e.dataset.component;
            console.log("hydrating island");
            mod["_island_" + l];
          }
          mod.hydrate();
        }"# */
        r#"() => {       
            for (let e of document.querySelectorAll("leptos-island")) {
                let l = e.dataset.component;
                mod["_island_" + l](e);
            }
            mod.hydrate();
        }
        "#
        //r#"()=>{for(let e of document.querySelectorAll("leptos-island")){let l=e.dataset.component;mod["_island_"+l](e)};mod.hydrate();}"#
    } else {
        "() => mod.hydrate()"
    };

    let (js_hash, wasm_hash, css_hash) = get_hashes(options);

    let head = head.replace(
        &format!("{output_name}.css"),
        &format!("{output_name}{css_hash}.css"),
    );

    let head = format!(
        r#"<!DOCTYPE html>
            <html{html_metadata}>
                <head>
                    <meta charset="utf-8"/>
                    <meta name="viewport" content="width=device-width, initial-scale=1"/>
                    {head}
                    <link rel="modulepreload" href="{pkg_path}/{output_name}{js_hash}.js"{nonce}>
                    <link rel="preload" href="{pkg_path}/{wasm_output_name}{wasm_hash}.wasm" as="fetch" type="application/wasm" crossorigin=""{nonce}>
                    <script type="module"{nonce}>
                        function idle(c) {{
                            if ("requestIdleCallback" in window) {{
                                window.requestIdleCallback(c);
                            }} else {{
                                c();
                            }}
                        }}
                        idle(() => {{
                            import('{pkg_path}/{output_name}{js_hash}.js')
                                .then(mod => {{
                                    mod.default('{pkg_path}/{wasm_output_name}{wasm_hash}.wasm').then({import_callback});
                                }})
                        }});
                    </script>
                    {leptos_autoreload}
                </head>"#
    );
    let tail = "</body></html>";
    (head, tail)
}

#[tracing::instrument(level = "trace", fields(error), skip_all)]
fn get_hashes(options: &LeptosOptions) -> (String, String, String) {
    let mut ext_to_hash = HashMap::from([
        ("js".to_string(), "".to_string()),
        ("wasm".to_string(), "".to_string()),
        ("css".to_string(), "".to_string()),
    ]);

    if options.hash_files {
        let hash_path = env::current_exe()
            .map(|path| {
                path.parent().map(|p| p.to_path_buf()).unwrap_or_default()
            })
            .unwrap_or_default()
            .join(&options.hash_file);

        if hash_path.exists() {
            let hashes = fs::read_to_string(&hash_path)
                .expect("failed to read hash file");
            for line in hashes.lines() {
                let line = line.trim();
                if !line.is_empty() {
                    if let Some((k, v)) = line.split_once(':') {
                        ext_to_hash.insert(
                            k.trim().to_string(),
                            format!(".{}", v.trim()),
                        );
                    }
                }
            }
        }
    }

    (
        ext_to_hash["js"].clone(),
        ext_to_hash["wasm"].clone(),
        ext_to_hash["css"].clone(),
    )
}

#[tracing::instrument(level = "trace", fields(error), skip_all)]
pub async fn build_async_response(
    stream: impl Stream<Item = String> + 'static,
    options: &LeptosOptions,
    runtime: RuntimeId,
) -> String {
    let mut buf = String::new();
    let mut stream = Box::pin(stream);
    while let Some(chunk) = stream.next().await {
        buf.push_str(&chunk);
    }

    let (head, tail) =
        html_parts_separated(options, use_context::<MetaContext>().as_ref());

    // in async, we load the meta content *now*, after the suspenses have resolved
    let meta = use_context::<MetaContext>();
    let body_meta = meta
        .as_ref()
        .and_then(|meta| meta.body.as_string())
        .unwrap_or_default();

    runtime.dispose();

    format!("{head}<body{body_meta}>{buf}{tail}")
}
