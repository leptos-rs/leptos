use crate::html::attribute::{global::AddAttribute, Attr, *};
pub trait AriaAttributes<Rndr, V>
where
    Self: Sized
        + AddAttribute<Attr<AriaAtomic, V, Rndr>, Rndr>
        + AddAttribute<Attr<AriaBusy, V, Rndr>, Rndr>
        + AddAttribute<Attr<AriaControls, V, Rndr>, Rndr>
        + AddAttribute<Attr<AriaCurrent, V, Rndr>, Rndr>
        + AddAttribute<Attr<AriaDescribedby, V, Rndr>, Rndr>
        + AddAttribute<Attr<AriaDescription, V, Rndr>, Rndr>
        + AddAttribute<Attr<AriaDetails, V, Rndr>, Rndr>
        + AddAttribute<Attr<AriaDisabled, V, Rndr>, Rndr>
        + AddAttribute<Attr<AriaDropeffect, V, Rndr>, Rndr>
        + AddAttribute<Attr<AriaErrormessage, V, Rndr>, Rndr>
        + AddAttribute<Attr<AriaFlowto, V, Rndr>, Rndr>
        + AddAttribute<Attr<AriaGrabbed, V, Rndr>, Rndr>
        + AddAttribute<Attr<AriaHaspopup, V, Rndr>, Rndr>
        + AddAttribute<Attr<AriaHidden, V, Rndr>, Rndr>
        + AddAttribute<Attr<AriaInvalid, V, Rndr>, Rndr>
        + AddAttribute<Attr<AriaKeyshortcuts, V, Rndr>, Rndr>
        + AddAttribute<Attr<AriaLabel, V, Rndr>, Rndr>
        + AddAttribute<Attr<AriaLabelledby, V, Rndr>, Rndr>
        + AddAttribute<Attr<AriaLive, V, Rndr>, Rndr>
        + AddAttribute<Attr<AriaOwns, V, Rndr>, Rndr>
        + AddAttribute<Attr<AriaRelevant, V, Rndr>, Rndr>
        + AddAttribute<Attr<AriaRoledescription, V, Rndr>, Rndr>,
    V: AttributeValue<Rndr>,
    Rndr: Renderer,
{
    fn aria_atomic(
        self,
        value: V,
    ) -> <Self as AddAttribute<Attr<AriaAtomic, V, Rndr>, Rndr>>::Output {
        self.add_attr(aria_atomic(value))
    }

    fn aria_busy(
        self,
        value: V,
    ) -> <Self as AddAttribute<Attr<AriaBusy, V, Rndr>, Rndr>>::Output {
        self.add_attr(aria_busy(value))
    }
    fn aria_controls(
        self,
        value: V,
    ) -> <Self as AddAttribute<Attr<AriaControls, V, Rndr>, Rndr>>::Output {
        self.add_attr(aria_controls(value))
    }
    fn aria_current(
        self,
        value: V,
    ) -> <Self as AddAttribute<Attr<AriaCurrent, V, Rndr>, Rndr>>::Output {
        self.add_attr(aria_current(value))
    }
    fn aria_describedby(
        self,
        value: V,
    ) -> <Self as AddAttribute<Attr<AriaDescribedby, V, Rndr>, Rndr>>::Output
    {
        self.add_attr(aria_describedby(value))
    }
    fn aria_description(
        self,
        value: V,
    ) -> <Self as AddAttribute<Attr<AriaDescription, V, Rndr>, Rndr>>::Output
    {
        self.add_attr(aria_description(value))
    }
    fn aria_details(
        self,
        value: V,
    ) -> <Self as AddAttribute<Attr<AriaDetails, V, Rndr>, Rndr>>::Output {
        self.add_attr(aria_details(value))
    }
    fn aria_disabled(
        self,
        value: V,
    ) -> <Self as AddAttribute<Attr<AriaDisabled, V, Rndr>, Rndr>>::Output {
        self.add_attr(aria_disabled(value))
    }
    fn aria_dropeffect(
        self,
        value: V,
    ) -> <Self as AddAttribute<Attr<AriaDropeffect, V, Rndr>, Rndr>>::Output
    {
        self.add_attr(aria_dropeffect(value))
    }
    fn aria_errormessage(
        self,
        value: V,
    ) -> <Self as AddAttribute<Attr<AriaErrormessage, V, Rndr>, Rndr>>::Output
    {
        self.add_attr(aria_errormessage(value))
    }
    fn aria_flowto(
        self,
        value: V,
    ) -> <Self as AddAttribute<Attr<AriaFlowto, V, Rndr>, Rndr>>::Output {
        self.add_attr(aria_flowto(value))
    }
    fn aria_grabbed(
        self,
        value: V,
    ) -> <Self as AddAttribute<Attr<AriaGrabbed, V, Rndr>, Rndr>>::Output {
        self.add_attr(aria_grabbed(value))
    }
    fn aria_haspopup(
        self,
        value: V,
    ) -> <Self as AddAttribute<Attr<AriaHaspopup, V, Rndr>, Rndr>>::Output {
        self.add_attr(aria_haspopup(value))
    }
    fn aria_hidden(
        self,
        value: V,
    ) -> <Self as AddAttribute<Attr<AriaHidden, V, Rndr>, Rndr>>::Output {
        self.add_attr(aria_hidden(value))
    }
    fn aria_invalid(
        self,
        value: V,
    ) -> <Self as AddAttribute<Attr<AriaInvalid, V, Rndr>, Rndr>>::Output {
        self.add_attr(aria_invalid(value))
    }
    fn aria_keyshortcuts(
        self,
        value: V,
    ) -> <Self as AddAttribute<Attr<AriaKeyshortcuts, V, Rndr>, Rndr>>::Output
    {
        self.add_attr(aria_keyshortcuts(value))
    }
    fn aria_label(
        self,
        value: V,
    ) -> <Self as AddAttribute<Attr<AriaLabel, V, Rndr>, Rndr>>::Output {
        self.add_attr(aria_label(value))
    }
    fn aria_labelledby(
        self,
        value: V,
    ) -> <Self as AddAttribute<Attr<AriaLabelledby, V, Rndr>, Rndr>>::Output
    {
        self.add_attr(aria_labelledby(value))
    }
    fn aria_live(
        self,
        value: V,
    ) -> <Self as AddAttribute<Attr<AriaLive, V, Rndr>, Rndr>>::Output {
        self.add_attr(aria_live(value))
    }
    fn aria_owns(
        self,
        value: V,
    ) -> <Self as AddAttribute<Attr<AriaOwns, V, Rndr>, Rndr>>::Output {
        self.add_attr(aria_owns(value))
    }
    fn aria_relevant(
        self,
        value: V,
    ) -> <Self as AddAttribute<Attr<AriaRelevant, V, Rndr>, Rndr>>::Output {
        self.add_attr(aria_relevant(value))
    }
    fn aria_roledescription(
        self,
        value: V,
    ) -> <Self as AddAttribute<Attr<AriaRoledescription, V, Rndr>, Rndr>>::Output
    {
        self.add_attr(aria_roledescription(value))
    }
}

