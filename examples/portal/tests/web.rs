use wasm_bindgen::JsCast;
use wasm_bindgen_test::*;

wasm_bindgen_test_configure!(run_in_browser);
use leptos::*;
use portal::App;
use web_sys::HtmlButtonElement;

#[wasm_bindgen_test]
fn inc() {
    mount_to_body(|| view! { <App/> });

    let document = leptos::document();
    let div = document.query_selector("div").unwrap().unwrap();
    let show_button = document
        .get_element_by_id("btn-show")
        .unwrap()
        .unchecked_into::<HtmlButtonElement>();

    show_button.click();

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

    let toggle_button = document
        .get_element_by_id("btn-toggle")
        .unwrap()
        .unchecked_into::<HtmlButtonElement>();

    toggle_button.click();

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

    let hide_button = document
        .get_element_by_id("btn-hide")
        .unwrap()
        .unchecked_into::<HtmlButtonElement>();

    hide_button.click();

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
