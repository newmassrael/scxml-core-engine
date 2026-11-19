// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! The source interface of a CBOR codec (docs/adr/0058, SCE_FORGE.md §4.6.1).
//!
//! A CBOR map has no length, no count and no flag the encoding derives from
//! another element in the sense a positional frame has: the number of entries
//! in the map head is what the encoding counts, and nothing a value says. So
//! every entry is presented, and the interface is one accessor for each: the
//! value itself for a required entry, an `Option` for one that may be absent.
//!
//! The encoding rule is the codec's own, written once against the interface:
//! the map head counts the entries present, and the entries follow in
//! ascending key order with the shortest heads (RFC 8949 §4.2.1). A string or
//! byte string with a bound is presented within it (`Text`, `Bytes`;
//! docs/adr/0077 §3), and a list, a record or a union is lent as the type a
//! schema unit holds it in (docs/adr/0101, section 5.3), so only a list of
//! fewer elements than its lower count, and a record whose byte string is past
//! its bound, can be refused, and a refusal found by writing into a sink that
//! only counts leaves the caller's sink untouched. An enum is the integer of
//! its declared number, of either sign (docs/adr/0101, section 2.1).
//!
//! The same module prints the reads and the writes of every value an entry
//! holds below its own level (docs/adr/0101, sections 2.1 and 5.1): a walk of
//! the plan's shape, with one read, one write and, where a value of it can be
//! refused, one check for each record and each union document the codec
//! reaches, named for that document and shared wherever it is met.

use super::super::*;
use crate::forge::cbor_codec::{self, CborComposite as Composite};
use crate::forge::codec_totality::RefusalKind;
use crate::forge::encode_plan::{CborEnum, CborItem, CborPlan, CborRecord, CborShape, CborUnion};
use crate::forge::model::CborEntry;
use crate::rust_derive_policy::RustDeriveCategory;

/// What the interface of one codec is made from.
pub(in super::super) struct Input<'a> {
    pub(in super::super) m: &'a CodecModel,
    pub(in super::super) imports: &'a [ImportContext],
    pub(in super::super) l: &'a LangCtx,
    /// `to_pascal_case(m.name)`.
    pub(in super::super) struct_name: &'a str,
    /// `<'a>` when the map borrows a string or bytes entry, otherwise nothing.
    pub(in super::super) struct_lifetime: &'a str,
    /// Whether the function that writes is always inlined (`--inline-encode`).
    pub(in super::super) inline_encode: bool,
}

/// The type an accessor returns for the value of `e` when it is present: a
/// string or byte string within its bounds where it has them (docs/adr/0077
/// §3), so a source cannot present a value the encode would have to refuse. A
/// list, a record and a union are lent: a reference to the value the map
/// holds, whose type holds no more elements than each list's bound.
fn value_type(input: &Input<'_>, e: &CborEntry) -> String {
    let value = cbor_codec::rust_view_value_type(input.l, input.imports, e, "'_");
    if e.leaf_type().is_none() {
        format!("&{value}")
    } else {
        value
    }
}

/// Whether the map lends a present value of `shape` by reference: a list, a
/// record or a union, which it holds by value.
fn lent(shape: &CborShape) -> bool {
    matches!(
        shape,
        CborShape::List { .. } | CborShape::Record(_) | CborShape::Union(_)
    )
}

fn accessor_type(input: &Input<'_>, item: &CborItem<'_>) -> String {
    let value = value_type(input, item.entry);
    if item.required {
        value
    } else {
        input.l.codec_optional_of(&value)
    }
}

/// The expression that reads the entry `acc` of `item` out of a value that
/// holds it as the accessor's type does but by value, as the type
/// [`accessor_type`] gives: a list, a record and a union lent by reference, and
/// any other entry read as it is held. The read the decoded map's own
/// interface makes.
fn held_read(item: &CborItem<'_>, acc: &str) -> String {
    match (lent(&item.value), item.required) {
        (true, true) => format!("&self.{acc}"),
        (true, false) => format!("self.{acc}.as_ref()"),
        (false, _) => format!("self.{acc}"),
    }
}

/// The expression that reads the entry `acc` of `item` out of the owned mirror as
/// the type [`accessor_type`] gives: the `Bounded` a string or a byte string is held
/// in lends it with its bounds, a list, a record and a union are lent as the map
/// lends them, and any other entry is read as it is held. The read the mirror's
/// `as_borrowed` reaches through the interface.
fn owned_read(item: &CborItem<'_>, acc: &str) -> String {
    let runtime = cbor_codec::RUST_RUNTIME_CODEC;
    let read = match item.value {
        CborShape::Text { .. } => Some(format!("{runtime}::Bounded::as_text")),
        CborShape::Bytes { .. } => Some(format!("{runtime}::Bounded::as_bytes")),
        _ => None,
    };
    match (read, item.required) {
        (Some(read), true) => format!("{read}(&self.{acc})"),
        (Some(read), false) => format!("self.{acc}.as_ref().map({read})"),
        (None, _) => held_read(item, acc),
    }
}

/// The statement that writes the value `v` of `item`, after its key. A refusal
/// is written where the plan's totality names one, and only there.
fn write_value(input: &Input<'_>, plan: &CborPlan<'_>, item: &CborItem<'_>) -> String {
    // A string or byte string is held within its bounds by its type
    // (docs/adr/0077 §3; every one states its bound, docs/adr/0101 section
    // 1.3), so there is nothing to check; the value written is the `str` or
    // slice it holds.
    let held = "v.into_inner()";
    match &item.value {
        CborShape::Bool => "cbor::write_bool(w, v)?;".to_string(),
        CborShape::Text { .. } => format!("cbor::write_text(w, {held})?;"),
        CborShape::Bytes { .. } => format!("cbor::write_bytes(w, {held})?;"),
        CborShape::Enum(enumeration) => {
            let present = item
                .entry
                .leaf_type()
                .expect("an enum entry is a leaf entry");
            let underlying = input.l.codec_carrier_expr(present, "v");
            // The integer of the variant's declared number, major type 1 below
            // zero (docs/adr/0101, section 2.1): every value of a carrier of
            // either sign is written.
            match &enumeration.carrier {
                SceType::Int64 => format!("cbor::write_int(w, {underlying})?;"),
                carrier if carrier.is_signed() => {
                    format!("cbor::write_int(w, i64::from({underlying}))?;")
                }
                SceType::Uint64 => format!("cbor::write_uint(w, {underlying})?;"),
                _ => format!("cbor::write_uint(w, u64::from({underlying}))?;"),
            }
        }
        // A number is presented by value, and a list, a record and a union are
        // lent. Each list holds no more than its bound by its type; a lower
        // count, and a byte string of a record past its bound, are refused
        // where the plan names them, before anything of the value is written.
        shape => {
            let access = if lent(shape) {
                Access::Ref("v".to_string())
            } else {
                Access::Place("v".to_string())
            };
            let mut lines = Vec::new();
            if can_refuse(shape, Held::Composed) {
                let refusal = plan
                    .totality
                    .require(RefusalKind::CborOutOfRange, &item.entry.id);
                lines.extend(check_stmts(shape, &access, Held::Composed, 0, &refusal));
            }
            lines.extend(write_stmts(shape, &access, 0));
            lines.join("\n")
        }
    }
}

// ── Values below an entry's own level ───────────────────────────────────

