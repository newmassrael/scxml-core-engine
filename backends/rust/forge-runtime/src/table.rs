// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! A cold codec read as a table of its fields, and a cold chain read by a table
//! of its rule (docs/adr/0155).
//!
//! A codec read once per entity, session or link event, and not on a build's
//! per-message path, costs nearly nothing per read and a whole printed body per
//! codec. Such a codec is generated as a [`FieldTable`]: one [`Row`] per field in
//! wire order, each saying how the field is read and what gates it. One walker,
//! [`read_table`], out of line and generic over nothing, reads every table; what
//! differs from codec to codec is the table and the stores of its value, which
//! the value says through [`TableOut`], one method per kind of store, each a
//! `match` on the row's index.
//!
//! The walker keeps the number each row read, so a gate, a length and the inputs
//! a body is handed are bit ranges ([`Bits`]) of rows already read or of the
//! inputs the codec itself is handed. A table is checked when it is made
//! ([`FieldTable::new`], in the `static` that holds it): every row reads only
//! rows before it, so the walker's reads are total.
//!
//! A chain of a cold codec whose kinds no gate reads is a [`ChainTable`]: how it
//! ends, its depth, the bit of an entry's identifier carrier that says another
//! entry follows, and the bits it carries to the next entry. One routine,
//! [`read_chain_table`], reads every such chain; the entry is read and held by a
//! [`ChainEntries`], which [`HeldEntries`] is for every list.
//!
//! Every read goes through the frame's reader (docs/adr/0128): it is total, the
//! first refusal recorded is the frame's verdict, and no read returns early.

use crate::chain::{close_chain, take_one, ChainEnd};
use crate::codec::{push_keeping, CodecError, NoKinds, SceSink};
use crate::reader::SceReader;
use crate::storage::HeldList;

/// The most rows a [`FieldTable`] holds: the size of the frame of numbers the
/// walker keeps while it reads one.
pub const TABLE_ROWS: usize = 32;

/// The most inputs a row hands the body it reads ([`Read::Body`]).
pub const ROW_INPUTS: usize = 4;

/// Where the number a [`Bits`] reads is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Source {
    /// The number the row at this index read: a row before the one that reads
    /// it, which [`FieldTable::new`] checks.
    Row(u8),
    /// The input at this index the codec is handed (a `<sce:flag-input>`), zero
    /// where the codec is handed fewer.
    Input(u8),
}

/// A bit range of a number: `(number >> shift) & mask`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Bits {
    /// Where the number is.
    pub from: Source,
    /// The lowest bit of the range.
    pub shift: u8,
    /// The range's bits once shifted down.
    pub mask: u64,
}

impl Bits {
    /// The whole number at `from`: a length, a count.
    pub const fn whole(from: Source) -> Self {
        Self {
            from,
            shift: 0,
            mask: u64::MAX,
        }
    }

    /// The `width` bits of the number at `from` from its bit `shift`: a flag
    /// of a carrier, a tag.
    pub const fn range(from: Source, shift: u8, width: u8) -> Self {
        let mask = if width >= 64 {
            u64::MAX
        } else {
            (1u64 << width) - 1
        };
        Self { from, shift, mask }
    }

    /// Whether the range reads only a row before the row at `at`, or an input.
    const fn before(self, at: usize) -> bool {
        match self.from {
            Source::Row(row) => (row as usize) < at,
            Source::Input(_) => true,
        }
    }

    /// The range's value, `frame` holding the numbers of the rows read so far
    /// and `inputs` the codec's inputs.
    fn of(self, frame: &[u64; TABLE_ROWS], inputs: &[u64]) -> u64 {
        let number = match self.from {
            Source::Row(row) => frame.get(usize::from(row)).copied().unwrap_or(0),
            Source::Input(input) => inputs.get(usize::from(input)).copied().unwrap_or(0),
        };
        number.checked_shr(u32::from(self.shift)).unwrap_or(0) & self.mask
    }
}

/// Whether a row is read (`sce:present-if` on a flag).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Gate {
    /// Always read.
    Always,
    /// Read where the range is not zero.
    Set(Bits),
    /// Read where the range is zero.
    Clear(Bits),
    /// Read where any of the clauses is open: `sce:present-if` of `a || b`,
    /// each clause a [`Gate::Set`] or a [`Gate::Clear`].
    Any(&'static [Gate]),
}

impl Gate {
    /// Whether the gate reads only rows before the row at `at`, or inputs,
    /// and a disjunction is of one clause or more, each a flag.
    const fn before(self, at: usize) -> bool {
        match self {
            Gate::Always => true,
            Gate::Set(bits) | Gate::Clear(bits) => bits.before(at),
            Gate::Any(clauses) => {
                let mut c = 0;
                while c < clauses.len() {
                    match clauses[c] {
                        Gate::Set(bits) | Gate::Clear(bits) => {
                            if !bits.before(at) {
                                return false;
                            }
                        }
                        Gate::Always | Gate::Any(_) => return false,
                    }
                    c += 1;
                }
                !clauses.is_empty()
            }
        }
    }

    /// Whether the row it gates is read.
    fn open(self, frame: &[u64; TABLE_ROWS], inputs: &[u64]) -> bool {
        match self {
            Gate::Always => true,
            Gate::Set(bits) => bits.of(frame, inputs) != 0,
            Gate::Clear(bits) => bits.of(frame, inputs) == 0,
            Gate::Any(clauses) => clauses.iter().any(|clause| clause.open(frame, inputs)),
        }
    }
}

/// How a row's field is read, and the number the row keeps for the rows after
/// it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Read {
    /// An unsigned integer of `bytes` bytes, 1 to 8, most significant first
    /// where `big`: a flags carrier, a fixed field. The row keeps the integer.
    Uint {
        /// How many bytes the integer is.
        bytes: u8,
        /// Whether its first byte is its most significant.
        big: bool,
    },
    /// A ZInt held to `bits`, 16, 32 or 64: a wider value is
    /// `VleWidthOverflow`. The row keeps the integer.
    Zint {
        /// The width the ZInt is held to.
        bits: u8,
    },
    /// The bytes whose count is `len`, at most `max`: a longer count is
    /// `LengthOutOfRange`. The row keeps the count.
    Bytes {
        /// The count.
        len: Bits,
        /// The most bytes the field holds.
        max: u64,
    },
    /// [`Read::Bytes`] that are UTF-8, or `InvalidUtf8`.
    Text {
        /// The count of bytes.
        len: Bits,
        /// The most bytes the field holds.
        max: u64,
    },
    /// What the value reads itself, handed `inputs` ([`TableOut::read`]): an
    /// embedded body, a variant arm by its tag, a list. The row keeps zero.
    Body {
        /// What the read is handed, at most [`ROW_INPUTS`].
        inputs: &'static [Bits],
    },
}

