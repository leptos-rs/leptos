use proc_macro::{TokenStream, TokenTree};
use proc_macro2::Span;
use proc_macro_error2::abort;
use quote::{quote, ToTokens};

const RFC3986_UNRESERVED: [char; 4] = ['-', '.', '_', '~'];
const RFC3986_PCHAR_OTHER: [char; 1] = ['@'];

/// Constructs a path for use in a [`leptos_router::Route`] definition.
///
/// Note that this is an optional convenience. Manually defining route segments
/// is equivalent.
///
/// # Examples
///
/// ```rust
/// use leptos_router::{path, ParamSegment, StaticSegment, WildcardSegment};
///
/// let path = path!("/foo/:bar/*any");
/// let output = (
///     StaticSegment("foo"),
///     ParamSegment("bar"),
///     WildcardSegment("any"),
/// );
///
/// assert_eq!(path, output);
/// ```
#[proc_macro_error2::proc_macro_error]
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
    Static(String),
    Param(String),
    Wildcard(String),
}

struct SegmentParser {
    input: proc_macro::token_stream::IntoIter,
    segments: Vec<Segment>,
}

impl SegmentParser {
    pub fn new(input: TokenStream) -> Self {
        Self {
            input: input.into_iter(),
            segments: Vec::new(),
        }
    }
}

impl SegmentParser {
    pub fn parse_all(&mut self) {
        for input in self.input.by_ref() {
            match input {
                TokenTree::Literal(lit) => {
                    let lit = lit.to_string();
                    if lit.contains("//") {
                        abort!(
                            proc_macro2::Span::call_site(),
                            "Consecutive '/' is not allowed"
                        );
                    }
                    Self::parse_str(
                        &mut self.segments,
                        lit.trim_start_matches(['"', '/'])
                            .trim_end_matches(['"', '/']),
                    );
                    if lit.ends_with(r#"/""#) && lit != r#""/""# {
                        self.segments.push(Segment::Static("/".to_string()));
                    }
                }
                TokenTree::Group(_) => unimplemented!(),
                TokenTree::Ident(_) => unimplemented!(),
                TokenTree::Punct(_) => unimplemented!(),
            }
        }
    }

    pub fn parse_str(segments: &mut Vec<Segment>, current_str: &str) {
        if ["", "*"].contains(&current_str) {
            return;
        }

        for segment in current_str.split('/') {
            if let Some(segment) = segment.strip_prefix(':') {
                segments.push(Segment::Param(segment.to_string()));
            } else if let Some(segment) = segment.strip_prefix('*') {
                segments.push(Segment::Wildcard(segment.to_string()));
            } else {
                segments.push(Segment::Static(segment.to_string()));
            }
        }
    }
}

impl Segment {
    fn is_valid(segment: &str) -> bool {
        segment == "/"
            || segment.chars().all(|c| {
                c.is_ascii_digit()
                    || c.is_ascii_lowercase()
                    || c.is_ascii_uppercase()
                    || RFC3986_UNRESERVED.contains(&c)
                    || RFC3986_PCHAR_OTHER.contains(&c)
            })
    }

    fn ensure_valid(&self) {
        match self {
            Self::Wildcard(s) if !Self::is_valid(s) => {
                abort!(Span::call_site(), "Invalid wildcard segment: {}", s)
            }
            Self::Static(s) if !Self::is_valid(s) => {
                abort!(Span::call_site(), "Invalid static segment: {}", s)
            }
            Self::Param(s) if !Self::is_valid(s) => {
                abort!(Span::call_site(), "Invalid param segment: {}", s)
            }
            _ => (),
        }
    }
}

impl Segments {
    fn ensure_valid(&self) {
        if let Some((_last, segments)) = self.0.split_last() {
            if let Some(Segment::Wildcard(s)) =
                segments.iter().find(|s| matches!(s, Segment::Wildcard(_)))
            {
                abort!(Span::call_site(), "Wildcard must be at end: {}", s)
            }
        }
    }
}

impl ToTokens for Segment {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        self.ensure_valid();
        match self {
            Segment::Wildcard(s) => {
                tokens.extend(quote! { leptos_router::WildcardSegment(#s) });
            }
            Segment::Static(s) => {
                tokens.extend(quote! { leptos_router::StaticSegment(#s) });
            }
            Segment::Param(p) => {
                tokens.extend(quote! { leptos_router::ParamSegment(#p) });
            }
        }
    }
}

impl ToTokens for Segments {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        self.ensure_valid();
        match self.0.as_slice() {
            [] => tokens.extend(quote! { () }),
            [segment] => tokens.extend(quote! { (#segment,) }),
            segments => tokens.extend(quote! { (#(#segments),*) }),
        }
    }
}
