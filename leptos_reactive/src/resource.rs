use std::{future::Future, rc::Rc};

use crate::{spawn_local, ReadSignal, Scope, WriteSignal};

pub struct Resource<S, T, Fu>
where
    S: 'static,
    T: 'static,
    Fu: Future<Output = T>,
{
    pub data: ReadSignal<Option<T>>,
    set_data: WriteSignal<Option<T>>,
    source: ReadSignal<S>,
    fetcher: Rc<dyn Fn(&S) -> Fu>,
}

impl<S, T, Fu> Resource<S, T, Fu>
where
    S: 'static,
    T: 'static,
    Fu: Future<Output = T> + 'static,
{
    pub fn new(cx: Scope, source: ReadSignal<S>, fetcher: impl Fn(&S) -> Fu + 'static) -> Self {
        // create signals to handle response
        let (data, set_data) = cx.create_signal_owned(None);
        let fetcher = Rc::new(fetcher);

        cx.create_effect({
            let source = source.clone();
            let set_data = set_data.clone();
            let fetcher = Rc::clone(&fetcher);
            move || {
                let fut = (fetcher)(&source.get());

                let set_data = set_data.clone();
                spawn_local(async move {
                    let res = fut.await;
                    set_data.update(move |data| *data = Some(res));
                });
            }
        });

        // return the Resource synchronously
        Self {
            data,
            set_data,
            source,
            fetcher,
        }
    }

    pub fn refetch(&self) {
        let source = self.source.clone();
        let set_data = self.set_data.clone();
        let fetcher = Rc::clone(&self.fetcher);
        let fut = (fetcher)(&source.get());

        spawn_local(async move {
            let res = fut.await;
            set_data.update(move |data| *data = Some(res));
        });
    }

    pub fn mutate(&self, update_fn: impl FnOnce(&mut Option<T>)) {
        self.set_data.update(update_fn);
    }
}