/// Where a value stands in the type a schema unit declares, which decides the
/// type the language holds a byte string in (docs/adr/0086, section 7.1): a
/// record's own field, bare or optional, is held as the unit holds one entry,
/// a byte string in the language's own vector, which carries no bound; every
/// other value, inside a list or held whole by an entry, in the bounded types
/// of a composition.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Held {
    Field,
    Composed,
}

/// How a printed statement reaches a value: a place of its type, which a read
/// of a number copies, or an expression that is a reference to it.
enum Access {
    Place(String),
    Ref(String),
}

impl Access {
    /// The value itself, for a number, a `bool` or an enum, each `Copy`.
    fn value(&self) -> String {
        match self {
            Access::Place(place) => place.clone(),
            Access::Ref(reference) => format!("*{reference}"),
        }
    }

    /// A reference to the value.
    fn reference(&self) -> String {
        match self {
            Access::Place(place) => format!("&{place}"),
            Access::Ref(reference) => reference.clone(),
        }
    }

    /// The call of the method `method` of no argument on the value.
    fn call(&self, method: &str) -> String {
        match self {
            Access::Place(at) | Access::Ref(at) => format!("{at}.{method}()"),
        }
    }
}

/// The name a walk binds at `depth`: `base` at the outermost, `base1`, `base2`
/// below it, so a nested walk names its own.
fn bound_name(base: &str, depth: usize) -> String {
    if depth == 0 {
        base.to_string()
    } else {
        format!("{base}{depth}")
    }
}

/// The module the document `name` is generated as, a sibling of the codec's
/// (`use super::<module>;`, the line every import of it writes).
fn module_of(name: &str) -> String {
    filters::to_snake_case(name.to_string())
}

/// The type the unit of the union document `union` declares, through its
/// module.
fn union_type(union: &CborUnion) -> String {
    format!(
        "{}::{}",
        module_of(&union.name),
        filters::to_pascal_case(union.name.clone())
    )
}

/// The codec's read, write and check of a value of the record or union
/// document `name` (`what` is `record` or `union`), as `[decode, encode,
/// check]`: functions of the module, named for the document, so a document met
/// at two places, under two aliases, is read by one function.
fn composite_fns(what: &str, name: &str) -> [String; 3] {
    let module = module_of(name);
    [
        format!("decode_{what}_{module}"),
        format!("encode_{what}_{module}"),
        format!("check_{what}_{module}"),
    ]
}

/// `text` with every line after its first indented by `columns`, for a
/// multi-line expression placed after the indent of the line it starts on.
fn continued(text: &str, columns: usize) -> String {
    let pad = " ".repeat(columns);
    let mut lines = text.lines();
    let mut out = lines.next().unwrap_or_default().to_string();
    for line in lines {
        out.push('\n');
        if !line.is_empty() {
            out.push_str(&pad);
        }
        out.push_str(line);
    }
    out
}

/// The Rust type a value of `shape` an entry holds is held in: the type a
/// schema unit holds it in (docs/adr/0086, section 7.1; docs/adr/0101, section
/// 5.3), the payload type of a record's event schema, the type of a union's
/// unit, the language's list of its bound and `Option`.
pub(crate) fn composed_type(shape: &CborShape) -> String {
    rust_type(shape, Held::Composed)
}

/// Whether the type [`composed_type`] names for a value of `shape` is `Copy`,
/// so a printer that takes a value out of a place copies it and clones only a
/// value that is not: a number and a `bool`; an enum, a record's payload and a
/// union's type where the derives each is declared with hold `Copy`, asked of
/// the one decision of each; and a value that may be absent where its element
/// is. The runtime's bounded text and bounded list, which a byte string is one
/// of, are `Clone` only.
pub(crate) fn composed_is_copy(shape: &CborShape) -> bool {
    match shape {
        CborShape::Int { .. } | CborShape::Float { .. } | CborShape::Bool => true,
        CborShape::Enum(enumeration) => rust_enum_derives(enumeration.closed).is_copy(),
        CborShape::Record(record) => {
            rust_event_schema_payload_derives(record.fields.iter().map(|field| &field.ty)).is_copy()
        }
        CborShape::Union(_) => RustDeriveCategory::UnionSum.is_copy(),
        CborShape::Nullable(elem) => composed_is_copy(elem),
        CborShape::Text { .. } | CborShape::Bytes { .. } | CborShape::List { .. } => false,
        CborShape::Unresolved { what } => unresolved(what),
    }
}

fn rust_type(shape: &CborShape, held: Held) -> String {
    let runtime = cbor_codec::RUST_RUNTIME_CODEC;
    match shape {
        CborShape::Int { bits, signed } => format!("{}{bits}", if *signed { "i" } else { "u" }),
        CborShape::Float { bits } => format!("f{bits}"),
        CborShape::Bool => "bool".to_string(),
        CborShape::Text { max } => format!("{runtime}::SceString<{max}>"),
        CborShape::Bytes { .. } if held == Held::Field => "Vec<u8>".to_string(),
        CborShape::Bytes { max, .. } => format!("{runtime}::SceBytes<{max}>"),
        CborShape::Enum(enumeration) => {
            enum_qualified_type(&enumeration.name, crate::generator::Language::Rust)
        }
        CborShape::Record(record) => {
            event_schema_payload_type(&record.name, crate::generator::Language::Rust)
        }
        CborShape::Union(union) => union_type(union),
        CborShape::List { elem, max, .. } => format!(
            "{runtime}::BoundedList<{}, {max}>",
            rust_type(elem, Held::Composed)
        ),
        CborShape::Nullable(elem) => format!("Option<{}>", rust_type(elem, Held::Composed)),
        CborShape::Unresolved { what } => unresolved(what),
    }
}

/// A part of a type the plan could not say, which the compile refused before
/// any printer reads the plan.
fn unresolved(what: &str) -> ! {
    unreachable!(
        "`{what}` of a CBOR entry did not resolve, which the compile refuses before any \
         printer reads the plan (`cbor_codec::unheld_value`)"
    )
}

/// The expression that reads a value of `shape` an entry holds from the cursor
/// `cursor` (`&mut c` in the map's own read, `c` in a record's or a union's),
/// as the type [`composed_type`] names: a value of the type, or the refusal of
/// the input, returned from the enclosing read.
pub(crate) fn decode(shape: &CborShape, cursor: &str) -> String {
    decode_expr(shape, cursor, Held::Composed, 0)
}

