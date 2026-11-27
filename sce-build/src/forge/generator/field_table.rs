// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! A cold codec's fields as one table the runtime's one walker reads
//! (docs/adr/0155).
//!
//! A build that names its hot roots says which codecs its per-message path
//! reads; every other codec of the set is read once per event, where a few
//! instructions a field are nothing and a printed body per codec is all of its
//! cost. Such a codec, where every field is of a kind a row states, is read by
//! one call of the runtime's `read_table` over a `static` `FieldTable` of
//! its rows, and its value's stores are an implementation of `TableOut` whose
//! methods each `match` on the row's index.
//!
//! A row is a field in wire order: an unsigned integer of whole bytes, a ZInt,
//! a byte string or a text counted by an earlier row, or a body the value reads
//! itself (an embedded body, a repeat, a chain), each gated by a flag of an
//! earlier carrier or of an input the codec is handed. A body's read is the
//! statement the printed codec reads it by, its own gate with it, handed the
//! earlier numbers and inputs it names. Any other field keeps the codec printed:
//! [`lower`] answers why, as the printers of docs/adr/0081 answer the forms they
//! do not lower.

use crate::filters;
use crate::forge::codec_roles::ElementId;
use crate::forge::encode_plan::{self, Absent, EncodePlan, Measured, Narrow, NumberInto, Offset};
use crate::forge::model::{
    BitSize, CodecEncoding, CodecField, CodecModel, Endian, PresentIfPredicate, PresentIfScope,
    SceType,
};
use crate::forge::shared_tables::{
    CodecShape, NotATable, NotWrittenByTable, TableHome, UnstatedDerive, UnstatedField,
    UnwrittenField,
};

use super::{rust_storage_tag, to_upper_snake, RUNTIME_CODEC, STORAGE_PARAM};

/// The most rows a table holds, the runtime's `TABLE_ROWS`.
const TABLE_ROWS: usize = 32;

/// The most numbers a body's read is handed, the runtime's `ROW_INPUTS`.
const ROW_INPUTS: usize = 4;

/// What the generation of a codec says of it, which its table is lowered from.
pub(super) struct Input<'a> {
    /// The codec.
    pub(super) m: &'a CodecModel,
    /// What each field is held as, in the order of `m.fields`: a field a gate
    /// group holds is no row of a table.
    pub(super) stored: &'a [serde_json::Value],
    /// What each field is read by at a cold site, in the order of `m.fields`.
    pub(super) wire: &'a [serde_json::Value],
    /// The codec's type name.
    pub(super) struct_name: &'a str,
    /// Whether the value is generic over a storage profile (`{Codec}In<S>`).
    pub(super) owned: bool,
}

/// The table of a codec, as the generated module prints it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct PrintedTable {
    /// The name of the `static` that holds the table.
    pub(super) name: String,
    /// The inputs `read_into` hands the walker, each a `u64` of an input of
    /// the codec, joined by commas.
    pub(super) inputs: String,
    /// The rows, one for each field of the codec in wire order: what the write
    /// of the codec is lowered over ([`derives`]).
    pub(super) rows: Vec<Row>,
    /// The type name of the codec's value.
    struct_name: String,
    /// The value's `TableOut`, its stores by the index of each row, as the
    /// template prints them.
    stores: serde_json::Value,
}

impl PrintedTable {
    /// Whether a gate or a body's read reads the kinds a chain's read answers,
    /// the number of a body row: no source of the value says it, so the codec
    /// is written by its printed encode (docs/adr/0155 section 3).
    pub(super) fn reads_kinds(&self) -> bool {
        let body = |place: Place| match place {
            Place::Row(row) => matches!(self.rows[row].kind, Kind::Body { .. }),
            Place::Input(_) => false,
        };
        self.rows.iter().any(|row| {
            let hands = match &row.kind {
                Kind::Body { handed, .. } => handed.iter().any(|h| body(h.from)),
                _ => false,
            };
            hands || row.gate.reads(&body)
        })
    }

    /// What the table is, row for row: its rows and the numbers its write
    /// derives (`derived`), with no name of the codec's. Two codecs of a set
    /// whose tables are the same are one table (docs/adr/0155 section 4).
    pub(super) fn rows_key(&self, derived: &[Derive]) -> String {
        serde_json::json!({
            "rows": self.rows.iter().map(Row::context).collect::<Vec<_>>(),
            "derived": derived.iter().map(|derive| derive.context()).collect::<Vec<_>>(),
        })
        .to_string()
    }

    /// What the codec template prints the table and the value's stores from
    /// (`field_table`): the rows and the numbers the encode derives where the
    /// codec is written by the runtime's one writer over its table
    /// (`derived`, docs/adr/0155 section 3), where the `static` is
    /// (`home`, section 4), and the stores by the index of each row.
    pub(super) fn context(&self, derived: &[Derive], home: &TableHome) -> serde_json::Value {
        let (home, path) = match home {
            TableHome::Own => ("own", None),
            TableHome::Lent => ("lent", None),
            TableHome::Borrowed { path, .. } => ("borrowed", Some(path.clone())),
        };
        let mut uses = vec!["FieldTable", "Gate", "Read", "Row"];
        if !derived.is_empty() || self.rows.iter().any(Row::names_bits) {
            uses.extend(["Bits", "Source"]);
        }
        if !derived.is_empty() {
            uses.extend(["Derive", "Derived"]);
        }
        uses.sort_unstable();
        serde_json::json!({
            "name": self.name,
            "inputs": self.inputs,
            "struct_name": self.struct_name,
            "home": home,
            "path": path,
            "uses": uses.join(", "),
            "rows": self.rows.iter().map(Row::context).collect::<Vec<_>>(),
            "derived": derived.iter().map(|derive| derive.context()).collect::<Vec<_>>(),
            "stores": self.stores,
        })
    }
}

/// Where the number a bit range reads is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Place {
    /// The number the row at this index read.
    Row(usize),
    /// The input of the codec at this index.
    Input(usize),
}

/// A bit range of a number: the whole number where `width` is `None`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct Bits {
    pub(super) from: Place,
    pub(super) shift: u32,
    pub(super) width: Option<u32>,
}

impl Bits {
    fn whole(from: Place) -> Self {
        Self {
            from,
            shift: 0,
            width: None,
        }
    }

    /// The range as the template prints a runtime `Bits` of it.
    fn context(self) -> serde_json::Value {
        let (source, index) = match self.from {
            Place::Row(row) => ("Row", row),
            Place::Input(input) => ("Input", input),
        };
        serde_json::json!({
            "source": source,
            "index": index,
            "shift": self.shift,
            "width": self.width,
        })
    }
}

/// Whether a row is read.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum Gate {
    Always,
    Set(Bits),
    Clear(Bits),
    /// Open where any of its clauses is, each a [`Gate::Set`] or a
    /// [`Gate::Clear`]: `sce:present-if` of `a || b`.
    Any(Vec<Gate>),
}

