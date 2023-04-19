mod expand_optionals;
mod matcher;
mod resolve_path;
mod route;

use crate::{Branches, RouteData};
pub use expand_optionals::*;
pub use matcher::*;
pub use resolve_path::*;
pub use route::*;

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct RouteMatch {
    pub path_match: PathMatch,
    pub route: RouteData,
}

use lru::LruCache;
use std::{cell::RefCell, num::NonZeroUsize, rc::Rc};
thread_local! {
    static ROUTE_MATCH_CACHE: RefCell<LruCache<String, Rc<Vec<RouteMatch>>>> = RefCell::new(LruCache::new(NonZeroUsize::new(32).unwrap()));
}

pub(crate) fn get_route_matches(location: String) -> Rc<Vec<RouteMatch>> {
    ROUTE_MATCH_CACHE.with(|cache| {
        let mut cache = cache.borrow_mut();
        Rc::clone(cache.get_or_insert(location.clone(), || {
            Rc::new(Branches::with(|branches| {
                for branch in branches {
                    if let Some(matches) = branch.matcher(&location) {
                        return matches;
                    }
                }
                vec![]
            }))
        }))
    })
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
