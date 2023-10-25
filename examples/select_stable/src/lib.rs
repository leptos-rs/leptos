use leptos::*;
use log::info;
use log::error;

#[component]
pub fn Selector() -> impl IntoView {
    #[derive(Debug, Clone, Copy)]
    pub struct SelectOption {
        pub label: &'static str,
        pub pos: usize,
    }
    let options: Vec<SelectOption> =  vec![
        SelectOption{ label: "h0rses", pos: 0},
        SelectOption{ label: "b1rds", pos: 1},
        SelectOption{ label: "2nfish", pos: 2},
    ];

    let options_clone = options.clone();
    let selection: RwSignal<Option<usize>> = create_rw_signal(Some(1)); // initial selection

    view! {
      <div style="background:#ffffbf">
      <h2>Selector</h2>
      <select
        id = "selector"
        on:change = move |ev| {
          let new_selection = event_target_value(&ev);
          if new_selection.is_empty() {
            selection.set(None);
          } else {
            match new_selection.parse() {
              Ok(v) => {
                info!("You selected {}", v);
                selection.set(Some(v))
              },
              Err(_) => {
                error!("Error: Unexpected option value {new_selection}");
              },
            }
          }
        }
      >
      <For
        each = move || options_clone.clone()
        key = |option| option.pos
        let:option
          >
          <option
            value = option.pos
            selected = (selection.get() == Some(option.pos))
            >
            { option.label }
          </option>
      </For>
      </select>
        <p>
        "You selected: "
        <span data-testid="my_selection">{move || {
          match selection.get() {
            Some(v) => {
              options[v].label
            },
            None => "No idea..."
          }
        }
        }</span>
        </p>
      </div>
    }
}

#[component]
pub fn Dynamic_selector() -> impl IntoView {
    #[derive(Debug, Clone, Copy)]
    pub struct SelectionOption {
        pub label: &'static str,
        pub id: u32,
        pub amount: RwSignal<u32>,
    }
    let selection_options_default: Vec<SelectionOption> = vec![
        SelectionOption{ label: "h0rses", id: 0, amount: create_rw_signal::<u32>(0)},
        SelectionOption{ label: "b1rds", id: 1, amount: create_rw_signal::<u32>(10)},
        SelectionOption{ label: "2nfish", id: 2, amount: create_rw_signal::<u32>(20)},
        SelectionOption{ label: "3lk", id: 3, amount: create_rw_signal::<u32>(30)},
    ];
    let default_selection: RwSignal<Option<u32>> = create_rw_signal(Some(0));
    let selection_options = create_rw_signal::<Vec<SelectionOption>>(selection_options_default);
    let print = move |_| {
        let s = selection_options.get();
        info!("{} {}", s[0].label, s[0].amount.get());
    };
    let add_option = move |_| {
        let a = SelectionOption{ label: "4eel", id: 4, amount: create_rw_signal::<u32>(20)};
        selection_options.update(|n| {
            if n.len() > 0 {
                n[0].amount.set(200);
                info!("n[0]: {} {}", n[0].label, n[0].amount.get())
            }
            n.push(a);
        });
    };
    view! {
      <div style="background:#eae3ff">
        <h2>Dynamic_selector</h2>
        <select
          id = "dynamic_selector"
          on:change = move |ev| {
            let target_value = event_target_value(&ev);
            if target_value.is_empty() {
              default_selection.set(None);
            } else {
               match target_value.parse() {
                 Ok(v) => {
                   info!("You selected {}", v);
                   default_selection.set(Some(v))
                 },
                 Err(_) => {
                   error!("Error: Unexpected option value {target_value}");
                 },
              }
            }
          }
        >
          <For
            each = {move || selection_options.clone().get()}
            key = |option| option.id
            let:option
          >
            <option
              value = move || option.id
              default_selection = (default_selection.get() == Some(option.id))
            > { move || {
               let z = option.amount.get();
               let v = format!("{} - {}", option.label, z);
               v
               }
              }
            </option>
          </For>
        </select>
        <p>
        "You selected: "
        <span data-testid="mymultiselection">{move || {
          let selected_option = default_selection.get();
            match selected_option {
                Some(v) => {
                    let l = selection_options.get()[v as usize].label;
                    let a = selection_options.get()[v as usize].amount.get();
                    format!("{} - {}", l, a.to_string())
                },
                None => "no idea...".to_string()
            }
        }
        }</span>
         <button on:click=add_option>
           "Add another option to selector"
         </button>
         <button on:click=print>
           "Print"
         </button>
        </p>
      </div>
    }
}
