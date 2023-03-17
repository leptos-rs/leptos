//! A variety of DOM utility functions.

use crate::{is_server, window};
use leptos_reactive::{on_cleanup, Scope};
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

/// Runs the given function between the next repaint
/// using [`Window.requestAnimationFrame`](https://developer.mozilla.org/en-US/docs/Web/API/window/requestAnimationFrame).
#[cfg_attr(debug_assertions, instrument(level = "trace", skip_all))]
pub fn request_animation_frame(cb: impl FnOnce() + 'static) {
    cfg_if::cfg_if! {
      if #[cfg(debug_assertions)] {
        let span = ::tracing::Span::current();
        let cb = move || {
          let _guard = span.enter();
          cb();
        };
      }
    }

    let cb = Closure::once_into_js(cb);
    _ = window().request_animation_frame(cb.as_ref().unchecked_ref());
}

/// Queues the given function during an idle period  
/// using [`Window.requestIdleCallback`](https://developer.mozilla.org/en-US/docs/Web/API/window/requestIdleCallback).
#[cfg_attr(debug_assertions, instrument(level = "trace", skip_all))]
pub fn request_idle_callback(cb: impl Fn() + 'static) {
    cfg_if::cfg_if! {
      if #[cfg(debug_assertions)] {
        let span = ::tracing::Span::current();
        let cb = move || {
          let _guard = span.enter();
          cb();
        };
      }
    }

    let cb = Closure::wrap(Box::new(cb) as Box<dyn Fn()>).into_js_value();
    _ = window().request_idle_callback(cb.as_ref().unchecked_ref());
}

/// Executes the given function after the given duration of time has passed.
/// [`setTimeout()`](https://developer.mozilla.org/en-US/docs/Web/API/setTimeout).
#[cfg_attr(
  debug_assertions,
  instrument(level = "trace", skip_all, fields(duration = ?duration))
)]
pub fn set_timeout(cb: impl FnOnce() + 'static, duration: Duration) {
    _ = set_timeout_with_handle(cb, duration);
}

/// Executes the given function after the given duration of time has passed, returning a cancelable handle.
/// [`setTimeout()`](https://developer.mozilla.org/en-US/docs/Web/API/setTimeout).
#[cfg_attr(
  debug_assertions,
  instrument(level = "trace", skip_all, fields(duration = ?duration))
)]
pub fn set_timeout_with_handle(
    cb: impl FnOnce() + 'static,
    duration: Duration,
) -> Result<TimeoutHandle, JsValue> {
    cfg_if::cfg_if! {
      if #[cfg(debug_assertions)] {
        let span = ::tracing::Span::current();
        let cb = move || {
          let _guard = span.enter();
          cb();
        };
      }
    }

    let cb = Closure::once_into_js(Box::new(cb) as Box<dyn FnOnce()>);
    window()
        .set_timeout_with_callback_and_timeout_and_arguments_0(
            cb.as_ref().unchecked_ref(),
            duration.as_millis().try_into().unwrap_throw(),
        )
        .map(TimeoutHandle)
}

/// Handle that is generated by [set_interval] and can be used to clear the interval.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct TimeoutHandle(i32);

impl TimeoutHandle {
    /// Cancels the timeout to which this refers.
    /// See [`clearTimeout()`](https://developer.mozilla.org/en-US/docs/Web/API/clearTimeout)
    pub fn clear(&self) {
        window().clear_timeout_with_handle(self.0);
    }
}

/// "Debounce" a callback function. This will cause it to wait for a period of `delay`
/// after it is called. If it is called again during that period, it will wait
/// `delay` before running, and so on. This can be used, for example, to wrap event
/// listeners to prevent them from firing constantly as you type.
///
/// ```
/// use leptos::{leptos_dom::helpers::debounce, *};
///
/// #[component]
/// fn DebouncedButton(cx: Scope) -> impl IntoView {
///     let delay = std::time::Duration::from_millis(250);
///     let on_click = debounce(cx, delay, move |_| {
///         log!("...so many clicks!");
///     });
///
///     view! { cx,
///       <button on:click=on_click>"Click me"</button>
///     }
/// }
/// ```
pub fn debounce<T: 'static>(
    cx: Scope,
    delay: Duration,
    mut cb: impl FnMut(T) + 'static,
) -> impl FnMut(T) {
    use std::{
        cell::{Cell, RefCell},
        rc::Rc,
    };

    cfg_if::cfg_if! {
      if #[cfg(debug_assertions)] {
        let span = ::tracing::Span::current();
        let cb = move |value| {
          let _guard = span.enter();
          cb(value);
        };
      }
    }
    let cb = Rc::new(RefCell::new(cb));

    let timer = Rc::new(Cell::new(None::<TimeoutHandle>));

    on_cleanup(cx, {
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

/// Repeatedly calls the given function, with a delay of the given duration between calls.
/// See [`setInterval()`](https://developer.mozilla.org/en-US/docs/Web/API/setInterval).
#[cfg_attr(
  debug_assertions,
  instrument(level = "trace", skip_all, fields(duration = ?duration))
)]
pub fn set_interval(
    cb: impl Fn() + 'static,
    duration: Duration,
) -> Result<IntervalHandle, JsValue> {
    cfg_if::cfg_if! {
      if #[cfg(debug_assertions)] {
        let span = ::tracing::Span::current();
        let cb = move || {
          let _guard = span.enter();
          cb();
        };
      }
    }

    let cb = Closure::wrap(Box::new(cb) as Box<dyn Fn()>).into_js_value();
    let handle = window()
        .set_interval_with_callback_and_timeout_and_arguments_0(
            cb.as_ref().unchecked_ref(),
            duration.as_millis().try_into().unwrap_throw(),
        )?;
    Ok(IntervalHandle(handle))
}

/// Adds an event listener to the `Window`.
#[cfg_attr(
  debug_assertions,
  instrument(level = "trace", skip_all, fields(event_name = %event_name))
)]
pub fn window_event_listener(
    event_name: &str,
    cb: impl Fn(web_sys::Event) + 'static,
) {
    cfg_if::cfg_if! {
      if #[cfg(debug_assertions)] {
        let span = ::tracing::Span::current();
        let cb = move |e| {
          let _guard = span.enter();
          cb(e);
        };
      }
    }

    if !is_server() {
        let handler = Box::new(cb) as Box<dyn FnMut(web_sys::Event)>;

        let cb = Closure::wrap(handler).into_js_value();
        _ = window()
            .add_event_listener_with_callback(event_name, cb.unchecked_ref());
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