fn decode_expr(shape: &CborShape, cursor: &str, held: Held, depth: usize) -> String {
    let runtime = cbor_codec::RUST_RUNTIME_CODEC;
    match shape {
        CborShape::Int { bits, signed } => int_decode(*bits, *signed, cursor),
        CborShape::Float { bits: 32 } => format!("cbor::read_float32({cursor})?"),
        CborShape::Float { .. } => format!("cbor::read_float({cursor})?"),
        CborShape::Bool => format!("cbor::read_bool({cursor})?"),
        CborShape::Text { max } => format!(
            "{runtime}::SceString::<{max}>::from_text(cbor::read_text({cursor}, Some({max}))?)?"
        ),
        CborShape::Bytes { max, .. } if held == Held::Field => {
            format!("cbor::read_bytes({cursor}, Some({max}))?.to_vec()")
        }
        CborShape::Bytes { max, .. } => format!(
            "{runtime}::SceBytes::<{max}>::from_slice(cbor::read_bytes({cursor}, Some({max}))?)?"
        ),
        CborShape::Enum(enumeration) => enum_decode(enumeration, cursor),
        CborShape::Record(record) => {
            let [decode, _, _] = composite_fns("record", &record.name);
            format!("{decode}({cursor})?")
        }
        CborShape::Union(union) => {
            let [decode, _, _] = composite_fns("union", &union.name);
            format!("{decode}({cursor})?")
        }
        // A definite-length array, its count judged against the list's bound
        // and lower count before any element is read (docs/adr/0101, section
        // 3.3).
        CborShape::List { elem, max, min } => {
            let (len, list) = (bound_name("len", depth), bound_name("list", depth));
            let refused = if *min > 0 {
                format!("!({min}..={max}).contains(&{len})")
            } else {
                format!("{len} > {max}")
            };
            let element = decode_expr(elem, cursor, Held::Composed, depth + 1);
            format!(
                "{{\n    // A definite-length array within the list's counts.\n    \
                 let {len} = cbor::read_array_len({cursor})?;\n    \
                 if {refused} {{\n        return Err(CodecError::CborOutOfRange);\n    }}\n    \
                 let mut {list}: {ty} = {runtime}::BoundedList::new();\n    \
                 for _ in 0..{len} {{\n        {list}.push({element})?;\n    }}\n    \
                 {list}\n}}",
                ty = rust_type(shape, Held::Composed),
                element = continued(&element, 8),
            )
        }
        // `null` is an absent element; anything else is read as the element,
        // which refuses what is no value of it.
        CborShape::Nullable(elem) => format!(
            "if cbor::take_null({cursor})? {{\n    None\n}} else {{\n    Some({})\n}}",
            continued(&decode_expr(elem, cursor, Held::Composed, depth + 1), 4)
        ),
        CborShape::Unresolved { what } => unresolved(what),
    }
}

/// The read of an integer of `bits` bits, of either sign where `signed`, held
/// to the range of its type: major type 0, and major type 1 below zero.
fn int_decode(bits: u32, signed: bool, cursor: &str) -> String {
    match (bits, signed) {
        (64, false) => format!("cbor::read_uint({cursor})?"),
        (64, true) => format!("cbor::read_int_within({cursor}, i64::MIN, i64::MAX)?"),
        (_, false) => {
            format!("cbor::read_uint_upto({cursor}, u64::from(u{bits}::MAX))? as u{bits}")
        }
        (_, true) => format!(
            "cbor::read_int_within({cursor}, i64::from(i{bits}::MIN), i64::from(i{bits}::MAX))? \
             as i{bits}"
        ),
    }
}

/// The width and the sign of the integer `carrier`, an enum's carrier.
fn carrier_int(carrier: &SceType) -> (u32, bool) {
    (carrier.int_bit_width().unwrap_or(64), carrier.is_signed())
}

/// The read of a value of `enumeration`: the integer of its carrier, the
/// variant it declares, and a number a closed enum does not declare refused.
fn enum_decode(enumeration: &CborEnum, cursor: &str) -> String {
    let (bits, signed) = carrier_int(&enumeration.carrier);
    let (_, from) = enum_number_symbols(
        &enumeration.name,
        !enumeration.closed,
        crate::generator::Language::Rust,
    );
    let read = format!("{from}({})", int_decode(bits, signed, cursor));
    if enumeration.closed {
        format!("{read}.ok_or(CodecError::UndeclaredEnumValue)?")
    } else {
        read
    }
}

/// The write of the integer `value` of `bits` bits, of either sign where
/// `signed`, in its shortest head.
fn int_write(bits: u32, signed: bool, value: &str) -> String {
    match (bits, signed) {
        (64, false) => format!("cbor::write_uint(w, {value})?;"),
        (64, true) => format!("cbor::write_int(w, {value})?;"),
        (_, false) => format!("cbor::write_uint(w, u64::from({value}))?;"),
        (_, true) => format!("cbor::write_int(w, i64::from({value}))?;"),
    }
}

/// The statements that write the value of `shape` `access` reaches, in its one
/// form (docs/adr/0101, section 2.1), into the sink `w`. They refuse nothing:
/// what a value can break is checked before ([`check_stmts`]). A write does not
/// ask where a value stands ([`Held`]): a byte string in the language's own
/// vector and one in a bounded byte string are each written as the slice they
/// lend.
fn write_stmts(shape: &CborShape, access: &Access, depth: usize) -> Vec<String> {
    let line = match shape {
        CborShape::Int { bits, signed } => int_write(*bits, *signed, &access.value()),
        CborShape::Float { bits: 32 } => {
            format!("cbor::write_float(w, f64::from({}))?;", access.value())
        }
        CborShape::Float { .. } => format!("cbor::write_float(w, {})?;", access.value()),
        CborShape::Bool => format!("cbor::write_bool(w, {})?;", access.value()),
        CborShape::Text { .. } => format!("cbor::write_text(w, {})?;", access.call("as_str")),
        CborShape::Bytes { .. } => format!("cbor::write_bytes(w, {})?;", access.call("as_slice")),
        CborShape::Enum(enumeration) => {
            let (bits, signed) = carrier_int(&enumeration.carrier);
            int_write(bits, signed, &access.call("to_underlying"))
        }
        CborShape::Record(record) => {
            let [_, encode, _] = composite_fns("record", &record.name);
            format!("{encode}(w, {})?;", access.reference())
        }
        CborShape::Union(union) => {
            let [_, encode, _] = composite_fns("union", &union.name);
            format!("{encode}(w, {})?;", access.reference())
        }
        CborShape::List { elem, .. } => {
            let item = bound_name("item", depth);
            let slice = access.call("as_slice");
            let inner = write_stmts(elem, &Access::Ref(item.clone()), depth + 1);
            format!(
                "cbor::write_array_head(w, ::sce_forge_runtime::codec::widen_usize({slice}.len()))?;\n\
                 for {item} in {slice} {{\n{}}}",
                super::indent(&inner, 4)
            )
        }
        CborShape::Nullable(elem) => {
            let present = bound_name("present", depth);
            let inner = write_stmts(elem, &Access::Ref(present.clone()), depth + 1);
            format!(
                "match {} {{\n    Some({present}) => {{\n{}    }}\n    None => cbor::write_null(w)?,\n}}",
                access.reference(),
                super::indent(&inner, 8)
            )
        }
        CborShape::Unresolved { what } => unresolved(what),
    };
    vec![line]
}

/// Whether a value of `shape` that its Rust type holds can still have no
/// encoding (docs/adr/0101, section 3.5): a list below its lower count at any
/// depth, and a byte string a record's field holds in the language's own
/// vector past its bound. The one reading the plan's totality makes of the
/// shape ([`crate::forge::codec_totality::cbor_value_refuses`]), so the two
/// name the same entries.
fn can_refuse(shape: &CborShape, held: Held) -> bool {
    crate::forge::codec_totality::cbor_value_refuses(shape, held == Held::Field)
}

fn record_can_refuse(record: &CborRecord) -> bool {
    record
        .fields
        .iter()
        .any(|field| can_refuse(&field.shape, Held::Field))
}

fn union_can_refuse(union: &CborUnion) -> bool {
    union
        .cases
        .iter()
        .any(|case| case.record.as_ref().is_some_and(record_can_refuse))
}

