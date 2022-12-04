//! Collection of typed events.

use std::borrow::Cow;
use wasm_bindgen::convert::FromWasmAbi;

/// A trait for converting types into [web_sys events](web_sys).
pub trait IntoEvent {
  /// The [`web_sys`] event type, such as [`web_sys::MouseEvent`].
  type EventType: FromWasmAbi;

  /// The name of the event, such as `click` or `mouseover`.
  fn name(&self) -> Cow<'static, str>;

  /// Indicates if this event bubbles. For example, `click` bubbles,
  /// but `focus` does not.
  ///
  /// If this method returns true, then the event will be delegated globally,
  /// otherwise, event listeners will be directly attached to the element.
  fn bubbles(&self) -> bool {
    true
  }
}

macro_rules! generate_event_types {
  [$([$web_sys_event:ident, [$($event:ident),* $(,)?]]),* $(,)?] => {
    paste::paste! {
      $(
        $(
          #[doc = "The "]
          #[doc = stringify!([<$event:lower>])]
          #[doc = "event."]
          pub struct $event;

          impl IntoEvent for $event {
            type EventType = web_sys::MouseEvent;

            fn name(&self) -> Cow<'static, str> {
              concat!("on", stringify!([<$event:lower>])).into()
            }
          }
        )*
      )*
    }
  };
}

generate_event_types![
  // ClipboardEvent is unstable
  [Event, [Copy, Cut, Paste]]
];
