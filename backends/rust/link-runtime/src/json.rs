// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! Writing a declared JSON document into a buffer the caller owns (SCE
//! Protocol-Synthesis RFC §synth-5-R, Queryables a machine serves).
//!
//! A `document:` reply in `machines.<m>.queryables` is a JSON object whose
//! fields, types and bounds the deploy states. The generated writer does one
//! thing with a value: it hands each piece of the document to a [`Sink`]. Two
//! sinks exist, and the generated code runs the same emission through both:
//!
//! - [`Counter`] measures the document, so the caller learns exactly how many
//!   bytes it needs, or that a value is outside the bound the deploy declared,
//!   before a byte is written;
//! - [`SliceSink`] writes into the caller's buffer, and the generated
//!   function only runs it after the measurement said the buffer is large
//!   enough, so a buffer that is too small is an error and never a document
//!   cut short.
//!
//! The output is compact JSON (no whitespace), with the fields in the order the
//! deploy lists them. A string is escaped as RFC 8259 requires and no more: the
//! quotation mark and the reverse solidus by a backslash, the control
//! characters below U+0020 by their short form or `\u00XX`, everything else as
//! the UTF-8 it already is. That is why a string of `n` bytes occupies at most
//! `6 * n + 2` bytes, the figure the deploy-time size bound uses.

/// Why a document could not be written.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WriteError {
    /// The buffer holds fewer bytes than the document needs. `needed` is the
    /// exact size of the document, or 0 when the sink could not say.
    BufferTooSmall { needed: usize },
    /// A string is longer than the `max_len` the deploy declared for `field`.
    StringTooLong { field: &'static str, max: usize },
    /// An array holds more elements than the `max_items` the deploy declared
    /// for `field`.
    TooManyItems { field: &'static str, max: usize },
}

impl core::fmt::Display for WriteError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            WriteError::BufferTooSmall { needed: 0 } => f.write_str("buffer too small"),
            WriteError::BufferTooSmall { needed } => {
                write!(f, "buffer too small: the document needs {needed} bytes")
            }
            WriteError::StringTooLong { field, max } => {
                write!(f, "`{field}` is longer than its declared max_len {max}")
            }
            WriteError::TooManyItems { field, max } => {
                write!(
                    f,
                    "`{field}` has more elements than its declared max_items {max}"
                )
            }
        }
    }
}

/// Where the pieces of a document go.
pub trait Sink {
    /// Take `bytes` as the next piece of the document.
    fn put(&mut self, bytes: &[u8]) -> Result<(), WriteError>;
}

/// A sink that only counts, so a document can be measured exactly.
#[derive(Debug, Default)]
pub struct Counter {
    len: usize,
}

impl Counter {
    pub const fn new() -> Self {
        Self { len: 0 }
    }

    /// The bytes counted so far.
    pub const fn len(&self) -> usize {
        self.len
    }

    pub const fn is_empty(&self) -> bool {
        self.len == 0
    }
}

impl Sink for Counter {
    fn put(&mut self, bytes: &[u8]) -> Result<(), WriteError> {
        self.len = self.len.saturating_add(bytes.len());
        Ok(())
    }
}

/// A sink over the caller's buffer.
#[derive(Debug)]
pub struct SliceSink<'a> {
    buf: &'a mut [u8],
    len: usize,
}

impl<'a> SliceSink<'a> {
    pub fn new(buf: &'a mut [u8]) -> Self {
        Self { buf, len: 0 }
    }

    /// The bytes written so far.
    pub const fn len(&self) -> usize {
        self.len
    }

    pub const fn is_empty(&self) -> bool {
        self.len == 0
    }
}

impl Sink for SliceSink<'_> {
    fn put(&mut self, bytes: &[u8]) -> Result<(), WriteError> {
        let end = self
            .len
            .checked_add(bytes.len())
            .filter(|end| *end <= self.buf.len())
            .ok_or(WriteError::BufferTooSmall { needed: 0 })?;
        self.buf[self.len..end].copy_from_slice(bytes);
        self.len = end;
        Ok(())
    }
}

/// Write `true` or `false`.
pub fn put_bool<S: Sink>(sink: &mut S, value: bool) -> Result<(), WriteError> {
    sink.put(if value { b"true" } else { b"false" })
}

/// Write `null`.
pub fn put_null<S: Sink>(sink: &mut S) -> Result<(), WriteError> {
    sink.put(b"null")
}

/// Write an unsigned integer in decimal.
pub fn put_u64<S: Sink>(sink: &mut S, mut value: u64) -> Result<(), WriteError> {
    // 20 digits hold u64::MAX.
    let mut digits = [0u8; 20];
    let mut at = digits.len();
    loop {
        at -= 1;
        digits[at] = b'0' + (value % 10) as u8;
        value /= 10;
        if value == 0 {
            break;
        }
    }
    sink.put(&digits[at..])
}

