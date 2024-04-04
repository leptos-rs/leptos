use leptos::*;

#[cfg(feature = "ssr")]
pub mod fallback;

#[server(endpoint = "hello_world")]
pub async fn hello_world_server() -> Result<String, ServerFnError> {
    Ok("Hey.".to_string())
}

#[component]
pub fn App() -> impl IntoView {
    let action = create_server_action::<HelloWorldServer>();
    let vals = create_rw_signal(String::new());
    create_effect(move |_| {
        if let Some(resp) = action.value().get() {
            match resp {
                Ok(val) => vals.set(val),
                Err(err) => vals.set(format!("{err:?}")),
            }
        }
    });
    view! {<button
        on:click=move |_| {
            action.dispatch(HelloWorldServer{});
        }
        >"Hello world."</button>
        {
            move || vals.get()
        }
    }
}
cfg_if::cfg_if! {
    if #[cfg(feature = "hydrate")] {
        use wasm_bindgen::prelude::wasm_bindgen;

        #[wasm_bindgen]
        pub fn hydrate() {
            #[cfg(debug_assertions)]
            console_error_panic_hook::set_once();
            leptos::mount_to_body(App);
        }
    }
}