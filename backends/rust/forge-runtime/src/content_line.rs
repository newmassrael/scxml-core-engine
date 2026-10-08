// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! The content lines (RFC 5545 §3.1) a `sce:encoding="content-line"` codec
//! reads and writes (SCE_FORGE.md §4.6.4, docs/adr/0010): `BEGIN:<component>`,
//! properties — a name, `;`-separated parameters, `:` and a value — and
//! `END:<component>`, folded at 75 octets.
//!
//! A generated codec calls these; it spells no line grammar of its own. The
//! rules are written once in SCE_FORGE.md §4.6.4 and implemented once per
//! backend runtime, here for Rust: heap-free, so a `no_std` build carries it.
//! A value is decoded into the bounded `heapless::String<N>` the codec's
//! `sce:max-size` names, which is why no allocation is needed to unfold and
//! unescape it.
//!
//! **Reading** is a walk over one component: [`ContentLineReader::next_property`]
//! yields each property line of the component that is not nested in another,
//! and the codec asks the [`Property`] whether it is one of its own. A property
//! the codec does not read is never looked at again; one it does is read
//! parameter by parameter and then by value. Nothing is copied until a value is
//! read into its bounded storage.
//!
//! **Writing** is the mirror: [`ContentLineWriter`] writes a property's name,
//! its parameters and its value, cutting a line only before the *unit* — one
//! UTF-8 character, or the two octets of a TEXT escape — that would take it past
//! 75 octets.

use crate::codec::{CodecError, SceSink};
use crate::heapless::{String as BoundedString, Vec as BoundedVec};

/// The most octets of one physical line (SCE_FORGE.md §4.6.4, *Folding*).
pub const FOLD_WIDTH: usize = 75;

/// A control character a value never holds: everything below U+0020 but the
/// tab, and U+007F.
fn is_control(b: u8) -> bool {
    (b < 0x20 && b != b'\t') || b == 0x7F
}

/// A byte of a property or parameter name: letters, digits and hyphens.
fn is_name_byte(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'-'
}

// ── Unfolding ───────────────────────────────────────────────────────────

/// A walk over the bytes of one logical line, with its folds removed as it
/// goes: a line break and the one space or tab after it are not part of the
/// text, wherever the sender cut.
#[derive(Debug, Clone, Copy)]
struct Scan<'a> {
    raw: &'a [u8],
    pos: usize,
}

impl<'a> Scan<'a> {
    fn new(raw: &'a [u8]) -> Self {
        Self { raw, pos: 0 }
    }

    /// Step over every fold at the current position.
    fn skip_folds(&mut self) {
        loop {
            let rest = &self.raw[self.pos..];
            let blank = |i: usize| matches!(rest.get(i), Some(b' ' | b'\t'));
            let skipped = if rest.starts_with(b"\r\n") && blank(2) {
                3
            } else if rest.first() == Some(&b'\n') && blank(1) {
                2
            } else {
                0
            };
            if skipped == 0 {
                return;
            }
            self.pos += skipped;
        }
    }

    fn peek(&mut self) -> Option<u8> {
        self.skip_folds();
        self.raw.get(self.pos).copied()
    }

    fn bump(&mut self) -> Option<u8> {
        let b = self.peek()?;
        self.pos += 1;
        Some(b)
    }
}

/// Whether the unfolded text of `raw` is `expected`, without regard to case.
fn unfolded_eq(raw: &[u8], expected: &str) -> bool {
    let mut scan = Scan::new(raw);
    for e in expected.bytes() {
        match scan.bump() {
            Some(b) if b.eq_ignore_ascii_case(&e) => {}
            _ => return false,
        }
    }
    scan.peek().is_none()
}

/// One logical line of the input, with its folds still inside it.
struct RawLine<'a> {
    raw: &'a [u8],
    /// Whether a line break ended it. The last line of an input that ends
    /// without one may be cut short.
    terminated: bool,
}

/// The next logical line from `pos`, or `None` at the end of the input. A line
/// ends at a line break (CRLF or LF) that no space or tab follows.
fn next_raw_line<'a>(input: &'a [u8], pos: &mut usize) -> Option<RawLine<'a>> {
    if *pos >= input.len() {
        return None;
    }
    let start = *pos;
    for (i, b) in input.iter().enumerate().skip(start) {
        if *b == b'\n' && !matches!(input.get(i + 1), Some(b' ' | b'\t')) {
            let end = if i > start && input[i - 1] == b'\r' {
                i - 1
            } else {
                i
            };
            *pos = i + 1;
            return Some(RawLine {
                raw: &input[start..end],
                terminated: true,
            });
        }
    }
    *pos = input.len();
    Some(RawLine {
        raw: &input[start..],
        terminated: false,
    })
}

/// The name of a line and what follows it: the byte after the name (`;` or
/// `:`), and where the rest of the line starts.
struct Head {
    name_end: usize,
    separator: u8,
    rest_start: usize,
}

fn head_of(raw: &[u8]) -> Option<Head> {
    let mut scan = Scan::new(raw);
    let mut named = false;
    while scan.peek().is_some_and(is_name_byte) {
        scan.bump();
        named = true;
    }
    if !named {
        return None;
    }
    let name_end = scan.pos;
    let separator = scan.peek().filter(|b| matches!(b, b';' | b':'))?;
    scan.bump();
    Some(Head {
        name_end,
        separator,
        rest_start: scan.pos,
    })
}

