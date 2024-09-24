#![forbid(unsafe_code)]

//! A simplification of Leptos Server Functions inventory.
//! Since it's not possible in Rust to use most crates relying on
//! linker sections when targeting `wasm32` (e.g. `inventory` crate),
//! and since our component's lifetime is bound to the one of the
//! incoming request, we can simplificate the codebase a lot.

use leptos::server_fn::ServerFnTraitObj;

use crate::{WasiRequest, WasiResponse};

pub enum Matcher {
    Found(ServerFnTraitObj<WasiRequest, WasiResponse>),
    NotFound,
}
