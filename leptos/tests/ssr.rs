#[cfg(not(any(feature = "csr", feature = "hydrate")))]
#[test]
fn simple_ssr_test() {
    use leptos_dom::*;
    use leptos_macro::view;
    use leptos_reactive::{create_runtime, create_scope, create_signal};

    _ = create_scope(create_runtime(), |cx| {
        let (value, set_value) = create_signal(cx, 0);
        let rendered = view! {
            cx,
            <div>
                <button on:click=move |_| set_value.update(|value| *value -= 1)>"-1"</button>
                <span>"Value: " {move || value().to_string()} "!"</span>
                <button on:click=move |_| set_value.update(|value| *value += 1)>"+1"</button>
            </div>
        };

        assert_eq!(
            rendered,
            r#"<div data-hk="0-0"><button>-1</button><span>Value: <!--#-->0<!--/-->!</span><button>+1</button></div>"# //r#"<div data-hk="0" id="hydrated" data-hk="0"><button>-1</button><span>Value: <!--#-->0<!--/-->!</span><button>+1</button></div>"#
        );
    });
}

#[cfg(not(any(feature = "csr", feature = "hydrate")))]
#[test]
fn ssr_test_with_components() {
    use leptos_core as leptos;
    use leptos_core::Prop;
    use leptos_dom::*;
    use leptos_macro::*;
    use leptos_reactive::{create_scope, create_signal, Scope};

    #[component]
    fn Counter(cx: Scope, initial_value: i32) -> Element {
        let (value, set_value) = create_signal(cx, initial_value);
        view! {
            cx,
            <div>
                <button on:click=move |_| set_value.update(|value| *value -= 1)>"-1"</button>
                <span>"Value: " {move || value().to_string()} "!"</span>
                <button on:click=move |_| set_value.update(|value| *value += 1)>"+1"</button>
            </div>
        }
    }

    _ = create_scope(create_runtime(), |cx| {
        let rendered = view! {
            cx,
            <div class="counters">
                <Counter initial_value=1/>
                <Counter initial_value=2/>
            </div>
        };

        assert_eq!(
            rendered,
            "<div data-hk=\"0-0\" class=\"counters\"><!--#--><div data-hk=\"0-2-0\"><button>-1</button><span>Value: <!--#-->1<!--/-->!</span><button>+1</button></div><!--/--><!--#--><div data-hk=\"0-3-0\"><button>-1</button><span>Value: <!--#-->2<!--/-->!</span><button>+1</button></div><!--/--></div>"
        );
    });
}

#[cfg(not(any(feature = "csr", feature = "hydrate")))]
#[test]
fn test_classes() {
    use leptos_dom::*;
    use leptos_macro::view;
    use leptos_reactive::{create_runtime, create_scope, create_signal};

    _ = create_scope(create_runtime(), |cx| {
        let (value, set_value) = create_signal(cx, 5);
        let rendered = view! {
            cx,
            <div class="my big" class:a={move || value() > 10} class:red=true class:car={move || value() > 1}></div>
        };

        assert_eq!(
            rendered,
            r#"<div data-hk="0-0" class="my big  red car"></div>"#
        );
    });
}
