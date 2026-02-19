//! Regression tests for SSR feature unification safety.
//!
//! These tests verify that event handlers, directives, and properties work
//! correctly regardless of which features are active. A previous bug
//! (pre-v0.2.13) used `cfg!(feature = "ssr")` to conditionally create
//! client-side values. When the `ssr` feature was activated via Cargo feature
//! unification (e.g., by an unrelated dependency like `radix-leptos`), these
//! values became `None`, causing runtime panics:
//!
//! - Events: "callback removed before attaching"
//! - Directives: "directive removed early"
//! - Properties: "property removed early"
//!
//! The fix ensures values are always created with `Some(...)`, making the
//! constructors resilient to feature conflicts. These tests catch regressions.
//!
//! # Running
//!
//! These tests require a browser DOM, so they must run under wasm-pack:
//!
//! ```sh
//! # Default features (includes testing):
//! wasm-pack test --headless --chrome -- -p tachys
//!
//! # Explicitly with SSR to verify the fix:
//! wasm-pack test --headless --chrome -- -p tachys --features ssr
//! ```

#![cfg(target_family = "wasm")]

use wasm_bindgen_test::*;

wasm_bindgen_test_configure!(run_in_browser);

// ---------------------------------------------------------------------------
// Event handler tests
// ---------------------------------------------------------------------------

mod event_safety {
    use super::*;
    use tachys::html::attribute::Attribute;
    use tachys::html::event::{click, on};

    /// Core regression test: `on()` must create a usable event handler.
    ///
    /// Before the fix, when `cfg!(feature = "ssr")` was true, `on()` created
    /// `On { cb: None, .. }`. Calling `build()` → `attach()` then panicked
    /// with "callback removed before attaching".
    #[wasm_bindgen_test]
    fn event_handler_build_does_not_panic() {
        let document = web_sys::window().unwrap().document().unwrap();
        let el = document.create_element("button").unwrap();

        let handler = on(click, |_: web_sys::MouseEvent| {});
        // This panicked before the fix:
        let _state = handler.build(&el);
    }

    /// Verify `hydrate()` path also works — it calls `attach()` internally.
    #[wasm_bindgen_test]
    fn event_handler_hydrate_does_not_panic() {
        let document = web_sys::window().unwrap().document().unwrap();
        let el = document.create_element("button").unwrap();

        let handler = on(click, |_: web_sys::MouseEvent| {});
        let _state = handler.hydrate::<true>(&el);
    }

    /// Verify `rebuild()` path — it attaches a new handler after detaching old.
    #[wasm_bindgen_test]
    fn event_handler_rebuild_does_not_panic() {
        let document = web_sys::window().unwrap().document().unwrap();
        let el = document.create_element("button").unwrap();

        let handler1 = on(click, |_: web_sys::MouseEvent| {});
        let mut state = handler1.build(&el);

        let handler2 = on(click, |_: web_sys::MouseEvent| {});
        // rebuild() re-attaches — must not panic:
        handler2.rebuild(&mut state);
    }

    /// Verify `to_html()` still does nothing — SSR path must not access cb.
    #[wasm_bindgen_test]
    fn event_handler_to_html_is_noop() {
        let handler = on(click, |_: web_sys::MouseEvent| {});
        let mut buf = String::new();
        let mut class = String::new();
        let mut style = String::new();
        let mut inner_html = String::new();
        handler.to_html(&mut buf, &mut class, &mut style, &mut inner_html);
        assert!(buf.is_empty(), "to_html must produce no output for events");
    }

    /// Verify `into_cloneable()` preserves the callback (doesn't lose it).
    #[wasm_bindgen_test]
    fn event_handler_cloneable_preserves_callback() {
        let document = web_sys::window().unwrap().document().unwrap();
        let el = document.create_element("button").unwrap();

        let handler = on(click, |_: web_sys::MouseEvent| {});
        let cloneable = handler.into_cloneable();
        // Must be able to build from the cloneable version:
        let _state = cloneable.build(&el);
    }
}

// ---------------------------------------------------------------------------
// Directive tests
// ---------------------------------------------------------------------------

mod directive_safety {
    use super::*;
    use tachys::html::attribute::Attribute;
    use tachys::html::directive::directive;

