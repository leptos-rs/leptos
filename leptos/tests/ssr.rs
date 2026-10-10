#[cfg(feature = "ssr")]
use leptos::html::HtmlElement;

#[cfg(feature = "ssr")]
#[test]
#[cfg_attr(
    erase_components,
    ignore = "erased views add hydration markers of their own"
)]
fn simple_ssr_test() {
    use leptos::prelude::*;

    let (value, set_value) = signal(0);
    let rendered: View<HtmlElement<_, _, _>> = view! {
        <div>
            <button on:click=move |_| set_value.update(|value| *value -= 1)>"-1"</button>
            <span>"Value: " {move || value.get().to_string()} "!"</span>
            <button on:click=move |_| set_value.update(|value| *value += 1)>"+1"</button>
        </div>
    };

    assert_eq!(
        rendered.to_html(),
        "<div><button>-1</button><span>Value: \
         <!>0<!>!</span><button>+1</button></div>"
    );
}

#[cfg(feature = "ssr")]
#[test]
#[cfg_attr(
    erase_components,
    ignore = "erased views add hydration markers of their own"
)]
fn ssr_test_with_components() {
    use leptos::prelude::*;

    #[component]
    fn Counter(initial_value: i32) -> impl IntoView {
        let (value, set_value) = signal(initial_value);
        view! {
            <div>
                <button on:click=move |_| set_value.update(|value| *value -= 1)>"-1"</button>
                <span>"Value: " {move || value.get().to_string()} "!"</span>
                <button on:click=move |_| set_value.update(|value| *value += 1)>"+1"</button>
            </div>
        }
    }

    let rendered: View<HtmlElement<_, _, _>> = view! {
        <div class="counters">
            <Counter initial_value=1/>
            <Counter initial_value=2/>
        </div>
    };

    assert_eq!(
        rendered.to_html(),
        "<div class=\"counters\"><div><button>-1</button><span>Value: \
         <!>1<!>!</span><button>+1</button></div><div><button>-1</\
         button><span>Value: <!>2<!>!</span><button>+1</button></div></div>"
    );
}

#[cfg(feature = "ssr")]
#[test]
#[cfg_attr(
    erase_components,
    ignore = "erased views add hydration markers of their own"
)]
fn ssr_test_with_snake_case_components() {
    use leptos::prelude::*;

    #[component]
    fn snake_case_counter(initial_value: i32) -> impl IntoView {
        let (value, set_value) = signal(initial_value);
        view! {
            <div>
                <button on:click=move |_| set_value.update(|value| *value -= 1)>"-1"</button>
                <span>"Value: " {move || value.get().to_string()} "!"</span>
                <button on:click=move |_| set_value.update(|value| *value += 1)>"+1"</button>
            </div>
        }
    }
    let rendered: View<HtmlElement<_, _, _>> = view! {
        <div class="counters">
            <SnakeCaseCounter initial_value=1/>
            <SnakeCaseCounter initial_value=2/>
        </div>
    };

    assert_eq!(
        rendered.to_html(),
        "<div class=\"counters\"><div><button>-1</button><span>Value: \
         <!>1<!>!</span><button>+1</button></div><div><button>-1</\
         button><span>Value: <!>2<!>!</span><button>+1</button></div></div>"
    );
}

#[cfg(feature = "ssr")]
#[test]
fn test_classes() {
    use leptos::prelude::*;

    let (value, _set_value) = signal(5);
    let rendered: View<HtmlElement<_, _, _>> = view! {
        <div
            class="my big"
            class:a=move || { value.get() > 10 }
            class:red=true
            class:car=move || { value.get() > 1 }
        ></div>
    };

    assert_eq!(rendered.to_html(), "<div class=\"my big  red car\"></div>");
}

#[cfg(feature = "ssr")]
#[test]
fn test_class_with_class_directive_merge() {
    use leptos::prelude::*;

    // class= followed by class: should merge
    let rendered: View<HtmlElement<_, _, _>> = view! {
        <div class="foo" class:bar=true></div>
    };

    assert_eq!(rendered.to_html(), "<div class=\"foo bar\"></div>");
}

#[cfg(feature = "ssr")]
#[test]
fn test_solo_class_directive() {
    use leptos::prelude::*;

    // Solo class: directive should work without class attribute
    let rendered: View<HtmlElement<_, _, _>> = view! {
        <div class:foo=true></div>
    };

    assert_eq!(rendered.to_html(), "<div class=\"foo\"></div>");
}

#[cfg(feature = "ssr")]
#[test]
fn test_class_directive_with_static_class() {
    use leptos::prelude::*;

    // class:foo comes after class= due to macro sorting
    // The class= clears buffer, then class:foo appends
    let rendered: View<HtmlElement<_, _, _>> = view! {
        <div class:foo=true class="bar"></div>
    };

    // After macro sorting: class="bar" class:foo=true
    // Expected: "bar foo"
    assert_eq!(rendered.to_html(), "<div class=\"bar foo\"></div>");
}

#[cfg(feature = "ssr")]
#[test]
fn test_global_class_applied() {
    use leptos::prelude::*;

    // Test that a global class is properly applied
    let rendered: View<HtmlElement<_, _, _>> = view! { class="global",
        <div></div>
    };

    assert_eq!(rendered.to_html(), "<div class=\"global\"></div>");
}

