use crate::{
    animation::*,
    matching::{
        expand_optionals, get_route_matches, join_paths, Branch, Matcher,
        RouteDefinition, RouteMatch,
    },
    use_is_back_navigation, RouteContext, RouterContext, SetIsRouting,
};
use leptos::{leptos_dom::HydrationCtx, *};
use std::{
    cell::{Cell, RefCell},
    cmp::Reverse,
    collections::HashMap,
    ops::IndexMut,
    rc::Rc,
};

/// Contains route definitions and manages the actual routing process.
///
/// You should locate the `<Routes/>` component wherever on the page you want the routes to appear.
///
/// **Note:** Your application should only include one `<Routes/>` or `<AnimatedRoutes/>` component.
#[cfg_attr(
    any(debug_assertions, feature = "ssr"),
    tracing::instrument(level = "info", skip_all,)
)]
#[component]
pub fn Routes(
    cx: Scope,
    /// Base path relative at which the routes are mounted.
    #[prop(optional)]
    base: Option<String>,
    children: Children,
) -> impl IntoView {
    let router = use_context::<RouterContext>(cx)
        .expect("<Routes/> component should be nested within a <Router/>.");

    let base_route = router.base();
    let base = base.unwrap_or_default();

    Branches::initialize(&base, children(cx));

    #[cfg(feature = "ssr")]
    if let Some(context) = use_context::<crate::PossibleBranchContext>(cx) {
        Branches::with(&base, |branches| {
            *context.0.borrow_mut() = branches.to_vec()
        });
    }

    let next_route = router.pathname();
    let current_route = next_route;

    let root_equal = Rc::new(Cell::new(true));
    let route_states =
        route_states(cx, base, &router, current_route, &root_equal);

    let id = HydrationCtx::id();
    let root = root_route(cx, base_route, route_states, root_equal);

    leptos::leptos_dom::DynChild::new_with_id(id, move || root.get())
        .into_view(cx)
}

