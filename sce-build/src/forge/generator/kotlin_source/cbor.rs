// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! The Kotlin printer of a CBOR codec's plan (docs/adr/0081 step 4,
//! SCE_FORGE.md §4.6.1).
//!
//! A CBOR map derives nothing a value says: the map head counts the entries
//! present, which is the encoding's and no element. So every entry is
//! presented, as the C++ printer presents it (`cpp_source::cbor`): a property
//! of the `{Codec}Source` interface for each, `null` for one the map may lack.
//! A text or a byte string is held in the value class of its bound, as a
//! positional codec's is ([`super::bounded_class`]; an exact length is a least
//! equal to the most), and a list of records in the value class of its most
//! ([`super::bounded_list_of`], its least 0, as Rust's `BoundedList` holds it),
//! each made only by its checked factory. An unsigned integer is the Kotlin
//! unsigned type of its width, which holds no number past it. What no class
//! holds is refused before the first byte, so a refusal leaves the sink
//! untouched: the refusals the plan's totality names (a list of fewer records
//! than its lower count, a record whose byte string is past its bound), and a
//! record's text past its bound, which the payload type of a record's event
//! schema holds in a bare `String`. An enum is the integer of its declared
//! number, major type 1 below zero (docs/adr/0101, section 2.1). The map head and then each entry present are written in ascending
//! key order (`by_key`), every head in its shortest form (RFC 8949 §4.2.1), a
//! list of records as a definite-length array of one map per record, each field
//! under its id as text (docs/adr/0086, section 9.2).

use super::super::*;
use super::{bounded_class, bounded_list_of, list_class, refinement_class};
use crate::forge::cbor_codec;
use crate::forge::codec_failure::CodecFailure;
use crate::forge::codec_totality::RefusalKind;
use crate::forge::encode_plan::{Bound, CborField, CborItem, CborPlan, CborShape};
use crate::forge::model::CborEntry;
use crate::forge::value_type::ValueType;
use crate::generator::Language;
use std::collections::{BTreeMap, BTreeSet};

/// How the Kotlin value holds one entry of a printed map: the type its
/// property is declared as, the value it holds by default, and what the decode
/// makes it of from the local it read the entry into, `null` where the map
/// lacked it.
pub(in super::super) struct Declared {
    /// The type of the property.
    pub(in super::super) ty: String,
    /// The value it holds by default.
    pub(in super::super) default: String,
    /// What the decode hands the constructor for it.
    pub(in super::super) made: String,
}

/// The Kotlin text the printer gives a CBOR codec's file.
pub(in super::super) struct CborSource {
    /// Before the value's class: the class of each text's, byte string's and
    /// list's bound and the interface a source of the codec implements.
    pub(in super::super) before: String,
    /// The encode over any source, `encodeSource`, and the write of each
    /// record the lists hold, functions of the value's companion object, at
    /// their indentation there.
    pub(in super::super) encode: String,
    /// For each entry, by its id, how the value holds it.
    pub(in super::super) declared: BTreeMap<String, Declared>,
    /// Whether the file declares a value class, for which it imports
    /// `JvmInline`.
    pub(in super::super) inline: bool,
}

/// The type a present value of the leaf entry `e` is, asked of an entry that
/// is no list of records.
fn leaf(e: &CborEntry) -> &SceType {
    e.leaf_type()
        .expect("a leaf entry has a leaf type; a list of records is asked apart")
}

/// The local the encode reads the entry `name` into, once.
fn local(name: &str) -> String {
    format!("_e_{name}")
}

/// What one record's field is on the wire, as the payload type of its event
/// schema's Kotlin unit holds it.
enum RecordFieldKind {
    Bool,
    /// An unsigned integer; `wide` where it is a `ULong`, which a CBOR integer
    /// takes as it is.
    Uint {
        wide: bool,
    },
    /// A text of at most `max` UTF-8 bytes, held in a bare `String`.
    Text {
        max: u32,
    },
    /// A byte string of at most `max` bytes, held in a bare `ByteArray`.
    Bytes {
        max: u32,
    },
}

/// A field of a record a list of the map holds.
struct RecordField {
    /// The field's id, the text its key is, as a Kotlin literal.
    key: String,
    /// The property of the payload type that holds it.
    name: String,
    kind: RecordFieldKind,
}

