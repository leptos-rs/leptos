use crate::{
    animation::{Animation, AnimationState},
    use_route,
};
use leptos::{leptos_dom::HydrationCtx, *};
use std::{cell::Cell, rc::Rc};
use web_sys::AnimationEvent;

/// Displays the child route nested in a parent route, allowing you to control exactly where
/// that child route is displayed. Renders nothing if there is no nested child.
#[component]
pub fn Outlet(cx: Scope) -> impl IntoView {
    let id = HydrationCtx::id();
    let route = use_route(cx);
    let is_showing = Rc::new(Cell::new(None::<(usize, Scope)>));
    let (outlet, set_outlet) = create_signal(cx, None::<View>);
    create_isomorphic_effect(cx, move |_| {
        match (route.child(cx), &is_showing.get()) {
            (None, prev) => {
                if let Some(prev_scope) = prev.map(|(_, scope)| scope) {
                    prev_scope.dispose();
                }
                set_outlet.set(None);
            }
            (Some(child), Some((is_showing_val, _)))
                if child.id() == *is_showing_val =>
            {
                // do nothing: we don't need to rerender the component, because it's the same
            }
            (Some(child), prev) => {
                if let Some(prev_scope) = prev.map(|(_, scope)| scope) {
                    prev_scope.dispose();
                }
                _ = cx.child_scope(|child_cx| {
                    provide_context(child_cx, child.clone());
                    set_outlet
                        .set(Some(child.outlet(child_cx).into_view(child_cx)));
                    is_showing.set(Some((child.id(), child_cx)));
                });
            }
        }
    });

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
    cx: Scope,
    /// CSS class added when route is being unmounted
    #[prop(optional)]
    outro: Option<&'static str>,
    /// CSS class added when route is first created
    #[prop(optional)]
    start: Option<&'static str>,
    /// CSS class added while the route is being mounted
    #[prop(optional)]
    intro: Option<&'static str>,
    /// CSS class added after other animations have completed.
    #[prop(optional)]
    finally: Option<&'static str>,
) -> impl IntoView {
    let id = HydrationCtx::id();
    let route = use_route(cx);
    let is_showing = Rc::new(Cell::new(None::<(usize, Scope)>));
    let (outlet, set_outlet) = create_signal(cx, None::<View>);

    let animation = Animation {
        outro,
        start,
        intro,
        finally,
    };
    let (animation_state, set_animation_state) =
        create_signal(cx, AnimationState::Finally);
    let trigger_animation = create_rw_signal(cx, ());
    let animation_and_outlet = create_memo(cx, {
        move |prev: Option<&(AnimationState, View)>| {
            let animation_state = animation_state.get();
            let next_outlet = outlet.get().unwrap_or_default();
            trigger_animation.track();
            match prev {
                None => (animation_state, next_outlet),
                Some((prev_state, prev_outlet)) => {
                    let (next_state, can_advance) =
                        animation.next_state(prev_state);

                    if can_advance {
                        (next_state, next_outlet)
                    } else {
                        (next_state, prev_outlet.to_owned())
                    }
                }
            }
        }
    });
    let current_animation =
        create_memo(cx, move |_| animation_and_outlet.get().0);
    let current_outlet = create_memo(cx, move |_| animation_and_outlet.get().1);

    create_isomorphic_effect(cx, move |_| {
        match (route.child(cx), &is_showing.get()) {
            (None, prev) => {
                if let Some(prev_scope) = prev.map(|(_, scope)| scope) {
                    prev_scope.dispose();
                }
                set_outlet.set(None);
            }
            (Some(child), Some((is_showing_val, _)))
                if child.id() == *is_showing_val =>
            {
                // do nothing: we don't need to rerender the component, because it's the same
                trigger_animation.set(());
            }
            (Some(child), prev) => {
                if let Some(prev_scope) = prev.map(|(_, scope)| scope) {
                    prev_scope.dispose();
                }
                _ = cx.child_scope(|child_cx| {
                    provide_context(child_cx, child.clone());
                    set_outlet
                        .set(Some(child.outlet(child_cx).into_view(child_cx)));
                    is_showing.set(Some((child.id(), child_cx)));
                });
            }
        }
    });

    let class = move || match current_animation.get() {
        AnimationState::Outro => outro.unwrap_or_default(),
        AnimationState::Start => start.unwrap_or_default(),
        AnimationState::Intro => intro.unwrap_or_default(),
        AnimationState::Finally => finally.unwrap_or_default(),
    };
    let animationend = move |ev: AnimationEvent| {
        ev.stop_propagation();
        let current = current_animation.get();
        set_animation_state.update(|current_state| {
            let (next, _) = animation.next_state(&current);
            *current_state = next;
        });
    };

    view! { cx,
        <div class=class on:animationend=animationend>
            {move || current_outlet.get()}
        </div>
    }
}
