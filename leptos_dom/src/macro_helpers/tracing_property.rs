use leptos_reactive::untrack;

#[macro_export]
/// Use for tracing property
macro_rules! tracing_props {
    () => {
        ::leptos::leptos_dom::tracing::span!(
            ::leptos::leptos_dom::tracing::Level::TRACE,
            "leptos_dom::tracing_props",
            props = String::from("[]")
        );
    };
    ($($prop:tt),+ $(,)?) => {
        {
            use ::leptos::leptos_dom::tracing_property::{Match, DebugMatch, DefaultMatch};
            let mut props = String::from('[');
            $(
                let prop = (&&Match {
                    name: stringify!{$prop},
                    value: std::cell::Cell::new(Some(&$prop))
                }).spez();
                props.push_str(&format!("{prop},"));
            )*
            props.pop();
            props.push(']');
            ::leptos::leptos_dom::tracing::span!(
                ::leptos::leptos_dom::tracing::Level::TRACE,
                "leptos_dom::tracing_props",
                props
            );
        }
    };
}

// Implementation based on spez
// see https://github.com/m-ou-se/spez

pub struct Match<T> {
    pub name: &'static str,
    pub value: std::cell::Cell<Option<T>>,
}

pub trait DebugMatch {
    type Return;
    fn spez(&self) -> Self::Return;
}
impl<T: std::fmt::Debug> DebugMatch for &Match<&T> {
    type Return = String;
    fn spez(&self) -> Self::Return {
        let name = self.name;
        untrack(move || {
            format!(
                r#"{{"name": "{name}", "value": {:?}}}"#,
                self.value.get().unwrap()
            )
        })
    }
}

pub trait DefaultMatch {
    type Return;
    fn spez(&self) -> Self::Return;
}
impl<T> DefaultMatch for Match<&T> {
    type Return = String;
    fn spez(&self) -> Self::Return {
        let name = self.name;
        format!(r#"{{"name": "{name}", "value": "{{no Debug value}}"}}"#)
    }
}

#[test]
fn match_primitive() {
    // String
    let test = String::from("string");
    let prop = (&&Match {
        name: stringify! {test},
        value: std::cell::Cell::new(Some(&test)),
    })
        .spez();
    assert_eq!(prop, r#"{"name": "test", "value": "string"}"#);

    // &str
    let test = "string";
    let prop = (&&Match {
        name: stringify! {test},
        value: std::cell::Cell::new(Some(&test)),
    })
        .spez();
    assert_eq!(prop, r#"{"name": "test", "value": "string"}"#);

    // u128
    let test: u128 = 1;
    let prop = (&&Match {
        name: stringify! {test},
        value: std::cell::Cell::new(Some(&test)),
    })
        .spez();
    assert_eq!(prop, r#"{"name": "test", "value": 1}"#);

    // i128
    let test: i128 = -1;
    let prop = (&&Match {
        name: stringify! {test},
        value: std::cell::Cell::new(Some(&test)),
    })
        .spez();
    assert_eq!(prop, r#"{"name": "test", "value": -1}"#);

    // f64
    let test = 3.14;
    let prop = (&&Match {
        name: stringify! {test},
        value: std::cell::Cell::new(Some(&test)),
    })
        .spez();
    assert_eq!(prop, r#"{"name": "test", "value": 3.14}"#);

    // bool
    let test = true;
    let prop = (&&Match {
        name: stringify! {test},
        value: std::cell::Cell::new(Some(&test)),
    })
        .spez();
    assert_eq!(prop, r#"{"name": "test", "value": true}"#);
}

#[test]
fn match_serialize() {
    use serde::Serialize;
    #[derive(Serialize)]
    struct CustomStruct {
        field: &'static str,
    }

    let test = CustomStruct { field: "field" };
    let prop = (&&Match {
        name: stringify! {test},
        value: std::cell::Cell::new(Some(&test)),
    })
        .spez();
    assert_eq!(prop, r#"{"name": "test", "value": {"field":"field"}}"#);
    // Verification of ownership
    assert_eq!(test.field, "field");
}

#[test]
fn match_no_serialize() {
    struct CustomStruct {
        field: &'static str,
    }

    let test = CustomStruct { field: "field" };
    let prop = (&&Match {
        name: stringify! {test},
        value: std::cell::Cell::new(Some(&test)),
    })
        .spez();
    assert_eq!(
        prop,
        r#"{"name": "test", "error": "The trait `serde::Serialize` is not implemented"}"#
    );
    // Verification of ownership
    assert_eq!(test.field, "field");
}
