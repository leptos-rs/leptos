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
        // no mutable references in const fns
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
        // no mutable references in const fns
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

/// Concatenates any number of strings into a buffer, inserting `separator`
/// between every pair of non-empty inputs.
///
/// Empty inputs are skipped, so no leading, trailing, or double separators
/// are ever produced. The buffer is zero-padded after the last written byte.
pub const fn const_concat_with_separator(
    strs: &[&str],
    separator: &'static str,
) -> [u8; MAX_TEMPLATE_SIZE] {
    let mut buffer = [0; MAX_TEMPLATE_SIZE];
    let mut position = 0;
    let mut remaining = strs;
    let separator = separator.as_bytes();
    let mut wrote_any = false;

    while let [current, tail @ ..] = remaining {
        let x = current.as_bytes();
        if !x.is_empty() {
            if wrote_any {
                let mut j = 0;
                while j < separator.len() {
                    buffer[position] = separator[j];
                    position += 1;
                    j += 1;
                }
            }
            let mut i = 0;
            while i < x.len() {
                buffer[position] = x[i];
                position += 1;
                i += 1;
            }
            wrote_any = true;
        }

        remaining = tail;
    }

    buffer
}

#[cfg(test)]
extern crate std;

#[cfg(test)]
mod tests {
    use super::*;

    fn as_str(buf: &[u8; MAX_TEMPLATE_SIZE]) -> &str {
        let end = buf.iter().position(|&b| b == 0).unwrap_or(buf.len());
        core::str::from_utf8(&buf[..end]).unwrap()
    }

    #[test]
    fn separator_joins_two_strings() {
        const PARTS: &[&str] = &["foo", "bar"];
        let out = const_concat_with_separator(PARTS, ", ");
        assert_eq!(as_str(&out), "foo, bar");
    }

    #[test]
    fn separator_with_three_strings() {
        const PARTS: &[&str] = &["a", "b", "c"];
        let out = const_concat_with_separator(PARTS, " | ");
        assert_eq!(as_str(&out), "a | b | c");
    }

    #[test]
    fn separator_does_not_corrupt_buffer_when_separator_longer_than_input() {
        // Regression: a previous implementation shadowed the write cursor and
        // wrote separator bytes back to offset 0, destroying earlier output.
        const PARTS: &[&str] = &["a"];
        let out = const_concat_with_separator(PARTS, "xxxxx");
        assert_eq!(as_str(&out), "a");
    }

    #[test]
    fn separator_no_trailing_separator() {
        const PARTS: &[&str] = &["foo"];
        let out = const_concat_with_separator(PARTS, ";");
        assert_eq!(as_str(&out), "foo");
    }

    #[test]
    fn separator_skips_empty_inputs() {
        const PARTS: &[&str] = &["", "foo", "", "bar", ""];
        let out = const_concat_with_separator(PARTS, " ");
        assert_eq!(as_str(&out), "foo bar");
    }

    #[test]
    fn separator_on_empty_slice_is_empty() {
        let out = const_concat_with_separator(&[], ", ");
        assert_eq!(as_str(&out), "");
    }

    #[test]
    fn separator_evaluable_in_const_context() {
        const OUT: [u8; MAX_TEMPLATE_SIZE] =
            const_concat_with_separator(&["foo", "bar"], "-");
        const OUT_STR: &str = str_from_buffer(&OUT);
        assert_eq!(OUT_STR, "foo-bar");
    }
}
