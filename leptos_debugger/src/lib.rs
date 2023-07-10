#![cfg_attr(feature = "nightly", feature(min_specialization))]

mod node;
mod prop;
#[cfg(feature = "nightly")]
mod prop_value_from;
mod runtime;

pub use node::DNode;
use node::{create_root_tree, remove_tree};
pub use prop::{Prop, PropValue};
#[cfg(feature = "nightly")]
pub use prop_value_from::PropValueFrom;
use runtime::with_runtime;
use std::collections::HashMap;

pub struct HookConfig {
    pub get_root_tree: Box<dyn Fn() -> DNode + 'static>,
}

pub trait Hook {
    fn set_config(&mut self, config: HookConfig);
    fn create_root(&mut self);
    fn update_view(&mut self);
}

pub fn set_debugger_hook(mut hook: impl Hook + 'static) {
    hook.set_config(HookConfig {
        get_root_tree: Box::new(move || {
            with_runtime(|runtime| create_root_tree(runtime))
        }),
    });
    with_runtime(|runtime| {
        *runtime.hook.borrow_mut() = Some(Box::new(hook));
    });
}

pub fn create_root() {
    with_runtime(|runtime| {
        runtime.config.borrow_mut().is_root = true;

        let mut hook = runtime.hook.borrow_mut();
        let hook = hook.as_deref_mut();
        if let Some(hook) = hook {
            hook.create_root();
        }
    });
}

pub fn insert_view(key: String, value: DNode) {
    with_runtime(|runtime| {
        let mut nodes = runtime.nodes.borrow_mut();
        if let Some(vec) = nodes.get_mut(&key) {
            vec.push(value);
        } else {
            nodes.insert(key, vec![value]);
        }
    });

    update_view()
}

pub fn remove_view(key: &String) {
    with_runtime(|runtime| {
        remove_tree(runtime, key);
    });

    update_view()
}

pub fn update_props(key: &String, value: Vec<Prop>) {
    with_runtime(|runtime| {
        let mut props = runtime.props.borrow_mut();
        if let Some(map) = props.get_mut(key) {
            value.iter().for_each(|v| {
                let key = v.key.clone();
                map.insert(key, v.clone());
            });
        } else {
            let mut map = HashMap::new();
            value.iter().for_each(|v| {
                let key = v.key.clone();
                map.insert(key, v.clone());
            });
            props.insert(key.clone(), map);
        }
    });
    update_view()
}

fn update_view() {
    with_runtime(|runtime| {
        if !runtime.config.borrow().is_root {
            return;
        }
        let mut hook = runtime.hook.borrow_mut();
        let hook = hook.as_deref_mut();
        if let Some(hook) = hook {
            hook.update_view();
        }
    });
}