// ── Reading ─────────────────────────────────────────────────────────────

/// A reader over the properties of one component.
#[derive(Debug, Clone, Copy)]
pub struct ContentLineReader<'a> {
    input: &'a [u8],
    pos: usize,
    component: &'static str,
    /// How many nested components the walk is inside (`VALARM` in `VEVENT`).
    depth: u32,
}

impl<'a> ContentLineReader<'a> {
    /// Skip lines up to the first `BEGIN:<component>` and stand after it.
    /// An input that ends before it is `NeedMoreBytes`.
    pub fn begin(input: &'a [u8], component: &'static str) -> Result<Self, CodecError> {
        let mut pos = 0;
        while let Some(line) = next_raw_line(input, &mut pos) {
            if !line.terminated {
                break;
            }
            let Some(head) = head_of(line.raw) else {
                continue;
            };
            if head.separator == b':'
                && unfolded_eq(&line.raw[..head.name_end], "BEGIN")
                && unfolded_eq(&line.raw[head.rest_start..], component)
            {
                return Ok(Self {
                    input,
                    pos,
                    component,
                    depth: 0,
                });
            }
        }
        Err(CodecError::NeedMoreBytes)
    }

    /// How many bytes of the input the walk has passed — after
    /// `END:<component>` once [`next_property`](Self::next_property) has
    /// returned `None`.
    pub fn consumed(&self) -> usize {
        self.pos
    }

    /// The next property line of the component, or `None` after its
    /// `END:<component>`.
    ///
    /// A nested component is skipped through its `END:` without reading its
    /// lines. An input that ends before `END:<component>` is `NeedMoreBytes`.
    pub fn next_property(&mut self) -> Result<Option<Property<'a>>, CodecError> {
        loop {
            let Some(line) = next_raw_line(self.input, &mut self.pos) else {
                return Err(CodecError::NeedMoreBytes);
            };
            let head = head_of(line.raw);
            if self.depth > 0 {
                if let Some(h) = head.filter(|h| h.separator == b':') {
                    let name = &line.raw[..h.name_end];
                    if unfolded_eq(name, "BEGIN") {
                        self.depth += 1;
                    } else if unfolded_eq(name, "END") {
                        self.depth -= 1;
                    }
                }
                continue;
            }
            let Some(head) = head else {
                // A line with no name is cut short at the end of the input and
                // malformed anywhere else.
                return Err(if line.terminated {
                    CodecError::LineMalformed
                } else {
                    CodecError::NeedMoreBytes
                });
            };
            if head.separator == b':' {
                let name = &line.raw[..head.name_end];
                if unfolded_eq(name, "END") {
                    if unfolded_eq(&line.raw[head.rest_start..], self.component) {
                        return Ok(None);
                    }
                    return Err(if line.terminated {
                        CodecError::LineMalformed
                    } else {
                        CodecError::NeedMoreBytes
                    });
                }
                if line.terminated && unfolded_eq(name, "BEGIN") {
                    self.depth = 1;
                    continue;
                }
            }
            if !line.terminated {
                return Err(CodecError::NeedMoreBytes);
            }
            return Ok(Some(Property::new(line.raw, &head)));
        }
    }
}

/// Where a [`Property`] stands in its line.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Phase {
    /// At the `;` that opens a parameter or the `:` that opens the value.
    AtSeparator,
    /// After a parameter's `=`, before its value.
    ParamValue,
    /// The value has been read.
    Done,
}

/// One property line the codec has been handed.
///
/// Ask [`is`](Self::is) whether it is one the codec reads. Then, for a property
/// that declares parameters, loop on [`next_param`](Self::next_param) and read
/// the ones the codec declares; then read the value. A parameter no one reads is
/// skipped by the next call, and a property read by value alone skips them all.
#[derive(Debug, Clone, Copy)]
pub struct Property<'a> {
    scan: Scan<'a>,
    name_end: usize,
    param: Option<(usize, usize)>,
    phase: Phase,
}

impl<'a> Property<'a> {
    fn new(raw: &'a [u8], head: &Head) -> Self {
        let mut scan = Scan::new(raw);
        // Stand on the separator, which `head_of` read past.
        scan.pos = head.name_end;
        Self {
            scan,
            name_end: head.name_end,
            param: None,
            phase: Phase::AtSeparator,
        }
    }

    /// Whether this property is `name`, without regard to case.
    pub fn is(&self, name: &str) -> bool {
        unfolded_eq(&self.scan.raw[..self.name_end], name)
    }

    /// Whether the parameter [`next_param`](Self::next_param) stands on is
    /// `name`, without regard to case.
    pub fn param_is(&self, name: &str) -> bool {
        self.param
            .is_some_and(|(start, end)| unfolded_eq(&self.scan.raw[start..end], name))
    }

