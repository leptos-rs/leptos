use wasm_bindgen_test::*;

wasm_bindgen_test_configure!(run_in_browser);
use leptos::*;
use web_sys::HtmlElement;

use counters::{Counters, CountersProps};

#[wasm_bindgen_test]
fn inc() {
    mount_to_body(|cx| view! { cx,  <Counters/> });

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
    assert_eq!(div.inner_html(), "<button>Add Counter</button><button>Add 1000 Counters</button><button>Clear Counters</button><p>Total: <span>0</span> from <span>3</span> counters.</p><ul><li><button>-1</button><input type=\"text\"><span>0</span><button>+1</button><button>x</button></li><li><button>-1</button><input type=\"text\"><span>0</span><button>+1</button><button>x</button></li><li><button>-1</button><input type=\"text\"><span>0</span><button>+1</button><button>x</button></li></ul>");

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

    assert_eq!(div.inner_html(), "<button>Add Counter</button><button>Add 1000 Counters</button><button>Clear Counters</button><p>Total: <span>6</span> from <span>3</span> counters.</p><ul><li><button>-1</button><input type=\"text\"><span>1</span><button>+1</button><button>x</button></li><li><button>-1</button><input type=\"text\"><span>2</span><button>+1</button><button>x</button></li><li><button>-1</button><input type=\"text\"><span>3</span><button>+1</button><button>x</button></li></ul>");

    // remove the first counter
    counters
        .item(0)
        .unwrap()
        .last_child()
        .unwrap()
        .unchecked_into::<HtmlElement>()
        .click();

    assert_eq!(div.inner_html(), "<button>Add Counter</button><button>Add 1000 Counters</button><button>Clear Counters</button><p>Total: <span>5</span> from <span>2</span> counters.</p><ul><li><button>-1</button><input type=\"text\"><span>2</span><button>+1</button><button>x</button></li><li><button>-1</button><input type=\"text\"><span>3</span><button>+1</button><button>x</button></li></ul>");

    // decrement all by 1
    for idx in 0..counters.length() {
        let counter = counters.item(idx).unwrap();
        let dec_button = counter
            .first_child()
            .unwrap()
            .unchecked_into::<HtmlElement>();
        dec_button.click();
    }

    run_scope(create_runtime(), move |cx| {
        // we can use RSX in test comparisons!
        // note that if RSX template creation is bugged, this probably won't catch it
        // (because the same bug will be reproduced in both sides of the assertion)
        // so I use HTML tests for most internal testing like this
        // but in user-land testing, RSX comparanda are cool
        assert_eq!(
            div.outer_html(),
            view! { cx,
                <div>
                    <button>"Add Counter"</button>
                    <button>"Add 1000 Counters"</button>
                    <button>"Clear Counters"</button>
                    <p>"Total: "<span>"3"</span>" from "<span>"2"</span>" counters."</p>
                    <ul>
                        <li>
                            <button>"-1"</button>
                            <input type="text"/>
                            <span>"1"</span>
                            <button>"+1"</button>
                            <button>"x"</button>
                        </li>
                        <li>
                            <button>"-1"</button>
                            <input type="text"/>
                            <span>"2"</span>
                            <button>"+1"</button>
                            <button>"x"</button>
                        </li>
                    </ul>
                </div>
            }
            .outer_html()
        );
    });
}
