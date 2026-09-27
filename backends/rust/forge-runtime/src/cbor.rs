// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! The CBOR (RFC 8949) items a `sce:encoding="cbor"` codec reads and writes
//! (SCE_FORGE.md §4.6.1): one definite-length map whose keys are small
//! unsigned integers and whose values are unsigned integers, booleans, text
//! strings and byte strings.
//!
//! A generated codec calls these; it spells no CBOR of its own. Written here
//! once, beside [`SceCursor`] and [`SceSink`], rather than per codec — the
//! same place the Zenoh variable-length integer lives — and heap-free, so a
//! `no_std` build carries it.
//!
//! **Writing** is deterministic (RFC 8949 §4.2.1): every head in its
//! shortest form, so two backends given the same value write the same bytes.
//! **Reading** takes a head in any valid length — what it reads is the value
//! — and refuses what the codec cannot read as its entry: a reserved
//! additional-information value, an indefinite length, another major type.

use crate::codec::{CodecError, SceCursor, SceSink};

/// Major type 0, an unsigned integer.
pub const MAJOR_UNSIGNED: u8 = 0;
/// Major type 2, a byte string.
pub const MAJOR_BYTES: u8 = 2;
/// Major type 3, a UTF-8 text string.
pub const MAJOR_TEXT: u8 = 3;
/// Major type 5, a map.
pub const MAJOR_MAP: u8 = 5;
/// Major type 7, simple values (`false` = 20, `true` = 21).
pub const MAJOR_SIMPLE: u8 = 7;

/// The deepest an unknown entry's value may nest before a skip refuses it
/// (SCE_FORGE.md §4.6.1).
pub const MAX_SKIP_DEPTH: u32 = 16;

// ── Writing ─────────────────────────────────────────────────────────────

/// Write a head of `major` carrying `value`, in its shortest form.
pub fn write_head<S: SceSink + ?Sized>(w: &mut S, major: u8, value: u64) -> Result<(), CodecError> {
    let m = major << 5;
    if value < 24 {
        w.write_u8(m | value as u8)
    } else if value <= u64::from(u8::MAX) {
        w.write_bytes(&[m | 24, value as u8])
    } else if value <= u64::from(u16::MAX) {
        w.write_u8(m | 25)?;
        w.write_bytes(&(value as u16).to_be_bytes())
    } else if value <= u64::from(u32::MAX) {
        w.write_u8(m | 26)?;
        w.write_bytes(&(value as u32).to_be_bytes())
    } else {
        w.write_u8(m | 27)?;
        w.write_bytes(&value.to_be_bytes())
    }
}

/// Write an unsigned integer.
pub fn write_uint<S: SceSink + ?Sized>(w: &mut S, value: u64) -> Result<(), CodecError> {
    write_head(w, MAJOR_UNSIGNED, value)
}

/// Write `false` or `true`.
pub fn write_bool<S: SceSink + ?Sized>(w: &mut S, value: bool) -> Result<(), CodecError> {
    w.write_u8((MAJOR_SIMPLE << 5) | if value { 21 } else { 20 })
}

/// Write a text string.
pub fn write_text<S: SceSink + ?Sized>(w: &mut S, value: &str) -> Result<(), CodecError> {
    write_head(w, MAJOR_TEXT, value.len() as u64)?;
    w.write_bytes(value.as_bytes())
}

/// Write a byte string.
pub fn write_bytes<S: SceSink + ?Sized>(w: &mut S, value: &[u8]) -> Result<(), CodecError> {
    write_head(w, MAJOR_BYTES, value.len() as u64)?;
    w.write_bytes(value)
}

/// Write the head of a definite-length map of `entries` entries.
pub fn write_map_head<S: SceSink + ?Sized>(w: &mut S, entries: u64) -> Result<(), CodecError> {
    write_head(w, MAJOR_MAP, entries)
}

// ── Reading ─────────────────────────────────────────────────────────────

fn read_u8(c: &mut SceCursor<'_>) -> Result<u8, CodecError> {
    let b = c.peek_slice(1)?[0];
    c.advance(1)?;
    Ok(b)
}

fn read_be(c: &mut SceCursor<'_>, n: usize) -> Result<u64, CodecError> {
    let bytes = c.peek_slice(n)?;
    let value = bytes.iter().fold(0u64, |v, b| (v << 8) | u64::from(*b));
    c.advance(n)?;
    Ok(value)
}

