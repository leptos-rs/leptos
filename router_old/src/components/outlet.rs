use crate::{
    animation::{Animation, AnimationState},
    use_is_back_navigation, use_location, use_route, RouteContext,
    SetIsRouting,
};
use leptos::{leptos_dom::HydrationCtx, *};
use std::{cell::Cell, rc::Rc};
use web_sys::AnimationEvent;

/// Displays the child route nested in a parent route, allowing you to control exactly where
/// that child route is displayed. Renders nothing if there is no nested child.
#[cfg_attr(
    any(debug_assertions, feature = "ssr"),
    tracing::instrument(level = "trace", skip_all,)
)]
#[component]
pub fn Outlet() -> impl IntoView {
    _ = HydrationCtx::next_outlet();
    let id = HydrationCtx::id();
    let route = use_route();
    let route_states = expect_context::<Memo<crate::RouterState>>();

    let child_id = create_memo({
        let route = route.clone();
        move |_| {
            route_states.track();
            route.child().map(|child| child.id())
        }
    });

    let is_showing = Rc::new(Cell::new(None::<usize>));
    let (outlet, set_outlet) = create_signal(None::<View>);
    let build_outlet = as_child_of_current_owner(|child: RouteContext| {
        provide_context(child.clone());
        child.outlet().into_view()
    });
    create_isomorphic_effect(move |prev_disposer| {
        child_id.track();
        match (route.child(), &is_showing.get()) {
            (None, _) => {
                set_outlet.set(None);

                // previous disposer will be dropped, and therefore disposed
                None
            }
            (Some(child), Some(is_showing_val))
                if child.id() == *is_showing_val =>
            {
                // do nothing: we don't need to rerender the component, because it's the same

                // returning the disposer keeps it alive until the next iteration
                prev_disposer.flatten()
            }
            (Some(child), _) => {
                drop(prev_disposer);
                is_showing.set(Some(child.id()));
                let (outlet, disposer) = build_outlet(child);
                set_outlet.set(Some(outlet));
                // returning the disposer keeps it alive until the next iteration
                Some(disposer)
            }
        }
    });

    let outlet: Signal<Option<View>> =
        if cfg!(any(feature = "csr", feature = "hydrate"))
            && use_context::<SetIsRouting>().is_some()
        {
            let global_suspense = expect_context::<GlobalSuspenseContext>();

            let (current_view, set_current_view) = create_signal(None);

            create_render_effect({
                move |prev| {
                    let outlet = outlet.get();
                    let is_fallback =
                        !global_suspense.with_inner(|c| c.ready().get());
                    if prev.is_none() {
                        set_current_view.set(outlet);
                    } else if !is_fallback {
                        queue_microtask({
                            let global_suspense = global_suspense.clone();
                            move || {
                                let is_fallback = untrack(move || {
                                    !global_suspense
                                        .with_inner(|c| c.ready().get())
                                });
                                if !is_fallback {
                                    set_current_view.set(outlet);
                                }
                            }
                        });
                    }
                }
            });
            current_view.into()
        } else {
            outlet.into()
        };

    leptos::leptos_dom::DynChild::new_with_id(id, move || outlet.get())
}

/// Displays the child route nested in a parent route, allowing you to control exactly where
/// that child route is displayed. Renders nothing if there is no nested child.
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
#[component]
pub fn AnimatedOutlet(
    /// Base classes to be applied to the `<div>` wrapping the outlet during any animation state.
    #[prop(optional, into)]
    class: Option<TextProp>,
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
) -> impl IntoView {
    let pathname = use_location().pathname;
    let route = use_route();
    let is_showing = Rc::new(Cell::new(None::<usize>));
    let (outlet, set_outlet) = create_signal(None::<View>);
    let build_outlet = as_child_of_current_owner(|child: RouteContext| {
        provide_context(child.clone());
        child.outlet().into_view()
    });

    let animation = Animation {
        outro,
        start,
        intro,
        finally,
        outro_back,
        intro_back,
    };
    let (animation_state, set_animation_state) =
        create_signal(AnimationState::Finally);
    let trigger_animation = create_rw_signal(());
    let is_back = use_is_back_navigation();
    let animation_and_outlet = create_memo({
        move |prev: Option<&(AnimationState, View)>| {
            let animation_state = animation_state.get();
            let next_outlet = outlet.get().unwrap_or_default();
            trigger_animation.track();
            match prev {
                None => (animation_state, next_outlet),
                Some((prev_state, prev_outlet)) => {
                    let (next_state, can_advance) = animation
                        .next_state(prev_state, is_back.get_untracked());

                    if can_advance {
                        (next_state, next_outlet)
                    } else {
                        (next_state, prev_outlet.to_owned())
                    }
                }
            }
        }
    });
    let current_animation = create_memo(move |_| animation_and_outlet.get().0);
    let current_outlet = create_memo(move |_| animation_and_outlet.get().1);

    create_isomorphic_effect(move |prev_disposer| {
        pathname.track();

        match (route.child(), &is_showing.get()) {
            (None, _) => {
                set_outlet.set(None);

                // previous disposer will be dropped, and therefore disposed
                None
            }
            (Some(child), Some(is_showing_val))
                if child.id() == *is_showing_val =>
            {
                trigger_animation.set(());

                // do nothing: we don't need to rerender the component, because it's the same
                // returning the disposer keeps it alive until the next iteration
                prev_disposer.flatten()
            }
            (Some(child), _) => {
                trigger_animation.set(());
                is_showing.set(Some(child.id()));
                let (outlet, disposer) = build_outlet(child);
                set_outlet.set(Some(outlet));
                // returning the disposer keeps it alive until the next iteration
                Some(disposer)
            }
        }
    });

    let class = move || {
        let animation_class = match current_animation.get() {
            AnimationState::Outro => outro.unwrap_or_default(),
            AnimationState::Start => start.unwrap_or_default(),
            AnimationState::Intro => intro.unwrap_or_default(),
            AnimationState::Finally => finally.unwrap_or_default(),
            AnimationState::OutroBack => outro_back.unwrap_or_default(),
            AnimationState::IntroBack => intro_back.unwrap_or_default(),
        };
        if let Some(class) = &class {
            format!("{} {animation_class}", class.get())
        } else {
            animation_class.to_string()
        }
    };
    let node_ref = create_node_ref::<html::Div>();
    let animationend = move |ev: AnimationEvent| {
        use wasm_bindgen::JsCast;
        if let Some(target) = ev.target() {
            let node_ref = node_ref.get();
            if node_ref.is_none()
                || target
                    .unchecked_ref::<web_sys::Node>()
                    .is_same_node(Some(&*node_ref.unwrap()))
            {
                ev.stop_propagation();
                let current = current_animation.get();
                set_animation_state.update(|current_state| {
                    let (next, _) =
                        animation.next_state(&current, is_back.get_untracked());
                    *current_state = next;
                });
            }
        }
    };

    view! {
        <div class=class on:animationend=animationend>
            {move || current_outlet.get()}
        </div>
    }
}

