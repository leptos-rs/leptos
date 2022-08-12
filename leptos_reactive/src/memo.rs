use serde::{Deserialize, Serialize};

use crate::{Runtime, Scope, ScopeId, Source, Subscriber};
use std::{
    any::{type_name, Any},
    cell::RefCell,
    collections::HashSet,
    fmt::Debug,
    marker::PhantomData,
};

impl Scope {
    pub fn create_memo<T>(self, f: impl FnMut(Option<&T>) -> T + 'static) -> Memo<T>
    where
        T: Debug + 'static,
    {
        let state = MemoState::new(self.runtime, f);

        let id = self.push_memo(state);

        let eff = Memo {
            runtime: self.runtime,
            scope: self.id,
            id,
            ty: PhantomData,
        };

        self.runtime
            .any_memo((self.id, id), |memo| memo.run((self.id, id)));

        eff
    }
}

#[derive(Debug, PartialEq, Eq, Hash)]
pub struct Memo<T>
where
    T: 'static,
{
    runtime: &'static Runtime,
    pub(crate) scope: ScopeId,
    pub(crate) id: MemoId,
    pub(crate) ty: PhantomData<T>,
}

impl<T> Clone for Memo<T> {
    fn clone(&self) -> Self {
        Self {
            runtime: self.runtime,
            scope: self.scope,
            id: self.id,
            ty: PhantomData,
        }
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
        if let Some(running_subscriber) = self.runtime.running_effect() {
            match running_subscriber {
                Subscriber::Memo(running_memo_id) => {
                    self.runtime.any_memo(running_memo_id, |running_memo| {
                        self.add_subscriber(Subscriber::Memo(running_memo_id));
                        running_memo.subscribe_to(Source::Memo((self.scope, self.id)));
                    });
                }
                Subscriber::Effect(running_effect_id) => {
                    self.runtime
                        .any_effect(running_effect_id, |running_effect| {
                            self.add_subscriber(Subscriber::Effect(running_effect_id));
                            running_effect.subscribe_to(Source::Memo((self.scope, self.id)));
                        });
                }
            }
        }

        // If transition is running, or contains this as a source, write to t_value
        if let Some(transition) = self.runtime.running_transition() {
            todo!()
        } else {
            self.runtime.memo(
                (self.scope, self.id),
                |memo_state: &MemoState<T>| match &*memo_state.value.borrow() {
                    Some(v) => f(v),
                    None => {
                        memo_state.run((self.scope, self.id));
                        f(memo_state.value.borrow().as_ref().unwrap())
                    }
                },
            )
        }
    }

    fn add_subscriber(&self, subscriber: Subscriber) {
        self.runtime
            .memo((self.scope, self.id), |memo_state: &MemoState<T>| {
                memo_state.subscribers.borrow_mut().insert(subscriber);
            })
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub(crate) struct MemoId(pub(crate) usize);

pub(crate) struct MemoState<T>
where
    T: Debug,
{
    runtime: &'static Runtime,
    f: Box<debug_cell::RefCell<dyn FnMut(Option<&T>) -> T>>,
    value: debug_cell::RefCell<Option<T>>,
    t_value: debug_cell::RefCell<Option<T>>,
    sources: debug_cell::RefCell<HashSet<Source>>,
    subscribers: debug_cell::RefCell<HashSet<Subscriber>>,
}

impl<T> Debug for MemoState<T>
where
    T: Debug,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MemoState")
            .field(
                "f",
                &format!(
                    "FnMut<Option<&{}>> -> {}",
                    type_name::<T>(),
                    type_name::<T>()
                ),
            )
            .field("value", &*self.value.borrow())
            .field("t_value", &*self.t_value.borrow())
            .field("sources", &*self.sources.borrow())
            .field("subscribers", &*self.subscribers.borrow())
            .finish()
    }
}

impl<T> MemoState<T>
where
    T: Debug,
{
    pub fn new(runtime: &'static Runtime, f: impl FnMut(Option<&T>) -> T + 'static) -> Self {
        let f = Box::new(debug_cell::RefCell::new(f));

        Self {
            runtime,
            f,
            value: debug_cell::RefCell::new(None),
            sources: Default::default(),
            t_value: Default::default(),
            subscribers: Default::default(),
        }
    }

    pub(crate) fn add_source(&self, source: Source) {
        self.sources.borrow_mut().insert(source);
    }

    fn cleanup(&self, id: (ScopeId, MemoId)) {
        for source in self.sources.borrow().iter() {
            source.unsubscribe(self.runtime, Subscriber::Memo(id))
        }
    }
}

pub(crate) trait AnyMemo: Debug {
    fn run(&self, id: (ScopeId, MemoId));

    fn unsubscribe(&self, subscriber: Subscriber);

    fn as_any(&self) -> &dyn Any;

    fn subscribe_to(&self, source: Source);
}

impl<T> AnyMemo for MemoState<T>
where
    T: Debug + 'static,
{
    fn run(&self, id: (ScopeId, MemoId)) {
        // clear previous dependencies
        // at this point, Effect dependencies have been added to Signal
        // and any Signal changes will call Effect dependency automatically
        self.cleanup(id);

        // add it to the Scope stack, which means any signals called
        // in the effect fn immediately below will add this Effect as a dependency
        self.runtime.push_stack(Subscriber::Memo(id));

        // actually run the effect
        let v = { (self.f.borrow_mut())(self.value.borrow().as_ref()) };
        *self.value.borrow_mut() = Some(v);

        // notify subscribers
        let subs = { self.subscribers.borrow().clone() };
        for subscriber in subs.iter() {
            subscriber.run(self.runtime);
        }

        // pop it back off the stack
        self.runtime.pop_stack();
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn subscribe_to(&self, source: Source) {
        self.add_source(source);
    }

    fn unsubscribe(&self, subscriber: Subscriber) {
        self.subscribers.borrow_mut().remove(&subscriber);
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
