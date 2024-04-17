 This module contains the `Oco` (Owned Clones Once) smart pointer,
 which is used to store immutable references to values.
 This is useful for storing, for example, strings.

 Imagine this as an alternative to [`Cow`] with an additional, reference-counted
 branch.

 ```rust
 use oco_ref::Oco;
 use std::rc::Rc;

 let static_str = "foo";
 let rc_str: Rc<str> = "bar".into();
 let owned_str: String = "baz".into();

 fn uses_oco(value: impl Into<Oco<'static, str>>) {
     let mut value = value.into();

     // ensures that the value is either a reference, or reference-counted
     // O(n) at worst
     let clone1 = value.clone_inplace();

     // these subsequent clones are O(1)
     let clone2 = value.clone();
     let clone3 = value.clone();
 }

 uses_oco(static_str);
 uses_oco(rc_str);
 uses_oco(owned_str);
 ```
