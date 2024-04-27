use proc_macro::{TokenStream, TokenTree};
use quote::{quote, ToTokens};
use std::borrow::Cow;
use syn::{
    parse::{Parse, ParseStream},
    parse_macro_input,
    token::Token,
};

#[proc_macro_error::proc_macro_error]
#[proc_macro]
pub fn path(tokens: TokenStream) -> TokenStream {
    let mut parser = SegmentParser::new(tokens);
    parser.parse_all();
    let segments = Segments(parser.segments);
    segments.into_token_stream().into()
}

#[derive(Debug, PartialEq)]
struct Segments(pub Vec<Segment>);

#[derive(Debug, PartialEq)]
enum Segment {
    Static(Cow<'static, str>),
}

struct SegmentParser {
    input: proc_macro::token_stream::IntoIter,
    current_str: Option<String>,
    segments: Vec<Segment>,
}

impl SegmentParser {
    pub fn new(input: TokenStream) -> Self {
        Self {
            input: input.into_iter(),
            current_str: None,
            segments: Vec::new(),
        }
    }
}

impl SegmentParser {
    pub fn parse_all(&mut self) {
        for input in self.input.by_ref() {
            match input {
                TokenTree::Literal(lit) => {
                    Self::parse_str(
                        lit.to_string()
                            .trim_start_matches(['"', '/'])
                            .trim_end_matches(['"', '/']),
                        &mut self.segments,
                    );
                }
                TokenTree::Group(_) => todo!(),
                TokenTree::Ident(_) => todo!(),
                TokenTree::Punct(_) => todo!(),
            }
        }
    }

    pub fn parse_str(current_str: &str, segments: &mut Vec<Segment>) {
        let mut chars = current_str.chars();
    }
}

impl ToTokens for Segments {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        let children = quote! {};
        if self.0.len() != 1 {
            tokens.extend(quote! { (#children) });
        } else {
            tokens.extend(children)
        }
    }
}
