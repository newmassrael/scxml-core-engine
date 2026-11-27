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
use crate::forge::model::{
    BitSize, CodecEncoding, CodecField, CodecModel, Endian, PresentIfPredicate, PresentIfScope,
    SceType,
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
    /// The module's items: the `static` and the value's `TableOut`.
    pub(super) items: String,
}

/// Where the number a bit range reads is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Place {
    /// The number the row at this index read.
    Row(usize),
    /// The input of the codec at this index.
    Input(usize),
}

/// A bit range of a number: the whole number where `width` is `None`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Bits {
    from: Place,
    shift: u32,
    width: Option<u32>,
}

impl Bits {
    fn whole(from: Place) -> Self {
        Self {
            from,
            shift: 0,
            width: None,
        }
    }

    fn print(self) -> String {
        let from = match self.from {
            Place::Row(row) => format!("Source::Row({row})"),
            Place::Input(input) => format!("Source::Input({input})"),
        };
        match self.width {
            None => format!("Bits::whole({from})"),
            Some(width) => format!("Bits::range({from}, {}, {width})", self.shift),
        }
    }
}

/// Whether a row is read.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Gate {
    Always,
    Set(Bits),
    Clear(Bits),
}

impl Gate {
    fn print(self) -> String {
        match self {
            Gate::Always => "Gate::Always".to_string(),
            Gate::Set(bits) => format!("Gate::Set({})", bits.print()),
            Gate::Clear(bits) => format!("Gate::Clear({})", bits.print()),
        }
    }
}

/// A number a body's read is handed, by the name its statement reads it by.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Handed {
    /// The local the statement names.
    local: String,
    /// The Rust type of the local.
    ty: &'static str,
    /// Where the number is.
    from: Place,
}

/// How a row's field is read.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Kind {
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
struct Row {
    kind: Kind,
    gate: Gate,
}

