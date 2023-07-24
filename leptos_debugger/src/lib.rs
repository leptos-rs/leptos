#![cfg_attr(feature = "nightly", feature(min_specialization))]

mod message;
mod prop;
#[cfg(feature = "nightly")]
mod prop_value_from;
mod runtime;

pub use message::*;
pub use prop::{Prop, PropValue};
#[cfg(feature = "nightly")]
pub use prop_value_from::PropValueFrom;
use runtime::with_runtime;

pub fn subscribe(hook: impl Fn(Message) + 'static) {
    with_runtime(|runtime| {
        *runtime.hook.borrow_mut() = Some(Box::new(hook));
    });
}

pub fn update_view(message: Message) {
    with_runtime(|runtime| {
        let mut hook = runtime.hook.borrow_mut();
        let hook = hook.as_deref_mut();
        if let Some(hook) = hook {
            hook(message);
        }
    });
}

pub fn update_props(id: &String, props: Vec<Prop>) {
    with_runtime(|runtime| {
        let mut hook = runtime.hook.borrow_mut();
        let hook = hook.as_deref_mut();
        if let Some(hook) = hook {
            hook(
                PropsMessage::Create {
                    id: id.clone(),
                    props,
                }
                .into(),
            );
        }
    });
}
