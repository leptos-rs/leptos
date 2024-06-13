#![forbid(unsafe_code)]
#![cfg_attr(feature = "nightly", feature(auto_traits))]
#![cfg_attr(feature = "nightly", feature(negative_impls))]

pub mod components;
pub mod flat_router;
mod generate_route_list;
pub mod hooks;
mod link;
pub mod location;
mod matching;
mod method;
mod navigate;
pub mod nested_router;
pub mod params;
//mod router;
mod ssr_mode;
mod static_route;

pub use generate_route_list::*;
pub use matching::*;
pub use method::*;
pub use navigate::*;
//pub use router::*;
pub use ssr_mode::*;
pub use static_route::*;
