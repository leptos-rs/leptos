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

    /// Checks if this segment matches beginning of the path
    ///
    ///
    /// # Arguments
    ///
    /// * path - unmatched reminder of the path.
    ///
    /// # Returns
    ///
    /// If segment doesn't match a path then returns `None`. In case of a match returns the
    /// information about which part of the path was matched.
    ///
    /// 1. Paths which are empty `""` or just `"/"` should match.
    /// 2. If you match just a path `"/"`, you should preserve the starting slash
    ///    in the [remaining](PartialPathMatch::remaining) part, so other segments which will be
    ///    tested can detect wherever they are matching from the beginning of the given path segment.
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
