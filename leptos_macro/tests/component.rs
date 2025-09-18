use core::num::NonZeroUsize;
use leptos::prelude::*;

#[derive(PartialEq, Debug)]
struct UserInfo {
    user_id: String,
    email: String,
}

#[derive(PartialEq, Debug)]
struct Admin(bool);

#[component]
fn Component(
    #[prop(optional)] optional: bool,
    #[prop(optional, into)] optional_into: Option<String>,
    #[prop(optional_no_strip)] optional_no_strip: Option<String>,
    #[prop(strip_option)] strip_option: Option<u8>,
    #[prop(default = NonZeroUsize::new(10).unwrap())] default: NonZeroUsize,
    #[prop(into)] into: String,
    impl_trait: impl Fn() -> i32 + 'static,
    #[prop(name = "data")] UserInfo { email, user_id }: UserInfo,
    #[prop(name = "tuple")] (name, id): (String, i32),
    #[prop(name = "tuple_struct")] Admin(is_admin): Admin,
    #[prop(name = "outside_name")] inside_name: i32,
) -> impl IntoView {
    _ = optional;
    _ = optional_into;
    _ = optional_no_strip;
    _ = strip_option;
    _ = default;
    _ = into;
    _ = impl_trait;
    _ = email;
    _ = user_id;
    _ = id;
    _ = name;
    _ = is_admin;
    _ = inside_name;
}

#[test]
fn component() {
    let cp = ComponentProps::builder()
        .into("")
        .strip_option(9)
        .impl_trait(|| 42)
        .data(UserInfo {
            email: "em@il".into(),
            user_id: "1".into(),
        })
        .tuple(("Joe".into(), 12))
        .tuple_struct(Admin(true))
        .outside_name(1)
        .build();
    assert!(!cp.optional);
    assert_eq!(cp.optional_into, None);
    assert_eq!(cp.optional_no_strip, None);
    assert_eq!(cp.strip_option, Some(9));
    assert_eq!(cp.default, NonZeroUsize::new(10).unwrap());
    assert_eq!(cp.into, "");
    assert_eq!((cp.impl_trait)(), 42);
    assert_eq!(
        cp.data,
        UserInfo {
            email: "em@il".into(),
            user_id: "1".into(),
        }
    );
    assert_eq!(cp.tuple, ("Joe".into(), 12));
    assert_eq!(cp.tuple_struct, Admin(true));
    assert_eq!(cp.outside_name, 1);
}

#[test]
fn component_nostrip() {
    // Should compile (using nostrip:optional_into in second <Component />)
    view! {
        <Component
            optional_into="foo"
            strip_option=9
            into=""
            impl_trait=|| 42
            data=UserInfo {
                email: "em@il".into(),
                user_id: "1".into(),
            }
            tuple=("Joe".into(), 12)
            tuple_struct=Admin(true)
            outside_name=1
        />
        <Component
            nostrip:optional_into=Some("foo")
            strip_option=9
            into=""
            impl_trait=|| 42
            data=UserInfo {
                email: "em@il".into(),
                user_id: "1".into(),
            }
            tuple=("Joe".into(), 12)
            tuple_struct=Admin(true)
            outside_name=1
        />
    };
}

#[component]
fn WithLifetime<'a>(data: &'a str) -> impl IntoView {
    _ = data;
    "static lifetime"
}

#[test]
fn returns_static_lifetime() {
    #[allow(unused)]
    fn can_return_impl_intoview_from_body() -> impl IntoView {
        let val = String::from("non_static_lifetime");
        WithLifetime(WithLifetimeProps::builder().data(&val).build())
    }
}

// an attempt to catch unhygienic macros regression
mod macro_hygiene {
    // To ensure no relative module path to leptos inside macros.
    mod leptos {}

    // doing this separately to below due to this being the smallest
    // unit with the lowest import surface.
    #[test]
    fn view() {
        use ::leptos::IntoView;
        use ::leptos_macro::{component, view};

        #[component]
        fn Component() -> impl IntoView {
            view! {
                {()}
                {()}
            }
        }
    }

    // may extend this test with other items as necessary.
    #[test]
    fn view_into_any() {
        use ::leptos::{
            prelude::{ElementChild, IntoAny},
            IntoView,
        };
        use ::leptos_macro::{component, view};

        #[component]
        fn Component() -> impl IntoView {
            view! {
                <div>
                    {().into_any()}
                    {()}
                </div>
            }
        }
    }
}

// Test for #[prop(default)] - using Default::default() for the type
#[component]
fn ComponentWithDefault(
    #[prop(default)] count: i32,
    #[prop(default)] name: String,
    #[prop(default)] enabled: bool,
) -> impl IntoView {
    view! {
        <div>
            <p>"Count: " {count}</p>
            <p>"Name: " {name}</p>
            <p>"Enabled: " {enabled}</p>
        </div>
    }
}

#[test]
fn component_with_default() {
    // Test that components with default props compile and use Default::default()
    let props1 = ComponentWithDefaultProps::builder().build();
    assert_eq!(props1.count, 0); // i32::default() is 0
    assert_eq!(props1.name, ""); // String::default() is empty string
    assert_eq!(props1.enabled, false); // bool::default() is false

    // Test with some values set
    let props2 = ComponentWithDefaultProps::builder()
        .count(42)
        .name("Test".to_string())
        .build();
    assert_eq!(props2.count, 42);
    assert_eq!(props2.name, "Test");
    assert_eq!(props2.enabled, false); // Still using default

    // Test with all values set
    let props3 = ComponentWithDefaultProps::builder()
        .count(100)
        .name("Full".to_string())
        .enabled(true)
        .build();
    assert_eq!(props3.count, 100);
    assert_eq!(props3.name, "Full");
    assert_eq!(props3.enabled, true);
}
