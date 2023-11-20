use test::Bencher;

mod leptos;
mod sycamore;
mod tachys;
mod tera;
mod yew;

#[bench]
fn leptos_todomvc_ssr(b: &mut Bencher) {
    use ::leptos::*;
    let runtime = create_runtime();
    b.iter(|| {
        use crate::todomvc::leptos::*;

        let html = ::leptos::ssr::render_to_string(|| {
            view! { <TodoMVC todos=Todos::new()/> }
        });
        assert!(html.len() > 1);
    });
    runtime.dispose();
}

#[bench]
fn tachys_todomvc_ssr(b: &mut Bencher) {
    use ::leptos::*;
    let runtime = create_runtime();
    b.iter(|| {
        use crate::todomvc::tachys::*;
        use tachydom::view::{Render, RenderHtml};

        let rendered = TodoMVC(Todos::new()).to_html();
        assert_eq!(
            rendered,
"<main><section class=\"todoapp\"><header class=\"header\"><h1>todos</h1><input placeholder=\"What needs to be done?\" autofocus class=\"new-todo\"></header><section class=\"main hidden\"><input id=\"toggle-all\" type=\"checkbox\" class=\"toggle-all\"><label for=\"toggle-all\">Mark all as complete</label><ul class=\"todo-list\"></ul></section><footer class=\"footer hidden\"><span class=\"todo-count\"><strong>0</strong><!> items<!> left</span><ul class=\"filters\"><li><a href=\"#/\" class=\"selected selected\">All</a></li><li><a href=\"#/active\" class=\"\">Active</a></li><li><a href=\"#/completed\" class=\"\">Completed</a></li></ul><button class=\"clear-completed hidden hidden\">Clear completed</button></footer></section><footer class=\"info\"><p>Double-click to edit a todo</p><p>Created by <a href=\"http://todomvc.com\">Greg Johnston</a></p><p>Part of <a href=\"http://todomvc.com\">TodoMVC</a></p></footer></main>"        );
    });
    runtime.dispose();
}

#[bench]
fn sycamore_todomvc_ssr(b: &mut Bencher) {
    use self::sycamore::*;
    use ::sycamore::{prelude::*, *};

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
    use ::yew::{prelude::*, ServerRenderer};

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
    b.iter(|| {
        use self::leptos::*;
        use ::leptos::*;

        let html = ::leptos::ssr::render_to_string(|| {
            view! {
                <TodoMVC todos=Todos::new_with_1000()/>
            }
        });
        assert!(html.len() > 1);
    });
}

#[bench]
fn tachys_todomvc_ssr_with_1000(b: &mut Bencher) {
    use ::leptos::*;
    let runtime = create_runtime();
    b.iter(|| {
        use crate::todomvc::tachys::*;
        use tachydom::view::{Render, RenderHtml};

        let rendered = TodoMVC(Todos::new_with_1000()).to_html();
        assert!(rendered.len() > 20_000)
    });
    runtime.dispose();
}

#[bench]
fn sycamore_todomvc_ssr_with_1000(b: &mut Bencher) {
    use self::sycamore::*;
    use ::sycamore::{prelude::*, *};

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
    use ::yew::{prelude::*, ServerRenderer};

    b.iter(|| {
        tokio_test::block_on(async {
            let renderer = ServerRenderer::<AppWith1000>::new();
            let rendered = renderer.render().await;
            assert!(rendered.len() > 1);
        });
    });
}

#[bench]
fn tera_todomvc_ssr(b: &mut Bencher) {
    use ::leptos::*;
    let runtime = create_runtime();
    b.iter(|| {
        use crate::todomvc::leptos::*;

        let html = ::leptos::ssr::render_to_string(|| {
            view! { <TodoMVC todos=Todos::new()/> }
        });
        assert!(html.len() > 1);
    });
    runtime.dispose();
}