impl<T, Rndr, V> AriaAttributes<Rndr, V> for T
where
    T: AddAttribute<Attr<AriaAtomic, V, Rndr>, Rndr>
        + AddAttribute<Attr<AriaBusy, V, Rndr>, Rndr>
        + AddAttribute<Attr<AriaControls, V, Rndr>, Rndr>
        + AddAttribute<Attr<AriaCurrent, V, Rndr>, Rndr>
        + AddAttribute<Attr<AriaDescribedby, V, Rndr>, Rndr>
        + AddAttribute<Attr<AriaDescription, V, Rndr>, Rndr>
        + AddAttribute<Attr<AriaDetails, V, Rndr>, Rndr>
        + AddAttribute<Attr<AriaDisabled, V, Rndr>, Rndr>
        + AddAttribute<Attr<AriaDropeffect, V, Rndr>, Rndr>
        + AddAttribute<Attr<AriaErrormessage, V, Rndr>, Rndr>
        + AddAttribute<Attr<AriaFlowto, V, Rndr>, Rndr>
        + AddAttribute<Attr<AriaGrabbed, V, Rndr>, Rndr>
        + AddAttribute<Attr<AriaHaspopup, V, Rndr>, Rndr>
        + AddAttribute<Attr<AriaHidden, V, Rndr>, Rndr>
        + AddAttribute<Attr<AriaInvalid, V, Rndr>, Rndr>
        + AddAttribute<Attr<AriaKeyshortcuts, V, Rndr>, Rndr>
        + AddAttribute<Attr<AriaLabel, V, Rndr>, Rndr>
        + AddAttribute<Attr<AriaLabelledby, V, Rndr>, Rndr>
        + AddAttribute<Attr<AriaLive, V, Rndr>, Rndr>
        + AddAttribute<Attr<AriaOwns, V, Rndr>, Rndr>
        + AddAttribute<Attr<AriaRelevant, V, Rndr>, Rndr>
        + AddAttribute<Attr<AriaRoledescription, V, Rndr>, Rndr>,
    V: AttributeValue<Rndr>,
    Rndr: Renderer,
{
}
