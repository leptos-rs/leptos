use leptos::{For, *};

const MANY_COUNTERS: usize = 1000;

type CounterHolder = Vec<(usize, (ReadSignal<i32>, WriteSignal<i32>))>;

#[derive(Copy, Clone)]
struct CounterUpdater {
    set_counters: WriteSignal<CounterHolder>,
}

#[component]
pub fn Counters(cx: Scope) -> impl IntoView {
    let (next_counter_id, set_next_counter_id) = create_signal(cx, 0);
    let (counters, set_counters) = create_signal::<CounterHolder>(cx, vec![]);
    provide_context(cx, CounterUpdater { set_counters });

    let add_counter = move |_| {
        let id = next_counter_id();
        let sig = create_signal(cx, 0);
        set_counters.update(move |counters| counters.push((id, sig)));
        set_next_counter_id.update(|id| *id += 1);
    };

    let add_many_counters = move |_| {
        let next_id = next_counter_id();
        let new_counters = (next_id..next_id + MANY_COUNTERS).map(|id| {
            let signal = create_signal(cx, 0);
            (id, signal)
        });

        set_counters.update(move |counters| counters.extend(new_counters));
        set_next_counter_id.update(|id| *id += MANY_COUNTERS);
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
                {format!("Add {MANY_COUNTERS} Counters")}
            </button>
            <button on:click=clear_counters>
                "Clear Counters"
            </button>
            <p>
                "Total: "
                <span>{move ||
                    counters.get()
                        .iter()
                        .map(|(_, (count, _))| count())
                        .sum::<i32>()
                        .to_string()
                }</span>
                " from "
                <span>{move || counters().len().to_string()}</span>
                " counters."
            </p>
            <ul>
                <For
                    each=counters
                    key=|counter| counter.0
                    view=move |cx, (id, (value, set_value)): (usize, (ReadSignal<i32>, WriteSignal<i32>))| {
                        view! { cx,
                            <Counter id value set_value/>
                        }
                    }
                />
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
) -> impl IntoView {
    let CounterUpdater { set_counters } = use_context(cx).unwrap();

    let input = move |ev| {
        set_value(event_target_value(&ev).parse::<i32>().unwrap_or_default())
    };

    // just an example of how a cleanup function works
    // this will run when the scope is disposed, i.e., when this row is deleted
    on_cleanup(cx, || log::debug!("deleted a row"));

    view! { cx,
        <li>
            <button on:click=move |_| set_value.update(move |value| *value -= 1)>"-1"</button>
            <input type="text"
                prop:value={value}
                on:input=input
            />
            <span>{value}</span>
            <button on:click=move |_| set_value.update(move |value| *value += 1)>"+1"</button>
            <button on:click=move |_| set_counters.update(move |counters| counters.retain(|(counter_id, _)| counter_id != &id))>"x"</button>
        </li>
    }
}
