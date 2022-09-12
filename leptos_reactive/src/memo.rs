use crate::{create_isomorphic_effect, create_signal, ReadSignal, Scope};
use std::fmt::Debug;

#[derive(Debug, PartialEq, Eq)]
pub struct Memo<T>(ReadSignal<Option<T>>)
where
    T: 'static;

pub fn create_memo<T>(cx: Scope, mut f: impl FnMut(Option<T>) -> T + 'static) -> Memo<T>
where
    T: PartialEq + Clone + Debug + 'static,
{
    let (read, set) = create_signal(cx, None);

    create_isomorphic_effect(cx, move |prev| {
        let new = f(prev.clone());
        if prev.as_ref() != Some(&new) {
            set(|n| *n = Some(new.clone()));
        }
        new
    });

    Memo(read)
}

impl<T> Clone for Memo<T>
where
    T: 'static,
{
    fn clone(&self) -> Self {
        Self(self.0)
    }
}

impl<T> Copy for Memo<T> {}

impl<T> Memo<T>
where
    T: Debug,
{
    pub fn get(&self) -> T
    where
        T: Clone,
    {
        self.with(T::clone)
    }

    pub fn with<U>(&self, f: impl Fn(&T) -> U) -> U {
        // okay to unwrap here, because the value will *always* have initially
        // been set by the effect, synchronously
        self.0.with(|n| f(n.as_ref().unwrap()))
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
