use leptos::prelude::*;
use leptos_router::{
    components::{ParentRoute, Route},
    lazy_route, path, Lazy, LazyRoute, MatchNestedRoutes,
};

#[derive(Clone)]
pub(crate) struct OuterAView;
#[derive(Clone)]
pub(crate) struct OuterBView;
#[derive(Clone)]
pub(crate) struct OuterCView;
#[derive(Clone)]
pub(crate) struct InnerView;

#[component]
pub fn Routes5013() -> impl MatchNestedRoutes + Clone {
    view! {
        <ParentRoute path=path!("5013") view=|| view! { <View5013 /> }>
            <Route path=path!("/") view=() />
            <Route path=path!("a") view={Lazy::<OuterAView>::new()} />
            <Route path=path!("b") view={Lazy::<OuterBView>::new()} />
            <Route path=path!("c") view={Lazy::<OuterCView>::new()} />
            <Route path=path!("inner") view={Lazy::<InnerView>::new()} />
        </ParentRoute>
    }
    .into_inner()
}

#[component]
pub fn View5013() -> impl IntoView {
    #[cfg(not(feature = "lazy"))]
    view! {
        "Compile with "<pre>"--lib-features=lazy,hydrate --bin-features=lazy,ssr --split"</pre>" to test this"
    }
    #[cfg(feature = "lazy")]
    view! {
        <ul>
            <li><a href="a">Outer A</a></li>
            <li><a href="b">Outer B</a></li>
            <li><a href="c">Outer C</a></li>
            <li><a href="inner">Inner</a></li>
        </ul>
        <leptos_router::nested_router::Outlet />
    }
}

#[lazy_route]
impl LazyRoute for OuterAView {
    fn data() -> Self {
        Self
    }

    async fn preload() {
        // Test that the compiler can se through the function modifications in lazy_route
        #[expect(unused_must_use)]
        InnerView::preload();
        InnerView::preload().await;
    }

    fn view(_this: Self) -> AnyView {
        view! {
            <i>Rendered by Outer A</i>
            {|| Suspend::new(InnerView::view(InnerView::data()))}
        }
        .into_any()
    }
}

#[lazy_route]
impl LazyRoute for OuterBView {
    fn data() -> Self {
        Self
    }

    fn preload() -> impl std::future::Future<Output = ()> {
        InnerView::preload()
    }

    fn view(_this: Self) -> AnyView {
        view! {
            <b>Rendered by Outer B</b>
            {|| Suspend::new(InnerView::view(InnerView::data()))}
        }
        .into_any()
    }
}

#[lazy_route]
impl LazyRoute for OuterCView {
    fn data() -> Self {
        Self
    }

    fn view(_this: Self) -> AnyView {
        view! {
            <b>Rendered by Outer C</b><br/>
            <i>This one should result in waterflow download of InnerView</i>
            <Transition fallback=|| view!{ "Loading InnerView" }>
                {|| Suspend::new(InnerView::view(InnerView::data()))}
            </Transition>
        }
        .into_any()
    }
}

#[lazy_route]
impl LazyRoute for InnerView {
    fn data() -> Self {
        Self
    }

    fn view(_this: Self) -> AnyView {
        view! {
            <h2>About the test</h2>
            <p>
                "The outer views will render the inner view without using <Suspense> or <Transition>. "
                "Preferrably, no view here will cause a hydration error. "
                "A and B should not cause a download waterfall, but C may."
            </p>
        }
        .into_any()
    }
}
