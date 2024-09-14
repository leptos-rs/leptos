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
