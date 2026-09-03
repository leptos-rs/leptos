//! Browser tests for the inert (static) SVG fast path.
//!
//! Static SVG children of `<svg>` take the inert HTML path and are
//! reconstructed on the client by `Dom::create_svg_element_from_html`, which
//! parses the fragment inside an SVG-namespaced scaffold so the markup inherits
//! the SVG namespace. These tests pin down two properties of that
//! reconstruction:
//!
//! 1. The mounted node is the authored element itself, not a spurious wrapper
//!    (the original bug mounted every inert SVG child inside an extra `<g>`).
//! 2. The element and its descendants are in the SVG namespace, so they
//!    actually render as SVG rather than as inert HTML-namespaced elements.
//!
//! Run in a browser: `wasm-pack test --headless --chrome -p tachys`.
#![cfg(target_family = "wasm")]

use tachys::{
    prelude::{Mountable, Render},
    svg::InertElement,
};
use wasm_bindgen_test::*;

wasm_bindgen_test_configure!(run_in_browser);

const SVG_NS: &str = "http://www.w3.org/2000/svg";

/// The reproducible example from the bug report: a nested SVG child must mount
/// as `<text><tspan>…</tspan></text>`, not `<g><text>…</text></g>`.
#[wasm_bindgen_test]
fn inert_svg_child_is_not_wrapped_in_a_scaffold() {
    let html = r#"<text x="18" y="32"><tspan>Just an example</tspan></text>"#;
    let elements = InertElement::new(html).build().elements();

    assert_eq!(
        elements.len(),
        1,
        "InertElement should mount exactly one element"
    );
    let el = &elements[0];

    // The mounted element is the authored <text>, not a <g>/<svg> scaffold.
    // (For SVG-namespaced elements `tagName` preserves the authored lowercase;
    // an HTML-namespaced parse would yield "TEXT" instead.)
    assert_eq!(
        el.tag_name(),
        "text",
        "expected the authored <text> element, found <{}>",
        el.tag_name()
    );
    assert_eq!(
        el.namespace_uri().as_deref(),
        Some(SVG_NS),
        "mounted element must be in the SVG namespace"
    );

    // The child is preserved and is itself SVG-namespaced.
    let child = el
        .first_element_child()
        .expect("<text> should have a <tspan> child");
    assert_eq!(child.tag_name(), "tspan");
    assert_eq!(child.namespace_uri().as_deref(), Some(SVG_NS));
    assert_eq!(child.text_content().as_deref(), Some("Just an example"));
}

/// A self-closing single element exercises the same path with no inner
/// children, ensuring the scaffold is still stripped and the namespace applied.
#[wasm_bindgen_test]
fn inert_self_closing_svg_element_keeps_namespace_without_wrapper() {
    let html = r#"<circle cx="5" cy="5" r="4"></circle>"#;
    let elements = InertElement::new(html).build().elements();

    assert_eq!(elements.len(), 1);
    let el = &elements[0];

    assert_eq!(el.tag_name(), "circle");
    assert_eq!(el.namespace_uri().as_deref(), Some(SVG_NS));
    assert!(
        el.first_element_child().is_none(),
        "a <circle> should mount with no element children"
    );
}
