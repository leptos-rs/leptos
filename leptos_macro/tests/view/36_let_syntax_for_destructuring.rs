// Using `let(Counter { id, count })` destructuring syntax inside a `<For>`
// loop to extract fields from each item. This should compile without errors.

use leptos::prelude::*;

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
struct Counter {
    id: usize,
    count: RwSignal<i32>,
}

#[component]
fn Counters() -> impl IntoView {
    let (counters, _set_counters) = signal::<Vec<Counter>>(vec![]);

    view! {
      <div>
          <For
            each=move || counters.get()
            key=|counter: &Counter| counter.id
            let(Counter { id, count })
          >
              <button>"Value (" {id} "): " {move || count.get()}</button>
          </For>
      </div>
    }
}

fn main() {}
