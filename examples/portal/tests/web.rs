#![allow(dead_code)]

use leptos::{leptos_dom::helpers::document, mount::mount_to, task::tick};
use portal::App;
use wasm_bindgen::JsCast;
use wasm_bindgen_test::*;
use web_sys::HtmlButtonElement;
wasm_bindgen_test_configure!(run_in_browser);

fn minify(html: &str) -> String {
    let mut result = String::with_capacity(html.len());
    let mut in_tag = false;
    let mut last_char_was_tag_end = false;
    let mut whitespace_buffer = String::new();

    for c in html.chars() {
        match c {
            '<' => {
                // Starting a new tag
                in_tag = true;
                last_char_was_tag_end = false;

                // Discard any buffered whitespace
                whitespace_buffer.clear();

                result.push(c);
            }
            '>' => {
                // Ending a tag
                in_tag = false;
                last_char_was_tag_end = true;
                result.push(c);
            }
            c if c.is_whitespace() => {
                if in_tag {
                    // Preserve whitespace inside tags
                    result.push(c);
                } else if !last_char_was_tag_end {
                    // Buffer whitespace between content
                    whitespace_buffer.push(c);
                }
                // Whitespace immediately after a tag end is ignored
            }
            _ => {
                // Regular character
                last_char_was_tag_end = false;

                // If we have buffered whitespace and are outputting content,
                // preserve a single space
                if !whitespace_buffer.is_empty() {
                    result.push(' ');
                    whitespace_buffer.clear();
                }

                result.push(c);
            }
        }
    }

    result
}

#[wasm_bindgen_test]
async fn portal() {
    let document = document();
    let body = document.body().unwrap();

    let div = document.create_element("div").unwrap();
    div.set_id("app");
    let _ = body.append_child(&div);

    let _handle = mount_to(div.clone().unchecked_into(), App);

    let show_button = document
        .get_element_by_id("btn-show")
        .unwrap()
        .unchecked_into::<HtmlButtonElement>();

    show_button.click();

    tick().await;

    // check HTML
    assert_eq!(
        minify(div.inner_html().as_str()),
        minify(
            "<div><button id=\"btn-show\">Show \
             Overlay</button><div>Show</div><!----></div><div><div \
             style=\"position: fixed; z-index: 10; width: 100vw; height: \
             100vh; top: 0; left: 0; background: rgba(0, 0, 0, 0.8); color: \
             white;\"><p>This is in the body element</p><button \
             id=\"btn-hide\">Close Overlay</button><button \
             id=\"btn-toggle\">Toggle inner</button>Hidden</div></div>"
        )
    );

    let toggle_button = document
        .get_element_by_id("btn-toggle")
        .unwrap()
        .unchecked_into::<HtmlButtonElement>();

    toggle_button.click();

    assert_eq!(
        minify(div.inner_html().as_str()),
        minify(
            "<div><button id=\"btn-show\">Show \
             Overlay</button><div>Show</div><!----></div><div><div \
             style=\"position: fixed; z-index: 10; width: 100vw; height: \
             100vh; top: 0; left: 0; background: rgba(0, 0, 0, 0.8); color: \
             white;\"><p>This is in the body element</p><button \
             id=\"btn-hide\">Close Overlay</button><button \
             id=\"btn-toggle\">Toggle inner</button>Hidden</div></div>"
        )
    );

    let hide_button = document
        .get_element_by_id("btn-hide")
        .unwrap()
        .unchecked_into::<HtmlButtonElement>();

    hide_button.click();

    assert_eq!(
        minify(div.inner_html().as_str()),
        minify(
            "<div><button id=\"btn-show\">Show \
             Overlay</button><div>Show</div><!----></div><div><div \
             style=\"position: fixed; z-index: 10; width: 100vw; height: \
             100vh; top: 0; left: 0; background: rgba(0, 0, 0, 0.8); color: \
             white;\"><p>This is in the body element</p><button \
             id=\"btn-hide\">Close Overlay</button><button \
             id=\"btn-toggle\">Toggle inner</button>Hidden</div></div>"
        )
    );
}

#[test]
fn test_minify() {
    let input = r#"<div>
            <p>   Hello   world!   </p>

            <ul>
                <li>Item 1</li>
                <li>Item 2</li>
            </ul>
        </div>"#;

    let expected = r#"<div><p>Hello world!</p><ul><li>Item 1</li><li>Item 2</li></ul></div>"#;

    assert_eq!(minify(input), expected);
}

#[test]
fn test_preserve_whitespace_in_tags() {
    let input = r#"<div class = "container">"#;
    let expected = r#"<div class = "container">"#;

    assert_eq!(minify(input), expected);
}
