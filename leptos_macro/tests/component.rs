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

#[component]
pub fn IntoLeptosValueTestComponent(
    #[prop(into)] arg1: Signal<String>,
    #[prop(into)] arg2: Signal<String>,
    #[prop(into)] arg3: Signal<String>,
    #[prop(into)] arg4: Signal<usize>,
    #[prop(into)] arg5: Signal<usize>,
    #[prop(into)] arg6: Signal<usize>,
    #[prop(into)] arg13: Callback<(), String>,
    #[prop(into)] arg14: Callback<usize, String>,
    #[prop(into)] arg15: Callback<(usize,), String>,
    #[prop(into)] arg16: Callback<(usize, String), String>,
    #[prop(into)] arg17: UnsyncCallback<(), String>,
    #[prop(into)] arg18: UnsyncCallback<usize, String>,
    #[prop(into)] arg19: UnsyncCallback<(usize,), String>,
    #[prop(into)] arg20: UnsyncCallback<(usize, String), String>,
    #[prop(into)] arg21: ArcSignal<String>,
    #[prop(into)] arg22: ArcSignal<String>,
    #[prop(into)] arg23: ArcSignal<String>,
    #[prop(into)] arg24: ArcSignal<usize>,
    #[prop(into)] arg25: ArcSignal<usize>,
    #[prop(into)] arg26: ArcSignal<usize>,
) -> impl IntoView {
    move || {
        view! {
            <div>
                <p>{arg1.get()}</p>
                <p>{arg2.get()}</p>
                <p>{arg3.get()}</p>
                <p>{arg4.get()}</p>
                <p>{arg5.get()}</p>
                <p>{arg6.get()}</p>
                <p>{arg13.run(())}</p>
                <p>{arg14.run(1)}</p>
                <p>{arg15.run((2,))}</p>
                <p>{arg16.run((3, "three".into()))}</p>
                <p>{arg17.run(())}</p>
                <p>{arg18.run(1)}</p>
                <p>{arg19.run((2,))}</p>
                <p>{arg20.run((3, "three".into()))}</p>
                <p>{arg21.get()}</p>
                <p>{arg22.get()}</p>
                <p>{arg23.get()}</p>
                <p>{arg24.get()}</p>
                <p>{arg25.get()}</p>
                <p>{arg26.get()}</p>
            </div>
        }
    }
}

#[test]
fn test_into_leptos_value() {
    let _ = view! {
        <IntoLeptosValueTestComponent
            arg1=move || "I was a reactive closure!"
            arg2="I was a basic str!"
            arg3=Signal::stored("I was already a signal!")
            arg4=move || 2
            arg5=3
            arg6=Signal::stored(4)
            arg13=|| "I was a callback static str!"
            arg14=|_n| "I was a callback static str!"
            arg15=|(_n,)| "I was a callback static str!"
            arg16=|(_n, _s)| "I was a callback static str!"
            arg17=|| "I was a callback static str!"
            arg18=|_n| "I was a callback static str!"
            arg19=|(_n,)| "I was a callback static str!"
            arg20=|(_n, _s)| "I was a callback static str!"
            arg21=move || "I was a reactive closure!"
            arg22="I was a basic str!"
            arg23=ArcSignal::stored("I was already a signal!".to_string())
            arg24=move || 2
            arg25=3
            arg26=ArcSignal::stored(4)
        />
    };
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
            view! { <div>{().into_any()} {()}</div> }
        }
    }
}
