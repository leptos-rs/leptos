use leptos::prelude::*;
use rand::prelude::*;
use std::sync::atomic::{AtomicUsize, Ordering};
static ADJECTIVES: &[&str] = &[
    "pretty",
    "large",
    "big",
    "small",
    "tall",
    "short",
    "long",
    "handsome",
    "plain",
    "quaint",
    "clean",
    "elegant",
    "easy",
    "angry",
    "crazy",
    "helpful",
    "mushy",
    "odd",
    "unsightly",
    "adorable",
    "important",
    "inexpensive",
    "cheap",
    "expensive",
    "fancy",
];

static COLOURS: &[&str] = &[
    "red", "yellow", "blue", "green", "pink", "brown", "purple", "brown",
    "white", "black", "orange",
];

static NOUNS: &[&str] = &[
    "table", "chair", "house", "bbq", "desk", "car", "pony", "cookie",
    "sandwich", "burger", "pizza", "mouse", "keyboard",
];

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct RowData {
    id: usize,
    label: ArcRwSignal<String>,
}

static ID_COUNTER: AtomicUsize = AtomicUsize::new(1);

fn build_data(count: usize) -> Vec<RowData> {
    let mut thread_rng = thread_rng();

    let mut data = Vec::new();
    data.reserve_exact(count);

    for _i in 0..count {
        let adjective = ADJECTIVES.choose(&mut thread_rng).unwrap();
        let colour = COLOURS.choose(&mut thread_rng).unwrap();
        let noun = NOUNS.choose(&mut thread_rng).unwrap();
        let capacity = adjective.len() + colour.len() + noun.len() + 2;
        let mut label = String::with_capacity(capacity);
        label.push_str(adjective);
        label.push(' ');
        label.push_str(colour);
        label.push(' ');
        label.push_str(noun);

        data.push(RowData {
            id: ID_COUNTER.load(Ordering::Relaxed),
            label: ArcRwSignal::new(label),
        });

        ID_COUNTER
            .store(ID_COUNTER.load(Ordering::Relaxed) + 1, Ordering::Relaxed);
    }

    data
}

/// Button component.
#[component]
fn Button(
    /// ID for the button element
    id: &'static str,
    /// Text that should be included
    text: &'static str,
) -> impl IntoView {
    view! {
        <div class="col-sm-6 smallpad">
            <button id=id class="btn btn-primary btn-block" type="button">
                {text}
            </button>
        </div>
    }
}

#[component]
pub fn App() -> impl IntoView {
    let (data, set_data) = signal(Vec::<RowData>::new());
    let (selected, set_selected) = signal(None::<usize>);

    let remove = move |id: usize| {
        set_data.update(move |data| data.retain(|row| row.id != id));
    };

    let run = move |_| {
        set_data.set(build_data(1000));
        set_selected.set(None);
    };

    let run_lots = move |_| {
        set_data.set(build_data(10000));
        set_selected.set(None);
    };

    let add = move |_| {
        set_data.update(move |data| data.append(&mut build_data(1000)));
    };

    let update = move |_| {
        data.with(|data| {
            for row in data.iter().step_by(10) {
                row.label.update(|n| n.push_str(" !!!"));
            }
        });
    };

    let clear = move |_| {
        set_data.set(Vec::new());
        set_selected.set(None);
    };

    let swap_rows = move |_| {
        set_data.update(|data| {
            if data.len() > 998 {
                data.swap(1, 998);
            }
        });
    };

    let is_selected = Selector::new(move || selected.get());

    view! {
        <div class="container">
            <div class="jumbotron">
                <div class="row">
                    <div class="col-md-6">
                        <h1>"Leptos"</h1>
                    </div>
                    <div class="col-md-6">
                        <div class="row">
                            <Button id="run" text="Create 1,000 rows" on:click=run />
                            <Button id="runlots" text="Create 10,000 rows" on:click=run_lots />
                            <Button id="add" text="Append 1,000 rows" on:click=add />
                            <Button id="update" text="Update every 10th row" on:click=update />
                            <Button id="clear" text="Clear" on:click=clear />
                            <Button id="swaprows" text="Swap Rows" on:click=swap_rows />
                        </div>
                    </div>
                </div>
            </div>
            <table class="table table-hover table-striped test-data">
                <tbody>
                    <For
                        each=move || data.get()
                        key=|row| row.id
                        children=move |row: RowData| {
                            let row_id = row.id;
                            let label = row.label;
                            let is_selected = is_selected.clone();
                            template! {
                                < tr class : danger = { move || is_selected.selected(&Some(row_id)) }
                                > < td class = "col-md-1" > { row_id.to_string() } </ td > < td
                                class = "col-md-4" >< a on : click = move | _ | set_selected
                                .set(Some(row_id)) > { move || label.get() } </ a ></ td > < td
                                class = "col-md-1" >< a on : click = move | _ | remove(row_id) ><
                                span class = "glyphicon glyphicon-remove" aria - hidden = "true" ></
                                span ></ a ></ td > < td class = "col-md-6" /> </ tr >
                            }
                        }
                    />

                </tbody>
            </table>
            <span class="preloadicon glyphicon glyphicon-remove" aria-hidden="true"></span>
        </div>
    }
}
