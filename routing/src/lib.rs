#![forbid(unsafe_code)]
#![cfg_attr(feature = "nightly", feature(auto_traits))]
#![cfg_attr(feature = "nightly", feature(negative_impls))]

pub mod components;
mod generate_route_list;
pub mod hooks;
pub mod location;
mod matching;
mod method;
mod nested_router;
pub mod params;
//mod router;
mod ssr_mode;
mod static_route;

pub use generate_route_list::*;
pub use matching::*;
pub use method::*;
pub use nested_router::*;
//pub use router::*;
pub use ssr_mode::*;
pub use static_route::*;
