use crate::{IntoView, View};
use std::{any::Any, rc::Rc};
use leptos_reactive::Scope;

/// Wrapper for arbitrary data that can be passed through the view.
#[derive(Clone)]
pub struct Transparent(Rc<dyn Any>);

impl Transparent {
	/// Creates a new wrapper for this data.
	pub fn new<T>(value: T) -> Self where T: 'static {
		Self(Rc::new(value))
	}

	/// Returns some reference to the inner value if it is of type `T`, or `None` if it isn't.
	pub fn downcast_ref<T>(&self) -> Option<&T> where T: 'static {
		self.0.downcast_ref()
	}
}

impl std::fmt::Debug for Transparent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("Transparent").finish()
    }
}

impl PartialEq for Transparent {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(&self.0, &other.0)
    }
}

impl Eq for Transparent { }

impl IntoView for Transparent {
	fn into_view(self, cx: Scope) -> View {
		View::Transparent(self)
	}
}