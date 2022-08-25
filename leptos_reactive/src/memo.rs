use crate::{create_effect, create_signal, ReadSignal, Scope};
use std::fmt::Debug;

pub type Memo<T> = ReadSignal<T>;

pub fn create_memo<T>(cx: Scope, mut f: impl FnMut(Option<T>) -> T + 'static) -> Memo<T>
where
    T: Clone + Debug + 'static,
{
    let initial = f(None);
    let (read, set) = create_signal(cx, initial);

    create_effect(cx, move |prev| {
        let new = f(prev);
        set(|n| *n = new.clone());
        new
    });

    read
}