impl Read {
    /// Whether the read is one the walker makes, and reads only rows before
    /// the row at `at`, or inputs.
    const fn well_formed(self, at: usize) -> bool {
        match self {
            Read::Uint { bytes, .. } => matches!(bytes, 1..=8),
            Read::Zint { bits } => matches!(bits, 16 | 32 | 64),
            Read::Bytes { len, .. } | Read::Text { len, .. } => len.before(at),
            Read::Body { inputs } => {
                if inputs.len() > ROW_INPUTS {
                    return false;
                }
                let mut i = 0;
                while i < inputs.len() {
                    if !inputs[i].before(at) {
                        return false;
                    }
                    i += 1;
                }
                true
            }
        }
    }
}

/// One field of a [`FieldTable`]: how it is read and what gates it. Its place
/// in the value is its index in the table.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Row {
    /// How the field is read.
    pub read: Read,
    /// Whether it is read.
    pub gate: Gate,
}

/// What a number the encode derives is computed from (docs/adr/0058): a
/// length, a count, the flag a gated field is announced by, or a number the
/// value of an embedded body answers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Derived {
    /// 1 where the field of this row is present, 0 where it is not.
    Present(u8),
    /// The count of bytes of the field of this row ([`Read::Bytes`],
    /// [`Read::Text`]).
    Count(u8),
    /// The number `what` the field of the row `row` answers
    /// ([`TableSource::ask`]): the flag of the arm an embedded variant holds, an
    /// input an embedded body is read with.
    Ask {
        /// The row of the field asked.
        row: u8,
        /// Which of its numbers.
        what: u8,
    },
}

/// A number the encode writes into a row's bits rather than reading it from the
/// value: `into` (of a [`Source::Row`]) is set to what `from` computes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Derive {
    /// The bits of the row it is written into.
    pub into: Bits,
    /// What it is computed from.
    pub from: Derived,
}

impl Derive {
    /// Whether the derive writes into a row of a table of `rows` rows and is
    /// computed from rows of it.
    const fn within(self, rows: usize) -> bool {
        let into = match self.into.from {
            Source::Row(row) => (row as usize) < rows,
            Source::Input(_) => false,
        };
        let from = match self.from {
            Derived::Present(row) | Derived::Count(row) | Derived::Ask { row, .. } => {
                (row as usize) < rows
            }
        };
        into && from
    }
}

/// The fields of a cold codec in wire order, read by [`read_table`] and written
/// by [`write_table`] (docs/adr/0155), and the numbers its encode derives. Made
/// by [`FieldTable::new`] or [`FieldTable::with_derived`] only, which check it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FieldTable {
    rows: &'static [Row],
    derived: &'static [Derive],
}

impl FieldTable {
    /// The table of `rows`, checked: at most [`TABLE_ROWS`] rows, each read one
    /// the walker makes, and every number a row reads that of a row before it.
    /// Made in the `static` that holds a codec's table, so a table that breaks
    /// one of these does not compile.
    pub const fn new(rows: &'static [Row]) -> Self {
        Self::with_derived(rows, &[])
    }

    /// [`Self::new`] of a codec whose encode derives the numbers `derived`, each
    /// checked to write into a row of the table and to be computed from rows of
    /// it.
    pub const fn with_derived(rows: &'static [Row], derived: &'static [Derive]) -> Self {
        assert!(
            rows.len() <= TABLE_ROWS,
            "a field table holds at most TABLE_ROWS rows"
        );
        let mut at = 0;
        while at < rows.len() {
            assert!(
                rows[at].gate.before(at),
                "a row's gate reads a row before it or an input, each clause a flag"
            );
            assert!(
                rows[at].read.well_formed(at),
                "a row's read is one the walker makes, over rows before it or inputs"
            );
            at += 1;
        }
        let mut d = 0;
        while d < derived.len() {
            assert!(
                derived[d].within(rows.len()),
                "a derived number is written into a row of the table from rows of it"
            );
            d += 1;
        }
        Self { rows, derived }
    }

    /// The rows, in wire order.
    pub const fn rows(&self) -> &'static [Row] {
        self.rows
    }
}

/// The stores of a value a [`FieldTable`] is read into, by the index of each
/// row: the part of a cold codec's read that is the codec's own. A generated
/// codec implements it once for each form of its value. A method of a kind of
/// store no row of the codec's table makes is left as provided, which stores
/// nothing.
pub trait TableOut<'f> {
    /// The number the row `row` read ([`Read::Uint`], [`Read::Zint`]).
    fn number(&mut self, row: usize, value: u64) {
        let _ = (row, value);
    }

    /// The bytes the row `row` read ([`Read::Bytes`]), within the row's bound:
    /// `CapacityExceeded` where the value's storage does not hold them, which
    /// the walker notes and reads on (docs/adr/0077 §7); any other refusal is
    /// recorded as the frame's.
    fn bytes(&mut self, row: usize, bytes: &'f [u8]) -> Result<(), CodecError> {
        let _ = (row, bytes);
        Ok(())
    }

    /// The text the row `row` read ([`Read::Text`]), answered as
    /// [`Self::bytes`] answers.
    fn text(&mut self, row: usize, text: &'f str) -> Result<(), CodecError> {
        let _ = (row, text);
        Ok(())
    }

    /// Read the field of the row `row` from `reader`, handed `inputs`
    /// ([`Read::Body`]): the body, the arm or the list the value holds there,
    /// each read by its own function.
    fn read(&mut self, row: usize, reader: &mut SceReader<'f>, inputs: &[u64]) {
        let _ = (row, reader, inputs);
    }

    /// The field of the row `row` is not on the wire: its gate is closed.
    fn absent(&mut self, row: usize) {
        let _ = row;
    }
}

/// Read the fields of `table` from `reader` into `out`, the codec being handed
/// `inputs`: the read of every cold codec, one function for every table
/// (docs/adr/0155).
///
/// Each row is read in order where its gate is open, its value stored through
/// `out`, and its field is stored absent where it is closed. Every read is total
/// and the first refusal recorded in `reader` is the frame's verdict
/// (docs/adr/0128): a row past a refusal reads nothing and is stored as what a
/// total read answers.
#[inline(never)]
pub fn read_table<'f>(
    reader: &mut SceReader<'f>,
    table: &FieldTable,
    inputs: &[u64],
    out: &mut dyn TableOut<'f>,
) {
    let mut frame = [0u64; TABLE_ROWS];
    for (at, row) in table.rows.iter().enumerate() {
        let kept = if row.gate.open(&frame, inputs) {
            read_row(reader, row.read, at, &frame, inputs, out)
        } else {
            out.absent(at);
            0
        };
        if let Some(slot) = frame.get_mut(at) {
            *slot = kept;
        }
    }
}

