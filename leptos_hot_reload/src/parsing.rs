use rstml::node::{CustomNode, NodeElement, NodeName};

/// Converts `syn::Block` to simple expression
///
/// For example:
/// ```no_build
/// // "string literal" in
/// {"string literal"}
/// // number literal
/// {0x12}
/// // boolean literal
/// {true}
/// // variable
/// {path::x}
/// ```
#[must_use]
pub fn block_to_primitive_expression(block: &syn::Block) -> Option<&syn::Expr> {
    // its empty block, or block with multi lines
    if block.stmts.len() != 1 {
        return None;
    }
    match &block.stmts[0] {
        syn::Stmt::Expr(e, None) => Some(e),
        _ => None,
    }
}

/// Converts simple literals to its string representation.
///
/// This function doesn't convert literal wrapped inside block
/// like: `{"string"}`.
#[must_use]
pub fn value_to_string(value: &syn::Expr) -> Option<String> {
    match &value {
        syn::Expr::Lit(lit) => match &lit.lit {
            syn::Lit::Str(s) => Some(s.value()),
            syn::Lit::Char(c) => Some(c.value().to_string()),
            syn::Lit::Int(i) => Some(i.base10_digits().to_string()),
            syn::Lit::Float(f) => Some(f.base10_digits().to_string()),
            _ => None,
        },
        _ => None,
    }
}

/// # Panics
///
/// Will panic if the last element does not exist in the path.
#[must_use]
pub fn is_component_tag_name(name: &NodeName) -> bool {
    match name {
        NodeName::Path(path) => {
            !path.path.segments.is_empty()
                && path
                    .path
                    .segments
                    .last()
                    .unwrap()
                    .ident
                    .to_string()
                    .starts_with(|c: char| c.is_ascii_uppercase())
        }
        NodeName::Block(_) | NodeName::Punctuated(_) => false,
    }
}

#[must_use]
pub fn is_component_node(node: &NodeElement<impl CustomNode>) -> bool {
    is_component_tag_name(node.name())
}
