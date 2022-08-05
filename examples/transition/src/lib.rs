use std::{pin::Pin, time::Duration};

use futures::{
    channel::oneshot::{self, Canceled},
    Future,
};
use leptos::*;

pub fn transition_tabs(cx: Scope) -> web_sys::Element {
    let (tab, set_tab) = cx.create_signal(0);
    let transition = cx.use_transition();

    view! {
        <div>
            <nav class="tabs">
                <button class:selected={move || *tab.get() == 0} on:click=move |_| set_tab(|n| *n = 0)>
                    "One"
                </button>
                <button class:selected={move || *tab.get() == 1} on:click=move |_| set_tab(|n| *n = 1)>
                    "Two"
                </button>
                <button class:selected={move || *tab.get() == 2} on:click=move |_| set_tab(|n| *n = 2)>
                    "Three"
                </button>
            </nav>
            <p>{move || tab.get().to_string()}</p>
            <div class="tab">
                <Suspense fallback=view! { <div class="loader">"Loading..."</div> }>
                    //{|| view! { <p>"test"</p> }}
                    {move || view! { <Child page=tab.clone() /> }}
                </Suspense>
            </div>
        </div>
    }
}

#[component]
pub fn Child(cx: Scope, page: ReadSignal<usize>) -> Element {
    let data = cx.create_ref(cx.create_resource(&page, |page| fake_data_load(*page)));

    view! {
        <div class="tab-content">
            <p>
                <Suspense fallback=view! { <div class="loader">"Lower suspense..."</div> }>
                    {move || match &*data.read() {
                        ResourceState::Idle => view! { <p>"(no data)"</p> },
                        ResourceState::Pending { .. } => view! { <p>"Loading..."</p> },
                        ResourceState::Ready { data } => view! {
                            <p>{data}</p>
                        }
                    }}
                </Suspense>
            </p>
        </div>
    }
}

async fn fake_data_load(page: usize) -> String {
    delay(Duration::from_millis(500)).await;
    let page_data = vec![
        "Lorem ipsum dolor sit amet, consectetur adipiscing elit, sed do eiusmod tempor incididunt ut labore et dolore magna aliqua. Nec dui nunc mattis enim ut tellus elementum sagittis vitae. Quam elementum pulvinar etiam non. Sed faucibus turpis in eu mi. Convallis a cras semper auctor neque vitae tempus quam pellentesque. Duis tristique sollicitudin nibh sit amet. Elementum curabitur vitae nunc sed velit dignissim sodales. Nibh venenatis cras sed felis eget velit aliquet sagittis. In pellentesque massa placerat duis. Integer quis auctor elit sed vulputate mi sit amet mauris. Luctus accumsan tortor posuere ac ut consequat semper. Lorem ipsum dolor sit amet consectetur adipiscing elit. Sed faucibus turpis in eu mi bibendum neque egestas. Dictumst vestibulum rhoncus est pellentesque elit.",
        "Placerat orci nulla pellentesque dignissim. Non curabitur gravida arcu ac. Sed odio morbi quis commodo odio aenean sed. Quam elementum pulvinar etiam non quam lacus. Est lorem ipsum dolor sit. Turpis massa sed elementum tempus egestas sed sed. Quam nulla porttitor massa id neque aliquam vestibulum morbi blandit. Aenean pharetra magna ac placerat. Donec et odio pellentesque diam volutpat commodo sed. Eget duis at tellus at urna condimentum. Rhoncus dolor purus non enim praesent elementum facilisis leo vel. Velit laoreet id donec ultrices. Aliquet eget sit amet tellus cras adipiscing enim eu.",
        "At consectetur lorem donec massa sapien faucibus et. Vivamus at augue eget arcu dictum. Phasellus vestibulum lorem sed risus ultricies tristique. Nulla aliquet enim tortor at. In tellus integer feugiat scelerisque varius morbi enim nunc. Suspendisse sed nisi lacus sed viverra tellus in. Turpis tincidunt id aliquet risus feugiat in ante metus dictum. Sem viverra aliquet eget sit amet tellus. Enim blandit volutpat maecenas volutpat. Bibendum enim facilisis gravida neque. Ornare quam viverra orci sagittis eu. Urna cursus eget nunc scelerisque viverra mauris. Nibh mauris cursus mattis molestie a. Eget egestas purus viverra accumsan in nisl nisi. Congue eu consequat ac felis donec et. Vulputate dignissim suspendisse in est ante in nibh. Faucibus scelerisque eleifend donec pretium vulputate sapien nec sagittis. Augue neque gravida in fermentum et sollicitudin ac orci phasellus. Id faucibus nisl tincidunt eget nullam non nisi."
    ];
    page_data[page].to_string()
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
