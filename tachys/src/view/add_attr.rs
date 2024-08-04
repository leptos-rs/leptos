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
    /// The new type once the attribute has been added.
    type Output<SomeNewAttr: Attribute<Rndr>>: RenderHtml<Rndr>;

    /// Adds an attribute to the view.
    fn add_any_attr<NewAttr: Attribute<Rndr>>(
        self,
        attr: NewAttr,
    ) -> Self::Output<NewAttr>
    where
        Self::Output<NewAttr>: RenderHtml<Rndr>;
}

/// Declares that spreading attributes onto a particular type has no effect.
#[macro_export]
macro_rules! no_attrs {
    ($ty_name:ty) => {
        impl<'a, R> $crate::view::add_attr::AddAnyAttr<R> for $ty_name
        where
            R: Renderer,
        {
            type Output<SomeNewAttr: $crate::html::attribute::Attribute<R>> =
                $ty_name;

            fn add_any_attr<NewAttr: $crate::html::attribute::Attribute<R>>(
                self,
                _attr: NewAttr,
            ) -> Self::Output<NewAttr> {
                self
            }
        }
    };
}
