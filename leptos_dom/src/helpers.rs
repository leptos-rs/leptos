//! A variety of DOM utility functions.

use crate::{events::typed as ev, is_server, window};
use leptos_reactive::on_cleanup;
use std::time::Duration;
use wasm_bindgen::{prelude::Closure, JsCast, JsValue, UnwrapThrowExt};

/// Sets a property on a DOM element.
pub fn set_property(
    el: &web_sys::Element,
    prop_name: &str,
    value: &Option<JsValue>,
) {
    let key = JsValue::from_str(prop_name);
    match value {
        Some(value) => _ = js_sys::Reflect::set(el, &key, value),
        None => _ = js_sys::Reflect::delete_property(el, &key),
    };
}

/// Gets the value of a property set on a DOM element.
#[doc(hidden)]
pub fn get_property(
    el: &web_sys::Element,
    prop_name: &str,
) -> Result<JsValue, JsValue> {
    let key = JsValue::from_str(prop_name);
    js_sys::Reflect::get(el, &key)
}

/// Returns the current [`window.location`](https://developer.mozilla.org/en-US/docs/Web/API/Window/location).
pub fn location() -> web_sys::Location {
    window().location()
}

/// Current [`window.location.hash`](https://developer.mozilla.org/en-US/docs/Web/API/Window/location)
/// without the beginning #.
pub fn location_hash() -> Option<String> {
    if is_server() {
        None
    } else {
        location().hash().ok().map(|hash| hash.replace('#', ""))
    }
}

/// Current [`window.location.pathname`](https://developer.mozilla.org/en-US/docs/Web/API/Window/location).
pub fn location_pathname() -> Option<String> {
    location().pathname().ok()
}

/// Helper function to extract [`Event.target`](https://developer.mozilla.org/en-US/docs/Web/API/Event/target)
/// from any event.
pub fn event_target<T>(event: &web_sys::Event) -> T
where
    T: JsCast,
{
    event.target().unwrap_throw().unchecked_into::<T>()
}

/// Helper function to extract `event.target.value` from an event.
///
/// This is useful in the `on:input` or `on:change` listeners for an `<input>` element.
pub fn event_target_value<T>(event: &T) -> String
where
    T: JsCast,
{
    event
        .unchecked_ref::<web_sys::Event>()
        .target()
        .unwrap_throw()
        .unchecked_into::<web_sys::HtmlInputElement>()
        .value()
}

/// Helper function to extract `event.target.checked` from an event.
///
/// This is useful in the `on:change` listeners for an `<input type="checkbox">` element.
pub fn event_target_checked(ev: &web_sys::Event) -> bool {
    ev.target()
        .unwrap()
        .unchecked_into::<web_sys::HtmlInputElement>()
        .checked()
}

/// Handle that is generated by [request_animation_frame_with_handle] and can
/// be used to cancel the animation frame request.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct AnimationFrameRequestHandle(i32);

impl AnimationFrameRequestHandle {
    /// Cancels the animation frame request to which this refers.
    /// See [`cancelAnimationFrame()`](https://developer.mozilla.org/en-US/docs/Web/API/Window/cancelAnimationFrame)
    pub fn cancel(&self) {
        _ = window().cancel_animation_frame(self.0);
    }
}

/// Runs the given function between the next repaint using
/// [`Window.requestAnimationFrame`](https://developer.mozilla.org/en-US/docs/Web/API/window/requestAnimationFrame).
#[cfg_attr(debug_assertions, instrument(level = "trace", skip_all))]
#[inline(always)]
pub fn request_animation_frame(cb: impl FnOnce() + 'static) {
    _ = request_animation_frame_with_handle(cb);
}