/// Contains route definitions and manages the actual routing process, with animated transitions
/// between routes.
///
/// You should locate the `<AnimatedRoutes/>` component wherever on the page you want the routes to appear.
///
/// ## Animations
/// The router uses CSS classes for animations, and transitions to the next specified class in order when
/// the `animationend` event fires. Each property takes a `&'static str` that can contain a class or classes
/// to be added at certain points. These CSS classes must have associated animations.
/// - `outro`: added when route is being unmounted
/// - `start`: added when route is first created
/// - `intro`: added after `start` has completed (if defined), and the route is being mounted
/// - `finally`: added after the `intro` animation is complete
///
/// Each of these properties is optional, and the router will transition to the next correct state
/// whenever an `animationend` event fires.
///
/// **Note:** Your application should only include one `<AnimatedRoutes/>` or `<Routes/>` component.
#[component]
pub fn AnimatedRoutes(
    cx: Scope,
    /// Base classes to be applied to the `<div>` wrapping the routes during any animation state.
    #[prop(optional, into)]
    class: Option<TextProp>,
    /// Base path relative at which the routes are mounted.
    #[prop(optional)]
    base: Option<String>,
    /// CSS class added when route is being unmounted
    #[prop(optional)]
    outro: Option<&'static str>,
    /// CSS class added when route is being unmounted, in a “back” navigation
    #[prop(optional)]
    outro_back: Option<&'static str>,
    /// CSS class added when route is first created
    #[prop(optional)]
    start: Option<&'static str>,
    /// CSS class added while the route is being mounted
    #[prop(optional)]
    intro: Option<&'static str>,
    /// CSS class added while the route is being mounted, in a “back” navigation
    #[prop(optional)]
    intro_back: Option<&'static str>,
    /// CSS class added after other animations have completed.
    #[prop(optional)]
    finally: Option<&'static str>,
    children: Children,
) -> impl IntoView {
    let router = use_context::<RouterContext>(cx)
        .expect("<Routes/> component should be nested within a <Router/>.");

    let base_route = router.base();
    let base = base.unwrap_or_default();

    Branches::initialize(&base, children(cx));

    #[cfg(feature = "ssr")]
    if let Some(context) = use_context::<crate::PossibleBranchContext>(cx) {
        Branches::with(&base, |branches| {
            *context.0.borrow_mut() = branches.to_vec()
        });
    }

    let animation = Animation {
        outro,
        start,
        intro,
        finally,
        outro_back,
        intro_back,
    };
    let is_back = use_is_back_navigation(cx);
    let (animation_state, set_animation_state) =
        create_signal(cx, AnimationState::Finally);
    let next_route = router.pathname();

    let is_complete = Rc::new(Cell::new(true));
    let animation_and_route = create_memo(cx, {
        let is_complete = Rc::clone(&is_complete);
        let base = base.clone();

        move |prev: Option<&(AnimationState, String)>| {
            let animation_state = animation_state.get();
            let next_route = next_route.get();
            let prev_matches = prev
                .map(|(_, r)| r)
                .cloned()
                .map(|location| get_route_matches(&base, location));
            let matches = get_route_matches(&base, next_route.clone());
            let same_route = prev_matches
                .and_then(|p| p.get(0).as_ref().map(|r| r.route.key.clone()))
                == matches.get(0).as_ref().map(|r| r.route.key.clone());
            if same_route {
                (animation_state, next_route)
            } else {
                match prev {
                    None => (animation_state, next_route),
                    Some((prev_state, prev_route)) => {
                        let (next_state, can_advance) = animation
                            .next_state(prev_state, is_back.get_untracked());

                        if can_advance || !is_complete.get() {
                            (next_state, next_route)
                        } else {
                            (next_state, prev_route.to_owned())
                        }
                    }
                }
            }
        }
    });
    let current_animation =
        create_memo(cx, move |_| animation_and_route.get().0);
    let current_route = create_memo(cx, move |_| animation_and_route.get().1);

    let root_equal = Rc::new(Cell::new(true));
    let route_states =
        route_states(cx, base, &router, current_route, &root_equal);

    let root = root_route(cx, base_route, route_states, root_equal);
    let node_ref = create_node_ref::<html::Div>(cx);

    html::div(cx)
        .node_ref(node_ref)
        .attr(
            "class",
            (cx, move || {
                let animation_class = match current_animation.get() {
                    AnimationState::Outro => outro.unwrap_or_default(),
                    AnimationState::Start => start.unwrap_or_default(),
                    AnimationState::Intro => intro.unwrap_or_default(),
                    AnimationState::Finally => finally.unwrap_or_default(),
                    AnimationState::OutroBack => outro_back.unwrap_or_default(),
                    AnimationState::IntroBack => intro_back.unwrap_or_default(),
                };
                is_complete.set(animation_class == finally.unwrap_or_default());
                if let Some(class) = &class {
                    format!("{} {animation_class}", class.get())
                } else {
                    animation_class.to_string()
                }
            }),
        )
        .on(leptos::ev::animationend, move |ev| {
            use wasm_bindgen::JsCast;
            if let Some(target) = ev.target() {
                if target
                    .unchecked_ref::<web_sys::Node>()
                    .is_same_node(Some(&*node_ref.get().unwrap()))
                {
                    let current = current_animation.get();
                    set_animation_state.update(|current_state| {
                        let (next, _) = animation
                            .next_state(&current, is_back.get_untracked());
                        *current_state = next;
                    })
                }
            }
        })
        .child(move || root.get())
        .into_view(cx)
}

pub(crate) struct Branches;

thread_local! {
    static BRANCHES: RefCell<HashMap<String, Vec<Branch>>> = RefCell::new(HashMap::new());
}

