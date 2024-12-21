#![forbid(unsafe_code)]
#![deny(missing_docs)]

//! This crate contains the default implementation of the #[macro@crate::server] macro without additional context from the server.
//! See the [server_fn_macro] crate for more information.

use proc_macro::TokenStream;
use server_fn_macro::server_macro_impl;
use syn::__private::ToTokens;

/// Declares that a function is a [server function](https://docs.rs/server_fn/).
/// This means that its body will only run on the server, i.e., when the `ssr`
/// feature is enabled on this crate.
///
/// ## Usage
/// ```rust,ignore
/// #[server]
/// pub async fn blog_posts(
///     category: String,
/// ) -> Result<Vec<BlogPost>, ServerFnError> {
///     let posts = load_posts(&category).await?;
///     // maybe do some other work
///     Ok(posts)
/// }
/// ```
///
/// ## Named Arguments
///
/// You can any combination of the following named arguments:
/// - `name`: sets the identifier for the server function’s type, which is a struct created
///    to hold the arguments (defaults to the function identifier in PascalCase)
/// - `prefix`: a prefix at which the server function handler will be mounted (defaults to `/api`)
/// - `endpoint`: specifies the exact path at which the server function handler will be mounted,
///   relative to the prefix (defaults to the function name followed by unique hash)
/// - `input`: the encoding for the arguments (defaults to `PostUrl`)
/// - `input_derive`: a list of derives to be added on the generated input struct (defaults to `(Clone, serde::Serialize, serde::Deserialize)` if `input` is set to a custom struct, won't have an effect otherwise)
/// - `output`: the encoding for the response (defaults to `Json`)
/// - `client`: a custom `Client` implementation that will be used for this server fn
/// - `encoding`: (legacy, may be deprecated in future) specifies the encoding, which may be one
///   of the following (not case sensitive)
///     - `"Url"`: `POST` request with URL-encoded arguments and JSON response
///     - `"GetUrl"`: `GET` request with URL-encoded arguments and JSON response
///     - `"Cbor"`: `POST` request with CBOR-encoded arguments and response
///     - `"GetCbor"`: `GET` request with URL-encoded arguments and CBOR response
/// - `req` and `res` specify the HTTP request and response types to be used on the server (these
///   should usually only be necessary if you are integrating with a server other than Actix/Axum)
/// ```rust,ignore
/// #[server(
///   name = SomeStructName,
///   prefix = "/my_api",
///   endpoint = "my_fn",
///   input = Cbor,
///   output = Json
/// )]
/// pub async fn my_wacky_server_fn(input: Vec<String>) -> Result<usize, ServerFnError> {
///   todo!()
/// }
///
/// // expands to
/// #[derive(Deserialize, Serialize)]
/// struct SomeStructName {
///   input: Vec<String>
/// }
///
/// impl ServerFn for SomeStructName {
///   const PATH: &'static str = "/my_api/my_fn";
///
///   // etc.
/// }
/// ```
///
/// ## Generic Server Functions
/// You can make your server function generic by writing the function generically and adding a register attribute which will works like (and overrides) the endpoint attribute.
/// but for specific identities of your generic server function.
///
/// When a generic type is not found in the inputs and is instead only found in the return type or the body the server function struct will include a `PhantomData<T>`
/// Where T is the not found type. Or in the case of multiple not found types T,...,Tn will include them in a tuple. i.e `PhantomData<(T,...,Tn)>`
///
/// ```rust, ignore
/// #[server]
/// #[register(
///     <SpecificT,DefaultU>,
///     <OtherSpecificT,NotDefaultU>="other_struct_has_specific_route"
/// )]
/// pub async fn my_generic_server_fn<T : SomeTrait, U = DefaultU>(input:T) -> Result<(), ServerFnError>
///     where
///     U: ThisTraitIsInAWhereClause
///  {
///     todo!()
/// }
///
/// // expands to
/// #[derive(Deserialize, Serialize)]
/// struct MyGenericServerFn<T,U>
///     where
///        // we require these traits always for generic fn input
///        T : SomeTrait + Send + Serialize + DeserializeOwned + 'static,
///        U : ThisTraitIsInAWhereClause  {
///     _marker:PhantomData<U>
///     input: T
/// }
///
/// impl ServerFn for MyGenericServerFn<SpecificT,DefaultU> {
///  // where our endpoint will be generated for us and unique to this type
///   const PATH: &'static str = "/api/...generated_endpoint...";
///     // ...
/// }
///
/// impl ServerFn for MyGenericServerFn<OtherSpecificT,NotDefaultU> {
///   const PATH: &'static str = "/api/other_struct_has_specific_route";
///     // ..
/// }
/// ```
///
/// If your server function is generic over types that are not isomorphic, i.e a backend type or a database connection. You can use the `generic_fn`
/// module helper shims to create
/// the traits types and impls that the server macro will use to map the client side code onto the backend.
///
/// You can find more details about the macros in their respective `generic_fn` module.
///
/// ```rust,ignore
/// ssr_type_shim!(BackendType);
/// // generates
/// pub struct BackendTypePhantom;
/// #[cfg(feature="ssr")]
/// impl ServerType for BackendTypePhantom{
///     type ServerType = BackendType;
/// }
/// ssr_trait_shim!(BackendTrait);
/// // generates
/// pub trait BackendTraitConstraint{}
/// ssr_impl_shim!(BackendType:BackendTrait);
/// // generates
/// impl BackendTypeConstraint for BackendTypePhantom{}
///
/// // see below how we are now registered with the phantom struct and not the original struct,
/// // the server macro will "move through" the phantom struct via it's ServerType implemented above to find the server type and pass that to your server function.
/// // We do this for any specified struct, in a register attribute, with no generic parameters that ends in the word Phantom. i.e Type1Phantom, DbPhantom, Phantom, PhantomPhantom, etc.
/// #[server]
/// #[register(<BackendTypePhantom,DefaultU>)]
/// pub async fn generic_fn<T:BackendTrait,U = DefaultU>() -> Result<U,ServerFnError> {
///     todo!()
/// }
///
/// // expands to
/// #[derive(Deserialize, Serialize)]
/// struct GenericFc<T,U>
///     where
///        T : BackendTraitConstraint, {
///     _marker:PhantomData<(T,U)>
/// }
///
/// impl ServerFn for GenericFn<BackendTypePhantom,DefaultU> {
///     // same as above...
/// }
/// ```
///
/// And can be referenced in your frontend code a `T:BackendTraitConstraint`.
#[proc_macro_attribute]
pub fn server(args: proc_macro::TokenStream, s: TokenStream) -> TokenStream {
    match server_macro_impl(
        args.into(),
        s.into(),
        Some(syn::parse_quote!(server_fns)),
        "/api",
        None,
        None,
    ) {
        Err(e) => e.to_compile_error().into(),
        Ok(s) => s.to_token_stream().into(),
    }
}