/// The statements that refuse, with `refusal`, a value of `shape` `access`
/// reaches that has no encoding ([`can_refuse`]); empty for one that cannot.
fn check_stmts(
    shape: &CborShape,
    access: &Access,
    held: Held,
    depth: usize,
    refusal: &str,
) -> Vec<String> {
    if !can_refuse(shape, held) {
        return Vec::new();
    }
    let refuse = format!("    return Err({refusal});\n");
    match shape {
        CborShape::List { elem, min, .. } => {
            let mut lines = Vec::new();
            if *min > 0 {
                // The length is bound first and compared bound, so no
                // comparison of a `len()` call against a literal is written.
                let len = bound_name("len", depth);
                lines.push(format!(
                    "let {len} = {}.len();\nif {len} < {min} {{\n{refuse}}}",
                    access.call("as_slice")
                ));
            }
            if can_refuse(elem, Held::Composed) {
                let item = bound_name("item", depth);
                let inner = check_stmts(
                    elem,
                    &Access::Ref(item.clone()),
                    Held::Composed,
                    depth + 1,
                    refusal,
                );
                lines.push(format!(
                    "for {item} in {} {{\n{}}}",
                    access.call("as_slice"),
                    super::indent(&inner, 4)
                ));
            }
            lines
        }
        CborShape::Nullable(elem) => {
            let present = bound_name("present", depth);
            let inner = check_stmts(
                elem,
                &Access::Ref(present.clone()),
                held,
                depth + 1,
                refusal,
            );
            vec![format!(
                "if let Some({present}) = {} {{\n{}}}",
                access.reference(),
                super::indent(&inner, 4)
            )]
        }
        CborShape::Bytes { max: 0, .. } => vec![format!(
            "if !{}.is_empty() {{\n{refuse}}}",
            access.call("as_slice")
        )],
        CborShape::Bytes { max, .. } => {
            vec![format!("if {} > {max} {{\n{refuse}}}", access.call("len"))]
        }
        CborShape::Record(record) => {
            let [_, _, check] = composite_fns("record", &record.name);
            vec![format!("{check}({})?;", access.reference())]
        }
        CborShape::Union(union) => {
            let [_, _, check] = composite_fns("union", &union.name);
            vec![format!("{check}({})?;", access.reference())]
        }
        CborShape::Int { .. }
        | CborShape::Float { .. }
        | CborShape::Bool
        | CborShape::Text { .. }
        | CborShape::Enum(_)
        | CborShape::Unresolved { .. } => Vec::new(),
    }
}

/// Append the module of the document `name` to `out`, once.
fn add_module(name: &str, out: &mut Vec<String>) {
    let module = module_of(name);
    if !out.contains(&module) {
        out.push(module);
    }
}

/// The module of each enum, record and union document a value of `shape`
/// names, appended to `out` once.
fn modules_of(shape: &CborShape, out: &mut Vec<String>) {
    match shape {
        CborShape::Enum(enumeration) => add_module(&enumeration.name, out),
        CborShape::Record(record) => record_modules(record, out),
        CborShape::Union(union) => {
            add_module(&union.name, out);
            for record in union.cases.iter().filter_map(|case| case.record.as_ref()) {
                record_modules(record, out);
            }
        }
        CborShape::List { elem, .. } | CborShape::Nullable(elem) => modules_of(elem, out),
        CborShape::Int { .. }
        | CborShape::Float { .. }
        | CborShape::Bool
        | CborShape::Text { .. }
        | CborShape::Bytes { .. }
        | CborShape::Unresolved { .. } => {}
    }
}

fn record_modules(record: &CborRecord, out: &mut Vec<String>) {
    add_module(&record.name, out);
    for field in &record.fields {
        modules_of(&field.shape, out);
    }
}

/// The module of each enum, record and union document the entries of `plan`
/// name, at any depth, in the order a walk of the entries meets them: the
/// siblings the codec's module reads them through (`use super::<module>;`),
/// those of a document the codec does not import itself among them.
pub(crate) fn reached_modules(plan: &CborPlan<'_>) -> Vec<String> {
    let mut modules = Vec::new();
    for item in &plan.entries {
        modules_of(&item.value, &mut modules);
    }
    modules
}

/// The read, the write and, where a value of it can be refused, the check of
/// each record and union document the entries of `plan` reach, as the text of
/// the module's functions (docs/adr/0101, section 5.1).
pub(crate) fn composites(plan: &CborPlan<'_>) -> String {
    let mut out = String::new();
    for composite in &cbor_codec::reached_composites(plan) {
        out.push('\n');
        out.push_str(&match composite {
            Composite::Record(record) => record_fns(record),
            Composite::Union(union) => union_fns(union),
        });
    }
    out
}

/// A field of a record as the record's functions name it: its member, its key
/// as a literal, and its type as the record's unit holds it.
struct FieldNames {
    member: String,
    key: String,
    ty: String,
}

fn field_names(field: &crate::forge::encode_plan::CborField) -> FieldNames {
    FieldNames {
        member: event_schema_field_ident(&field.id, crate::generator::Language::Rust),
        key: format!("{:?}", field.id),
        ty: rust_type(&field.shape, Held::Field),
    }
}

