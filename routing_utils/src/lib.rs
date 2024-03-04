//#![no_std]

#[macro_use]
extern crate alloc;

pub mod location;
pub mod matching;
pub mod params;
mod path_segment;
pub use path_segment::*;