/// A record the lists of the map hold, written by the codec's own function.
struct Record {
    /// The schema's name.
    schema: String,
    /// The alias the codec imports the schema under.
    alias: String,
    /// The payload type its event schema's own unit declares.
    payload: String,
    /// The companion's write of one record.
    encode_fn: String,
    /// Its fields in the order its map writes them, as the plan decides it
    /// (`CborRecord::by_key`).
    by_key: Vec<RecordField>,
}

/// The record the list `item` holds.
fn record(plan: &CborPlan<'_>, imports: &[ImportContext], item: &CborItem<'_>) -> Record {
    let scoped = item
        .value
        .listed_record()
        .expect("a list entry holds a list of records");
    let field = |f: &CborField| -> RecordField {
        let (id, ty) = (&f.id, &f.ty);
        let kind = match ty {
            ValueType::Text { max } => RecordFieldKind::Text { max: max.max },
            ValueType::Bytes { max } => RecordFieldKind::Bytes { max: max.max },
            ValueType::Scalar(t) => match cbor_codec::record_field(ty) {
                Some(("bool", _)) => RecordFieldKind::Bool,
                Some(("uint", _)) => RecordFieldKind::Uint {
                    wide: *t == SceType::Uint64,
                },
                _ => unreachable!(
                    "a record's scalar field is a boolean or an unsigned integer \
                     (`cbor_codec::record_list_unlowered`)"
                ),
            },
            _ => unreachable!(
                "a record a CBOR list holds is a map of unsigned integers, booleans, texts and \
                 byte strings (`cbor_codec::record_list_unlowered`)"
            ),
        };
        RecordField {
            key: format!("{id:?}"),
            name: crate::forge::generator::event_schema_field_ident(id, Language::Kotlin),
            kind,
        }
    };
    let (_, encode_fn) =
        cbor_codec::record_codec_fns(Language::Kotlin, &plan.model.name, &scoped.alias);
    Record {
        schema: scoped.name.clone(),
        alias: scoped.alias.clone(),
        payload: cbor_codec::record_payload_type(Language::Kotlin, imports, &scoped.alias),
        encode_fn,
        by_key: scoped.fields_by_key().map(field).collect(),
    }
}

/// How the value of `struct_name` holds the present value of `item`: its type,
/// its default, and the class whose `of` the decode makes it by, where it has
/// one. The class's declaration goes into `before`.
fn held(
    plan: &CborPlan<'_>,
    l: &LangCtx,
    imports: &[ImportContext],
    item: &CborItem<'_>,
    struct_name: &str,
    before: &mut String,
) -> (String, String, Option<String>) {
    let e = item.entry;
    let classed = |class: String| (class.clone(), format!("{class}.fewest()"), Some(class));
    match &item.value {
        CborShape::Bool | CborShape::Enum(_) | CborShape::Int { signed: false, .. } => (
            l.type_name(leaf(e)).into_owned(),
            l.default_expr(leaf(e)),
            None,
        ),
        CborShape::Text { .. } => {
            let class = refinement_class(struct_name, &e.id, &SceType::String);
            let bound = Bound {
                least: 0,
                most: e.size_bound().map(u64::from),
            };
            before.push_str(&bounded_class(struct_name, &e.id, &class, &bound, true));
            classed(class)
        }
        CborShape::Bytes { exact, .. } => {
            let class = refinement_class(struct_name, &e.id, &SceType::Bytes);
            let bound = match (*exact, e.length) {
                (true, Some(length)) => Bound {
                    least: u64::from(length),
                    most: Some(u64::from(length)),
                },
                _ => Bound {
                    least: 0,
                    most: e.size_bound().map(u64::from),
                },
            };
            before.push_str(&bounded_class(struct_name, &e.id, &class, &bound, false));
            classed(class)
        }
        CborShape::List { elem, max, .. } if matches!(**elem, CborShape::Record(_)) => {
            let payload = record(plan, imports, item).payload;
            let class = list_class(struct_name, &e.id);
            // The least is 0, as Rust's `BoundedList` holds it, so a value of
            // fewer records than the lower count is one the encode refuses.
            let bound = Bound {
                least: 0,
                most: Some(u64::from(*max)),
            };
            before.push_str(&bounded_list_of(
                struct_name,
                &e.id,
                &class,
                &bound,
                &payload,
                &payload,
            ));
            classed(class)
        }
        _ => unreachable!(
            "a codec of an entry no printer lowers is refused before it renders \
             (`cbor_codec::unlowered_forms`)"
        ),
    }
}

