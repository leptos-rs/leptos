use leptos::prelude::*;
use router::*;
use tracing_subscriber::fmt;
use tracing_subscriber_wasm::MakeConsoleWriter;

pub fn main() {
    fmt()
        .with_writer(
            MakeConsoleWriter::default()
                .map_trace_level_to(tracing::Level::DEBUG),
        )
        .without_time()
        .init();
    console_error_panic_hook::set_once();
    mount_to_body(RouterExample);
}
