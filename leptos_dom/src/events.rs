pub mod typed;

use std::{borrow::Cow, cell::RefCell, collections::HashSet};
#[cfg(all(target_arch = "wasm32", feature = "web"))]
use wasm_bindgen::{
    convert::FromWasmAbi, intern, prelude::Closure, JsCast, JsValue,
    UnwrapThrowExt,
};

thread_local! {
    pub(crate) static GLOBAL_EVENTS: RefCell<HashSet<Cow<'static, str>>> = RefCell::new(HashSet::new());
}

// Used in template macro
#[doc(hidden)]
#[cfg(all(target_arch = "wasm32", feature = "web"))]
pub fn add_event_helper<E: crate::ev::EventDescriptor + 'static>(
    target: &web_sys::Element,
    event: E,
    #[allow(unused_mut)] // used for tracing in debug
    mut event_handler: impl FnMut(E::EventType) + 'static,
) {
    let event_name = event.name();

    if event.bubbles() {
        add_event_listener(
            target,
            event.event_delegation_key(),
            event_name,
            event_handler,
        );
    } else {
        add_event_listener_undelegated(target, &event_name, event_handler);
    }
}

/// Adds an event listener to the target DOM element using implicit event delegation.
#[doc(hidden)]
#[cfg(all(target_arch = "wasm32", feature = "web"))]
pub fn add_event_listener<E>(
    target: &web_sys::Element,
    key: Cow<'static, str>,
    event_name: Cow<'static, str>,
    #[cfg(debug_assertions)] mut cb: impl FnMut(E) + 'static,
    #[cfg(not(debug_assertions))] cb: impl FnMut(E) + 'static,
) where
    E: FromWasmAbi + 'static,
{
    cfg_if::cfg_if! {
      if #[cfg(debug_assertions)] {
        let span = ::tracing::Span::current();
        let cb = move |e| {
          let _guard = span.enter();
          cb(e);
        };
      }
    }

    let cb = Closure::wrap(Box::new(cb) as Box<dyn FnMut(E)>).into_js_value();
    let key = intern(&key);
    _ = js_sys::Reflect::set(target, &JsValue::from_str(&key), &cb);
    add_delegated_event_listener(&key, event_name);
}

#[doc(hidden)]
#[cfg(all(target_arch = "wasm32", feature = "web"))]
pub(crate) fn add_event_listener_undelegated<E>(
    target: &web_sys::Element,
    event_name: &str,
    #[cfg(debug_assertions)] mut cb: impl FnMut(E) + 'static,
    #[cfg(not(debug_assertions))] cb: impl FnMut(E) + 'static,
) where
    E: FromWasmAbi + 'static,
{
    cfg_if::cfg_if! {
      if #[cfg(debug_assertions)] {
        let span = ::tracing::Span::current();
        let cb = move |e| {
          let _guard = span.enter();
          cb(e);
        };
      }
    }

    let event_name = intern(event_name);
    let cb = Closure::wrap(Box::new(cb) as Box<dyn FnMut(E)>).into_js_value();
    _ = target.add_event_listener_with_callback(event_name, cb.unchecked_ref());
}

// cf eventHandler in ryansolid/dom-expressions
#[cfg(all(target_arch = "wasm32", feature = "web"))]
pub(crate) fn add_delegated_event_listener(
    key: &str,
    event_name: Cow<'static, str>,
) {
    GLOBAL_EVENTS.with(|global_events| {
        let mut events = global_events.borrow_mut();
        if !events.contains(&event_name) {
            // create global handler
            let key = JsValue::from_str(&key);
            let handler = move |ev: web_sys::Event| {
                let target = ev.target();
                let node = ev.composed_path().get(0);
                let mut node = if node.is_undefined() || node.is_null() {
                    JsValue::from(target)
                } else {
                    node
                };

                // TODO reverse Shadow DOM retargetting

                // TODO simulate currentTarget

                while !node.is_null() {
                    let node_is_disabled = js_sys::Reflect::get(
                        &node,
                        &JsValue::from_str("disabled"),
                    )
                    .unwrap_throw()
                    .is_truthy();
                    if !node_is_disabled {
                        let maybe_handler =
                            js_sys::Reflect::get(&node, &key).unwrap_throw();
                        if !maybe_handler.is_undefined() {
                            let f = maybe_handler
                                .unchecked_ref::<js_sys::Function>();
                            let _ = f.call1(&node, &ev);

                            if ev.cancel_bubble() {
                                return;
                            }
                        }
                    }

                    // navigate up tree
                    let host =
                        js_sys::Reflect::get(&node, &JsValue::from_str("host"))
                            .unwrap_throw();
                    if host.is_truthy()
                        && host != node
                        && host.dyn_ref::<web_sys::Node>().is_some()
                    {
                        node = host;
                    } else if let Some(parent) =
                        node.unchecked_into::<web_sys::Node>().parent_node()
                    {
                        node = parent.into()
                    } else {
                        node = JsValue::null()
                    }
                }
            };

            cfg_if::cfg_if! {
              if #[cfg(debug_assertions)] {
                let span = ::tracing::Span::current();
                let handler = move |e| {
                  let _guard = span.enter();
                  handler(e);
                };
              }
            }

            let handler = Box::new(handler) as Box<dyn FnMut(web_sys::Event)>;
            let handler = Closure::wrap(handler).into_js_value();
            _ = crate::window().add_event_listener_with_callback(
                &event_name,
                handler.unchecked_ref(),
            );

            // register that we've created handler
            events.insert(event_name);
        }
    })
}
