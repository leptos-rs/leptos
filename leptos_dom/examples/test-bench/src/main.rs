#![allow(warnings)]

#[macro_use]
extern crate tracing;

use leptos::*;
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

fn view_fn(cx: Scope) -> impl IntoView {
    view! { cx,
        <h2>"Passing Tests"</h2>
        <ul>
          /* These work! */
          <li><strong>"should be []"</strong></li>
          <Test from=&[1] to=&[]/>
          <Test from=&[1, 2] to=&[]/>
          <Test from=&[1, 2, 3] to=&[]/>
          <hr/>
          <li><strong>"should be [1]"</strong></li>
          <Test from=&[] to=&[1]/>
          <Test from=&[1, 2] to=&[1]/>
          <Test from=&[2, 1] to=&[1]/>
          <hr/>
          <li><strong>"should be [1, 2]"</strong></li>
          <Test from=&[1, 2, 3] to=&[1, 2]/>
          <Test from=&[2] to=&[1, 2]/>
          <Test from=&[1] to=&[1, 2]/>
          <li><strong>"should be [1, 2, 3]"</strong></li>
          <Test from=&[] to=&[1, 2, 3]/>
          <Test from=&[2] to=&[1, 2, 3]/>
          <Test from=&[1] to=&[1, 2, 3]/>
          <Test from=&[1, 3, 2] to=&[1, 2, 3]/>
          <Test from=&[2, 1, 3] to=&[1, 2, 3]/>
        </ul>
        <h2>"Broken Tests"</h2>
        <ul>
          <li><strong>"should be [1, 2, 3]"</strong></li>
          <Test from=&[3] to=&[1, 2, 3]/>
          <Test from=&[3, 1] to=&[1, 2, 3]/>
          <Test from=&[3, 2, 1] to=&[1, 2, 3]/> 
         <hr/>
          <li><strong>"should be [1, 2, 3, 4]"</strong></li>
          <Test from=&[1, 4, 2, 3] to=&[1, 2, 3, 4]/> 
          <hr/>
          <li><strong>"should be [1, 2, 3, 4, 5]"</strong></li>
          <Test from=&[1, 4, 3, 2, 5] to=&[1, 2, 3, 4, 5]/> 
          <Test from=&[4, 5, 3, 1, 2] to=&[1, 2, 3, 4, 5]/> 
        </ul>
    }
}

#[component]
fn Test(
    cx: Scope,
    from: &'static [usize],
    to: &'static [usize],
) -> impl IntoView {
    let (list, set_list) = create_signal(cx, from.to_vec());
    request_animation_frame(move || {
        set_list(to.to_vec());
    });

    view! { cx,
      <li>
          <For
              each=list
              key=|i| *i
              view=|cx, i| {
                  view! { cx, <span>{i}</span> }
              }
          />
        /* <p>
          "Pre | "
          <For
              each=list
              key=|i| *i
              view=|cx, i| {
                  view! { cx, <span>{i}</span> }
              }
          />
          " | Post"
        </p> */
      </li>
    }
}