/// Runs the given function between the next repaint using
/// [`Window.requestAnimationFrame`](https://developer.mozilla.org/en-US/docs/Web/API/window/requestAnimationFrame),
/// returning a cancelable handle.
#[cfg_attr(debug_assertions, instrument(level = "trace", skip_all))]
#[inline(always)]
pub fn request_animation_frame_with_handle(
    cb: impl FnOnce() + 'static,
) -> Result<AnimationFrameRequestHandle, JsValue> {
    cfg_if::cfg_if! {
      if #[cfg(debug_assertions)] {
        let span = ::tracing::Span::current();
        let cb = move || {
          let _guard = span.enter();
          cb();
        };
      }
    }

    #[inline(never)]
    fn raf(cb: JsValue) -> Result<AnimationFrameRequestHandle, JsValue> {
        window()
            .request_animation_frame(cb.as_ref().unchecked_ref())
            .map(AnimationFrameRequestHandle)
    }

    raf(Closure::once_into_js(cb))
}

/// Handle that is generated by [request_idle_callback_with_handle] and can be
/// used to cancel the idle callback.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct IdleCallbackHandle(u32);

impl IdleCallbackHandle {
    /// Cancels the idle callback to which this refers.
    /// See [`cancelAnimationFrame()`](https://developer.mozilla.org/en-US/docs/Web/API/Window/cancelIdleCallback)
    pub fn cancel(&self) {
        window().cancel_idle_callback(self.0);
    }
}

/// Queues the given function during an idle period using
/// [`Window.requestIdleCallback`](https://developer.mozilla.org/en-US/docs/Web/API/window/requestIdleCallback).
#[cfg_attr(debug_assertions, instrument(level = "trace", skip_all))]
#[inline(always)]
pub fn request_idle_callback(cb: impl Fn() + 'static) {
    _ = request_idle_callback_with_handle(cb);
}

/// Queues the given function during an idle period using
/// [`Window.requestIdleCallback`](https://developer.mozilla.org/en-US/docs/Web/API/window/requestIdleCallback),
/// returning a cancelable handle.
#[cfg_attr(debug_assertions, instrument(level = "trace", skip_all))]
#[inline(always)]
pub fn request_idle_callback_with_handle(
    cb: impl Fn() + 'static,
) -> Result<IdleCallbackHandle, JsValue> {
    cfg_if::cfg_if! {
      if #[cfg(debug_assertions)] {
        let span = ::tracing::Span::current();
        let cb = move || {
          let _guard = span.enter();
          cb();
        };
      }
    }

    #[inline(never)]
    fn ric(cb: Box<dyn Fn()>) -> Result<IdleCallbackHandle, JsValue> {
        let cb = Closure::wrap(cb).into_js_value();

        window()
            .request_idle_callback(cb.as_ref().unchecked_ref())
            .map(IdleCallbackHandle)
    }

    ric(Box::new(cb))
}

/// Handle that is generated by [set_timeout_with_handle] and can be used to clear the timeout.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct TimeoutHandle(i32);

impl TimeoutHandle {
    /// Cancels the timeout to which this refers.
    /// See [`clearTimeout()`](https://developer.mozilla.org/en-US/docs/Web/API/clearTimeout)
    pub fn clear(&self) {
        window().clear_timeout_with_handle(self.0);
    }
}

/// Executes the given function after the given duration of time has passed.
/// [`setTimeout()`](https://developer.mozilla.org/en-US/docs/Web/API/setTimeout).
#[cfg_attr(
  any(debug_assertions, feature = "ssr"),
  instrument(level = "trace", skip_all, fields(duration = ?duration))
)]
pub fn set_timeout(cb: impl FnOnce() + 'static, duration: Duration) {
    _ = set_timeout_with_handle(cb, duration);
}

