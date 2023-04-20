use crate::{IntoView, View};
use leptos_reactive::Scope;
use std::{any::Any, fmt, rc::Rc};

/// Wrapper for arbitrary data that can be passed through the view.
#[derive(Clone)]
#[repr(transparent)]
pub struct Transparent(Rc<dyn Any>);

impl Transparent {
    /// Creates a new wrapper for this data.
    #[inline(always)]
    pub fn new<T>(value: T) -> Self
    where
        T: 'static,
    {
        Self(Rc::new(value))
    }

    /// Returns some reference to the inner value if it is of type `T`, or `None` if it isn't.
    #[inline(always)]
    pub fn downcast_ref<T>(&self) -> Option<&T>
    where
        T: 'static,
    {
        self.0.downcast_ref()
    }
}

impl fmt::Debug for Transparent {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("Transparent").finish()
    }
}

impl PartialEq for Transparent {
    #[inline(always)]
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(&self.0, &other.0)
    }
}

impl Eq for Transparent {}

impl IntoView for Transparent {
    #[inline(always)]
    fn into_view(self, _: Scope) -> View {
        View::Transparent(self)
    }
}
