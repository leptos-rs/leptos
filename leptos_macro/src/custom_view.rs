use proc_macro::TokenStream;
use quote::{quote, ToTokens};
use syn::{parse::Parse, parse_macro_input, ImplItem, ItemImpl};

pub fn custom_view_impl(tokens: TokenStream) -> TokenStream {
    let input = parse_macro_input!(tokens as CustomViewMacroInput);
    input.into_token_stream().into()
}

#[derive(Debug)]
struct CustomViewMacroInput {
    impl_block: ItemImpl,
}

impl Parse for CustomViewMacroInput {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let impl_block = input.parse()?;
        Ok(CustomViewMacroInput { impl_block })
    }
}

impl ToTokens for CustomViewMacroInput {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        let ItemImpl {
            impl_token,
            generics,
            self_ty,
            items,
            ..
        } = &self.impl_block;
        let impl_span = &impl_token;
        let view_ty = items
            .iter()
            .find_map(|item| match item {
                ImplItem::Type(ty) => (ty.ident == "View").then_some(&ty.ty),
                _ => None,
            })
            .unwrap_or_else(|| {
                proc_macro_error::abort!(
                    impl_span,
                    "You must include `type View = ...;` to specify the type. \
                     In most cases, this will be `type View = AnyView<Rndr>;"
                )
            });

        let view_fn = items
            .iter()
            .find_map(|item| match item {
                ImplItem::Fn(f) => {
                    (f.sig.ident == "into_view").then_some(&f.block)
                }
                _ => None,
            })
            .unwrap_or_else(|| {
                proc_macro_error::abort!(
                    impl_span,
                    "You must include `fn into_view(self) -> Self::View` to \
                     specify the view function."
                )
            });
        let generic_params = &generics.params;
        let where_preds =
            &generics.where_clause.as_ref().map(|wc| &wc.predicates);

        tokens.extend(quote! {
            #impl_token<#generic_params, Rndr> ::leptos::tachys::view::Render<Rndr> for #self_ty
                where Rndr: ::leptos::tachys::renderer::Renderer, #where_preds {
                    type State = <#view_ty as ::leptos::tachys::view::Render<Rndr>>::State;

                    fn build(self) -> Self::State {
                        let view = #view_fn;
                        view.build()
                    }

                    fn rebuild(self, state: &mut Self::State) {
                        let view = #view_fn;
                        view.rebuild(state);
                    }
                }

            #impl_token<#generic_params, Rndr> ::leptos::tachys::view::add_attr::AddAnyAttr<Rndr> for #self_ty
                where Rndr: ::leptos::tachys::renderer::Renderer, #where_preds {
                type Output<SomeNewAttr: ::leptos::tachys::html::attribute::Attribute<Rndr>> =
                    <#view_ty as ::leptos::tachys::view::add_attr::AddAnyAttr<Rndr>>::Output<SomeNewAttr>;

                fn add_any_attr<NewAttr: ::leptos::tachys::html::attribute::Attribute<Rndr>>(
                    self,
                    attr: NewAttr,
                ) -> Self::Output<NewAttr>
                where
                    Self::Output<NewAttr>: ::leptos::tachys::view::RenderHtml<Rndr>,
                {
                    let view = #view_fn;
                    view.add_any_attr(attr)
                }
            }

            #impl_token<#generic_params, Rndr> ::leptos::tachys::view::RenderHtml<Rndr> for #self_ty
                where Rndr: ::leptos::tachys::renderer::Renderer, #where_preds {
                type AsyncOutput = <#view_ty as ::leptos::tachys::view::RenderHtml<Rndr>>::AsyncOutput;
                const MIN_LENGTH: usize = <#view_ty as ::leptos::tachys::view::RenderHtml<Rndr>>::MIN_LENGTH;

                async fn resolve(self) -> Self::AsyncOutput {
                    let view = #view_fn;
                    ::leptos::tachys::view::RenderHtml::<Rndr>::resolve(view).await
                }

                fn dry_resolve(&mut self) {
                    // TODO... The problem is that view_fn expects to take self
                    // dry_resolve is the only one that takes &mut self
                    // this can only have an effect if walking over the view would read from
                    // resources that are not otherwise read synchronously, which is an interesting
                    // edge case to handle but probably (?) irrelevant for most actual use cases of
                    // this macro
                }

                fn to_html_with_buf(
                    self,
                    buf: &mut String,
                    position: &mut ::leptos::tachys::view::Position,
                    escape: bool,
                    mark_branches: bool,
                ) {
                    let view = #view_fn;
                    ::leptos::tachys::view::RenderHtml::<Rndr>::to_html_with_buf(
                        view,
                        buf,
                        position,
                        escape,
                        mark_branches
                    );
                }

                fn to_html_async_with_buf<const OUT_OF_ORDER: bool>(
                    self,
                    buf: &mut ::leptos::tachys::ssr::StreamBuilder,
                    position: &mut ::leptos::tachys::view::Position,
                    escape: bool,
                    mark_branches: bool,
                ) where
                    Self: Sized,
                {
                    let view = #view_fn;
                    ::leptos::tachys::view::RenderHtml::<Rndr>::to_html_async_with_buf::<OUT_OF_ORDER>(
                        view,
                        buf,
                        position,
                        escape,
                        mark_branches
                    );
                }

                fn hydrate<const FROM_SERVER: bool>(
                    self,
                    cursor: &::leptos::tachys::hydration::Cursor<Rndr>,
                    position: &::leptos::tachys::view::PositionState,
                ) -> Self::State {
                    let view = #view_fn;
                    ::leptos::tachys::view::RenderHtml::<Rndr>::hydrate::<FROM_SERVER>(
                        view, cursor, position
                    )
                }
            }
        });
    }
}
