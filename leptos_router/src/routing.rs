use std::{any::Any, borrow::Cow, rc::Rc};

use leptos_dom::{Child, IntoChild};
use leptos_reactive::{create_memo, use_context, ReadSignal, Scope};

use crate::{
    expand_optionals, join_paths, Location, Matcher, PathMatch, RouteContext, RouteDefinition,
    RouterContext, State, Url,
};

pub fn use_router(cx: Scope) -> RouterContext {
    use_context(cx).expect("You must call use_router() within a <Router/> component")
}

pub fn use_route(cx: Scope) -> RouteContext {
    use_context(cx).unwrap_or(use_router(cx).inner.base.clone())
}

pub fn create_location(cx: Scope, path: ReadSignal<String>, state: ReadSignal<State>) -> Location {
    let url = create_memo(cx, move |prev: Option<&Url>| {
        path.with(|path| {
            log::debug!("create_location with path {path}");
            match Url::try_from(path.as_str()) {
                Ok(url) => url,
                Err(e) => {
                    log::error!("[Leptos Router] Invalid path {path}\n\n{e:?}");
                    prev.unwrap().clone()
                }
            }
        })
    });

    let path_name = create_memo(cx, move |_| url.with(|url| url.path_name.clone()));
    let search = create_memo(cx, move |_| url.with(|url| url.search.clone()));
    let hash = create_memo(cx, move |_| url.with(|url| url.hash.clone()));
    let query = create_memo(cx, move |_| url.with(|url| url.search_params()));

    Location {
        path_name,
        search,
        hash,
        query,
    }
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Route {
    pub key: RouteDefinition,
    pub pattern: String,
    pub original_path: String,
    pub matcher: Matcher,
}

pub(crate) fn create_branches(
    route_defs: &[RouteDefinition],
    base: &str,
    fallback: Option<Child>,
) -> Vec<Branch> {
    let mut branches = Vec::new();
    create_branches_helper(route_defs, base, fallback, &mut Vec::new(), &mut branches);
    branches
}

pub(crate) fn create_branches_helper(
    route_defs: &[RouteDefinition],
    base: &str,
    fallback: Option<Child>,
    stack: &mut Vec<Route>,
    branches: &mut Vec<Branch>,
) {
    for def in route_defs {
        let routes = create_routes(def, base, fallback.clone());
        for route in routes {
            stack.push(route.clone());

            if def.children.is_empty() {
                let branch = create_branch(&stack, branches.len());
                branches.push(branch);
            } else {
                create_branches_helper(
                    &def.children,
                    &route.pattern,
                    fallback.clone(),
                    stack,
                    branches,
                );
            }

            stack.pop();
        }
    }

    if stack.is_empty() {
        branches.sort_by_key(|branch| branch.score);
    }
}

pub(crate) fn create_routes(
    route_def: &RouteDefinition,
    base: &str,
    fallback: Option<impl IntoChild>,
) -> Vec<Route> {
    let RouteDefinition {
        data,
        children,
        component,
        ..
    } = route_def;
    let is_leaf = children.is_empty();
    route_def.path.iter().fold(Vec::new(), |mut acc, path| {
        for original_path in expand_optionals(&path) {
            let path = join_paths(base, &original_path);
            let pattern = if is_leaf {
                path
            } else {
                path.split("/*")
                    .next()
                    .map(|n| n.to_string())
                    .unwrap_or(path)
            };
            acc.push(Route {
                key: route_def.clone(),
                matcher: Matcher::new_with_partial(&pattern, !is_leaf),
                pattern,
                original_path: original_path.to_string(),
            })
        }
        acc
    })
}

pub(crate) fn create_branch(routes: &[Route], index: usize) -> Branch {
    Branch {
        routes: routes.to_vec(),
        score: score_route(routes.last().unwrap()) * 10000 - index,
    }
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Branch {
    routes: Vec<Route>,
    score: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct RouteMatch {
    pub path_match: PathMatch,
    pub route: Route,
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

fn score_route(route: &Route) -> usize {
    let (pattern, splat) = match route.pattern.split_once("/*") {
        Some((p, s)) => (p, Some(s)),
        None => (route.pattern.as_str(), None),
    };
    let segments = pattern
        .split('/')
        .filter(|n| !n.is_empty())
        .collect::<Vec<_>>();
    segments.iter().fold(
        segments.len() - if splat.is_none() { 0 } else { 1 },
        |score, segment| score + if segment.starts_with(':') { 2 } else { 3 },
    )
}

pub(crate) fn get_route_matches(branches: Vec<Branch>, location: String) -> Vec<RouteMatch> {
    for branch in branches {
        if let Some(matches) = branch.matcher(&location) {
            return matches;
        }
    }
    vec![]
}