/// Read one head: its major type and the value its additional information
/// carries (for a string or a map, the length). An indefinite length and
/// the reserved values 28–30 are refused.
pub fn read_head(c: &mut SceCursor<'_>) -> Result<(u8, u64), CodecError> {
    let initial = read_u8(c)?;
    let major = initial >> 5;
    let info = initial & 0x1F;
    let value = match info {
        0..=23 => u64::from(info),
        24 => read_be(c, 1)?,
        25 => read_be(c, 2)?,
        26 => read_be(c, 4)?,
        27 => read_be(c, 8)?,
        _ => return Err(CodecError::CborMalformed),
    };
    Ok((major, value))
}

fn expect(c: &mut SceCursor<'_>, major: u8) -> Result<u64, CodecError> {
    match read_head(c)? {
        (m, value) if m == major => Ok(value),
        _ => Err(CodecError::CborMalformed),
    }
}

/// Read an unsigned integer.
pub fn read_uint(c: &mut SceCursor<'_>) -> Result<u64, CodecError> {
    expect(c, MAJOR_UNSIGNED)
}

/// Read an unsigned integer an entry of `max` holds.
pub fn read_uint_upto(c: &mut SceCursor<'_>, max: u64) -> Result<u64, CodecError> {
    let value = read_uint(c)?;
    if value > max {
        return Err(CodecError::CborOutOfRange);
    }
    Ok(value)
}

/// Read `false` or `true`.
pub fn read_bool(c: &mut SceCursor<'_>) -> Result<bool, CodecError> {
    match read_head(c)? {
        (MAJOR_SIMPLE, 20) => Ok(false),
        (MAJOR_SIMPLE, 21) => Ok(true),
        _ => Err(CodecError::CborMalformed),
    }
}

fn read_payload<'a>(c: &mut SceCursor<'a>, len: u64) -> Result<&'a [u8], CodecError> {
    let len = usize::try_from(len).map_err(|_| CodecError::NeedMoreBytes)?;
    let bytes = c.peek_slice(len)?;
    c.advance(len)?;
    Ok(bytes)
}

/// Read a text string of at most `max_size` bytes (`None`: no bound), as a
/// view of the input.
pub fn read_text<'a>(
    c: &mut SceCursor<'a>,
    max_size: Option<usize>,
) -> Result<&'a str, CodecError> {
    let len = expect(c, MAJOR_TEXT)?;
    if max_size.is_some_and(|max| len > max as u64) {
        return Err(CodecError::CborOutOfRange);
    }
    core::str::from_utf8(read_payload(c, len)?).map_err(|_| CodecError::InvalidUtf8)
}

/// Read a byte string of at most `max_size` bytes (`None`: no bound), as a
/// view of the input.
pub fn read_bytes<'a>(
    c: &mut SceCursor<'a>,
    max_size: Option<usize>,
) -> Result<&'a [u8], CodecError> {
    let len = expect(c, MAJOR_BYTES)?;
    if max_size.is_some_and(|max| len > max as u64) {
        return Err(CodecError::CborOutOfRange);
    }
    read_payload(c, len)
}

/// Read a byte string of exactly `length` bytes.
pub fn read_bytes_exact<'a>(c: &mut SceCursor<'a>, length: usize) -> Result<&'a [u8], CodecError> {
    let len = expect(c, MAJOR_BYTES)?;
    if len != length as u64 {
        return Err(CodecError::CborWrongLength);
    }
    read_payload(c, len)
}

/// Read the head of a definite-length map: how many entries follow.
pub fn read_map_len(c: &mut SceCursor<'_>) -> Result<u64, CodecError> {
    expect(c, MAJOR_MAP)
}

/// Skip one item — the value of a key the codec does not declare — and
/// everything nested in it, refusing one nested deeper than
/// [`MAX_SKIP_DEPTH`].
pub fn skip(c: &mut SceCursor<'_>) -> Result<(), CodecError> {
    skip_at(c, 0)
}

fn skip_at(c: &mut SceCursor<'_>, depth: u32) -> Result<(), CodecError> {
    if depth >= MAX_SKIP_DEPTH {
        return Err(CodecError::CborTooDeep);
    }
    let (major, value) = read_head(c)?;
    match major {
        // Unsigned, negative, simple / float: the head is the whole item.
        0 | 1 | 7 => Ok(()),
        2 | 3 => read_payload(c, value).map(|_| ()),
        4 => {
            for _ in 0..value {
                skip_at(c, depth + 1)?;
            }
            Ok(())
        }
        5 => {
            for _ in 0..value {
                skip_at(c, depth + 1)?;
                skip_at(c, depth + 1)?;
            }
            Ok(())
        }
        // A tag: its one enclosed item follows.
        6 => skip_at(c, depth + 1),
        _ => Err(CodecError::CborMalformed),
    }
}

