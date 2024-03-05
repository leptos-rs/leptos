use core::num::NonZeroUsize;
use leptos::*;

#[component]
fn Component(
    #[prop(optional)] optional: bool,
    #[prop(optional_no_strip)] optional_no_strip: Option<String>,
    #[prop(strip_option)] strip_option: Option<u8>,
    #[prop(default = NonZeroUsize::new(10).unwrap())] default: NonZeroUsize,
    #[prop(into)] into: String,
    #[prop(into, default = "Default value")] into_default: String,
    #[prop(into, default = NonZeroUsize::new(11).unwrap())] into_default_expr: NonZeroUsize,
) -> impl IntoView {
    _ = optional;
    _ = optional_no_strip;
    _ = strip_option;
    _ = default;
    _ = into;
    _ = into_default;
    _ = into_default_expr;
}

#[test]
fn component() {
    let cp = ComponentProps::builder().into("").strip_option(9).build();
    assert!(!cp.optional);
    assert_eq!(cp.optional_no_strip, None);
    assert_eq!(cp.strip_option, Some(9));
    assert_eq!(cp.default, NonZeroUsize::new(10).unwrap());
    assert_eq!(cp.into, "");
    assert_eq!(cp.into_default, "Default value");
    assert_eq!(cp.into_default_expr, NonZeroUsize::new(11).unwrap());
}