/// Executes the given function after the given duration of time has passed, returning a cancelable handle.
/// [`setTimeout()`](https://developer.mozilla.org/en-US/docs/Web/API/setTimeout).
#[cfg_attr(
  any(debug_assertions, feature = "ssr"),
  instrument(level = "trace", skip_all, fields(duration = ?duration))
)]
#[inline(always)]
pub fn set_timeout_with_handle(
    cb: impl FnOnce() + 'static,
    duration: Duration,
) -> Result<TimeoutHandle, JsValue> {
    cfg_if::cfg_if! {
      if #[cfg(debug_assertions)] {
        let span = ::tracing::Span::current();
        let cb = move || {
          let prev = leptos_reactive::SpecialNonReactiveZone::enter();
          let _guard = span.enter();
          cb();
          leptos_reactive::SpecialNonReactiveZone::exit(prev);
        };
      }
    }

    #[inline(never)]
    fn st(cb: JsValue, duration: Duration) -> Result<TimeoutHandle, JsValue> {
        window()
            .set_timeout_with_callback_and_timeout_and_arguments_0(
                cb.as_ref().unchecked_ref(),
                duration.as_millis().try_into().unwrap_throw(),
            )
            .map(TimeoutHandle)
    }

    st(Closure::once_into_js(cb), duration)
}

/// "Debounce" a callback function. This will cause it to wait for a period of `delay`
/// after it is called. If it is called again during that period, it will wait
/// `delay` before running, and so on. This can be used, for example, to wrap event
/// listeners to prevent them from firing constantly as you type.
///
/// ```
/// use leptos::{leptos_dom::helpers::debounce, logging::log, *};
///
/// #[component]
/// fn DebouncedButton() -> impl IntoView {
///     let delay = std::time::Duration::from_millis(250);
///     let on_click = debounce(delay, move |_| {
///         log!("...so many clicks!");
///     });
///
///     view! {
///       <button on:click=on_click>"Click me"</button>
///     }
/// }
/// ```
pub fn debounce<T: 'static>(
    delay: Duration,
    #[cfg(debug_assertions)] mut cb: impl FnMut(T) + 'static,
    #[cfg(not(debug_assertions))] cb: impl FnMut(T) + 'static,
) -> impl FnMut(T) {
    use std::{
        cell::{Cell, RefCell},
        rc::Rc,
    };

    cfg_if::cfg_if! {
      if #[cfg(debug_assertions)] {
        let span = ::tracing::Span::current();
        let cb = move |value| {
          let prev = leptos_reactive::SpecialNonReactiveZone::enter();
          let _guard = span.enter();
          cb(value);
          leptos_reactive::SpecialNonReactiveZone::exit(prev);
        };
      }
    }
    let cb = Rc::new(RefCell::new(cb));

    let timer = Rc::new(Cell::new(None::<TimeoutHandle>));

    on_cleanup({
        let timer = Rc::clone(&timer);
        move || {
            if let Some(timer) = timer.take() {
                timer.clear();
            }
        }
    });

    move |arg| {
        if let Some(timer) = timer.take() {
            timer.clear();
        }
        let handle = set_timeout_with_handle(
            {
                let cb = Rc::clone(&cb);
                move || {
                    cb.borrow_mut()(arg);
                }
            },
            delay,
        );
        if let Ok(handle) = handle {
            timer.set(Some(handle));
        }
    }
}

/// Handle that is generated by [set_interval] and can be used to clear the interval.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct IntervalHandle(i32);

impl IntervalHandle {
    /// Cancels the repeating event to which this refers.
    /// See [`clearInterval()`](https://developer.mozilla.org/en-US/docs/Web/API/clearInterval)
    pub fn clear(&self) {
        window().clear_interval_with_handle(self.0);
    }
}

/// Repeatedly calls the given function, with a delay of the given duration between calls,
/// returning a cancelable handle.
/// See [`setInterval()`](https://developer.mozilla.org/en-US/docs/Web/API/setInterval).
#[cfg_attr(
  any(debug_assertions, feature = "ssr"),
  instrument(level = "trace", skip_all, fields(duration = ?duration))
)]
pub fn set_interval(cb: impl Fn() + 'static, duration: Duration) {
    _ = set_interval_with_handle(cb, duration);
}

