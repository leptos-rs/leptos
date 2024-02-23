use super::RenderHtml;
use crate::{html::attribute::Attribute, renderer::Renderer};

/// Allows adding a new attribute to some type, before it is rendered.
/// This takes place at compile time as part of the builder syntax for creating a statically typed
/// view tree.
///
/// Normally, this is used to add an attribute to an HTML element. But it is required to be
/// implemented for all types that implement [`RenderHtml`], so that attributes can be spread onto
/// other structures like the return type of a component.
pub trait AddAnyAttr<Rndr>
where
    Rndr: Renderer,
{
    type Output<SomeNewAttr: Attribute<Rndr>>: RenderHtml<Rndr>;

    fn add_any_attr<NewAttr: Attribute<Rndr>>(
        self,
        attr: NewAttr,
    ) -> Self::Output<NewAttr>
    where
        Self::Output<NewAttr>: RenderHtml<Rndr>;

    fn add_any_attr_by_ref<NewAttr: Attribute<Rndr>>(
        self,
        attr: &NewAttr,
    ) -> Self::Output<NewAttr>
    where
        Self::Output<NewAttr>: RenderHtml<Rndr>;
}
