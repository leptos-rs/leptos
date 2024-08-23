use wasm_bindgen::UnwrapThrowExt;

#[macro_export]
/// Use for tracing property
macro_rules! tracing_props {
    () => {
        ::leptos::tracing::span!(
            ::leptos::tracing::Level::TRACE,
            "leptos_dom::tracing_props",
            props = String::from("[]")
        );
    };
    ($($prop:tt),+ $(,)?) => {
        {
            use ::leptos::leptos_dom::macro_helpers::tracing_property::{Match, SerializeMatch, DefaultMatch};
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
            ::leptos::tracing::span!(
                ::leptos::tracing::Level::TRACE,
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

pub trait SerializeMatch {
    type Return;
    fn spez(&self) -> Self::Return;
}
impl<T: serde::Serialize> SerializeMatch for &Match<&T> {
    type Return = String;
    fn spez(&self) -> Self::Return {
        let name = self.name;

        // suppresses warnings when serializing signals into props
        #[cfg(debug_assertions)]
        let _z = reactive_graph::diagnostics::SpecialNonReactiveZone::enter();

        serde_json::to_string(self.value.get().unwrap_throw()).map_or_else(
            |err| format!(r#"{{"name": "{name}", "error": "{err}"}}"#),
            |value| format!(r#"{{"name": "{name}", "value": {value}}}"#),
        )
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
        format!(r#"{{"name": "{name}", "value": "[unserializable value]"}}"#)
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
    let test = 3.25;
    let prop = (&&Match {
        name: stringify! {test},
        value: std::cell::Cell::new(Some(&test)),
    })
        .spez();
    assert_eq!(prop, r#"{"name": "test", "value": 3.25}"#);

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
#[allow(clippy::needless_borrow)]
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
        r#"{"name": "test", "value": "[unserializable value]"}"#
    );
    // Verification of ownership
    assert_eq!(test.field, "field");
}
