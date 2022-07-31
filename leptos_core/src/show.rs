use std::marker::PhantomData;

use crate as leptos;
use leptos_dom::{Child, Element, IntoChild};
use leptos_macro::*;
use leptos_reactive::{ReadSignal, Scope};

#[derive(Props)]
pub struct ShowProps<W, C>
where
    W: Fn() -> bool,
    C: for<'a> IntoChild<'a>,
{
    when: W,
    children: C,
}

#[allow(non_snake_case)]
pub fn Show<'a, W, C>(cx: Scope<'a>, props: ShowProps<W, C>) -> impl Fn() -> Child<'a>
where
    W: Fn() -> bool,
    C: for<'c> IntoChild<'c> + Clone,
{
    move || {
        if (props.when)() {
            props.children.clone().into_child(cx)
        } else {
            Child::Null
        }
    }
}