#[cfg(feature = "ssr")]
#[test]
fn test_multiple_class_attributes_overwrite() {
    use leptos::prelude::*;

    // When multiple class attributes are applied, the last one should win (browser behavior)
    // This simulates what happens when attributes are combined programmatically
    let el = leptos::html::div().class("first").class("second");

    let html = el.to_html();

    // The second class attribute should overwrite the first
    assert_eq!(html, "<div class=\"second\"></div>");
}

#[cfg(feature = "ssr")]
#[test]
#[cfg_attr(
    erase_components,
    ignore = "erased views add hydration markers of their own"
)]
fn ssr_with_styles() {
    use leptos::prelude::*;

    let (_, set_value) = signal(0);
    let styles = "myclass";
    let rendered: View<HtmlElement<_, _, _>> = view! { class=styles,
        <div>
            <button class="btn" on:click=move |_| set_value.update(|value| *value -= 1)>
                "-1"
            </button>
        </div>
    };

    assert_eq!(
        rendered.to_html(),
        "<div class=\"myclass\"><button class=\"btn \
         myclass\">-1</button></div>"
    );
}

#[cfg(feature = "ssr")]
#[test]
fn ssr_option() {
    use leptos::prelude::*;

    let (_, _) = signal(0);
    let rendered: View<HtmlElement<_, _, _>> = view! { <option></option> };

    assert_eq!(rendered.to_html(), "<option></option>");
}

#[cfg(feature = "ssr")]
fn render_hydration_scripts(defer: bool) -> String {
    use leptos::prelude::*;

    let options: LeptosOptions =
        serde_json::from_str(r#"{ "output-name": "app" }"#).unwrap();
    let rendered = view! { <HydrationScripts options=options defer=defer /> };

    rendered.to_html()
}

#[cfg(feature = "ssr")]
#[test]
fn hydration_scripts_default_output() {
    // The default `HydrationScripts` output is load-bearing for every existing
    // app, so lock it down: no `fetchpriority` hints, and the hydration script
    // invoked eagerly.
    let html = render_hydration_scripts(false);

    // no priority hints, and the idle branch is still present but unused
    assert!(!html.contains("fetchpriority"), "{html}");
    assert!(html.contains("requestIdleCallback"), "{html}");
    // the script is handed `false`, so it takes the eager path
    assert!(html.contains(r#""pkg", "app", "app", false);"#), "{html}");
}

#[cfg(feature = "ssr")]
#[test]
fn hydration_scripts_deferred_output() {
    // With `defer`, the resource hints drop to `low` priority so they don't
    // compete with the render-blocking resources, and the hydration script is
    // told to wait for an idle callback.
    let html = render_hydration_scripts(true);

    assert_eq!(html.matches(r#"fetchpriority="low""#).count(), 2, "{html}");
    assert!(html.contains(r#""pkg", "app", "app", true);"#), "{html}");
}

#[cfg(feature = "ssr")]
#[test]
fn hydration_scripts_defer_is_a_no_op_in_islands_mode() {
    // `islands` already hydrates on idle, and pushing it later would widen the
    // window in which `islands_routing.js` can call `__hydrateIsland` before it
    // exists, so `defer` must not reach that path.
    use leptos::prelude::*;

    let options: LeptosOptions =
        serde_json::from_str(r#"{ "output-name": "app" }"#).unwrap();
    let rendered =
        view! { <HydrationScripts options=options islands=true defer=true /> };
    let html = rendered.to_html();

    assert!(!html.contains("fetchpriority"), "{html}");
    assert!(html.contains(r#""pkg", "app", "app", false);"#), "{html}");
}

#[cfg(feature = "ssr")]
#[test]
#[cfg_attr(
    erase_components,
    ignore = "erased views add hydration markers of their own"
)]
fn ssr_textarea_escapes_static_content() {
    use leptos::prelude::*;

    // Nested (non-top-level) static textarea exercises the macro's inert
    // HTML path; its content must be HTML-escaped.
    let rendered: View<HtmlElement<_, _, _>> = view! {
        <div><textarea>"a < b & c"</textarea></div>
    };

    assert_eq!(
        rendered.to_html(),
        "<div><textarea>a &lt; b &amp; c</textarea></div>"
    );
}

// NOTE: This test was added in 12e1d7c.
//
// It never ran in CI due to issues described in https://github.com/leptos-rs/leptos/pull/4840
// It was added as a test for the code changes in that commmit. However, the changes are irrelevant
// here; this example does not actually exercise the InertElement path, because it contains
// a non-inert text node `{untrusted}`. (`is_inert_element` bails on anything that is not an element
// or static text node.)
//
// This scenario has already been fixed with a new approach on the `leptos_0.9` branch. Leaving this
// here for clarity, but the test will be restored for 0.9.
//
// #[cfg(feature = "ssr")]
// #[test]
// fn ssr_textarea_escapes_dynamic_content() {
//     use leptos::prelude::*;

//     // A dynamic child makes the textarea non-inert, exercising the runtime
//     // render path; its content must also be HTML-escaped.
//     let untrusted = "</textarea><script>alert('xss')</script>".to_string();
//     let rendered: View<HtmlElement<_, _, _>> = view! {
//         <textarea>{untrusted}</textarea>
//     };

//     assert_eq!(
//         rendered.to_html(),
//         "<textarea>&lt;/textarea&gt;&lt;script&gt;alert('xss')&lt;/script&gt;\
//          </textarea>"
//     );
// }
