// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! The CBOR (RFC 8949) items a `sce:encoding="cbor"` codec reads and writes
//! (SCE_FORGE.md §4.6.1, docs/adr/0101): one definite-length map whose keys
//! are small unsigned integers and whose values are values of the language,
//! each in its one form: an integer, a float, `bool`, a text or byte string,
//! an array, a map keyed by text, and a union's map of one entry.
//!
//! A generated codec calls these; it spells no CBOR of its own. Written here
//! once, beside [`SceCursor`] and [`SceSink`], rather than per codec — the
//! same place the Zenoh variable-length integer lives — and heap-free, so a
//! `no_std` build carries it. The trust layer's signed encoding writes its
//! integers and floats through the same writers.
//!
//! **Writing** is deterministic (RFC 8949 §4.2.1): every head in its
//! shortest form and every float in the shortest precision that holds it
//! exactly, so two backends given the same value write the same bytes.
//! **Reading** takes the profile of definite lengths (docs/adr/0101 §3.1),
//! and within it every well-formed encoding of a value: a head in any valid
//! length, a float in any precision that holds the value. It refuses each
//! input by the class RFC 8949 §1.2 puts it in (docs/adr/0101 §3.3): an item
//! that is not well-formed or has an indefinite length is
//! [`CodecError::CborMalformed`], an item of a major type, simple value or
//! tag the place does not take is [`CodecError::CborWrongType`], and a value
//! outside its type is [`CodecError::CborOutOfRange`].

use crate::codec::{CodecError, SceCursor, SceSink};

/// Major type 0, an unsigned integer.
pub const MAJOR_UNSIGNED: u8 = 0;
/// Major type 1, the negative integer `-1 - n` for the argument `n`.
pub const MAJOR_NEGATIVE: u8 = 1;
/// Major type 2, a byte string.
pub const MAJOR_BYTES: u8 = 2;
/// Major type 3, a UTF-8 text string.
pub const MAJOR_TEXT: u8 = 3;
/// Major type 4, an array.
pub const MAJOR_ARRAY: u8 = 4;
/// Major type 5, a map.
pub const MAJOR_MAP: u8 = 5;
/// Major type 7, simple values (`false` = 20, `true` = 21) and floats.
pub const MAJOR_SIMPLE: u8 = 7;

/// The additional information of a half, a single and a double precision
/// float under major type 7 (RFC 8949 §3.3).
const INFO_HALF: u8 = 25;
const INFO_SINGLE: u8 = 26;
const INFO_DOUBLE: u8 = 27;

/// The one NaN written: the quiet NaN of half precision (docs/adr/0086,
/// section 9.2).
const NAN: [u8; 3] = [(MAJOR_SIMPLE << 5) | INFO_HALF, 0x7e, 0x00];

/// The simple value `null` (RFC 8949 §3.3): an absent element of a list
/// (docs/adr/0101 §2.1).
const SIMPLE_NULL: u8 = 22;

/// `null`'s one encoding, `f6`: the two-byte form of a simple value below 32
/// is not well-formed.
const NULL: u8 = (MAJOR_SIMPLE << 5) | SIMPLE_NULL;

/// The deepest an unknown entry's value may nest before a skip refuses it
/// (SCE_FORGE.md §4.6.1).
pub const MAX_SKIP_DEPTH: u32 = 16;

// ── Writing ─────────────────────────────────────────────────────────────

// A write refuses nothing of its own: what is left is the sink's error
// (docs/adr/0065 §4), so a codec whose entries cannot break a bound is total.

