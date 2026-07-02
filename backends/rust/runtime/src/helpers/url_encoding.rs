// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael

//! W3C SCXML C.2: URL encoding/decoding utilities.
//!
//! 1:1 port of `sce/include/common/UrlEncodingHelper.h`. Provides RFC 3986
//! percent-encoding for `application/x-www-form-urlencoded` format used by
//! the BasicHTTP Event I/O Processor.
//!
//! Watching-zenoh RFC §synth-5-J-2 (lines 1989-1994): whole-module gated to
//! `!no_std` because the only intended consumer is the BasicHTTP Event I/O
//! Processor, which is itself `!no_std`-gated (the
//! `codegen/no-std-http-not-supported` validator rejects HTTP `<send>`
//! up-front so URL encoding is unreachable from generated no_std code).
//! Mirrors the `event_data.rs` precedent (B-γ2d-1) for HTTP-coupled helpers
//! with no no_std consumer.

#![cfg(not(feature = "no_std"))]

/// W3C SCXML C.2: Percent-encode a string for URL transmission.
///
/// RFC 3986: Unreserved characters (`A-Za-z0-9-._~`) are not encoded.
/// All other characters are percent-encoded as `%XX`.
///
/// Ports C++ `UrlEncodingHelper::urlEncode`.
///
/// # Examples
///
/// ```
/// use sce_rust_runtime::helpers::url_encoding::url_encode;
///
/// assert_eq!(url_encode("hello world"), "hello%20world");
/// assert_eq!(url_encode("test@example.com"), "test%40example.com");
/// assert_eq!(url_encode("param1"), "param1");
/// ```
pub fn url_encode(input: &str) -> String {
    let mut encoded = String::with_capacity(input.len());

    for byte in input.bytes() {
        if is_unreserved(byte) {
            encoded.push(byte as char);
        } else {
            encoded.push('%');
            encoded.push(HEX_CHARS[(byte >> 4) as usize] as char);
            encoded.push(HEX_CHARS[(byte & 0x0F) as usize] as char);
        }
    }

    encoded
}

/// W3C SCXML C.2: Decode a percent-encoded URL string.
///
/// Reverses the encoding applied by [`url_encode`]. Invalid percent sequences
/// are passed through unchanged.
pub fn url_decode(input: &str) -> String {
    let mut decoded = String::with_capacity(input.len());
    let bytes = input.as_bytes();
    let mut i = 0;

    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            if let (Some(hi), Some(lo)) =
                (hex_digit_value(bytes[i + 1]), hex_digit_value(bytes[i + 2]))
            {
                decoded.push((hi << 4 | lo) as char);
                i += 3;
                continue;
            }
        }
        decoded.push(bytes[i] as char);
        i += 1;
    }

    decoded
}

/// RFC 3986: Check if a byte is an unreserved character.
///
/// Unreserved = ALPHA / DIGIT / "-" / "." / "_" / "~"
#[inline]
fn is_unreserved(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || byte == b'-' || byte == b'.' || byte == b'_' || byte == b'~'
}

/// Parse a hex digit character to its numeric value.
fn hex_digit_value(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'A'..=b'F' => Some(byte - b'A' + 10),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        _ => None,
    }
}

/// Hex character lookup table.
const HEX_CHARS: &[u8; 16] = b"0123456789ABCDEF";
