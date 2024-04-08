use wasm_bindgen::JsCast;
use wasm_bindgen_test::*;

wasm_bindgen_test_configure!(run_in_browser);
use leptos::*;
use portal::App;
use web_sys::HtmlButtonElement;

async fn next_tick() {
    gloo_timers::future::TimeoutFuture::new(25).await;
}

#[wasm_bindgen_test]
async fn portal() {
    let document = leptos::document();
    let body = document.body().unwrap();

    let div = document.create_element("div").unwrap();
    div.set_id("app");
    let _ = body.append_child(&div);

    mount_to(div.clone().unchecked_into(), || view! { <App/> });

    let show_button = document
        .get_element_by_id("btn-show")
        .unwrap()
        .unchecked_into::<HtmlButtonElement>();

    show_button.click();

    next_tick().await;

    // check HTML
    assert_eq!(
        div.inner_html(),
           "<!-- <App> --><div><button id=\"btn-show\">Show Overlay</button><!-- <Show> --><!-- <DynChild> --><!-- <> --><div>Show</div><!-- <Portal> --><!-- <() /> --><!-- </Portal> --><!-- </> --><!-- </DynChild> --><!-- </Show> --></div><!-- </App> --><div><!-- <> --><div style=\"position: fixed; z-index: 10; width: 100vw; height: 100vh; top: 0; left: 0; background: rgba(0, 0, 0, 0.8); color: white;\"><p>This is in the body element</p><button id=\"btn-hide\">Close Overlay</button><button id=\"btn-toggle\">Toggle inner</button><!-- <Show> --><!-- <DynChild> -->Hidden<!-- </DynChild> --><!-- </Show> --></div><!-- </> --></div>"
        );

    let toggle_button = document
        .get_element_by_id("btn-toggle")
        .unwrap()
        .unchecked_into::<HtmlButtonElement>();

    toggle_button.click();

    assert_eq!(
        div.inner_html(),
        "<!-- <App> --><div><button id=\"btn-show\">Show Overlay</button><!-- <Show> --><!-- <DynChild> --><!-- <> --><div>Show</div><!-- <Portal> --><!-- <() /> --><!-- </Portal> --><!-- </> --><!-- </DynChild> --><!-- </Show> --></div><!-- </App> --><div><!-- <> --><div style=\"position: fixed; z-index: 10; width: 100vw; height: 100vh; top: 0; left: 0; background: rgba(0, 0, 0, 0.8); color: white;\"><p>This is in the body element</p><button id=\"btn-hide\">Close Overlay</button><button id=\"btn-toggle\">Toggle inner</button><!-- <Show> --><!-- <DynChild> --><!-- <> -->Visible<!-- </> --><!-- </DynChild> --><!-- </Show> --></div><!-- </> --></div>"
    );

    let hide_button = document
        .get_element_by_id("btn-hide")
        .unwrap()
        .unchecked_into::<HtmlButtonElement>();

    hide_button.click();

    assert_eq!(
        div.inner_html(),
       "<!-- <App> --><div><button id=\"btn-show\">Show Overlay</button><!-- <Show> --><!-- <DynChild> --><!-- <() /> --><!-- </DynChild> --><!-- </Show> --></div><!-- </App> -->"
    );
}
