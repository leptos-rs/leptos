use crate::{use_navigate, use_resolved_path};
use leptos_dom as leptos;
use leptos_dom::*;
use leptos_macro::view;
use leptos_reactive::*;
use typed_builder::TypedBuilder;
use wasm_bindgen::JsCast;

#[derive(TypedBuilder)]
pub struct FormProps {
    #[builder(default, setter(strip_option))]
    method: Option<String>,
    #[builder(default, setter(strip_option))]
    action: Option<String>,
    #[builder(default, setter(strip_option))]
    enctype: Option<String>,
    children: Vec<Element>,
}

#[allow(non_snake_case)]
pub fn Form(cx: Scope, props: FormProps) -> Element {
    let FormProps {
        method,
        action,
        enctype,
        children,
    } = props;

    let on_submit = move |ev: web_sys::Event| {
        if ev.default_prevented() {
            return;
        }
        ev.prevent_default();
        let submitter = ev.unchecked_ref::<web_sys::SubmitEvent>().submitter();
        let navigate = use_navigate(cx);

        let (form, method, action, enctype) = match &submitter {
            Some(el) => {
                if let Some(form) = el.dyn_ref::<web_sys::HtmlFormElement>() {
                    (
                        form.clone(),
                        form.get_attribute("method")
                            .unwrap_or_else(|| "get".to_string())
                            .to_lowercase(),
                        form.get_attribute("action")
                            .unwrap_or_else(|| "".to_string())
                            .to_lowercase(),
                        form.get_attribute("enctype")
                            .unwrap_or_else(|| "application/x-www-form-urlencoded".to_string())
                            .to_lowercase(),
                    )
                } else if let Some(input) = el.dyn_ref::<web_sys::HtmlInputElement>() {
                    let form = ev
                        .target()
                        .unwrap()
                        .unchecked_into::<web_sys::HtmlFormElement>();
                    (
                        form.clone(),
                        input.get_attribute("method").unwrap_or_else(|| {
                            form.get_attribute("method")
                                .unwrap_or_else(|| "get".to_string())
                                .to_lowercase()
                        }),
                        input.get_attribute("action").unwrap_or_else(|| {
                            form.get_attribute("action")
                                .unwrap_or_else(|| "".to_string())
                                .to_lowercase()
                        }),
                        input.get_attribute("enctype").unwrap_or_else(|| {
                            form.get_attribute("enctype")
                                .unwrap_or_else(|| "application/x-www-form-urlencoded".to_string())
                                .to_lowercase()
                        }),
                    )
                } else if let Some(button) = el.dyn_ref::<web_sys::HtmlButtonElement>() {
                    let form = ev
                        .target()
                        .unwrap()
                        .unchecked_into::<web_sys::HtmlFormElement>();
                    (
                        form.clone(),
                        button.get_attribute("method").unwrap_or_else(|| {
                            form.get_attribute("method")
                                .unwrap_or_else(|| "get".to_string())
                                .to_lowercase()
                        }),
                        button.get_attribute("action").unwrap_or_else(|| {
                            form.get_attribute("action")
                                .unwrap_or_else(|| "".to_string())
                                .to_lowercase()
                        }),
                        button.get_attribute("enctype").unwrap_or_else(|| {
                            form.get_attribute("enctype")
                                .unwrap_or_else(|| "application/x-www-form-urlencoded".to_string())
                                .to_lowercase()
                        }),
                    )
                } else {
                    log::warn!("<Form/> cannot be submitted from a tag other than <form>, <input>, or <button>");
                    panic!()
                }
            }
            None => {
                log::warn!("<Form/> component: no submitter found for SubmitEvent");
                panic!()
            }
        };

        if method == "get" {
            let form_data = web_sys::FormData::new_with_form(&form).unwrap_throw();
            let params =
                web_sys::UrlSearchParams::new_with_str_sequence_sequence(&form_data).unwrap_throw();
            let params = params.to_string().as_string().unwrap_or_default();
            let action = use_resolved_path(cx, move || action.clone())
                .get()
                .unwrap_or_default();
            navigate(&format!("{action}?{params}"), Default::default());
        } else {
            // TODO POST
            log::warn!("<Form/> component: POST not yet implemented");
            todo!()
        }
    };

    view! {
        <form
            method={method}
            action={action}
            enctype={enctype}
            on:submit=on_submit
        >
            {children}
        </form>
    }
}
