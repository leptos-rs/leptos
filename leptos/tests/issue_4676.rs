#[cfg(feature = "ssr")]
use leptos::attribute_interceptor::AttributeInterceptor;
#[cfg(feature = "ssr")]
use leptos::prelude::*;

#[cfg(feature = "ssr")]
#[component]
pub fn Child() -> impl IntoView {
    view! {
        <AttributeInterceptor let:attrs>
            <div id="wrapper">
                <div id="inner" {..attrs} />
            </div>
        </AttributeInterceptor>
    }
}

#[cfg(feature = "ssr")]
#[component]
pub fn Parent() -> impl IntoView {
    let spread_onto_component = view! {
        <{..} aria-label="a component with attribute spreading"/>
    };

    view! {
        <Child {..spread_onto_component} />
    }
}

#[cfg(feature = "ssr")]
#[test]
fn test_attribute_interceptor_erased() {
    // Test the output html
    let html = Parent().into_view().to_html();

    assert!(html.contains(
        "<div id=\"inner\" aria-label=\"a component with attribute \
         spreading\"></div>"
    ));
}