impl Gate {
    /// Whether a clause of the gate reads a number at a place `at` answers.
    fn reads(&self, at: &impl Fn(Place) -> bool) -> bool {
        match self {
            Gate::Always => false,
            Gate::Set(bits) | Gate::Clear(bits) => at(bits.from),
            Gate::Any(clauses) => clauses.iter().any(|clause| clause.reads(at)),
        }
    }

    /// The gate as the template prints a runtime `Gate` of it.
    fn context(&self) -> serde_json::Value {
        match self {
            Gate::Always => serde_json::json!({ "kind": "Always", "bits": null }),
            Gate::Set(bits) => serde_json::json!({ "kind": "Set", "bits": bits.context() }),
            Gate::Clear(bits) => serde_json::json!({ "kind": "Clear", "bits": bits.context() }),
            Gate::Any(clauses) => serde_json::json!({
                "kind": "Any",
                "bits": null,
                "clauses": clauses.iter().map(Gate::context).collect::<Vec<_>>(),
            }),
        }
    }
}

/// A number a body's read is handed, by the name its statement reads it by.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct Handed {
    /// The local the statement names.
    pub(super) local: String,
    /// The Rust type of the local: `bool` for a kind of a chain's entries.
    pub(super) ty: &'static str,
    /// Where the number is.
    pub(super) from: Place,
    /// The bit of the number the local is, where it is a kind a chain's read
    /// answers (`ty` `bool`); `None` where it is the whole number.
    pub(super) bit: Option<u32>,
}

impl Handed {
    /// The range of the number the body is handed.
    fn bits(&self) -> Bits {
        match self.bit {
            Some(bit) => Bits {
                from: self.from,
                shift: bit,
                width: Some(1),
            },
            None => Bits::whole(self.from),
        }
    }
}

/// How a row's field is read.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum Kind {
    /// An unsigned integer of `bytes` bytes.
    Uint {
        bytes: u32,
        big: bool,
        ty: &'static str,
    },
    /// A ZInt held to `bits`.
    Zint { bits: u32, ty: &'static str },
    /// A byte string or a text counted by the row `len`, at most `max`
    /// bytes.
    Counted {
        text: bool,
        len: usize,
        max: Option<u64>,
    },
    /// A body the value reads itself, by the printed statement, handed
    /// `handed`.
    Body {
        statement: String,
        handed: Vec<Handed>,
    },
}

/// One row of a table.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct Row {
    /// How the row's field is read.
    pub(super) kind: Kind,
    /// Whether it is read.
    pub(super) gate: Gate,
}

impl Row {
    /// The row as the template prints a runtime `Row` of it.
    fn context(&self) -> serde_json::Value {
        let read = match &self.kind {
            Kind::Uint { bytes, big, .. } => {
                serde_json::json!({ "kind": "Uint", "bytes": bytes, "big": big })
            }
            Kind::Zint { bits, .. } => serde_json::json!({ "kind": "Zint", "bits": bits }),
            Kind::Counted { text, len, max } => serde_json::json!({
                "kind": if *text { "Text" } else { "Bytes" },
                "len": Bits::whole(Place::Row(*len)).context(),
                "max": max.map_or_else(|| "u64::MAX".to_string(), |max| max.to_string()),
            }),
            Kind::Body { handed, .. } => serde_json::json!({
                "kind": "Body",
                "inputs": handed
                    .iter()
                    .map(|h| h.bits().context())
                    .collect::<Vec<_>>(),
            }),
        };
        serde_json::json!({ "read": read, "gate": self.gate.context() })
    }

    /// Whether the row's print names a `Bits`.
    fn names_bits(&self) -> bool {
        !matches!(self.gate, Gate::Always)
            || matches!(&self.kind, Kind::Counted { .. })
            || matches!(&self.kind, Kind::Body { handed, .. } if !handed.is_empty())
    }
}

/// What the generation says of one field, read off its context.
struct Said<'a> {
    /// The member of the value the field is.
    ident: &'a str,
    /// The local a printed statement reads the field by.
    local: &'a str,
    /// The statement a list or an embedded body is read by.
    statement: Option<&'a str>,
    /// The statement any other field is read by.
    read: Option<&'a str>,
    /// Whether a byte string or a text is made through `Bounded::new`.
    bounded: bool,
}

impl<'a> Said<'a> {
    fn of(wire: &'a serde_json::Value) -> Option<Self> {
        let text = |key: &str| wire.get(key).and_then(serde_json::Value::as_str);
        let ident = text("id")?;
        Some(Self {
            ident,
            local: text("local").unwrap_or(ident),
            statement: [
                "embed_decode_stmt",
                "repeat_decode_stmt",
                "tlv_chain_decode_stmt",
            ]
            .into_iter()
            .find_map(text),
            read: text("present_if_decode_stmt"),
            bounded: wire.get("bounded_rebind").is_some(),
        })
    }
}

/// The Rust type of an unsigned integer field and its width in bits.
fn unsigned(ty: &SceType) -> Option<(&'static str, u32)> {
    match ty {
        SceType::Uint8 => Some(("u8", 8)),
        SceType::Uint16 => Some(("u16", 16)),
        SceType::Uint32 => Some(("u32", 32)),
        SceType::Uint64 => Some(("u64", 64)),
        _ => None,
    }
}

/// Whether `text` names the local `name`: an identifier of its own, not a
/// member after a `.`.
pub(super) fn mentions(text: &str, name: &str) -> bool {
    let ident = |c: char| c.is_ascii_alphanumeric() || c == '_';
    !name.is_empty()
        && text.match_indices(name).any(|(at, _)| {
            let before = text[..at].chars().next_back();
            let after = text[at + name.len()..].chars().next();
            !before.is_some_and(|c| ident(c) || c == '.' || c == '#') && !after.is_some_and(ident)
        })
}

/// The gate of the field at `at`: each clause a flag of an earlier carrier no
/// gate decides, an input of the codec, or a kind of an earlier chain's entries,
/// and a disjunction of several clauses open where any of them is.
fn gate_of(
    m: &CodecModel,
    at: usize,
    gate: Option<&PresentIfPredicate>,
) -> Result<Gate, UnstatedField> {
    let Some(gate) = gate else {
        return Ok(Gate::Always);
    };
    let clauses = std::iter::successors(Some(gate), |clause| clause.or_with.as_deref())
        .map(|clause| clause_of(m, at, clause))
        .collect::<Result<Vec<Gate>, UnstatedField>>()?;
    Ok(match <[Gate; 1]>::try_from(clauses) {
        Ok([one]) => one,
        Err(clauses) => Gate::Any(clauses),
    })
}

