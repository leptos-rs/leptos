use leptos::prelude::*;

const MANY_COUNTERS: usize = 1000;

type CounterHolder = Vec<(usize, ArcRwSignal<i32>)>;

#[derive(Copy, Clone)]
struct CounterUpdater {
    set_counters: WriteSignal<CounterHolder>,
}

#[component]
pub fn Counters() -> impl IntoView {
    let (next_counter_id, set_next_counter_id) = signal(0);
    let (counters, set_counters) = signal::<CounterHolder>(vec![]);
    provide_context(CounterUpdater { set_counters });

    let add_counter = move |_| {
        let id = next_counter_id.get();
        let sig = ArcRwSignal::new(0);
        set_counters.update(move |counters| counters.push((id, sig)));
        set_next_counter_id.update(|id| *id += 1);
    };

    let add_many_counters = move |_| {
        let next_id = next_counter_id.get();
        let new_counters = (next_id..next_id + MANY_COUNTERS).map(|id| {
            let signal = ArcRwSignal::new(0);
            (id, signal)
        });

        set_counters.update(move |counters| counters.extend(new_counters));
        set_next_counter_id.update(|id| *id += MANY_COUNTERS);
    };

    let clear_counters = move |_| {
        set_counters.update(|counters| counters.clear());
    };

    view! {
        <div>
            <button on:click=add_counter>"Add Counter"</button>
            <button on:click=add_many_counters>{format!("Add {MANY_COUNTERS} Counters")}</button>
            <button on:click=clear_counters>"Clear Counters"</button>
            <p>
                "Total: "
                <span data-testid="total">
                    {move || {
                        counters.get().iter().map(|(_, count)| count.get()).sum::<i32>().to_string()
                    }}

                </span> " from "
                <span data-testid="counters">{move || counters.get().len().to_string()}</span>
                " counters."
            </p>
            <ul>
                <For
                    each=move || counters.get()
                    key=|counter| counter.0
                    children=move |(id, value)| {
                        view! { <Counter id value/> }
                    }
                />

            </ul>
        </div>
    }
}

#[component]
fn Counter(id: usize, value: ArcRwSignal<i32>) -> impl IntoView {
    let value = RwSignal::from(value);
    let CounterUpdater { set_counters } = use_context().unwrap();

    view! {
        <li>
            <button on:click=move |_| value.update(move |value| *value -= 1)>"-1"</button>
            <input
                type="text"
                prop:value=value
                on:input:target=move |ev| {
                    value.set(ev.target().value().parse::<i32>().unwrap_or_default())
                }
            />

            <span>{value}</span>
            <button on:click=move |_| value.update(move |value| *value += 1)>"+1"</button>
            <button on:click=move |_| {
                set_counters
                    .update(move |counters| counters.retain(|(counter_id, _)| counter_id != &id))
            }>"x"</button>
        </li>
    }
}