    /// Stand on the next parameter, skipping the value of one not read; `false`
    /// when the value is next.
    pub fn next_param(&mut self) -> Result<bool, CodecError> {
        match self.phase {
            Phase::ParamValue => self.skip_param_value()?,
            Phase::Done => return Err(CodecError::LineMalformed),
            Phase::AtSeparator => {}
        }
        match self.scan.peek() {
            Some(b':') => Ok(false),
            Some(b';') => {
                self.scan.bump();
                self.scan.peek();
                let start = self.scan.pos;
                while self.scan.peek().is_some_and(is_name_byte) {
                    self.scan.bump();
                }
                let end = self.scan.pos;
                if end == start || self.scan.bump() != Some(b'=') {
                    return Err(CodecError::LineMalformed);
                }
                self.param = Some((start, end));
                self.phase = Phase::ParamValue;
                Ok(true)
            }
            _ => Err(CodecError::LineMalformed),
        }
    }

    /// Scan one parameter value — a quoted string, or text up to `;`, `:`, `,`
    /// or `"` — handing each byte of it to `emit`. Returns whether another value
    /// follows a `,`.
    fn scan_param_value(
        &mut self,
        mut emit: impl FnMut(u8) -> Result<(), CodecError>,
    ) -> Result<bool, CodecError> {
        if self.scan.peek() == Some(b'"') {
            self.scan.bump();
            loop {
                match self.scan.bump() {
                    None => return Err(CodecError::LineMalformed),
                    Some(b'"') => break,
                    Some(b) => emit(b)?,
                }
            }
        } else {
            while let Some(b) = self.scan.peek() {
                match b {
                    b';' | b':' | b',' => break,
                    b'"' => return Err(CodecError::LineMalformed),
                    _ => {
                        self.scan.bump();
                        emit(b)?;
                    }
                }
            }
        }
        match self.scan.peek() {
            Some(b',') => {
                self.scan.bump();
                Ok(true)
            }
            Some(b';' | b':') => Ok(false),
            _ => Err(CodecError::LineMalformed),
        }
    }

    fn skip_param_value(&mut self) -> Result<(), CodecError> {
        while self.scan_param_value(|_| Ok(()))? {}
        self.param = None;
        self.phase = Phase::AtSeparator;
        Ok(())
    }

    /// Read the value of the parameter [`next_param`](Self::next_param) stands
    /// on, into at most `N` bytes. A second value is `LineBadValue`.
    pub fn read_param_string<const N: usize>(&mut self) -> Result<BoundedString<N>, CodecError> {
        if self.phase != Phase::ParamValue {
            return Err(CodecError::LineMalformed);
        }
        let mut buf: BoundedVec<u8, N> = BoundedVec::new();
        let more = self.scan_param_value(|b| {
            if is_control(b) {
                return Err(CodecError::LineBadValue);
            }
            buf.push(b).map_err(|_| CodecError::LineTooLong)
        })?;
        if more {
            return Err(CodecError::LineBadValue);
        }
        self.param = None;
        self.phase = Phase::AtSeparator;
        BoundedString::from_utf8(buf).map_err(|_| CodecError::LineBadValue)
    }

    /// Stand at the value: skip the parameters still unread and step over `:`.
    fn begin_value(&mut self) -> Result<(), CodecError> {
        while self.next_param()? {}
        self.scan.bump();
        self.phase = Phase::Done;
        Ok(())
    }

    /// Read the value as a `string` of at most `N` bytes. With `text`, `\\`,
    /// `\;`, `\,`, `\n` and `\N` are escapes; an unescaped `;` or `,` is itself.
    pub fn read_string<const N: usize>(
        &mut self,
        text: bool,
    ) -> Result<BoundedString<N>, CodecError> {
        self.begin_value()?;
        let mut buf: BoundedVec<u8, N> = BoundedVec::new();
        while let Some(b) = self.scan.bump() {
            let byte = if text && b == b'\\' {
                match self.scan.bump() {
                    Some(b'\\') => b'\\',
                    Some(b';') => b';',
                    Some(b',') => b',',
                    Some(b'n' | b'N') => b'\n',
                    _ => return Err(CodecError::LineBadEscape),
                }
            } else if is_control(b) {
                return Err(CodecError::LineBadValue);
            } else {
                b
            };
            buf.push(byte).map_err(|_| CodecError::LineTooLong)?;
        }
        BoundedString::from_utf8(buf).map_err(|_| CodecError::LineBadValue)
    }

    /// The decimal the rest of the value is: an optional sign, digits. A `-` is
    /// read only when `allow_minus`, so an unsigned type refuses `-0` as well.
    fn read_decimal(&mut self, allow_minus: bool) -> Result<i128, CodecError> {
        self.begin_value()?;
        let mut next = self.scan.bump();
        let negative = next == Some(b'-');
        if negative && !allow_minus {
            return Err(CodecError::LineBadValue);
        }
        if negative || next == Some(b'+') {
            next = self.scan.bump();
        }
        let mut magnitude: i128 = 0;
        let mut digits = 0u32;
        while let Some(d) = next {
            if !d.is_ascii_digit() {
                return Err(CodecError::LineBadValue);
            }
            magnitude = magnitude
                .checked_mul(10)
                .and_then(|m| m.checked_add(i128::from(d - b'0')))
                .ok_or(CodecError::LineBadValue)?;
            digits += 1;
            next = self.scan.bump();
        }
        if digits == 0 {
            return Err(CodecError::LineBadValue);
        }
        Ok(if negative { -magnitude } else { magnitude })
    }