#[cfg(test)]
mod tests {
    extern crate std;
    use std::vec::Vec;

    use super::*;
    use crate::codec::SliceSink;

    fn written(f: impl FnOnce(&mut SliceSink<'_>) -> Result<(), CodecError>) -> Vec<u8> {
        let mut buf = [0u8; 32];
        let mut sink = SliceSink::new(&mut buf);
        f(&mut sink).expect("writes");
        sink.into_written().to_vec()
    }

    #[test]
    fn every_head_is_written_in_its_shortest_form() {
        // RFC 8949 Appendix A: 0, 23, 24, 255, 256, 65535, 65536, 2^32.
        assert_eq!(written(|w| write_uint(w, 0)), [0x00]);
        assert_eq!(written(|w| write_uint(w, 23)), [0x17]);
        assert_eq!(written(|w| write_uint(w, 24)), [0x18, 0x18]);
        assert_eq!(written(|w| write_uint(w, 255)), [0x18, 0xff]);
        assert_eq!(written(|w| write_uint(w, 256)), [0x19, 0x01, 0x00]);
        assert_eq!(written(|w| write_uint(w, 65535)), [0x19, 0xff, 0xff]);
        assert_eq!(
            written(|w| write_uint(w, 65536)),
            [0x1a, 0x00, 0x01, 0x00, 0x00]
        );
        assert_eq!(
            written(|w| write_uint(w, 1 << 32)),
            [0x1b, 0, 0, 0, 1, 0, 0, 0, 0]
        );
        assert_eq!(written(|w| write_text(w, "a")), [0x61, b'a']);
        assert_eq!(written(|w| write_bytes(w, &[1, 2])), [0x42, 1, 2]);
        assert_eq!(written(|w| write_bool(w, true)), [0xf5]);
        assert_eq!(written(|w| write_map_head(w, 2)), [0xa2]);
    }

    #[test]
    fn a_head_is_read_in_any_valid_length() {
        // 1 written in its 2-byte form still reads as 1.
        let bytes = [0x18, 0x01];
        assert_eq!(read_uint(&mut SceCursor::new(&bytes)), Ok(1));
        let bytes = [0x1b, 0, 0, 0, 1, 0, 0, 0, 0];
        assert_eq!(read_uint(&mut SceCursor::new(&bytes)), Ok(1 << 32));
    }

    #[test]
    fn what_the_codec_cannot_read_is_refused() {
        // Indefinite-length map, reserved additional information, another
        // major type, a width the entry cannot hold, a wrong exact length.
        for (bytes, error) in [
            (&[0xbf][..], CodecError::CborMalformed),
            (&[0x1c][..], CodecError::CborMalformed),
            (&[0x61, b'a'][..], CodecError::CborMalformed),
        ] {
            assert_eq!(read_uint(&mut SceCursor::new(bytes)), Err(error));
        }
        assert_eq!(
            read_map_len(&mut SceCursor::new(&[0xbf])),
            Err(CodecError::CborMalformed)
        );
        assert_eq!(
            read_uint_upto(&mut SceCursor::new(&[0x19, 0x01, 0x00]), 255),
            Err(CodecError::CborOutOfRange)
        );
        assert_eq!(
            read_bytes_exact(&mut SceCursor::new(&[0x42, 1, 2]), 16),
            Err(CodecError::CborWrongLength)
        );
        assert_eq!(
            read_text(&mut SceCursor::new(&[0x62, b'a', b'b']), Some(1)),
            Err(CodecError::CborOutOfRange)
        );
        assert_eq!(
            read_text(&mut SceCursor::new(&[0x61, 0xff]), None),
            Err(CodecError::InvalidUtf8)
        );
    }

    #[test]
    fn an_unknown_value_is_skipped_whole_and_a_deep_one_refused() {
        // {1: [2, {3: h'00'}]} then 7: the skip lands on the 7.
        let bytes = [0xa1, 0x01, 0x82, 0x02, 0xa1, 0x03, 0x41, 0x00, 0x07];
        let mut c = SceCursor::new(&bytes);
        skip(&mut c).expect("skips");
        assert_eq!(read_uint(&mut c), Ok(7));

        let mut deep = [0x81u8; 20].to_vec();
        deep.push(0x00);
        assert_eq!(
            skip(&mut SceCursor::new(&deep)),
            Err(CodecError::CborTooDeep)
        );
    }
}
