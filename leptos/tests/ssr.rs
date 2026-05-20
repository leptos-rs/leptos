#[cfg(feature = "ssr")]
use leptos::html::HtmlElement;

#[cfg(feature = "ssr")]
#[test]
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