    /// Core regression test: `directive()` must create a usable handler.
    ///
    /// Before the fix, `directive()` created `Directive(None)`. Calling
    /// `build()` then panicked with "directive removed early".
    #[wasm_bindgen_test]
    fn directive_build_does_not_panic() {
        let document = web_sys::window().unwrap().document().unwrap();
        let el = document.create_element("div").unwrap();

        let d = directive(|_el: web_sys::Element, _: ()| {}, ());
        let _state = d.build(&el);
    }

    /// Verify `hydrate()` path works for directives.
    #[wasm_bindgen_test]
    fn directive_hydrate_does_not_panic() {
        let document = web_sys::window().unwrap().document().unwrap();
        let el = document.create_element("div").unwrap();

        let d = directive(|_el: web_sys::Element, _: ()| {}, ());
        let _state = d.hydrate::<true>(&el);
    }

    /// Verify `rebuild()` path works for directives.
    #[wasm_bindgen_test]
    fn directive_rebuild_does_not_panic() {
        let document = web_sys::window().unwrap().document().unwrap();
        let el = document.create_element("div").unwrap();

        let d1 = directive(|_el: web_sys::Element, _: ()| {}, ());
        let mut state = d1.build(&el);

        let d2 = directive(|_el: web_sys::Element, _: ()| {}, ());
        d2.rebuild(&mut state);
    }

    /// Verify directive handler actually runs during build.
    #[wasm_bindgen_test]
    fn directive_handler_is_executed() {
        use std::cell::Cell;
        use std::rc::Rc;

        let document = web_sys::window().unwrap().document().unwrap();
        let el = document.create_element("div").unwrap();

        let ran = Rc::new(Cell::new(false));
        let ran_clone = ran.clone();

        let d = directive(
            move |_el: web_sys::Element, _: ()| {
                ran_clone.set(true);
            },
            (),
        );
        d.build(&el);
        assert!(ran.get(), "directive handler must execute during build()");
    }

    /// Verify `to_html()` still does nothing for directives.
    #[wasm_bindgen_test]
    fn directive_to_html_is_noop() {
        let d = directive(|_el: web_sys::Element, _: ()| {}, ());
        let mut buf = String::new();
        let mut class = String::new();
        let mut style = String::new();
        let mut inner_html = String::new();
        d.to_html(&mut buf, &mut class, &mut style, &mut inner_html);
        assert!(
            buf.is_empty(),
            "to_html must produce no output for directives"
        );
    }
}

// ---------------------------------------------------------------------------
// Property tests
// ---------------------------------------------------------------------------

mod property_safety {
    use super::*;
    use tachys::html::attribute::Attribute;
    use tachys::html::property::prop;

    /// Core regression test: `prop()` must create a usable property.
    ///
    /// Before the fix, `prop()` created `Property { value: None, .. }`.
    /// Calling `build()` then panicked with "property removed early".
    #[wasm_bindgen_test]
    fn property_build_does_not_panic() {
        let document = web_sys::window().unwrap().document().unwrap();
        let el = document.create_element("input").unwrap();

        let p = prop("checked", true);
        let _state = p.build(&el);
    }

    /// Verify `hydrate()` path works for properties.
    #[wasm_bindgen_test]
    fn property_hydrate_does_not_panic() {
        let document = web_sys::window().unwrap().document().unwrap();
        let el = document.create_element("input").unwrap();

        let p = prop("value", "hello");
        let _state = p.hydrate::<true>(&el);
    }

    /// Verify `rebuild()` path works for properties.
    #[wasm_bindgen_test]
    fn property_rebuild_does_not_panic() {
        let document = web_sys::window().unwrap().document().unwrap();
        let el = document.create_element("input").unwrap();

        let p1 = prop("checked", true);
        let mut state = p1.build(&el);

        let p2 = prop("checked", false);
        p2.rebuild(&mut state);
    }

    /// Verify string property values work.
    #[wasm_bindgen_test]
    fn property_string_value_does_not_panic() {
        let document = web_sys::window().unwrap().document().unwrap();
        let el = document.create_element("input").unwrap();

        let p = prop("placeholder", "Enter text...");
        let _state = p.build(&el);
    }

    /// Verify `to_html()` still does nothing for properties.
    #[wasm_bindgen_test]
    fn property_to_html_is_noop() {
        let p = prop("checked", true);
        let mut buf = String::new();
        let mut class = String::new();
        let mut style = String::new();
        let mut inner_html = String::new();
        p.to_html(&mut buf, &mut class, &mut style, &mut inner_html);
        assert!(
            buf.is_empty(),
            "to_html must produce no output for properties"
        );
    }
}
