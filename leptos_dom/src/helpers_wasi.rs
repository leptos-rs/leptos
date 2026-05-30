#![allow(missing_docs)]
//! WASI stubs for DOM utility functions.

use std::time::Duration;
use reactive_graph::owner::Owner;

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct TimeoutHandle;

impl TimeoutHandle {
    pub fn clear(&self) {}
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct AnimationFrameRequestHandle;

impl AnimationFrameRequestHandle {
    pub fn cancel(&self) {}
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct IdleCallbackHandle;

impl IdleCallbackHandle {
    pub fn cancel(&self) {}
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct IntervalHandle;

impl IntervalHandle {
    pub fn clear(&self) {}
}

pub struct WindowListenerHandle;

impl core::fmt::Debug for WindowListenerHandle {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_tuple("WindowListenerHandle").finish()
    }
}

impl WindowListenerHandle {
    pub fn remove(self) {}
}

pub fn window() {}
pub fn document() {}
pub fn location() {}
pub fn location_hash() -> Option<String> { None }
pub fn location_pathname() -> Option<String> { None }

pub fn set_property<T, U, V>(el: T, prop_name: U, value: V) {
    let _ = (el, prop_name, value);
}

pub fn get_property<T, U>(el: T, prop_name: U) -> Result<(), ()> {
    let _ = (el, prop_name);
    Err(())
}

pub fn event_target<T, U>(event: T) -> U {
    let _ = event;
    unreachable!()
}

pub fn event_target_value<T>(event: T) -> String {
    let _ = event;
    unreachable!()
}

pub fn event_target_checked<T>(ev: T) -> bool {
    let _ = ev;
    unreachable!()
}

pub fn set_timeout_with_handle<T>(
    cb: T,
    duration: Duration,
) -> Result<TimeoutHandle, ()> {
    let _ = (cb, duration);
    Err(())
}

pub fn set_timeout<T>(cb: T, duration: Duration) {
    let _ = (cb, duration);
}

pub fn set_interval_with_handle<T>(
    cb: T,
    duration: Duration,
) -> Result<IntervalHandle, ()> {
    let _ = (cb, duration);
    Err(())
}

pub fn set_interval<T>(cb: T, duration: Duration) {
    let _ = (cb, duration);
}

pub fn request_animation_frame<T>(cb: T) {
    let _ = cb;
}

pub fn request_animation_frame_with_handle<T>(
    cb: T,
) -> Result<AnimationFrameRequestHandle, ()> {
    let _ = cb;
    Err(())
}

pub fn request_idle_callback<T>(cb: T) {
    let _ = cb;
}

pub fn request_idle_callback_with_handle<T>(
    cb: T,
) -> Result<IdleCallbackHandle, ()> {
    let _ = cb;
    Err(())
}

pub fn queue_microtask<T>(task: T) {
    let _ = task;
}

pub fn debounce<T, U>(
    delay: Duration,
    cb: U,
) -> impl FnMut(T) {
    let _ = (delay, cb);
    move |_| {}
}

pub fn window_event_listener_untyped<T, U>(
    event_name: T,
    cb: U,
) -> WindowListenerHandle {
    let _ = (event_name, cb);
    WindowListenerHandle
}

pub fn window_event_listener<T, U>(
    event: T,
    cb: U,
) -> WindowListenerHandle {
    let _ = (event, cb);
    WindowListenerHandle
}

/// Returns `true` if the current environment is a server.
pub fn is_server() -> bool {
    #[cfg(feature = "hydration")]
    {
        Owner::current_shared_context()
            .map(|sc| !sc.is_browser())
            .unwrap_or(false)
    }
    #[cfg(not(feature = "hydration"))]
    {
        false
    }
}

/// Returns `true` if the current environment is a browser.
pub fn is_browser() -> bool {
    !is_server()
}