    /// Read the value as an unsigned integer of at most `max`.
    pub fn read_uint(&mut self, max: u64) -> Result<u64, CodecError> {
        let value = self.read_decimal(false)?;
        u64::try_from(value)
            .ok()
            .filter(|v| *v <= max)
            .ok_or(CodecError::LineBadValue)
    }

    /// Read the value as a signed integer within `min..=max`.
    pub fn read_int(&mut self, min: i64, max: i64) -> Result<i64, CodecError> {
        let value = self.read_decimal(true)?;
        i64::try_from(value)
            .ok()
            .filter(|v| (min..=max).contains(v))
            .ok_or(CodecError::LineBadValue)
    }

    /// Read the value as `TRUE` or `FALSE`, in either case.
    pub fn read_bool(&mut self) -> Result<bool, CodecError> {
        self.begin_value()?;
        let mut word = [0u8; 5];
        let mut n = 0;
        while let Some(b) = self.scan.bump() {
            if n == word.len() {
                return Err(CodecError::LineBadValue);
            }
            word[n] = b.to_ascii_uppercase();
            n += 1;
        }
        match &word[..n] {
            b"TRUE" => Ok(true),
            b"FALSE" => Ok(false),
            _ => Err(CodecError::LineBadValue),
        }
    }
}

// ── Writing ─────────────────────────────────────────────────────────────

/// A writer of the lines of one component into a sink.
///
/// A property is written by [`property`](Self::property), then each present
/// parameter by [`param`](Self::param), then the value by one of the value
/// methods, which ends the line.
pub struct ContentLineWriter<'s, S: SceSink + ?Sized> {
    sink: &'s mut S,
    component: &'static str,
    /// Octets already on the current physical line.
    column: usize,
}

impl<'s, S: SceSink + ?Sized> ContentLineWriter<'s, S> {
    /// Write `BEGIN:<component>`.
    pub fn begin(sink: &'s mut S, component: &'static str) -> Result<Self, CodecError> {
        sink.write_bytes(b"BEGIN:")?;
        sink.write_bytes(component.as_bytes())?;
        sink.write_bytes(b"\r\n")?;
        Ok(Self {
            sink,
            component,
            column: 0,
        })
    }

    /// Write `END:<component>`.
    pub fn finish(self) -> Result<(), CodecError> {
        self.sink.write_bytes(b"END:")?;
        self.sink.write_bytes(self.component.as_bytes())?;
        self.sink.write_bytes(b"\r\n")
    }

    /// Write one unit — a character, or an escape — on the current line, after
    /// cutting the line if it would pass [`FOLD_WIDTH`] octets.
    fn unit(&mut self, bytes: &[u8]) -> Result<(), CodecError> {
        if self.column + bytes.len() > FOLD_WIDTH {
            self.sink.write_bytes(b"\r\n ")?;
            self.column = 1;
        }
        self.sink.write_bytes(bytes)?;
        self.column += bytes.len();
        Ok(())
    }

    fn units(&mut self, text: &str) -> Result<(), CodecError> {
        for c in text.chars() {
            let mut buf = [0u8; 4];
            self.unit(c.encode_utf8(&mut buf).as_bytes())?;
        }
        Ok(())
    }

    fn end_line(&mut self) -> Result<(), CodecError> {
        self.column = 0;
        self.sink.write_bytes(b"\r\n")
    }

    /// Start a property's line with its name.
    pub fn property(&mut self, name: &str) -> Result<(), CodecError> {
        self.column = 0;
        self.units(name)
    }

    /// Write `;<name>=<value>`, quoting the value when it holds `:`, `;` or `,`.
    /// A value past `max_size` is `LineTooLong`; one with a control character or a
    /// `"` is `LineBadValue`.
    pub fn param(&mut self, name: &str, value: &str, max_size: usize) -> Result<(), CodecError> {
        if value.len() > max_size {
            return Err(CodecError::LineTooLong);
        }
        if value.bytes().any(|b| b == b'"' || is_control(b)) {
            return Err(CodecError::LineBadValue);
        }
        let quoted = value.bytes().any(|b| matches!(b, b':' | b';' | b','));
        self.unit(b";")?;
        self.units(name)?;
        self.unit(b"=")?;
        if quoted {
            self.unit(b"\"")?;
        }
        self.units(value)?;
        if quoted {
            self.unit(b"\"")?;
        }
        Ok(())
    }

    /// Write `:<value>` and end the line. With `text`, `\`, `;`, `,` and a line
    /// feed are written as escapes. A value past `max_size` is `LineTooLong`; one
    /// with a control character (but a TEXT's line feed) is `LineBadValue`.
    pub fn string(&mut self, value: &str, text: bool, max_size: usize) -> Result<(), CodecError> {
        if value.len() > max_size {
            return Err(CodecError::LineTooLong);
        }
        if value
            .bytes()
            .any(|b| is_control(b) && !(text && b == b'\n'))
        {
            return Err(CodecError::LineBadValue);
        }
        self.unit(b":")?;
        if text {
            for c in value.chars() {
                let mut buf = [0u8; 4];
                match c {
                    '\\' => self.unit(b"\\\\")?,
                    ';' => self.unit(b"\\;")?,
                    ',' => self.unit(b"\\,")?,
                    '\n' => self.unit(b"\\n")?,
                    _ => self.unit(c.encode_utf8(&mut buf).as_bytes())?,
                }
            }
        } else {
            self.units(value)?;
        }
        self.end_line()
    }

