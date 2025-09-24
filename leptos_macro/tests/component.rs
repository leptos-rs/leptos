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

#[cfg(not(feature = "nightly"))]
#[component]
pub fn IntoReactiveValueTestComponentSignal(
    #[prop(into)] arg1: Signal<String>,
    #[prop(into)] arg2: Signal<String>,
    #[prop(into)] arg3: Signal<String>,
    #[prop(into)] arg4: Signal<usize>,
    #[prop(into)] arg5: Signal<usize>,
    #[prop(into)] arg6: Signal<usize>,
    #[prop(into)] arg7: Signal<Option<usize>>,
    #[prop(into)] arg8: ArcSignal<String>,
    #[prop(into)] arg9: ArcSignal<String>,
    #[prop(into)] arg10: ArcSignal<String>,
    #[prop(into)] arg11: ArcSignal<usize>,
    #[prop(into)] arg12: ArcSignal<usize>,
    #[prop(into)] arg13: ArcSignal<usize>,
    #[prop(into)] arg14: ArcSignal<Option<usize>>,
    // Optionals:
    #[prop(into, optional)] arg15: Option<Signal<usize>>,
    #[prop(into, optional)] arg16_purposely_omitted: Option<Signal<usize>>,
    #[prop(into, optional)] arg17: Option<Signal<usize>>,
    #[prop(into, strip_option)] arg18: Option<Signal<usize>>,
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
                <p>{arg7.get()}</p>
                <p>{arg8.get()}</p>
                <p>{arg9.get()}</p>
                <p>{arg10.get()}</p>
                <p>{arg11.get()}</p>
                <p>{arg12.get()}</p>
                <p>{arg13.get()}</p>
                <p>{arg14.get()}</p>
                <p>{arg15.get()}</p>
                <p>{arg16_purposely_omitted.get()}</p>
                <p>{arg17.get()}</p>
                <p>{arg18.get()}</p>
            </div>
        }
    }
}

#[component]
pub fn IntoReactiveValueTestComponentCallback(
    #[prop(into)] arg1: Callback<(), String>,
    #[prop(into)] arg2: Callback<usize, String>,
    #[prop(into)] arg3: Callback<(usize,), String>,
    #[prop(into)] arg4: Callback<(usize, String), String>,
    #[prop(into)] arg5: UnsyncCallback<(), String>,
    #[prop(into)] arg6: UnsyncCallback<usize, String>,
    #[prop(into)] arg7: UnsyncCallback<(usize,), String>,
    #[prop(into)] arg8: UnsyncCallback<(usize, String), String>,
) -> impl IntoView {
    move || {
        view! {
            <div>
                <p>{arg1.run(())}</p>
                <p>{arg2.run(1)}</p>
                <p>{arg3.run((2,))}</p>
                <p>{arg4.run((3, "three".into()))}</p>
                <p>{arg5.run(())}</p>
                <p>{arg6.run(1)}</p>
                <p>{arg7.run((2,))}</p>
                <p>{arg8.run((3, "three".into()))}</p>
            </div>
        }
    }
}

#[cfg(not(feature = "nightly"))]
#[test]
fn test_into_reactive_value_signal() {
    let _ = view! {
        <IntoReactiveValueTestComponentSignal
            arg1=move || "I was a reactive closure!"
            arg2="I was a basic str!"
            arg3=Signal::stored("I was already a signal!")
            arg4=move || 2
            arg5=3
            arg6=Signal::stored(4)
            arg7=|| 2
            arg8=move || "I was a reactive closure!"
            arg9="I was a basic str!"
            arg10=ArcSignal::stored("I was already a signal!".to_string())
            arg11=move || 2
            arg12=3
            arg13=ArcSignal::stored(4)
            arg14=|| 2
            arg15=|| 2
            nostrip:arg17=Some(|| 2)
            arg18=|| 2
        />
    };
}

#[test]
fn test_into_reactive_value_callback() {
    let _ = view! {
        <IntoReactiveValueTestComponentCallback
            arg1=|| "I was a callback static str!"
            arg2=|_n| "I was a callback static str!"
            arg3=|(_n,)| "I was a callback static str!"
            arg4=|(_n, _s)| "I was a callback static str!"
            arg5=|| "I was a callback static str!"
            arg6=|_n| "I was a callback static str!"
            arg7=|(_n,)| "I was a callback static str!"
            arg8=|(_n, _s)| "I was a callback static str!"
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
