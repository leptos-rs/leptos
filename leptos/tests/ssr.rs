#[cfg(not(any(feature = "csr", feature = "hydrate")))]
#[test]
fn simple_ssr_test() {
    use leptos::*;

    let runtime = create_runtime();
    let (value, set_value) = create_signal(0);
    let rendered = view! {
        <div>
            <button on:click=move |_| set_value.update(|value| *value -= 1)>"-1"</button>
            <span>"Value: " {move || value.get().to_string()} "!"</span>
            <button on:click=move |_| set_value.update(|value| *value += 1)>"+1"</button>
        </div>
    };

    if cfg!(all(feature = "experimental-islands", feature = "ssr")) {
        assert_eq!(
            rendered.into_view().render_to_string(),
            "<div><button>-1</button><span>Value: \
             0!</span><button>+1</button></div>"
        );
    } else {
        assert!(rendered.into_view().render_to_string().contains(
            "<div data-hk=\"0-0-0-1\"><button \
             data-hk=\"0-0-0-2\">-1</button><span data-hk=\"0-0-0-3\">Value: \
             <!--hk=0-0-0-4o|leptos-dyn-child-start-->0<!\
             --hk=0-0-0-4c|leptos-dyn-child-end-->!</span><button \
             data-hk=\"0-0-0-5\">+1</button></div>"
        ));
    }

    runtime.dispose();
}

#[cfg(not(any(feature = "csr", feature = "hydrate")))]
#[test]
fn ssr_test_with_components() {
    use leptos::*;

    #[component]
    fn Counter(initial_value: i32) -> impl IntoView {
        let (value, set_value) = create_signal(initial_value);
        view! {

            <div>
                <button on:click=move |_| set_value.update(|value| *value -= 1)>"-1"</button>
                <span>"Value: " {move || value.get().to_string()} "!"</span>
                <button on:click=move |_| set_value.update(|value| *value += 1)>"+1"</button>
            </div>
        }
    }

    let runtime = create_runtime();
    let rendered = view! {

        <div class="counters">
            <Counter initial_value=1/>
            <Counter initial_value=2/>
        </div>
    };

    if cfg!(all(feature = "experimental-islands", feature = "ssr")) {
        assert_eq!(
            rendered.into_view().render_to_string(),
            "<div class=\"counters\"><div><button>-1</button><span>Value: \
             1!</span><button>+1</button></div><div><button>-1</\
             button><span>Value: 2!</span><button>+1</button></div></div>"
        );
    } else {
        assert!(rendered.into_view().render_to_string().contains(
            "<div data-hk=\"0-0-0-3\"><button \
             data-hk=\"0-0-0-4\">-1</button><span data-hk=\"0-0-0-5\">Value: \
             <!--hk=0-0-0-6o|leptos-dyn-child-start-->1<!\
             --hk=0-0-0-6c|leptos-dyn-child-end-->!</span><button \
             data-hk=\"0-0-0-7\">+1</button></div>"
        ));
    }
    runtime.dispose();
}

#[cfg(not(any(feature = "csr", feature = "hydrate")))]
#[test]
fn ssr_test_with_snake_case_components() {
    use leptos::*;

    #[component]
    fn snake_case_counter(initial_value: i32) -> impl IntoView {
        let (value, set_value) = create_signal(initial_value);
        view! {

            <div>
                <button on:click=move |_| set_value.update(|value| *value -= 1)>"-1"</button>
                <span>"Value: " {move || value.get().to_string()} "!"</span>
                <button on:click=move |_| set_value.update(|value| *value += 1)>"+1"</button>
            </div>
        }
    }

    let runtime = create_runtime();
    let rendered = view! {

        <div class="counters">
            <SnakeCaseCounter initial_value=1/>
            <SnakeCaseCounter initial_value=2/>
        </div>
    };

    if cfg!(all(feature = "experimental-islands", feature = "ssr")) {
        assert_eq!(
            rendered.into_view().render_to_string(),
            "<div class=\"counters\"><div><button>-1</button><span>Value: \
             1!</span><button>+1</button></div><div><button>-1</\
             button><span>Value: 2!</span><button>+1</button></div></div>"
        );
    } else {
        assert!(rendered.into_view().render_to_string().contains(
            "<div data-hk=\"0-0-0-3\"><button \
             data-hk=\"0-0-0-4\">-1</button><span data-hk=\"0-0-0-5\">Value: \
             <!--hk=0-0-0-6o|leptos-dyn-child-start-->1<!\
             --hk=0-0-0-6c|leptos-dyn-child-end-->!</span><button \
             data-hk=\"0-0-0-7\">+1</button></div>"
        ));
    }

    runtime.dispose();
}

#[cfg(not(any(feature = "csr", feature = "hydrate")))]
#[test]
fn test_classes() {
    use leptos::*;

    let runtime = create_runtime();
    let (value, _set_value) = create_signal(5);
    let rendered = view! {

        <div class="my big" class:a={move || value.get() > 10} class:red=true class:car={move || value.get() > 1}></div>
    };

    if cfg!(all(feature = "experimental-islands", feature = "ssr")) {
        assert_eq!(
            rendered.into_view().render_to_string(),
            "<div class=\"my big  red car\"></div>"
        );
    } else {
        assert!(rendered.into_view().render_to_string().contains(
            "<div data-hk=\"0-0-0-1\" class=\"my big  red car\"></div>"
        ));
    }
    runtime.dispose();
}

#[cfg(not(any(feature = "csr", feature = "hydrate")))]
#[test]
fn ssr_with_styles() {
    use leptos::*;

    let runtime = create_runtime();
    let (_, set_value) = create_signal(0);
    let styles = "myclass";
    let rendered = view! {
         class = styles,
        <div>
            <button class="btn" on:click=move |_| set_value.update(|value| *value -= 1)>"-1"</button>
        </div>
    };

    if cfg!(all(feature = "experimental-islands", feature = "ssr")) {
        assert_eq!(
            rendered.into_view().render_to_string(),
            "<div class=\" myclass\"><button class=\"btn \
             myclass\">-1</button></div>"
        );
    } else {
        assert!(rendered.into_view().render_to_string().contains(
            "<div data-hk=\"0-0-0-1\" class=\" myclass\"><button \
             data-hk=\"0-0-0-2\" class=\"btn myclass\">-1</button></div>"
        ));
    }
    runtime.dispose();
}

#[cfg(not(any(feature = "csr", feature = "hydrate")))]
#[test]
fn ssr_option() {
    use leptos::*;

    let runtime = create_runtime();
    let (_, _) = create_signal(0);
    let rendered = view! {

        <option/>
    };

    if cfg!(all(feature = "experimental-islands", feature = "ssr")) {
        assert_eq!(
            rendered.into_view().render_to_string(),
            "<option></option>"
        );
    } else {
        assert!(rendered
            .into_view()
            .render_to_string()
            .contains("<option data-hk=\"0-0-0-1\"></option>"));
    }

    runtime.dispose();
}