/// The kind a decoded value of `item` that the class of its bound does not
/// hold is refused with: a byte string of a stated length that has another
/// `CborWrongLength`, and any other value past its bound `CborOutOfRange`, the
/// kinds the runtime's CBOR readers answer for them (SCE_FORGE.md §4.6.1).
fn past_bound(item: &CborItem<'_>) -> CodecFailure {
    match (&item.value, item.entry.length) {
        (CborShape::Bytes { exact: true, .. }, Some(_)) => CodecFailure::CborWrongLength,
        _ => CodecFailure::CborOutOfRange,
    }
}

/// The checks of the value `v` of `item` the encode makes before the first
/// byte, each statement at `indent`; empty where it makes none. Only a list of
/// records has any: every other entry is held in a class that holds only what
/// the encoding carries, an enum of a signed carrier among them (docs/adr/0101,
/// section 2.1).
fn checks(
    plan: &CborPlan<'_>,
    imports: &[ImportContext],
    item: &CborItem<'_>,
    v: &str,
    indent: &str,
) -> String {
    let e = item.entry;
    let mut out = String::new();
    let CborShape::List { min, .. } = &item.value else {
        return out;
    };
    // Where the analysis names the refusal, the checks below are the ones that
    // write it: the lower count, and a byte string past its bound, which no
    // payload type holds. A Kotlin payload holds a text in a bare `String`
    // too, so a text past its bound is checked alike, a refusal the Kotlin
    // value alone can reach.
    let refuse = if plan.totality.refuses(RefusalKind::CborOutOfRange, &e.id) {
        plan.totality
            .require(RefusalKind::CborOutOfRange, &e.id)
            .replace("::", ".")
    } else {
        "CodecError.CborOutOfRange".to_string()
    };
    if *min > 0 {
        out.push_str(&format!("{indent}if ({v}.size < {min}) return {refuse}\n"));
    }
    let record = record(plan, imports, item);
    let mut each = String::new();
    for f in &record.by_key {
        match f.kind {
            // A text no UTF-8 holds (one with a lone surrogate) has no CBOR
            // text either.
            RecordFieldKind::Text { max } => each.push_str(&format!(
                "{indent}    if ((com.sce.forge.runtime.Utf8.size(_record.{name}) ?: return {refuse}) > {max}) \
                 return {refuse}\n",
                name = f.name
            )),
            RecordFieldKind::Bytes { max } => each.push_str(&format!(
                "{indent}    if (_record.{}.size > {max}) return {refuse}\n",
                f.name
            )),
            RecordFieldKind::Bool | RecordFieldKind::Uint { .. } => {}
        }
    }
    if !each.is_empty() {
        out.push_str(&format!(
            "{indent}for (_record in {v}.entries) {{\n{each}{indent}}}\n"
        ));
    }
    out
}

/// The writes of the value `v` of `item`, its key already written, each line
/// at `indent`.
fn write_value(
    plan: &CborPlan<'_>,
    l: &LangCtx,
    imports: &[ImportContext],
    item: &CborItem<'_>,
    v: &str,
    indent: &str,
) -> String {
    let e = item.entry;
    let call = |call: String| format!("{indent}{call}?.let {{ return it }}\n");
    match &item.value {
        CborShape::Bool => call(format!("Cbor.writeBool(w, {v})")),
        CborShape::Text { .. } => call(format!("Cbor.writeText(w, {v}.text)")),
        CborShape::Bytes { .. } => call(format!("Cbor.writeBytes(w, {v}.bytes)")),
        CborShape::Int {
            bits: 64,
            signed: false,
        } => call(format!("Cbor.writeUint(w, {v})")),
        CborShape::Int { signed: false, .. } => call(format!("Cbor.writeUint(w, {v}.toULong())")),
        // The integer of the variant's declared number, major type 1 below zero
        // (docs/adr/0101, section 2.1); a `Long` and a `ULong` carrier are
        // written as they are.
        CborShape::Enum(enumeration) => {
            let number = l.codec_carrier_expr(leaf(e), v);
            call(match &enumeration.carrier {
                SceType::Int64 => format!("Cbor.writeInt(w, {number})"),
                carrier if carrier.is_signed() => format!("Cbor.writeInt(w, {number}.toLong())"),
                SceType::Uint64 => format!("Cbor.writeUint(w, {number})"),
                _ => format!("Cbor.writeUint(w, {number}.toULong())"),
            })
        }
        CborShape::List { elem, .. } if matches!(**elem, CborShape::Record(_)) => {
            let encode_fn = record(plan, imports, item).encode_fn;
            let mut out = call(format!("Cbor.writeArrayHead(w, {v}.size.toULong())"));
            out.push_str(&format!(
                "{indent}for (_record in {v}.entries) {{\n\
                 {indent}    {encode_fn}(w, _record)?.let {{ return it }}\n\
                 {indent}}}\n"
            ));
            out
        }
        _ => unreachable!(
            "a codec of an entry no printer lowers is refused before it renders \
             (`cbor_codec::unlowered_forms`)"
        ),
    }
}

