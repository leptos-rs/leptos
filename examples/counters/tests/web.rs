use wasm_bindgen::JsCast;
use wasm_bindgen_test::*;

wasm_bindgen_test_configure!(run_in_browser);
use counters::Counters;
use leptos::*;
use web_sys::HtmlElement;

#[wasm_bindgen_test]
fn inc() {
    mount_to_body(|| view! { <Counters/> });

    let document = leptos::document();
    let div = document.query_selector("div").unwrap().unwrap();
    let add_counter = div
        .first_child()
        .unwrap()
        .dyn_into::<HtmlElement>()
        .unwrap();

    // add 3 counters
    add_counter.click();
    add_counter.click();
    add_counter.click();

    // check HTML
    assert_eq!(
        div.inner_html(),
        "<button>Add Counter</button><button>Add 1000 \
         Counters</button><button>Clear Counters</button><p>Total: <span><!-- \
         <DynChild> -->0<!-- </DynChild> --></span> from <span><!-- \
         <DynChild> -->3<!-- </DynChild> --></span> counters.</p><ul><!-- \
         <Each> --><!-- <EachItem> --><!-- <Counter> \
         --><li><button>-1</button><input type=\"text\"><span><!-- <DynChild> \
         -->0<!-- </DynChild> \
         --></span><button>+1</button><button>x</button></li><!-- </Counter> \
         --><!-- </EachItem> --><!-- <EachItem> --><!-- <Counter> \
         --><li><button>-1</button><input type=\"text\"><span><!-- <DynChild> \
         -->0<!-- </DynChild> \
         --></span><button>+1</button><button>x</button></li><!-- </Counter> \
         --><!-- </EachItem> --><!-- <EachItem> --><!-- <Counter> \
         --><li><button>-1</button><input type=\"text\"><span><!-- <DynChild> \
         -->0<!-- </DynChild> \
         --></span><button>+1</button><button>x</button></li><!-- </Counter> \
         --><!-- </EachItem> --><!-- </Each> --></ul>"
    );

    let counters = div
        .query_selector("ul")
        .unwrap()
        .unwrap()
        .unchecked_into::<HtmlElement>()
        .children();

    // click first counter once, second counter twice, etc.
    // `NodeList` isn't a `Vec` so we iterate over it in this slightly awkward way
    for idx in 0..counters.length() {
        let counter = counters.item(idx).unwrap();
        let inc_button = counter
            .first_child()
            .unwrap()
            .next_sibling()
            .unwrap()
            .next_sibling()
            .unwrap()
            .next_sibling()
            .unwrap()
            .unchecked_into::<HtmlElement>();
        for _ in 0..=idx {
            inc_button.click();
        }
    }

    assert_eq!(
        div.inner_html(),
        "<button>Add Counter</button><button>Add 1000 \
         Counters</button><button>Clear Counters</button><p>Total: <span><!-- \
         <DynChild> -->6<!-- </DynChild> --></span> from <span><!-- \
         <DynChild> -->3<!-- </DynChild> --></span> counters.</p><ul><!-- \
         <Each> --><!-- <EachItem> --><!-- <Counter> \
         --><li><button>-1</button><input type=\"text\"><span><!-- <DynChild> \
         -->1<!-- </DynChild> \
         --></span><button>+1</button><button>x</button></li><!-- </Counter> \
         --><!-- </EachItem> --><!-- <EachItem> --><!-- <Counter> \
         --><li><button>-1</button><input type=\"text\"><span><!-- <DynChild> \
         -->2<!-- </DynChild> \
         --></span><button>+1</button><button>x</button></li><!-- </Counter> \
         --><!-- </EachItem> --><!-- <EachItem> --><!-- <Counter> \
         --><li><button>-1</button><input type=\"text\"><span><!-- <DynChild> \
         -->3<!-- </DynChild> \
         --></span><button>+1</button><button>x</button></li><!-- </Counter> \
         --><!-- </EachItem> --><!-- </Each> --></ul>"
    );

    // remove the first counter
    counters
        .item(0)
        .unwrap()
        .last_child()
        .unwrap()
        .unchecked_into::<HtmlElement>()
        .click();

    assert_eq!(
        div.inner_html(),
        "<button>Add Counter</button><button>Add 1000 \
         Counters</button><button>Clear Counters</button><p>Total: <span><!-- \
         <DynChild> -->5<!-- </DynChild> --></span> from <span><!-- \
         <DynChild> -->2<!-- </DynChild> --></span> counters.</p><ul><!-- \
         <Each> --><!-- <EachItem> --><!-- <EachItem> --><!-- <Counter> \
         --><li><button>-1</button><input type=\"text\"><span><!-- <DynChild> \
         -->2<!-- </DynChild> \
         --></span><button>+1</button><button>x</button></li><!-- </Counter> \
         --><!-- </EachItem> --><!-- <EachItem> --><!-- <Counter> \
         --><li><button>-1</button><input type=\"text\"><span><!-- <DynChild> \
         -->3<!-- </DynChild> \
         --></span><button>+1</button><button>x</button></li><!-- </Counter> \
         --><!-- </EachItem> --><!-- </Each> --></ul>"
    );
}
