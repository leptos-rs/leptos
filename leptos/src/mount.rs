use crate::IntoView;
use any_spawner::Executor;
use reactive_graph::owner::Owner;
use std::marker::PhantomData;
use tachys::{
    dom::body,
    hydration::Cursor,
    renderer::{dom::Dom, Renderer},
    view::{Mountable, PositionState, Render, RenderHtml},
};
use wasm_bindgen::JsCast;
use web_sys::HtmlElement;

#[cfg(feature = "hydrate")]
/// Hydrates the app described by the provided function, starting at `<body>`.
pub fn hydrate_body<F, N>(f: F)
where
    F: FnOnce() -> N + 'static,
    N: IntoView,
{
    let owner = hydrate_from(body(), f);
    owner.forget();
}

#[cfg(feature = "hydrate")]
/// Runs the provided closure and mounts the result to the provided element.
pub fn hydrate_from<F, N>(
    parent: HtmlElement,
    f: F,
) -> UnmountHandle<N::State, Dom>
where
    F: FnOnce() -> N + 'static,
    N: IntoView,
{
    use hydration_context::HydrateSharedContext;
    use std::sync::Arc;

    // use wasm-bindgen-futures to drive the reactive system
    Executor::init_wasm_bindgen();

    // create a new reactive owner and use it as the root node to run the app
    let owner = Owner::new_root(Arc::new(HydrateSharedContext::new()));
    let mountable = owner.with(move || {
        let view = f().into_view();
        view.hydrate::<true>(
            &Cursor::new(parent.unchecked_into()),
            &PositionState::default(),
        )
    });

    // returns a handle that owns the owner
    // when this is dropped, it will clean up the reactive system and unmount the view
    UnmountHandle {
        owner,
        mountable,
        rndr: PhantomData,
    }
}

/// Runs the provided closure and mounts the result to the `<body>`.
pub fn mount_to_body<F, N>(f: F)
where
    F: FnOnce() -> N + 'static,
    N: IntoView,
{
    let owner = mount_to(body(), f);
    owner.forget();
}

/// Runs the provided closure and mounts the result to the provided element.
pub fn mount_to<F, N>(parent: HtmlElement, f: F) -> UnmountHandle<N::State, Dom>
where
    F: FnOnce() -> N + 'static,
    N: IntoView,
{
    // use wasm-bindgen-futures to drive the reactive system
    Executor::init_wasm_bindgen();

    // create a new reactive owner and use it as the root node to run the app
    let owner = Owner::new();
    let mountable = owner.with(move || {
        let view = f().into_view();
        let mut mountable = view.build();
        mountable.mount(&parent, None);
        mountable
    });

    // returns a handle that owns the owner
    // when this is dropped, it will clean up the reactive system and unmount the view
    UnmountHandle {
        owner,
        mountable,
        rndr: PhantomData,
    }
}

/// On drop, this will clean up the reactive [`Owner`] and unmount the view created by
/// [`mount_to`].
///
/// If you are using it to create the root of an application, you should use
/// [`UnmountHandle::forget`] to leak it.
#[must_use]
pub struct UnmountHandle<M, R>
where
    M: Mountable<R>,
    R: Renderer,
{
    owner: Owner,
    mountable: M,
    rndr: PhantomData<R>,
}

impl<M, R> UnmountHandle<M, R>
where
    M: Mountable<R>,
    R: Renderer,
{
    /// Leaks the handle, preventing the reactive system from being cleaned up and the view from
    /// being unmounted. This should always be called when [`mount_to`] is used for the root of an
    /// application that should live for the long term.
    pub fn forget(self) {
        std::mem::forget(self);
    }
}

impl<M, R> Drop for UnmountHandle<M, R>
where
    M: Mountable<R>,
    R: Renderer,
{
    fn drop(&mut self) {
        self.mountable.unmount();
    }
}