/// Read one row's field and store it, answering the number the row keeps.
fn read_row<'f>(
    reader: &mut SceReader<'f>,
    read: Read,
    at: usize,
    frame: &[u64; TABLE_ROWS],
    inputs: &[u64],
    out: &mut dyn TableOut<'f>,
) -> u64 {
    match read {
        Read::Uint { bytes, big } => {
            let value = read_uint(reader, bytes, big);
            out.number(at, value);
            value
        }
        Read::Zint { bits } => {
            let value = match bits {
                16 => u64::from(reader.read_vle_u16()),
                32 => u64::from(reader.read_vle_u32()),
                _ => reader.read_vle_u64(),
            };
            out.number(at, value);
            value
        }
        Read::Bytes { len, max } => {
            let count = len.of(frame, inputs);
            let bytes = take_counted(reader, count, max);
            let held = out.bytes(at, bytes);
            reader.hold(held);
            count
        }
        Read::Text { len, max } => {
            let count = len.of(frame, inputs);
            let bytes = take_counted(reader, count, max);
            let text =
                reader.settle(core::str::from_utf8(bytes).map_err(|_| CodecError::InvalidUtf8));
            let held = out.text(at, text);
            reader.hold(held);
            count
        }
        Read::Body { inputs: wanted } => {
            let handed = body_inputs(wanted, frame, inputs);
            let count = wanted.len().min(ROW_INPUTS);
            out.read(at, reader, &handed[..count]);
            0
        }
    }
}

/// The numbers a body row hands its read or its write: each range `wanted`
/// names, over `frame` and the codec's `inputs`, the first [`ROW_INPUTS`].
fn body_inputs(wanted: &[Bits], frame: &[u64; TABLE_ROWS], inputs: &[u64]) -> [u64; ROW_INPUTS] {
    let mut handed = [0u64; ROW_INPUTS];
    for (slot, bits) in handed.iter_mut().zip(wanted) {
        *slot = bits.of(frame, inputs);
    }
    handed
}

/// An unsigned integer of `bytes` bytes, consumed; zero and `NeedMoreBytes`
/// recorded where fewer remain.
fn read_uint(reader: &mut SceReader<'_>, bytes: u8, big: bool) -> u64 {
    let raw = reader.take_slice(usize::from(bytes));
    if big {
        raw.iter().fold(0, |value, &b| (value << 8) | u64::from(b))
    } else {
        raw.iter()
            .rev()
            .fold(0, |value, &b| (value << 8) | u64::from(b))
    }
}

/// The next `count` bytes, consumed, `count` being at most `max`: a longer
/// count is `LengthOutOfRange`, recorded before the bytes are asked for, as a
/// printed read of a length-prefixed field records it.
fn take_counted<'f>(reader: &mut SceReader<'f>, count: u64, max: u64) -> &'f [u8] {
    if count > max {
        reader.fail(CodecError::LengthOutOfRange);
    }
    let n = reader.settle(usize::try_from(count).map_err(|_| CodecError::NeedMoreBytes));
    reader.take_slice(n)
}

/// What a value written by a [`FieldTable`] says, by the index of each row: the
/// part of a cold codec's write that is the codec's own, read off a source of
/// the value (docs/adr/0058). A generated codec implements it once over its
/// source; `E` is the error of the sink it writes to. A method of a kind of field
/// no row of the codec's table holds is left as provided.
pub trait TableSource<E: core::fmt::Debug> {
    /// The number of the row `row` ([`Read::Uint`], [`Read::Zint`]).
    fn number(&self, row: usize) -> u64 {
        let _ = row;
        0
    }

    /// Whether the field of the row `row` is present: what a
    /// [`Derived::Present`] computes. Whether a gated row is written is its
    /// gate's, as whether it is read is.
    fn present(&self, row: usize) -> bool {
        let _ = row;
        true
    }

    /// The bytes of the row `row` ([`Read::Bytes`]), or a text's UTF-8 bytes
    /// ([`Read::Text`]).
    fn bytes(&self, row: usize) -> &[u8] {
        let _ = row;
        &[]
    }

    /// The number `what` the field of the row `row` answers
    /// ([`Derived::Ask`]).
    fn ask(&self, row: usize, what: u8) -> u64 {
        let _ = (row, what);
        0
    }

    /// Write the field of the row `row` ([`Read::Body`]) into `sink`, handed
    /// `inputs`: the body, the arm or the list, each by its own codec's write.
    /// The inputs are the row's, read off the numbers the write derived, as
    /// [`TableOut::read`] is handed them off the numbers the read read.
    fn write(
        &self,
        row: usize,
        sink: &mut dyn SceSink<Error = E>,
        inputs: &[u64],
    ) -> Result<(), E> {
        let _ = (row, sink, inputs);
        Ok(())
    }
}

/// A sink a [`TableSource`] is handed, held as a sized value, so that a codec's
/// write generic over its sink writes through it: every write is the inner
/// sink's own.
pub struct DynSink<'s, E: core::fmt::Debug>(pub &'s mut dyn SceSink<Error = E>);

impl<E: core::fmt::Debug> SceSink for DynSink<'_, E> {
    type Error = E;

    fn write_bytes(&mut self, bytes: &[u8]) -> Result<(), E> {
        self.0.write_bytes(bytes)
    }

    fn position(&self) -> usize {
        self.0.position()
    }

    fn write_u8(&mut self, b: u8) -> Result<(), E> {
        self.0.write_u8(b)
    }

    fn write_u16_le(&mut self, v: u16) -> Result<(), E> {
        self.0.write_u16_le(v)
    }

    fn write_u16_be(&mut self, v: u16) -> Result<(), E> {
        self.0.write_u16_be(v)
    }

    fn write_u32_le(&mut self, v: u32) -> Result<(), E> {
        self.0.write_u32_le(v)
    }

    fn write_u32_be(&mut self, v: u32) -> Result<(), E> {
        self.0.write_u32_be(v)
    }

    fn write_u64_le(&mut self, v: u64) -> Result<(), E> {
        self.0.write_u64_le(v)
    }

    fn write_u64_be(&mut self, v: u64) -> Result<(), E> {
        self.0.write_u64_be(v)
    }

    fn write_vle_inner(&mut self, value: u64, max_bits: u32) -> Result<(), E> {
        self.0.write_vle_inner(value, max_bits)
    }

    fn write_vle_u16(&mut self, v: u16) -> Result<(), E> {
        self.0.write_vle_u16(v)
    }

    fn write_vle_u32(&mut self, v: u32) -> Result<(), E> {
        self.0.write_vle_u32(v)
    }

    fn write_vle_u64(&mut self, v: u64) -> Result<(), E> {
        self.0.write_vle_u64(v)
    }
}

