//!
//! This module contains macros that generate code that can be used when turning server only generic server functions into isomorphic generic functions.
//! The way it works is that we produce phantom types and empty trait constraints that shadow server only types and traits and within the server function macro
//! it connects the shims.
//!
//! for example
//!
//! ```rust, ignore
//! 
//! // Supose we have two server only types `BackendType` and `MockBackendTrait`
//! // We can generate a "phantom" type for each of our server only types.
//! ssr_type_shim!(BackendType,MockBackendTrait);
//!
//! // Suppose we had a server only trait `BackendTrait`
//! // We can generate empty "constraint" traits for our server only traits;
//! ssr_trait_shim!(BackendTrait);
//!
//! // And suppose that our backend types implemented our backend trait.
//! // We can implement our constraint traits over our phantom types.
//! ssr_impl_shim!(BackendType:BackendTrait,MockBackendTrait:BackendTrait);
//!
//! // Now we're able to write a component for our frontend that is generic over it's backend implementation.
//! #[component]
//! pub fn MyComponent<T:BackendTraitConstraint>() -> impl IntoView {
//!     Suspense::new(async move {
//!         get_data::<T>().await.unwrap().into_view()
//!     })
//! }
//!
//! // We register every different type configurations that we want access to.
//! // This is because each specific monomorphized function needs a seperate route.
//! #[server]
//! #[register(<BackendTypePhantom>,<MockBackendTrait>)]
//! pub async fn get_data<T:BackendTrait>() -> Result<Data,ServerFnError> {
//!     // Suppose there existed a BackendTrait which implemented this method.
//!     Ok(T::my_data().await?)
//! }
//!
//! // Now we can create two frontend functions that each elicit different backend behavior based on their type.
//! #[component]
//! pub fn MyComponentParent() -> impl IntoView {
//!     view!{
//!         <MyComponent<BackendTypePhantom>>
//!         <MyComponent<MockBackendTraitPhantom>>
//!     }
//! }
//! ```

#[doc(hidden)]
pub trait ServerType {
    type ServerType;
}
/// Generates a new struct $type_namePhantom for a list of identifiers.
///
/// ```rust,ignore
/// ssr_type_shim!(SpecificType);
///
/// fn main() {
///     let _ = SpecificTypePhantom{};
/// }
/// ```
///
/// It also implements a hidden trait ServerType under an ssr feature flag whose associated type is the original server only type.
macro_rules! ssr_type_shim{
    ($($type_name:ident),*) => {
        $(
            paste::paste!{
                /// An isomorphic marker type for $type_name
                pub struct  [<$type_name Phantom>];
             }
        )*
        $(
            #[cfg(feature="ssr")]
            paste::paste! { impl ServerType for [<$type_name Phantom>] {
                type ServerType = $type_name;
            }
            }
        )*
    }
}

/// Generates new empty traits $trait_nameConstraint for a list of identifiers.
///
/// /// ```rust,ignore
/// ssr_type_shim!(SpecificTrait);
///
/// // Will generate code
/// // pub trait SpecificTraitConstraint{}
/// ```
///
macro_rules! ssr_trait_shim{
    ($($trait_name:ident),*) => {
        $(
            paste::paste! {
                /// An empty isomorphic trait to mirror $trait_name
                pub trait [<$trait_name Constraint>]  {}
            }
        )*
    }
}

/// Takes type names and trait names for the traits they need implemented and implements the "constraint" traits for the "phantom" versions.
///
/// ```rust,ignore
///     // uses traditional + syntax for additonal traits past 1 like in normal trait bounds.
///     ssr_impl_shim!(BackendType:BackendTrait, BackendType2:BackendTrait + BackendTrait2);
/// ```
macro_rules! ssr_impl_shim{
     ($($type_name:ident : $trait_name:ident $(+ $trait_name_tail:ident)*),*) => {
        $(
           paste:: paste! { impl [<$trait_name Constraint>] for [<$type_name Phantom>] {} }
            $(
               paste:: paste! { impl [<$trait_name_tail Constraint>] for [<$type_name Phantom>] {} }
            )*
        )*
    }
}

#[doc(hidden)]
#[cfg(test)]
pub mod tests {
    use super::*;

    #[test]
    fn server_fn_shims_generate_code_correctly() {
        pub struct BackendType;
        pub trait BackendTrait {}
        impl BackendTrait for BackendType {}
        ssr_type_shim!(BackendType);
        ssr_trait_shim!(BackendTrait);
        ssr_impl_shim!(BackendType:BackendTrait);

        pub fn generic_fn<T: BackendTraitConstraint + ServerType>()
        where
            <T as ServerType>::ServerType: BackendTrait,
        {
        }
        generic_fn::<BackendTypePhantom>();
        // If this compiles it passes.
        assert!(true);
    }
}
