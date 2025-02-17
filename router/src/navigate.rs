use crate::location::State;

/// Options that can be used to configure a navigation. Used with [use_navigate](crate::hooks::use_navigate).
#[derive(Clone, Debug)]
pub struct NavigateOptions {
    /// Whether the URL being navigated to should be resolved relative to the current route.
    pub resolve: bool,
    /// If `true` the new location will replace the current route in the history stack, meaning
    /// the "back" button will skip over the current route. (Defaults to `false`).
    pub replace: bool,
    /// If `true`, the router will scroll to the top of the window at the end of navigation.
    /// Defaults to `true`.
    pub scroll: bool,
    /// [State](https://developer.mozilla.org/en-US/docs/Web/API/History/state) that should be pushed
    /// onto the history stack during navigation.
    pub state: State,
}

impl Default for NavigateOptions {
    fn default() -> Self {
        Self {
            resolve: true,
            replace: false,
            scroll: true,
            state: State::new(None),
        }
    }
}
