use leptos::*;

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
                <span>{move ||
                    counters.get()
                        .iter()
                        .map(|(_, (count, _))| count.get())
                        .sum::<i32>()
                        .to_string()
                }</span>
                " from "
                <span>{move || counters.get().len().to_string()}</span>
                " counters."
            </p>
            <ul>
                <For
                    each=move || counters.get()
                    key=|counter| counter.0
                    children=move |(id, (value, set_value)): (usize, (ReadSignal<i32>, WriteSignal<i32>))| {
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

    // this will run when the scope is disposed, i.e., when this row is deleted
    // because the signal was created in the parent scope, it won't be disposed
    // of until the parent scope is. but we no longer need it, so we'll dispose of
    // it when this row is deleted, instead. if we don't dispose of it here,
    // this memory will "leak," i.e., the signal will continue to exist until the
    // parent component is removed. in the case of this component, where it's the
    // root, that's the lifetime of the program.
    on_cleanup(move || {
        log::debug!("deleted a row");
        value.dispose();
    });

    view! {
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