/// Write a head of `major` carrying `value`, in its shortest form.
pub fn write_head<S: SceSink + ?Sized>(w: &mut S, major: u8, value: u64) -> Result<(), S::Error> {
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
pub fn write_uint<S: SceSink + ?Sized>(w: &mut S, value: u64) -> Result<(), S::Error> {
    write_head(w, MAJOR_UNSIGNED, value)
}

/// Write a signed integer: major type 0 for zero or more, major type 1 with
/// the argument `-1 - n` below zero, each with its shortest head (RFC 8949
/// §3.1 and §4.2.1).
pub fn write_int<S: SceSink + ?Sized>(w: &mut S, value: i64) -> Result<(), S::Error> {
    if value < 0 {
        // `-1 - value` is at most `i64::MAX`: it neither overflows nor loses
        // a bit as a `u64`.
        write_head(w, MAJOR_NEGATIVE, (-1 - value) as u64)
    } else {
        write_head(w, MAJOR_UNSIGNED, value as u64)
    }
}

/// The half precision float that is `x` exactly, as its bits, or `None` when
/// no half is (IEEE 754 binary16: 5 bits of exponent, 10 of fraction,
/// subnormal below 2^-14). A NaN has none: [`write_float`] writes the one NaN
/// before it asks.
pub fn half_of(x: f64) -> Option<u16> {
    let bits = x.to_bits();
    let sign = ((bits >> 48) & 0x8000) as u16;
    let exponent = ((bits >> 52) & 0x7ff) as i32;
    let fraction = bits & ((1 << 52) - 1);
    if exponent == 0x7ff {
        // An infinity; a NaN is no half's.
        return (fraction == 0).then_some(sign | 0x7c00);
    }
    if exponent == 0 {
        // Zero, or a double's subnormal, which is far below a half's least.
        return (fraction == 0).then_some(sign);
    }
    let power = exponent - 1023;
    if (-14..=15).contains(&power) {
        // A half's normal number keeps the top 10 bits of the fraction and nothing below them.
        if (fraction & ((1 << 42) - 1)) != 0 {
            return None;
        }
        return Some(sign | (((power + 15) as u16) << 10) | (fraction >> 42) as u16);
    }
    if (-24..=-15).contains(&power) {
        // A half's subnormal number is k * 2^-24 for k from 1 to 1023.
        let significand = fraction | (1 << 52);
        let shift = (28 - power) as u32;
        if (significand & ((1 << shift) - 1)) != 0 {
            return None;
        }
        return Some(sign | (significand >> shift) as u16);
    }
    None
}

/// The single precision float that is `x` exactly, or `None` when no single
/// is. A NaN has none.
pub fn single_of(x: f64) -> Option<f32> {
    let single = x as f32;
    (f64::from(single) == x).then_some(single)
}

/// Write a float in the shortest of half, single and double precision that
/// holds it exactly, and a NaN as `f9 7e 00` (RFC 8949 §4.1 and §4.2.1,
/// docs/adr/0086 section 9.2). A `float32` is written by its value as a
/// double, which a single always holds.
pub fn write_float<S: SceSink + ?Sized>(w: &mut S, x: f64) -> Result<(), S::Error> {
    if x.is_nan() {
        w.write_bytes(&NAN)
    } else if let Some(half) = half_of(x) {
        w.write_u8((MAJOR_SIMPLE << 5) | INFO_HALF)?;
        w.write_bytes(&half.to_be_bytes())
    } else if let Some(single) = single_of(x) {
        w.write_u8((MAJOR_SIMPLE << 5) | INFO_SINGLE)?;
        w.write_bytes(&single.to_bits().to_be_bytes())
    } else {
        w.write_u8((MAJOR_SIMPLE << 5) | INFO_DOUBLE)?;
        w.write_bytes(&x.to_bits().to_be_bytes())
    }
}

/// Write `false` or `true`.
pub fn write_bool<S: SceSink + ?Sized>(w: &mut S, value: bool) -> Result<(), S::Error> {
    w.write_u8((MAJOR_SIMPLE << 5) | if value { 21 } else { 20 })
}

/// Write a text string.
pub fn write_text<S: SceSink + ?Sized>(w: &mut S, value: &str) -> Result<(), S::Error> {
    write_head(w, MAJOR_TEXT, crate::codec::widen_usize(value.len()))?;
    w.write_bytes(value.as_bytes())
}

/// Write a byte string.
pub fn write_bytes<S: SceSink + ?Sized>(w: &mut S, value: &[u8]) -> Result<(), S::Error> {
    write_head(w, MAJOR_BYTES, crate::codec::widen_usize(value.len()))?;
    w.write_bytes(value)
}

/// Write the head of a definite-length array of `items` items: a list of
/// records an entry holds (docs/adr/0086, section 9.2).
pub fn write_array_head<S: SceSink + ?Sized>(w: &mut S, items: u64) -> Result<(), S::Error> {
    write_head(w, MAJOR_ARRAY, items)
}

/// Write the head of a definite-length map of `entries` entries.
pub fn write_map_head<S: SceSink + ?Sized>(w: &mut S, entries: u64) -> Result<(), S::Error> {
    write_head(w, MAJOR_MAP, entries)
}

/// Write the head of a union's value: a map of one entry keyed by the case's
/// variant name as a text string (docs/adr/0101 §2.1). The case's record, or
/// the empty map for a case that holds none, is written after it.
pub fn write_variant<S: SceSink + ?Sized>(w: &mut S, name: &str) -> Result<(), S::Error> {
    write_map_head(w, 1)?;
    write_text(w, name)
}

/// Write `null`, an absent element of a list (docs/adr/0101 §2.1). An absent
/// field of a record and an absent entry of the codec's own map are not
/// written at all: their key is left out.
pub fn write_null<S: SceSink + ?Sized>(w: &mut S) -> Result<(), S::Error> {
    w.write_u8(NULL)
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

/// One head as the input wrote it: its major type, its additional
/// information, which tells a float's precision apart, and its argument.
struct Head {
    major: u8,
    info: u8,
    argument: u64,
}

/// Read one head of the profile. What is not well-formed (RFC 8949 Appendix
/// F) is refused: the reserved additional information 28-30, a simple value
/// in the two-byte form below 32 (RFC 8949 §3.3), and a break code where an
/// item stands; so is an indefinite length, which the profile does not hold
/// (docs/adr/0101 §3.1). Each is [`CodecError::CborMalformed`].
fn read_item_head(c: &mut SceCursor<'_>) -> Result<Head, CodecError> {
    let initial = read_u8(c)?;
    let major = initial >> 5;
    let info = initial & 0x1F;
    let argument = match info {
        0..=23 => u64::from(info),
        24 => read_be(c, 1)?,
        25 => read_be(c, 2)?,
        26 => read_be(c, 4)?,
        27 => read_be(c, 8)?,
        _ => return Err(CodecError::CborMalformed),
    };
    if major == MAJOR_SIMPLE && info == 24 && argument < 32 {
        return Err(CodecError::CborMalformed);
    }
    Ok(Head {
        major,
        info,
        argument,
    })
}

/// Read one head: its major type and the value its additional information
/// carries (for a string or a map, the length). An indefinite length, the
/// reserved values 28–30 and a two-byte simple value below 32 are refused.
pub fn read_head(c: &mut SceCursor<'_>) -> Result<(u8, u64), CodecError> {
    read_item_head(c).map(|head| (head.major, head.argument))
}

/// The argument of a head of `major`; a head of another major type is an
/// item the place does not take.
fn expect(c: &mut SceCursor<'_>, major: u8) -> Result<u64, CodecError> {
    let head = read_item_head(c)?;
    if head.major == major {
        Ok(head.argument)
    } else {
        Err(CodecError::CborWrongType)
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

/// Read a signed integer of `least..=greatest`, the range of its type:
/// major type 0, or major type 1 below zero. A number outside the range is
/// [`CodecError::CborOutOfRange`], another item
/// [`CodecError::CborWrongType`].
pub fn read_int_within(
    c: &mut SceCursor<'_>,
    least: i64,
    greatest: i64,
) -> Result<i64, CodecError> {
    let head = read_item_head(c)?;
    let value = match head.major {
        MAJOR_UNSIGNED => i128::from(head.argument),
        MAJOR_NEGATIVE => -1 - i128::from(head.argument),
        _ => return Err(CodecError::CborWrongType),
    };
    i64::try_from(value)
        .ok()
        .filter(|n| (least..=greatest).contains(n))
        .ok_or(CodecError::CborOutOfRange)
}

/// 2^`e` as a double, for `e` among a double's normal exponents.
fn pow2(e: i32) -> f64 {
    f64::from_bits(((e + 1023) as u64) << 52)
}

/// The value of a half precision float's bits (IEEE 754 binary16), exactly.
fn from_half(bits: u16) -> f64 {
    let exponent = i32::from((bits >> 10) & 0x1f);
    let fraction = f64::from(bits & 0x3ff);
    let magnitude = match exponent {
        0 => fraction * pow2(-24),
        0x1f if fraction == 0.0 => f64::INFINITY,
        0x1f => f64::NAN,
        _ => (1024.0 + fraction) * pow2(exponent - 25),
    };
    if bits & 0x8000 != 0 {
        -magnitude
    } else {
        magnitude
    }
}

/// Read a float in any of half, single and double precision, as its value:
/// a variation-tolerant read (RFC 8949 §4.1). An integer, a simple value and
/// every other item are [`CodecError::CborWrongType`].
pub fn read_float(c: &mut SceCursor<'_>) -> Result<f64, CodecError> {
    let head = read_item_head(c)?;
    match (head.major, head.info) {
        (MAJOR_SIMPLE, INFO_HALF) => Ok(from_half(head.argument as u16)),
        (MAJOR_SIMPLE, INFO_SINGLE) => Ok(f64::from(f32::from_bits(head.argument as u32))),
        (MAJOR_SIMPLE, INFO_DOUBLE) => Ok(f64::from_bits(head.argument)),
        _ => Err(CodecError::CborWrongType),
    }
}

/// Read a `float32`: a float in any precision whose value a single holds.
/// A double that no single is exactly is [`CodecError::CborOutOfRange`]; a
/// NaN of any precision is a NaN.
pub fn read_float32(c: &mut SceCursor<'_>) -> Result<f32, CodecError> {
    let x = read_float(c)?;
    if x.is_nan() {
        return Ok(f32::NAN);
    }
    single_of(x).ok_or(CodecError::CborOutOfRange)
}

/// Read `false` or `true`. Every other item, `null` and a float among them,
/// is [`CodecError::CborWrongType`].
pub fn read_bool(c: &mut SceCursor<'_>) -> Result<bool, CodecError> {
    let head = read_item_head(c)?;
    match (head.major, head.info) {
        (MAJOR_SIMPLE, 20) => Ok(false),
        (MAJOR_SIMPLE, 21) => Ok(true),
        _ => Err(CodecError::CborWrongType),
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

/// Read the head of a definite-length array: how many items follow. An
/// indefinite length is refused, as a map's is.
pub fn read_array_len(c: &mut SceCursor<'_>) -> Result<u64, CodecError> {
    expect(c, MAJOR_ARRAY)
}

/// Read the head of a definite-length map: how many entries follow.
pub fn read_map_len(c: &mut SceCursor<'_>) -> Result<u64, CodecError> {
    expect(c, MAJOR_MAP)
}

/// Read the head of a union's value and name its case (docs/adr/0101 §2.1
/// and §3.3): a map, judged for exactly one entry before its key is read
/// ([`CodecError::CborUnionEntries`]), whose key is a text string
/// ([`CodecError::CborWrongType`]) naming one of `names`
/// ([`CodecError::CborUnknownVariant`]). Answers the case's index in
/// `names`; the case's record, or the empty map, follows.
pub fn read_variant(c: &mut SceCursor<'_>, names: &[&str]) -> Result<usize, CodecError> {
    if read_map_len(c)? != 1 {
        return Err(CodecError::CborUnionEntries);
    }
    let name = read_text(c, None)?;
    names
        .iter()
        .position(|candidate| *candidate == name)
        .ok_or(CodecError::CborUnknownVariant)
}

/// Read the value of a union's case that holds no record: a map, the empty
/// map as an encode writes it (docs/adr/0101 §2.1). Read as a record's map is
/// (§3.2): every key a text string ([`CodecError::CborWrongType`]), and each
/// stepped over with its value, since the case declares none.
pub fn read_empty_case(c: &mut SceCursor<'_>) -> Result<(), CodecError> {
    let count = read_map_len(c)?;
    for _ in 0..count {
        read_text(c, None)?;
        skip(c)?;
    }
    Ok(())
}

/// Take `null` where a list's element that may be absent stands
/// (docs/adr/0101 §2.1): whether the item there is `null`, which is then read.
/// Any other item is left where it is, for the element's own read, which
/// refuses what is no value of the element.
pub fn take_null(c: &mut SceCursor<'_>) -> Result<bool, CodecError> {
    if c.peek_slice(1)?[0] == NULL {
        c.advance(1)?;
        Ok(true)
    } else {
        Ok(false)
    }
}

/// Skip one item — the value of a key the codec does not declare — and
/// everything nested in it, refusing one nested deeper than
/// [`MAX_SKIP_DEPTH`] and one the profile does not hold (an indefinite
/// length at any depth).
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
        assert_eq!(written(|w| write_array_head(w, 2)), [0x82]);
        assert_eq!(written(|w| write_array_head(w, 24)), [0x98, 0x18]);
    }

    /// A signed integer at each boundary of a head's length, on both sides of
    /// zero: the argument of a negative number is `-1 - n`, so -24 is the
    /// last of one byte and -25 the first of two.
    const SIGNED: [(i64, &[u8]); 16] = [
        (0, &[0x00]),
        (23, &[0x17]),
        (24, &[0x18, 0x18]),
        (-1, &[0x20]),
        (-24, &[0x37]),
        (-25, &[0x38, 0x18]),
        (-256, &[0x38, 0xff]),
        (-257, &[0x39, 0x01, 0x00]),
        (-500, &[0x39, 0x01, 0xf3]),
        (-65536, &[0x39, 0xff, 0xff]),
        (-65537, &[0x3a, 0x00, 0x01, 0x00, 0x00]),
        (-(1 << 32), &[0x3a, 0xff, 0xff, 0xff, 0xff]),
        (-(1 << 32) - 1, &[0x3b, 0, 0, 0, 1, 0, 0, 0, 0]),
        (
            i64::MIN,
            &[0x3b, 0x7f, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff],
        ),
        (
            i64::MAX,
            &[0x1b, 0x7f, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff],
        ),
        ((1 << 32) - 1, &[0x1a, 0xff, 0xff, 0xff, 0xff]),
    ];

    #[test]
    fn a_signed_integer_is_written_in_its_shortest_head_and_read_back() {
        for (value, bytes) in SIGNED {
            assert_eq!(written(|w| write_int(w, value)), bytes, "{value}");
            assert_eq!(
                read_int_within(&mut SceCursor::new(bytes), i64::MIN, i64::MAX),
                Ok(value),
                "{value}"
            );
        }
    }

    #[test]
    fn a_signed_integer_is_read_in_any_head_and_held_to_its_range() {
        // -1 in its two-byte form.
        assert_eq!(
            read_int_within(&mut SceCursor::new(&[0x38, 0x00]), -128, 127),
            Ok(-1)
        );
        // -128 and 127 are an int8's ends; -129 and 128 are past them.
        assert_eq!(
            read_int_within(&mut SceCursor::new(&[0x38, 0x7f]), -128, 127),
            Ok(-128)
        );
        assert_eq!(
            read_int_within(&mut SceCursor::new(&[0x18, 0x7f]), -128, 127),
            Ok(127)
        );
        assert_eq!(
            read_int_within(&mut SceCursor::new(&[0x38, 0x80]), -128, 127),
            Err(CodecError::CborOutOfRange)
        );
        assert_eq!(
            read_int_within(&mut SceCursor::new(&[0x18, 0x80]), -128, 127),
            Err(CodecError::CborOutOfRange)
        );
        // Below int64's least and past its greatest: no i64 holds either.
        assert_eq!(
            read_int_within(
                &mut SceCursor::new(&[0x3b, 0x80, 0, 0, 0, 0, 0, 0, 0]),
                i64::MIN,
                i64::MAX
            ),
            Err(CodecError::CborOutOfRange)
        );
        assert_eq!(
            read_int_within(
                &mut SceCursor::new(&[0x1b, 0x80, 0, 0, 0, 0, 0, 0, 0]),
                i64::MIN,
                i64::MAX
            ),
            Err(CodecError::CborOutOfRange)
        );
        // A float and a text are no integer.
        assert_eq!(
            read_int_within(&mut SceCursor::new(&[0xf9, 0x3c, 0x00]), i64::MIN, i64::MAX),
            Err(CodecError::CborWrongType)
        );
        assert_eq!(
            read_int_within(&mut SceCursor::new(&[0x61, b'a']), i64::MIN, i64::MAX),
            Err(CodecError::CborWrongType)
        );
    }

    /// RFC 8949 Appendix A, and the edges of each precision.
    fn floats() -> [(f64, &'static [u8]); 20] {
        [
            (0.0, &[0xf9, 0x00, 0x00]),
            (-0.0, &[0xf9, 0x80, 0x00]),
            (1.0, &[0xf9, 0x3c, 0x00]),
            (1.1, &[0xfb, 0x3f, 0xf1, 0x99, 0x99, 0x99, 0x99, 0x99, 0x9a]),
            (1.5, &[0xf9, 0x3e, 0x00]),
            // The greatest half, and one past it in a half's spacing there,
            // which a single holds.
            (65504.0, &[0xf9, 0x7b, 0xff]),
            (65505.0, &[0xfa, 0x47, 0x7f, 0xe1, 0x00]),
            (100000.0, &[0xfa, 0x47, 0xc3, 0x50, 0x00]),
            (f64::from(f32::MAX), &[0xfa, 0x7f, 0x7f, 0xff, 0xff]),
            (f64::from(0.1f32), &[0xfa, 0x3d, 0xcc, 0xcc, 0xcd]),
            (
                1.0e300,
                &[0xfb, 0x7e, 0x37, 0xe4, 0x3c, 0x88, 0x00, 0x75, 0x9c],
            ),
            // The least half, a subnormal; the greatest half subnormal; the
            // least half normal; half the least half, which only a single
            // holds.
            (1.0 / 16_777_216.0, &[0xf9, 0x00, 0x01]),
            (1023.0 / 16_777_216.0, &[0xf9, 0x03, 0xff]),
            (1.0 / 16384.0, &[0xf9, 0x04, 0x00]),
            (1.0 / 33_554_432.0, &[0xfa, 0x33, 0x00, 0x00, 0x00]),
            // One bit below a single's last: only a double holds it.
            (
                1.0 + 1.0 / 16_777_216.0,
                &[0xfb, 0x3f, 0xf0, 0x00, 0x00, 0x10, 0x00, 0x00, 0x00],
            ),
            (-4.0, &[0xf9, 0xc4, 0x00]),
            (
                -4.1,
                &[0xfb, 0xc0, 0x10, 0x66, 0x66, 0x66, 0x66, 0x66, 0x66],
            ),
            (f64::INFINITY, &[0xf9, 0x7c, 0x00]),
            (f64::NEG_INFINITY, &[0xf9, 0xfc, 0x00]),
        ]
    }

    #[test]
    fn a_float_is_written_in_the_shortest_precision_that_holds_it_and_read_back() {
        for (x, bytes) in floats() {
            assert_eq!(written(|w| write_float(w, x)), bytes, "{x:e}");
            let read = read_float(&mut SceCursor::new(bytes)).expect("reads");
            assert_eq!(read.to_bits(), x.to_bits(), "{x:e}");
        }
        // Every NaN is the one NaN, whatever its sign and payload.
        for nan in [
            f64::NAN,
            f64::from_bits(0x7ff0_0000_0000_0001),
            f64::from_bits(0xfff8_0000_0000_0000),
        ] {
            assert_eq!(written(|w| write_float(w, nan)), [0xf9, 0x7e, 0x00]);
        }
    }

    #[test]
    fn a_float_is_read_in_any_precision_that_holds_it() {
        // 1.5 as a half, a single and a double.
        for bytes in [
            &[0xf9, 0x3e, 0x00][..],
            &[0xfa, 0x3f, 0xc0, 0x00, 0x00][..],
            &[0xfb, 0x3f, 0xf8, 0, 0, 0, 0, 0, 0][..],
        ] {
            assert_eq!(read_float(&mut SceCursor::new(bytes)), Ok(1.5));
            assert_eq!(read_float32(&mut SceCursor::new(bytes)), Ok(1.5));
        }
        // A NaN of each precision.
        for bytes in [
            &[0xf9, 0x7e, 0x00][..],
            &[0xfa, 0x7f, 0xc0, 0x00, 0x00][..],
            &[0xfb, 0x7f, 0xf8, 0, 0, 0, 0, 0, 0][..],
        ] {
            assert!(read_float(&mut SceCursor::new(bytes))
                .expect("reads")
                .is_nan());
            assert!(read_float32(&mut SceCursor::new(bytes))
                .expect("reads")
                .is_nan());
        }
        // 0.1 as a double is no single: a float32 refuses it.
        let tenth = [0xfb, 0x3f, 0xb9, 0x99, 0x99, 0x99, 0x99, 0x99, 0x9a];
        assert_eq!(read_float(&mut SceCursor::new(&tenth)), Ok(0.1));
        assert_eq!(
            read_float32(&mut SceCursor::new(&tenth)),
            Err(CodecError::CborOutOfRange)
        );
        // An integer, a truth value and null are no float.
        for bytes in [&[0x01][..], &[0xf5][..], &[0xf6][..], &[0x20][..]] {
            assert_eq!(
                read_float(&mut SceCursor::new(bytes)),
                Err(CodecError::CborWrongType)
            );
        }
    }

    #[test]
    fn an_array_head_is_read_as_its_count_and_no_other_item_is() {
        assert_eq!(read_array_len(&mut SceCursor::new(&[0x83])), Ok(3));
        assert_eq!(read_array_len(&mut SceCursor::new(&[0x98, 0x18])), Ok(24));
        // An indefinite-length array, and a map where an array stands.
        assert_eq!(
            read_array_len(&mut SceCursor::new(&[0x9f])),
            Err(CodecError::CborMalformed)
        );
        assert_eq!(
            read_array_len(&mut SceCursor::new(&[0xa1])),
            Err(CodecError::CborWrongType)
        );
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
            (&[0x61, b'a'][..], CodecError::CborWrongType),
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
    fn an_item_that_is_not_well_formed_or_not_in_the_profile_is_malformed() {
        // An indefinite text and byte string, in chunks, and a break code
        // where an item stands; a simple value in the two-byte form below
        // 32, which is not well-formed (RFC 8949 §3.3).
        assert_eq!(
            read_text(
                &mut SceCursor::new(&[0x7f, 0x61, b'a', 0x61, b'b', 0xff]),
                None
            ),
            Err(CodecError::CborMalformed)
        );
        assert_eq!(
            read_bytes(&mut SceCursor::new(&[0x5f, 0x41, 0x00, 0xff]), None),
            Err(CodecError::CborMalformed)
        );
        assert_eq!(
            read_uint(&mut SceCursor::new(&[0xff])),
            Err(CodecError::CborMalformed)
        );
        assert_eq!(
            read_bool(&mut SceCursor::new(&[0xf8, 0x14])),
            Err(CodecError::CborMalformed)
        );
        for reserved in [0x1c, 0x1d, 0x1e] {
            assert_eq!(
                read_head(&mut SceCursor::new(&[reserved])),
                Err(CodecError::CborMalformed)
            );
        }
    }

    #[test]
    fn an_item_the_place_does_not_take_is_of_the_wrong_type() {
        // A negative integer where an unsigned stands, null, a bignum (a
        // tagged item), a float whose bits are a truth value's number.
        for bytes in [&[0x20][..], &[0xf6][..], &[0xc2, 0x41, 0x01][..]] {
            assert_eq!(
                read_uint(&mut SceCursor::new(bytes)),
                Err(CodecError::CborWrongType)
            );
        }
        for bytes in [&[0xf6][..], &[0xf9, 0x00, 0x14][..], &[0x01][..]] {
            assert_eq!(
                read_bool(&mut SceCursor::new(bytes)),
                Err(CodecError::CborWrongType)
            );
        }
        // An unsigned key where a record's text key stands, an array where a
        // map stands.
        assert_eq!(
            read_text(&mut SceCursor::new(&[0x01]), None),
            Err(CodecError::CborWrongType)
        );
        assert_eq!(
            read_map_len(&mut SceCursor::new(&[0x81, 0x01])),
            Err(CodecError::CborWrongType)
        );
    }

    const PAYMENT: [&str; 3] = ["card", "cash", "voucher"];

    #[test]
    fn a_union_is_a_map_of_one_entry_named_by_its_case() {
        // docs/adr/0101 §2.8: `cash` of `union:Payment` is `a1 64 63 61 73 68 a0`.
        let cash = [0xa1, 0x64, b'c', b'a', b's', b'h', 0xa0];
        assert_eq!(
            written(|w| {
                write_variant(w, "cash")?;
                write_map_head(w, 0)
            }),
            cash
        );
        let mut c = SceCursor::new(&cash);
        assert_eq!(read_variant(&mut c, &PAYMENT), Ok(1));
        assert_eq!(read_map_len(&mut c), Ok(0));
    }

    #[test]
    fn a_union_of_no_entry_or_two_or_of_an_unknown_case_is_refused() {
        // No entry, and two: the count is judged before any key is read.
        assert_eq!(
            read_variant(&mut SceCursor::new(&[0xa0]), &PAYMENT),
            Err(CodecError::CborUnionEntries)
        );
        assert_eq!(
            read_variant(&mut SceCursor::new(&[0xa2]), &PAYMENT),
            Err(CodecError::CborUnionEntries)
        );
        // A case no variant names.
        assert_eq!(
            read_variant(
                &mut SceCursor::new(&[0xa1, 0x64, b'c', b'o', b'i', b'n', 0xa0]),
                &PAYMENT
            ),
            Err(CodecError::CborUnknownVariant)
        );
        // A key that is no text, and an array where the union's map stands.
        assert_eq!(
            read_variant(&mut SceCursor::new(&[0xa1, 0x01, 0xa0]), &PAYMENT),
            Err(CodecError::CborWrongType)
        );
        assert_eq!(
            read_variant(&mut SceCursor::new(&[0x81, 0xa0]), &PAYMENT),
            Err(CodecError::CborWrongType)
        );
    }

    #[test]
    fn a_case_that_holds_no_record_is_read_from_any_map_of_text_keys() {
        // The empty map an encode writes, and a map of keys the case does not
        // declare, each stepped over with its value: both are the case.
        // The second is followed by a 7 the read leaves.
        for (bytes, left) in [
            (&[0xa0][..], 0),
            (&[0xa1, 0x61, b'x', 0x82, 0x01, 0x02, 0x07][..], 1),
        ] {
            let mut c = SceCursor::new(bytes);
            assert_eq!(read_empty_case(&mut c), Ok(()), "{bytes:02x?}");
            assert_eq!(c.remaining(), left, "{bytes:02x?}");
        }
        // A key that is no text, and `null` or an array where the map stands.
        for bytes in [&[0xa1, 0x01, 0x01][..], &[0xf6][..], &[0x80][..]] {
            assert_eq!(
                read_empty_case(&mut SceCursor::new(bytes)),
                Err(CodecError::CborWrongType),
                "{bytes:02x?}"
            );
        }
    }

    #[test]
    fn an_absent_element_is_null_and_a_present_one_is_left_for_its_read() {
        let mut buf = [0u8; 1];
        let mut sink = SliceSink::new(&mut buf);
        write_null(&mut sink).expect("writes");
        assert_eq!(sink.into_written().to_vec(), [0xf6]);
        // `null`, then the element 7: the first is taken, the second left.
        let bytes = [0xf6, 0x07];
        let mut c = SceCursor::new(&bytes);
        assert_eq!(take_null(&mut c), Ok(true));
        assert_eq!(take_null(&mut c), Ok(false));
        assert_eq!(c.remaining(), 1);
        assert_eq!(read_uint(&mut c), Ok(7));
        // `false` is no `null`, and nothing at all is an input that ended.
        assert_eq!(take_null(&mut SceCursor::new(&[0xf4])), Ok(false));
        assert_eq!(
            take_null(&mut SceCursor::new(&[])),
            Err(CodecError::NeedMoreBytes)
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

    #[test]
    fn a_skipped_value_of_indefinite_length_is_refused_at_any_depth() {
        for bytes in [
            &[0x9f, 0x01, 0xff][..],
            &[0xbf, 0x01, 0x02, 0xff][..],
            &[0x7f, 0x61, b'a', 0xff][..],
            &[0x5f, 0x41, 0x00, 0xff][..],
            // Nested one and two levels down.
            &[0x81, 0x9f, 0xff][..],
            &[0xa1, 0x01, 0x81, 0x7f, 0xff][..],
        ] {
            assert_eq!(
                skip(&mut SceCursor::new(bytes)),
                Err(CodecError::CborMalformed),
                "{bytes:02x?}"
            );
        }
    }
}
