#![feature(iter_intersperse)]
#![allow(warnings)]

#[macro_use]
extern crate tracing;

use leptos::prelude::*;
use tracing::field::debug;
use tracing_subscriber::util::SubscriberInitExt;

fn main() {
    console_error_panic_hook::set_once();

    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::TRACE)
        .without_time()
        .with_file(true)
        .with_line_number(true)
        .with_target(false)
        .with_writer(tracing_subscriber_wasm::MakeConsoleWriter::default())
        .with_ansi(false)
        .pretty()
        .finish()
        .init();

    mount_to_body(view_fn);
}

fn view_fn() -> impl IntoView {
    view! {
        <h2>"Passing Tests"</h2>
        <ul>
            <Test from=[1] to=[]/>
            <Test from=[1, 2] to=[3, 2] then=vec![2]/>
            <Test from=[1, 2] to=[]/>
            <Test from=[1, 2, 3] to=[]/>
            <hr/>
            <Test from=[] to=[1]/>
            <Test from=[1, 2] to=[1]/>
            <Test from=[2, 1] to=[1]/>
            <hr/>
            <Test from=[1, 2, 3] to=[1, 2]/>
            <Test from=[2] to=[1, 2]/>
            <Test from=[1] to=[1, 2]/>
            <Test from=[] to=[1, 2, 3]/>
            <Test from=[2] to=[1, 2, 3]/>
            <Test from=[1] to=[1, 2, 3]/>
            <Test from=[1, 3, 2] to=[1, 2, 3]/>
            <Test from=[2, 1, 3] to=[1, 2, 3]/>
            <Test from=[3] to=[1, 2, 3]/>
            <Test from=[3, 1] to=[1, 2, 3]/>
            <Test from=[3, 2, 1] to=[1, 2, 3]/>
            <hr/>
            <Test from=[1, 4, 2, 3] to=[1, 2, 3, 4]/>
            <hr/>
            <Test from=[1, 4, 3, 2, 5] to=[1, 2, 3, 4, 5]/>
            <Test from=[4, 5, 3, 1, 2] to=[1, 2, 3, 4, 5]/>
        </ul>
    }
}

#[component]
fn Test<From, To>(
    from: From,
    to: To,
    #[prop(optional)] then: Option<Vec<usize>>,
) -> impl IntoView
where
    From: IntoIterator<Item = usize>,
    To: IntoIterator<Item = usize>,
{
    let from = from.into_iter().collect::<Vec<_>>();
    let to = to.into_iter().collect::<Vec<_>>();

    let (list, set_list) = create_signal(from.clone());
    request_animation_frame({
        let to = to.clone();
        let then = then.clone();
        move || {
            set_list(to);

            if let Some(then) = then {
                request_animation_frame({
                    move || {
                        set_list(then);
                    }
                });
            }
        }
    });

    view! {
        <li>
            "from: [" {move || {
                from
                    .iter()
                    .map(ToString::to_string)
                    .intersperse(", ".to_string())
                    .collect::<String>()
            }} "]" <br/> "to: [" {
                let then = then.clone();
                move || {
                    then
                        .clone()
                        .unwrap_or(to.iter().copied().collect())
                        .iter()
                        .map(ToString::to_string)
                        .intersperse(", ".to_string())
                        .collect::<String>()
                }
            } "]" <br/> "result: ["
            <For
                each=list
                key=|i| *i
                view=|i| {
                    view! { <span>{i} ", "</span> }
                }
            /> "]"
        </li>
    }
}

// fn view_fn(cx: Scope) -> impl IntoView {
//     let (should_show_a, sett_should_show_a) = create_signal(cx, true);

//     let a = vec![2];
//     let b = vec![1, 2, 3];

//     view! { cx,
//       <button on:click=move |_| sett_should_show_a.update(|show| *show = !*show)>"Toggle"</button>

//       <For
//         each={move || if should_show_a.get() {
//           a.clone()
//         } else {
//           b.clone()
//         }}
//         key=|i| *i
//         view=|cx, i| view! { cx, <h1>{i}</h1> }
//       />
//     }
// }