/*
/// Displays the child route nested in a parent route, allowing you to control exactly where
/// that child route is displayed. Renders nothing if there is no nested child.
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
#[component]
pub fn AnimatedOutlet(
    /// Base classes to be applied to the `<div>` wrapping the outlet during any animation state.
    #[prop(optional, into)]
    class: Option<TextProp>,
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
) -> impl IntoView {
    let route = use_route();
    let is_showing = Rc::new(Cell::new(None::<usize>));
    let (outlet, set_outlet) = create_signal(None::<View>);

    let animation = Animation {
        outro,
        start,
        intro,
        finally,
        outro_back,
        intro_back,
    };
    let (animation_state, set_animation_state) =
        create_signal(AnimationState::Finally);
    let trigger_animation = create_rw_signal(());
    let is_back = use_is_back_navigation();
    let animation_and_outlet = create_memo({
        move |prev: Option<&(AnimationState, View)>| {
            let animation_state = animation_state.get();
            let next_outlet = outlet.get().unwrap_or_default();
            trigger_animation.track();
            match prev {
                None => (animation_state, next_outlet),
                Some((prev_state, prev_outlet)) => {
                    let (next_state, can_advance) = animation
                        .next_state(prev_state, is_back.get_untracked());

                    if can_advance {
                        (next_state, next_outlet)
                    } else {
                        (next_state, prev_outlet.to_owned())
                    }
                }
            }
        }
    });
    let current_animation = create_memo(move |_| animation_and_outlet.get().0);
    let current_outlet = create_memo(move |_| animation_and_outlet.get().1);

    create_isomorphic_effect(move |_| {
        match (route.child(), &is_showing.get()) {
            (None, prev) => {
                /* if let Some(prev_scope) = prev.map(|(_, scope)| scope) {
                    prev_scope.dispose();
                } */
                set_outlet.set(None);
            }
            (Some(child), Some(is_showing_val))
                if child.id() == *is_showing_val =>
            {
                // do nothing: we don't need to rerender the component, because it's the same
                trigger_animation.set(());
            }
            (Some(child), prev) => {
                    //provide_context(child_child.clone());
                    set_outlet
                        .set(Some(child.outlet().into_view()));
                    is_showing.set(Some(child.id()));
            }
        }
    });

    let class = move || {
        let animation_class = match current_animation.get() {
            AnimationState::Outro => outro.unwrap_or_default(),
            AnimationState::Start => start.unwrap_or_default(),
            AnimationState::Intro => intro.unwrap_or_default(),
            AnimationState::Finally => finally.unwrap_or_default(),
            AnimationState::OutroBack => outro_back.unwrap_or_default(),
            AnimationState::IntroBack => intro_back.unwrap_or_default(),
        };
        if let Some(class) = &class {
            format!("{} {animation_class}", class.get())
        } else {
            animation_class.to_string()
        }
    };
    let node_ref = create_node_ref::<html::Div>();
    let animationend = move |ev: AnimationEvent| {
        use wasm_bindgen::JsCast;
        if let Some(target) = ev.target() {
            let node_ref = node_ref.get();
            if node_ref.is_none()
                || target
                    .unchecked_ref::<web_sys::Node>()
                    .is_same_node(Some(&*node_ref.unwrap()))
            {
                ev.stop_propagation();
                let current = current_animation.get();
                set_animation_state.update(|current_state| {
                    let (next, _) =
                        animation.next_state(&current, is_back.get_untracked());
                    *current_state = next;
                });
            }
        }
    };

    view! {
        <div class=class on:animationend=animationend>
            {move || current_outlet.get()}
        </div>
    }
}
 */
