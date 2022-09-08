use leptos_reactive::Scope;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum SsrNode {
    Text(String),
    Element(SsrElement),
    DynamicText(String),
    DynamicElement(SsrElement),
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SsrElement {
    is_root_el: bool,
    tag_name: &'static str,
    attrs: Vec<(&'static str, String)>,
    inner_html: Option<String>,
    children: Vec<SsrNode>,
}

impl SsrElement {
    pub fn render_to_string(&self, cx: Scope) -> String {
        let mut buf = String::new();
        self.render_to_string_with_buf(cx, &mut buf);
        buf
    }

    pub fn render_to_string_with_buf(&self, cx: Scope, buf: &mut String) {
        // open tag
        buf.push('<');
        buf.push_str(&self.tag_name);

        // hydration key
        if self.is_root_el {
            buf.push_str(" data-hk=\"");
            buf.push_str(&cx.next_hydration_key().to_string());
            buf.push('"');
        }

        // attributes
        for (name, value) in &self.attrs {
            if value.is_empty() {
                buf.push(' ');
                buf.push_str(name);
            } else {
                buf.push(' ');
                buf.push_str(name);
                buf.push_str("=\"");
                buf.push_str(&value);
                buf.push('"');
            }
        }

        // children
        if is_self_closing(&self.tag_name) {
            buf.push_str("/>");
        } else {
            buf.push('>');

            if let Some(inner_html) = &self.inner_html {
                buf.push_str(inner_html);
            } else {
                for child in &self.children {
                    match child {
                        SsrNode::Text(value) => buf.push_str(value),
                        SsrNode::Element(el) => el.render_to_string_with_buf(cx, buf),
                        SsrNode::DynamicText(value) => {
                            buf.push_str("<!--#-->");
                            buf.push_str(value);
                            buf.push_str("<!--/-->");
                        }
                        SsrNode::DynamicElement(el) => {
                            buf.push_str("<!--#-->");
                            el.render_to_string_with_buf(cx, buf);
                            buf.push_str("<!--/-->");
                        }
                    }
                }
            }
        }
    }
}

fn is_self_closing(tag_name: &str) -> bool {
    matches!(
        tag_name,
        "area"
            | "base"
            | "br"
            | "col"
            | "embed"
            | "hr"
            | "img"
            | "input"
            | "link"
            | "meta"
            | "param"
            | "source"
            | "track"
            | "wbr"
    )
}
