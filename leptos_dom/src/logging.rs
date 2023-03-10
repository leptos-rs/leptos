use crate::is_server;
use cfg_if::cfg_if;
use wasm_bindgen::JsValue;

/// Uses `println!()`-style formatting to log something to the console (in the browser)
/// or via `println!()` (if not in the browser).
#[macro_export]
macro_rules! log {
    ($($t:tt)*) => ($crate::console_log(&format_args!($($t)*).to_string()))
}

/// Uses `println!()`-style formatting to log warnings to the console (in the browser)
/// or via `eprintln!()` (if not in the browser).
#[macro_export]
macro_rules! warn {
    ($($t:tt)*) => ($crate::console_warn(&format_args!($($t)*).to_string()))
}

/// Uses `println!()`-style formatting to log errors to the console (in the browser)
/// or via `eprintln!()` (if not in the browser).
#[macro_export]
macro_rules! error {
    ($($t:tt)*) => ($crate::console_error(&format_args!($($t)*).to_string()))
}

/// Uses `println!()`-style formatting to log warnings to the console (in the browser)
/// or via `eprintln!()` (if not in the browser), but only if it's a debug build.
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

/// Log a string to the console (in the browser)
/// or via `println!()` (if not in the browser).
pub fn console_log(s: &str) {
    if is_server() {
        println!("{s}");
    } else {
        web_sys::console::log_1(&JsValue::from_str(s));
    }
}

/// Log a warning to the console (in the browser)
/// or via `println!()` (if not in the browser).
pub fn console_warn(s: &str) {
    if is_server() {
        eprintln!("{s}");
    } else {
        web_sys::console::warn_1(&JsValue::from_str(s));
    }
}

/// Log an error to the console (in the browser)
/// or via `println!()` (if not in the browser).
pub fn console_error(s: &str) {
    if is_server() {
        eprintln!("{s}");
    } else {
        web_sys::console::error_1(&JsValue::from_str(s));
    }
}

/// Log an error to the console (in the browser)
/// or via `println!()` (if not in the browser), but only in a debug build.
pub fn console_debug_warn(s: &str) {
    cfg_if! {
        if #[cfg(debug_assertions)] {
            if is_server() {
                eprintln!("{s}");
            } else {
                web_sys::console::warn_1(&JsValue::from_str(s));
            }
        } else {
          let _ = s;
        }
    }
}
