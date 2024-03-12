//! Utilities for simple isomorphic logging to the console or terminal.

use wasm_bindgen::JsValue;

/// Uses `println!()`-style formatting to log something to the console (in the browser)
/// or via `println!()` (if not in the browser).
#[macro_export]
macro_rules! log {
    ($($t:tt)*) => ($crate::logging::console_log(&format_args!($($t)*).to_string()))
}

/// Uses `println!()`-style formatting to log warnings to the console (in the browser)
/// or via `eprintln!()` (if not in the browser).
#[macro_export]
macro_rules! warn {
    ($($t:tt)*) => ($crate::logging::console_warn(&format_args!($($t)*).to_string()))
}

/// Uses `println!()`-style formatting to log errors to the console (in the browser)
/// or via `eprintln!()` (if not in the browser).
#[macro_export]
macro_rules! error {
    ($($t:tt)*) => ($crate::logging::console_error(&format_args!($($t)*).to_string()))
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

const fn log_to_stdout() -> bool {
    cfg!(not(all(
        target_arch = "wasm32",
        not(any(target_os = "emscripten", target_os = "wasi"))
    )))
}

/// Log a string to the console (in the browser)
/// or via `println!()` (if not in the browser).
pub fn console_log(s: &str) {
    #[allow(clippy::print_stdout)]
    if log_to_stdout() {
        println!("{s}");
    } else {
        web_sys::console::log_1(&JsValue::from_str(s));
    }
}

/// Log a warning to the console (in the browser)
/// or via `println!()` (if not in the browser).
pub fn console_warn(s: &str) {
    if log_to_stdout() {
        eprintln!("{s}");
    } else {
        web_sys::console::warn_1(&JsValue::from_str(s));
    }
}

/// Log an error to the console (in the browser)
/// or via `println!()` (if not in the browser).
#[inline(always)]
pub fn console_error(s: &str) {
    if log_to_stdout() {
        eprintln!("{s}");
    } else {
        web_sys::console::error_1(&JsValue::from_str(s));
    }
}

/// Log an error to the console (in the browser)
/// or via `println!()` (if not in the browser), but only in a debug build.
#[inline(always)]
pub fn console_debug_warn(s: &str) {
    #[cfg(debug_assertions)]
    {
        if log_to_stdout() {
            eprintln!("{s}");
        } else {
            web_sys::console::warn_1(&JsValue::from_str(s));
        }
    }

    #[cfg(not(debug_assertions))]
    {
        let _ = s;
    }
}