/// One clause of the gate of the field at `at`, its disjunction aside: the
/// flag it reads, set or clear. A kind of a chain's entries (`<chain>.has(<id>)`)
/// is the bit of the chain's row its read answers ([`chain_kinds`]).
fn clause_of(m: &CodecModel, at: usize, gate: &PresentIfPredicate) -> Result<Gate, UnstatedField> {
    let bits = match gate.scope {
        PresentIfScope::Local => {
            let carrier = m.fields[..at]
                .iter()
                .position(|f| f.id == gate.field_id)
                .ok_or(UnstatedField::GatedByNoCarrier)?;
            if m.fields[carrier].present_if.is_some() {
                return Err(UnstatedField::GatedByGatedCarrier);
            }
            let flag = m.fields[carrier]
                .flags
                .iter()
                .find(|flag| flag.name == gate.flag_name)
                .ok_or(UnstatedField::GatedByNoFlag)?;
            Bits {
                from: Place::Row(carrier),
                shift: flag.bit,
                width: Some(flag.width),
            }
        }
        PresentIfScope::Input => {
            let input = m
                .flag_inputs
                .iter()
                .position(|input| input.name == gate.flag_name)
                .ok_or(UnstatedField::GatedByNoInput)?;
            Bits {
                from: Place::Input(input),
                shift: 0,
                width: Some(m.flag_inputs[input].width),
            }
        }
        PresentIfScope::Chain => {
            let chain = m.fields[..at]
                .iter()
                .position(|f| f.id == gate.field_id)
                .filter(|&chain| matches!(m.fields[chain].bit_size, BitSize::TlvChain { .. }))
                .ok_or(UnstatedField::GatedByNoCarrier)?;
            let bit = gate
                .entry_id
                .and_then(|id| {
                    super::chain_has_values(&m.fields, &gate.field_id)
                        .iter()
                        .position(|&value| value == id)
                })
                .and_then(|bit| u32::try_from(bit).ok())
                .filter(|&bit| bit < u64::BITS)
                .ok_or(UnstatedField::GatedByChain)?;
            Bits {
                from: Place::Row(chain),
                shift: bit,
                width: Some(1),
            }
        }
    };
    Ok(if gate.negate {
        Gate::Clear(bits)
    } else {
        Gate::Set(bits)
    })
}

/// The row of a number: an unsigned integer of whole bytes, or a ZInt held to
/// its type's width.
fn number_row(m: &CodecModel, f: &CodecField, read: Option<&str>) -> Result<Kind, UnstatedField> {
    let (ty, width) = unsigned(&f.sce_type).ok_or(UnstatedField::NotUnsigned)?;
    // The printed read of a number refuses nothing but a short frame; a read
    // that refuses a value (a range, a conversion) is one no row states.
    let read = read.ok_or(UnstatedField::NoPrintedRead)?;
    if ["fail(", "settle(", "hold(", "ok_or("]
        .iter()
        .any(|refusal| read.contains(refusal))
    {
        return Err(UnstatedField::RefusesValue);
    }
    match f.bit_size {
        BitSize::Fixed { bits } if bits == width && f.bit_offset.is_none() => {
            let big = match f.effective_endian(m.default_endian) {
                Endian::Big => true,
                Endian::Little => false,
                Endian::Native if bits == 8 => true,
                Endian::Native => return Err(UnstatedField::NativeOrder),
            };
            Ok(Kind::Uint {
                bytes: bits / 8,
                big,
                ty,
            })
        }
        BitSize::Vle { width_bits } if width_bits == width && width_bits >= 16 => {
            Ok(Kind::Zint { bits: width, ty })
        }
        _ => Err(UnstatedField::NumberWidth {
            bits: f.bit_size.as_attr(),
        }),
    }
}

/// The row of a byte string or a text counted by an earlier number that the
/// same gate decides.
fn counted_row(m: &CodecModel, at: usize, rows: &[Row]) -> Result<Kind, UnstatedField> {
    let f = &m.fields[at];
    let text = match f.sce_type {
        SceType::String => true,
        SceType::Bytes => false,
        _ => return Err(UnstatedField::NotBytesOrText),
    };
    let count = f
        .length_field
        .as_deref()
        .ok_or(UnstatedField::CountedByNoField)?;
    let len = m.fields[..at]
        .iter()
        .position(|other| other.id == count)
        .ok_or(UnstatedField::CountedByNoEarlierNumber)?;
    if !matches!(rows[len].kind, Kind::Uint { .. } | Kind::Zint { .. }) {
        return Err(UnstatedField::CountedByNoEarlierNumber);
    }
    if m.fields[len].present_if != f.present_if {
        return Err(UnstatedField::GatedOtherwiseThanCount);
    }
    let bound = f
        .length_bound(&m.fields)
        .ok_or(UnstatedField::NoLengthBound)?;
    let (least, most) = bound.carried_range();
    if least.is_some_and(|least| least > 0) {
        return Err(UnstatedField::LeastLength);
    }
    Ok(Kind::Counted {
        text,
        len,
        max: most,
    })
}

