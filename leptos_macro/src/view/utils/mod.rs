//! view!-side utility helpers for ident/span manipulation.
//!
//! These helpers operate on `rstml` nodes and are used during view
//! macro expansion. In contrast, `crate::util` contains type-analysis
//! and companion-module generation logic shared by the `#[component]`
//! and `#[slot]` proc macros.

mod builder;
mod props;
mod span;

pub(crate) use builder::{
    extract_children_arg, generate_checked_builder_block,
};
pub use props::{filter_prefixed_attrs, is_nostrip_optional_and_update_key};
pub(crate) use props::{turbofish_generics, PropInfo};
pub(crate) use span::delinked_path_from_node_name;
pub use span::{children_span, key_value_span, prop_span_info};