/// The read, the write and the check of the record `record`.
fn record_fns(record: &CborRecord) -> String {
    let [decode, encode, check] = composite_fns("record", &record.name);
    let payload = event_schema_payload_type(&record.name, crate::generator::Language::Rust);
    let schema = &record.name;

    // The read: each field held until the map has been read to its end, so a
    // key given twice and a field that is not optional missing are told apart
    // from the value.
    let mut read = format!(
        "/// Read one record of the event schema `{schema}` from `c`, into the payload\n\
         /// type the schema's own unit declares: one CBOR map of its present fields,\n\
         /// each under its id as text (docs/adr/0101, section 2.1). Its keys in any\n\
         /// order, a key the schema does not declare skipped whole, a key given twice\n\
         /// and a field that is not optional missing refused, and a value of no field's\n\
         /// type refused.\n\
         fn {decode}(c: &mut SceCursor<'_>) -> Result<{payload}, CodecError> {{\n    \
         let count = cbor::read_map_len(c)?;\n"
    );
    for field in &record.fields {
        let names = field_names(field);
        read.push_str(&format!(
            "    let mut held_{}: Option<{}> = None;\n",
            names.member, names.ty
        ));
    }
    read.push_str("    for _ in 0..count {\n        match cbor::read_text(c, None)? {\n");
    for field in &record.fields {
        let names = field_names(field);
        let value = decode_expr(&field.shape, "c", Held::Field, 0);
        read.push_str(&format!(
            "            {key} => {{\n                \
             // A key given twice is not one map entry.\n                \
             if held_{member}.is_some() {{\n                    \
             return Err(CodecError::CborDuplicateKey);\n                }}\n                \
             held_{member} = Some({value});\n            }}\n",
            key = names.key,
            member = names.member,
            value = continued(&value, 16),
        ));
    }
    read.push_str(&format!(
        "            _ => cbor::skip(c)?,\n        }}\n    }}\n    Ok({payload} {{\n"
    ));
    for field in &record.fields {
        let member = field_names(field).member;
        if field.optional {
            read.push_str(&format!("        {member}: held_{member},\n"));
        } else {
            read.push_str(&format!(
                "        {member}: held_{member}.ok_or(CodecError::CborRequiredKeyMissing)?,\n"
            ));
        }
    }
    read.push_str("    })\n}\n");

    // The write: the map head counts the fields present, then each follows in
    // the order of its key's encoding, which the plan decided.
    let required = record.fields.iter().filter(|f| !f.optional).count();
    let mut body: Vec<String> = Vec::new();
    if required == record.fields.len() {
        body.push(format!("cbor::write_map_head(w, {required})?;"));
    } else {
        body.push(format!("let mut count: u64 = {required};"));
        for field in record.fields.iter().filter(|f| f.optional) {
            body.push(format!(
                "if record.{}.is_some() {{\n    count += 1;\n}}",
                field_names(field).member
            ));
        }
        body.push("cbor::write_map_head(w, count)?;".to_string());
    }
    for field in record.fields_by_key() {
        let names = field_names(field);
        if field.optional {
            let mut inner = vec![format!("cbor::write_text(w, {})?;", names.key)];
            inner.extend(write_stmts(
                &field.shape,
                &Access::Ref("value".to_string()),
                0,
            ));
            body.push(format!(
                "if let Some(value) = &record.{} {{\n{}}}",
                names.member,
                super::indent(&inner, 4)
            ));
        } else {
            body.push(format!("cbor::write_text(w, {})?;", names.key));
            body.extend(write_stmts(
                &field.shape,
                &Access::Place(format!("record.{}", names.member)),
                0,
            ));
        }
    }
    body.push("Ok(())".to_string());
    let write = format!(
        "\n/// Write one record of `{schema}` as one map of its present fields, the keys\n\
         /// in the order RFC 8949 §4.2.1 sorts their encodings: a shorter key first.\n\
         fn {encode}<S: SceSink + ?Sized>(w: &mut S, record: &{payload}) -> Result<(), S::Error> {{\n\
         {}}}\n",
        super::indent(&body, 4)
    );

    // The check: what the record's unit holds and its encoding cannot carry.
    let check = if record_can_refuse(record) {
        let refusal = "CodecError::CborOutOfRange";
        let mut body: Vec<String> = Vec::new();
        for field in record
            .fields
            .iter()
            .filter(|f| can_refuse(&f.shape, Held::Field))
        {
            let names = field_names(field);
            if field.optional {
                let inner = check_stmts(
                    &field.shape,
                    &Access::Ref("value".to_string()),
                    Held::Field,
                    0,
                    refusal,
                );
                body.push(format!(
                    "if let Some(value) = &record.{} {{\n{}}}",
                    names.member,
                    super::indent(&inner, 4)
                ));
            } else {
                body.extend(check_stmts(
                    &field.shape,
                    &Access::Place(format!("record.{}", names.member)),
                    Held::Field,
                    0,
                    refusal,
                ));
            }
        }
        body.push("Ok(())".to_string());
        format!(
            "\n/// Refuse a record of `{schema}` that its unit holds and no encoding\n\
             /// carries: a list below its lower count, a byte string past its bound.\n\
             fn {check}(record: &{payload}) -> Result<(), CodecError> {{\n{}}}\n",
            super::indent(&body, 4)
        )
    } else {
        String::new()
    };
    format!("{read}{write}{check}")
}

/// The read, the write and the check of the union `union`: a map of one entry
/// keyed by its case's variant name, whose value is the case's record or the
/// empty map (docs/adr/0101, section 2.1).
fn union_fns(union: &CborUnion) -> String {
    let [decode, encode, check] = composite_fns("union", &union.name);
    let ty = union_type(union);
    let name = &union.name;
    use crate::forge::composed_lowering::rust_case_ident as case_ident;
    let names: Vec<String> = union
        .cases
        .iter()
        .map(|case| format!("{:?}", case.variant))
        .collect();

    let mut read = format!(
        "/// Read one value of the union `{name}` from `c`: a map of exactly one entry,\n\
         /// judged for its count before its key is read, keyed by a case's variant\n\
         /// name, whose value is the case's record or a map of no field it declares.\n\
         fn {decode}(c: &mut SceCursor<'_>) -> Result<{ty}, CodecError> {{\n    \
         match cbor::read_variant(c, &[{}])? {{\n",
        names.join(", ")
    );
    for (at, case) in union.cases.iter().enumerate() {
        let ident = case_ident(&case.variant);
        match &case.record {
            Some(record) => {
                let [decode_record, _, _] = composite_fns("record", &record.name);
                read.push_str(&format!(
                    "        {at} => Ok({ty}::{ident}({decode_record}(c)?)),\n"
                ));
            }
            None => read.push_str(&format!(
                "        {at} => {{\n            cbor::read_empty_case(c)?;\n            \
                 Ok({ty}::{ident})\n        }}\n"
            )),
        }
    }
    // `read_variant` answers the place of a case it names, so no other number
    // reaches here; a decode refuses rather than panics all the same.
    read.push_str("        _ => Err(CodecError::CborUnknownVariant),\n    }\n}\n");

    let mut write = format!(
        "\n/// Write one value of the union `{name}`: the map of one entry its case's\n\
         /// variant name keys, then the case's record or the empty map.\n\
         fn {encode}<S: SceSink + ?Sized>(w: &mut S, value: &{ty}) -> Result<(), S::Error> {{\n    \
         match value {{\n"
    );
    for case in &union.cases {
        let ident = case_ident(&case.variant);
        let variant = format!("{:?}", case.variant);
        match &case.record {
            Some(record) => {
                let [_, encode_record, _] = composite_fns("record", &record.name);
                write.push_str(&format!(
                    "        {ty}::{ident}(record) => {{\n            \
                     cbor::write_variant(w, {variant})?;\n            \
                     {encode_record}(w, record)\n        }}\n"
                ));
            }
            None => write.push_str(&format!(
                "        {ty}::{ident} => {{\n            \
                 cbor::write_variant(w, {variant})?;\n            \
                 cbor::write_map_head(w, 0)\n        }}\n"
            )),
        }
    }
    write.push_str("    }\n}\n");

    let check = if union_can_refuse(union) {
        let mut arms = String::new();
        for case in &union.cases {
            let ident = case_ident(&case.variant);
            match &case.record {
                Some(record) if record_can_refuse(record) => {
                    let [_, _, check_record] = composite_fns("record", &record.name);
                    arms.push_str(&format!(
                        "        {ty}::{ident}(record) => {check_record}(record),\n"
                    ));
                }
                Some(_) => arms.push_str(&format!("        {ty}::{ident}(_) => Ok(()),\n")),
                None => arms.push_str(&format!("        {ty}::{ident} => Ok(()),\n")),
            }
        }
        format!(
            "\n/// Refuse a value of `{name}` whose case's record has no encoding.\n\
             fn {check}(value: &{ty}) -> Result<(), CodecError> {{\n    \
             match value {{\n{arms}    }}\n}}\n"
        )
    } else {
        String::new()
    };
    format!("{read}{write}{check}")
}