/// Write the fields of `table` that `src` says into `sink`, the codec being
/// handed `inputs`: the write of every cold codec whose encode refuses no
/// value, one function for every table and sink error (docs/adr/0155 section
/// 3).
///
/// The number of each row is the value's, or the count of its bytes, with the
/// bits each of the table's derives computes written over it; then each row is
/// written in order where its gate is open over those numbers and `inputs`,
/// the one rule [`read_table`] reads it by, so a frame written is read back
/// row for row. The only error is the sink's (docs/adr/0065): a byte string or
/// a text is held to its row's bound by its type in every codec whose encode
/// refuses no value, which is the only codec a table is written for.
#[inline(never)]
pub fn write_table<E: core::fmt::Debug>(
    sink: &mut dyn SceSink<Error = E>,
    table: &FieldTable,
    inputs: &[u64],
    src: &dyn TableSource<E>,
) -> Result<(), E> {
    let mut frame = [0u64; TABLE_ROWS];
    for ((at, row), slot) in table.rows.iter().enumerate().zip(frame.iter_mut()) {
        *slot = match row.read {
            Read::Uint { .. } | Read::Zint { .. } => src.number(at),
            Read::Bytes { .. } | Read::Text { .. } => src.bytes(at).len() as u64,
            Read::Body { .. } => 0,
        };
    }
    for derive in table.derived {
        let value = match derive.from {
            Derived::Present(row) => u64::from(src.present(usize::from(row))),
            Derived::Count(row) => src.bytes(usize::from(row)).len() as u64,
            Derived::Ask { row, what } => src.ask(usize::from(row), what),
        };
        if let Source::Row(row) = derive.into.from {
            if let Some(slot) = frame.get_mut(usize::from(row)) {
                let shift = u32::from(derive.into.shift);
                let place = derive.into.mask.checked_shl(shift).unwrap_or(0);
                let bits = (value & derive.into.mask).checked_shl(shift).unwrap_or(0);
                *slot = (*slot & !place) | bits;
            }
        }
    }
    for (at, row) in table.rows.iter().enumerate() {
        if !row.gate.open(&frame, inputs) {
            continue;
        }
        let number = frame.get(at).copied().unwrap_or(0);
        match row.read {
            Read::Uint { bytes, big } => write_uint(sink, number, bytes, big)?,
            Read::Zint { bits } => sink.write_vle_inner(number, u32::from(bits))?,
            Read::Bytes { max, .. } | Read::Text { max, .. } => {
                let bytes = src.bytes(at);
                debug_assert!(
                    bytes.len() as u64 <= max,
                    "a byte string or a text written by a table is held to its row's bound"
                );
                sink.write_bytes(bytes)?;
            }
            Read::Body { inputs: wanted } => {
                let handed = body_inputs(wanted, &frame, inputs);
                let count = wanted.len().min(ROW_INPUTS);
                src.write(at, sink, &handed[..count])?;
            }
        }
    }
    Ok(())
}

/// An unsigned integer of `bytes` bytes, 1 to 8, in its byte order.
fn write_uint<E: core::fmt::Debug>(
    sink: &mut dyn SceSink<Error = E>,
    value: u64,
    bytes: u8,
    big: bool,
) -> Result<(), E> {
    let n = usize::from(bytes).min(8);
    if n == 1 {
        return sink.write_u8(value as u8);
    }
    if big {
        let all = value.to_be_bytes();
        sink.write_bytes(&all[8 - n..])
    } else {
        let all = value.to_le_bytes();
        sink.write_bytes(&all[..n])
    }
}

/// A bit range of an entry's identifier carrier: `(carrier >> shift) & mask`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Span {
    /// The lowest bit of the range.
    pub shift: u8,
    /// The range's bits once shifted down.
    pub mask: u64,
}

impl Span {
    /// The `width` bits of a carrier from its bit `shift`.
    pub const fn range(shift: u8, width: u8) -> Self {
        let mask = if width >= 64 {
            u64::MAX
        } else {
            (1u64 << width) - 1
        };
        Self { shift, mask }
    }

    /// The range's value in `carrier`.
    fn of(self, carrier: u64) -> u64 {
        carrier.checked_shr(u32::from(self.shift)).unwrap_or(0) & self.mask
    }
}

/// What a chain carries from one entry to the next (`<sce:entry-flag-bind>`): the
/// input `arg` of the next entry is the range `span` of this entry's carrier.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Carry {
    /// The index of the input the next entry is handed.
    pub arg: u8,
    /// The range of this entry's carrier it is.
    pub span: Span,
}

/// The rule of a chain of a cold codec whose kinds no gate reads, as data
/// (docs/adr/0155 section 5): what [`crate::chain::ChainRule`] says of a chain by
/// a type, read by [`read_chain_table`]. Made by [`ChainTable::new`] only,
/// which checks it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ChainTable {
    /// How the chain ends.
    end: ChainEnd,
    /// The most entries the chain holds, `None` where the protocol states no
    /// depth (docs/adr/0077 §7).
    depth: Option<u32>,
    /// The range of an entry's carrier that says another entry follows, read
    /// where the chain ends on a flag ([`ChainEnd::Flag`]).
    follows: Span,
    /// What the chain carries from each entry to the next.
    carry: &'static [Carry],
    /// How many inputs an entry is handed.
    args: u8,
}

impl ChainTable {
    /// The rule of a chain whose entries are handed `args` inputs, checked:
    /// each carry writes an input an entry is handed, and a range that fits the
    /// input's byte. Made in the `static` that holds it, so a rule that breaks
    /// one of these does not compile.
    pub const fn new(
        end: ChainEnd,
        depth: Option<u32>,
        follows: Span,
        carry: &'static [Carry],
        args: u8,
    ) -> Self {
        let mut c = 0;
        while c < carry.len() {
            assert!(
                carry[c].arg < args,
                "a chain carries into an input its entries are handed"
            );
            assert!(
                carry[c].span.mask <= u8::MAX as u64,
                "a chain carries a range an input's byte holds"
            );
            c += 1;
        }
        Self {
            end,
            depth,
            follows,
            carry,
            args,
        }
    }
}

/// The entries of one chain a [`ChainTable`] reads: how one is read and held,
/// and the number its rule is read off.
pub trait ChainEntries<'f> {
    /// Read one entry from `reader`, handed `args`, and hold it in the list as
    /// [`push_keeping`] holds an entry no gate asks a kind of, `full` noted where
    /// it is read and not held; answer the entry's identifier carrier, the number
    /// the chain's table reads.
    fn read_entry(&mut self, reader: &mut SceReader<'f>, args: &[u8], full: &mut bool) -> u64;
}

/// The [`ChainEntries`] of a list `L`: each entry read by `read` and its carrier
/// read off it by `carrier`. Generic over the list and its entry's read, so each
/// list has its own small copy of the hold; the chain's loop is
/// [`read_chain_table`], one function for every chain.
pub struct HeldEntries<'l, L, R, C> {
    list: &'l mut L,
    read: R,
    carrier: C,
}

impl<'l, L, R, C> HeldEntries<'l, L, R, C> {
    /// The entries of `list`, which a list's `refill` hands over cleared.
    pub fn new<'f>(list: &'l mut L, read: R, carrier: C) -> Self
    where
        L: HeldList,
        R: FnMut(&mut SceReader<'f>, &[u8]) -> L::Entry,
        C: Fn(&L::Entry) -> u64,
    {
        Self {
            list,
            read,
            carrier,
        }
    }
}

