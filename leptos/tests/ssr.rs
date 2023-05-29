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

        assert!(rendered.into_view(cx).render_to_string(cx).contains(
            "<div id=\"_0-1\"><button id=\"_0-2\">-1</button><span \
             id=\"_0-3\">Value: \
             <!--hk=_0-4o|leptos-dyn-child-start-->0<!\
             --hk=_0-4c|leptos-dyn-child-end-->!</span><button \
             id=\"_0-5\">+1</button></div>"
        ));
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

        assert!(rendered.into_view(cx).render_to_string(cx).contains(
            "<div id=\"_0-3\"><button id=\"_0-4\">-1</button><span \
             id=\"_0-5\">Value: \
             <!--hk=_0-6o|leptos-dyn-child-start-->1<!\
             --hk=_0-6c|leptos-dyn-child-end-->!</span><button \
             id=\"_0-7\">+1</button></div>"
        ));
    });
}

#[cfg(not(any(feature = "csr", feature = "hydrate")))]
#[test]
fn ssr_test_with_snake_case_components() {
    use leptos::*;

    #[component]
    fn snake_case_counter(cx: Scope, initial_value: i32) -> impl IntoView {
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
                <SnakeCaseCounter initial_value=1/>
                <SnakeCaseCounter initial_value=2/>
            </div>
        };

        assert!(rendered.into_view(cx).render_to_string(cx).contains(
            "<div id=\"_0-3\"><button id=\"_0-4\">-1</button><span \
             id=\"_0-5\">Value: \
             <!--hk=_0-6o|leptos-dyn-child-start-->1<!\
             --hk=_0-6c|leptos-dyn-child-end-->!</span><button \
             id=\"_0-7\">+1</button></div>"
        ));
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

        assert!(rendered
            .into_view(cx)
            .render_to_string(cx)
            .contains("<div id=\"_0-1\" class=\"my big  red car\"></div>"));
    });
}

#[cfg(not(any(feature = "csr", feature = "hydrate")))]
#[test]
fn ssr_with_styles() {
    use leptos::*;

    _ = create_scope(create_runtime(), |cx| {
        let (_, set_value) = create_signal(cx, 0);
        let styles = "myclass";
        let rendered = view! {
            cx, class = styles,
            <div>
                <button class="btn" on:click=move |_| set_value.update(|value| *value -= 1)>"-1"</button>
            </div>
        };

        assert!(rendered.into_view(cx).render_to_string(cx).contains(
            "<div id=\"_0-1\" class=\" myclass\"><button id=\"_0-2\" \
             class=\"btn myclass\">-1</button></div>"
        ));
    });
}

#[cfg(not(any(feature = "csr", feature = "hydrate")))]
#[test]
fn ssr_option() {
    use leptos::*;

    _ = create_scope(create_runtime(), |cx| {
        let (_, _) = create_signal(cx, 0);
        let rendered = view! {
            cx,
            <option/>
        };

        assert!(rendered
            .into_view(cx)
            .render_to_string(cx)
            .contains("<option id=\"_0-1\"></option>"));
    });
}
