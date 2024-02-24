use crate::PathSegment;
use alloc::vec::Vec;
use core::str::Chars;

mod param_segments;
mod static_segment;
mod tuples;
use super::PartialPathMatch;
pub use param_segments::*;
pub use static_segment::*;

/// Defines a route which may or may not be matched by any given URL,
/// or URL segment.
///
/// This is a "horizontal" matching: i.e., it treats a tuple of route segments
/// as subsequent segments of the URL and tries to match them all. For a "vertical"
/// matching that sees a tuple as alternatives to one another, see [`RouteChild`](super::RouteChild).
pub trait PossibleRouteMatch {
    fn matches(&self, path: &str) -> bool {
        self.matches_iter(&mut path.chars())
    }

    fn matches_iter(&self, path: &mut Chars) -> bool;

    fn test<'a>(&self, path: &'a str) -> Option<PartialPathMatch<'a>>;

    fn generate_path(&self, path: &mut Vec<PathSegment>);
}
