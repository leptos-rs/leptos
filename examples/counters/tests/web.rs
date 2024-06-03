use wasm_bindgen::JsCast;
use wasm_bindgen_test::*;

wasm_bindgen_test_configure!(run_in_browser);
use counters::Counters;
use gloo_timers::future::TimeoutFuture;
use leptos::prelude::*;
use web_sys::HtmlElement;

#[wasm_bindgen_test]
async fn inc() {
    mount_to_body(Counters);

    let document = document();
    let div = document.query_selector("div").unwrap().unwrap();
    let add_counter = div
        .first_child()
        .unwrap()
        .dyn_into::<HtmlElement>()
        .unwrap();

    assert_eq!(
        div.inner_html(),
        "<button>Add Counter</button><button>Add 1000 \
         Counters</button><button>Clear Counters</button><p>Total: \
         <span>0</span> from <span>0</span> counters.</p><ul></ul>"
    );

    // add 3 counters
    add_counter.click();
    add_counter.click();
    add_counter.click();

    TimeoutFuture::new(10).await;

    // check HTML
    assert_eq!(
        div.inner_html(),
        "<button>Add Counter</button><button>Add 1000 \
         Counters</button><button>Clear Counters</button><p>Total: \
         <span>0</span> from <span>3</span> \
         counters.</p><ul><li><button>-1</button><input \
         type=\"text\"><span>0</span><button>+1</button><button>x</button></\
         li><li><button>-1</button><input \
         type=\"text\"><span>0</span><button>+1</button><button>x</button></\
         li><li><button>-1</button><input \
         type=\"text\"><span>0</span><button>+1</button><button>x</button></\
         li></ul>"
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

    TimeoutFuture::new(10).await;

    assert_eq!(
        div.inner_html(),
        "<button>Add Counter</button><button>Add 1000 \
         Counters</button><button>Clear Counters</button><p>Total: \
         <span>6</span> from <span>3</span> \
         counters.</p><ul><li><button>-1</button><input \
         type=\"text\"><span>1</span><button>+1</button><button>x</button></\
         li><li><button>-1</button><input \
         type=\"text\"><span>2</span><button>+1</button><button>x</button></\
         li><li><button>-1</button><input \
         type=\"text\"><span>3</span><button>+1</button><button>x</button></\
         li></ul>"
    );

    // remove the first counter
    counters
        .item(0)
        .unwrap()
        .last_child()
        .unwrap()
        .unchecked_into::<HtmlElement>()
        .click();

    TimeoutFuture::new(10).await;

    assert_eq!(
        div.inner_html(),
        "<button>Add Counter</button><button>Add 1000 \
         Counters</button><button>Clear Counters</button><p>Total: \
         <span>5</span> from <span>2</span> \
         counters.</p><ul><li><button>-1</button><input \
         type=\"text\"><span>2</span><button>+1</button><button>x</button></\
         li><li><button>-1</button><input \
         type=\"text\"><span>3</span><button>+1</button><button>x</button></\
         li></ul>"
    );
}