    fn digits(&mut self, negative: bool, mut magnitude: u64) -> Result<(), CodecError> {
        let mut buf = [0u8; 21];
        let mut at = buf.len();
        loop {
            at -= 1;
            buf[at] = b'0' + (magnitude % 10) as u8;
            magnitude /= 10;
            if magnitude == 0 {
                break;
            }
        }
        if negative {
            at -= 1;
            buf[at] = b'-';
        }
        self.unit(b":")?;
        for b in &buf[at..] {
            self.unit(core::slice::from_ref(b))?;
        }
        self.end_line()
    }

    /// Write `:<value>` as decimal digits and end the line.
    pub fn uint(&mut self, value: u64) -> Result<(), CodecError> {
        self.digits(false, value)
    }

    /// Write `:<value>` as decimal digits, `-` first when negative, and end the
    /// line.
    pub fn int(&mut self, value: i64) -> Result<(), CodecError> {
        self.digits(value < 0, value.unsigned_abs())
    }

    /// Write `:TRUE` or `:FALSE` and end the line.
    pub fn boolean(&mut self, value: bool) -> Result<(), CodecError> {
        self.unit(b":")?;
        for b in if value { &b"TRUE"[..] } else { &b"FALSE"[..] } {
            self.unit(core::slice::from_ref(b))?;
        }
        self.end_line()
    }
}

#[cfg(test)]
mod tests {
    extern crate std;
    use std::format;
    use std::string::String;
    use std::vec::Vec;

    use super::*;
    use crate::codec::{SceCursor, SliceSink};

    type Text<const N: usize> = BoundedString<N>;

    fn read_all(input: &[u8]) -> Result<Vec<(String, String)>, CodecError> {
        let mut r = ContentLineReader::begin(input, "VEVENT")?;
        let mut out = Vec::new();
        while let Some(mut p) = r.next_property()? {
            let name = if p.is("UID") {
                "UID"
            } else if p.is("SUMMARY") {
                "SUMMARY"
            } else if p.is("DTSTART") {
                "DTSTART"
            } else {
                continue;
            };
            let value = if name == "SUMMARY" {
                p.read_string::<32>(true)?
            } else {
                p.read_string::<32>(false)?
            };
            out.push((name.into(), value.as_str().into()));
        }
        Ok(out)
    }

    fn one(value: &[u8]) -> Result<Vec<(String, String)>, CodecError> {
        let mut input = Vec::from(&b"BEGIN:VEVENT\r\nSUMMARY:"[..]);
        input.extend_from_slice(value);
        input.extend_from_slice(b"\r\nEND:VEVENT\r\n");
        read_all(&input)
    }

    fn written(f: impl FnOnce(&mut ContentLineWriter<'_, SliceSink<'_>>)) -> String {
        let mut buf = [0u8; 512];
        let mut sink = SliceSink::new(&mut buf);
        {
            let mut w = ContentLineWriter::begin(&mut sink, "VEVENT").expect("begin");
            f(&mut w);
            w.finish().expect("finish");
        }
        String::from_utf8(sink.into_written().to_vec()).expect("utf-8")
    }

    #[test]
    fn a_component_is_read_property_by_property_and_the_cursor_ends_after_it() {
        let input = b"BEGIN:VCALENDAR\r\nVERSION:2.0\r\nBEGIN:VEVENT\r\nUID:a1\r\nSUMMARY:hi\r\nEND:VEVENT\r\nEND:VCALENDAR\r\n";
        let mut r = ContentLineReader::begin(input, "VEVENT").expect("begin");
        let mut seen = 0;
        while let Some(p) = r.next_property().expect("property") {
            assert!(p.is("UID") || p.is("SUMMARY"));
            seen += 1;
        }
        assert_eq!(seen, 2);
        let rest = &input[r.consumed()..];
        assert_eq!(rest, b"END:VCALENDAR\r\n");
    }

    #[test]
    fn names_are_matched_without_regard_to_case_and_lf_ends_a_line() {
        let got = read_all(b"begin:vevent\nuid:x\nend:Vevent\n").expect("read");
        assert_eq!(got, [("UID".into(), "x".into())]);
    }

    #[test]
    fn a_folded_value_and_a_folded_name_are_unfolded_wherever_the_sender_cut() {
        let got =
            read_all(b"BEGIN:VEVENT\r\nUI\r\n D:ab\r\n\tcd\r\n e\r\nEND:VEVENT\r\n").expect("read");
        assert_eq!(got, [("UID".into(), "abcde".into())]);
        // A cut inside a UTF-8 character is whole once unfolded.
        let mut raw = Vec::from(&b"BEGIN:VEVENT\r\nUID:"[..]);
        raw.push(0xC3);
        raw.extend_from_slice(b"\r\n ");
        raw.push(0xA9);
        raw.extend_from_slice(b"\r\nEND:VEVENT\r\n");
        assert_eq!(
            read_all(&raw).expect("read"),
            [("UID".into(), "\u{e9}".into())]
        );
    }