impl Branches {
    pub fn initialize(base: &str, children: Fragment) {
        BRANCHES.with(|branches| {
            let mut current = branches.borrow_mut();
            if !current.contains_key(base) {
                let mut branches = Vec::new();
                let children = children
                    .as_children()
                    .iter()
                    .filter_map(|child| {
                        let def = child
                            .as_transparent()
                            .and_then(|t| t.downcast_ref::<RouteDefinition>());
                        if def.is_none() {
                            warn!(
                                "[NOTE] The <Routes/> component should \
                                 include *only* <Route/>or <ProtectedRoute/> \
                                 components, or some \
                                 #[component(transparent)] that returns a \
                                 RouteDefinition."
                            );
                        }
                        def
                    })
                    .cloned()
                    .collect::<Vec<_>>();
                create_branches(
                    &children,
                    base,
                    &mut Vec::new(),
                    &mut branches,
                );
                current.insert(base.to_string(), branches);
            }
        })
    }

    pub fn with<T>(base: &str, cb: impl FnOnce(&[Branch]) -> T) -> T {
        BRANCHES.with(|branches| {
            let branches = branches.borrow();
            let branches = branches.get(base).expect(
                "Branches::initialize() should be called before \
                 Branches::with()",
            );
            cb(branches)
        })
    }
}

fn route_states(
    cx: Scope,
    base: String,
    router: &RouterContext,
    current_route: Memo<String>,
    root_equal: &Rc<Cell<bool>>,
) -> Memo<RouterState> {
    // whenever path changes, update matches
    let matches =
        create_memo(cx, move |_| get_route_matches(&base, current_route.get()));

    // iterate over the new matches, reusing old routes when they are the same
    // and replacing them with new routes when they differ
    let next: Rc<RefCell<Vec<RouteContext>>> = Default::default();
    let router = Rc::clone(&router.inner);

    create_memo(cx, {
        let root_equal = Rc::clone(root_equal);
        move |prev: Option<&RouterState>| {
            root_equal.set(true);
            next.borrow_mut().clear();

            let next_matches = matches.get();
            let prev_matches = prev.as_ref().map(|p| &p.matches);
            let prev_routes = prev.as_ref().map(|p| &p.routes);

            // are the new route matches the same as the previous route matches so far?
            let mut equal = prev_matches
                .map(|prev_matches| next_matches.len() == prev_matches.len())
                .unwrap_or(false);

            for i in 0..next_matches.len() {
                let next = next.clone();
                let prev_match = prev_matches.and_then(|p| p.get(i));
                let next_match = next_matches.get(i).unwrap();

                match (prev_routes, prev_match) {
                    (Some(prev), Some(prev_match))
                        if next_match.route.key == prev_match.route.key
                            && next_match.route.id == prev_match.route.id =>
                    {
                        let prev_one = { prev.borrow()[i].clone() };
                        if next_match.path_match.path != prev_one.path() {
                            prev_one
                                .set_path(next_match.path_match.path.clone());
                        }
                        if i >= next.borrow().len() {
                            next.borrow_mut().push(prev_one);
                        } else {
                            *(next.borrow_mut().index_mut(i)) = prev_one;
                        }
                    }
                    _ => {
                        equal = false;
                        if i == 0 {
                            root_equal.set(false);
                        }

                        let next = next.clone();

                        let next = next.clone();
                        let next_ctx = RouteContext::new(
                            cx,
                            &RouterContext {
                                inner: Rc::clone(&router),
                            },
                            {
                                let next = next.clone();
                                move |cx| {
                                    if let Some(route_states) =
                                        use_context::<Memo<RouterState>>(cx)
                                    {
                                        route_states.with(|route_states| {
                                            let routes =
                                                route_states.routes.borrow();
                                            routes.get(i + 1).cloned()
                                        })
                                    } else {
                                        next.borrow().get(i + 1).cloned()
                                    }
                                }
                            },
                            move || matches.with(|m| m.get(i).cloned()),
                        );

                        if let Some(next_ctx) = next_ctx {
                            if next.borrow().len() > i + 1 {
                                next.borrow_mut()[i] = next_ctx;
                            } else {
                                next.borrow_mut().push(next_ctx);
                            }
                        }
                    }
                }
            }

            if let Some(prev) = &prev {
                if equal {
                    RouterState {
                        matches: next_matches.to_vec(),
                        routes: prev_routes.cloned().unwrap_or_default(),
                        root: prev.root.clone(),
                    }
                } else {
                    let root = next.borrow().get(0).cloned();
                    RouterState {
                        matches: next_matches.to_vec(),
                        routes: Rc::new(RefCell::new(next.borrow().to_vec())),
                        root,
                    }
                }
            } else {
                let root = next.borrow().get(0).cloned();
                RouterState {
                    matches: next_matches.to_vec(),
                    routes: Rc::new(RefCell::new(next.borrow().to_vec())),
                    root,
                }
            }
        }
    })
}

