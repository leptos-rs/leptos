// Recursive expansion of view! macro
// ===================================
fn x() {
    {
        let _ = leptos::leptos_dom::html::div;
        leptos::leptos_dom::html::div(cx).attr("class",(cx,"container")).child((cx,{
let _ = leptos::leptos_dom::html::div;
leptos::leptos_dom::html::div(cx).attr("class",(cx,"jumbotron")).child((cx,{
  let _ = leptos::leptos_dom::html::div;
  leptos::leptos_dom::html::div(cx).attr("class",(cx,"row")).child((cx,{
    let _ = leptos::leptos_dom::html::div;
    leptos::leptos_dom::html::div(cx).attr("class",(cx,"col-md-6")).child((cx,{
      let _ = leptos::leptos_dom::html::h1;
      leptos::leptos_dom::html::h1(cx).child("Leptos")
    }))
  })).child((cx,{
    let _ = leptos::leptos_dom::html::div;
    leptos::leptos_dom::html::div(cx).attr("class",(cx,"col-md-6")).child((cx,{
      let _ = leptos::leptos_dom::html::div;
      leptos::leptos_dom::html::div(cx).attr("class",(cx,"row")).child((cx,if false {
        #[allow(unreachable_code)]
        Button(cx,std::panicking::panic("internal error: entered unreachable code"))
      }else {
        Button(cx, ::leptos::component_props_builder(&Button).id(#[allow(unused_braces)]
        "run").text(#[allow(unused_braces)]
        "Create 1,000 rows").build())
      }.into_view(cx).on(::leptos::leptos_dom::ev::undelegated(::leptos::leptos_dom::ev::click),run))).child((cx,Button(cx, ::leptos::component_props_builder(&Button).id(#[allow(unused_braces)]
      "runlots").text(#[allow(unused_braces)]
      "Create 10,000 rows").build()).into_view(cx).on(::leptos::leptos_dom::ev::undelegated(::leptos::leptos_dom::ev::click),run_lots))).child((cx,Button(cx, ::leptos::component_props_builder(&Button).id(#[allow(unused_braces)]
      "add").text(#[allow(unused_braces)]
      "Append 1,000 rows").build()).into_view(cx).on(::leptos::leptos_dom::ev::undelegated(::leptos::leptos_dom::ev::click),add))).child((cx,Button(cx, ::leptos::component_props_builder(&Button).id(#[allow(unused_braces)]
      "update").text(#[allow(unused_braces)]
      "Update every 10th row").build()).into_view(cx).on(::leptos::leptos_dom::ev::undelegated(::leptos::leptos_dom::ev::click),update))).child((cx,Button(cx, ::leptos::component_props_builder(&Button).id(#[allow(unused_braces)]
      "clear").text(#[allow(unused_braces)]
      "Clear").build()).into_view(cx).on(::leptos::leptos_dom::ev::undelegated(::leptos::leptos_dom::ev::click),clear))).child((cx,Button(cx, ::leptos::component_props_builder(&Button).id(#[allow(unused_braces)]
      "swaprows").text(#[allow(unused_braces)]
      "Swap Rows").build()).into_view(cx).on(::leptos::leptos_dom::ev::undelegated(::leptos::leptos_dom::ev::click),swap_rows)))
    }))
  }))
}))
})).child((cx,{
let _ = leptos::leptos_dom::html::table;
leptos::leptos_dom::html::table(cx).attr("class",(cx,"table table-hover table-striped test-data")).child((cx,{
  let _ = leptos::leptos_dom::html::tbody;
  leptos::leptos_dom::html::tbody(cx).child((cx,For(cx, ::leptos::component_props_builder(&For).each(#[allow(unused_braces)]
  {
    data
  }).key(#[allow(unused_braces)]
  {
    |row|row.id
  }).view(#[allow(unused_braces)]
  move|cx,row:RowData|{
    let row_id = row.id;
    let(label,_) = row.label;
    let is_selected = is_selected.clone();
    {
      const TEMPLATE_ecd03fbf3f234416805d1420eb58bcee:std::thread::LocalKey<leptos::web_sys::HtmlTemplateElement>  = {
        #[inline]
        fn __init() -> leptos::web_sys::HtmlTemplateElement {
          {
            let document = leptos::document();
            let el = document.create_element("template").unwrap();
            el.set_inner_html("<tr><td class=\"col-md-1\"><!></td><td class=\"col-md-4\"><a><!></a></td><td class=\"col-md-1\"><a><span class=\"glyphicon glyphicon-remove\" aria-hidden=\"true\"></span></a></td><td class=\"col-md-6\"></td></tr>");
            leptos::wasm_bindgen::JsCast::unchecked_into(el)
          }
        }
        #[inline]
        unsafe fn __getit(init:std::option::Option<&mut std::option::Option<leptos::web_sys::HtmlTemplateElement>>,) -> std::option::Option<&'static leptos::web_sys::HtmlTemplateElement>{
          static __KEY:std::thread::__LocalKeyInner<leptos::web_sys::HtmlTemplateElement>  = std::thread::__LocalKeyInner::new();
          #[allow(unused_unsafe)]
          unsafe {
            __KEY.get(move||{
              if let std::option::Option::Some(init) = init {
                if let std::option::Option::Some(value) = init.take(){
                  return value;
                }else if true {
                  std::panicking::panic_fmt(::core::fmt::Arguments::new_v1(&["internal error: entered unreachable code: ",], &[::core::fmt::ArgumentV1::new(&(::core::fmt::Arguments::new_v1(&["missing default value",], &[])),::core::fmt::Display::fmt),]));
                }
              }__init()
            })
          }
        }
        unsafe {
          std::thread::LocalKey::new(__getit)
        }
      };
      ;
      let _ = leptos::leptos_dom::html::tr;
      let _ = leptos::leptos_dom::html::tr;
      let _ = leptos::leptos_dom::html::td;
      let _ = leptos::leptos_dom::html::td;
      let _ = leptos::leptos_dom::html::td;
      let _ = leptos::leptos_dom::html::td;
      let _ = leptos::leptos_dom::html::a;
      let _ = leptos::leptos_dom::html::a;
      let _ = leptos::leptos_dom::html::td;
      let _ = leptos::leptos_dom::html::td;
      let _ = leptos::leptos_dom::html::a;
      let _ = leptos::leptos_dom::html::a;
      let _ = leptos::leptos_dom::html::span;
      let _ = leptos::leptos_dom::html::span;
      let _ = leptos::leptos_dom::html::td;
      let root = TEMPLATE_ecd03fbf3f234416805d1420eb58bcee.with(|tpl|tpl.content().clone_node_with_deep(true)).unwrap().first_child().unwrap();
      let _el1 = "tr";
      let _el1 = leptos::wasm_bindgen::JsCast::unchecked_into:: <leptos::web_sys::Node>(root.clone());
      let _el2 = "td";
      let _el2 = _el1.first_child().unwrap_or_else(||std::panicking::panic_fmt(::core::fmt::Arguments::new_v1(&["error: "," => ",], &[::core::fmt::ArgumentV1::new(&("td"),::core::fmt::Display::fmt), ::core::fmt::ArgumentV1::new(&("firstChild"),::core::fmt::Display::fmt),])));
      let _el3 = _el2.first_child().unwrap_or_else(||std::panicking::panic_fmt(::core::fmt::Arguments::new_v1(&["error : "," => "," ",], &[::core::fmt::ArgumentV1::new(&("{block}"),::core::fmt::Display::fmt), ::core::fmt::ArgumentV1::new(&("firstChild"),::core::fmt::Display::fmt),])));
      let _el4 = "td";
      let _el4 = _el2.next_sibling().unwrap_or_else(||std::panicking::panic_fmt(::core::fmt::Arguments::new_v1(&["error : "," => "," ",], &[::core::fmt::ArgumentV1::new(&("td"),::core::fmt::Display::fmt), ::core::fmt::ArgumentV1::new(&("nextSibling"),::core::fmt::Display::fmt),])));
      let _el5 = "a";
      let _el5 = _el4.first_child().unwrap_or_else(||std::panicking::panic_fmt(::core::fmt::Arguments::new_v1(&["error: "," => ",], &[::core::fmt::ArgumentV1::new(&("a"),::core::fmt::Display::fmt), ::core::fmt::ArgumentV1::new(&("firstChild"),::core::fmt::Display::fmt),])));
      let _el6 = _el5.first_child().unwrap_or_else(||std::panicking::panic_fmt(::core::fmt::Arguments::new_v1(&["error : "," => "," ",], &[::core::fmt::ArgumentV1::new(&("{block}"),::core::fmt::Display::fmt), ::core::fmt::ArgumentV1::new(&("firstChild"),::core::fmt::Display::fmt),])));
      let _el7 = "td";
      let _el7 = _el4.next_sibling().unwrap_or_else(||std::panicking::panic_fmt(::core::fmt::Arguments::new_v1(&["error : "," => "," ",], &[::core::fmt::ArgumentV1::new(&("td"),::core::fmt::Display::fmt), ::core::fmt::ArgumentV1::new(&("nextSibling"),::core::fmt::Display::fmt),])));
      let _el8 = "a";
      let _el8 = _el7.first_child().unwrap_or_else(||std::panicking::panic_fmt(::core::fmt::Arguments::new_v1(&["error: "," => ",], &[::core::fmt::ArgumentV1::new(&("a"),::core::fmt::Display::fmt), ::core::fmt::ArgumentV1::new(&("firstChild"),::core::fmt::Display::fmt),])));
      let _el9 = "span";
      let _el9 = _el8.first_child().unwrap_or_else(||std::panicking::panic_fmt(::core::fmt::Arguments::new_v1(&["error: "," => ",], &[::core::fmt::ArgumentV1::new(&("span"),::core::fmt::Display::fmt), ::core::fmt::ArgumentV1::new(&("firstChild"),::core::fmt::Display::fmt),])));
      let _el10 = "td";
      let _el10 = _el7.next_sibling().unwrap_or_else(||std::panicking::panic_fmt(::core::fmt::Arguments::new_v1(&["error : "," => "," ",], &[::core::fmt::ArgumentV1::new(&("td"),::core::fmt::Display::fmt), ::core::fmt::ArgumentV1::new(&("nextSibling"),::core::fmt::Display::fmt),])));
      leptos::leptos_dom::class_helper(leptos::wasm_bindgen::JsCast::unchecked_ref(&_el1),"danger".into(),{
        move| |is_selected(Some(row_id))
      }.into_class(cx));
      leptos::leptos_dom::mount_child(leptos::leptos_dom::MountKind::Append(&_el2), &{
        {
          row_id.to_string()
        }
      }.into_view(cx));
      ;
      leptos::leptos_dom::add_event_helper(leptos::wasm_bindgen::JsCast::unchecked_ref(&_el5), ::leptos::leptos_dom::ev::click,move|_|set_selected(Some(row_id)));
      ;
      leptos::leptos_dom::mount_child(leptos::leptos_dom::MountKind::Append(&_el5), &{
        {
          move| |label.get()
        }
      }.into_view(cx));
      ;
      leptos::leptos_dom::add_event_helper(leptos::wasm_bindgen::JsCast::unchecked_ref(&_el8), ::leptos::leptos_dom::ev::click,move|_|remove(row_id));
      ;
      leptos::leptos_dom::View::Element(leptos::leptos_dom::Element {
        #[cfg(debug_assertions)]
        name:"tr".into(),element:leptos::wasm_bindgen::JsCast::unchecked_into(root), #[cfg(debug_assertions)]
        view_marker:None
      })
    }
  }).build())))
}))
})).child((cx,{
leptos::leptos_dom::html::span(cx).attr("class",(cx,"preloadicon glyphicon glyphicon-remove")).attr("aria-hidden",(cx,"true"))
})).with_view_marker("-0")
    }
}
