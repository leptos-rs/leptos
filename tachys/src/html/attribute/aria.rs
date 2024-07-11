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
    fn aria_activedescendant(
        self,
        value: V,
    ) -> <Self as AddAnyAttr<Rndr>>::Output<Attr<AriaActivedescendant, V, Rndr>>
    {
        self.add_any_attr(aria_activedescendant(value))
    }

    fn aria_atomic(
        self,
        value: V,
    ) -> <Self as AddAnyAttr<Rndr>>::Output<Attr<AriaAtomic, V, Rndr>> {
        self.add_any_attr(aria_atomic(value))
    }

    fn aria_autocomplete(
        self,
        value: V,
    ) -> <Self as AddAnyAttr<Rndr>>::Output<Attr<AriaAutocomplete, V, Rndr>>
    {
        self.add_any_attr(aria_autocomplete(value))
    }

    fn aria_busy(
        self,
        value: V,
    ) -> <Self as AddAnyAttr<Rndr>>::Output<Attr<AriaBusy, V, Rndr>> {
        self.add_any_attr(aria_busy(value))
    }

    fn aria_checked(
        self,
        value: V,
    ) -> <Self as AddAnyAttr<Rndr>>::Output<Attr<AriaChecked, V, Rndr>> {
        self.add_any_attr(aria_checked(value))
    }

    fn aria_colcount(
        self,
        value: V,
    ) -> <Self as AddAnyAttr<Rndr>>::Output<Attr<AriaColcount, V, Rndr>> {
        self.add_any_attr(aria_colcount(value))
    }

    fn aria_colindex(
        self,
        value: V,
    ) -> <Self as AddAnyAttr<Rndr>>::Output<Attr<AriaColindex, V, Rndr>> {
        self.add_any_attr(aria_colindex(value))
    }

    fn aria_colspan(
        self,
        value: V,
    ) -> <Self as AddAnyAttr<Rndr>>::Output<Attr<AriaColspan, V, Rndr>> {
        self.add_any_attr(aria_colspan(value))
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

    fn aria_expanded(
        self,
        value: V,
    ) -> <Self as AddAnyAttr<Rndr>>::Output<Attr<AriaExpanded, V, Rndr>> {
        self.add_any_attr(aria_expanded(value))
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

    fn aria_modal(
        self,
        value: V,
    ) -> <Self as AddAnyAttr<Rndr>>::Output<Attr<AriaModal, V, Rndr>> {
        self.add_any_attr(aria_modal(value))
    }

    fn aria_multiline(
        self,
        value: V,
    ) -> <Self as AddAnyAttr<Rndr>>::Output<Attr<AriaMultiline, V, Rndr>> {
        self.add_any_attr(aria_multiline(value))
    }

    fn aria_multiselectable(
        self,
        value: V,
    ) -> <Self as AddAnyAttr<Rndr>>::Output<Attr<AriaMultiselectable, V, Rndr>>
    {
        self.add_any_attr(aria_multiselectable(value))
    }

    fn aria_orientation(
        self,
        value: V,
    ) -> <Self as AddAnyAttr<Rndr>>::Output<Attr<AriaOrientation, V, Rndr>>
    {
        self.add_any_attr(aria_orientation(value))
    }

    fn aria_owns(
        self,
        value: V,
    ) -> <Self as AddAnyAttr<Rndr>>::Output<Attr<AriaOwns, V, Rndr>> {
        self.add_any_attr(aria_owns(value))
    }

    fn aria_placeholder(
        self,
        value: V,
    ) -> <Self as AddAnyAttr<Rndr>>::Output<Attr<AriaPlaceholder, V, Rndr>>
    {
        self.add_any_attr(aria_placeholder(value))
    }

    fn aria_posinset(
        self,
        value: V,
    ) -> <Self as AddAnyAttr<Rndr>>::Output<Attr<AriaPosinset, V, Rndr>> {
        self.add_any_attr(aria_posinset(value))
    }

    fn aria_pressed(
        self,
        value: V,
    ) -> <Self as AddAnyAttr<Rndr>>::Output<Attr<AriaPressed, V, Rndr>> {
        self.add_any_attr(aria_pressed(value))
    }

    fn aria_readonly(
        self,
        value: V,
    ) -> <Self as AddAnyAttr<Rndr>>::Output<Attr<AriaReadonly, V, Rndr>> {
        self.add_any_attr(aria_readonly(value))
    }

    fn aria_relevant(
        self,
        value: V,
    ) -> <Self as AddAnyAttr<Rndr>>::Output<Attr<AriaRelevant, V, Rndr>> {
        self.add_any_attr(aria_relevant(value))
    }

    fn aria_required(
        self,
        value: V,
    ) -> <Self as AddAnyAttr<Rndr>>::Output<Attr<AriaRequired, V, Rndr>> {
        self.add_any_attr(aria_required(value))
    }

    fn aria_roledescription(
        self,
        value: V,
    ) -> <Self as AddAnyAttr<Rndr>>::Output<Attr<AriaRoledescription, V, Rndr>>
    {
        self.add_any_attr(aria_roledescription(value))
    }

    fn aria_rowcount(
        self,
        value: V,
    ) -> <Self as AddAnyAttr<Rndr>>::Output<Attr<AriaRowcount, V, Rndr>> {
        self.add_any_attr(aria_rowcount(value))
    }

    fn aria_rowindex(
        self,
        value: V,
    ) -> <Self as AddAnyAttr<Rndr>>::Output<Attr<AriaRowindex, V, Rndr>> {
        self.add_any_attr(aria_rowindex(value))
    }

    fn aria_rowspan(
        self,
        value: V,
    ) -> <Self as AddAnyAttr<Rndr>>::Output<Attr<AriaRowspan, V, Rndr>> {
        self.add_any_attr(aria_rowspan(value))
    }

    fn aria_selected(
        self,
        value: V,
    ) -> <Self as AddAnyAttr<Rndr>>::Output<Attr<AriaSelected, V, Rndr>> {
        self.add_any_attr(aria_selected(value))
    }

    fn aria_setsize(
        self,
        value: V,
    ) -> <Self as AddAnyAttr<Rndr>>::Output<Attr<AriaSetsize, V, Rndr>> {
        self.add_any_attr(aria_setsize(value))
    }

    fn aria_sort(
        self,
        value: V,
    ) -> <Self as AddAnyAttr<Rndr>>::Output<Attr<AriaSort, V, Rndr>> {
        self.add_any_attr(aria_sort(value))
    }

    fn aria_valuemax(
        self,
        value: V,
    ) -> <Self as AddAnyAttr<Rndr>>::Output<Attr<AriaValuemax, V, Rndr>> {
        self.add_any_attr(aria_valuemax(value))
    }

    fn aria_valuemin(
        self,
        value: V,
    ) -> <Self as AddAnyAttr<Rndr>>::Output<Attr<AriaValuemin, V, Rndr>> {
        self.add_any_attr(aria_valuemin(value))
    }

    fn aria_valuenow(
        self,
        value: V,
    ) -> <Self as AddAnyAttr<Rndr>>::Output<Attr<AriaValuenow, V, Rndr>> {
        self.add_any_attr(aria_valuenow(value))
    }

    fn aria_valuetext(
        self,
        value: V,
    ) -> <Self as AddAnyAttr<Rndr>>::Output<Attr<AriaValuetext, V, Rndr>> {
        self.add_any_attr(aria_valuetext(value))
    }
}

impl<T, Rndr, V> AriaAttributes<Rndr, V> for T
where
    T: AddAnyAttr<Rndr>,
    V: AttributeValue<Rndr>,
    Rndr: Renderer,
{
}
