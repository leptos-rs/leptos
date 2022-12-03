use std::cell::OnceCell;

use leptos_reactive::Scope;
use cfg_if::cfg_if;
use crate::{IntoElement, IntoNode, HtmlElement, text, Unit};
//#[cfg(all(target = "wasm32", feature = "web"))]
use crate::{MountKind, mount_child};

mod into_child;
use into_child::IntoChild;

impl<El: IntoElement> HtmlElement<El> {
	#[doc(hidden)]
	#[track_caller]
	pub fn _child<C: IntoChild>(mut self, cx: Scope, child: C) -> Self {
		let child = child.into_child(cx);
		cfg_if! {
		  if #[cfg(all(target_arch = "wasm32", feature = "web"))] {
			mount_child(MountKind::Append(self.element.get_element()), &child.into_node(cx))
		  }
		  else {
			self.children.push(Box::new(move |cx| child.into_node(cx)));
		  }
		}
	
		self
	  }
}