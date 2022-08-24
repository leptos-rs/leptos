use std::{fmt::Debug, marker::PhantomData, rc::Rc};

use crate::{Computation, Observer, ReadSignal, Scope, SignalState};

pub fn create_memo<T>(cx: Scope, f: impl FnMut(Option<T>) -> T + 'static) -> Memo<T>
where
    T: Debug + Clone + 'static,
{
    // create the computation
    let sig = Rc::new(SignalState::new(cx.system, None));
    let c = Rc::new(Computation::new(cx.system, f, Some(Rc::clone(&sig))));
    Rc::clone(&c).update();
    cx.push_computation(c);

    // generate ReadSignal handle for the memo
    let id = cx.push_signal(sig);
    Memo {
        inner: ReadSignal {
            system: cx.system,
            scope: cx.id,
            id,
            ty: PhantomData,
        },
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Memo<T>
where
    T: Clone + 'static,
{
    inner: ReadSignal<Option<T>>,
}

impl<T> Copy for Memo<T> where T: Clone + 'static {}

impl<T> Memo<T>
where
    T: Clone + 'static,
{
    pub fn get(self) -> T {
        self.with(|val| val.clone())
    }

    pub fn with<U>(self, f: impl Fn(&T) -> U) -> U {
        // unwrap because the effect runs while the memo is being created
        // so there will always be a value here
        self.inner.with(|n| f(n.as_ref().unwrap()))
    }
}

impl<T> FnOnce<()> for Memo<T>
where
    T: Debug + Clone,
{
    type Output = T;

    extern "rust-call" fn call_once(self, _args: ()) -> Self::Output {
        self.get()
    }
}

impl<T> FnMut<()> for Memo<T>
where
    T: Debug + Clone,
{
    extern "rust-call" fn call_mut(&mut self, _args: ()) -> Self::Output {
        self.get()
    }
}

impl<T> Fn<()> for Memo<T>
where
    T: Debug + Clone,
{
    extern "rust-call" fn call(&self, _args: ()) -> Self::Output {
        self.get()
    }
}
