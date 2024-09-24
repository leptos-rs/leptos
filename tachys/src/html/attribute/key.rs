use super::{Attr, AttributeValue};
use std::fmt::Debug;

/// An HTML attribute key.
pub trait AttributeKey: Clone + Send + 'static {
    /// The name of the attribute.
    const KEY: &'static str;
}

macro_rules! attributes {
	($(#[$meta:meta] $key:ident $html:literal),* $(,)?) => {
        paste::paste! {
            $(
                #[$meta]
                #[track_caller]
                pub fn $key<V>(value: V) -> Attr<[<$key:camel>], V>
				where V: AttributeValue,

                {
                    Attr([<$key:camel>], value)
                }

                #[$meta]
				#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
				pub struct [<$key:camel>];

				impl AttributeKey for [<$key:camel>] {
					const KEY: &'static str = $html;
				}
            )*
		}
    }
}

attributes! {
    // HTML
    /// The `abbr` attribute specifies an abbreviated form of the element's content.
    abbr "abbr",
    /// The `accept-charset` attribute specifies the character encodings that are to be used for the form submission.
    accept_charset "accept-charset",
    /// The `accept` attribute specifies a list of types the server accepts, typically a file type.
    accept "accept",
    /// The `accesskey` attribute specifies a shortcut key to activate or focus an element.
    accesskey "accesskey",
    /// The `action` attribute defines the URL to which the form data will be sent.
    action "action",
    /// The `align` attribute specifies the alignment of an element.
    align "align",
    /// The `allow` attribute defines a feature policy for the content in an iframe.
    allow "allow",
    /// The `allowfullscreen` attribute allows the iframe to be displayed in fullscreen mode.
    allowfullscreen "allowfullscreen",
    /// The `allowpaymentrequest` attribute allows a cross-origin iframe to invoke the Payment Request API.
    allowpaymentrequest "allowpaymentrequest",
    /// The `alt` attribute provides alternative text for an image, if the image cannot be displayed.
    alt "alt",
    // ARIA
    /// The `aria-activedescendant` attribute identifies the currently active element when DOM focus is on a composite widget, textbox, group, or application.
    aria_activedescendant "aria-activedescendant",
    /// The `aria-atomic` attribute indicates whether assistive technologies will present all, or only parts of, the changed region based on the change notifications defined by the aria-relevant attribute.
    aria_atomic "aria-atomic",
    /// The `aria-autocomplete` attribute indicates whether user input completion suggestions are provided.
    aria_autocomplete "aria-autocomplete",
    /// The `aria-busy` attribute indicates whether an element, and its subtree, are currently being updated.
    aria_busy "aria-busy",
    /// The `aria-checked` attribute indicates the current "checked" state of checkboxes, radio buttons, and other widgets.
    aria_checked "aria-checked",
    /// The `aria-colcount` attribute defines the total number of columns in a table, grid, or treegrid.
    aria_colcount "aria-colcount",
    /// The `aria-colindex` attribute defines an element's column index or position with respect to the total number of columns within a table, grid, or treegrid.
    aria_colindex "aria-colindex",
    /// The `aria-colspan` attribute defines the number of columns spanned by a cell or gridcell within a table, grid, or treegrid.
    aria_colspan "aria-colspan",
    /// The `aria-controls` attribute identifies the element (or elements) whose contents or presence are controlled by the current element.
    aria_controls "aria-controls",
    /// The `aria-current` attribute indicates the element representing the current item within a container or set of related elements.
    aria_current "aria-current",
    /// The `aria-describedby` attribute identifies the element (or elements) that describes the object.
    aria_describedby "aria-describedby",
    /// The `aria-description` attribute provides a string value that describes or annotates the current element.
    aria_description "aria-description",
    /// The `aria-details` attribute identifies the element that provides a detailed, extended description for the object.
    aria_details "aria-details",
    /// The `aria-disabled` attribute indicates that the element is perceivable but disabled, so it is not editable or otherwise operable.
    aria_disabled "aria-disabled",
    /// The `aria-dropeffect` attribute indicates what functions can be performed when a dragged object is released on the drop target.
    aria_dropeffect "aria-dropeffect",
    /// The `aria-errormessage` attribute identifies the element that provides an error message for the object.
    aria_errormessage "aria-errormessage",
    /// The `aria-expanded` attribute indicates whether an element, or another grouping element it controls, is currently expanded or collapsed.
    aria_expanded "aria-expanded",
    /// The `aria-flowto` attribute identifies the next element (or elements) in an alternate reading order of content.
    aria_flowto "aria-flowto",
    /// The `aria-grabbed` attribute indicates an element's "grabbed" state in a drag-and-drop operation.
    aria_grabbed "aria-grabbed",
    /// The `aria-haspopup` attribute indicates the availability and type of interactive popup element, such as menu or dialog, that can be triggered by an element.
    aria_haspopup "aria-haspopup",
    /// The `aria-hidden` attribute indicates whether the element is exposed to an accessibility API.
    aria_hidden "aria-hidden",
    /// The `aria-invalid` attribute indicates the entered value does not conform to the format expected by the application.
    aria_invalid "aria-invalid",
    /// The `aria-keyshortcuts` attribute indicates keyboard shortcuts that an author has implemented to activate or give focus to an element.
    aria_keyshortcuts "aria-keyshortcuts",
    /// The `aria-label` attribute defines a string value that labels the current element.
    aria_label "aria-label",
    /// The `aria-labelledby` attribute identifies the element (or elements) that labels the current element.
    aria_labelledby "aria-labelledby",
    /// The `aria-live` attribute indicates that an element will be updated, and describes the types of updates the user agents, assistive technologies, and user can expect from the live region.
    aria_live "aria-live",
    /// The `aria-modal` attribute indicates whether an element is modal when displayed.
    aria_modal "aria-modal",
    /// The `aria-multiline` attribute indicates whether a text box accepts multiple lines of input or only a single line.
    aria_multiline "aria-multiline",
    /// The `aria-multiselectable` attribute indicates that the user may select more than one item from the current selectable descendants.
    aria_multiselectable "aria-multiselectable",
    /// The `aria-orientation` attribute indicates whether the element's orientation is horizontal, vertical, or unknown/ambiguous.
    aria_orientation "aria-orientation",
    /// The `aria-owns` attribute identifies an element (or elements) in order to define a relationship between the element with `aria-owns` and the target element.
    aria_owns "aria-owns",
    /// The `aria-placeholder` attribute defines a short hint (a word or short phrase) intended to aid the user with data entry when the control has no value.
    aria_placeholder "aria-placeholder",
    /// The `aria-posinset` attribute defines an element's position within a set or treegrid.
    aria_posinset "aria-posinset",
    /// The `aria-pressed` attribute indicates the current "pressed" state of toggle buttons.
    aria_pressed "aria-pressed",
    /// The `aria-readonly` attribute indicates that the element is not editable, but is otherwise operable.
    aria_readonly "aria-readonly",
    /// The `aria-relevant` attribute indicates what user agent changes to the accessibility tree should be monitored.
    aria_relevant "aria-relevant",
    /// The `aria-required` attribute indicates that user input is required on the element before a form may be submitted.
    aria_required "aria-required",
    /// The `aria-roledescription` attribute defines a human-readable, author-localized description for the role of an element.
    aria_roledescription "aria-roledescription",
    /// The `aria-rowcount` attribute defines the total number of rows in a table, grid, or treegrid.
    aria_rowcount "aria-rowcount",
    /// The `aria-rowindex` attribute defines an element's row index or position with respect to the total number of rows within a table, grid, or treegrid.
    aria_rowindex "aria-rowindex",
    /// The `aria-rowspan` attribute defines the number of rows spanned by a cell or gridcell within a table, grid, or treegrid.
    aria_rowspan "aria-rowspan",
    /// The `aria-selected` attribute indicates the current "selected" state of various widgets.
    aria_selected "aria-selected",
    /// The `aria-setsize` attribute defines the number of items in the current set of listitems or treeitems.
    aria_setsize "aria-setsize",
    /// The `aria-sort` attribute indicates if items in a table or grid are sorted in ascending or descending order.
    aria_sort "aria-sort",
    /// The `aria-valuemax` attribute defines the maximum allowed value for a range widget.
    aria_valuemax "aria-valuemax",
    /// The `aria-valuemin` attribute defines the minimum allowed value for a range widget.
    aria_valuemin "aria-valuemin",
    /// The `aria-valuenow` attribute defines the current value for a range widget.
    aria_valuenow "aria-valuenow",
    /// The `aria-valuetext` attribute defines the human-readable text alternative of aria-valuenow for a range widget.
    aria_valuetext "aria-valuetext",
    /// The `as` attribute specifies the type of destination for the content of the link.
    r#as "as",
    /// The `async` attribute indicates that the script should be executed asynchronously.
    r#async "async",
    /// The `attributionsrc` attribute indicates that you want the browser to send an `Attribution-Reporting-Eligible` header along with a request.
    attributionsrc "attributionsrc",
    /// The `autocapitalize` attribute controls whether and how text input is automatically capitalized as it is entered/edited by the user.
    autocapitalize "autocapitalize",
    /// The `autocomplete` attribute indicates whether an input field can have its value automatically completed by the browser.
    autocomplete "autocomplete",
    /// The `autofocus` attribute indicates that an element should be focused on page load.
    autofocus "autofocus",
    /// The `autoplay` attribute indicates that the media should start playing as soon as it is loaded.
    autoplay "autoplay",
    /// The `background` attribute sets the URL of the background image for the document.
    background "background",
    /// The `bgcolor` attribute sets the background color of an element.
    bgcolor "bgcolor",
    /// The `blocking` attribute indicates that the script will block the page loading until it is executed.
    blocking "blocking",
    /// The `border` attribute sets the width of an element's border.
    border "border",
    /// The `buffered` attribute contains the time ranges that the media has been buffered.
    buffered "buffered",
    /// The `capture` attribute indicates that the user must capture media using a camera or microphone instead of selecting a file from the file picker.
    capture "capture",
    /// The `challenge` attribute specifies the challenge string that is paired with the keygen element.
    challenge "challenge",
    /// The `charset` attribute specifies the character encoding of the HTML document.
    charset "charset",
    /// The `checked` attribute indicates whether an input element is checked or not.
    checked "checked",
    /// The `cite` attribute contains a URL that points to the source of the quotation or change.
    cite "cite",
    // class is handled in ../class.rs instead
    //class "class",
    /// The `code` attribute specifies the URL of the applet's class file to be loaded and executed.
    code "code",
    /// The `color` attribute specifies the color of an element's text.
    color "color",
    /// The `cols` attribute specifies the visible width of a text area.
    cols "cols",
    /// The `colspan` attribute defines the number of columns a cell should span.
    colspan "colspan",
    /// The `content` attribute gives the value associated with the http-equiv or name attribute.
    content "content",
    /// The `contenteditable` attribute indicates whether the element's content is editable.
    contenteditable "contenteditable",
    /// The `contextmenu` attribute specifies the ID of a `<menu>` element to open as a context menu.
    contextmenu "contextmenu",
    /// The `controls` attribute indicates whether the browser should display playback controls for the media.
    controls "controls",
    /// The `controlslist` attribute allows the control of which controls to show on the media element whenever the browser shows its native controls.
    controlslist "controlslist",
    /// The `coords` attribute specifies the coordinates of an area in an image map.
    coords "coords",
    /// The `crossorigin` attribute indicates whether the resource should be fetched with a CORS request.
    crossorigin "crossorigin",
    /// The `csp` attribute allows the embedding document to define the Content Security Policy that an embedded document must agree to enforce upon itself.
    csp "csp",
    /// The `data` attribute specifies the URL of the resource that is being embedded.
    data "data",
    /// The `datetime` attribute specifies the date and time.
    datetime "datetime",
    /// The `decoding` attribute indicates the preferred method for decoding images.
    decoding "decoding",
    /// The `default` attribute indicates that the track should be enabled unless the user's preferences indicate that another track is more appropriate.
    default "default",
    /// The `defer` attribute indicates that the script should be executed after the document has been parsed.
    defer "defer",
    /// The `dir` attribute specifies the text direction for the content in an element.
    dir "dir",
    /// The `dirname` attribute identifies the text directionality of an input element.
    dirname "dirname",
    /// The `disabled` attribute indicates whether the element is disabled.
    disabled "disabled",
    /// The `disablepictureinpicture` attribute indicates that the element is not allowed to be displayed in Picture-in-Picture mode.
    disablepictureinpicture "disablepictureinpicture",
    /// The `disableremoteplayback` attribute indicates that the element is not allowed to be displayed using remote playback.
    disableremoteplayback "disableremoteplayback",
    /// The `download` attribute indicates that the linked resource is intended to be downloaded rather than displayed in the browser.
    download "download",
    /// The `draggable` attribute indicates whether the element is draggable.
    draggable "draggable",
    /// The `elementtiming` attributes marks the element for observation by the `PerformanceElementTiming` API.
    elementtiming "elementtiming",
    /// The `enctype` attribute specifies the MIME type of the form submission.
    enctype "enctype",
    /// The `enterkeyhint` attribute allows authors to specify what kind of action label or icon will be presented to users in a virtual keyboard's enter key.
    enterkeyhint "enterkeyhint",
    /// The `exportparts` attribute enables the sharing of parts of an element's shadow DOM with a containing document.
    exportparts "exportparts",
    /// The `fetchpriority` attribute allows developers to specify the priority of a resource fetch request.
    fetchpriority "fetchpriority",
    /// The `for` attribute specifies which form element a label is bound to.
    r#for "for",
    /// The `form` attribute associates the element with a form element.
    form "form",
    /// The `formaction` attribute specifies the URL that processes the form submission.
    formaction "formaction",
    /// The `formenctype` attribute specifies how the form data should be encoded when submitted.
    formenctype "formenctype",
    /// The `formmethod` attribute specifies the HTTP method to use when submitting the form.
    formmethod "formmethod",
    /// The `formnovalidate` attribute indicates that the form should not be validated when submitted.
    formnovalidate "formnovalidate",
    /// The `formtarget` attribute specifies where to display the response after submitting the form.
    formtarget "formtarget",
    /// The `headers` attribute specifies the headers associated with the element.
    headers "headers",
    /// The `height` attribute specifies the height of an element.
    height "height",
    /// The `hidden` attribute indicates that the element is not yet, or is no longer, relevant.
    hidden "hidden",
    /// The `high` attribute specifies the range that is considered to be a high value.
    high "high",
    /// The `href` attribute specifies the URL of a linked resource.
    href "href",
    /// The `hreflang` attribute specifies the language of the linked resource.
    hreflang "hreflang",
    /// The `http-equiv` attribute provides an HTTP header for the information/value of the content attribute.
    http_equiv "http-equiv",
    /// The `icon` attribute specifies the URL of an image to be used as a graphical icon for the element.
    icon "icon",
    /// The `id` attribute specifies a unique id for an element.
    id "id",
    /// The `imagesizes` attribute specifies image sizes for different page layouts.
    imagesizes "imagesizes",
    /// The `imagesrcset` attribute specifies the URLs of multiple images to be used in different situations.
    imagesrcset "imagesrcset",
    /// The `importance` attribute specifies the relative importance of the element.
    importance "importance",
    /// The `inert` attribute indicates that the element is non-interactive and won't be accessible to user interactions or assistive technologies.
    inert "inert",
    /// The `inputmode` attribute specifies the type of data that the user will enter.
    inputmode "inputmode",
    /// The `integrity` attribute contains a hash value that the browser can use to verify that the resource hasn't been altered.
    integrity "integrity",
    /// The `intrinsicsize` attribute specifies the intrinsic size of an image or video.
    intrinsicsize "intrinsicsize",
    /// The `is` attribute allows you to specify the name of a custom element.
    is "is",
    /// The `ismap` attribute indicates that the image is part of a server-side image map.
    ismap "ismap",
    /// The `itemid` attribute assigns a unique identifier to an item.
    itemid "itemid",
    /// The `itemprop` attribute adds a property to an item.
    itemprop "itemprop",
    /// The `itemref` attribute provides a list of element IDs that have additional properties for the item.
    itemref "itemref",
    /// The `itemscope` attribute creates a new item and adds it to the page's items.
    itemscope "itemscope",
    /// The `itemtype` attribute specifies the type of an item.
    itemtype "itemtype",
    /// The `keytype` attribute specifies the type of key used by the `<keygen>` element.
    keytype "keytype",
    /// The `kind` attribute specifies the kind of text track.
    kind "kind",
    /// The `label` attribute provides a user-readable title for an element.
    label "label",
    /// The `lang` attribute specifies the language of the element's content.
    lang "lang",
    /// The `language` attribute specifies the scripting language used for the script.
    language "language",
    /// The `list` attribute identifies a `<datalist>` element that contains pre-defined options for an `<input>` element.
    list "list",
    /// The `loading` attribute indicates how the browser should load the image.
    loading "loading",
    /// The `loop` attribute indicates whether the media should start over again when it reaches the end.
    r#loop "loop",
    /// The `low` attribute specifies the range that is considered to be a low value.
    low "low",
    /// The `manifest` attribute specifies the URL of a document's cache manifest.
    manifest "manifest",
    /// The `max` attribute specifies the maximum value for an input element.
    max "max",
    /// The `maxlength` attribute specifies the maximum number of characters that an input element can accept.
    maxlength "maxlength",
    /// The `media` attribute specifies what media/device the linked resource is optimized for.
    media "media",
    /// The `method` attribute specifies the HTTP method to use when submitting the form.
    method "method",
    /// The `min` attribute specifies the minimum value for an input element.
    min "min",
    /// The `minlength` attribute specifies the minimum number of characters that an input element can accept.
    minlength "minlength",
    /// The `multiple` attribute indicates whether the user can enter more than one value.
    multiple "multiple",
    /// The `muted` attribute indicates whether the audio will be initially silenced on page load.
    muted "muted",
    /// The `name` attribute specifies the name of the element.
    name "name",
    /// The `nomodule` attribute indicates that the script should not be executed in browsers that support ES modules.
    nomodule "nomodule",
    /// The `nonce` attribute provides a cryptographic nonce to ensure that a script or style is approved for execution.
    nonce "nonce",
    /// The `novalidate` attribute indicates that the form should not be validated when submitted.
    novalidate "novalidate",
    /// The `open` attribute indicates whether the details element is open or closed.
    open "open",
    /// The `optimum` attribute specifies the range that is considered to be an optimum value.
    optimum "optimum",
    /// The `part` attribute identifies the element as a shadow DOM part.
    part "part",
    /// The `pattern` attribute specifies a regular expression that the input element's value is checked against.
    pattern "pattern",
    /// The `ping` attribute contains a space-separated list of URLs to be notified if the user follows the hyperlink.
    ping "ping",
    /// The `placeholder` attribute provides a short hint that describes the expected value of the input element.
    placeholder "placeholder",
    /// The `playsinline` attribute indicates that the video should play inline in the element's playback area.
    playsinline "playsinline",
    /// The `popover` attribute indicates that an element is a popover and specifies the event that causes the popover to be shown.
    popover "popover",
    /// The `popovertarget` attribute specifies the ID of an element to toggle a popover.
    popovertarget "popovertarget",
    /// The `popovertargetaction` attribute specifies the action that shows the popover.
    popovertargetaction "popovertargetaction",
    /// The `poster` attribute specifies an image to be shown while the video is downloading or until the user hits the play button.
    poster "poster",
    /// The `preload` attribute specifies if and how the author thinks that the media file should be loaded when the page loads.
    preload "preload",
    /// The `radiogroup` attribute specifies the name of the group to which the element belongs.
    radiogroup "radiogroup",
    /// The `readonly` attribute indicates that the user cannot modify the value of the input element.
    readonly "readonly",
    /// The `referrerpolicy` attribute specifies which referrer information to include with requests.
    referrerpolicy "referrerpolicy",
    /// The `rel` attribute specifies the relationship between the current document and the linked document.
    rel "rel",
    /// The `required` attribute indicates that the user must fill in the input element before submitting the form.
    required "required",
    /// The `reversed` attribute indicates that the list should be displayed in a descending order.
    reversed "reversed",
    /// The `role` attribute defines the role of an element in the context of a web application.
    role "role",
    /// The `rows` attribute specifies the number of visible text lines for a text area.
    rows "rows",
    /// The `rowspan` attribute defines the number of rows a cell should span.
    rowspan "rowspan",
    /// The `sandbox` attribute applies extra restrictions to the content in the `<iframe>`.
    sandbox "sandbox",
    /// The `scope` attribute specifies whether a header cell is a header for a column, row, or group of columns or rows.
    scope "scope",
    /// The `scoped` attribute indicates that the styles in a `<style>` element are scoped to the parent element.
    scoped "scoped",
    /// The `selected` attribute indicates that the option is selected.
    selected "selected",
    /// The `shape` attribute specifies the shape of the area.
    shape "shape",
    /// The `size` attribute specifies the width of the input element.
    size "size",
    /// The `sizes` attribute specifies the sizes of icons for visual media.
    sizes "sizes",
    /// The `slot` attribute assigns a slot to an element.
    slot "slot",
    /// The `span` attribute defines the number of columns in a `<colgroup>` or the number of rows in a `<rowgroup>`.
    span "span",
    /// The `spellcheck` attribute indicates whether spell checking is allowed for the element.
    spellcheck "spellcheck",
    /// The `src` attribute specifies the URL of the media resource.
    src "src",
    /// The `srcdoc` attribute specifies the HTML content of the page to show in the `<iframe>`.
    srcdoc "srcdoc",
    /// The `srclang` attribute specifies the language of the text track.
    srclang "srclang",
    /// The `srcset` attribute specifies the URLs of multiple images to be used in different situations.
    srcset "srcset",
    /// The `start` attribute specifies the start value of the list.
    start "start",
    /// The `step` attribute specifies the legal number intervals for an input element.
    step "step",
    // style is handled in ../style.rs instead
    // style "style",
    /// The `summary` attribute provides a summary of the content of the table.
    summary "summary",
    /// The `tabindex` attribute specifies the tab order of an element.
    tabindex "tabindex",
    /// The `target` attribute specifies where to open the linked document.
    target "target",
    /// The `title` attribute provides additional information about an element.
    title "title",
    /// The `translate` attribute specifies whether the content of an element should be translated or not.
    translate "translate",
    /// The `type` attribute specifies the type of the element.
    r#type "type",
    /// The `usemap` attribute specifies the image map to be used by an `<img>` element.
    usemap "usemap",
    /// The `value` attribute specifies the value of the element.
    value "value",
    /// The `virtualkeyboardpolicy` attribute controls the policy for virtual keyboards.
    virtualkeyboardpolicy "virtualkeyboardpolicy",
    /// The `width` attribute specifies the width of an element.
    width "width",
    /// The `wrap` attribute specifies how the text in a text area is to be wrapped when submitted in a form.
    wrap "wrap",
    // Event Handler Attributes
    /// The `onabort` attribute specifies the event handler for the abort event.
    onabort "onabort",
    /// The `onautocomplete` attribute specifies the event handler for the autocomplete event.
    onautocomplete "onautocomplete",
    /// The `onautocompleteerror` attribute specifies the event handler for the autocompleteerror event.
    onautocompleteerror "onautocompleteerror",
    /// The `onblur` attribute specifies the event handler for the blur event.
    onblur "onblur",
    /// The `oncancel` attribute specifies the event handler for the cancel event.
    oncancel "oncancel",
    /// The `oncanplay` attribute specifies the event handler for the canplay event.
    oncanplay "oncanplay",
    /// The `oncanplaythrough` attribute specifies the event handler for the canplaythrough event.
    oncanplaythrough "oncanplaythrough",
    /// The `onchange` attribute specifies the event handler for the change event.
    onchange "onchange",
    /// The `onclick` attribute specifies the event handler for the click event.
    onclick "onclick",
    /// The `onclose` attribute specifies the event handler for the close event.
    onclose "onclose",
    /// The `oncontextmenu` attribute specifies the event handler for the contextmenu event.
    oncontextmenu "oncontextmenu",
    /// The `oncuechange` attribute specifies the event handler for the cuechange event.
    oncuechange "oncuechange",
    /// The `ondblclick` attribute specifies the event handler for the double click event.
    ondblclick "ondblclick",
    /// The `ondrag` attribute specifies the event handler for the drag event.
    ondrag "ondrag",
    /// The `ondragend` attribute specifies the event handler for the dragend event.
    ondragend "ondragend",
    /// The `ondragenter` attribute specifies the event handler for the dragenter event.
    ondragenter "ondragenter",
    /// The `ondragleave` attribute specifies the event handler for the dragleave event.
    ondragleave "ondragleave",
    /// The `ondragover` attribute specifies the event handler for the dragover event.
    ondragover "ondragover",
    /// The `ondragstart` attribute specifies the event handler for the dragstart event.
    ondragstart "ondragstart",
    /// The `ondrop` attribute specifies the event handler for the drop event.
    ondrop "ondrop",
    /// The `ondurationchange` attribute specifies the event handler for the durationchange event.
    ondurationchange "ondurationchange",
    /// The `onemptied` attribute specifies the event handler for the emptied event.
    onemptied "onemptied",
    /// The `onended` attribute specifies the event handler for the ended event.
    onended "onended",
    /// The `onerror` attribute specifies the event handler for the error event.
    onerror "onerror",
    /// The `onfocus` attribute specifies the event handler for the focus event.
    onfocus "onfocus",
    /// The `onformdata` attribute specifies the event handler for the formdata event.
    onformdata "onformdata",
    /// The `oninput` attribute specifies the event handler for the input event.
    oninput "oninput",
    /// The `oninvalid` attribute specifies the event handler for the invalid event.
    oninvalid "oninvalid",
    /// The `onkeydown` attribute specifies the event handler for the keydown event.
    onkeydown "onkeydown",
    /// The `onkeypress` attribute specifies the event handler for the keypress event.
    onkeypress "onkeypress",
    /// The `onkeyup` attribute specifies the event handler for the keyup event.
    onkeyup "onkeyup",
    /// The `onlanguagechange` attribute specifies the event handler for the languagechange event.
    onlanguagechange "onlanguagechange",
    /// The `onload` attribute specifies the event handler for the load event.
    onload "onload",
    /// The `onloadeddata` attribute specifies the event handler for the loadeddata event.
    onloadeddata "onloadeddata",
    /// The `onloadedmetadata` attribute specifies the event handler for the loadedmetadata event.
    onloadedmetadata "onloadedmetadata",
    /// The `onloadstart` attribute specifies the event handler for the loadstart event.
    onloadstart "onloadstart",
    /// The `onmousedown` attribute specifies the event handler for the mousedown event.
    onmousedown "onmousedown",
    /// The `onmouseenter` attribute specifies the event handler for the mouseenter event.
    onmouseenter "onmouseenter",
    /// The `onmouseleave` attribute specifies the event handler for the mouseleave event.
    onmouseleave "onmouseleave",
    /// The `onmousemove` attribute specifies the event handler for the mousemove event.
    onmousemove "onmousemove",
    /// The `onmouseout` attribute specifies the event handler for the mouseout event.
    onmouseout "onmouseout",
    /// The `onmouseover` attribute specifies the event handler for the mouseover event.
    onmouseover "onmouseover",
    /// The `onmouseup` attribute specifies the event handler for the mouseup event.
    onmouseup "onmouseup",
    /// The `onpause` attribute specifies the event handler for the pause event.
    onpause "onpause",
    /// The `onplay` attribute specifies the event handler for the play event.
    onplay "onplay",
    /// The `onplaying` attribute specifies the event handler for the playing event.
    onplaying "onplaying",
    /// The `onprogress` attribute specifies the event handler for the progress event.
    onprogress "onprogress",
    /// The `onratechange` attribute specifies the event handler for the ratechange event.
    onratechange "onratechange",
    /// The `onreset` attribute specifies the event handler for the reset event.
    onreset "onreset",
    /// The `onresize` attribute specifies the event handler for the resize event.
    onresize "onresize",
    /// The `onscroll` attribute specifies the event handler for the scroll event.
    onscroll "onscroll",
    /// The `onsecuritypolicyviolation` attribute specifies the event handler for the securitypolicyviolation event.
    onsecuritypolicyviolation "onsecuritypolicyviolation",
    /// The `onseeked` attribute specifies the event handler for the seeked event.
    onseeked "onseeked",
    /// The `onseeking` attribute specifies the event handler for the seeking event.
    onseeking "onseeking",
    /// The `onselect` attribute specifies the event handler for the select event.
    onselect "onselect",
    /// The `onslotchange` attribute specifies the event handler for the slotchange event.
    onslotchange "onslotchange",
    /// The `onstalled` attribute specifies the event handler for the stalled event.
    onstalled "onstalled",
    /// The `onsubmit` attribute specifies the event handler for the submit event.
    onsubmit "onsubmit",
    /// The `onsuspend` attribute specifies the event handler for the suspend event.
    onsuspend "onsuspend",
    /// The `ontimeupdate` attribute specifies the event handler for the timeupdate event.
    ontimeupdate "ontimeupdate",
    /// The `ontoggle` attribute specifies the event handler for the toggle event.
    ontoggle "ontoggle",
    /// The `onvolumechange` attribute specifies the event handler for the volumechange event.
    onvolumechange "onvolumechange",
    /// The `onwaiting` attribute specifies the event handler for the waiting event.
    onwaiting "onwaiting",
    /// The `onwebkitanimationend` attribute specifies the event handler for the webkitanimationend event.
    onwebkitanimationend "onwebkitanimationend",
    /// The `onwebkitanimationiteration` attribute specifies the event handler for the webkitanimationiteration event.
    onwebkitanimationiteration "onwebkitanimationiteration",
    /// The `onwebkitanimationstart` attribute specifies the event handler for the webkitanimationstart event.
    onwebkitanimationstart "onwebkitanimationstart",
    /// The `onwebkittransitionend` attribute specifies the event handler for the webkittransitionend event.
    onwebkittransitionend "onwebkittransitionend",
    /// The `onwheel` attribute specifies the event handler for the wheel event.
    onwheel "onwheel",

    // MathML attributes
    /// The `accent` attribute specifies whether the element should be treated as an accent.
    accent "accent",
    /// The `accentunder` attribute specifies whether the element should be treated as an accent under the base element.
    accentunder "accentunder",
    /// The `columnalign` attribute specifies the alignment of columns.
    columnalign "columnalign",
    /// The `columnlines` attribute specifies the presence of lines between columns.
    columnlines "columnlines",
    /// The `columnspacing` attribute specifies the spacing between columns.
    columnspacing "columnspacing",
    /// The `columnspan` attribute specifies the number of columns the element should span.
    columnspan "columnspan",
    /// The `depth` attribute specifies the depth of the element.
    depth "depth",
    /// The `display` attribute specifies the display style of the element.
    display "display",
    /// The `displaystyle` attribute specifies whether the element is displayed in display style.
    displaystyle "displaystyle",
    /// The `fence` attribute specifies whether the element should act as a fence.
    fence "fence",
    /// The `frame` attribute specifies the type of frame for the element.
    frame "frame",
    /// The `framespacing` attribute specifies the spacing around frames.
    framespacing "framespacing",
    /// The `linethickness` attribute specifies the thickness of lines.
    linethickness "linethickness",
    /// The `lspace` attribute specifies the space on the left side of the element.
    lspace "lspace",
    /// The `mathbackground` attribute specifies the background color of the element.
    mathbackground "mathbackground",
    /// The `mathcolor` attribute specifies the color of the element.
    mathcolor "mathcolor",
    /// The `mathsize` attribute specifies the size of the element.
    mathsize "mathsize",
    /// The `mathvariant` attribute specifies the mathematical variant of the element.
    mathvariant "mathvariant",
    /// The `maxsize` attribute specifies the maximum size of the element.
    maxsize "maxsize",
    /// The `minsize` attribute specifies the minimum size of the element.
    minsize "minsize",
    /// The `movablelimits` attribute specifies whether the limits of the element are movable.
    movablelimits "movablelimits",
    /// The `notation` attribute specifies the type of notation for the element.
    notation "notation",
    /// The `rowalign` attribute specifies the alignment of rows.
    rowalign "rowalign",
    /// The `rowlines` attribute specifies the presence of lines between rows.
    rowlines "rowlines",
    /// The `rowspacing` attribute specifies the spacing between rows.
    rowspacing "rowspacing",
    /// The `rspace` attribute specifies the space on the right side of the element.
    rspace "rspace",
    /// The `scriptlevel` attribute specifies the script level of the element.
    scriptlevel "scriptlevel",
    /// The `separator` attribute specifies whether the element is a separator.
    separator "separator",
    /// The `stretchy` attribute specifies whether the element is stretchy.
    stretchy "stretchy",
    /// The `symmetric` attribute specifies whether the element is symmetric.
    symmetric "symmetric",
    /// The `voffset` attribute specifies the vertical offset of the element.
    voffset "voffset",
    /// The `xmlns` attribute specifies the XML namespace of the element.
    xmlns "xmlns",
}
