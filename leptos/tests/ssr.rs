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
            "<div id=\"_0-1\"><button id=\"_0-2\">-1</button><span id=\"_0-3\">Value: <!--hk=_0-4o|leptos-dyn-child-start-->0<!--hk=_0-4c|leptos-dyn-child-end-->!</span><button id=\"_0-5\">+1</button></div>"
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
            "<div class=\"counters\" id=\"_0-1\"><!--hk=_0-1-0o|leptos-counter-start--><div id=\"_0-1-1\"><button id=\"_0-1-2\">-1</button><span id=\"_0-1-3\">Value: <!--hk=_0-1-4o|leptos-dyn-child-start-->1<!--hk=_0-1-4c|leptos-dyn-child-end-->!</span><button id=\"_0-1-5\">+1</button></div><!--hk=_0-1-0c|leptos-counter-end--><!--hk=_0-1-5-0o|leptos-counter-start--><div id=\"_0-1-5-1\"><button id=\"_0-1-5-2\">-1</button><span id=\"_0-1-5-3\">Value: <!--hk=_0-1-5-4o|leptos-dyn-child-start-->2<!--hk=_0-1-5-4c|leptos-dyn-child-end-->!</span><button id=\"_0-1-5-5\">+1</button></div><!--hk=_0-1-5-0c|leptos-counter-end--></div>"
        );
    });
}

#[cfg(not(any(feature = "csr", feature = "hydrate")))]
#[test]
fn test_classes() {
    use leptos::*;

    _ = create_scope(create_runtime(), |cx| {
        let (value, _set_value) = create_signal(cx, 5);
        let rendered = view! {
            cx,
            <div class="my big" class:a={move || value.get() > 10} class:red=true class:car={move || value.get() > 1}></div>
        };

        assert_eq!(
            rendered.into_view(cx).render_to_string(cx),
            "<div class=\"my big red car\" id=\"_0-1\"></div>"
        );
    });
}
