//! Exports types for working with SVG elements.

#[cfg(not(all(target_arch = "wasm32", feature = "web")))]
use super::{html::HTML_ELEMENT_DEREF_UNIMPLEMENTED_MSG, HydrationKey};
use super::{ElementDescriptor, HtmlElement};
use crate::HydrationCtx;
use leptos_reactive::Scope;
#[cfg(all(target_arch = "wasm32", feature = "web"))]
use once_cell::unsync::Lazy as LazyCell;
use std::borrow::Cow;
#[cfg(all(target_arch = "wasm32", feature = "web"))]
use wasm_bindgen::JsCast;

macro_rules! generate_svg_tags {
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
                Some(wasm_bindgen::intern("http://www.w3.org/2000/svg")),
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
          type Target = web_sys::SvgElement;

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

          generate_svg_tags! { @void $($void)? }
        }

        #[$meta]
        #[allow(non_snake_case)]
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

generate_svg_tags![
  /// SVG Element.
  a,
  /// SVG Element.
  animate,
  /// SVG Element.
  animateMotion,
  /// SVG Element.
  animateTransform,
  /// SVG Element.
  circle,
  /// SVG Element.
  clipPath,
  /// SVG Element.
  defs,
  /// SVG Element.
  desc,
  /// SVG Element.
  discard,
  /// SVG Element.
  ellipse,
  /// SVG Element.
  feBlend,
  /// SVG Element.
  feColorMatrix,
  /// SVG Element.
  feComponentTransfer,
  /// SVG Element.
  feComposite,
  /// SVG Element.
  feConvolveMatrix,
  /// SVG Element.
  feDiffuseLighting,
  /// SVG Element.
  feDisplacementMap,
  /// SVG Element.
  feDistantLight,
  /// SVG Element.
  feDropShadow,
  /// SVG Element.
  feFlood,
  /// SVG Element.
  feFuncA,
  /// SVG Element.
  feFuncB,
  /// SVG Element.
  feFuncG,
  /// SVG Element.
  feFuncR,
  /// SVG Element.
  feGaussianBlur,
  /// SVG Element.
  feImage,
  /// SVG Element.
  feMerge,
  /// SVG Element.
  feMergeNode,
  /// SVG Element.
  feMorphology,
  /// SVG Element.
  feOffset,
  /// SVG Element.
  fePointLight,
  /// SVG Element.
  feSpecularLighting,
  /// SVG Element.
  feSpotLight,
  /// SVG Element.
  feTile,
  /// SVG Element.
  feTurbulence,
  /// SVG Element.
  filter,
  /// SVG Element.
  foreignObject,
  /// SVG Element.
  g,
  /// SVG Element.
  hatch,
  /// SVG Element.
  hatchpath,
  /// SVG Element.
  image,
  /// SVG Element.
  line,
  /// SVG Element.
  linearGradient,
  /// SVG Element.
  marker,
  /// SVG Element.
  mask,
  /// SVG Element.
  metadata,
  /// SVG Element.
  mpath,
  /// SVG Element.
  path,
  /// SVG Element.
  pattern,
  /// SVG Element.
  polygon,
  /// SVG Element.
  polyline,
  /// SVG Element.
  radialGradient,
  /// SVG Element.
  rect,
  /// SVG Element.
  script,
  /// SVG Element.
  set,
  /// SVG Element.
  stop,
  /// SVG Element.
  style,
  /// SVG Element.
  svg,
  /// SVG Element.
  switch,
  /// SVG Element.
  symbol,
  /// SVG Element.
  text,
  /// SVG Element.
  textPath,
  /// SVG Element.
  title,
  /// SVG Element.
  tspan,
  /// SVG Element.
  use @_,
  /// SVG Element.
  view,
];
