use debugger::SimpleCounter;
use leptos::*;

pub fn main() {
    _ = console_log::init_with_level(log::Level::Debug);
    console_error_panic_hook::set_once();
    leptos_debugger::set_debugger_hook(Devtools::default());
    mount_to_body(|cx| {
        view! { cx,
            <SimpleCounter
                initial_value=0
                step=1
            />
        }
    })
}

#[derive(Default)]
struct Devtools {
    debugger_config: Option<leptos_debugger::HookConfig>,
}

impl leptos_debugger::Hook for Devtools {
    fn set_config(&mut self, config: leptos_debugger::HookConfig) {
        self.debugger_config = Some(config);
    }

    fn create_root(&mut self) {
        if let Some(debugger_config) = &self.debugger_config {
            log!("create-root {:#?}", (debugger_config.get_root_tree)());
        }
    }

    fn update_view(&mut self) {
        if let Some(debugger_config) = &self.debugger_config {
            log!("update-view {:#?}", (debugger_config.get_root_tree)());
        }
    }
}