use std::{any::Any, fmt::Debug, rc::Rc};

use leptos_reactive::{Memo, ReadSignal, Scope};

use crate::{use_route, Location, ParamsMap};

pub fn use_loader<T>(cx: Scope) -> T
where
    T: Clone + Debug + 'static,
{
    log::debug!("use_loader on cx {:?}\n\n{:#?}", cx.id(), cx);

    let route = use_route(cx);

    log::debug!("use_loader route = {route:#?}");

    let data = route.data().as_ref().unwrap();

    log::debug!("use_loader data = {data:?}");

    let data = data.downcast_ref::<T>().unwrap();

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