    #[test]
    fn a_nested_component_and_a_property_nobody_declared_are_skipped() {
        let got = read_all(
            b"BEGIN:VEVENT\r\nX-NOISE;A=\"b:c\":d\r\nBEGIN:VALARM\r\nUID:inner\r\nBEGIN:X\r\nEND:X\r\nEND:VALARM\r\nUID:outer\r\nEND:VEVENT\r\n",
        )
        .expect("read");
        assert_eq!(got, [("UID".into(), "outer".into())]);
    }

    #[test]
    fn an_input_that_ends_early_needs_more_bytes() {
        for input in [
            &b""[..],
            b"BEGIN:VCALENDAR\r\n",
            b"BEGIN:VEVENT",
            b"BEGIN:VEVENT\r\nUID:a",
            b"BEGIN:VEVENT\r\nUID:a\r\n",
            b"BEGIN:VEVENT\r\nUID:a\r\nEND:VEVEN",
            b"BEGIN:VEVENT\r\nBEGIN:VALARM\r\nEND:VALARM\r\n",
        ] {
            assert_eq!(read_all(input), Err(CodecError::NeedMoreBytes), "{input:?}");
        }
        // The END line alone needs no line break after it.
        assert!(read_all(b"BEGIN:VEVENT\r\nUID:a\r\nEND:VEVENT").is_ok());
    }

    #[test]
    fn a_line_the_grammar_does_not_admit_is_malformed() {
        for input in [
            &b"BEGIN:VEVENT\r\n\r\nEND:VEVENT\r\n"[..],
            b"BEGIN:VEVENT\r\nUID\r\nEND:VEVENT\r\n",
            b"BEGIN:VEVENT\r\n=x:y\r\nEND:VEVENT\r\n",
            b"BEGIN:VEVENT\r\nUID:a\r\nEND:VTODO\r\n",
            b"BEGIN:VEVENT\r\nUID;TZID:a\r\nEND:VEVENT\r\n",
            b"BEGIN:VEVENT\r\nUID;TZID=\"a:b\r\nEND:VEVENT\r\n",
            b"BEGIN:VEVENT\r\nUID;TZID=\"a\"b:c\r\nEND:VEVENT\r\n",
            b"BEGIN:VEVENT\r\nUID;TZID=a\"b:c\r\nEND:VEVENT\r\n",
        ] {
            assert_eq!(read_all(input), Err(CodecError::LineMalformed), "{input:?}");
        }
    }

    #[test]
    fn a_text_unescapes_and_a_bad_escape_is_refused() {
        assert_eq!(
            one(b"a\\\\b\\;c\\,d\\ne\\Nf;g,h").expect("read"),
            [("SUMMARY".into(), "a\\b;c,d\ne\nf;g,h".into())]
        );
        for bad in [&b"a\\xb"[..], b"a\\"] {
            assert_eq!(one(bad), Err(CodecError::LineBadEscape), "{bad:?}");
        }
        // Without sce:value="text" a backslash is a byte like another.
        let mut r =
            ContentLineReader::begin(b"BEGIN:VEVENT\r\nUID:a\\nb\r\nEND:VEVENT\r\n", "VEVENT")
                .expect("begin");
        let mut p = r.next_property().expect("line").expect("property");
        assert_eq!(p.read_string::<8>(false).expect("read").as_str(), "a\\nb");
    }

    #[test]
    fn a_control_character_a_bad_utf8_value_and_a_long_value_are_refused() {
        assert_eq!(one(b"a\x01b"), Err(CodecError::LineBadValue));
        assert_eq!(one(b"a\rb"), Err(CodecError::LineBadValue));
        assert_eq!(one(b"a\x7fb"), Err(CodecError::LineBadValue));
        assert_eq!(one(b"a\xffb"), Err(CodecError::LineBadValue));
        assert!(one(b"a\tb").is_ok());
        let long = [b'x'; 33];
        assert_eq!(one(&long), Err(CodecError::LineTooLong));
        assert!(one(&long[..32]).is_ok());
        // The size is that of the unescaped text.
        let escaped = b"\\;".repeat(32);
        assert!(one(&escaped).is_ok());
    }

    #[test]
    fn parameters_are_read_by_name_quoted_or_not_and_the_rest_are_skipped() {
        let input = b"BEGIN:VEVENT\r\nDTSTART;X-A=1;TZID=\"Europe/Seoul:KST\";X-B=\"p,q\",r:20260101T000000\r\nEND:VEVENT\r\n";
        let mut r = ContentLineReader::begin(input, "VEVENT").expect("begin");
        let mut p = r.next_property().expect("line").expect("property");
        assert!(p.is("dtstart"));
        let mut tzid = None;
        while p.next_param().expect("param") {
            if p.param_is("tzid") {
                tzid = Some(p.read_param_string::<32>().expect("tzid"));
            }
        }
        assert_eq!(tzid.expect("tzid").as_str(), "Europe/Seoul:KST");
        assert_eq!(
            p.read_string::<32>(false).expect("value").as_str(),
            "20260101T000000"
        );
        assert!(r.next_property().expect("end").is_none());
    }