/// The byte case of `source`, a value of the CBOR codec `plan` at `path`
/// (docs/adr/0081 §5): the map itself, which is its own source, made of its
/// fields, a text and a byte string through `Bounded::new`, the one door of the
/// bound its type holds, an entry the map may lack as an `Option`; and for each
/// entry an expression that holds when the decode of the case's bytes holds what
/// the value does, the two compared field by field as the map's derived
/// equality compares them. A map takes no flag input and no caller's tag.
pub(crate) fn source_case(
    plan: &CborPlan<'_>,
    path: &super::CodecPath,
    source: &serde_json::Value,
) -> Result<super::SourceCase, String> {
    use super::super::source_notation::{cbor_elements, Scalar, Stated};
    let l = LangCtx::new(crate::generator::Language::Rust, &[]);
    let runtime = cbor_codec::RUST_RUNTIME_CODEC;
    let bounded = |inner: String| {
        format!("{runtime}::Bounded::new({inner}).expect(\"the case is within its bound\")")
    };
    let held = |scalar: &Scalar| match scalar {
        Scalar::Bool(b) => b.to_string(),
        Scalar::Int(n) => n.to_string(),
        Scalar::Text(text) => bounded(format!("{text:?}")),
        Scalar::Bytes(bytes) => {
            let bytes: Vec<String> = bytes.iter().map(u8::to_string).collect();
            bounded(format!("&[{}][..]", bytes.join(", ")))
        }
    };
    let mut members = Vec::new();
    let mut checks = Vec::new();
    for element in cbor_elements(plan, source)? {
        let field = l.codec_field_id(&element.id);
        let value = match &element.stated {
            Stated::Absent => "None".to_string(),
            Stated::One(scalar) if element.optional => format!("Some({})", held(scalar)),
            Stated::One(scalar) => held(scalar),
            Stated::List(_) => unreachable!("an entry of a map is one value"),
        };
        members.push(format!("{field}: {value}"));
        checks.push((element.id, format!("decoded.{field} == parts.{field}")));
    }
    Ok(super::SourceCase {
        bindings: Vec::new(),
        literal: format!(
            "{}::{} {{ {} }}",
            path.module,
            path.type_name,
            members.join(", ")
        ),
        checks,
        inputs: Vec::new(),
        encode_inputs: Vec::new(),
    })
}