/// The row of a body the value reads itself: the printed statement, gated by
/// itself, handed each earlier number and input it names.
fn body_row(
    m: &CodecModel,
    at: usize,
    rows: &[Row],
    said: &[Said<'_>],
) -> Result<Kind, UnstatedField> {
    let statement = said[at].statement.ok_or(UnstatedField::NoPrintedRead)?;
    let mut handed = Vec::new();
    for (row, earlier) in rows.iter().enumerate() {
        if !mentions(statement, said[row].local) {
            continue;
        }
        let ty = match (&earlier.kind, &earlier.gate) {
            (Kind::Uint { ty, .. } | Kind::Zint { ty, .. }, Gate::Always) => *ty,
            _ => {
                return Err(UnstatedField::NamesUnstatedNumber {
                    named: m.fields[row].id.clone(),
                });
            }
        };
        handed.push(Handed {
            local: said[row].local.to_string(),
            ty,
            from: Place::Row(row),
            bit: None,
        });
    }
    // A body gated by a kind of an earlier chain's entries reads the local the
    // chain's read binds it to, handed as the bit of the chain's row.
    for chain in 0..rows.len() {
        for (bit, local) in chain_kinds(m, chain).into_iter().enumerate() {
            if mentions(statement, &local) {
                handed.push(Handed {
                    local,
                    ty: "bool",
                    from: Place::Row(chain),
                    bit: Some(u32::try_from(bit).map_err(|_| UnstatedField::GatedByChain)?),
                });
            }
        }
    }
    for (input, declared) in m.flag_inputs.iter().enumerate() {
        let local = filters::to_snake_case(declared.name.clone());
        if mentions(statement, &local) {
            handed.push(Handed {
                local,
                ty: "u8",
                from: Place::Input(input),
                bit: None,
            });
        }
    }
    if handed.len() > ROW_INPUTS {
        return Err(UnstatedField::NamesTooManyNumbers);
    }
    Ok(Kind::Body {
        statement: statement.to_string(),
        handed,
    })
}

/// The locals the read of the field at `at` binds the kinds of its entries to
/// that later gates ask of it, in the order of the bits its row keeps them in:
/// one for each identifier a `<chain>.has(<id>)` of the codec names, ascending
/// (docs/adr/0065 §6 R6). Empty for a field that is no chain, or a chain no gate
/// asks a kind of.
fn chain_kinds(m: &CodecModel, at: usize) -> Vec<String> {
    let chain = &m.fields[at];
    if !matches!(chain.bit_size, BitSize::TlvChain { .. }) {
        return Vec::new();
    }
    super::chain_has_values(&m.fields, &chain.id)
        .into_iter()
        .map(|value| super::chain_has_local(&chain.id, value, crate::generator::Language::Rust))
        .collect()
}

/// The table of the codec `input.m`, read at a cold site, or why it is not
/// one: the first field no row states, or the shape of the codec.
pub(super) fn lower(input: &Input<'_>) -> Result<PrintedTable, NotATable> {
    let m = input.m;
    let shape =
        |shape: CodecShape| -> Result<PrintedTable, NotATable> { Err(NotATable::Codec(shape)) };
    if m.encoding != CodecEncoding::Positional {
        return shape(CodecShape::NotPositional);
    }
    if m.variant.is_some() {
        return shape(CodecShape::Variant);
    }
    if m.fields.is_empty() {
        return shape(CodecShape::NoField);
    }
    if m.fields.len() > TABLE_ROWS {
        return shape(CodecShape::TooManyFields);
    }
    if input.wire.len() != m.fields.len() || input.stored.len() != m.fields.len() {
        return shape(CodecShape::GateGroup);
    }
    let Some(said) = input
        .wire
        .iter()
        .map(Said::of)
        .collect::<Option<Vec<Said<'_>>>>()
    else {
        return shape(CodecShape::NoContext);
    };
    let mut rows: Vec<Row> = Vec::with_capacity(m.fields.len());
    for (at, (f, stored)) in m.fields.iter().zip(input.stored).enumerate() {
        let row = row_of(input, at, stored, &rows, &said).map_err(|form| NotATable::Field {
            field: f.id.clone(),
            form,
        })?;
        rows.push(row);
    }
    Ok(print(input, rows, &said))
}

/// The row of the field at `at`, held as `stored` says, after the rows
/// `rows`, or what no row states of it.
fn row_of(
    input: &Input<'_>,
    at: usize,
    stored: &serde_json::Value,
    rows: &[Row],
    said: &[Said<'_>],
) -> Result<Row, UnstatedField> {
    let m = input.m;
    let f = &m.fields[at];
    if stored.get("assign_stmt").is_some() || stored.get("assign_expr").is_some() {
        return Err(UnstatedField::HeldByGateGroup);
    }
    if f.dma_burst_align.is_some()
        || f.length_arith.is_some()
        || f.embed_length_from.is_some()
        || f.quantity.is_some()
    {
        return Err(UnstatedField::OwnForm);
    }
    let row = match f.bit_size {
        BitSize::Fixed { .. } | BitSize::Vle { .. } => Row {
            kind: number_row(m, f, said[at].read)?,
            gate: gate_of(m, at, f.present_if.as_ref())?,
        },
        BitSize::LengthRef => Row {
            kind: counted_row(m, at, rows)?,
            gate: gate_of(m, at, f.present_if.as_ref())?,
        },
        BitSize::Embed | BitSize::Repeat { .. } | BitSize::TlvChain { .. } => Row {
            kind: body_row(m, at, rows, said)?,
            gate: Gate::Always,
        },
        BitSize::Tail => return Err(UnstatedField::Tail),
    };
    if matches!(row.kind, Kind::Counted { .. }) && !input.owned {
        return Err(UnstatedField::NoStorageProfile);
    }
    Ok(row)
}

/// The store of a counted row's bytes or text into its member, the field `f`
/// of the codec named `codec`, as the template prints it.
fn counted_store(codec: &str, at: usize, f: &CodecField, said: &Said<'_>) -> serde_json::Value {
    serde_json::json!({
        "row": at,
        "ident": said.ident,
        "tag": rust_storage_tag(codec, &f.id),
        "storage_param": STORAGE_PARAM,
        "runtime": RUNTIME_CODEC,
        "bounded": said.bounded,
        "gated": f.present_if.is_some(),
    })
}

/// The table of the rows `rows` and the value's stores by their indexes,
/// which the codec template prints.
fn print(input: &Input<'_>, rows: Vec<Row>, said: &[Said<'_>]) -> PrintedTable {
    let m = input.m;
    let struct_name = input.struct_name;
    let name = format!("{}_FIELDS", to_upper_snake(&m.name));
    // The stores of each kind of row, by the row's index.
    let mut numbers = Vec::new();
    let mut counted = (Vec::new(), Vec::new());
    let mut bodies = Vec::new();
    let mut absent = Vec::new();
    for (at, (row, f)) in rows.iter().zip(&m.fields).enumerate() {
        let ident = said[at].ident;
        let gated = !matches!(row.gate, Gate::Always);
        match &row.kind {
            Kind::Uint { ty, .. } | Kind::Zint { ty, .. } => {
                numbers.push(serde_json::json!({
                    "row": at,
                    "ident": ident,
                    "cast": (*ty != "u64").then_some(*ty),
                    "gated": gated,
                }));
            }
            Kind::Counted { text, .. } => {
                let arms = if *text {
                    &mut counted.1
                } else {
                    &mut counted.0
                };
                arms.push(counted_store(&m.name, at, f, &said[at]));
            }
            Kind::Body { statement, handed } => {
                let bindings: Vec<serde_json::Value> = handed
                    .iter()
                    .enumerate()
                    .map(|(i, h)| {
                        serde_json::json!({
                            "binding": handed_binding(i, &h.local, h.ty),
                            "local": h.local,
                        })
                    })
                    .collect();
                bodies.push(serde_json::json!({
                    "row": at,
                    "bindings": bindings,
                    "statement": statement,
                    "kinds": chain_kinds(m, at),
                }));
            }
        }
        if gated {
            absent.push(serde_json::json!({ "row": at, "ident": ident }));
        }
    }
    let (impl_generics, value_type) = if input.owned {
        (
            format!("<'f, {STORAGE_PARAM}: {struct_name}From<'f>>"),
            format!("{struct_name}In<{STORAGE_PARAM}>"),
        )
    } else {
        ("<'f>".to_string(), struct_name.to_string())
    };
    let inputs = m
        .flag_inputs
        .iter()
        .map(|declared| {
            format!(
                "u64::from({})",
                filters::to_snake_case(declared.name.clone())
            )
        })
        .collect::<Vec<_>>()
        .join(", ");
    PrintedTable {
        name,
        inputs,
        rows,
        struct_name: struct_name.to_string(),
        stores: serde_json::json!({
            "impl_generics": impl_generics,
            "value_type": value_type,
            "number": numbers,
            "bytes": counted.0,
            "text": counted.1,
            "read": bodies,
            "absent": absent,
        }),
    }
}

/// `let {local}: {ty} = ..;`: the binding of the number at `i` among those a
/// body row is handed (`inputs`), by the name its statement reads it by. The
/// read of a body and its write bind them alike. A `bool` is a kind of a
/// chain's entries, handed as one bit.
pub(super) fn handed_binding(i: usize, local: &str, ty: &str) -> String {
    let number = if i == 0 {
        "inputs.first().copied().unwrap_or(0)".to_string()
    } else {
        format!("inputs.get({i}).copied().unwrap_or(0)")
    };
    let number = match ty {
        "u64" => number,
        "bool" => format!("{number} != 0"),
        _ => format!("{number} as {ty}"),
    };
    format!("let {local}: {ty} = {number};")
}

/// What a number the encode derives is computed from, as the runtime's
/// `Derived` says it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum DerivedFrom {
    /// 1 where the field of this row is present, 0 where it is not.
    Present(usize),
    /// The count of bytes of the byte string or text of this row.
    Count(usize),
}