    #[test]
    fn a_parameter_of_two_values_or_past_its_bound_is_refused_where_it_is_read() {
        for (param, want) in [
            (&b"TZID=a,b"[..], CodecError::LineBadValue),
            (b"TZID=\"a\",\"b\"", CodecError::LineBadValue),
            (b"TZID=abcdefghi", CodecError::LineTooLong),
            (b"TZID=a\x01", CodecError::LineBadValue),
        ] {
            let mut input = Vec::from(&b"BEGIN:VEVENT\r\nDTSTART;"[..]);
            input.extend_from_slice(param);
            input.extend_from_slice(b":x\r\nEND:VEVENT\r\n");
            let mut r = ContentLineReader::begin(&input, "VEVENT").expect("begin");
            let mut p = r.next_property().expect("line").expect("property");
            assert!(p.next_param().expect("param"));
            assert_eq!(p.read_param_string::<8>(), Err(want), "{param:?}");
        }
    }

    #[test]
    fn integers_and_booleans_hold_their_type_or_are_refused() {
        /// Read the value `text` of a property `N` with `read`.
        fn on<T>(text: &[u8], read: impl FnOnce(&mut Property<'_>) -> T) -> T {
            let mut input = Vec::from(&b"BEGIN:VEVENT\r\nN:"[..]);
            input.extend_from_slice(text);
            input.extend_from_slice(b"\r\nEND:VEVENT\r\n");
            let mut r = ContentLineReader::begin(&input, "VEVENT").expect("begin");
            let mut p = r.next_property().expect("line").expect("property");
            read(&mut p)
        }
        let bad = CodecError::LineBadValue;
        assert_eq!(on(b"255", |p| p.read_uint(255)), Ok(255));
        assert_eq!(on(b"+007", |p| p.read_uint(255)), Ok(7));
        assert_eq!(on(b"256", |p| p.read_uint(255)), Err(bad));
        assert_eq!(on(b"-0", |p| p.read_uint(255)), Err(bad));
        assert_eq!(on(b"", |p| p.read_uint(255)), Err(bad));
        assert_eq!(on(b"1x", |p| p.read_uint(255)), Err(bad));
        assert_eq!(on(b" 1", |p| p.read_uint(255)), Err(bad));
        assert_eq!(
            on(b"99999999999999999999999999999999999999999", |p| p
                .read_uint(u64::MAX)),
            Err(bad)
        );
        assert_eq!(
            on(b"18446744073709551615", |p| p.read_uint(u64::MAX)),
            Ok(u64::MAX)
        );
        assert_eq!(on(b"-128", |p| p.read_int(-128, 127)), Ok(-128));
        assert_eq!(on(b"-129", |p| p.read_int(-128, 127)), Err(bad));
        assert_eq!(
            on(b"-9223372036854775808", |p| p.read_int(i64::MIN, i64::MAX)),
            Ok(i64::MIN)
        );
        assert_eq!(on(b"true", |p| p.read_bool()), Ok(true));
        assert_eq!(on(b"False", |p| p.read_bool()), Ok(false));
        assert_eq!(on(b"yes", |p| p.read_bool()), Err(CodecError::LineBadValue));
        assert_eq!(
            on(b"TRUEE", |p| p.read_bool()),
            Err(CodecError::LineBadValue)
        );
    }

    #[test]
    fn a_value_is_read_once() {
        let mut r = ContentLineReader::begin(b"BEGIN:VEVENT\r\nUID:a\r\nEND:VEVENT\r\n", "VEVENT")
            .expect("begin");
        let mut p = r.next_property().expect("line").expect("property");
        p.read_string::<4>(false).expect("first");
        assert_eq!(p.read_string::<4>(false), Err(CodecError::LineMalformed));
    }

    #[test]
    fn a_decode_through_a_cursor_leaves_it_after_the_component() {
        let wire = b"BEGIN:VEVENT\r\nUID:a\r\nEND:VEVENT\r\nTAIL";
        let mut cursor = SceCursor::new(wire);
        let rest = cursor.peek_slice(cursor.remaining()).expect("rest");
        let mut r = ContentLineReader::begin(rest, "VEVENT").expect("begin");
        while r.next_property().expect("property").is_some() {}
        cursor.advance(r.consumed()).expect("advance");
        assert_eq!(cursor.remaining(), 4);
    }

    #[test]
    fn a_property_is_written_with_its_parameters_and_a_value_that_is_escaped() {
        let text = written(|w| {
            w.property("UID").expect("name");
            w.string("a1", false, 8).expect("value");
            w.property("DTSTART").expect("name");
            w.param("TZID", "Europe/Seoul", 64).expect("param");
            w.param("X-Q", "a:b", 8).expect("quoted");
            w.string("20260101T000000", false, 32).expect("value");
            w.property("SUMMARY").expect("name");
            w.string("a\\b;c,d\ne", true, 32).expect("text");
            w.property("N").expect("name");
            w.uint(18446744073709551615).expect("uint");
            w.property("M").expect("name");
            w.int(i64::MIN).expect("int");
            w.property("B").expect("name");
            w.boolean(true).expect("bool");
        });
        assert_eq!(
            text,
            "BEGIN:VEVENT\r\nUID:a1\r\nDTSTART;TZID=Europe/Seoul;X-Q=\"a:b\":20260101T000000\r\n\
             SUMMARY:a\\\\b\\;c\\,d\\ne\r\nN:18446744073709551615\r\nM:-9223372036854775808\r\nB:TRUE\r\nEND:VEVENT\r\n"
        );
    }

    #[test]
    fn a_line_is_cut_before_the_unit_that_would_pass_75_octets() {
        let text = written(|w| {
            w.property("SUMMARY").expect("name");
            w.string(&"x".repeat(200), false, 256).expect("value");
        });
        let lines: Vec<&str> = text.split("\r\n").collect();
        assert_eq!(lines[0], "BEGIN:VEVENT");
        assert_eq!(lines[1].len(), 75);
        assert!(
            lines[2].starts_with(' ') && lines[2].len() == 75,
            "{}",
            lines[2]
        );
        // "SUMMARY:" takes 8 of the first 75 octets, leaving 67 of the 200; each
        // continuation line carries its space and 74 more.
        assert!(lines[3].starts_with(' ') && lines[3].len() == 1 + 200 - 67 - 74);
        assert_eq!(lines[4], "END:VEVENT");
    }

    #[test]
    fn a_cut_never_splits_a_character_or_an_escape() {
        // Seventy-three octets of name and colon and padding, then a two-octet
        // character that would end at 75 exactly and one that would pass it.
        let text = written(|w| {
            w.property("S").expect("name");
            w.string(&format!("{}\u{e9}\u{e9}", "x".repeat(71)), false, 256)
                .expect("value");
            w.property("T").expect("name");
            w.string(&format!("{};", "x".repeat(72)), true, 256)
                .expect("value");
        });
        assert!(
            text.contains(&format!("S:{}\u{e9}\r\n \u{e9}\r\n", "x".repeat(71))),
            "{text:?}"
        );
        assert!(
            text.contains(&format!("T:{}\r\n \\;\r\n", "x".repeat(72))),
            "{text:?}"
        );
        let mut round = Vec::from(text.as_bytes());
        round.extend_from_slice(b"");
        let mut r = ContentLineReader::begin(&round, "VEVENT").expect("begin");
        let mut s = r.next_property().expect("line").expect("property");
        assert_eq!(
            s.read_string::<80>(false).expect("s").as_str(),
            format!("{}\u{e9}\u{e9}", "x".repeat(71))
        );
        let mut t = r.next_property().expect("line").expect("property");
        assert_eq!(
            t.read_string::<80>(true).expect("t").as_str(),
            format!("{};", "x".repeat(72))
        );
    }

    #[test]
    fn a_value_a_line_could_not_carry_is_refused_before_it_is_written() {
        let mut buf = [0u8; 128];
        let mut sink = SliceSink::new(&mut buf);
        let mut w = ContentLineWriter::begin(&mut sink, "VEVENT").expect("begin");
        w.property("P").expect("name");
        assert_eq!(
            w.string("a\r\nATTENDEE:x", false, 64),
            Err(CodecError::LineBadValue)
        );
        assert_eq!(w.string("a\nb", false, 64), Err(CodecError::LineBadValue));
        assert_eq!(w.string("a\rb", true, 64), Err(CodecError::LineBadValue));
        assert_eq!(w.string("abcd", false, 3), Err(CodecError::LineTooLong));
        assert_eq!(w.param("Q", "a\"b", 8), Err(CodecError::LineBadValue));
        assert_eq!(w.param("Q", "a\nb", 8), Err(CodecError::LineBadValue));
        assert_eq!(w.param("Q", "abcd", 3), Err(CodecError::LineTooLong));
        // Nothing of a refused value reached the sink; the writer's borrow of
        // it ends at its last use above.
        assert_eq!(sink.position(), "BEGIN:VEVENT\r\nP".len());
    }

    #[test]
    fn a_full_sink_is_reported_as_the_sink_reports_it() {
        let mut buf = [0u8; 8];
        let mut sink = SliceSink::new(&mut buf);
        assert_eq!(
            ContentLineWriter::begin(&mut sink, "VEVENT").err(),
            Some(CodecError::BufferOverflow)
        );
    }

    #[test]
    fn what_is_written_is_read_back() {
        let values = [
            "",
            "plain",
            "a;b,c\\d\ne",
            "caf\u{e9} \u{1F600}",
            &"y".repeat(150),
        ];
        for value in values {
            let text = written(|w| {
                w.property("SUMMARY").expect("name");
                w.string(value, true, 256).expect("value");
            });
            let mut r = ContentLineReader::begin(text.as_bytes(), "VEVENT").expect("begin");
            let mut p = r.next_property().expect("line").expect("property");
            let got: Text<256> = p.read_string(true).expect("read");
            assert_eq!(got.as_str(), value);
            assert!(r.next_property().expect("end").is_none());
            assert_eq!(r.consumed(), text.len());
        }
    }
}
