use leptos::*;

fn main() {
    _ = console_log::init_with_level(log::Level::Debug);
    console_error_panic_hook::set_once();
    mount_to_body(|| view! { <Counters/> })
}

const MANY_COUNTERS: usize = 1000;

type CounterHolder = Vec<(usize, (ReadSignal<i32>, WriteSignal<i32>))>;

#[derive(Copy, Clone)]
struct CounterUpdater {
    set_counters: WriteSignal<CounterHolder>,
}

#[component]
pub fn Counters() -> impl IntoView {
    let (next_counter_id, set_next_counter_id) = create_signal(0);
    let (counters, set_counters) = create_signal::<CounterHolder>(vec![]);
    provide_context(CounterUpdater { set_counters });

    let add_counter = move |_| {
        let id = next_counter_id.get();
        let sig = create_signal(0);
        set_counters.update(move |counters| counters.push((id, sig)));
        set_next_counter_id.update(|id| *id += 1);
    };

    let add_many_counters = move |_| {
        let next_id = next_counter_id.get();
        let new_counters = (next_id..next_id + MANY_COUNTERS).map(|id| {
            let signal = create_signal(0);
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
                <span data-testid="total">{move ||
                    counters.get()
                        .iter()
                        .map(|(_, (count, _))| count.get())
                        .sum::<i32>()
                        .to_string()
                }</span>
                " from "
                <span data-testid="counters">{move || counters.with(|counters| counters.len()).to_string()}</span>
                " counters."
            </p>
            <ul>
                <For
                    each={move || counters.get()}
                    key={|counter| counter.0}
                    view=move |(id, (value, set_value))| {
                        view! {
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
    id: usize,
    value: ReadSignal<i32>,
    set_value: WriteSignal<i32>,
) -> impl IntoView {
    let CounterUpdater { set_counters } = use_context().unwrap();

    let input = move |ev| {
        set_value
            .set(event_target_value(&ev).parse::<i32>().unwrap_or_default())
    };

    view! {
        <li>
            <button id="decrement_count" on:click=move |_| set_value.update(move |value| *value -= 1)>"-1"</button>
            <input type="text"
                prop:value={move || value.get().to_string()}
                on:input=input
            />
            <span>{value}</span>
            <button id="increment_count" on:click=move |_| set_value.update(move |value| *value += 1)>"+1"</button>
            <button on:click=move |_| set_counters.update(move |counters| counters.retain(|(counter_id, _)| counter_id != &id))>"x"</button>
        </li>
    }
}
