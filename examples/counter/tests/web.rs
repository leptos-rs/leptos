use counter::*;
use leptos::mount::mount_to;
use leptos::prelude::*;
use leptos::task::tick;
use wasm_bindgen::JsCast;
use wasm_bindgen_test::*;

wasm_bindgen_test_configure!(run_in_browser);

#[wasm_bindgen_test]
async fn clear() {
    let document = document();
    let test_wrapper = document.create_element("section").unwrap();
    let _ = document.body().unwrap().append_child(&test_wrapper);

    // start by rendering our counter and mounting it to the DOM
    // note that we start at the initial value of 10
    let _dispose = mount_to(
        test_wrapper.clone().unchecked_into(),
        || view! { <SimpleCounter initial_value=10 step=1/> },
    );

    // now we extract the buttons by iterating over the DOM
    // this would be easier if they had IDs
    let div = test_wrapper.query_selector("div").unwrap().unwrap();
    let clear = test_wrapper
        .query_selector("button")
        .unwrap()
        .unwrap()
        .unchecked_into::<web_sys::HtmlElement>();

    // now let's click the `clear` button
    clear.click();

    // the reactive system is built on top of the async system, so changes are not reflected
    // synchronously in the DOM
    // in order to detect the changes here, we'll just yield for a brief time after each change,
    // allowing the effects that update the view to run
    tick().await;

    // now let's test the <div> against the expected value
    // we can do this by testing its `outerHTML`
    assert_eq!(div.outer_html(), {
        // it's as if we're creating it with a value of 0, right?
        let (value, _set_value) = signal(0);

        // we can remove the event listeners because they're not rendered to HTML
        view! {
            <div>
                <button>"Clear"</button>
                <button>"-1"</button>
                <span>"Value: " {value} "!"</span>
                <button>"+1"</button>
            </div>
        }
        // Leptos supports multiple backend renderers for HTML elements
        // .into_view() here is just a convenient way of specifying "use the regular DOM renderer"
        .into_view()
        // views are lazy -- they describe a DOM tree but don't create it yet
        // calling .build() will actually build the DOM elements
        .build()
        // .build() returned an ElementState, which is a smart pointer for
        // a DOM element. So we can still just call .outer_html(), which access the outerHTML on
        // the actual DOM element
        .outer_html()
    });

    // There's actually an easier way to do this...
    // We can just test against a <SimpleCounter/> with the initial value 0
    assert_eq!(test_wrapper.inner_html(), {
        let comparison_wrapper = document.create_element("section").unwrap();
        let _dispose = mount_to(
            comparison_wrapper.clone().unchecked_into(),
            || view! { <SimpleCounter initial_value=0 step=1/>},
        );
        comparison_wrapper.inner_html()
    });
}

#[wasm_bindgen_test]
async fn inc() {
    let document = document();
    let test_wrapper = document.create_element("section").unwrap();
    let _ = document.body().unwrap().append_child(&test_wrapper);

    let _dispose = mount_to(
        test_wrapper.clone().unchecked_into(),
        || view! { <SimpleCounter initial_value=0 step=1/> },
    );

    // You can do testing with vanilla DOM operations
    let div = test_wrapper.query_selector("div").unwrap().unwrap();
    let clear = div
        .first_child()
        .unwrap()
        .dyn_into::<web_sys::HtmlElement>()
        .unwrap();
    let dec = clear
        .next_sibling()
        .unwrap()
        .dyn_into::<web_sys::HtmlElement>()
        .unwrap();
    let text = dec
        .next_sibling()
        .unwrap()
        .dyn_into::<web_sys::HtmlElement>()
        .unwrap();
    let inc = text
        .next_sibling()
        .unwrap()
        .dyn_into::<web_sys::HtmlElement>()
        .unwrap();

    inc.click();
    inc.click();

    tick().await;

    assert_eq!(text.text_content(), Some("Value: 2!".to_string()));

    dec.click();
    dec.click();
    dec.click();
    dec.click();

    tick().await;

    assert_eq!(text.text_content(), Some("Value: -2!".to_string()));

    clear.click();

    tick().await;

    assert_eq!(text.text_content(), Some("Value: 0!".to_string()));

    // Or you can test against a sample view!
    assert_eq!(
        div.outer_html(),
        {
            let (value, _) = signal(0);
            view! {
                <div>
                    <button>"Clear"</button>
                    <button>"-1"</button>
                    <span>"Value: " {value} "!"</span>
                    <button>"+1"</button>
                </div>
            }
        }
        .into_view()
        .build()
        .outer_html()
    );

    inc.click();

    tick().await;

    assert_eq!(
        div.outer_html(),
        {
            // because we've clicked, it's as if the signal is starting at 1
            let (value, _) = signal(1);
            view! {
                <div>
                    <button>"Clear"</button>
                    <button>"-1"</button>
                    <span>"Value: " {value} "!"</span>
                    <button>"+1"</button>
                </div>
            }
        }
        .into_view()
        .build()
        .outer_html()
    );
}
