use proc_macro::TokenStream;
use quote::ToTokens;
use syn::{parse_macro_input, DeriveInput};
use syn_rsx::{parse, Node, NodeType};

enum Mode {
    Client,
    Hydrate,
    Dehydrate,
    Static,
}

impl Default for Mode {
    fn default() -> Self {
        Self::Client
    }
}

mod csr;
use csr::client_side_rendering;
mod component;
mod props;

#[proc_macro]
pub fn view(tokens: TokenStream) -> TokenStream {
    match parse(tokens) {
        Ok(nodes) => {
            let mode = std::env::var("LEPTOS_MODE")
                .map(|mode| match mode.to_lowercase().as_str() {
                    "client" => Mode::Client,
                    "hydrate" => Mode::Hydrate,
                    "dehydrate" => Mode::Dehydrate,
                    "static" => Mode::Static,
                    _ => Mode::Client,
                })
                .unwrap_or_default();

            match mode {
                Mode::Client => client_side_rendering(&nodes),
                _ => todo!(),
            }
        }
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
