#[cfg(not(any(feature = "csr", feature = "hydrate")))]
#[test]
fn ssr_for_ifempty() {
    use leptos::*;

    let runtime = create_runtime();
    let (value, _) = create_signal(Vec::<u32>::new());
    let ifm = || view! {<p>"array is empty!"</p>};
    let rendered = view! {
        <div>
            <For each=move|| value.get() key=|v| *v let:value ifempty=ifm>
                <p>{value}</p>
            </For>
        </div>
    };

    if cfg!(all(feature = "experimental-islands", feature = "ssr")) {
        assert_eq!(
            rendered.into_view().render_to_string(),
            "<div><p>array is empty!</p></div>"
        );
    } else {
        assert!(rendered.into_view().render_to_string().contains(
            "<div data-hk=\"0-0-0-1\"><p data-hk=\"0-0-0-2\">array is \
             empty!</p></div>"
        ));
    }

    runtime.dispose();
}

#[cfg(not(any(feature = "csr", feature = "hydrate")))]
#[test]
fn ssr_for_if_not_empty() {
    use leptos::*;

    let runtime = create_runtime();
    let (value, _) = create_signal(vec![31_u32, 63]);
    let ifm = || view! {<p>"array is empty!"</p>};
    let rendered = view! {
        <div>
            <For each=move|| value.get() key=|v| *v let:value ifempty=ifm>
                <p>{value}</p>
            </For>
        </div>
    };

    if cfg!(all(feature = "experimental-islands", feature = "ssr")) {
        assert_eq!(
            rendered.into_view().render_to_string(),
            "<div><p>31</p><p>63</p></div>"
        );
    } else {
        assert!(rendered.into_view().render_to_string().contains(
            "<div data-hk=\"0-0-0-1\"><!--hk=0-0-0-2o|leptos-each-start--><!\
             --hk=0-0-0-5o|leptos-each-item-start--><!--leptos-view|<For/\
             >-children|open--><!--hk=0-0-0-3o|leptos-<>-start--><p \
             data-hk=\"0-0-0-4\">31</p><!--hk=0-0-0-3c|leptos-<>-end--><!\
             --leptos-view|<For/>-children|close--><!\
             --hk=0-0-0-5c|leptos-each-item-end--><!\
             --hk=0-0-0-8o|leptos-each-item-start--><!--leptos-view|<For/\
             >-children|open--><!--hk=0-0-0-6o|leptos-<>-start--><p \
             data-hk=\"0-0-0-7\">63</p><!--hk=0-0-0-6c|leptos-<>-end--><!\
             --leptos-view|<For/>-children|close--><!\
             --hk=0-0-0-8c|leptos-each-item-end--><!\
             --hk=0-0-0-2c|leptos-each-end--></div>"
        ));
    }

    runtime.dispose();
}
