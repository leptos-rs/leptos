#![no_std]
#![forbid(unsafe_code)]
#![deny(missing_docs)]

//! Utilities for const concatenation of string slices.

pub(crate) const MAX_TEMPLATE_SIZE: usize = 4096;

/// Converts a zero-terminated buffer of bytes into a UTF-8 string.
///
/// The returned slice ends at the first `0` byte, so inputs containing an
/// interior `\0` are truncated at that point. Callers that need to preserve
/// embedded nul bytes must use a different framing strategy.
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

/// Concatenates `strs` and, if any byte was written, wraps the result with
/// `prefix` and `suffix`.
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

    if position == 0 {
        buffer
    } else {
        let body_end = position;
        let mut new_buf = [0; MAX_TEMPLATE_SIZE];
        let prefix = prefix.as_bytes();
        let suffix = suffix.as_bytes();
        let mut new_pos = 0;
        let mut i = 0;
        while i < prefix.len() {
            new_buf[new_pos] = prefix[i];
            new_pos += 1;
            i += 1;
        }
        i = 0;
        while i < body_end {
            new_buf[new_pos] = buffer[i];
            new_pos += 1;
            i += 1;
        }
        i = 0;
        while i < suffix.len() {
            new_buf[new_pos] = suffix[i];
            new_pos += 1;
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

    #[test]
    fn prefix_preserves_embedded_nul_in_body() {
        const PARTS: &[&str] = &["foo\0bar", "baz"];
        let out = const_concat_with_prefix(PARTS, "<", ">");

        let expected: &[u8] = b"<foo\0barbaz>";
        assert_eq!(&out[..expected.len()], expected);
        assert_eq!(out[expected.len()], 0);
    }

    #[test]
    fn prefix_wraps_body_and_zero_pads() {
        const PARTS: &[&str] = &["hello"];
        let out = const_concat_with_prefix(PARTS, "(", ")");

        let expected: &[u8] = b"(hello)";
        assert_eq!(&out[..expected.len()], expected);
        assert_eq!(out[expected.len()], 0);
        assert!(out[expected.len()..].iter().all(|&b| b == 0));
    }

    #[test]
    fn prefix_evaluable_in_const_context_with_nul() {
        const OUT: [u8; MAX_TEMPLATE_SIZE] =
            const_concat_with_prefix(&["a\0b"], "[", "]");
        let expected: &[u8] = b"[a\0b]";
        assert_eq!(&OUT[..expected.len()], expected);
    }
}
