use test::Bencher;

mod leptos;
mod sycamore;
mod tera;
mod yew;

#[bench]
fn leptos_todomvc_ssr(b: &mut Bencher) {
    use self::leptos::*;
    use ::leptos::*;

    b.iter(|| {
        _ = create_scope(create_runtime(), |cx| {
            let rendered = view! {
                cx,
                <TodoMVC todos=Todos::new(cx)/>
            };

            assert!(rendered.len() > 1);
        });
    });
}

#[bench]
fn sycamore_todomvc_ssr(b: &mut Bencher) {
    use self::sycamore::*;
    use ::sycamore::prelude::*;
    use ::sycamore::*;

    b.iter(|| {
        _ = create_scope(|cx| {
            let rendered = render_to_string(|cx| {
                view! {
                    cx,
                    App()
                }
            });

            assert!(rendered.len() > 1);
        });
    });
}

#[bench]
fn yew_todomvc_ssr(b: &mut Bencher) {
    use self::yew::*;
    use ::yew::prelude::*;
    use ::yew::ServerRenderer;

    b.iter(|| {
        tokio_test::block_on(async {
            let renderer = ServerRenderer::<App>::new();
            let rendered = renderer.render().await;
            assert!(rendered.len() > 1);
        });
    });
}

#[bench]
fn leptos_todomvc_ssr_with_1000(b: &mut Bencher) {
    use self::leptos::*;
    use ::leptos::*;

    b.iter(|| {
        _ = create_scope(create_runtime(), |cx| {
            let rendered = view! {
                cx,
                <TodoMVC todos=Todos::new_with_1000(cx)/>
            };

            assert!(rendered.len() > 1);
        });
    });
}

#[bench]
fn sycamore_todomvc_ssr_with_1000(b: &mut Bencher) {
    use self::sycamore::*;
    use ::sycamore::prelude::*;
    use ::sycamore::*;

    b.iter(|| {
        _ = create_scope(|cx| {
            let rendered = render_to_string(|cx| {
                view! {
                    cx,
                    AppWith1000()
                }
            });

            assert!(rendered.len() > 1);
        });
    });
}

#[bench]
fn yew_todomvc_ssr_with_1000(b: &mut Bencher) {
    use self::yew::*;
    use ::yew::prelude::*;
    use ::yew::ServerRenderer;

    b.iter(|| {
        tokio_test::block_on(async {
            let renderer = ServerRenderer::<AppWith1000>::new();
            let rendered = renderer.render().await;
            assert!(rendered.len() > 1);
        });
    });
}
