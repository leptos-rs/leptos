// allow because our ReadSignal call requires &'a ReadSignal
// hopefully this will make more sense at some point
#![allow(clippy::needless_borrow)]
#![feature(fn_traits)]
#![feature(unboxed_closures)]

mod effect;
mod root_context;
mod scope;
mod scope_arena;
mod signal;

pub use effect::*;
pub use root_context::*;
pub use scope::*;
pub use signal::*;

#[cfg(test)]
mod tests {
    use crate::{create_scope, root_context::RootContext};

    #[test]
    fn compute_signal() {
        let stack = Box::leak(Box::new(RootContext::new()));
        let _ = create_scope(stack, |cx| {
            let (a, set_a) = cx.create_signal(0);
            let (b, set_b) = cx.create_signal(0);

            let c = cx.create_memo(|| *(&a)() + *(&b)());

            assert_eq!(*c.get_untracked(), 0);

            set_a(|n| *n = 2);

            assert_eq!(*c.get_untracked(), 2);

            set_b(|n| *n = 2);

            assert_eq!(*c.get_untracked(), 4);
        });
    }

    #[test]
    fn memo_with_conditional_branches() {
        let stack = Box::leak(Box::new(RootContext::new()));
        let _ = create_scope(stack, |cx| {
            let (first_name, set_first_name) = cx.create_signal("Greg");
            let (last_name, set_last_name) = cx.create_signal("Johnston");
            let (show_last_name, set_show_last_name) = cx.create_signal(true);

            let out = cx.create_memo(move || {
                if *(&show_last_name)() {
                    format!("{} {}", *(&first_name)(), *(&last_name)())
                } else {
                    (*(&first_name)()).to_string()
                }
            });

            assert_eq!(*out.get_untracked(), "Greg Johnston");

            set_first_name(|n| *n = "Murray");
            assert_eq!(*out.get_untracked(), "Murray Johnston");

            set_show_last_name(|n| *n = false);
            assert_eq!(*out.get_untracked(), "Murray");

            set_last_name(|n| *n = "Kenney");
            assert_eq!(*out.get_untracked(), "Murray");

            set_show_last_name(|n| *n = true);
            assert_eq!(*out.get_untracked(), "Murray Kenney");
        });
    }
}
