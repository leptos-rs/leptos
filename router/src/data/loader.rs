use std::{any::Any, fmt::Debug, rc::Rc};

use leptos_reactive::{debug_warn, Memo, Scope};

use crate::{use_route, Location, ParamsMap};

pub fn use_loader<T>(cx: Scope) -> T
where
    T: Clone + Debug + 'static,
{
    let route = use_route(cx);
    let data = match route.data().as_ref() {
        Some(data) => data,
        None => {
            debug_warn!("(use_loader) could not find any data for route");
            panic!()
        }
    };
    let data = match data.downcast_ref::<T>() {
        Some(data) => data,
        None => {
            debug_warn!("(use_loader) could not downcast data to requested type");
            panic!()
        }
    };
    data.clone()
}

#[derive(Clone)]
pub struct Loader {
    pub(crate) data: Rc<dyn Fn(Scope, Memo<ParamsMap>, Location) -> Box<dyn Any>>,
}

impl<F, T> From<F> for Loader
where
    F: Fn(Scope, Memo<ParamsMap>, Location) -> T + 'static,
    T: Any + 'static,
{
    fn from(f: F) -> Self {
        Self {
            data: Rc::new(move |cx, params, location| Box::new(f(cx, params, location))),
        }
    }
}

impl std::fmt::Debug for Loader {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Loader").finish()
    }
}
