use std::sync::OnceLock;

static REDIRECT_HOOK: OnceLock<Box<dyn Fn(&str) + Send + Sync>> =
    OnceLock::new();

pub fn set_redirect_hook(hook: impl Fn(&str) + Send + Sync + 'static) {
    REDIRECT_HOOK.set(Box::new(hook));
}

pub fn call_redirect_hook(path: &str) {
    if let Some(hook) = REDIRECT_HOOK.get() {
        hook(path)
    }
}
