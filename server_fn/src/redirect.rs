use std::sync::OnceLock;

pub const REDIRECT_HEADER: &str = "serverfnredirect";

pub type RedirectHook = Box<dyn Fn(&str) + Send + Sync>;

// allowed: not in a public API, and pretty straightforward
#[allow(clippy::type_complexity)]
pub(crate) static REDIRECT_HOOK: OnceLock<RedirectHook> = OnceLock::new();

pub fn set_redirect_hook(
    hook: impl Fn(&str) + Send + Sync + 'static,
) -> Result<(), RedirectHook> {
    REDIRECT_HOOK.set(Box::new(hook))
}

pub fn call_redirect_hook(path: &str) {
    if let Some(hook) = REDIRECT_HOOK.get() {
        hook(path)
    }
}
