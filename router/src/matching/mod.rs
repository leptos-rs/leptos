mod expand_optionals;
mod matcher;
mod resolve_path;
mod route;

pub use expand_optionals::*;
pub use matcher::*;
pub use resolve_path::*;
pub use route::*;

use crate::RouteData;

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct RouteMatch {
    pub path_match: PathMatch,
    pub route: RouteData,
}

pub(crate) fn get_route_matches(branches: Vec<Branch>, location: String) -> Vec<RouteMatch> {
    for branch in branches {
        if let Some(matches) = branch.matcher(&location) {
            return matches;
        }
    }
    vec![]
}

/// Describes a branch of the route tree.
#[derive(Debug, Clone, PartialEq)]
pub struct Branch {
    /// All the routes contained in the branch.
    pub routes: Vec<RouteData>,
    /// How closely this branch matches the current URL.
    pub score: i32,
}

impl Branch {
    fn matcher<'a>(&'a self, location: &'a str) -> Option<Vec<RouteMatch>> {
        let mut matches = Vec::new();
        for route in self.routes.iter().rev() {
            match route.matcher.test(location) {
                None => return None,
                Some(m) => matches.push(RouteMatch {
                    path_match: m,
                    route: route.clone(),
                }),
            }
        }
        matches.reverse();
        Some(matches)
    }
}