impl Row {
    fn print(&self) -> String {
        let read = match &self.kind {
            Kind::Uint { bytes, big, .. } => format!("Read::Uint {{ bytes: {bytes}, big: {big} }}"),
            Kind::Zint { bits, .. } => format!("Read::Zint {{ bits: {bits} }}"),
            Kind::Counted { text, len, max } => format!(
                "Read::{} {{ len: {}, max: {} }}",
                if *text { "Text" } else { "Bytes" },
                Bits::whole(Place::Row(*len)).print(),
                max.map_or_else(|| "u64::MAX".to_string(), |max| max.to_string())
            ),
            Kind::Body { handed, .. } => format!(
                "Read::Body {{ inputs: &[{}] }}",
                handed
                    .iter()
                    .map(|h| Bits::whole(h.from).print())
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
        };
        format!("Row {{ read: {read}, gate: {} }}", self.gate.print())
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
fn mentions(text: &str, name: &str) -> bool {
    let ident = |c: char| c.is_ascii_alphanumeric() || c == '_';
    !name.is_empty()
        && text.match_indices(name).any(|(at, _)| {
            let before = text[..at].chars().next_back();
            let after = text[at + name.len()..].chars().next();
            !before.is_some_and(|c| ident(c) || c == '.' || c == '#') && !after.is_some_and(ident)
        })
}

/// Whether a gate reads the entries of a chain, which no row states.
fn reads_a_chain(gate: &PresentIfPredicate) -> bool {
    gate.scope == PresentIfScope::Chain || gate.or_with.as_deref().is_some_and(reads_a_chain)
}

/// The gate of the field at `at`, a flag of an earlier carrier no gate decides
/// or an input of the codec.
fn gate_of(m: &CodecModel, at: usize, gate: Option<&PresentIfPredicate>) -> Result<Gate, String> {
    let Some(gate) = gate else {
        return Ok(Gate::Always);
    };
    let field = &m.fields[at].id;
    if gate.or_with.is_some() {
        return Err(format!("field '{field}' is gated by several clauses"));
    }
    let bits = match gate.scope {
        PresentIfScope::Local => {
            let carrier = m.fields[..at]
                .iter()
                .position(|f| f.id == gate.field_id)
                .ok_or_else(|| format!("field '{field}' is gated by no earlier carrier"))?;
            if m.fields[carrier].present_if.is_some() {
                return Err(format!(
                    "field '{field}' is gated by a carrier a gate decides"
                ));
            }
            let flag = m.fields[carrier]
                .flags
                .iter()
                .find(|flag| flag.name == gate.flag_name)
                .ok_or_else(|| format!("field '{field}' is gated by no flag of its carrier"))?;
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
                .ok_or_else(|| format!("field '{field}' is gated by no input of the codec"))?;
            Bits {
                from: Place::Input(input),
                shift: 0,
                width: Some(m.flag_inputs[input].width),
            }
        }
        PresentIfScope::Chain => {
            return Err(format!("field '{field}' is gated by a chain's entries"));
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
fn number_row(m: &CodecModel, f: &CodecField, read: Option<&str>) -> Result<Kind, String> {
    let (ty, width) =
        unsigned(&f.sce_type).ok_or_else(|| format!("field '{}' is no unsigned integer", f.id))?;
    // The printed read of a number refuses nothing but a short frame; a read
    // that refuses a value (a range, a conversion) is one no row states.
    let read = read.ok_or_else(|| format!("field '{}' has no printed read", f.id))?;
    if ["fail(", "settle(", "hold(", "ok_or("]
        .iter()
        .any(|refusal| read.contains(refusal))
    {
        return Err(format!(
            "field '{}' is read by a read that refuses a value",
            f.id
        ));
    }
    match f.bit_size {
        BitSize::Fixed { bits } if bits == width && f.bit_offset.is_none() => {
            let big = match f.effective_endian(m.default_endian) {
                Endian::Big => true,
                Endian::Little => false,
                Endian::Native if bits == 8 => true,
                Endian::Native => {
                    return Err(format!("field '{}' is in the native byte order", f.id));
                }
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
        _ => Err(format!(
            "field '{}' is a number of {} bits no row reads",
            f.id,
            f.bit_size.as_attr()
        )),
    }
}

/// The row of a byte string or a text counted by an earlier number that the
/// same gate decides.
fn counted_row(m: &CodecModel, at: usize, rows: &[Row]) -> Result<Kind, String> {
    let f = &m.fields[at];
    let text = match f.sce_type {
        SceType::String => true,
        SceType::Bytes => false,
        _ => return Err(format!("field '{}' is a length of no bytes or text", f.id)),
    };
    let count = f
        .length_field
        .as_deref()
        .ok_or_else(|| format!("field '{}' is counted by no field", f.id))?;
    let len = m.fields[..at]
        .iter()
        .position(|other| other.id == count)
        .ok_or_else(|| format!("field '{}' is counted by no earlier number", f.id))?;
    if !matches!(rows[len].kind, Kind::Uint { .. } | Kind::Zint { .. }) {
        return Err(format!("field '{}' is counted by no earlier number", f.id));
    }
    if m.fields[len].present_if != f.present_if {
        return Err(format!(
            "field '{}' is gated otherwise than its count",
            f.id
        ));
    }
    let bound = f
        .length_bound(&m.fields)
        .ok_or_else(|| format!("field '{}' has no length bound", f.id))?;
    let (least, most) = bound.carried_range();
    if least.is_some_and(|least| least > 0) {
        return Err(format!("field '{}' is held to a least length", f.id));
    }
    Ok(Kind::Counted {
        text,
        len,
        max: most,
    })
}

/// The row of a body the value reads itself: the printed statement, gated by
/// itself, handed each earlier number and input it names.
fn body_row(m: &CodecModel, at: usize, rows: &[Row], said: &[Said<'_>]) -> Result<Kind, String> {
    let f = &m.fields[at];
    let statement = said[at]
        .statement
        .ok_or_else(|| format!("field '{}' has no printed read", f.id))?;
    let mut handed = Vec::new();
    for (row, earlier) in rows.iter().enumerate() {
        if !mentions(statement, said[row].local) {
            continue;
        }
        let ty = match (&earlier.kind, earlier.gate) {
            (Kind::Uint { ty, .. } | Kind::Zint { ty, .. }, Gate::Always) => *ty,
            _ => {
                return Err(format!(
                    "field '{}' is read by a statement that names '{}', which no number \
                     states",
                    f.id, m.fields[row].id
                ));
            }
        };
        handed.push(Handed {
            local: said[row].local.to_string(),
            ty,
            from: Place::Row(row),
        });
    }
    for (input, declared) in m.flag_inputs.iter().enumerate() {
        let local = filters::to_snake_case(declared.name.clone());
        if mentions(statement, &local) {
            handed.push(Handed {
                local,
                ty: "u8",
                from: Place::Input(input),
            });
        }
    }
    if handed.len() > ROW_INPUTS {
        return Err(format!(
            "field '{}' is read by a statement that names more than {ROW_INPUTS} numbers",
            f.id
        ));
    }
    Ok(Kind::Body {
        statement: statement.to_string(),
        handed,
    })
}

/// The table of the codec `input.m`, read at a cold site, or why it is not
/// one: the first field no row states, or the shape of the codec.
pub(super) fn lower(input: &Input<'_>) -> Result<PrintedTable, String> {
    let m = input.m;
    if m.encoding != CodecEncoding::Positional {
        return Err("the codec is not positional".to_string());
    }
    if m.variant.is_some() {
        return Err("the codec dispatches a variant".to_string());
    }
    if m.fields.is_empty() {
        return Err("the codec has no field".to_string());
    }
    if m.fields.len() > TABLE_ROWS {
        return Err(format!("the codec has more than {TABLE_ROWS} fields"));
    }
    if input.wire.len() != m.fields.len() || input.stored.len() != m.fields.len() {
        return Err("a gate group holds fields of the codec as one".to_string());
    }
    if m.fields
        .iter()
        .filter_map(|f| f.present_if.as_ref())
        .any(reads_a_chain)
    {
        return Err("a gate reads the entries of a chain".to_string());
    }
    let said: Vec<Said<'_>> = input
        .wire
        .iter()
        .map(Said::of)
        .collect::<Option<_>>()
        .ok_or_else(|| "a field has no context".to_string())?;
    let mut rows: Vec<Row> = Vec::with_capacity(m.fields.len());
    for (at, (f, stored)) in m.fields.iter().zip(input.stored).enumerate() {
        if stored.get("assign_stmt").is_some() || stored.get("assign_expr").is_some() {
            return Err(format!("field '{}' is held by a gate group", f.id));
        }
        if f.dma_burst_align.is_some()
            || f.length_arith.is_some()
            || f.embed_length_from.is_some()
            || f.quantity.is_some()
        {
            return Err(format!("field '{}' has a form no row states", f.id));
        }
        let row = match f.bit_size {
            BitSize::Fixed { .. } | BitSize::Vle { .. } => Row {
                kind: number_row(m, f, said[at].read)?,
                gate: gate_of(m, at, f.present_if.as_ref())?,
            },
            BitSize::LengthRef => Row {
                kind: counted_row(m, at, &rows)?,
                gate: gate_of(m, at, f.present_if.as_ref())?,
            },
            BitSize::Embed | BitSize::Repeat { .. } | BitSize::TlvChain { .. } => Row {
                kind: body_row(m, at, &rows, &said)?,
                gate: Gate::Always,
            },
            BitSize::Tail => {
                return Err(format!("field '{}' is the bytes left in the frame", f.id));
            }
        };
        if matches!(row.kind, Kind::Counted { .. }) && !input.owned {
            return Err(format!("field '{}' is held by no storage profile", f.id));
        }
        rows.push(row);
    }
    Ok(print(input, &rows, &said))
}

/// `if row == N { .. }` where one row has a store, a `match` otherwise.
fn dispatch(arms: &[(usize, String)], otherwise: &str) -> String {
    match arms {
        [(row, body)] if otherwise.is_empty() => {
            format!("        if row == {row} {{\n{body}        }}\n")
        }
        _ => {
            let mut out = String::from("        match row {\n");
            for (row, body) in arms {
                out.push_str(&format!("            {row} => {{\n{body}            }}\n"));
            }
            let otherwise = if otherwise.is_empty() {
                "{}"
            } else {
                otherwise
            };
            out.push_str(&format!("            _ => {otherwise},\n        }}\n"));
            out
        }
    }
}

/// The store of a counted row's bytes or text, `value`, into its member, the
/// field `f` of the codec named `codec`.
fn counted_store(codec: &str, f: &CodecField, said: &Said<'_>, value: &str) -> String {
    let ident = said.ident;
    let tag = rust_storage_tag(codec, &f.id);
    let gated = f.present_if.is_some();
    let mut out = String::new();
    let value = if said.bounded {
        out.push_str(&format!(
            "                let value = {RUNTIME_CODEC}::Bounded::new({value}).ok_or(CodecError::LengthOutOfRange)?;\n"
        ));
        "value"
    } else {
        value
    };
    let (held, none) = if gated {
        ("Some(stored)", "None")
    } else {
        ("stored", "Default::default()")
    };
    out.push_str(&format!(
        "                match {value}.store::<{STORAGE_PARAM}, {tag}>() {{\n\
         \x20                   Ok(stored) => {{\n\
         \x20                       self.{ident} = {held};\n\
         \x20                       Ok(())\n\
         \x20                   }}\n\
         \x20                   Err(refused) => {{\n\
         \x20                       self.{ident} = {none};\n\
         \x20                       Err(refused)\n\
         \x20                   }}\n\
         \x20               }}\n"
    ));
    out
}

/// The module's items and the walker's call of the table `rows`.
fn print(input: &Input<'_>, rows: &[Row], said: &[Said<'_>]) -> PrintedTable {
    let m = input.m;
    let struct_name = input.struct_name;
    let name = format!("{}_FIELDS", to_upper_snake(&m.name));
    let rows_text: String = rows
        .iter()
        .map(|row| format!("        {},\n", row.print()))
        .collect();
    let mut uses = vec!["FieldTable", "Gate", "Read", "Row"];
    if rows.iter().any(Row::names_bits) {
        uses.extend(["Bits", "Source"]);
        uses.sort_unstable();
    }
    let mut items = format!(
        "/// The fields of `{struct_name}` in wire order, read by the runtime's one\n\
         /// walker: the codec is read once per event, not on the per-message path\n\
         /// (docs/adr/0155).\n\
         static {name}: {RUNTIME_CODEC}::FieldTable = {{\n\
         \x20   use {RUNTIME_CODEC}::{{{}}};\n\
         \x20   const ROWS: &[Row] = &[\n{rows_text}    ];\n\
         \x20   FieldTable::new(ROWS)\n\
         }};\n\n",
        uses.join(", ")
    );
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
                let value = if *ty == "u64" {
                    "value".to_string()
                } else {
                    format!("value as {ty}")
                };
                let value = if gated {
                    format!("Some({value})")
                } else {
                    value
                };
                numbers.push((at, format!("                self.{ident} = {value};\n")));
            }
            Kind::Counted { text, .. } => {
                let (arms, value) = if *text {
                    (&mut counted.1, "text")
                } else {
                    (&mut counted.0, "bytes")
                };
                arms.push((at, counted_store(&m.name, f, &said[at], value)));
            }
            Kind::Body { statement, handed } => {
                let mut body = String::new();
                for (i, h) in handed.iter().enumerate() {
                    let number = if i == 0 {
                        "inputs.first().copied().unwrap_or(0)".to_string()
                    } else {
                        format!("inputs.get({i}).copied().unwrap_or(0)")
                    };
                    let number = if h.ty == "u64" {
                        number
                    } else {
                        format!("{number} as {}", h.ty)
                    };
                    body.push_str(&format!(
                        "                let {}: {} = {number};\n                let _ = {};\n",
                        h.local, h.ty, h.local
                    ));
                }
                body.push_str(&format!("                {statement}\n"));
                bodies.push((at, body));
            }
        }
        if gated {
            absent.push((at, format!("                self.{ident} = None;\n")));
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
    items.push_str(&format!(
        "// The stores of `{struct_name}` by the index of each row of {name}.\n\
         impl{impl_generics} {RUNTIME_CODEC}::TableOut<'f> for {value_type} {{\n"
    ));
    let mut methods = Vec::new();
    if !numbers.is_empty() {
        methods.push(format!(
            "    fn number(&mut self, row: usize, value: u64) {{\n{}    }}\n",
            dispatch(&numbers, "")
        ));
    }
    if !counted.0.is_empty() {
        methods.push(format!(
            "    fn bytes(&mut self, row: usize, bytes: &'f [u8]) -> Result<(), CodecError> {{\n{}    }}\n",
            dispatch(&counted.0, "Ok(())")
        ));
    }
    if !counted.1.is_empty() {
        methods.push(format!(
            "    fn text(&mut self, row: usize, text: &'f str) -> Result<(), CodecError> {{\n{}    }}\n",
            dispatch(&counted.1, "Ok(())")
        ));
    }
    if !bodies.is_empty() {
        methods.push(format!(
            "    fn read(&mut self, row: usize, cursor: &mut SceReader<'f>, inputs: &[u64]) {{\n\
             \x20       let _ = inputs;\n\
             \x20       let _out = self;\n{}    }}\n",
            dispatch(&bodies, "")
        ));
    }
    if !absent.is_empty() {
        methods.push(format!(
            "    fn absent(&mut self, row: usize) {{\n{}    }}\n",
            dispatch(&absent, "")
        ));
    }
    items.push_str(&methods.join("\n"));
    items.push_str("}\n");
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
        items,
    }
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
    fn lowered(m: &CodecModel, wire: &[serde_json::Value]) -> Result<PrintedTable, String> {
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
        for printed in [
            "Row { read: Read::Zint { bits: 64 }, gate: Gate::Always },",
            "Row { read: Read::Zint { bits: 64 }, gate: \
             Gate::Set(Bits::range(Source::Input(0), 0, 1)) },",
            "Row { read: Read::Text { len: Bits::whole(Source::Row(1)), max: 65535 }, gate: \
             Gate::Set(Bits::range(Source::Input(0), 0, 1)) },",
            "self.id = value;",
            "self.suffix_len = Some(value);",
            "self.suffix = None;",
            "Bounded::new(text).ok_or(CodecError::LengthOutOfRange)?;",
            "match value.store::<S, CodecZenohWireexprSuffixField>() {",
            "impl<'f, S: CodecZenohWireexprFrom<'f>> ::sce_forge_runtime::codec::TableOut<'f> \
             for CodecZenohWireexprIn<S> {",
        ] {
            assert!(
                table.items.contains(printed),
                "the table of codec_zenoh_wireexpr has no `{printed}`:\n{}",
                table.items
            );
        }
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
        for printed in [
            "Row { read: Read::Uint { bytes: 1, big: true }, gate: Gate::Always },",
            "Row { read: Read::Body { inputs: &[Bits::whole(Source::Row(0))] }, gate: \
             Gate::Always },",
            "let header: u8 = inputs.first().copied().unwrap_or(0) as u8;",
            "self.header = value as u8;",
            embed,
            chain,
        ] {
            assert!(
                table.items.contains(printed),
                "the table of codec_zenoh_msg_del has no `{printed}`:\n{}",
                table.items
            );
        }
        assert!(
            !table.items.contains("fn absent("),
            "codec_zenoh_msg_del gates its bodies in their statements, and has an absent \
             store:\n{}",
            table.items
        );
    }

    /// A codec that dispatches a variant is printed, and says why.
    #[test]
    fn a_variant_is_no_table() {
        let m = codec("codec_zenoh_request");
        let why = lowered(&m, &[]).expect_err("a variant is no table");
        assert!(why.contains("variant"), "{why}");
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
        assert!(why.contains("consolidation"), "{why}");
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
