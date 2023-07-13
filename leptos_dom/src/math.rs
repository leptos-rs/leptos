//! Exports types for working with MathML elements.

use super::{ElementDescriptor, HtmlElement};
use crate::HydrationCtx;
use cfg_if::cfg_if;
use leptos_reactive::Scope;
use std::borrow::Cow;
cfg_if! {
  if #[cfg(all(target_arch = "wasm32", feature = "web"))] {
    use once_cell::unsync::Lazy as LazyCell;
    use wasm_bindgen::JsCast;
  } else {
    use super::{HydrationKey, html::HTML_ELEMENT_DEREF_UNIMPLEMENTED_MSG};
  }
}

macro_rules! generate_math_tags {
  (
    $(
      #[$meta:meta]
      $(#[$void:ident])?
      $tag:ident $(- $second:ident $(- $third:ident)?)? $(@ $trailing_:pat)?
    ),* $(,)?
  ) => {
    paste::paste! {
      $(
        #[cfg(all(target_arch = "wasm32", feature = "web"))]
        thread_local! {
          static [<$tag:upper $(_ $second:upper $(_ $third:upper)?)?>]: LazyCell<web_sys::HtmlElement> = LazyCell::new(|| {
            crate::document()
              .create_element_ns(
                Some(wasm_bindgen::intern("http://www.w3.org/1998/Math/MathML")),
                concat![
                  stringify!($tag),
                  $(
                    "-", stringify!($second),
                    $(
                      "-", stringify!($third)
                    )?
                  )?
                ],
              )
              .unwrap()
              .unchecked_into()
          });
        }

        #[derive(Clone, Debug)]
        #[$meta]
        pub struct [<$tag:camel $($second:camel $($third:camel)?)?>] {
          #[cfg(all(target_arch = "wasm32", feature = "web"))]
          element: web_sys::HtmlElement,
          #[cfg(not(all(target_arch = "wasm32", feature = "web")))]
          id: HydrationKey,
        }

        impl Default for [<$tag:camel $($second:camel $($third:camel)?)?>] {
          fn default() -> Self {
            let id = HydrationCtx::id();

            #[cfg(all(target_arch = "wasm32", feature = "web"))]
            let element = if HydrationCtx::is_hydrating() {
              if let Some(el) = crate::document().get_element_by_id(
                &format!("_{id}")
              ) {
                #[cfg(debug_assertions)]
                assert_eq!(
                  el.node_name().to_ascii_uppercase(),
                  stringify!([<$tag:upper $(_ $second:upper $(_ $third:upper)?)?>]),
                  "SSR and CSR elements have the same hydration key but \
                  different node kinds. Check out the docs for information \
                  about this kind of hydration bug: https://leptos-rs.github.io/leptos/ssr/24_hydration_bugs.html"
                );

                el.remove_attribute("id").unwrap();

                el.unchecked_into()
              } else if let Ok(Some(el)) = crate::document().query_selector(
                &format!("[leptos-hk=_{id}]")
              ) {
                #[cfg(debug_assertions)]
                assert_eq!(
                  el.node_name().to_ascii_uppercase(),
                  stringify!([<$tag:upper $(_ $second:upper $(_ $third:upper)?)?>]),
                  "SSR and CSR elements have the same hydration key but \
                  different node kinds. Check out the docs for information \
                  about this kind of hydration bug: https://leptos-rs.github.io/leptos/ssr/24_hydration_bugs.html"
                );

                el.remove_attribute("leptos-hk").unwrap();

                el.unchecked_into()
              } else {
                crate::warn!(
                  "element with id {id} not found, ignoring it for hydration"
                );

                [<$tag:upper $(_ $second:upper $(_ $third:upper)?)?>]
                  .with(|el|
                    el.clone_node()
                      .unwrap()
                      .unchecked_into()
                  )
              }
            } else {
              [<$tag:upper $(_ $second:upper $(_ $third:upper)?)?>]
                .with(|el|
                  el.clone_node()
                    .unwrap()
                    .unchecked_into()
                )
            };

            Self {
              #[cfg(all(target_arch = "wasm32", feature = "web"))]
              element,
              #[cfg(not(all(target_arch = "wasm32", feature = "web")))]
              id
            }
          }
        }

        impl std::ops::Deref for [<$tag:camel $($second:camel $($third:camel)?)?>] {
          type Target = web_sys::Element;

          fn deref(&self) -> &Self::Target {
            #[cfg(all(target_arch = "wasm32", feature = "web"))]
            {
              use wasm_bindgen::JsCast;
              return &self.element.unchecked_ref();
            }

            #[cfg(not(all(target_arch = "wasm32", feature = "web")))]
            unimplemented!("{HTML_ELEMENT_DEREF_UNIMPLEMENTED_MSG}");
          }
        }

        impl std::convert::AsRef<web_sys::HtmlElement> for [<$tag:camel $($second:camel $($third:camel)?)?>] {
          fn as_ref(&self) -> &web_sys::HtmlElement {
            #[cfg(all(target_arch = "wasm32", feature = "web"))]
            return &self.element;

            #[cfg(not(all(target_arch = "wasm32", feature = "web")))]
            unimplemented!("{HTML_ELEMENT_DEREF_UNIMPLEMENTED_MSG}");
          }
        }

        impl ElementDescriptor for [<$tag:camel $($second:camel $($third:camel)?)?>] {
          fn name(&self) -> Cow<'static, str> {
            stringify!($tag).into()
          }

          #[cfg(not(all(target_arch = "wasm32", feature = "web")))]
          fn hydration_id(&self) -> &HydrationKey {
            &self.id
          }

          generate_math_tags! { @void $($void)? }
        }

        #[$meta]
        pub fn [<$tag $(_ $second $(_ $third)?)? $($trailing_)?>](cx: Scope) -> HtmlElement<[<$tag:camel $($second:camel $($third:camel)?)?>]> {
          HtmlElement::new(cx, [<$tag:camel $($second:camel $($third:camel)?)?>]::default())
        }
      )*
    }
  };
  (@void) => {};
  (@void void) => {
    fn is_void(&self) -> bool {
      true
    }
  };
}

generate_math_tags![
    /// MathML element.
    math,
    /// MathML element.
    mi,
    /// MathML element.
    mn,
    /// MathML element.
    mo,
    /// MathML element.
    ms,
    /// MathML element.
    mspace,
    /// MathML element.
    mtext,
    /// MathML element.
    menclose,
    /// MathML element.
    merror,
    /// MathML element.
    mfenced,
    /// MathML element.
    mfrac,
    /// MathML element.
    mpadded,
    /// MathML element.
    mphantom,
    /// MathML element.
    mroot,
    /// MathML element.
    mrow,
    /// MathML element.
    msqrt,
    /// MathML element.
    mstyle,
    /// MathML element.
    mmultiscripts,
    /// MathML element.
    mover,
    /// MathML element.
    mprescripts,
    /// MathML element.
    msub,
    /// MathML element.
    msubsup,
    /// MathML element.
    msup,
    /// MathML element.
    munder,
    /// MathML element.
    munderover,
    /// MathML element.
    mtable,
    /// MathML element.
    mtd,
    /// MathML element.
    mtr,
    /// MathML element.
    maction,
    /// MathML element.
    annotation,
    /// MathML element.
    annotation
        - xml,
    /// MathML element.
    semantics,
];
