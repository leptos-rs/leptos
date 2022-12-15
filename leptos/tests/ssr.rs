#[cfg(not(any(feature = "csr", feature = "hydrate")))]
#[test]
fn simple_ssr_test() {
    use leptos::*;

    _ = create_scope(create_runtime(), |cx| {
        let (value, set_value) = create_signal(cx, 0);
        let rendered = view! {
            cx,
            <div>
                <button on:click=move |_| set_value.update(|value| *value -= 1)>"-1"</button>
                <span>"Value: " {move || value.get().to_string()} "!"</span>
                <button on:click=move |_| set_value.update(|value| *value += 1)>"+1"</button>
            </div>
        };

        assert_eq!(
            rendered.into_view(cx).render_to_string(cx),
            r#"<div data-hk="0-0"><button>-1</button><span>Value: <!--#-->0<!--/-->!</span><button>+1</button></div>"# //r#"<div data-hk="0" id="hydrated" data-hk="0"><button>-1</button><span>Value: <!--#-->0<!--/-->!</span><button>+1</button></div>"#
        );
    });
}

#[cfg(not(any(feature = "csr", feature = "hydrate")))]
#[test]
fn ssr_test_with_components() {
    use leptos::*;

    #[component]
    fn Counter(cx: Scope, initial_value: i32) -> impl IntoView {
        let (value, set_value) = create_signal(cx, initial_value);
        view! {
            cx,
            <div>
                <button on:click=move |_| set_value.update(|value| *value -= 1)>"-1"</button>
                <span>"Value: " {move || value.get().to_string()} "!"</span>
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
            rendered.into_view(cx).render_to_string(cx),
            "<div data-hk=\"0-0\" class=\"counters\"><!--#--><div data-hk=\"0-2-0\"><button>-1</button><span>Value: <!--#-->1<!--/-->!</span><button>+1</button></div><!--/--><!--#--><div data-hk=\"0-3-0\"><button>-1</button><span>Value: <!--#-->2<!--/-->!</span><button>+1</button></div><!--/--></div>"
        );
    });
}

#[cfg(not(any(feature = "csr", feature = "hydrate")))]
#[test]
fn test_classes() {
    use leptos::*;

    _ = create_scope(create_runtime(), |cx| {
        let (value, set_value) = create_signal(cx, 5);
        let rendered = view! {
            cx,
            <div class="my big" class:a={move || value.get() > 10} class:red=true class:car={move || value.get() > 1}></div>
        };

        assert_eq!(
            rendered.into_view(cx).render_to_string(cx),
            r#"<div data-hk="0-0" class="my big  red car"></div>"#
        );
    });
}
