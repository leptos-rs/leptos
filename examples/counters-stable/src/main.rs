use leptos::*;

fn main() {
    console_error_panic_hook::set_once();
    _ = console_log::init_with_level(log::Level::Debug);
    console_error_panic_hook::set_once();
    mount_to_body(|cx| view! { cx,  <Counters/> })
}

type CounterHolder = Vec<(usize, (ReadSignal<i32>, WriteSignal<i32>))>;

#[derive(Copy, Clone)]
struct CounterUpdater {
    set_counters: WriteSignal<CounterHolder>,
}

#[component]
pub fn Counters(cx: Scope) -> web_sys::Element {
    let (next_counter_id, set_next_counter_id) = create_signal(cx, 0);
    let (counters, set_counters) = create_signal::<CounterHolder>(cx, vec![]);
    provide_context(cx, CounterUpdater { set_counters });

    let add_counter = move |_| {
        let id = next_counter_id.get();
        let sig = create_signal(cx, 0);
        set_counters.update(move |counters| counters.push((id, sig)));
        set_next_counter_id.update(|id| *id += 1);
    };

    let add_many_counters = move |_| {
        let mut new_counters = vec![];
        for next_id in 0..1000 {
            let signal = create_signal(cx, 0);
            new_counters.push((next_id, signal));
        }
        set_counters.update(|counters| counters.extend(new_counters.iter()));
    };

    let clear_counters = move |_| {
        set_counters.update(|counters| counters.clear());
    };

    view! { cx,
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
                        .map(|(_, (count, _))| count.get())
                        .sum::<i32>()
                        .to_string()
                }</span>
                " from "
                <span>{move || counters.with(|counters| counters.len()).to_string()}</span>
                " counters."
            </p>
            <ul>
                <For each={move || counters.get()} key={|counter| counter.0}>{
                    |cx, (id, (value, set_value)): &(usize, (ReadSignal<i32>, WriteSignal<i32>))| {
                        view! {
                            cx,
                            <Counter id=*id value=*value set_value=*set_value/>
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
    let CounterUpdater { set_counters } = use_context(cx).unwrap_throw();

    let input = move |ev| set_value.set(event_target_value(&ev).parse::<i32>().unwrap_or_default());

    view! { cx,
        <li>
            <button on:click={move |_| set_value.update(move |value| *value -= 1)}>"-1"</button>
            <input type="text"
                prop:value={move || value.get().to_string()}
                on:input=input
            />
            <span>{move || value.get().to_string()}</span>
            <button on:click=move |_| set_value.update(move |value| *value += 1)>"+1"</button>
            <button on:click=move |_| set_counters.update(move |counters| counters.retain(|(counter_id, _)| counter_id != &id))>"x"</button>
        </li>
    }
}
