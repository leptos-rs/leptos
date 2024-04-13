pub mod error_template;
pub mod errors;
#[cfg(feature = "ssr")]
pub mod fallback;
pub mod todo;

#[cfg(feature = "hydrate")]
#[wasm_bindgen::prelude::wasm_bindgen]
pub fn hydrate() {
    use crate::todo::TodoApp;
    use tracing_subscriber::fmt;
    use tracing_subscriber_wasm::MakeConsoleWriter;

    fmt()
        .with_writer(
            // To avoide trace events in the browser from showing their
            // JS backtrace, which is very annoying, in my opinion
            MakeConsoleWriter::default()
                .map_trace_level_to(tracing::Level::DEBUG),
        )
        // For some reason, if we don't do this in the browser, we get
        // a runtime error.
        .without_time()
        .init();
    _ = console_log::init_with_level(log::Level::Error);
    console_error_panic_hook::set_once();

    leptos::hydrate_body(TodoApp);
}
