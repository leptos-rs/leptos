use std::{any::Any, future::Future, pin::Pin};

use crate::{Location, Params};

pub struct DataFunction {
    data: Box<dyn Fn(Params, Location) -> Pin<Box<dyn Future<Output = Box<dyn Any>>>>>,
}

impl<F, Fu, T> From<F> for DataFunction
where
    F: Fn(Params, Location) -> Fu + Clone + 'static,
    Fu: Future<Output = T>,
    T: Any + 'static,
{
    fn from(f: F) -> Self {
        Self {
            data: Box::new(move |params, location| {
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
