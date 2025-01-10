use super::{PartialPathMatch, PathSegment};
use std::sync::Arc;
mod param_segments;
mod static_segment;
mod tuples;
pub use param_segments::*;
pub use static_segment::*;

/// Defines a route which may or may not be matched by any given URL,
/// or URL segment.
///
/// This is a "horizontal" matching: i.e., it treats a tuple of route segments
/// as subsequent segments of the URL and tries to match them all.
pub trait PossibleRouteMatch {
    fn optional(&self) -> bool;

    fn test<'a>(&self, path: &'a str) -> Option<PartialPathMatch<'a>>;

    fn generate_path(&self, path: &mut Vec<PathSegment>);
}

impl PossibleRouteMatch for Box<dyn PossibleRouteMatch + Send + Sync> {
    fn optional(&self) -> bool {
        (**self).optional()
    }

    fn test<'a>(&self, path: &'a str) -> Option<PartialPathMatch<'a>> {
        (**self).test(path)
    }

    fn generate_path(&self, path: &mut Vec<PathSegment>) {
        (**self).generate_path(path);
    }
}

impl PossibleRouteMatch for Arc<dyn PossibleRouteMatch + Send + Sync> {
    fn optional(&self) -> bool {
        (**self).optional()
    }

    fn test<'a>(&self, path: &'a str) -> Option<PartialPathMatch<'a>> {
        (**self).test(path)
    }

    fn generate_path(&self, path: &mut Vec<PathSegment>) {
        (**self).generate_path(path);
    }
}
