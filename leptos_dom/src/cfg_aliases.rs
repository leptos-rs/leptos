attribute_alias! {
  #[apply(web)] = #[cfg(all(target_arch = "wasm32", feature = "web"))];

  #[apply(not_web)] = #[cfg(not(all(target_arch = "wasm32", feature = "web")))];
}
