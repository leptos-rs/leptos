use erased::ErasedBox;

#[cfg(not(erase_components))]
fn check(id_1: &std::any::TypeId, id_2: &std::any::TypeId) {
    if id_1 != id_2 {
        panic!("Erased: type mismatch")
    }
}

macro_rules! erased {
    ([$($new_t_params:tt)*], $name:ident) => {
        /// A type-erased item. This is slightly more efficient than using `Box<dyn Any (+ Send)>`.
        ///
        /// With the caveat that T must always be correct upon retrieval.
        /// In erased mode T retrieval is unchecked to minimise codegen, in other modes T will be verified and a panic otherwise.
        pub struct $name {
            #[cfg(not(erase_components))]
            type_id: std::any::TypeId,
            value: Option<ErasedBox>,
            drop: fn(ErasedBox),
        }


        impl $name {
            /// Create a new type-erased item.
            pub fn new<T: $($new_t_params)*>(item: T) -> Self {
                Self {
                    #[cfg(not(erase_components))]
                    type_id: std::any::TypeId::of::<T>(),
                    value: Some(ErasedBox::new(Box::new(item))),
                    drop: |value| {
                        let _ = unsafe { value.into_inner::<T>() };
                    },
                }
            }

            /// Get a reference to the inner value.
            pub fn get_ref<T: 'static>(&self) -> &T {
                #[cfg(not(erase_components))]
                check(&self.type_id, &std::any::TypeId::of::<T>());
                unsafe { self.value.as_ref().unwrap().get_ref::<T>() }
            }

            /// Get a mutable reference to the inner value.
            pub fn get_mut<T: 'static>(&mut self) -> &mut T {
                #[cfg(not(erase_components))]
                check(&self.type_id, &std::any::TypeId::of::<T>());
                unsafe { self.value.as_mut().unwrap().get_mut::<T>() }
            }

            /// Consume the item and return the inner value.
            pub fn into_inner<T: 'static>(mut self) -> T {
                #[cfg(not(erase_components))]
                check(&self.type_id, &std::any::TypeId::of::<T>());
                *unsafe { self.value.take().unwrap().into_inner::<T>() }
            }
        }

        /// If into_inner() wasn't called, the value would leak and destructors wouldn't run, this prevents that from happening.
        impl Drop for $name {
            fn drop(&mut self) {
                if let Some(value) = self.value.take() {
                    (self.drop)(value);
                }
            }
        }
    };

}

erased!([Send + 'static], Erased);
erased!(['static], ErasedLocal);

/// SAFETY: `Erased::new` ensures that `T` is `Send` and `'static`.
unsafe impl Send for Erased {}
