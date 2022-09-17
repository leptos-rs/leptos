use std::{cmp::Reverse, rc::Rc};

use leptos_core::IntoVec;
use leptos_dom::{Child, IntoChild};
use leptos_reactive::Scope;
use typed_builder::TypedBuilder;

use crate::matching::{expand_optionals, join_paths, Branch, Matcher, RouteDefinition};

#[derive(TypedBuilder)]
pub struct RoutesProps {
    #[builder(default, setter(strip_option))]
    base: Option<String>,
    children: Box<dyn Fn() -> Vec<RouteDefinition>>,
}

#[allow(non_snake_case)]
pub fn Routes(_cx: Scope, props: RoutesProps) -> Vec<Branch> {
    let mut branches = Vec::new();

    create_branches(
        &(props.children)(),
        &props.base.unwrap_or_default(),
        &mut Vec::new(),
        &mut branches,
    );

    branches
}

#[derive(Debug, Clone, PartialEq)]
pub struct RouteData {
    pub key: RouteDefinition,
    pub pattern: String,
    pub original_path: String,
    pub matcher: Matcher,
}

impl RouteData {
    fn score(&self) -> i32 {
        let (pattern, splat) = match self.pattern.split_once("/*") {
            Some((p, s)) => (p, Some(s)),
            None => (self.pattern.as_str(), None),
        };
        let segments = pattern
            .split('/')
            .filter(|n| !n.is_empty())
            .collect::<Vec<_>>();
        segments.iter().fold(
            (segments.len() as i32) - if splat.is_none() { 0 } else { 1 },
            |score, segment| score + if segment.starts_with(':') { 2 } else { 3 },
        )
    }
}

fn create_branches(
    route_defs: &[RouteDefinition],
    base: &str,
    stack: &mut Vec<RouteData>,
    branches: &mut Vec<Branch>,
) {
    for def in route_defs {
        let routes = create_routes(def, base);
        for route in routes {
            stack.push(route.clone());

            if def.children.is_empty() {
                let branch = create_branch(stack, branches.len());
                branches.push(branch);
            } else {
                create_branches(&def.children, &route.pattern, stack, branches);
            }

            stack.pop();
        }
    }

    if stack.is_empty() {
        branches.sort_by_key(|branch| Reverse(branch.score));
    }
}

pub(crate) fn create_branch(routes: &[RouteData], index: usize) -> Branch {
    Branch {
        routes: routes.to_vec(),
        score: routes.last().unwrap().score() * 10000 - (index as i32),
    }
}

fn create_routes(route_def: &RouteDefinition, base: &str) -> Vec<RouteData> {
    let RouteDefinition { children, .. } = route_def;
    let is_leaf = children.is_empty();
    let mut acc = Vec::new();
    for original_path in expand_optionals(&route_def.path) {
        let path = join_paths(base, &original_path);
        let pattern = if is_leaf {
            path
        } else {
            path.split("/*")
                .next()
                .map(|n| n.to_string())
                .unwrap_or(path)
        };
        acc.push(RouteData {
            key: route_def.clone(),
            matcher: Matcher::new_with_partial(&pattern, !is_leaf),
            pattern,
            original_path: original_path.to_string(),
        });
    }
    acc
}
