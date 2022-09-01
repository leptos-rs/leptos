use proc_macro::TokenStream;
use quote::ToTokens;
use syn::{parse_macro_input, DeriveInput};
use syn_rsx::{parse, Node, NodeType};

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub(crate) enum Mode {
    Client,
    Hydrate,
    Ssr,
}

impl Default for Mode {
    fn default() -> Self {
        if cfg!(feature = "ssr") {
            Mode::Ssr
        } else if cfg!(feature = "hydrate") {
            Mode::Hydrate
        } else {
            Mode::Client
        }
    }
}

mod view;
use view::render_view;
mod component;
mod props;

#[proc_macro]
pub fn view(tokens: TokenStream) -> TokenStream {
    match parse(tokens) {
        Ok(nodes) => render_view(&nodes, Mode::default()),
        Err(error) => error.to_compile_error(),
    }
    .into()
}

#[proc_macro_attribute]
pub fn component(_args: proc_macro::TokenStream, s: TokenStream) -> TokenStream {
    match syn::parse::<component::InlinePropsBody>(s) {
        Err(e) => e.to_compile_error().into(),
        Ok(s) => s.to_token_stream().into(),
    }
}

#[proc_macro_derive(Props, attributes(builder))]
pub fn derive_prop(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    props::impl_derive_prop(&input)
        .unwrap_or_else(|err| err.to_compile_error())
        .into()
}

pub(crate) fn is_component_node(node: &Node) -> bool {
    if let NodeType::Element = node.node_type {
        node.name_as_string()
            .and_then(|node_name| node_name.chars().next())
            .map(|first_char| first_char.is_ascii_uppercase())
            .unwrap_or(false)
    } else {
        false
    }
}
