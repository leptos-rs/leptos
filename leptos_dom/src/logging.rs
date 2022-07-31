use wasm_bindgen::JsValue;

use crate::is_server;

#[macro_export]
macro_rules! log {
    ($($t:tt)*) => ($crate::console_log(&format_args!($($t)*).to_string()))
}

#[macro_export]
macro_rules! warn {
    ($($t:tt)*) => ($crate::console_warn(&format_args!($($t)*).to_string()))
}

#[macro_export]
macro_rules! error {
    ($($t:tt)*) => ($crate::console_error(&format_args!($($t)*).to_string()))
}

#[macro_export]
macro_rules! debug_warn {
    ($($x:tt)*) => {
        {
            #[cfg(debug_assertions)]
            {
                $crate::warn!($($x)*)
            }
            #[cfg(not(debug_assertions))]
            {
                ($($x)*)
            }
        }
    }
}

pub fn console_log(s: &str) {
    if is_server!() {
        println!("{}", s);
    } else {
        web_sys::console::log_1(&JsValue::from_str(s));
    }
}

pub fn console_warn(s: &str) {
    if is_server!() {
        eprintln!("{}", s);
    } else {
        web_sys::console::warn_1(&JsValue::from_str(s));
    }
}

pub fn console_error(s: &str) {
    if is_server!() {
        eprintln!("{}", s);
    } else {
        web_sys::console::warn_1(&JsValue::from_str(s));
    }
}

#[cfg(debug_assertions)]
pub fn console_debug_warn(s: &str) {
    if is_server!() {
        eprintln!("{}", s);
    } else {
        web_sys::console::warn_1(&JsValue::from_str(s));
    }
}

#[cfg(not(debug_assertions))]
pub fn console_debug_warn(s: &str) {}