/// Write a signed integer in decimal.
pub fn put_i64<S: Sink>(sink: &mut S, value: i64) -> Result<(), WriteError> {
    if value < 0 {
        sink.put(b"-")?;
    }
    // `unsigned_abs` is exact for i64::MIN, where negation would overflow.
    put_u64(sink, value.unsigned_abs())
}

/// Write `value` as a JSON string, escaped.
pub fn put_str<S: Sink>(sink: &mut S, value: &str) -> Result<(), WriteError> {
    sink.put(b"\"")?;
    let bytes = value.as_bytes();
    let mut plain_from = 0;
    for (at, &byte) in bytes.iter().enumerate() {
        let escape: &[u8] = match byte {
            b'"' => b"\\\"",
            b'\\' => b"\\\\",
            b'\n' => b"\\n",
            b'\r' => b"\\r",
            b'\t' => b"\\t",
            0x08 => b"\\b",
            0x0c => b"\\f",
            0x00..=0x1f => {
                sink.put(&bytes[plain_from..at])?;
                plain_from = at + 1;
                const HEX: &[u8; 16] = b"0123456789abcdef";
                sink.put(&[
                    b'\\',
                    b'u',
                    b'0',
                    b'0',
                    HEX[(byte >> 4) as usize],
                    HEX[(byte & 0x0f) as usize],
                ])?;
                continue;
            }
            _ => continue,
        };
        sink.put(&bytes[plain_from..at])?;
        plain_from = at + 1;
        sink.put(escape)?;
    }
    sink.put(&bytes[plain_from..])?;
    sink.put(b"\"")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::string::String;
    use std::vec;

    fn written(f: impl Fn(&mut SliceSink<'_>) -> Result<(), WriteError>) -> String {
        let mut buf = vec![0u8; 256];
        let mut sink = SliceSink::new(&mut buf);
        f(&mut sink).expect("fits");
        let len = sink.len();
        String::from_utf8(buf[..len].to_vec()).expect("utf-8")
    }

    fn counted(f: impl Fn(&mut Counter) -> Result<(), WriteError>) -> usize {
        let mut counter = Counter::new();
        f(&mut counter).expect("counts");
        counter.len()
    }

    #[test]
    fn integers_are_decimal_at_both_ends_of_their_range() {
        assert_eq!(written(|s| put_u64(s, 0)), "0");
        assert_eq!(written(|s| put_u64(s, u64::MAX)), "18446744073709551615");
        assert_eq!(written(|s| put_i64(s, i64::MIN)), "-9223372036854775808");
        assert_eq!(written(|s| put_i64(s, i64::MAX)), "9223372036854775807");
        assert_eq!(written(|s| put_i64(s, -1)), "-1");
        assert_eq!(counted(|s| put_i64(s, i64::MIN)), 20);
        assert_eq!(counted(|s| put_u64(s, u64::MAX)), 20);
    }

    #[test]
    fn scalars_have_the_json_spellings() {
        assert_eq!(written(|s| put_bool(s, true)), "true");
        assert_eq!(written(|s| put_bool(s, false)), "false");
        assert_eq!(written(|s| put_null(s)), "null");
    }

    #[test]
    fn strings_are_escaped_as_rfc_8259_requires() {
        assert_eq!(
            written(|s| put_str(s, "udp/192.168.0.1:7447")),
            "\"udp/192.168.0.1:7447\""
        );
        assert_eq!(written(|s| put_str(s, "a\"b\\c")), "\"a\\\"b\\\\c\"");
        assert_eq!(written(|s| put_str(s, "l1\nl2\r\t")), "\"l1\\nl2\\r\\t\"");
        assert_eq!(written(|s| put_str(s, "\u{8}\u{c}")), "\"\\b\\f\"");
        assert_eq!(written(|s| put_str(s, "\u{1}\u{1f}")), "\"\\u0001\\u001f\"");
        assert_eq!(written(|s| put_str(s, "\u{7f}")), "\"\u{7f}\"");
        assert_eq!(written(|s| put_str(s, "h\u{e9}llo")), "\"h\u{e9}llo\"");
        assert_eq!(written(|s| put_str(s, "")), "\"\"");
    }

    #[test]
    fn a_string_of_n_bytes_never_exceeds_six_n_plus_two() {
        let worst = "\u{1}".repeat(11);
        assert_eq!(counted(|s| put_str(s, &worst)), 6 * 11 + 2);
    }

    #[test]
    fn a_full_buffer_is_an_error_and_a_short_one_is_too() {
        let mut buf = [0u8; 4];
        let mut sink = SliceSink::new(&mut buf);
        assert_eq!(
            put_str(&mut sink, "abcd"),
            Err(WriteError::BufferTooSmall { needed: 0 })
        );
        let mut buf = [0u8; 6];
        let mut sink = SliceSink::new(&mut buf);
        assert_eq!(put_str(&mut sink, "abcd"), Ok(()));
        assert_eq!(sink.len(), 6);
    }
}
