use leptos_hot_reload::parsing::is_component_tag_name;
use proc_macro2::{Ident, Span, TokenStream};
use quote::quote;
use rstml::node::{NodeElement, NodeName};
use syn::spanned::Spanned;

/// Helper type to emit semantic info about tags, for IDE.
/// Implement `IntoIterator` with `Item="let _ = foo::docs;"`.
///
/// `IdeTagHelper` uses warning instead of errors everywhere,
/// it's aim is to add usability, not introduce additional typecheck in `view`/`template` code.
/// On stable `emit_warning` don't produce anything.
pub(crate) struct IdeTagHelper(Vec<TokenStream>);

// TODO: Unhandled cases:
// - svg::div, my_elements::foo - tags with custom paths, that doesnt look like component
// - my_component::Foo - components with custom paths
// - html:div - tags punctuated by `:`
// - {div}, {"div"} - any rust expression
impl IdeTagHelper {
    pub fn new() -> Self {
        Self(Vec::new())
    }
    /// Save stmts for tag name.
    /// Emit warning if tag is component.
    pub fn save_tag_completion(&mut self, name: &NodeName) {
        if is_component_tag_name(name) {
            proc_macro_error2::emit_warning!(
                name.span(),
                "BUG: Component tag is used in regular tag completion."
            );
        }
        for path in Self::completion_stmts(name) {
            self.0.push(quote! {
                    let _ = #path;
            });
        }
    }

    /// Save stmts for open and close tags.
    /// Emit warning if tag is component.
    pub fn save_element_completion(&mut self, node: &NodeElement) {
        self.save_tag_completion(node.name());
        if let Some(close_tag) = node.close_tag.as_ref().map(|c| &c.name) {
            self.save_tag_completion(close_tag)
        }
    }

    /* This has been (temporarily?) removed.
     * Its purpose was simply to add syntax highlighting and IDE hints for
     * component closing tags in debug mode by associating the closing tag
     * ident with the component function.
     *
     * Doing this in a way that correctly inferred types, however, required
     * duplicating the entire component constructor.
     *
     * In view trees with many nested components, this led to a massive explosion
     * in compile times.
     *
     * See https://github.com/leptos-rs/leptos/issues/1283
     *
    /// Add completion to the closing tag of the component.
    ///
    /// In order to ensure that generics are passed through correctly in the
    /// current builder pattern, this clones the whole component constructor,
    /// but it will never be used.
    ///
    /// ```no_build
    /// if false {
    ///     close_tag(unreachable!())
    /// }
    /// else {
    ///     open_tag(open_tag.props().slots().children().build())
    /// }
    /// ```
    #[cfg(debug_assertions)]
    pub fn add_component_completion(

        component: &mut TokenStream,
        node: &NodeElement,
    ) {
        // emit ide helper info
        if let Some(close_tag) = node.close_tag.as_ref().map(|c| &c.name) {
            *component = quote! {
                {
                    let #close_tag = || #component;
                    #close_tag()
                }
            }
        }
    }
     */

    /// Returns `syn::Path`-like `TokenStream` to the fn in docs.
    /// If tag name is `Component` returns `None`.
    fn create_regular_tag_fn_path(name: &Ident) -> TokenStream {
        let tag_name = name.to_string();
        let namespace = if crate::view::is_svg_element(&tag_name) {
            quote! { ::leptos::leptos_dom::svg }
        } else if crate::view::is_math_ml_element(&tag_name) {
            quote! { ::leptos::leptos_dom::math }
        } else {
            // todo: check is html, and emit_warning in case of custom tag
            quote! { ::leptos::leptos_dom::html }
        };
        quote! { #namespace::#name }
    }

    /// Returns `syn::Path`-like `TokenStream` to the `custom` section in docs.
    fn create_custom_tag_fn_path(span: Span) -> TokenStream {
        let custom_ident = Ident::new("custom", span);
        quote! { ::leptos::leptos_dom::html::#custom_ident::<::leptos::leptos_dom::html::Custom> }
    }

    // Extract from NodeName completion idents.
    // Custom tags (like foo-bar-baz) is mapped
    // to vec!["custom", "custom",.. ] for each token in tag, even for "-".
    // Only last ident from `Path` is used.
    fn completion_stmts(name: &NodeName) -> Vec<TokenStream> {
        match name {
            NodeName::Block(_) => vec![],
            NodeName::Punctuated(c) => c
                .pairs()
                .flat_map(|c| {
                    let mut idents =
                        vec![Self::create_custom_tag_fn_path(c.value().span())];
                    if let Some(p) = c.punct() {
                        idents.push(Self::create_custom_tag_fn_path(p.span()))
                    }
                    idents
                })
                .collect(),
            NodeName::Path(e) => e
                .path
                .segments
                .last()
                .map(|p| &p.ident)
                .map(Self::create_regular_tag_fn_path)
                .into_iter()
                .collect(),
        }
    }
}

impl IntoIterator for IdeTagHelper {
    type Item = TokenStream;
    type IntoIter = <Vec<TokenStream> as IntoIterator>::IntoIter;
    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}
