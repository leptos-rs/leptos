#![no_std]
#![forbid(unsafe_code)]
#![deny(missing_docs)]

//! Utilities for const concatenation of string slices.

pub(crate) const MAX_TEMPLATE_SIZE: usize = 4096;

/// Converts a zero-terminated buffer of bytes into a UTF-8 string.
pub const fn str_from_buffer(buf: &[u8; MAX_TEMPLATE_SIZE]) -> &str {
    match core::ffi::CStr::from_bytes_until_nul(buf) {
        Ok(cstr) => match cstr.to_str() {
            Ok(str) => str,
            Err(_) => panic!("TEMPLATE FAILURE"),
        },
        Err(_) => panic!("TEMPLATE FAILURE"),
    }
}

/// Concatenates any number of static strings into a single array.
// credit to Rainer Stropek, "Constant fun," Rust Linz, June 2022
pub const fn const_concat(
    strs: &'static [&'static str],
) -> [u8; MAX_TEMPLATE_SIZE] {
    let mut buffer = [0; MAX_TEMPLATE_SIZE];
    let mut position = 0;
    let mut remaining = strs;

    while let [current, tail @ ..] = remaining {
        let x = current.as_bytes();
        let mut i = 0;

        // have it iterate over bytes manually, because, again,
        // no mutable refernces in const fns
        while i < x.len() {
            buffer[position] = x[i];
            position += 1;
            i += 1;
        }

        remaining = tail;
    }

    buffer
}

/// Converts a zero-terminated buffer of bytes into a UTF-8 string with the given prefix.
pub const fn const_concat_with_prefix(
    strs: &'static [&'static str],
    prefix: &'static str,
    suffix: &'static str,
) -> [u8; MAX_TEMPLATE_SIZE] {
    let mut buffer = [0; MAX_TEMPLATE_SIZE];
    let mut position = 0;
    let mut remaining = strs;

    while let [current, tail @ ..] = remaining {
        let x = current.as_bytes();
        let mut i = 0;

        // have it iterate over bytes manually, because, again,
        // no mutable refernces in const fns
        while i < x.len() {
            buffer[position] = x[i];
            position += 1;
            i += 1;
        }

        remaining = tail;
    }

    if buffer[0] == 0 {
        buffer
    } else {
        let mut new_buf = [0; MAX_TEMPLATE_SIZE];
        let prefix = prefix.as_bytes();
        let suffix = suffix.as_bytes();
        let mut position = 0;
        let mut i = 0;
        while i < prefix.len() {
            new_buf[position] = prefix[i];
            position += 1;
            i += 1;
        }
        i = 0;
        while i < buffer.len() {
            if buffer[i] == 0 {
                break;
            }
            new_buf[position] = buffer[i];
            position += 1;
            i += 1;
        }
        i = 0;
        while i < suffix.len() {
            new_buf[position] = suffix[i];
            position += 1;
            i += 1;
        }

        new_buf
    }
}

/// Converts any number of strings into a UTF-8 string, separated by the given string.
pub const fn const_concat_with_separator(
    strs: &[&str],
    separator: &'static str,
) -> [u8; MAX_TEMPLATE_SIZE] {
    let mut buffer = [0; MAX_TEMPLATE_SIZE];
    let mut position = 0;
    let mut remaining = strs;

    while let [current, tail @ ..] = remaining {
        let x = current.as_bytes();
        let mut i = 0;

        // have it iterate over bytes manually, because, again,
        // no mutable refernces in const fns
        while i < x.len() {
            buffer[position] = x[i];
            position += 1;
            i += 1;
        }
        if !x.is_empty() {
            let mut position = 0;
            let separator = separator.as_bytes();
            while i < separator.len() {
                buffer[position] = separator[i];
                position += 1;
                i += 1;
            }
        }

        remaining = tail;
    }

    buffer
}
