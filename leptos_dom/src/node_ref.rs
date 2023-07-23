use crate::{html::ElementDescriptor, HtmlElement};
use leptos_reactive::{
    create_effect, create_rw_signal, signal_prelude::*, RwSignal, Scope,
};
use std::cell::Cell;

/// Contains a shared reference to a DOM node created while using the `view`
/// macro to create your UI.
///
/// ```
/// # use leptos::*;
///
/// use leptos::html::Input;
///
/// #[component]
/// pub fn MyComponent(cx: Scope) -> impl IntoView {
///     let input_ref = create_node_ref::<Input>(cx);
///
///     let on_click = move |_| {
///         let node =
///             input_ref.get().expect("input_ref should be loaded by now");
///         // `node` is strongly typed
///         // it is dereferenced to an `HtmlInputElement` automatically
///         log!("value is {:?}", node.value())
///     };
///
///     view! {
///       cx,
///       <div>
///       // `node_ref` loads the input
///       <input _ref=input_ref type="text"/>
///       // the button consumes it
///       <button on:click=on_click>"Click me"</button>
///       </div>
///     }
/// }
/// ```
#[repr(transparent)]
pub struct NodeRef<T: ElementDescriptor + 'static>(
    RwSignal<Option<HtmlElement<T>>>,
);

/// Creates a shared reference to a DOM node created while using the `view`
/// macro to create your UI.
///
/// ```
/// # use leptos::*;
///
/// use leptos::html::Input;
///
/// #[component]
/// pub fn MyComponent(cx: Scope) -> impl IntoView {
///     let input_ref = create_node_ref::<Input>(cx);
///
///     let on_click = move |_| {
///         let node =
///             input_ref.get().expect("input_ref should be loaded by now");
///         // `node` is strongly typed
///         // it is dereferenced to an `HtmlInputElement` automatically
///         log!("value is {:?}", node.value())
///     };
///
///     view! {
///       cx,
///       <div>
///       // `node_ref` loads the input
///       <input _ref=input_ref type="text"/>
///       // the button consumes it
///       <button on:click=on_click>"Click me"</button>
///       </div>
///     }
/// }
/// ```
#[inline(always)]
pub fn create_node_ref<T: ElementDescriptor + 'static>(
    cx: Scope,
) -> NodeRef<T> {
    NodeRef(create_rw_signal(cx, None))
}

impl<T: ElementDescriptor + 'static> NodeRef<T> {
    /// Gets the element that is currently stored in the reference.
    ///
    /// This tracks reactively, so that node references can be used in effects.
    /// Initially, the value will be `None`, but once it is loaded the effect
    /// will rerun and its value will be `Some(Element)`.
    #[track_caller]
    #[inline(always)]
    pub fn get(&self) -> Option<HtmlElement<T>>
    where
        T: Clone,
    {
        self.0.get()
    }

    /// Gets the element that is currently stored in the reference.
    ///
    /// This **does not** track reactively.
    #[track_caller]
    #[inline(always)]
    pub fn get_untracked(&self) -> Option<HtmlElement<T>>
    where
        T: Clone,
    {
        self.0.get_untracked()
    }

    #[doc(hidden)]
    /// Loads an element into the reference. This tracks reactively,
    /// so that effects that use the node reference will rerun once it is loaded,
    /// i.e., effects can be forward-declared.
    #[track_caller]
    pub fn load(&self, node: &HtmlElement<T>)
    where
        T: Clone,
    {
        self.0.update(|current| {
            if current.is_some() {
                crate::debug_warn!(
                    "You are setting a NodeRef that has already been filled. \
                     It’s possible this is intentional, but it’s also \
                     possible that you’re accidentally using the same NodeRef \
                     for multiple _ref attributes."
                );
            }
            *current = Some(node.clone());
        });
    }

    /// Runs the provided closure when the `NodeRef` has been connected
    /// with it's [`HtmlElement`].
    #[inline(always)]
    pub fn on_load<F>(self, cx: Scope, f: F)
    where
        T: Clone,
        F: FnOnce(HtmlElement<T>) + 'static,
    {
        let f = Cell::new(Some(f));

        create_effect(cx, move |_| {
            if let Some(node_ref) = self.get() {
                f.take().unwrap()(node_ref);
            }
        });
    }
}

impl<T: ElementDescriptor> Clone for NodeRef<T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T: ElementDescriptor + 'static> Copy for NodeRef<T> {}

cfg_if::cfg_if! {
    if #[cfg(feature = "nightly")] {
        impl<T: Clone + ElementDescriptor + 'static> FnOnce<()> for NodeRef<T> {
            type Output = Option<HtmlElement<T>>;

            #[inline(always)]
            extern "rust-call" fn call_once(self, _args: ()) -> Self::Output {
                self.get()
            }
        }

        impl<T: Clone + ElementDescriptor + 'static> FnMut<()> for NodeRef<T> {
            #[inline(always)]
            extern "rust-call" fn call_mut(&mut self, _args: ()) -> Self::Output {
                self.get()
            }
        }

        impl<T: Clone + ElementDescriptor + Clone + 'static> Fn<()> for NodeRef<T> {
            #[inline(always)]
            extern "rust-call" fn call(&self, _args: ()) -> Self::Output {
                self.get()
            }
        }
    }
}
