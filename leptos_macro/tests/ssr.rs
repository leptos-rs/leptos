#[cfg(feature = "ssr")]
#[test]
fn simple_ssr_test() {
    use leptos_dom::*;
    use leptos_macro::view;
    use leptos_reactive::{create_scope, create_signal};

    _ = create_scope(|cx| {
        let (value, set_value) = create_signal(cx, 0);
        let rendered = view! { cx,
            cx,
            <div>
                <button on:click=move |_| set_value(|value| *value -= 1)>"-1"</button>
                <span>"Value: " {move || value().to_string()} "!"</span>
                <button on:click=move |_| set_value(|value| *value += 1)>"+1"</button>
            </div>
        };

        assert_eq!(
            rendered,
            r#"<div data-hk="0"><button>-1</button><span>Value: <!--#-->0<!--/-->!</span><button>+1</button></div>"# //r#"<div data-hk="0" id="hydrated" data-hk="0"><button>-1</button><span>Value: <!--#-->0<!--/-->!</span><button>+1</button></div>"#
        );
    });
}

#[cfg(feature = "ssr")]
#[test]
fn ssr_test_with_components() {
    use leptos_core as leptos;
    use leptos_core::Prop;
    use leptos_dom::*;
    use leptos_macro::*;
    use leptos_reactive::{create_scope, create_signal, Scope};

    #[component]
    fn Counter(cx: Scope, initial_value: i32) -> Element {
        let (value, set_value) = create_signal(cx, 0);
        view! { cx,
            cx,
            <div>
                <button on:click=move |_| set_value(|value| *value -= 1)>"-1"</button>
                <span>"Value: " {move || value().to_string()} "!"</span>
                <button on:click=move |_| set_value(|value| *value += 1)>"+1"</button>
            </div>
        }
    }

    _ = create_scope(|cx| {
        let rendered = view! { cx,
            cx,
            <div class="counters">
                <Counter initial_value=1/>
                <Counter initial_value=2/>
            </div>
        };

        assert_eq!(
            rendered,
            r#"<div data-hk="0" class="counters"><!--#--><div data-hk="1"><button>-1</button><span>Value: <!--#-->1<!--/-->!</span><button>+1</button></div><!--/--><!--#--><div data-hk="2"><button>-1</button><span>Value: <!--#-->2<!--/-->!</span><button>+1</button></div><!--/--></div>"#
        );
    });
}