/// The source interface of a CBOR codec, as the text its file holds.
pub(in super::super) fn render(input: &Input<'_>) -> String {
    let m = input.m;
    let name = input.struct_name;
    let lt = input.struct_lifetime;
    let trait_name = format!("{name}Source");
    let accessor = |e: &CborEntry| input.l.codec_field_id(&e.id);

    let plan = CborPlan::of(m, input.imports);
    let mut trait_items = String::new();
    let mut ref_items = String::new();
    let mut builder_items = String::new();
    for item in &plan.entries {
        let e = item.entry;
        let (acc, ty) = (accessor(e), accessor_type(input, item));
        trait_items.push_str(&format!(
            "    /// The entry with key {key}{required}.\n    fn {acc}(&self) -> {ty};\n",
            key = e.key,
            required = if item.required { ", required" } else { "" },
        ));
        ref_items.push_str(&format!(
            "    fn {acc}(&self) -> {ty} {{\n        (**self).{acc}()\n    }}\n"
        ));
        builder_items.push_str(&format!(
            "    fn {acc}(&self) -> {ty} {{\n        {}\n    }}\n",
            held_read(item, &acc)
        ));
    }

    // The entries as written: the map head counts the present ones, then each
    // follows in ascending key order.
    let may_fail = !plan.totality.is_total();
    let required = plan.required();
    let mut body: Vec<String> = Vec::new();
    let optional: Vec<&CborItem<'_>> = plan.entries.iter().filter(|item| !item.required).collect();
    if optional.is_empty() {
        body.push(format!("let count: u64 = {required};"));
    } else {
        body.push(format!("let mut count: u64 = {required};"));
        for item in &optional {
            body.push(format!(
                "if src.{}().is_some() {{\n    count += 1;\n}}",
                accessor(item.entry)
            ));
        }
    }
    body.push("cbor::write_map_head(w, count)?;".to_string());
    for &index in &plan.by_key {
        let item = &plan.entries[index];
        let e = item.entry;
        let acc = accessor(e);
        let write = write_value(input, &plan, item);
        if item.required {
            body.push(format!("let v = src.{acc}();"));
            body.push(format!("cbor::write_uint(w, {})?;", e.key));
            body.push(write);
        } else {
            body.push(format!("if let Some(v) = src.{acc}() {{"));
            body.push(format!("    cbor::write_uint(w, {})?;", e.key));
            body.extend(write.lines().map(|line| format!("    {line}")));
            body.push("}".to_string());
        }
    }
    body.push("Ok(())".to_string());
    let body = super::indent(&body, 8);
    plan.totality.assert_all_written(&m.name);

    // A map whose entries can break a bound is written into a sink that only
    // counts first, so a refusal leaves the caller's sink untouched; one whose
    // entries cannot has nothing to find first.
    let (doc, guard) = if may_fail {
        (
            "\x20   /// declared bound is refused before the first byte is written.\n",
            "\x20       Self::count_source(src)?;\n",
        )
    } else {
        (
            "\x20   /// declared bound is not one a value of it can break: the only error is\n\
             \x20   /// the sink's (docs/adr/0065).\n",
            "",
        )
    };
    let result = super::write_result(may_fail, "K");

    // The owned mirror (`{name}Owned<S>`, generated when every string and byte
    // string of the map states a bound) says what it holds through the same
    // interface, so `encode_source` writes it as it writes the view, and the
    // mirror's `as_borrowed` reads each entry from here.
    let owned_impl = if cbor_codec::rust_owned_mirror(m) {
        let mut items = String::new();
        for item in &plan.entries {
            let e = item.entry;
            let (acc, ty) = (accessor(e), accessor_type(input, item));
            let read = owned_read(item, &acc);
            items.push_str(&format!(
                "    fn {acc}(&self) -> {ty} {{\n        {read}\n    }}\n"
            ));
        }
        format!(
            "impl<{param}: {name}Storage> {trait_name} for {name}Owned<{param}> {{\n{items}}}\n\n",
            param = cbor_codec::RUST_STORAGE_PARAM,
        )
    } else {
        String::new()
    };

    format!(
        "// ── Source (ADR 0058) ────────────────────────────────────────────────\n\
         /// What a value of `{name}` says: its entries. The count the map head holds\n\
         /// is not among them; `{name}` counts the entries present\n\
         /// ([`{name}::encode_source`]).\n\
         pub trait {trait_name} {{\n{trait_items}}}\n\n\
         impl<T: {trait_name} + ?Sized> {trait_name} for &T {{\n{ref_items}}}\n\n\
         impl{lt} {trait_name} for {name}{lt} {{\n{builder_items}}}\n\n\
         {owned_impl}\
         #[allow(dead_code)]\n\
         impl{lt} {name}{lt} {{\n\
         \x20   /// Encode what `src` says as one CBOR map into `w`: the entries present in\n\
         \x20   /// ascending key order, every head in its shortest form. An entry past its\n\
         {doc}\
         \x20   pub fn encode_source<S: {trait_name} + ?Sized, K: SceSink>(src: &S, w: &mut K) -> {result} {{\n\
         {guard}\
         \x20       Self::write_source(src, w)\n\
         \x20   }}\n\n\
         \x20   /// [`Self::encode_source`] without the pre-pass a parent has already made.\n\
         \x20   #[doc(hidden)]\n\
         {inline}\
         \x20   pub fn write_source<S: {trait_name} + ?Sized, K: SceSink>(src: &S, w: &mut K) -> {result} {{\n\
         {body}\
         \x20   }}\n\n\
         {count}\
         }}\n\n\
         {measure}",
        count = super::count_function(&trait_name, "", "", may_fail),
        inline = super::write_attribute(input.inline_encode),
        measure = super::measure_impl(name, lt, &trait_name, input.inline_encode, may_fail),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::forge::encode_plan::{text_key_order, CborCase, CborField};
    use crate::forge::value_type::{ValueType, WrittenType};

    fn ty(text: &str) -> ValueType {
        WrittenType::read(text)
            .and_then(WrittenType::bounded)
            .unwrap_or_else(|why| panic!("`{text}` is a type: {why:?}"))
    }

    fn int(bits: u32, signed: bool) -> CborShape {
        CborShape::Int { bits, signed }
    }

    fn list(elem: CborShape, max: u32, min: u32) -> CborShape {
        CborShape::List {
            elem: Box::new(elem),
            max,
            min,
        }
    }

    /// A field `id` of the type `text`, its shape `shape`.
    fn field(id: &str, text: &str, shape: CborShape) -> CborField {
        let ty = ty(text);
        CborField {
            id: id.to_string(),
            optional: ty.optional_elem().is_some(),
            ty,
            shape,
        }
    }

    /// The record of the event schema `name` of `fields`.
    fn record(name: &str, fields: Vec<CborField>) -> CborRecord {
        CborRecord {
            alias: filters::to_pascal_case(name.to_string()),
            name: name.to_string(),
            by_key: text_key_order(fields.iter().map(|f| f.id.as_str())),
            fields,
        }
    }

    fn color() -> CborShape {
        CborShape::Enum(CborEnum {
            alias: "Color".to_string(),
            name: "paint_color".to_string(),
            carrier: SceType::Uint8,
            closed: true,
        })
    }

    /// `order_payment` of docs/adr/0101 section 2.7: `card`, `cash`, which holds
    /// no record, and `voucher`, whose byte string its unit holds unbounded.
    fn payment() -> CborUnion {
        let card = record(
            "card_payment",
            vec![
                field("last4", "string[4]", CborShape::Text { max: 4 }),
                field("amount", "uint32", int(32, false)),
            ],
        );
        let voucher = record(
            "voucher_payment",
            vec![field(
                "voucher_code",
                "bytes[4]",
                CborShape::Bytes {
                    max: 4,
                    exact: false,
                },
            )],
        );
        let case = |variant: &str, number: i128, record: Option<CborRecord>| CborCase {
            variant: variant.to_string(),
            number,
            record,
        };
        CborUnion {
            alias: "Payment".to_string(),
            name: "order_payment".to_string(),
            cases: vec![
                case("card", 0, Some(card)),
                case("cash", 1, None),
                case("voucher", 2, Some(voucher)),
            ],
        }
    }

    #[test]
    fn a_byte_string_is_held_bounded_in_a_composition_and_as_its_unit_holds_a_field() {
        let bytes = CborShape::Bytes {
            max: 4,
            exact: false,
        };
        assert_eq!(rust_type(&bytes, Held::Field), "Vec<u8>");
        // Either is written as the slice it lends.
        assert_eq!(
            write_stmts(&bytes, &Access::Place("record.blob".to_string()), 0),
            ["cbor::write_bytes(w, record.blob.as_slice())?;"]
        );
        assert_eq!(
            composed_type(&list(bytes, 2, 0)),
            "::sce_forge_runtime::codec::BoundedList<::sce_forge_runtime::codec::SceBytes<4>, 2>"
        );
        assert_eq!(
            composed_type(&list(CborShape::Nullable(Box::new(int(8, true))), 3, 0)),
            "::sce_forge_runtime::codec::BoundedList<Option<i8>, 3>"
        );
        assert_eq!(composed_type(&color()), "paint_color::PaintColor");
        assert_eq!(
            composed_type(&CborShape::Union(payment())),
            "order_payment::OrderPayment"
        );
        assert_eq!(
            composed_type(&CborShape::Record(record("order_customer", Vec::new()))),
            "order_customer::OrderCustomerPayload"
        );
    }

    #[test]
    fn a_number_is_read_within_its_type_and_written_in_either_sign() {
        assert_eq!(
            decode(&int(16, true), "c"),
            "cbor::read_int_within(c, i64::from(i16::MIN), i64::from(i16::MAX))? as i16"
        );
        assert_eq!(decode(&int(64, false), "c"), "cbor::read_uint(c)?");
        assert_eq!(
            decode(&CborShape::Float { bits: 32 }, "&mut c"),
            "cbor::read_float32(&mut c)?"
        );
        let place = Access::Place("record.n".to_string());
        let reference = Access::Ref("item".to_string());
        assert_eq!(
            write_stmts(&int(16, true), &place, 0),
            ["cbor::write_int(w, i64::from(record.n))?;"]
        );
        assert_eq!(
            write_stmts(&int(64, false), &reference, 0),
            ["cbor::write_uint(w, *item)?;"]
        );
        assert_eq!(
            write_stmts(&CborShape::Float { bits: 32 }, &reference, 0),
            ["cbor::write_float(w, f64::from(*item))?;"]
        );
        // An enum is the integer of its declared number, the variant it names
        // read back and a number a closed enum does not declare refused.
        assert_eq!(
            decode(&color(), "c"),
            "paint_color::PaintColor::from_underlying(cbor::read_uint_upto(c, \
             u64::from(u8::MAX))? as u8).ok_or(CodecError::UndeclaredEnumValue)?"
        );
        assert_eq!(
            write_stmts(&color(), &place, 0),
            ["cbor::write_uint(w, u64::from(record.n.to_underlying()))?;"]
        );
    }

    #[test]
    fn a_list_is_read_within_its_counts_and_an_absent_element_is_null() {
        let shape = list(CborShape::Nullable(Box::new(int(16, false))), 3, 1);
        let read = decode(&shape, "c");
        for expected in [
            "let len = cbor::read_array_len(c)?;",
            "if !(1..=3).contains(&len) {",
            "let mut list: ::sce_forge_runtime::codec::BoundedList<Option<u16>, 3> =",
            "list.push(if cbor::take_null(c)? {",
            "Some(cbor::read_uint_upto(c, u64::from(u16::MAX))? as u16)",
        ] {
            assert!(read.contains(expected), "missing `{expected}`:\n{read}");
        }
        let write = write_stmts(&shape, &Access::Ref("v".to_string()), 0).join("\n");
        for expected in [
            "cbor::write_array_head(w, ::sce_forge_runtime::codec::widen_usize(v.as_slice().len()))?;",
            "for item in v.as_slice() {",
            "    match item {",
            "        Some(present1) => {",
            "            cbor::write_uint(w, u64::from(*present1))?;",
            "        None => cbor::write_null(w)?,",
        ] {
            assert!(write.contains(expected), "missing `{expected}`:\n{write}");
        }
        // A list in a list reads and writes its own, named apart.
        let nested = list(list(int(8, false), 2, 0), 2, 0);
        assert!(decode(&nested, "c").contains("let len1 = cbor::read_array_len(c)?;"));
        assert!(write_stmts(&nested, &Access::Ref("v".to_string()), 0)[0]
            .contains("for item1 in item.as_slice() {"));
    }

    #[test]
    fn a_lower_count_and_a_fields_byte_string_are_checked_and_nothing_else() {
        let bytes = CborShape::Bytes {
            max: 4,
            exact: false,
        };
        // A byte string inside a list is held to its bound by its type.
        assert!(!can_refuse(&list(bytes.clone(), 2, 0), Held::Composed));
        assert!(can_refuse(&bytes, Held::Field));
        assert!(can_refuse(&list(int(8, false), 2, 1), Held::Composed));
        assert!(!can_refuse(&CborShape::Text { max: 8 }, Held::Field));
        assert!(can_refuse(&CborShape::Union(payment()), Held::Composed));
        let v = Access::Ref("v".to_string());
        assert_eq!(
            check_stmts(&list(int(8, false), 2, 1), &v, Held::Composed, 0, "E"),
            ["let len = v.as_slice().len();\nif len < 1 {\n    return Err(E);\n}"]
        );
        assert!(check_stmts(&list(bytes.clone(), 2, 0), &v, Held::Composed, 0, "E").is_empty());
        assert_eq!(
            check_stmts(
                &bytes,
                &Access::Place("record.blob".to_string()),
                Held::Field,
                0,
                "E"
            ),
            ["if record.blob.len() > 4 {\n    return Err(E);\n}"]
        );
    }

    #[test]
    fn a_record_is_read_by_its_document_and_written_in_the_order_of_its_keys() {
        let line = record(
            "order_line",
            vec![
                field("sku", "string[16]", CborShape::Text { max: 16 }),
                field("quantity", "uint16", int(16, false)),
                field("note", "optional<string[8]>", CborShape::Text { max: 8 }),
                field("color", "enum:Color", color()),
            ],
        );
        let code = record_fns(&line);
        for expected in [
            "fn decode_record_order_line(c: &mut SceCursor<'_>) -> Result<order_line::OrderLinePayload, CodecError> {",
            "let mut held_note: Option<::sce_forge_runtime::codec::SceString<8>> = None;",
            "        note: held_note,\n",
            "        quantity: held_quantity.ok_or(CodecError::CborRequiredKeyMissing)?,\n",
            "fn encode_record_order_line<S: SceSink + ?Sized>(w: &mut S, record: &order_line::OrderLinePayload) -> Result<(), S::Error> {",
            // An absent field is left out of the map and its count.
            "    let mut count: u64 = 3;\n    if record.note.is_some() {\n        count += 1;\n    }\n",
            "    if let Some(value) = &record.note {\n        cbor::write_text(w, \"note\")?;\n        \
             cbor::write_text(w, value.as_str())?;\n    }\n",
        ] {
            assert!(code.contains(expected), "missing `{expected}`:\n{code}");
        }
        // The plan's order of the keys' encodings: `sku`, `note`, `color`,
        // `quantity`, whatever the order the schema declares them in.
        let write = code
            .split("fn encode_record_order_line")
            .nth(1)
            .expect("the record has a write");
        let at = |key: &str| {
            write
                .find(&format!("cbor::write_text(w, \"{key}\")?;"))
                .unwrap_or_else(|| panic!("the record writes `{key}`:\n{write}"))
        };
        assert!(at("sku") < at("note") && at("note") < at("color") && at("color") < at("quantity"));
        // Nothing of it is refused, so it has no check.
        assert!(!code.contains("fn check_record_order_line"), "{code}");
    }

    #[test]
    fn a_union_is_one_entry_named_by_its_case_and_an_empty_case_is_the_empty_map() {
        let code = union_fns(&payment());
        for expected in [
            "fn decode_union_order_payment(c: &mut SceCursor<'_>) -> Result<order_payment::OrderPayment, CodecError> {",
            "match cbor::read_variant(c, &[\"card\", \"cash\", \"voucher\"])? {",
            "0 => Ok(order_payment::OrderPayment::Card(decode_record_card_payment(c)?)),",
            "1 => {\n            cbor::read_empty_case(c)?;\n            Ok(order_payment::OrderPayment::Cash)\n        }",
            "_ => Err(CodecError::CborUnknownVariant),",
            "order_payment::OrderPayment::Cash => {\n            cbor::write_variant(w, \"cash\")?;\n            cbor::write_map_head(w, 0)\n        }",
            "order_payment::OrderPayment::Card(record) => {\n            cbor::write_variant(w, \"card\")?;\n            encode_record_card_payment(w, record)\n        }",
            // Only the `voucher` case's record can be refused.
            "order_payment::OrderPayment::Card(_) => Ok(()),",
            "order_payment::OrderPayment::Cash => Ok(()),",
            "order_payment::OrderPayment::Voucher(record) => check_record_voucher_payment(record),",
        ] {
            assert!(code.contains(expected), "missing `{expected}`:\n{code}");
        }
    }

    #[test]
    fn a_value_is_copy_where_the_type_it_is_held_in_derives_copy() {
        // A record of numbers, of either presence, is plain data: its payload
        // derives `Copy`, and so does an absent-or-present one of it.
        let point = record(
            "cbor_point",
            vec![
                field("x", "int16", int(16, true)),
                field("y", "optional<int16>", int(16, true)),
            ],
        );
        assert!(composed_is_copy(&CborShape::Record(point.clone())));
        assert!(composed_is_copy(&CborShape::Nullable(Box::new(
            CborShape::Record(point.clone())
        ))));
        assert!(composed_is_copy(&color()));
        assert!(composed_is_copy(&CborShape::Float { bits: 64 }));
        assert!(composed_is_copy(&CborShape::Bool));
        // A record that holds a text, or another record, is a composition.
        let line = record(
            "order_line",
            vec![
                field("sku", "string[16]", CborShape::Text { max: 16 }),
                field("color", "enum:Color", color()),
            ],
        );
        assert!(!composed_is_copy(&CborShape::Record(line)));
        let at = record(
            "cbor_at",
            vec![field(
                "at",
                "record:Point",
                CborShape::Record(point.clone()),
            )],
        );
        assert!(!composed_is_copy(&CborShape::Record(at)));
        // A list, even of numbers, a union, a text and a byte string are not.
        assert!(!composed_is_copy(&list(int(8, false), 4, 0)));
        assert!(!composed_is_copy(&list(CborShape::Record(point), 2, 0)));
        assert!(!composed_is_copy(&CborShape::Union(payment())));
        assert!(!composed_is_copy(&CborShape::Text { max: 8 }));
        assert!(!composed_is_copy(&CborShape::Bytes {
            max: 4,
            exact: false
        }));
    }

    #[test]
    fn a_document_met_at_two_places_is_read_by_one_function() {
        let card = payment().cases[0]
            .record
            .clone()
            .expect("the card case holds a record");
        let shapes = [
            list(CborShape::Record(card), 2, 0),
            CborShape::Nullable(Box::new(CborShape::Union(payment()))),
        ];
        let mut reached = Vec::new();
        for shape in &shapes {
            cbor_codec::reach_composites(shape, &mut reached);
        }
        let names: Vec<&str> = reached.iter().map(Composite::name).collect();
        assert_eq!(names, ["card_payment", "order_payment", "voucher_payment"]);
        // Every document a value names, an enum a record holds among them.
        let line = record("order_line", vec![field("color", "enum:Color", color())]);
        let mut modules = Vec::new();
        for shape in [CborShape::Record(line), CborShape::Union(payment())] {
            modules_of(&shape, &mut modules);
        }
        assert_eq!(
            modules,
            [
                "order_line",
                "paint_color",
                "order_payment",
                "card_payment",
                "voucher_payment"
            ]
        );
    }
}