impl<'f, L, R, C> ChainEntries<'f> for HeldEntries<'_, L, R, C>
where
    L: HeldList,
    R: FnMut(&mut SceReader<'f>, &[u8]) -> L::Entry,
    C: Fn(&L::Entry) -> u64,
{
    fn read_entry(&mut self, reader: &mut SceReader<'f>, args: &[u8], full: &mut bool) -> u64 {
        let entry = (self.read)(reader, args);
        let carrier = (self.carrier)(&entry);
        push_keeping::<L, NoKinds>(self.list, entry, full);
        carrier
    }
}

/// Read the chain whose rule is `chain` from `reader` through `entries`, its
/// first entry handed `args`, which the chain rewrites as it carries
/// (docs/adr/0155 section 5): the loop of [`crate::chain::read_chain_into`], one
/// function for every chain whose rule is data.
///
/// Each entry is read until the chain ends, the loop being left where the frame
/// has failed (docs/adr/0128); the guards after the loop are the typed routine's.
/// The answer is `CapacityExceeded` where an entry was read and not held, which
/// the list's fill answers once the chain is read whole, and `Ok` otherwise.
#[inline(never)]
pub fn read_chain_table<'f>(
    reader: &mut SceReader<'f>,
    chain: &ChainTable,
    args: &mut [u8],
    entries: &mut dyn ChainEntries<'f>,
) -> Result<(), CodecError> {
    debug_assert!(
        args.len() >= usize::from(chain.args),
        "a chain is handed every input its entries are handed"
    );
    let with_frame = chain.end.with_frame();
    let mut left = chain.depth;
    let mut full = false;
    let mut more = false;
    while take_one(&mut left) {
        if with_frame && reader.remaining() == 0 {
            break;
        }
        if reader.failed() {
            break;
        }
        let carrier = entries.read_entry(reader, args, &mut full);
        if !with_frame {
            more = chain.follows.of(carrier) != 0;
        }
        for carry in chain.carry {
            if let Some(arg) = args.get_mut(usize::from(carry.arg)) {
                *arg = carry.span.of(carrier) as u8;
            }
        }
        if !with_frame && !more {
            break;
        }
    }
    close_chain(reader, chain.end, chain.depth, more);
    if full {
        Err(CodecError::CapacityExceeded)
    } else {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::codec::SliceSink;

    /// What a test value's stores were handed, by row.
    #[derive(Debug, Default, PartialEq)]
    struct Stored<'f> {
        numbers: [Option<u64>; 8],
        text: Option<&'f str>,
        bytes: Option<&'f [u8]>,
        absent: [bool; 8],
        /// The inputs the body was handed and the byte it read.
        body: Option<([u64; 2], u8)>,
        /// What the bytes store answers.
        refuse_bytes: Option<CodecError>,
    }

    impl<'f> TableOut<'f> for Stored<'f> {
        fn number(&mut self, row: usize, value: u64) {
            self.numbers[row] = Some(value);
        }

        fn bytes(&mut self, row: usize, bytes: &'f [u8]) -> Result<(), CodecError> {
            assert_eq!(row, 5);
            self.bytes = Some(bytes);
            match self.refuse_bytes {
                Some(refused) => Err(refused),
                None => Ok(()),
            }
        }

        fn text(&mut self, row: usize, text: &'f str) -> Result<(), CodecError> {
            assert_eq!(row, 3);
            self.text = Some(text);
            Ok(())
        }

        fn read(&mut self, row: usize, reader: &mut SceReader<'f>, inputs: &[u64]) {
            assert_eq!(row, 4);
            let byte = reader.take_array::<1>()[0];
            self.body = Some(([inputs[0], inputs[1]], byte));
        }

        fn absent(&mut self, row: usize) {
            self.absent[row] = true;
        }
    }

    /// A header byte; a `vle_u16` id where its bit 7 is set; a text's length
    /// and the text where the codec's input 0 is set, at most 8 bytes; a body
    /// handed the header's bits 5 and 6 and its bit 0, read where its bit 4 is
    /// clear; and a byte string whose count is the header's low nibble, at most
    /// 4 bytes.
    const ROWS: &[Row] = &[
        Row {
            read: Read::Uint {
                bytes: 1,
                big: true,
            },
            gate: Gate::Always,
        },
        Row {
            read: Read::Zint { bits: 16 },
            gate: Gate::Set(Bits::range(Source::Row(0), 7, 1)),
        },
        Row {
            read: Read::Zint { bits: 64 },
            gate: Gate::Set(Bits::range(Source::Input(0), 0, 1)),
        },
        Row {
            read: Read::Text {
                len: Bits::whole(Source::Row(2)),
                max: 8,
            },
            gate: Gate::Set(Bits::range(Source::Input(0), 0, 1)),
        },
        Row {
            read: Read::Body {
                inputs: &[
                    Bits::range(Source::Row(0), 5, 2),
                    Bits::range(Source::Row(0), 0, 1),
                ],
            },
            gate: Gate::Clear(Bits::range(Source::Row(0), 4, 1)),
        },
        Row {
            read: Read::Bytes {
                len: Bits::range(Source::Row(0), 0, 4),
                max: 4,
            },
            gate: Gate::Always,
        },
    ];

    static TABLE: FieldTable = FieldTable::new(ROWS);

    fn read<'f>(bytes: &'f [u8], inputs: &[u64], out: &mut Stored<'f>) -> Result<(), CodecError> {
        let mut reader = SceReader::new(bytes);
        read_table(&mut reader, &TABLE, inputs, out);
        reader.verdict()
    }

    #[test]
    fn every_open_row_is_read_in_order_and_stored_by_its_index() {
        // Header 0xE1: bit 7 (the id), bits 5 and 6 (3), bit 0 (1), bit 4 clear
        // (the body), and a byte string of one byte.
        let wire = [0xE1, 0x2A, 0x02, b'o', b'k', 0x77, 0x99];
        let mut out = Stored::default();
        assert_eq!(read(&wire, &[1], &mut out), Ok(()));
        assert_eq!(out.numbers[0], Some(0xE1));
        assert_eq!(out.numbers[1], Some(0x2A));
        assert_eq!(out.numbers[2], Some(2));
        assert_eq!(out.text, Some("ok"));
        assert_eq!(out.body, Some(([3, 1], 0x77)));
        assert_eq!(out.bytes, Some(&[0x99][..]));
        assert_eq!(out.absent, [false; 8]);
    }

    #[test]
    fn a_closed_gate_stores_its_field_absent_and_reads_no_byte() {
        // Header 0x10: no id, the body's gate closed, no bytes; input 0 clear,
        // so no text either.
        let mut out = Stored::default();
        assert_eq!(read(&[0x10], &[0], &mut out), Ok(()));
        assert_eq!(out.numbers[0], Some(0x10));
        assert_eq!(
            out.absent,
            [false, true, true, true, true, false, false, false]
        );
        assert_eq!(out.body, None);
        assert_eq!(out.bytes, Some(&[0u8; 0][..]));
        // An input the codec is not handed reads as zero.
        let mut out = Stored::default();
        assert_eq!(read(&[0x10], &[], &mut out), Ok(()));
        assert!(out.absent[2] && out.absent[3]);
    }

    #[test]
    fn a_short_frame_is_need_more_bytes_and_the_rows_after_read_nothing() {
        // The id is announced and missing.
        let mut out = Stored::default();
        assert_eq!(
            read(&[0x90], &[0], &mut out),
            Err(CodecError::NeedMoreBytes)
        );
        assert_eq!(out.numbers[1], Some(0));
        // The text is two bytes and one is there.
        let mut out = Stored::default();
        assert_eq!(
            read(&[0x10, 0x02, b'o'], &[1], &mut out),
            Err(CodecError::NeedMoreBytes)
        );
        assert_eq!(out.text, Some(""));
    }

    #[test]
    fn a_count_past_its_bound_is_out_of_range_before_its_bytes_are_asked_for() {
        // The text says 9 bytes, past its 8, and the frame has none of them:
        // the first refusal is the bound's.
        let mut out = Stored::default();
        assert_eq!(
            read(&[0x10, 0x09], &[1], &mut out),
            Err(CodecError::LengthOutOfRange)
        );
        // The byte string says 5, past its 4.
        let mut out = Stored::default();
        assert_eq!(
            read(&[0x15, 1, 2, 3, 4, 5], &[0], &mut out),
            Err(CodecError::LengthOutOfRange)
        );
    }

    #[test]
    fn a_text_that_is_not_utf8_is_refused_as_such() {
        let mut out = Stored::default();
        assert_eq!(
            read(&[0x10, 0x01, 0xFF], &[1], &mut out),
            Err(CodecError::InvalidUtf8)
        );
    }

    #[test]
    fn a_zint_past_its_width_is_refused_as_such() {
        // 0xFF 0xFF 0x04 is 0x13FFF, past a vle_u16.
        let mut out = Stored::default();
        assert_eq!(
            read(&[0x90, 0xFF, 0xFF, 0x04], &[0], &mut out),
            Err(CodecError::VleWidthOverflow)
        );
    }

    #[test]
    fn a_store_past_its_capacity_is_noted_and_the_frame_is_read_on() {
        // The byte string's store holds none: the frame is read whole and its
        // verdict is the capacity's.
        let mut out = Stored {
            refuse_bytes: Some(CodecError::CapacityExceeded),
            ..Stored::default()
        };
        let mut reader = SceReader::new(&[0x01, 0x77, 0x99]);
        read_table(&mut reader, &TABLE, &[0], &mut out);
        assert_eq!(reader.remaining(), 0);
        assert!(!reader.failed());
        assert_eq!(reader.verdict(), Err(CodecError::CapacityExceeded));
        // Any other refusal of a store is the frame's.
        let mut out = Stored {
            refuse_bytes: Some(CodecError::LengthOutOfRange),
            ..Stored::default()
        };
        assert_eq!(
            read(&[0x01, 0x77, 0x99], &[0], &mut out),
            Err(CodecError::LengthOutOfRange)
        );
    }

    #[test]
    fn an_unsigned_integer_is_read_in_its_byte_order() {
        const WIDE_ROWS: &[Row] = &[
            Row {
                read: Read::Uint {
                    bytes: 2,
                    big: true,
                },
                gate: Gate::Always,
            },
            Row {
                read: Read::Uint {
                    bytes: 2,
                    big: false,
                },
                gate: Gate::Always,
            },
            Row {
                read: Read::Uint {
                    bytes: 8,
                    big: true,
                },
                gate: Gate::Always,
            },
        ];
        static WIDE: FieldTable = FieldTable::new(WIDE_ROWS);
        let wire = [0x12, 0x34, 0x12, 0x34, 1, 2, 3, 4, 5, 6, 7, 8];
        let mut out = Stored::default();
        let mut reader = SceReader::new(&wire);
        read_table(&mut reader, &WIDE, &[], &mut out);
        assert_eq!(reader.verdict(), Ok(()));
        assert_eq!(out.numbers[0], Some(0x1234));
        assert_eq!(out.numbers[1], Some(0x3412));
        assert_eq!(out.numbers[2], Some(0x0102_0304_0506_0708));
    }

    #[test]
    #[should_panic(expected = "a row's gate reads a row before it or an input")]
    fn a_table_whose_gate_reads_a_row_after_it_is_refused() {
        static FORWARD: [Row; 2] = [
            Row {
                read: Read::Uint {
                    bytes: 1,
                    big: true,
                },
                gate: Gate::Set(Bits::range(Source::Row(1), 0, 1)),
            },
            Row {
                read: Read::Uint {
                    bytes: 1,
                    big: true,
                },
                gate: Gate::Always,
            },
        ];
        let _ = FieldTable::new(&FORWARD);
    }

    #[test]
    #[should_panic(expected = "a row's read is one the walker makes")]
    fn a_table_with_a_zint_of_no_width_the_reader_has_is_refused() {
        static ODD: [Row; 1] = [Row {
            read: Read::Zint { bits: 8 },
            gate: Gate::Always,
        }];
        let _ = FieldTable::new(&ODD);
    }

    /// A carrier byte, and a 16-bit number read where its bit 0 or its bit 1
    /// is set.
    const EITHER_ROWS: &[Row] = &[
        Row {
            read: Read::Uint {
                bytes: 1,
                big: true,
            },
            gate: Gate::Always,
        },
        Row {
            read: Read::Uint {
                bytes: 2,
                big: true,
            },
            gate: Gate::Any(&[
                Gate::Set(Bits::range(Source::Row(0), 0, 1)),
                Gate::Set(Bits::range(Source::Row(0), 1, 1)),
            ]),
        },
    ];

    static EITHER: FieldTable = FieldTable::new(EITHER_ROWS);

    #[test]
    fn a_gate_of_several_clauses_reads_its_row_where_any_of_them_is_open() {
        for carrier in [0x01u8, 0x02, 0x03] {
            let wire = [carrier, 0x12, 0x34];
            let mut out = Stored::default();
            let mut reader = SceReader::new(&wire);
            read_table(&mut reader, &EITHER, &[], &mut out);
            assert_eq!(reader.verdict(), Ok(()), "carrier {carrier:#04x}");
            assert_eq!(reader.remaining(), 0, "carrier {carrier:#04x}");
            assert_eq!(out.numbers[1], Some(0x1234), "carrier {carrier:#04x}");
            assert!(!out.absent[1], "carrier {carrier:#04x}");
        }
        // Neither bit: the number is absent and no byte of it is read.
        let mut out = Stored::default();
        let mut reader = SceReader::new(&[0x04, 0x12, 0x34]);
        read_table(&mut reader, &EITHER, &[], &mut out);
        assert_eq!(reader.verdict(), Ok(()));
        assert_eq!(reader.remaining(), 2);
        assert_eq!(out.numbers[1], None);
        assert!(out.absent[1]);
    }

    #[test]
    #[should_panic(expected = "each clause a flag")]
    fn a_disjunction_with_a_clause_that_is_no_flag_is_refused() {
        static ODD: [Row; 2] = [
            Row {
                read: Read::Uint {
                    bytes: 1,
                    big: true,
                },
                gate: Gate::Always,
            },
            Row {
                read: Read::Uint {
                    bytes: 1,
                    big: true,
                },
                gate: Gate::Any(&[Gate::Set(Bits::range(Source::Row(0), 0, 1)), Gate::Always]),
            },
        ];
        let _ = FieldTable::new(&ODD);
    }

    #[test]
    #[should_panic(expected = "a row's gate reads a row before it or an input")]
    fn a_disjunction_whose_clause_reads_a_row_after_it_is_refused() {
        static FORWARD: [Row; 2] = [
            Row {
                read: Read::Uint {
                    bytes: 1,
                    big: true,
                },
                gate: Gate::Any(&[
                    Gate::Set(Bits::range(Source::Input(0), 0, 1)),
                    Gate::Clear(Bits::range(Source::Row(1), 0, 1)),
                ]),
            },
            Row {
                read: Read::Uint {
                    bytes: 1,
                    big: true,
                },
                gate: Gate::Always,
            },
        ];
        let _ = FieldTable::new(&FORWARD);
    }

    /// The value of [`WRITTEN`] a write reads, by row: the header, the id, the
    /// text, the body's one byte and the byte string.
    struct Said {
        header: u8,
        id: Option<u16>,
        name: Option<&'static str>,
        body: Option<u8>,
        bytes: &'static [u8],
    }

    impl<E: core::fmt::Debug> TableSource<E> for Said {
        fn number(&self, row: usize) -> u64 {
            match row {
                0 => u64::from(self.header),
                1 => self.id.map_or(0, u64::from),
                _ => 0,
            }
        }

        fn present(&self, row: usize) -> bool {
            match row {
                1 => self.id.is_some(),
                2 | 3 => self.name.is_some(),
                4 => self.body.is_some(),
                _ => true,
            }
        }

        fn bytes(&self, row: usize) -> &[u8] {
            match row {
                3 => self.name.map(str::as_bytes).unwrap_or_default(),
                5 => self.bytes,
                _ => &[],
            }
        }

        fn write(
            &self,
            row: usize,
            sink: &mut dyn SceSink<Error = E>,
            inputs: &[u64],
        ) -> Result<(), E> {
            assert_eq!(row, 4);
            // The header's bits 5 and 6, which the value states, and its bit 0,
            // which the count of the byte string derives: the bits of the
            // header as written, not as the value holds it.
            assert_eq!(
                inputs,
                &[
                    u64::from(self.header >> 5) & 0x3,
                    self.bytes.len() as u64 & 0x1
                ]
            );
            match self.body {
                Some(byte) => sink.write_u8(byte),
                None => Ok(()),
            }
        }
    }

    /// [`ROWS`] with the numbers its encode derives: the header's bit 7 from
    /// whether the id is present, the text's length from the text, and the
    /// header's low nibble from the byte string.
    const DERIVED: &[Derive] = &[
        Derive {
            into: Bits::range(Source::Row(0), 7, 1),
            from: Derived::Present(1),
        },
        Derive {
            into: Bits::whole(Source::Row(2)),
            from: Derived::Count(3),
        },
        Derive {
            into: Bits::range(Source::Row(0), 0, 4),
            from: Derived::Count(5),
        },
    ];

    static WRITTEN: FieldTable = FieldTable::with_derived(ROWS, DERIVED);

    #[test]
    fn a_table_writes_what_its_source_says_and_the_numbers_it_derives() {
        let said = Said {
            header: 0x60,
            id: Some(42),
            name: Some("ok"),
            body: Some(0x77),
            bytes: &[0x99],
        };
        let mut buf = [0u8; 16];
        let mut sink = SliceSink::new(&mut buf);
        assert_eq!(write_table(&mut sink, &WRITTEN, &[1], &said), Ok(()));
        let n = sink.position();
        // The bytes the read of the same table reads back.
        assert_eq!(&buf[..n], &[0xE1, 0x2A, 0x02, b'o', b'k', 0x77, 0x99]);
        let mut out = Stored::default();
        assert_eq!(read(&buf[..n], &[1], &mut out), Ok(()));
        assert_eq!(out.numbers[1], Some(42));
        assert_eq!(out.text, Some("ok"));
        assert_eq!(out.body, Some(([3, 1], 0x77)));
    }

    #[test]
    fn a_table_writes_no_byte_of_an_absent_field_and_clears_its_flag() {
        let said = Said {
            header: 0x90,
            id: None,
            name: None,
            body: None,
            bytes: &[],
        };
        let mut buf = [0u8; 16];
        let mut sink = SliceSink::new(&mut buf);
        assert_eq!(write_table(&mut sink, &WRITTEN, &[0], &said), Ok(()));
        let n = sink.position();
        // Bit 7 is the id's presence, whatever the value's header held.
        assert_eq!(&buf[..n], &[0x10]);
    }

    #[test]
    fn a_table_write_past_its_sinks_room_is_the_sinks_refusal() {
        let said = Said {
            header: 0x60,
            id: Some(42),
            name: Some("ok"),
            body: Some(0x77),
            bytes: &[0x99],
        };
        let mut buf = [0u8; 3];
        let mut sink = SliceSink::new(&mut buf);
        assert_eq!(
            write_table(&mut sink, &WRITTEN, &[1], &said),
            Err(CodecError::BufferOverflow)
        );
    }

    #[test]
    fn a_dyn_sink_writes_through_its_inner_sink() {
        let mut buf = [0u8; 8];
        let mut inner = SliceSink::new(&mut buf);
        let mut sink = DynSink(&mut inner);
        assert_eq!(sink.write_u8(0x42), Ok(()));
        assert_eq!(sink.write_vle_u16(300), Ok(()));
        assert_eq!(sink.position(), 3);
        let n = inner.position();
        assert_eq!(&buf[..n], &[0x42, 0xAC, 0x02]);
    }

    #[test]
    #[should_panic(expected = "a derived number is written into a row of the table")]
    fn a_derive_into_an_input_is_refused() {
        static INTO_INPUT: [Derive; 1] = [Derive {
            into: Bits {
                from: Source::Input(0),
                shift: 0,
                mask: 1,
            },
            from: Derived::Present(0),
        }];
        let _ = FieldTable::with_derived(ROWS, &INTO_INPUT);
    }

    /// An entry of one byte, and one more where its low bit is set; its high
    /// bit says another entry follows.
    #[derive(Debug, Clone, Copy, Default, PartialEq)]
    struct Entry {
        head: u8,
        extra: Option<u8>,
        /// The input the chain handed this entry.
        carried: u8,
    }

    fn read_entry(r: &mut SceReader<'_>, a: &[u8]) -> Entry {
        let head = r.take_array::<1>()[0];
        let extra = if head & 1 == 1 {
            Some(r.take_array::<1>()[0])
        } else {
            None
        };
        Entry {
            head,
            extra,
            carried: a[0],
        }
    }

    fn carrier(e: &Entry) -> u64 {
        u64::from(e.head)
    }

    const fn chain(end: ChainEnd, depth: Option<u32>) -> ChainTable {
        ChainTable::new(end, depth, Span::range(7, 1), &[], 1)
    }

    type List = heapless::Vec<Entry, 4>;

    /// The list a chain was read into, the reader's verdict and what the
    /// routine answered.
    type ChainRead = (List, Result<(), CodecError>, Result<(), CodecError>);

    fn read_chain(table: &ChainTable, bytes: &'static [u8]) -> ChainRead {
        let mut r = SceReader::new(bytes);
        let mut list = List::new();
        let mut args = [0u8; 1];
        let answer = read_chain_table(
            &mut r,
            table,
            &mut args,
            &mut HeldEntries::new(&mut list, read_entry, carrier),
        );
        (list, r.verdict(), answer)
    }

    fn heads(list: &List) -> heapless::Vec<u8, 4> {
        list.iter().map(|e| e.head).collect()
    }

    #[test]
    fn a_chain_table_to_the_end_of_its_frame_reads_every_entry() {
        let (list, verdict, answer) = read_chain(&chain(ChainEnd::Frame, Some(4)), &[2, 3, 9, 4]);
        assert_eq!(heads(&list), [2, 3, 4]);
        assert_eq!(list[1].extra, Some(9));
        assert_eq!((verdict, answer), (Ok(()), Ok(())));
        // Every flag is clear and each entry is read all the same.
        let (list, _, _) = read_chain(&chain(ChainEnd::Frame, None), &[0x02, 0x04, 0x06]);
        assert_eq!(heads(&list), [0x02, 0x04, 0x06]);
    }

    #[test]
    fn a_chain_table_past_its_depth_is_too_many() {
        let (list, verdict, _) = read_chain(&chain(ChainEnd::Frame, Some(2)), &[2, 4, 6]);
        assert_eq!(heads(&list), [2, 4]);
        assert_eq!(verdict, Err(CodecError::TooManyElements));
        let (list, verdict, _) = read_chain(&chain(ChainEnd::Flag, Some(2)), &[0x82, 0x84, 0x06]);
        assert_eq!(heads(&list), [0x82, 0x84]);
        assert_eq!(verdict, Err(CodecError::TooManyElements));
    }

    #[test]
    fn a_chain_table_closed_by_a_flag_ends_after_the_entry_whose_flag_is_clear() {
        let (list, verdict, _) = read_chain(&chain(ChainEnd::Flag, Some(4)), &[0x82, 0x04, 0x06]);
        assert_eq!(heads(&list), [0x82, 0x04]);
        assert_eq!(verdict, Ok(()));
        // The frame ends after an entry that says another follows.
        let (_, verdict, _) = read_chain(&chain(ChainEnd::Flag, None), &[0x82]);
        assert_eq!(verdict, Err(CodecError::NeedMoreBytes));
        // An empty frame is short: the chain is one entry or more.
        let (_, verdict, _) = read_chain(&chain(ChainEnd::Flag, Some(4)), &[]);
        assert_eq!(verdict, Err(CodecError::NeedMoreBytes));
    }

    #[test]
    fn a_chain_table_carries_a_range_of_each_entry_to_the_next() {
        const SECOND_BIT: &[Carry] = &[Carry {
            arg: 0,
            span: Span::range(1, 1),
        }];
        let carried = ChainTable::new(ChainEnd::Frame, None, Span::range(7, 1), SECOND_BIT, 1);
        let (list, verdict, _) = read_chain(&carried, &[0x02, 0x00, 0x04]);
        assert_eq!(verdict, Ok(()));
        let handed: heapless::Vec<u8, 4> = list.iter().map(|e| e.carried).collect();
        assert_eq!(handed, [0, 1, 0]);
    }

    #[test]
    fn a_chain_table_past_its_lists_capacity_reads_on_and_answers_capacity() {
        let (list, verdict, answer) = read_chain(&chain(ChainEnd::Frame, None), &[2, 4, 6, 8, 10]);
        assert_eq!(heads(&list), [2, 4, 6, 8]);
        assert_eq!(verdict, Ok(()));
        assert_eq!(answer, Err(CodecError::CapacityExceeded));
    }
    #[test]
    #[should_panic(expected = "a chain carries into an input its entries are handed")]
    fn a_chain_that_carries_into_no_input_is_refused() {
        const PAST: &[Carry] = &[Carry {
            arg: 1,
            span: Span::range(1, 1),
        }];
        let _ = ChainTable::new(ChainEnd::Frame, None, Span::range(7, 1), PAST, 1);
    }

    #[test]
    #[should_panic(expected = "a chain carries a range an input's byte holds")]
    fn a_chain_that_carries_a_range_wider_than_a_byte_is_refused() {
        const WIDE: &[Carry] = &[Carry {
            arg: 0,
            span: Span::range(0, 9),
        }];
        let _ = ChainTable::new(ChainEnd::Frame, None, Span::range(7, 1), WIDE, 1);
    }

    /// The next number of a sequence that covers every `u64` (xorshift), so a
    /// test reads many values and the same ones every run.
    fn next(state: &mut u64) -> u64 {
        *state ^= *state << 13;
        *state ^= *state >> 7;
        *state ^= *state << 17;
        *state
    }

    /// Every value of [`WRITTEN`] a run draws is read back as it was written:
    /// each kind of row, gated by a carrier's bit, by an input and by a clear
    /// bit, with the numbers the encode derives. A frame is written and read by
    /// one rule, the gate over the same numbers, so what is written is read
    /// whole and row for row.
    #[test]
    fn a_value_written_by_a_table_is_read_back_by_the_same_table() {
        const NAMES: [&str; 4] = ["", "a", "ok", "12345678"];
        const BYTES: [&[u8]; 4] = [&[], &[0x99], &[1, 2, 3], &[0xFF, 0, 0xFF, 0]];
        let mut state = 0x9E37_79B9_7F4A_7C15u64;
        for _ in 0..512 {
            let draw = next(&mut state);
            let id = (draw & 1 == 1).then_some((draw >> 8) as u16);
            let name = (draw & 2 == 2).then_some(NAMES[(draw >> 24) as usize % NAMES.len()]);
            let body = (draw & 4 == 4).then_some((draw >> 32) as u8);
            let bytes = BYTES[(draw >> 40) as usize % BYTES.len()];
            // The body's gate is the header's bit 4 clear, which the value
            // states; bits 5 and 6 are the value's own.
            let header = ((draw >> 48) as u8 & 0x60) | if body.is_some() { 0 } else { 0x10 };
            let said = Said {
                header,
                id,
                name,
                body,
                bytes,
            };
            let inputs = [u64::from(name.is_some())];
            let mut buf = [0u8; 32];
            let mut sink = SliceSink::new(&mut buf);
            assert_eq!(write_table(&mut sink, &WRITTEN, &inputs, &said), Ok(()));
            let n = sink.position();
            let mut reader = SceReader::new(&buf[..n]);
            let mut out = Stored::default();
            read_table(&mut reader, &WRITTEN, &inputs, &mut out);
            assert_eq!(reader.verdict(), Ok(()), "{:x?}", &buf[..n]);
            assert_eq!(reader.remaining(), 0, "{:x?}", &buf[..n]);
            // The header as written: bit 7 the id's presence, the low nibble
            // the count of the byte string.
            let written =
                u64::from(header) | if id.is_some() { 0x80 } else { 0 } | bytes.len() as u64;
            assert_eq!(out.numbers[0], Some(written));
            assert_eq!(out.numbers[1], id.map(u64::from));
            assert_eq!(out.absent[1], id.is_none());
            assert_eq!(out.text, name);
            assert_eq!(out.absent[3], name.is_none());
            assert_eq!(
                out.body,
                body.map(|byte| ([(written >> 5) & 3, written & 1], byte))
            );
            assert_eq!(out.absent[4], body.is_none());
            assert_eq!(out.bytes, Some(bytes));
        }
    }
}