/// The write of one record of each record the lists of `plan` hold, once, a
/// private function of the companion object: one map of every field, the keys
/// in the order of their encodings. Its value was checked before the first
/// byte, so it refuses nothing a sink does not.
fn record_writers(plan: &CborPlan<'_>, imports: &[ImportContext]) -> String {
    let mut out = String::new();
    let mut done = BTreeSet::new();
    for item in &plan.entries {
        if item.value.listed_record().is_none() {
            continue;
        }
        let record = record(plan, imports, item);
        if !done.insert(record.encode_fn.clone()) {
            continue;
        }
        out.push_str(&format!(
            "\n\
             \x20       /// Write one record of `{schema}` (`record:{alias}`) into [w] as one map of\n\
             \x20       /// every field, the keys in the order RFC 8949 §4.2.1 sorts their encodings: a\n\
             \x20       /// shorter key first. The encode held the record to its bounds before the\n\
             \x20       /// first byte.\n\
             \x20       private fun {encode}(w: SceSink, record: {payload}): CodecError? {{\n\
             \x20           Cbor.writeMapHead(w, {count}UL)?.let {{ return it }}\n",
            schema = record.schema,
            alias = record.alias,
            encode = record.encode_fn,
            payload = record.payload,
            count = record.by_key.len(),
        ));
        for f in &record.by_key {
            let value = match f.kind {
                RecordFieldKind::Bool => format!("Cbor.writeBool(w, record.{})", f.name),
                RecordFieldKind::Uint { wide: true } => {
                    format!("Cbor.writeUint(w, record.{})", f.name)
                }
                RecordFieldKind::Uint { wide: false } => {
                    format!("Cbor.writeUint(w, record.{}.toULong())", f.name)
                }
                RecordFieldKind::Text { .. } => format!("Cbor.writeText(w, record.{})", f.name),
                RecordFieldKind::Bytes { .. } => {
                    format!("Cbor.writeBytes(w, record.{})", f.name)
                }
            };
            out.push_str(&format!(
                "            Cbor.writeText(w, {})?.let {{ return it }}\n\
                 \x20           {value}?.let {{ return it }}\n",
                f.key
            ));
        }
        out.push_str("            return null\n        }\n");
    }
    out
}

/// The byte case of `source`, a value of the CBOR codec `plan` at `path`
/// (docs/adr/0081 §5): its data class made by its constructor, each text and
/// byte string by the factory of the value class of its bound, as [`held`]
/// holds it, and each entry compared as the value holds it. A map takes no
/// flag input and no caller's tag.
pub(crate) fn source_case(
    plan: &CborPlan<'_>,
    path: &super::source_case::CodecPath,
    source: &serde_json::Value,
) -> Result<super::source_case::SourceCase, String> {
    use super::source_case::Holds;
    let elements = super::super::source_notation::cbor_elements(plan, source)?;
    let holds = plan.entries.iter().map(|item| {
        let e = item.entry;
        match item.value {
            CborShape::Bool => Holds::Bool,
            CborShape::Int { signed: false, .. } => Holds::Int(leaf(e)),
            CborShape::Text { .. } => {
                Holds::Classed(refinement_class(&path.type_name, &e.id, &SceType::String))
            }
            CborShape::Bytes { .. } => {
                Holds::Classed(refinement_class(&path.type_name, &e.id, &SceType::Bytes))
            }
            _ => unreachable!(
                "the notation refuses a value of an enum, a list of records and a type no \
                 printer lowers (`source_notation::cbor_elements`)"
            ),
        }
    });
    super::source_case::map_or_line_case(path, elements.into_iter().zip(holds).collect())
}

