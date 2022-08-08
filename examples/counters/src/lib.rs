use leptos::*;
use leptos::{For, ForProps};

type CounterHolder = Vec<(usize, (ReadSignal<i32>, WriteSignal<i32>))>;

struct CounterUpdater {
    set_counters: WriteSignal<CounterHolder>,
}

#[component]
pub fn Counters(cx: Scope) -> web_sys::Element {
    let (next_counter_id, set_next_counter_id) = cx.create_signal(0);
    let (counters, set_counters) = cx.create_signal::<CounterHolder>(Vec::new());
    cx.provide_context(CounterUpdater {
        set_counters: (*set_counters).clone(),
    });

    let add_counter = move |_| {
        let id = next_counter_id();
        let (read, write) = cx.create_signal(0);
        set_counters(|counters| counters.push((id, (read.clone(), write.clone()))));
        set_next_counter_id(|id| *id += 1);
    };

    let add_many_counters = move |_| {
        let mut new_counters = vec![];
        for next_id in 0..1000 {
            let signal = cx.create_signal(0);
            new_counters.push((next_id, (signal.0.clone(), signal.1.clone())));
        }
        set_counters(move |n| *n = new_counters.clone());
    };

    let clear_counters = move |_| {
        set_counters(|counters| counters.clear());
    };

    view! {
        <div>
            <button on:click=add_counter>
                "Add Counter"
            </button>
            <button on:click=add_many_counters>
                "Add 1000 Counters"
            </button>
            <button on:click=clear_counters>
                "Clear Counters"
            </button>
            <p>
                "Total: "
                <span>{move ||
                    counters.get()
                        .iter()
                        .map(|(_, (count, _))| *count.get())
                        .sum::<i32>()
                        .to_string()
                }</span>
                " from "
                <span>{move || counters.get().len().to_string()}</span>
                " counters."
            </p>
            <ul>
                <For each={counters} key={|counter| counter.0}>{
                    |cx, (id, (value, set_value))| {
                        view! {
                            <Counter id=id value=value.clone() set_value=set_value.clone()/>
                        }
                    }
                }</For>
            </ul>
        </div>
    }
}

#[component]
fn Counter(
    cx: Scope,
    id: usize,
    value: ReadSignal<i32>,
    set_value: WriteSignal<i32>,
) -> web_sys::Element {
    let CounterUpdater { set_counters } = cx.use_context().unwrap_throw();

    let input = {
        let set_value = set_value.clone();
        move |ev| {
            set_value(|value| *value = event_target_value(&ev).parse::<i32>().unwrap_or_default())
        }
    };

    view! {
        <li>
            <button on:click={let set_value = set_value.clone(); move |_| set_value(|value| *value -= 1)}>"-1"</button>
            <input type="text"
                prop:value={let value = value.clone(); move || value.get().to_string()}
                on:input=input
            />
            <span>{move || value.get().to_string()}</span>
            <button on:click=move |_| set_value(|value| *value += 1)>"+1"</button>
            <button on:click=move |_| set_counters(|counters| counters.retain(|(counter_id, _)| counter_id != &id))>"x"</button>
        </li>
    }
}
