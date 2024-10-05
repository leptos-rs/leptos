//! A first-party support of the `wasm32-wasip1` target for the **Server-Side**
//! of Leptos using the [`wasi:http`][wasi-http] proposal.
//! 
//! [wasi-http]: https://github.com/WebAssembly/wasi-http
//! 
//! # `Handler`
//! 
//! The [`prelude::Handler`] is the main abstraction you will use.
//! 
//! It expects being run in the context of a Future Executor `Task`,
//! since WASI is, at the moment, a single-threaded environment,
//! we provide a simple abstraction in the form of [`leptos::spawn::Executor`]
//! that you can leverage to use this crate.
//! 
//! ```
//! use leptos_wasi::{bindings::exports::wasi::http::incoming_handler::Guest, prelude::{IncomingRequest, ResponseOutparam}};
//! 
//! struct LeptosServer;
//!
//! // NB(raskyld): for now, the types to use for the HTTP handlers are the one from
//! // the `leptos_wasi` crate, not the one generated in your crate.
//! impl Guest for LeptosServer {
//!     fn handle(request: IncomingRequest, response_out: ResponseOutparam) {
//!         // Initiate a single-threaded [`Future`] Executor so we can run the
//!         // rendering system and take advantage of bodies streaming.
//!         Executor::init_futures_local_executor().expect("cannot init future executor");
//!         Executor::spawn(async {
//!             // declare an async function called `handle_request` and
//!             // use the Handler in this function.
//!             handle_request(request, response_out).await;
//!         });
//!         Executor::run();
//!     }
//! }
//! ```
//! 
//! # WASI Bindings
//! 
//! You are free to use any WIT imports and export any WIT exports but at the moment,
//! when interacting with this crate, you must use the types that you can find in
//! this crate [`bindings`].
//! 
//! You then need to export your implementation using:
//! 
//! ```
//! export!(LeptosServer with_types_in leptos_wasi::bindings);
//! ```
//! 
//! If you want to use your own bindings for `wasi:http`,
//! then you need to implement `From` traits
//! to convert your own bindings into the one in [`bindings`].
//! Please, note that it will likely implies doing `unsafe`
//! operations to wrap the resource's `handle() -> u64` in
//! another type.

pub mod bindings {
    wit_bindgen::generate!({
        path: "wit",
        pub_export_macro: true,
        world: "http",
        generate_all,
    });
}

pub mod request;
pub mod handler;
pub mod response;
pub mod utils;

pub mod prelude {
    pub use crate::utils::redirect;
    pub use crate::handler::Handler;
    pub use crate::bindings::exports::wasi::http::incoming_handler::{IncomingRequest, ResponseOutparam};
    pub use crate::response::Body;
}

/// When working with streams, this crate will try to chunk bytes with
/// this size.
const CHUNK_BYTE_SIZE: u64 = 64;

