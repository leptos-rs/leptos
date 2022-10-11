use std::{fmt::Display, pin::Pin, time::Duration};

use futures::{
    channel::oneshot::{self, Canceled},
    Future,
};
use leptos::*;

#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Tab {
    A,
    B,
    C,
}

impl Display for Tab {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Tab::A => "Tab A",
                Tab::B => "Tab B",
                Tab::C => "Tab C",
            }
        )
    }
}

pub fn transition_tabs(cx: Scope) -> web_sys::Element {
    let (tab, set_tab) = create_signal(cx, Tab::A);
    let (progress, set_progress) = create_signal(cx, 0);

    let transition = use_transition(cx);

    create_effect(cx, move |handle: Option<Option<IntervalHandle>>| {
        if let Some(Some(handle)) = handle {
            if transition.pending() {
                Some(handle)
            } else {
                handle.clear();
                None
            }
        } else {
            if transition.pending() {
                set_progress(0);
                Some(
                    set_interval(move || set_progress.update(|n| *n += 1), Duration::from_millis(10))
                        .unwrap(),
                )
            } else {
                None
            }
        }
    });

    view! { cx, 
        <div>
            <progress class:visible={move || transition.pending()} value={move || progress().to_string()} max="40"></progress>
            <nav class="tabs" class:pending={move || transition.pending()}>
                <button class:selected={move || tab() == Tab::A} on:click=move |_| transition.start(move || set_tab(Tab::A))>
                    "One"
                </button>
                <button class:selected={move || tab() == Tab::B} on:click=move |_| transition.start(move || set_tab(Tab::B))>
                    "Two"
                </button>
                <button class:selected={move || tab() == Tab::C} on:click=move |_| transition.start(move || set_tab(Tab::C))>
                    "Three"
                </button>
            </nav>
            <p>{move || tab.get().to_string()}</p>
            <div class="tab">
                //<Suspense fallback=view! { cx,  <div class="loader">"Loading..."</div> }>
                    {move || view! { cx,  <Child page=tab /> }}
                //</Suspense>
            </div>
        </div>
    }
}

#[component]
pub fn Child(cx: Scope, page: ReadSignal<Tab>) -> Element {
    let data = create_resource(cx, page, |page| fake_data_load(page));

    view! { cx, 
        <div class="tab-content">
            <p>
                //<Suspense fallback=view! { cx,  <div class="loader">"Lower suspense..."</div> }>
                    {move || data.read().map(|data| view! { cx, 
                        <div>
                            <p>{data}</p>
                        </div>
                    })}
                //</Suspense>
            </p>
        </div>
    }
}

async fn fake_data_load(page: Tab) -> String {
    delay(Duration::from_millis(400)).await;
    let page_data = vec![
        "Lorem ipsum dolor sit amet, consectetur adipiscing elit, sed do eiusmod tempor incididunt ut labore et dolore magna aliqua. Nec dui nunc mattis enim ut tellus elementum sagittis vitae. Quam elementum pulvinar etiam non. Sed faucibus turpis in eu mi. Convallis a cras semper auctor neque vitae tempus quam pellentesque. Duis tristique sollicitudin nibh sit amet. Elementum curabitur vitae nunc sed velit dignissim sodales. Nibh venenatis cras sed felis eget velit aliquet sagittis. In pellentesque massa placerat duis. Integer quis auctor elit sed vulputate mi sit amet mauris. Luctus accumsan tortor posuere ac ut consequat semper. Lorem ipsum dolor sit amet consectetur adipiscing elit. Sed faucibus turpis in eu mi bibendum neque egestas. Dictumst vestibulum rhoncus est pellentesque elit.",
        "Placerat orci nulla pellentesque dignissim. Non curabitur gravida arcu ac. Sed odio morbi quis commodo odio aenean sed. Quam elementum pulvinar etiam non quam lacus. Est lorem ipsum dolor sit. Turpis massa sed elementum tempus egestas sed sed. Quam nulla porttitor massa id neque aliquam vestibulum morbi blandit. Aenean pharetra magna ac placerat. Donec et odio pellentesque diam volutpat commodo sed. Eget duis at tellus at urna condimentum. Rhoncus dolor purus non enim praesent elementum facilisis leo vel. Velit laoreet id donec ultrices. Aliquet eget sit amet tellus cras adipiscing enim eu.",
        "At consectetur lorem donec massa sapien faucibus et. Vivamus at augue eget arcu dictum. Phasellus vestibulum lorem sed risus ultricies tristique. Nulla aliquet enim tortor at. In tellus integer feugiat scelerisque varius morbi enim nunc. Suspendisse sed nisi lacus sed viverra tellus in. Turpis tincidunt id aliquet risus feugiat in ante metus dictum. Sem viverra aliquet eget sit amet tellus. Enim blandit volutpat maecenas volutpat. Bibendum enim facilisis gravida neque. Ornare quam viverra orci sagittis eu. Urna cursus eget nunc scelerisque viverra mauris. Nibh mauris cursus mattis molestie a. Eget egestas purus viverra accumsan in nisl nisi. Congue eu consequat ac felis donec et. Vulputate dignissim suspendisse in est ante in nibh. Faucibus scelerisque eleifend donec pretium vulputate sapien nec sagittis. Augue neque gravida in fermentum et sollicitudin ac orci phasellus. Id faucibus nisl tincidunt eget nullam non nisi."
    ];
    page_data[page as usize].to_string()
}

fn delay(duration: Duration) -> Pin<Box<dyn Future<Output = Result<(), Canceled>>>> {
    let (tx, rx) = oneshot::channel();
    set_timeout(
        move || {
            tx.send(());
        },
        duration,
    );
    Box::pin(rx)
}
