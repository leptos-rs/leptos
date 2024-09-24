use crate::{
    html::{
        attribute::{Attr, *},
        element::{ElementType, HtmlElement},
    },
    renderer::Rndr,
    view::{add_attr::AddAnyAttr, RenderHtml},
};

/// Applies ARIA attributes to an HTML element.
pub trait AriaAttributes<Rndr, V>
where
    Self: Sized + AddAnyAttr,
    V: AttributeValue,
{
    /// Identifies the currently active descendant of a composite widget.
    fn aria_activedescendant(
        self,
        value: V,
    ) -> <Self as AddAnyAttr>::Output<Attr<AriaActivedescendant, V>> {
        self.add_any_attr(aria_activedescendant(value))
    }

    /// Indicates whether assistive technologies will present all, or only parts of, the changed region based on the change notifications defined by the `aria-relevant` attribute.
    fn aria_atomic(
        self,
        value: V,
    ) -> <Self as AddAnyAttr>::Output<Attr<AriaAtomic, V>> {
        self.add_any_attr(aria_atomic(value))
    }

    /// Indicates whether user input completion suggestions are provided.
    fn aria_autocomplete(
        self,
        value: V,
    ) -> <Self as AddAnyAttr>::Output<Attr<AriaAutocomplete, V>> {
        self.add_any_attr(aria_autocomplete(value))
    }

    /// Indicates whether an element, and its subtree, are currently being updated.
    fn aria_busy(
        self,
        value: V,
    ) -> <Self as AddAnyAttr>::Output<Attr<AriaBusy, V>> {
        self.add_any_attr(aria_busy(value))
    }

    /// Indicates the current "checked" state of checkboxes, radio buttons, and other widgets.
    fn aria_checked(
        self,
        value: V,
    ) -> <Self as AddAnyAttr>::Output<Attr<AriaChecked, V>> {
        self.add_any_attr(aria_checked(value))
    }

    /// Defines the number of columns in a table, grid, or treegrid.
    fn aria_colcount(
        self,
        value: V,
    ) -> <Self as AddAnyAttr>::Output<Attr<AriaColcount, V>> {
        self.add_any_attr(aria_colcount(value))
    }

    /// Defines an element's column index or position with respect to the total number of columns within a table, grid, or treegrid.
    fn aria_colindex(
        self,
        value: V,
    ) -> <Self as AddAnyAttr>::Output<Attr<AriaColindex, V>> {
        self.add_any_attr(aria_colindex(value))
    }

    /// Defines the number of columns spanned by a cell or gridcell within a table, grid, or treegrid.
    fn aria_colspan(
        self,
        value: V,
    ) -> <Self as AddAnyAttr>::Output<Attr<AriaColspan, V>> {
        self.add_any_attr(aria_colspan(value))
    }

    /// Identifies the element (or elements) whose contents or presence are controlled by the current element.
    fn aria_controls(
        self,
        value: V,
    ) -> <Self as AddAnyAttr>::Output<Attr<AriaControls, V>> {
        self.add_any_attr(aria_controls(value))
    }

    /// Indicates the element that represents the current item within a container or set of related elements.
    fn aria_current(
        self,
        value: V,
    ) -> <Self as AddAnyAttr>::Output<Attr<AriaCurrent, V>> {
        self.add_any_attr(aria_current(value))
    }

    /// Identifies the element (or elements) that describes the object.
    fn aria_describedby(
        self,
        value: V,
    ) -> <Self as AddAnyAttr>::Output<Attr<AriaDescribedby, V>> {
        self.add_any_attr(aria_describedby(value))
    }

    /// Defines a string value that describes or annotates the current element.
    fn aria_description(
        self,
        value: V,
    ) -> <Self as AddAnyAttr>::Output<Attr<AriaDescription, V>> {
        self.add_any_attr(aria_description(value))
    }

    /// Identifies the element that provides additional information related to the object.
    fn aria_details(
        self,
        value: V,
    ) -> <Self as AddAnyAttr>::Output<Attr<AriaDetails, V>> {
        self.add_any_attr(aria_details(value))
    }

    /// Indicates that the element is perceivable but disabled, so it is not editable or otherwise operable.
    fn aria_disabled(
        self,
        value: V,
    ) -> <Self as AddAnyAttr>::Output<Attr<AriaDisabled, V>> {
        self.add_any_attr(aria_disabled(value))
    }

    /// Indicates what functions can be performed when a dragged object is released on the drop target.
    fn aria_dropeffect(
        self,
        value: V,
    ) -> <Self as AddAnyAttr>::Output<Attr<AriaDropeffect, V>> {
        self.add_any_attr(aria_dropeffect(value))
    }

    /// Defines the element that provides an error message related to the object.
    fn aria_errormessage(
        self,
        value: V,
    ) -> <Self as AddAnyAttr>::Output<Attr<AriaErrormessage, V>> {
        self.add_any_attr(aria_errormessage(value))
    }

    /// Indicates whether the element, or another grouping element it controls, is currently expanded or collapsed.
    fn aria_expanded(
        self,
        value: V,
    ) -> <Self as AddAnyAttr>::Output<Attr<AriaExpanded, V>> {
        self.add_any_attr(aria_expanded(value))
    }

    /// Identifies the next element (or elements) in an alternate reading order of content.
    fn aria_flowto(
        self,
        value: V,
    ) -> <Self as AddAnyAttr>::Output<Attr<AriaFlowto, V>> {
        self.add_any_attr(aria_flowto(value))
    }

    /// Indicates an element's "grabbed" state in a drag-and-drop operation.
    fn aria_grabbed(
        self,
        value: V,
    ) -> <Self as AddAnyAttr>::Output<Attr<AriaGrabbed, V>> {
        self.add_any_attr(aria_grabbed(value))
    }

    /// Indicates the availability and type of interactive popup element, such as menu or dialog, that can be triggered by an element.
    fn aria_haspopup(
        self,
        value: V,
    ) -> <Self as AddAnyAttr>::Output<Attr<AriaHaspopup, V>> {
        self.add_any_attr(aria_haspopup(value))
    }

    /// Indicates whether the element is exposed to an accessibility API.
    fn aria_hidden(
        self,
        value: V,
    ) -> <Self as AddAnyAttr>::Output<Attr<AriaHidden, V>> {
        self.add_any_attr(aria_hidden(value))
    }

    /// Indicates the entered value does not conform to the format expected by the application.
    fn aria_invalid(
        self,
        value: V,
    ) -> <Self as AddAnyAttr>::Output<Attr<AriaInvalid, V>> {
        self.add_any_attr(aria_invalid(value))
    }

    /// Indicates keyboard shortcuts that an author has implemented to activate or give focus to an element.
    fn aria_keyshortcuts(
        self,
        value: V,
    ) -> <Self as AddAnyAttr>::Output<Attr<AriaKeyshortcuts, V>> {
        self.add_any_attr(aria_keyshortcuts(value))
    }

    /// Defines a string value that labels the current element.
    fn aria_label(
        self,
        value: V,
    ) -> <Self as AddAnyAttr>::Output<Attr<AriaLabel, V>> {
        self.add_any_attr(aria_label(value))
    }

    /// Identifies the element (or elements) that labels the current element.
    fn aria_labelledby(
        self,
        value: V,
    ) -> <Self as AddAnyAttr>::Output<Attr<AriaLabelledby, V>> {
        self.add_any_attr(aria_labelledby(value))
    }

    /// Indicates that an element will be updated, and describes the types of updates the user agents, assistive technologies, and user can expect from the live region.
    fn aria_live(
        self,
        value: V,
    ) -> <Self as AddAnyAttr>::Output<Attr<AriaLive, V>> {
        self.add_any_attr(aria_live(value))
    }

    /// Indicates whether an element is modal when displayed.
    fn aria_modal(
        self,
        value: V,
    ) -> <Self as AddAnyAttr>::Output<Attr<AriaModal, V>> {
        self.add_any_attr(aria_modal(value))
    }

    /// Indicates whether a text box accepts multiple lines of input or only a single line.
    fn aria_multiline(
        self,
        value: V,
    ) -> <Self as AddAnyAttr>::Output<Attr<AriaMultiline, V>> {
        self.add_any_attr(aria_multiline(value))
    }

    /// Indicates that the user may select more than one item from the current selectable descendants.
    fn aria_multiselectable(
        self,
        value: V,
    ) -> <Self as AddAnyAttr>::Output<Attr<AriaMultiselectable, V>> {
        self.add_any_attr(aria_multiselectable(value))
    }

    /// Indicates whether the element's orientation is horizontal, vertical, or undefined.
    fn aria_orientation(
        self,
        value: V,
    ) -> <Self as AddAnyAttr>::Output<Attr<AriaOrientation, V>> {
        self.add_any_attr(aria_orientation(value))
    }

    /// Identifies an element (or elements) in order to define a visual, functional, or contextual parent/child relationship between DOM elements where the DOM hierarchy cannot be used to represent the relationship.
    fn aria_owns(
        self,
        value: V,
    ) -> <Self as AddAnyAttr>::Output<Attr<AriaOwns, V>> {
        self.add_any_attr(aria_owns(value))
    }

    /// Defines a short hint (a word or short phrase) intended to help the user with data entry when the control has no value.
    fn aria_placeholder(
        self,
        value: V,
    ) -> <Self as AddAnyAttr>::Output<Attr<AriaPlaceholder, V>> {
        self.add_any_attr(aria_placeholder(value))
    }

    /// Defines an element's number or position in the current set of listitems or treeitems.
    fn aria_posinset(
        self,
        value: V,
    ) -> <Self as AddAnyAttr>::Output<Attr<AriaPosinset, V>> {
        self.add_any_attr(aria_posinset(value))
    }

    /// Indicates the current "pressed" state of toggle buttons.
    fn aria_pressed(
        self,
        value: V,
    ) -> <Self as AddAnyAttr>::Output<Attr<AriaPressed, V>> {
        self.add_any_attr(aria_pressed(value))
    }

    /// Indicates that the element is not editable, but is otherwise operable.
    fn aria_readonly(
        self,
        value: V,
    ) -> <Self as AddAnyAttr>::Output<Attr<AriaReadonly, V>> {
        self.add_any_attr(aria_readonly(value))
    }

    /// Indicates what notifications the user agent will trigger when the accessibility tree within a live region is modified.
    fn aria_relevant(
        self,
        value: V,
    ) -> <Self as AddAnyAttr>::Output<Attr<AriaRelevant, V>> {
        self.add_any_attr(aria_relevant(value))
    }

    /// Indicates that user input is required on the element before a form may be submitted.
    fn aria_required(
        self,
        value: V,
    ) -> <Self as AddAnyAttr>::Output<Attr<AriaRequired, V>> {
        self.add_any_attr(aria_required(value))
    }

    /// Defines a human-readable, author-localized description for the role of an element.
    fn aria_roledescription(
        self,
        value: V,
    ) -> <Self as AddAnyAttr>::Output<Attr<AriaRoledescription, V>> {
        self.add_any_attr(aria_roledescription(value))
    }

    /// Defines the total number of rows in a table, grid, or treegrid.
    fn aria_rowcount(
        self,
        value: V,
    ) -> <Self as AddAnyAttr>::Output<Attr<AriaRowcount, V>> {
        self.add_any_attr(aria_rowcount(value))
    }

    /// Defines an element's row index or position with respect to the total number of rows within a table, grid, or treegrid.
    fn aria_rowindex(
        self,
        value: V,
    ) -> <Self as AddAnyAttr>::Output<Attr<AriaRowindex, V>> {
        self.add_any_attr(aria_rowindex(value))
    }

    /// Defines the number of rows spanned by a cell or gridcell within a table, grid, or treegrid.
    fn aria_rowspan(
        self,
        value: V,
    ) -> <Self as AddAnyAttr>::Output<Attr<AriaRowspan, V>> {
        self.add_any_attr(aria_rowspan(value))
    }

    /// Indicates the current "selected" state of various widgets.
    fn aria_selected(
        self,
        value: V,
    ) -> <Self as AddAnyAttr>::Output<Attr<AriaSelected, V>> {
        self.add_any_attr(aria_selected(value))
    }

    /// Defines the number of items in the current set of listitems or treeitems.
    fn aria_setsize(
        self,
        value: V,
    ) -> <Self as AddAnyAttr>::Output<Attr<AriaSetsize, V>> {
        self.add_any_attr(aria_setsize(value))
    }

    /// Indicates if items in a table or grid are sorted in ascending or descending order.
    fn aria_sort(
        self,
        value: V,
    ) -> <Self as AddAnyAttr>::Output<Attr<AriaSort, V>> {
        self.add_any_attr(aria_sort(value))
    }

    /// Defines the maximum allowed value for a range widget.
    fn aria_valuemax(
        self,
        value: V,
    ) -> <Self as AddAnyAttr>::Output<Attr<AriaValuemax, V>> {
        self.add_any_attr(aria_valuemax(value))
    }

    /// Defines the minimum allowed value for a range widget.
    fn aria_valuemin(
        self,
        value: V,
    ) -> <Self as AddAnyAttr>::Output<Attr<AriaValuemin, V>> {
        self.add_any_attr(aria_valuemin(value))
    }

    /// Defines the current value for a range widget.
    fn aria_valuenow(
        self,
        value: V,
    ) -> <Self as AddAnyAttr>::Output<Attr<AriaValuenow, V>> {
        self.add_any_attr(aria_valuenow(value))
    }

    /// Defines the human-readable text alternative of `aria-valuenow` for a range widget.
    fn aria_valuetext(
        self,
        value: V,
    ) -> <Self as AddAnyAttr>::Output<Attr<AriaValuetext, V>> {
        self.add_any_attr(aria_valuetext(value))
    }
}

impl<El, At, Ch, V> AriaAttributes<Rndr, V> for HtmlElement<El, At, Ch>
where
    El: ElementType + Send,
    At: Attribute + Send,
    Ch: RenderHtml + Send,
    V: AttributeValue,
{
}
