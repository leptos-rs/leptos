#![forbid(unsafe_code)]
use cfg_if::cfg_if;
use std::future::Future;

/// Spawns and runs a thread-local [`Future`] in a platform-independent way.
///
/// This can be used to interface with any `async` code by spawning a task
/// to run a `Future`.
///
/// ## Limitations
///
/// You should not use `spawn_local` to synchronize `async` code with a
/// signal’s value during server rendering. The server response will not
/// be notified to wait for the spawned task to complete, creating a race
/// condition between the response and your task. Instead, use
/// [`create_resource`](crate::create_resource) and `<Suspense/>` to coordinate
/// asynchronous work with the rendering process.
///
/// ```
/// # use leptos::*;
/// # #[cfg(not(any(feature = "csr", feature = "serde-lite", feature = "miniserde", feature = "rkyv")))]
/// # {
///
/// async fn get_user(user: String) -> Result<String, ServerFnError> {
///     Ok(format!("this user is {user}"))
/// }
///
/// // ❌ Write into a signal from `spawn_local` on the serevr
/// #[component]
/// fn UserBad(cx: Scope) -> impl IntoView {
///     let signal = create_rw_signal(cx, String::new());
///
///     // ❌ If the rest of the response is already complete,
///     //    `signal` will no longer exist when `get_user` resolves
///     #[cfg(feature = "ssr")]
///     spawn_local(async move {
///         let user_res = get_user("user".into()).await.unwrap_or_default();
///         signal.set(user_res);
///     });
///
///     view!{cx,
///         <p>
///             "This will be empty (hopefully the client will render it) -> "
///             {move || signal.get()}
///         </p>
///     }
/// }
///
/// // ✅ Use a resource and suspense
/// #[component]
/// fn UserGood(cx: Scope) -> impl IntoView {
///     // new resource with no dependencies (it will only called once)
///     let user = create_resource(cx, || (), |_| async { get_user("john".into()).await });
///     view!{cx,
///         // handles the loading
///         <Suspense fallback=move || view! {cx, <p>"Loading User"</p> }>
///             // handles the error from the resource
///             <ErrorBoundary fallback=|cx, _| {view! {cx, <p>"Something went wrong"</p>}}>
///                 {move || {
///                     user.read(cx).map(move |x| {
///                         // the resource has a result
///                         x.map(move |y| {
///                             // successful call from the server fn
///                             view!{cx, <p>"User result filled in server and client: "{y}</p>}
///                         })
///                     })
///                 }}
///             </ErrorBoundary>
///         </Suspense>
///     }
/// }
/// # }
/// ```
pub fn spawn_local<F>(fut: F)
where
    F: Future<Output = ()> + 'static,
{
    cfg_if! {
        if #[cfg(target_arch = "wasm32")] {
            wasm_bindgen_futures::spawn_local(fut)
        }
        else if #[cfg(any(test, doctest))] {
            tokio_test::block_on(fut);
        } else if #[cfg(feature = "ssr")] {
            tokio::task::spawn_local(fut);
        }  else {
            futures::executor::block_on(fut)
        }
    }
}