/// A number the encode writes into the bits `into` of a row, computed by the
/// runtime's writer from `from` (docs/adr/0155 section 3).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct Derive {
    /// The bits of the row the number is written into.
    pub(super) into: Bits,
    /// What it is computed from.
    pub(super) from: DerivedFrom,
}

impl Derive {
    /// The derive as the template prints a runtime `Derive` of it.
    fn context(self) -> serde_json::Value {
        let (kind, row) = match self.from {
            DerivedFrom::Present(row) => ("Present", row),
            DerivedFrom::Count(row) => ("Count", row),
        };
        serde_json::json!({
            "into": self.into.context(),
            "from": { "kind": kind, "row": row },
        })
    }
}

/// The numbers the encode of `plan` derives, each as a derive of a row of
/// `table` the runtime's writer computes (docs/adr/0155 section 3), or why
/// the codec keeps its write printed: a derivation no row states.
///
/// A flag set exactly where one field is present and the count of bytes of a
/// byte string or a text a row counts are derives of rows; a count of a list's
/// entries, the length of an embedded body, a flag a body or an arm says, the
/// tag of a variant and the arms of a group are not. The end flag of a chain's
/// entries is the chain's to set as it writes each, and no number of the
/// codec's own. Every counted row must be counted by a derived count, or the
/// value would state a count its bytes need not hold.
pub(super) fn derives(
    plan: &EncodePlan<'_>,
    table: &PrintedTable,
) -> Result<Vec<Derive>, NotWrittenByTable> {
    let m = plan.model;
    let mut derives: Vec<Derive> = Vec::new();
    for planned in &plan.derivations {
        let element = planned.element.to_string();
        let derive = derive_of(m, table, planned).map_err(|form| NotWrittenByTable::Derive {
            element: element.clone(),
            form,
        })?;
        let Some(derive) = derive else {
            continue;
        };
        if derives.iter().any(|other| other.into == derive.into) {
            return Err(NotWrittenByTable::Derive {
                element,
                form: UnstatedDerive::Twice,
            });
        }
        derives.push(derive);
    }
    for (at, row) in table.rows.iter().enumerate() {
        if let Kind::Counted { len, .. } = row.kind {
            let counted = Derive {
                into: Bits::whole(Place::Row(len)),
                from: DerivedFrom::Count(at),
            };
            if !derives.contains(&counted) {
                return Err(NotWrittenByTable::Field {
                    field: m.fields[at].id.clone(),
                    form: UnwrittenField::CountStated,
                });
            }
        }
    }
    Ok(derives)
}

