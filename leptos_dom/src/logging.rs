use wasm_bindgen::JsValue;

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

/// Uses `println!()`-style formatting to log warnings to the console (in the browser)
/// or via `eprintln!()` (if not in the browser).
#[macro_export]
macro_rules! warn {
    ($($t:tt)*) => ($crate::console_warn(&format_args!($($t)*).to_string()))
}

/// Logs a string to the console (in the browser) or via `eprintln!()` (if not in the browser).
pub fn console_warn(s: &str) {
    if cfg!(any(feature = "csr", feature = "hydrate")) {
        web_sys::console::warn_1(&JsValue::from_str(s));
    } else {
        eprintln!("{}", s);
    }
}
