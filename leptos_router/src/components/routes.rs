use std::rc::Rc;

use leptos_dom::*;
use leptos_dom::wasm_bindgen::JsCast;
use leptos_macro::{view, Props};
use leptos_reactive::{create_memo, Scope, ScopeDisposer, create_effect, provide_context};
use leptos_core as leptos;

use crate::{
    create_branches, get_route_matches, join_paths, use_route, use_router, Outlet, RouteContext,
    RouteDefinition, RouteMatch, RouteContextInner, create_route_context,
};

#[derive(Props)]
pub struct RoutesProps {
    base: Option<String>,
    children: Vec<RouteDefinition>,
}

#[allow(non_snake_case)]
pub fn Routes(cx: Scope, props: RoutesProps) -> Element {
    let router = use_router(cx);

    let parent_route = use_route(cx);

    let route_defs = props.children;

    let branches = create_branches(
        &route_defs,
        &join_paths(&parent_route.inner.path, &props.base.unwrap_or_default()),
        Some((move || Outlet(cx)).into_child(cx)),
    );

    let path_name = router.inner.location.path_name;
    let matches = create_memo(cx, move |_| {
        get_route_matches(branches.clone(), path_name.get())
    });

    // TODO router.out (for SSR)

    let mut disposers: Vec<ScopeDisposer> = Vec::new();

    let route_states = create_memo(
        cx,
        move |prev: Option<&(Vec<RouteMatch>, Vec<RouteContext>, Option<RouteContext>)>| {
            
            let (prev_matches, prev, root) = match prev {
                Some((a, b, c)) => (Some(a), Some(b), c),
                None => (None, None, &None),
            };
            let next_matches = matches.get();
            let mut equal = prev_matches
                .map(|prev_matches| next_matches.len() == prev_matches.len())
                .unwrap_or(false);

            let mut next: Vec<RouteContext> = Vec::new();

            for i in 0..next_matches.len() {
                let prev_match = prev_matches.and_then(|p| p.get(i));
                let next_match = next_matches.get(i).unwrap();

                if let Some(prev) = prev && let Some(prev_match) = prev_match && next_match.route.key == prev_match.route.key {
                    next[i] = prev[i].clone();
                } else {
                    equal = false; 
                    if let Some(disposer) = disposers.get(i) {
                        // TODO
                        //disposer.dispose();
                    }


                    let disposer = cx.child_scope(|cx| {
                        let possible_parent = if i == 0 {
                            None
                        } else {
                            next.get(i - 1)
                        };

                        let next_ctx = create_route_context(&router, possible_parent.unwrap_or(&parent_route), { let c = next.get(i + 1).cloned(); move || c.clone()}, move || matches().get(i).cloned().unwrap());
                        
                        if next.len() > i + 1 {
                        next[i] = next_ctx;
                        } else {
                            next.push(next_ctx);
                        }
                    });

                    if disposers.len() > i + 1 {
                        disposers[i] = disposer;
                    } else {
                        disposers.push(disposer);
                    }
                }
            }

            if let Some(prev) = prev && equal {
                (next_matches, prev.to_vec(), root.clone())
            } else {
                let root = next.get(0).cloned();
                (next_matches, next, root)
            }
        },
    );

    todo!()

    /* let outlet = view! { <div></div> };
    leptos_dom::insert(
        cx,
        outlet.clone().unchecked_into(),
        (move || route_states.with(|(_, _, route)| {
        log::debug!("new route {route:#?}");
        if let Some(route) = route {
        provide_context(cx, route.clone());
        }
        route.as_ref().map(|route| route.outlet())
        })).into_child(cx),
        None,
        None
    );
    outlet */
    
}