/// The Kotlin text of the CBOR codec `plan`, whose value's data class is
/// `struct_name`.
pub(in super::super) fn render(
    plan: &CborPlan<'_>,
    l: &LangCtx,
    imports: &[ImportContext],
    struct_name: &str,
) -> CborSource {
    let mut before = String::new();
    let mut declared = BTreeMap::new();
    let mut inline = false;
    let mut interface = format!(
        "/// The entries a value of [{struct_name}] presents, which its encode reads\n\
         /// (docs/adr/0081 §2): any object that presents them is a source of it, which\n\
         /// [{struct_name}.encodeSource] writes.\n\
         interface {struct_name}Source {{\n"
    );
    for item in &plan.entries {
        let e = item.entry;
        let name = l.codec_field_id(&e.id);
        let (ty, default, class) = held(plan, l, imports, item, struct_name, &mut before);
        inline |= class.is_some();
        // The decode read the entry into a local of its name, `null` where the
        // map lacked it, which a required entry may not: the map is refused as
        // one that lacks a required key. A value its class does not hold is
        // refused with the kind a CBOR reader refuses it with (docs/adr/0099,
        // amendment of 2026-11-16).
        let missing = CodecFailure::CborRequiredKeyMissing.raise_stmt(Language::Kotlin);
        let past = past_bound(item).raise_stmt(Language::Kotlin);
        let (ty, default, made) = match (item.required, class) {
            (true, Some(class)) => (
                ty,
                default,
                format!("{class}.of({name} ?: {missing}) ?: {past}"),
            ),
            (true, None) => (ty, default, format!("{name} ?: {missing}")),
            (false, Some(class)) => (
                format!("{ty}?"),
                "null".to_string(),
                format!("{name}?.let {{ {class}.of(it) ?: {past} }}"),
            ),
            (false, None) => (format!("{ty}?"), "null".to_string(), name.clone()),
        };
        interface.push_str(&format!("    val {name}: {ty}\n"));
        declared.insert(e.id.clone(), Declared { ty, default, made });
    }
    interface.push_str("}\n");
    before.push_str(&interface);

    // The encode: each entry read once, the checks, then the map head, then
    // each entry present by ascending key.
    let mut encode = format!(
        "        /// Write the map [src] presents into [w]: the entries present in ascending key\n\
         \x20       /// order, every head in its shortest form (RFC 8949 §4.2.1). `null` on\n\
         \x20       /// success; a value no class of an entry holds that the map cannot carry is\n\
         \x20       /// refused with [CodecError.CborOutOfRange] before the first byte.\n\
         \x20       fun encodeSource(src: {struct_name}Source, w: SceSink): CodecError? {{\n"
    );
    for item in &plan.entries {
        let name = l.codec_field_id(&item.entry.id);
        encode.push_str(&format!("            val {} = src.{name}\n", local(&name)));
    }
    for item in &plan.entries {
        let v = local(&l.codec_field_id(&item.entry.id));
        if item.required {
            encode.push_str(&checks(plan, imports, item, &v, "            "));
        } else {
            let body = checks(plan, imports, item, &v, "                ");
            if !body.is_empty() {
                encode.push_str(&format!(
                    "            if ({v} != null) {{\n{body}            }}\n"
                ));
            }
        }
    }
    let required = plan.required();
    if plan.entries.iter().all(|item| item.required) {
        encode.push_str(&format!("            val count = {required}UL\n"));
    } else {
        encode.push_str(&format!("            var count = {required}UL\n"));
        for item in plan.entries.iter().filter(|item| !item.required) {
            encode.push_str(&format!(
                "            if ({} != null) count++\n",
                local(&l.codec_field_id(&item.entry.id))
            ));
        }
    }
    encode.push_str("            Cbor.writeMapHead(w, count)?.let { return it }\n");
    for &at in &plan.by_key {
        let item = &plan.entries[at];
        let v = local(&l.codec_field_id(&item.entry.id));
        let key = item.entry.key;
        if item.required {
            encode.push_str(&format!(
                "            Cbor.writeUint(w, {key}UL)?.let {{ return it }}\n"
            ));
            encode.push_str(&write_value(plan, l, imports, item, &v, "            "));
        } else {
            encode.push_str(&format!(
                "            if ({v} != null) {{\n\
                 \x20               Cbor.writeUint(w, {key}UL)?.let {{ return it }}\n\
                 {}\
                 \x20           }}\n",
                write_value(plan, l, imports, item, &v, "                ")
            ));
        }
    }
    encode.push_str("            return null\n        }\n");
    encode.push_str(&record_writers(plan, imports));
    plan.totality.assert_all_written(struct_name);

    CborSource {
        before: before.trim_end_matches('\n').to_string(),
        encode: encode.trim_end_matches('\n').to_string(),
        declared,
        inline,
    }
}