/// The derive of a row the derivation `planned` is, `None` where it is no
/// number of the codec's own (a chain's end flag), or what no row states of it.
fn derive_of(
    m: &CodecModel,
    table: &PrintedTable,
    planned: &encode_plan::Derived<'_>,
) -> Result<Option<Derive>, UnstatedDerive> {
    let row_of = |id: &str| {
        m.fields
            .iter()
            .position(|f| f.id == id)
            .filter(|&row| row < table.rows.len())
            .ok_or(UnstatedDerive::NoRow)
    };
    let number = |row: usize| match table.rows[row].kind {
        Kind::Uint { .. } | Kind::Zint { .. } => Ok(()),
        _ => Err(UnstatedDerive::NoNumberRow),
    };
    let derive = match (&planned.element, &planned.by) {
        (_, encode_plan::Derive::ChainContinuation) => return Ok(None),
        (ElementId::Flag { field, flag }, encode_plan::Derive::Presence(presence)) => {
            let [(gated, when_clear)] = presence.of.as_slice() else {
                return Err(UnstatedDerive::SeveralFields);
            };
            if *when_clear {
                return Err(UnstatedDerive::SetWhereAbsent);
            }
            let carrier = row_of(field)?;
            number(carrier)?;
            let def = m.fields[carrier]
                .flags
                .iter()
                .find(|def| def.name == *flag)
                .ok_or(UnstatedDerive::NoFlag)?;
            Derive {
                into: Bits {
                    from: Place::Row(carrier),
                    shift: def.bit,
                    width: Some(def.width),
                },
                from: DerivedFrom::Present(row_of(&gated.id)?),
            }
        }
        (ElementId::Field(_), encode_plan::Derive::Number(derived)) => {
            let [measure] = derived.measures.as_slice() else {
                return Err(UnstatedDerive::SeveralMeasures);
            };
            let Measured::Bytes {
                field: content,
                offset: Offset::None,
            } = &measure.of
            else {
                return Err(UnstatedDerive::UnstatedMeasure);
            };
            let NumberInto::Field {
                field: into,
                narrow,
                absent,
            } = &derived.into
            else {
                return Err(UnstatedDerive::IntoFlag);
            };
            if *narrow == Narrow::Checked {
                return Err(UnstatedDerive::Narrowed);
            }
            if !matches!(absent, Absent::Never | Absent::Zero) {
                return Err(UnstatedDerive::StatedWhereAbsent);
            }
            let count = row_of(&into.id)?;
            number(count)?;
            let counted = row_of(&content.id)?;
            if !matches!(table.rows[counted].kind, Kind::Counted { len, .. } if len == count) {
                return Err(UnstatedDerive::CountsOtherRow);
            }
            Derive {
                into: Bits::whole(Place::Row(count)),
                from: DerivedFrom::Count(counted),
            }
        }
        _ => return Err(UnstatedDerive::NoRowNumber),
    };
    Ok(Some(derive))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::forge::model::ForgeDocument;
    use crate::DocumentLabel;

    /// The codec of the conformance resource `name`.
    fn codec(name: &str) -> CodecModel {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../tests/forge/resources")
            .join(format!("{name}.scxml"));
        let loaded = crate::load_forge_source(&path, &[])
            .unwrap_or_else(|e| panic!("{name} is readable: {e:?}"));
        let spelled = path.to_string_lossy();
        let parsed = crate::forge::parser::parse_forge_with_imports(
            loaded.text(),
            DocumentLabel::for_input_path(&spelled),
        )
        .unwrap_or_else(|e| panic!("{name} parses: {e:?}"))
        .unwrap_or_else(|| panic!("{name} is a forge document"));
        match parsed.document {
            ForgeDocument::Codec(m) => m,
            _ => panic!("{name} is a codec"),
        }
    }

    /// The context of one field, as the generation says it.
    fn context(pairs: &[(&str, &str)]) -> serde_json::Value {
        serde_json::Value::Object(
            pairs
                .iter()
                .map(|(key, value)| ((*key).to_string(), serde_json::Value::from(*value)))
                .collect(),
        )
    }

    /// [`lower`] of `m`, each field read by `wire` and held as it is read.
    fn lowered(m: &CodecModel, wire: &[serde_json::Value]) -> Result<PrintedTable, NotATable> {
        lower(&Input {
            m,
            stored: wire,
            wire,
            struct_name: &crate::filters::to_pascal_case(m.name.clone()),
            owned: true,
        })
    }

    /// A ZInt, a gated ZInt and the text it counts are three rows the walker
    /// reads, gated by the codec's input, and their stores are the value's.
    #[test]
    fn numbers_and_a_counted_text_are_rows_of_the_table() {
        let m = codec("codec_zenoh_wireexpr");
        let wire = [
            context(&[
                ("id", "id"),
                ("present_if_decode_stmt", "let id = cursor.read_vle_u64();"),
            ]),
            context(&[
                ("id", "suffix_len"),
                (
                    "present_if_decode_stmt",
                    "let suffix_len = if (n & 0x01u8) != 0 { let _v = cursor.read_vle_u64(); \
                     Some(_v) } else { None };",
                ),
            ]),
            context(&[
                ("id", "suffix"),
                ("present_if_decode_stmt", "let suffix = None;"),
                ("bounded_rebind", "let suffix = suffix;"),
            ]),
        ];
        let table = lowered(&m, &wire).unwrap_or_else(|why| panic!("not a table: {why}"));
        assert_eq!(table.name, "CODEC_ZENOH_WIREEXPR_FIELDS");
        assert_eq!(table.inputs, "u64::from(n)");
        let flag = Gate::Set(Bits {
            from: Place::Input(0),
            shift: 0,
            width: Some(1),
        });
        assert_eq!(
            table.rows,
            vec![
                Row {
                    kind: Kind::Zint {
                        bits: 64,
                        ty: "u64"
                    },
                    gate: Gate::Always,
                },
                Row {
                    kind: Kind::Zint {
                        bits: 64,
                        ty: "u64"
                    },
                    gate: flag.clone(),
                },
                Row {
                    kind: Kind::Counted {
                        text: true,
                        len: 1,
                        max: Some(65535),
                    },
                    gate: flag,
                },
            ]
        );
        let context = table.context(&[], &TableHome::Own);
        let stores = &context["stores"];
        assert_eq!(
            stores["number"],
            serde_json::json!([
                { "row": 0, "ident": "id", "cast": null, "gated": false },
                { "row": 1, "ident": "suffix_len", "cast": null, "gated": true },
            ]),
            "{context}"
        );
        assert_eq!(
            stores["text"],
            serde_json::json!([{
                "row": 2,
                "ident": "suffix",
                "tag": "CodecZenohWireexprSuffixField",
                "storage_param": "S",
                "runtime": RUNTIME_CODEC,
                "bounded": true,
                "gated": true,
            }]),
            "{context}"
        );
        assert_eq!(
            stores["absent"],
            serde_json::json!([
                { "row": 1, "ident": "suffix_len" },
                { "row": 2, "ident": "suffix" },
            ]),
            "{context}"
        );
        assert_eq!(
            (&stores["impl_generics"], &stores["value_type"]),
            (
                &serde_json::json!("<'f, S: CodecZenohWireexprFrom<'f>>"),
                &serde_json::json!("CodecZenohWireexprIn<S>")
            ),
            "{context}"
        );
        assert_eq!(
            context["rows"][2],
            serde_json::json!({
                "read": {
                    "kind": "Text",
                    "len": { "source": "Row", "index": 1, "shift": 0, "width": null },
                    "max": "65535",
                },
                "gate": {
                    "kind": "Set",
                    "bits": { "source": "Input", "index": 0, "shift": 0, "width": 1 },
                },
            }),
            "{context}"
        );
    }

    /// An embedded body and a chain are read by their printed statements, each
    /// handed the carrier its own gate reads.
    #[test]
    fn a_body_is_read_by_its_statement_handed_the_numbers_it_names() {
        let m = codec("codec_zenoh_msg_del");
        let embed = "if (header & 0x20u8) != 0 {\n    \
                     CodecZenohTimestampOwned::<S>::read_into(cursor, _out.timestamp.\
                     get_or_insert_with(CodecZenohTimestampOwned::<S>::default));\n} else {\n    \
                     _out.timestamp = None;\n}";
        let chain = "if (header & 0x80u8) != 0 {\n    _out.extensions = None;\n} else {\n    \
                     _out.extensions = None;\n}";
        let wire = [
            context(&[
                ("id", "header"),
                (
                    "present_if_decode_stmt",
                    "let header = { let raw = cursor.take_array::<1>(); raw[0] };",
                ),
            ]),
            context(&[("id", "timestamp"), ("embed_decode_stmt", embed)]),
            context(&[("id", "extensions"), ("tlv_chain_decode_stmt", chain)]),
        ];
        let table = lowered(&m, &wire).unwrap_or_else(|why| panic!("not a table: {why}"));
        let handed = vec![Handed {
            local: "header".to_string(),
            ty: "u8",
            from: Place::Row(0),
            bit: None,
        }];
        assert_eq!(
            table.rows,
            vec![
                Row {
                    kind: Kind::Uint {
                        bytes: 1,
                        big: true,
                        ty: "u8",
                    },
                    gate: Gate::Always,
                },
                Row {
                    kind: Kind::Body {
                        statement: embed.to_string(),
                        handed: handed.clone(),
                    },
                    gate: Gate::Always,
                },
                Row {
                    kind: Kind::Body {
                        statement: chain.to_string(),
                        handed,
                    },
                    gate: Gate::Always,
                },
            ]
        );
        let context = table.context(&[], &TableHome::Own);
        let stores = &context["stores"];
        let binding = serde_json::json!({
            "binding": "let header: u8 = inputs.first().copied().unwrap_or(0) as u8;",
            "local": "header",
        });
        assert_eq!(
            stores["read"],
            serde_json::json!([
                { "row": 1, "bindings": [binding.clone()], "statement": embed, "kinds": [] },
                { "row": 2, "bindings": [binding], "statement": chain, "kinds": [] },
            ]),
            "{context}"
        );
        assert_eq!(
            stores["number"],
            serde_json::json!([{ "row": 0, "ident": "header", "cast": "u8", "gated": false }]),
            "{context}"
        );
        assert_eq!(
            stores["absent"],
            serde_json::json!([]),
            "codec_zenoh_msg_del gates its bodies in their statements, and has an absent \
             store: {context}"
        );
        assert_eq!(
            context["rows"][1]["read"],
            serde_json::json!({
                "kind": "Body",
                "inputs": [{ "source": "Row", "index": 0, "shift": 0, "width": null }],
            }),
            "{context}"
        );
    }

    /// A codec that dispatches a variant is printed, and says why.
    #[test]
    fn a_variant_is_no_table() {
        let m = codec("codec_zenoh_request");
        let why = lowered(&m, &[]).expect_err("a variant is no table");
        assert_eq!(why, NotATable::Codec(CodecShape::Variant), "{why}");
    }

    /// A field gated by several flags (`a || b`) is a row whose gate is the
    /// disjunction of a clause for each, which the template prints as the
    /// runtime's `Gate::Any`.
    #[test]
    fn a_gate_of_several_clauses_is_a_disjunction_of_flags() {
        let m = codec("codec_present_if_disjunction");
        let wire = [
            context(&[
                ("id", "flags"),
                (
                    "present_if_decode_stmt",
                    "let flags = { let raw = cursor.take_array::<1>(); raw[0] };",
                ),
            ]),
            context(&[
                ("id", "seq"),
                (
                    "present_if_decode_stmt",
                    "let seq = if (flags & 0x01u8) != 0 || (flags & 0x02u8) != 0 { \
                     Some(0) } else { None };",
                ),
            ]),
        ];
        let table = lowered(&m, &wire).unwrap_or_else(|why| panic!("not a table: {why}"));
        let flag = |bit| Bits {
            from: Place::Row(0),
            shift: bit,
            width: Some(1),
        };
        assert_eq!(
            table.rows[1],
            Row {
                kind: Kind::Uint {
                    bytes: 2,
                    big: true,
                    ty: "u16",
                },
                gate: Gate::Any(vec![Gate::Set(flag(0)), Gate::Set(flag(1))]),
            }
        );
        let context = table.context(&[], &TableHome::Own);
        assert_eq!(
            context["rows"][1]["gate"],
            serde_json::json!({
                "kind": "Any",
                "bits": null,
                "clauses": [
                    {
                        "kind": "Set",
                        "bits": { "source": "Row", "index": 0, "shift": 0, "width": 1 },
                    },
                    {
                        "kind": "Set",
                        "bits": { "source": "Row", "index": 0, "shift": 1, "width": 1 },
                    },
                ],
            }),
            "{context}"
        );
        assert_eq!(
            (
                &context["stores"]["number"][1],
                &context["stores"]["absent"]
            ),
            (
                &serde_json::json!({ "row": 1, "ident": "seq", "cast": "u16", "gated": true }),
                &serde_json::json!([{ "row": 1, "ident": "seq" }]),
            ),
            "a field a disjunction gates is stored absent where it is closed: {context}"
        );
    }

    /// A field gated by a kind of an earlier chain's entries (`<chain>.has(<id>)`)
    /// is gated by the bit of the chain's row its read answers, and a body gated
    /// so is handed that bit as the local its statement reads.
    #[test]
    fn a_gate_on_a_chain_kind_is_a_bit_the_chain_row_answers() {
        let m = codec("codec_chain_has_marker");
        let chain = "let mut _found_extensions = [false; 1];\n\
                     if (header & 0x80u8) != 0 { _out.extensions = None; }\n\
                     let _has_extensions_18 = _found_extensions[0];";
        let slices = "if _has_extensions_18 { _out.slices = None; }";
        let wire = [
            context(&[
                ("id", "header"),
                (
                    "present_if_decode_stmt",
                    "let header = { let raw = cursor.take_array::<1>(); raw[0] };",
                ),
            ]),
            context(&[("id", "extensions"), ("tlv_chain_decode_stmt", chain)]),
            context(&[
                ("id", "payload_len"),
                (
                    "present_if_decode_stmt",
                    "let payload_len = if !_has_extensions_18 { Some(cursor.read_vle_u64()) } \
                     else { None };",
                ),
            ]),
            context(&[
                ("id", "payload"),
                ("present_if_decode_stmt", "let payload = None;"),
                ("bounded_rebind", "let payload = payload;"),
            ]),
            context(&[
                ("id", "slice_count"),
                (
                    "present_if_decode_stmt",
                    "let slice_count = if _has_extensions_18 { Some(cursor.read_vle_u32()) } \
                     else { None };",
                ),
            ]),
            context(&[("id", "slices"), ("repeat_decode_stmt", slices)]),
        ];
        let table = lowered(&m, &wire).unwrap_or_else(|why| panic!("not a table: {why}"));
        let kind = Bits {
            from: Place::Row(1),
            shift: 0,
            width: Some(1),
        };
        assert_eq!(
            table
                .rows
                .iter()
                .map(|row| row.gate.clone())
                .collect::<Vec<_>>(),
            vec![
                Gate::Always,
                Gate::Always,
                Gate::Clear(kind),
                Gate::Clear(kind),
                Gate::Set(kind),
                Gate::Always,
            ],
            "{:?}",
            table.rows
        );
        assert_eq!(
            table.rows[5].kind,
            Kind::Body {
                statement: slices.to_string(),
                handed: vec![Handed {
                    local: "_has_extensions_18".to_string(),
                    ty: "bool",
                    from: Place::Row(1),
                    bit: Some(0),
                }],
            }
        );
        assert!(
            table.reads_kinds(),
            "a table whose gates read a chain's kinds says so, and its write stays printed"
        );
        let context = table.context(&[], &TableHome::Own);
        let stores = &context["stores"]["read"];
        assert_eq!(
            (&stores[0]["kinds"], &stores[1]["kinds"]),
            (
                &serde_json::json!(["_has_extensions_18"]),
                &serde_json::json!([]),
            ),
            "the chain's read answers the kind its gates ask, the repeat's none: {context}"
        );
        assert_eq!(
            stores[1]["bindings"],
            serde_json::json!([{
                "binding": "let _has_extensions_18: bool = inputs.first().copied().unwrap_or(0) != 0;",
                "local": "_has_extensions_18",
            }]),
            "{context}"
        );
        assert_eq!(
            context["rows"][4]["gate"],
            serde_json::json!({
                "kind": "Set",
                "bits": { "source": "Row", "index": 1, "shift": 0, "width": 1 },
            }),
            "{context}"
        );
    }

    /// A body whose statement names a number a gate decides is no row: the
    /// walker hands a body numbers, not their absence.
    #[test]
    fn a_body_that_names_a_gated_number_is_no_table() {
        let m = codec("codec_zenoh_query");
        let wire = [
            context(&[
                ("id", "header"),
                (
                    "present_if_decode_stmt",
                    "let header = { let raw = cursor.take_array::<1>(); raw[0] };",
                ),
            ]),
            context(&[
                ("id", "consolidation"),
                (
                    "present_if_decode_stmt",
                    "let consolidation = if (header & 0x20u8) != 0 { Some(0) } else { None };",
                ),
            ]),
            context(&[
                ("id", "parameters_len"),
                (
                    "present_if_decode_stmt",
                    "let parameters_len = if (header & 0x40u8) != 0 { \
                     Some(cursor.read_vle_u64()) } else { None };",
                ),
            ]),
            context(&[
                ("id", "parameters"),
                ("present_if_decode_stmt", "let parameters = None;"),
            ]),
            context(&[
                ("id", "extensions"),
                (
                    "tlv_chain_decode_stmt",
                    "if (header & 0x80u8) != 0 { let _ = consolidation; }",
                ),
            ]),
        ];
        let why = lowered(&m, &wire).expect_err("a body names a gated number");
        assert_eq!(
            why,
            NotATable::Field {
                field: "extensions".to_string(),
                form: UnstatedField::NamesUnstatedNumber {
                    named: "consolidation".to_string(),
                },
            },
            "{why}"
        );
    }

    /// The context of each field of `codec_present_if_string`: a carrier, and
    /// the count and the text it gates.
    fn present_if_string_wire() -> [serde_json::Value; 3] {
        [
            context(&[
                ("id", "carrier"),
                (
                    "present_if_decode_stmt",
                    "let carrier = { let raw = cursor.take_array::<1>(); raw[0] };",
                ),
            ]),
            context(&[
                ("id", "text_len"),
                (
                    "present_if_decode_stmt",
                    "let text_len = if (carrier & 0x01u8) != 0 { Some(0) } else { None };",
                ),
            ]),
            context(&[
                ("id", "text"),
                ("present_if_decode_stmt", "let text = None;"),
                ("bounded_rebind", "let text = text;"),
            ]),
        ]
    }

    /// The flag a gated text is announced by and the count of its bytes are
    /// derives of the rows the text is read by, and the table is made with them
    /// where its write is the writer's.
    #[test]
    fn a_presence_flag_and_a_count_are_derives_of_rows() {
        let m = codec("codec_present_if_string");
        let table = lowered(&m, &present_if_string_wire())
            .unwrap_or_else(|why| panic!("not a table: {why}"));
        let plan = EncodePlan::of(&m, &[]);
        let derived =
            derives(&plan, &table).unwrap_or_else(|why| panic!("no derive of rows: {why:?}"));
        let flag = Derive {
            into: Bits {
                from: Place::Row(0),
                shift: 0,
                width: Some(1),
            },
            from: DerivedFrom::Present(2),
        };
        let count = Derive {
            into: Bits::whole(Place::Row(1)),
            from: DerivedFrom::Count(2),
        };
        assert_eq!(derived.len(), 2, "{derived:?}");
        assert!(derived.contains(&flag), "{derived:?}");
        assert!(derived.contains(&count), "{derived:?}");
        let context = table.context(&derived, &TableHome::Own);
        assert_eq!(
            context["uses"],
            serde_json::json!("Bits, Derive, Derived, FieldTable, Gate, Read, Row, Source"),
            "{context}"
        );
        assert_eq!(
            context["derived"],
            serde_json::json!([
                {
                    "into": { "source": "Row", "index": 0, "shift": 0, "width": 1 },
                    "from": { "kind": "Present", "row": 2 },
                },
                {
                    "into": { "source": "Row", "index": 1, "shift": 0, "width": null },
                    "from": { "kind": "Count", "row": 2 },
                },
            ]),
            "{context}"
        );
        let read_only = table.context(&[], &TableHome::Own);
        assert_eq!(
            (&read_only["derived"], &read_only["home"]),
            (&serde_json::json!([]), &serde_json::json!("own")),
            "a table no write derives a number of is made with none: {read_only}"
        );
        // docs/adr/0155 section 4: what the table is names no codec, so two
        // codecs of the same rows have one; the first lends its `static` to
        // the set, and a later one names it in place of one of its own.
        let rows = table.rows_key(&derived);
        assert!(
            rows.contains("\"derived\"") && !rows.contains("PRESENT_IF_STRING"),
            "{rows}"
        );
        assert_ne!(
            rows,
            table.rows_key(&[]),
            "the derived numbers are no part of the rows"
        );
        let lent = table.context(&derived, &TableHome::Lent);
        assert_eq!(lent["home"], serde_json::json!("lent"), "{lent}");
        let borrowed = table.context(
            &derived,
            &TableHome::Borrowed {
                path: "super::codec_a::CODEC_A_FIELDS".to_string(),
                rows,
            },
        );
        assert_eq!(
            (&borrowed["home"], &borrowed["path"], &borrowed["stores"]),
            (
                &serde_json::json!("borrowed"),
                &serde_json::json!("super::codec_a::CODEC_A_FIELDS"),
                &context["stores"]
            ),
            "a borrowed table is not the earlier codec's, with stores of its own: {borrowed}"
        );
    }

    /// A flag set where its field is absent is no derive the writer computes:
    /// the write of the codec stays printed.
    #[test]
    fn a_flag_set_where_its_field_is_absent_is_no_derive() {
        let m = codec("codec_present_if_negation");
        let wire = [
            context(&[
                ("id", "flags"),
                (
                    "present_if_decode_stmt",
                    "let flags = { let raw = cursor.take_array::<1>(); raw[0] };",
                ),
            ]),
            context(&[
                ("id", "seq"),
                (
                    "present_if_decode_stmt",
                    "let seq = if (flags & 0x01u8) == 0 { Some(0) } else { None };",
                ),
            ]),
        ];
        let table = lowered(&m, &wire).unwrap_or_else(|why| panic!("not a table: {why}"));
        let why = derives(&EncodePlan::of(&m, &[]), &table)
            .expect_err("a flag set where its field is absent is no derive");
        assert!(
            matches!(
                why,
                NotWrittenByTable::Derive {
                    form: UnstatedDerive::SetWhereAbsent,
                    ..
                }
            ),
            "{why:?}"
        );
    }

    /// A local is named where it stands alone, not as a member or a part of
    /// another name.
    #[test]
    fn a_local_is_named_where_it_stands_alone() {
        assert!(mentions("_out.header = header;", "header"));
        assert!(!mentions("_out.header = 0;", "header"));
        assert!(!mentions("let _n = len;", "n"));
        assert!(mentions("read(cursor, n)", "n"));
    }
}
