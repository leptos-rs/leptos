use super::*;
use ory_kratos_client::models::ui_node_attributes::UiNodeAttributes;
use ory_kratos_client::models::ui_node_attributes::UiNodeAttributesTypeEnum;
use ory_kratos_client::models::UiNode;
use ory_kratos_client::models::UiText;
use std::collections::HashMap;

/// https://www.ory.sh/docs/kratos/concepts/ui-user-interface
pub fn kratos_html(node: UiNode, body: RwSignal<HashMap<String, String>>) -> impl IntoView {
    // the label that goes as the child of our label
    let label_text = node.meta.label.map(|text| text.text);
    // each node MAY have messages (i.e password is bad, email is wrong form etc)
    let messages_html = view! {
        <For
        // a function that returns the items we're iterating over; a signal is fine
        each=move || node.messages.clone()
        // a unique key for each item
        key=|ui_text| ui_text.id
        // renders each item to a view
        children=move |UiText { text,_type,.. }: UiText| {
            // colored red, because we assume _type == error...
            view!{<p style="color:red;">{text}</p>}
        }
      />
    };

    let node_html = match *node.attributes {
        UiNodeAttributes::UiNodeInputAttributes {
            autocomplete,
            disabled,
            name,
            required,
            _type,
            value,
            // this is often empty for some reason?
            label: _label,
            ..
        } => {
            let autocomplete =
                autocomplete.map_or(String::new(), |t| serde_json::to_string(&t).unwrap());
            let label = label_text.unwrap_or(String::from("Unlabeled Input"));
            let required = required.unwrap_or_default();
            let _type_str = serde_json::to_string(&_type).unwrap();
            let name_clone = name.clone();
            let name_clone_2 = name.clone();
            let value = if let Some(serde_json::Value::String(value)) = value {
                value
            } else if value.is_none() {
                "".to_string()
            } else {
                match serde_json::to_string(&value) {
                    Ok(value) => value,
                    Err(err) => {
                        leptos::logging::log!("ERROR: not value? {:?}", err);
                        "".to_string()
                    }
                }
            };
            if _type == UiNodeAttributesTypeEnum::Submit {
                body.update(|map| {
                    _ = map.insert(name.clone(), value.clone());
                });
                view! {
                    // will be something like value="password" name="method"
                    // or value="oidc" name="method"
                    <input type="hidden" value=value name=name/>
                    <input type="submit" value=label/>
                }
                .into_view()
            } else if _type != UiNodeAttributesTypeEnum::Hidden {
                let id = ids::match_name_to_id(name.clone());

                view! {
                    <label>
                       <span>{&label}</span>
                      <input name=name
                      id=id
                      // we use replace here and in autocomplete because serde_json adds double quotes for some reason?
                      type=_type_str.replace("\"","")
                      value=move||body.get().get(&name_clone_2).cloned().unwrap_or_default()
                      autocomplete=autocomplete.replace("\"","")
                    disabled=disabled
                    required=required placeholder=label
                      on:input=move |ev|{
                        let name = name_clone.clone();
                        body.update(|map|{_=map.insert(name,event_target_value(&ev));})
                      }
                        />
                    </label>
                }
                .into_view()
            } else {
                body.update(|map| {
                    _ = map.insert(name.clone(), value.clone());
                });
                // this expects the identifier to be an email, but it could be telephone etc so code is extra fragile
                view! {<input type="hidden" value=value name=name /> }.into_view()
            }
        }
        UiNodeAttributes::UiNodeAnchorAttributes { href, id, title } => {
            let inner = title.text;
            view! {<a href=href id=id>{inner}</a>}.into_view()
        }
        UiNodeAttributes::UiNodeImageAttributes {
            height,
            id,
            src,
            width,
        } => view! {<img src=src height=height width=width id=id/>}.into_view(),
        UiNodeAttributes::UiNodeScriptAttributes { .. } => view! {script not supported}.into_view(),
        UiNodeAttributes::UiNodeTextAttributes {
            id,
            text:
                box UiText {
                    // not sure how to make use of context yet.
                    context: _context,
                    // redundant id?
                    id: _id,
                    text,
                    // This could be, info, error, success. i.e context for msg responses on bad input etc
                    _type,
                },
        } => view! {<p id=id>{text}</p>}.into_view(),
    };
    view! {
        {node_html}
        {messages_html}
    }
}
