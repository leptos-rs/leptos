use crate::{
    html::attribute::{Attr, *},
    view::add_attr::AddAnyAttr,
};
pub trait AriaAttributes<Rndr, V>
where
    Self: Sized + AddAnyAttr<Rndr>,
    V: AttributeValue<Rndr>,
    Rndr: Renderer,
{
    fn aria_atomic(
        self,
        value: V,
    ) -> <Self as AddAnyAttr<Rndr>>::Output<Attr<AriaAtomic, V, Rndr>> {
        self.add_any_attr(aria_atomic(value))
    }

    fn aria_busy(
        self,
        value: V,
    ) -> <Self as AddAnyAttr<Rndr>>::Output<Attr<AriaBusy, V, Rndr>> {
        self.add_any_attr(aria_busy(value))
    }
    fn aria_controls(
        self,
        value: V,
    ) -> <Self as AddAnyAttr<Rndr>>::Output<Attr<AriaControls, V, Rndr>> {
        self.add_any_attr(aria_controls(value))
    }
    fn aria_current(
        self,
        value: V,
    ) -> <Self as AddAnyAttr<Rndr>>::Output<Attr<AriaCurrent, V, Rndr>> {
        self.add_any_attr(aria_current(value))
    }
    fn aria_describedby(
        self,
        value: V,
    ) -> <Self as AddAnyAttr<Rndr>>::Output<Attr<AriaDescribedby, V, Rndr>>
    {
        self.add_any_attr(aria_describedby(value))
    }
    fn aria_description(
        self,
        value: V,
    ) -> <Self as AddAnyAttr<Rndr>>::Output<Attr<AriaDescription, V, Rndr>>
    {
        self.add_any_attr(aria_description(value))
    }
    fn aria_details(
        self,
        value: V,
    ) -> <Self as AddAnyAttr<Rndr>>::Output<Attr<AriaDetails, V, Rndr>> {
        self.add_any_attr(aria_details(value))
    }
    fn aria_disabled(
        self,
        value: V,
    ) -> <Self as AddAnyAttr<Rndr>>::Output<Attr<AriaDisabled, V, Rndr>> {
        self.add_any_attr(aria_disabled(value))
    }
    fn aria_dropeffect(
        self,
        value: V,
    ) -> <Self as AddAnyAttr<Rndr>>::Output<Attr<AriaDropeffect, V, Rndr>> {
        self.add_any_attr(aria_dropeffect(value))
    }
    fn aria_errormessage(
        self,
        value: V,
    ) -> <Self as AddAnyAttr<Rndr>>::Output<Attr<AriaErrormessage, V, Rndr>>
    {
        self.add_any_attr(aria_errormessage(value))
    }
    fn aria_flowto(
        self,
        value: V,
    ) -> <Self as AddAnyAttr<Rndr>>::Output<Attr<AriaFlowto, V, Rndr>> {
        self.add_any_attr(aria_flowto(value))
    }
    fn aria_grabbed(
        self,
        value: V,
    ) -> <Self as AddAnyAttr<Rndr>>::Output<Attr<AriaGrabbed, V, Rndr>> {
        self.add_any_attr(aria_grabbed(value))
    }
    fn aria_haspopup(
        self,
        value: V,
    ) -> <Self as AddAnyAttr<Rndr>>::Output<Attr<AriaHaspopup, V, Rndr>> {
        self.add_any_attr(aria_haspopup(value))
    }
    fn aria_hidden(
        self,
        value: V,
    ) -> <Self as AddAnyAttr<Rndr>>::Output<Attr<AriaHidden, V, Rndr>> {
        self.add_any_attr(aria_hidden(value))
    }
    fn aria_invalid(
        self,
        value: V,
    ) -> <Self as AddAnyAttr<Rndr>>::Output<Attr<AriaInvalid, V, Rndr>> {
        self.add_any_attr(aria_invalid(value))
    }
    fn aria_keyshortcuts(
        self,
        value: V,
    ) -> <Self as AddAnyAttr<Rndr>>::Output<Attr<AriaKeyshortcuts, V, Rndr>>
    {
        self.add_any_attr(aria_keyshortcuts(value))
    }
    fn aria_label(
        self,
        value: V,
    ) -> <Self as AddAnyAttr<Rndr>>::Output<Attr<AriaLabel, V, Rndr>> {
        self.add_any_attr(aria_label(value))
    }
    fn aria_labelledby(
        self,
        value: V,
    ) -> <Self as AddAnyAttr<Rndr>>::Output<Attr<AriaLabelledby, V, Rndr>> {
        self.add_any_attr(aria_labelledby(value))
    }
    fn aria_live(
        self,
        value: V,
    ) -> <Self as AddAnyAttr<Rndr>>::Output<Attr<AriaLive, V, Rndr>> {
        self.add_any_attr(aria_live(value))
    }
    fn aria_owns(
        self,
        value: V,
    ) -> <Self as AddAnyAttr<Rndr>>::Output<Attr<AriaOwns, V, Rndr>> {
        self.add_any_attr(aria_owns(value))
    }
    fn aria_relevant(
        self,
        value: V,
    ) -> <Self as AddAnyAttr<Rndr>>::Output<Attr<AriaRelevant, V, Rndr>> {
        self.add_any_attr(aria_relevant(value))
    }
    fn aria_roledescription(
        self,
        value: V,
    ) -> <Self as AddAnyAttr<Rndr>>::Output<Attr<AriaRoledescription, V, Rndr>>
    {
        self.add_any_attr(aria_roledescription(value))
    }
}

impl<T, Rndr, V> AriaAttributes<Rndr, V> for T
where
    T: AddAnyAttr<Rndr>,
    V: AttributeValue<Rndr>,
    Rndr: Renderer,
{
}
