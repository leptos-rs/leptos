use std::{any::Any, future::Future, pin::Pin, rc::Rc};

use crate::{Location, ParamsMap};

#[derive(Clone)]
pub struct DataFunction {
    data: Rc<dyn Fn(ParamsMap, Location) -> Pin<Box<dyn Future<Output = Box<dyn Any>>>>>,
}

impl<F, Fu, T> From<F> for DataFunction
where
    F: Fn(ParamsMap, Location) -> Fu + Clone + 'static,
    Fu: Future<Output = T>,
    T: Any + 'static,
{
    fn from(f: F) -> Self {
        Self {
            data: Rc::new(move |params, location| {
                Box::pin({
                    let f = f.clone();
                    async move {
                        let data = f(params, location).await;
                        Box::new(data) as Box<dyn Any>
                    }
                })
            }),
        }
    }
}

impl std::fmt::Debug for DataFunction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DataFunction").finish()
    }
}