/// Repeatedly calls the given function, with a delay of the given duration between calls,
/// returning a cancelable handle.
/// See [`setInterval()`](https://developer.mozilla.org/en-US/docs/Web/API/setInterval).
#[cfg_attr(
  any(debug_assertions, feature = "ssr"),
  instrument(level = "trace", skip_all, fields(duration = ?duration))
)]
#[inline(always)]
pub fn set_interval_with_handle(
    cb: impl Fn() + 'static,
    duration: Duration,
) -> Result<IntervalHandle, JsValue> {
    cfg_if::cfg_if! {
      if #[cfg(debug_assertions)] {
        let span = ::tracing::Span::current();
        let cb = move || {
          let prev = leptos_reactive::SpecialNonReactiveZone::enter();
          let _guard = span.enter();
          cb();
          leptos_reactive::SpecialNonReactiveZone::exit(prev);
        };
      }
    }

    #[inline(never)]
    fn si(
        cb: Box<dyn Fn()>,
        duration: Duration,
    ) -> Result<IntervalHandle, JsValue> {
        let cb = Closure::wrap(cb).into_js_value();

        window()
            .set_interval_with_callback_and_timeout_and_arguments_0(
                cb.as_ref().unchecked_ref(),
                duration.as_millis().try_into().unwrap_throw(),
            )
            .map(IntervalHandle)
    }

    si(Box::new(cb), duration)
}

/// Adds an event listener to the `Window`, typed as a generic `Event`,
/// returning a cancelable handle.
#[cfg_attr(
  debug_assertions,
  instrument(level = "trace", skip_all, fields(event_name = %event_name))
)]
#[inline(always)]
pub fn window_event_listener_untyped(
    event_name: &str,
    cb: impl Fn(web_sys::Event) + 'static,
) -> WindowListenerHandle {
    cfg_if::cfg_if! {
      if #[cfg(debug_assertions)] {
        let span = ::tracing::Span::current();
        let cb = move |e| {
          let prev = leptos_reactive::SpecialNonReactiveZone::enter();
          let _guard = span.enter();
          cb(e);
          leptos_reactive::SpecialNonReactiveZone::exit(prev);
        };
      }
    }

    if !is_server() {
        #[inline(never)]
        fn wel(
            cb: Box<dyn FnMut(web_sys::Event)>,
            event_name: &str,
        ) -> WindowListenerHandle {
            let cb = Closure::wrap(cb).into_js_value();
            _ = window().add_event_listener_with_callback(
                event_name,
                cb.unchecked_ref(),
            );
            let event_name = event_name.to_string();
            WindowListenerHandle(Box::new(move || {
                _ = window().remove_event_listener_with_callback(
                    &event_name,
                    cb.unchecked_ref(),
                );
            }))
        }

        wel(Box::new(cb), event_name)
    } else {
        WindowListenerHandle(Box::new(|| ()))
    }
}

/// Creates a window event listener from a typed event, returning a
/// cancelable handle.
/// ```
/// use leptos::{leptos_dom::helpers::window_event_listener, logging::log, *};
///
/// #[component]
/// fn App() -> impl IntoView {
///     let handle = window_event_listener(ev::keypress, |ev| {
///         // ev is typed as KeyboardEvent automatically,
///         // so .code() can be called
///         let code = ev.code();
///         log!("code = {code:?}");
///     });
///     on_cleanup(move || handle.remove());
/// }
/// ```
pub fn window_event_listener<E: ev::EventDescriptor + 'static>(
    event: E,
    cb: impl Fn(E::EventType) + 'static,
) -> WindowListenerHandle
where
    E::EventType: JsCast,
{
    window_event_listener_untyped(&event.name(), move |e| {
        cb(e.unchecked_into::<E::EventType>())
    })
}

/// A handle that can be called to remove a global event listener.
pub struct WindowListenerHandle(Box<dyn FnOnce()>);

impl std::fmt::Debug for WindowListenerHandle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("WindowListenerHandle").finish()
    }
}

impl WindowListenerHandle {
    /// Removes the event listener.
    pub fn remove(self) {
        (self.0)()
    }
}

#[doc(hidden)]
/// This exists only to enable type inference on event listeners when in SSR mode.
pub fn ssr_event_listener<E: crate::ev::EventDescriptor + 'static>(
    event: E,
    event_handler: impl FnMut(E::EventType) + 'static,
) {
    _ = event;
    _ = event_handler;
}
