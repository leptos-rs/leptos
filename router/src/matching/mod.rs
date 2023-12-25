mod expand_optionals;
mod matcher;
mod resolve_path;
mod route;

use crate::{Branches, RouteData};
pub use expand_optionals::*;
pub use matcher::*;
pub use resolve_path::*;
pub use route::*;
use std::rc::Rc;
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct RouteMatch {
    pub path_match: PathMatch,
    pub route: RouteData,
}

pub(crate) fn get_route_matches(
    router_id: Uuid,
    base: &str,
    location: String,
) -> Rc<Vec<RouteMatch>> {
    #[cfg(feature = "ssr")]
    {
        use lru::LruCache;
        use std::{cell::RefCell, collections::HashMap, num::NonZeroUsize};
        type RouteMatchCache = LruCache<String, Rc<Vec<RouteMatch>>>;
        thread_local! {
            static ROUTE_MATCH_CACHES: RefCell<HashMap<Uuid , RouteMatchCache>> = RefCell::new(HashMap::new());
        }

        ROUTE_MATCH_CACHES.with(|caches| {
            let mut caches = caches.borrow_mut();
            caches.entry(router_id).or_insert_with(|| {
                LruCache::new(NonZeroUsize::new(32).unwrap())
            });
            let cache = caches.get_mut(&router_id).unwrap();
            Rc::clone(cache.get_or_insert(location.clone(), || {
                build_route_matches(router_id, base, location)
            }))
        })
    }

    #[cfg(not(feature = "ssr"))]
    build_route_matches(router_id, base, location)
}

fn build_route_matches(
    router_id: Uuid,
    base: &str,
    location: String,
) -> Rc<Vec<RouteMatch>> {
    Rc::new(Branches::with(router_id, base, |branches| {
        for branch in branches {
            if let Some(matches) = branch.matcher(&location) {
                return matches;
            }
        }
        vec![]
    }))
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
