#![forbid(unsafe_code)]
#![cfg_attr(feature = "nightly", feature(auto_traits))]
#![cfg_attr(feature = "nightly", feature(negative_impls))]

pub mod components;
pub mod flat_router;
mod form;
mod generate_route_list;
pub mod hooks;
mod link;
pub mod location;
mod matching;
mod method;
mod navigate;
pub mod nested_router;
pub mod params;
mod ssr_mode;
pub mod static_routes;

pub use generate_route_list::*;
#[doc(inline)]
pub use leptos_router_macro::path;
pub use matching::*;
pub use method::*;
pub use navigate::*;
pub use ssr_mode::*;