fn root_route(
    cx: Scope,
    base_route: RouteContext,
    route_states: Memo<RouterState>,
    root_equal: Rc<Cell<bool>>,
) -> Signal<Option<View>> {
    let root_cx = RefCell::new(None);

    let root_view = create_memo(cx, {
        let root_equal = Rc::clone(&root_equal);
        move |prev| {
            provide_context(cx, route_states);
            route_states.with(|state| {
                if state.routes.borrow().is_empty() {
                    Some(base_route.outlet(cx).into_view(cx))
                } else {
                    let root = state.routes.borrow();
                    let root = root.get(0);
                    if let Some(route) = root {
                        provide_context(cx, route.clone());
                    }

                    if prev.is_none() || !root_equal.get() {
                        let (root_view, _) = cx.run_child_scope(|cx| {
                            let prev_cx = std::mem::replace(
                                &mut *root_cx.borrow_mut(),
                                Some(cx),
                            );
                            if let Some(prev_cx) = prev_cx {
                                prev_cx.dispose();
                            }
                            root.as_ref()
                                .map(|route| route.outlet(cx).into_view(cx))
                        });
                        root_view
                    } else {
                        prev.cloned().unwrap()
                    }
                }
            })
        }
    });

    if cfg!(any(feature = "csr", feature = "hydrate"))
        && use_context::<SetIsRouting>(cx).is_some()
    {
        let global_suspense = expect_context::<GlobalSuspenseContext>(cx);

        let (current_view, set_current_view) = create_signal(cx, None);

        create_effect(cx, move |prev| {
            let root = root_view.get();
            let is_fallback =
                !global_suspense.with_inner(SuspenseContext::ready);
            if prev.is_none() {
                set_current_view.set(root);
            } else if !is_fallback {
                queue_microtask({
                    let global_suspense = global_suspense.clone();
                    move || {
                        let is_fallback = cx.untrack(move || {
                            !global_suspense.with_inner(SuspenseContext::ready)
                        });
                        if !is_fallback {
                            set_current_view.set(root);
                        }
                    }
                });
            }
        });
        current_view.into()
    } else {
        root_view.into()
    }
}

#[derive(Clone, Debug, PartialEq)]
struct RouterState {
    matches: Vec<RouteMatch>,
    routes: Rc<RefCell<Vec<RouteContext>>>,
    root: Option<RouteContext>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct RouteData {
    pub id: usize,
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
        #[allow(clippy::bool_to_int_with_if)] // on the splat.is_none()
        segments.iter().fold(
            (segments.len() as i32) - if splat.is_none() { 0 } else { 1 },
            |score, segment| {
                score + if segment.starts_with(':') { 2 } else { 3 }
            },
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
#[cfg_attr(
    any(debug_assertions, feature = "ssr"),
    tracing::instrument(level = "info", skip_all,)
)]
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
            id: route_def.id,
            matcher: Matcher::new_with_partial(&pattern, !is_leaf),
            pattern,
            original_path: original_path.to_string(),
        });
    }
    acc
}
