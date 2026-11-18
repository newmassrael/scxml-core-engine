// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! The Python printer of a CBOR codec's plan (docs/adr/0081 step 4,
//! SCE_FORGE.md §4.6.1).
//!
//! A CBOR map derives nothing a value says: the map head counts the entries
//! present, which is the encoding's and no element. So every entry is
//! presented, as the C++ printer presents it (`cpp_source::cbor`): a member of
//! the `{Codec}Source` `Protocol` for each, `Optional` for one that may be
//! absent. An unsigned integer is held in the class of its width
//! (`BoundedInt`), a text or a byte string in the class of its bound
//! (`BoundedText`, `BoundedBytes`; an exact length is a least equal to the
//! most), and a list of records in the class of its most (`BoundedTuple`, its
//! least 0, as Rust's `BoundedList` holds it), each made where the value is
//! set. What no class holds is refused before the first byte, so a refusal
//! leaves the sink untouched: the refusals the plan's totality names (a list of
//! fewer records than its lower count, a record whose byte string is past its
//! bound), and what the payload type of a record's event schema holds in a bare
//! Python type, its text past its bound and its integer past its width. An enum
//! is the integer of its declared number, major type 1 below zero
//! (docs/adr/0101, section 2.1). The map head
//! and then each entry present are written in ascending key order (`by_key`),
//! every head in its shortest form (RFC 8949 §4.2.1), a list of records as a
//! definite-length array of one map per record, each field under its id as text
//! (docs/adr/0086, section 9.2).

use super::super::*;
use crate::forge::cbor_codec;
use crate::forge::codec_totality::RefusalKind;
use crate::forge::encode_plan::{CborField, CborItem, CborPlan, CborShape};
use crate::forge::model::CborEntry;
use crate::forge::value_type::ValueType;
use crate::generator::Language;
use std::collections::{BTreeMap, BTreeSet};

/// The Python text the printer gives a CBOR codec's module.
pub(in super::super) struct CborSource {
    /// Before the value's class: the class of each entry's bound or width,
    /// the `Protocol` a source of the codec satisfies, the write of each
    /// record, and the encode over any source, `encode_source`.
    pub(in super::super) before: String,
    /// The value's `__setattr__`, which holds each entry to its class wherever
    /// it is set; empty where no entry has one.
    pub(in super::super) hold: String,
    /// For each entry, by its id, the type the value's field is declared as
    /// and the default it takes.
    pub(in super::super) declared: BTreeMap<String, (String, String)>,
    /// The import of the runtime's classes the module names, one statement for
    /// each module of `sce_forge_runtime` a line.
    pub(in super::super) runtime: String,
}

impl CborSource {
    /// What the codec's template reads of the printed text, as its
    /// `python_source`.
    pub(in super::super) fn context(&self) -> serde_json::Value {
        serde_json::json!({
            "before": self.before,
            "hold": self.hold,
            "runtime": self.runtime,
        })
    }
}

/// The type a present value of the leaf entry `e` is, asked of an entry that
/// is no list of records.
fn leaf(e: &CborEntry) -> &SceType {
    e.leaf_type()
        .expect("a leaf entry has a leaf type; a list of records is asked apart")
}

/// The largest number `bits` unsigned bits hold, spelled in hex.
fn most_of(bits: u32) -> String {
    format!(
        "0x{:X}",
        if bits >= 64 {
            u64::MAX
        } else {
            (1u64 << bits) - 1
        }
    )
}

/// How the value holds one entry: the class it is made in, the class's
/// definition, and the default a required entry takes.
struct Held {
    /// The type of a present value: a class of the module, or the enum's or
    /// `bool`.
    ty: String,
    /// The class `__setattr__` makes of what it is handed, where the entry
    /// has one.
    class: Option<String>,
    /// The default of a required entry.
    default: String,
}

/// How the value of `struct_name` holds the entry `item`, its class's
/// definition put into `classes` and the runtime's class it names into
/// `runtime`.
fn held(
    l: &LangCtx,
    item: &CborItem<'_>,
    struct_name: &str,
    classes: &mut String,
    runtime: &mut BTreeSet<&'static str>,
) -> Held {
    let e = item.entry;
    let pascal = filters::to_pascal_case(e.id.clone());
    let mut class_of = |suffix: &str, base: &'static str, attrs: String, what: &str| {
        let class = format!("{struct_name}{pascal}{suffix}");
        runtime.insert(base);
        classes.push_str(&format!(
            "class {class}({base}):\n\
             \x20   \"\"\"The ``{id}`` entry of :class:`{struct_name}`, {what}, held to it where a\n\
             \x20   value of it is made (docs/adr/0077 §3).\"\"\"\n\
             \n\
             {attrs}\n\n",
            id = e.id
        ));
        class
    };
    let bounded = |least: u32, most: Option<u32>| {
        format!(
            "    LEAST = {least}\n    MOST = {}\n",
            most.map_or_else(|| "None".to_string(), |m| m.to_string())
        )
    };
    match &item.value {
        CborShape::Bool => Held {
            ty: "bool".to_string(),
            class: None,
            default: "False".to_string(),
        },
        CborShape::Enum(_) => Held {
            ty: l.type_name(leaf(e)).into_owned(),
            class: None,
            default: l.default_expr(leaf(e)),
        },
        CborShape::Int { signed: false, .. } => {
            let bits = leaf(e)
                .int_bit_width()
                .expect("an unsigned entry has a width");
            let class = class_of(
                "Int",
                "BoundedInt",
                format!("    BITS = {bits}\n    SIGNED = False\n"),
                &format!("an unsigned integer of {bits} bits"),
            );
            Held {
                ty: class.clone(),
                class: Some(class),
                default: "0".to_string(),
            }
        }
        CborShape::Text { .. } => {
            let class = class_of(
                "Text",
                "BoundedText",
                bounded(0, e.size_bound()),
                "a text within its bound in UTF-8 bytes",
            );
            Held {
                ty: class.clone(),
                default: format!("{class}()"),
                class: Some(class),
            }
        }
        CborShape::Bytes { exact, .. } => {
            let exact = *exact;
            let (least, most) = match (exact, e.length) {
                (true, Some(length)) => (length, Some(length)),
                _ => (0, e.size_bound()),
            };
            let what = if exact {
                format!("a byte string of exactly {least} bytes")
            } else {
                "a byte string within its bound".to_string()
            };
            let class = class_of("Bytes", "BoundedBytes", bounded(least, most), &what);
            Held {
                ty: class.clone(),
                default: format!("{class}(b\"\\x00\" * {least})"),
                class: Some(class),
            }
        }
        CborShape::List { elem, max, .. } if matches!(**elem, CborShape::Record(_)) => {
            // The least is 0, as Rust's `BoundedList` holds it, so a value of
            // fewer records than the lower count is one the encode refuses.
            let class = class_of(
                "List",
                "BoundedTuple",
                bounded(0, Some(*max)),
                &format!("at most {max} records"),
            );
            Held {
                ty: class.clone(),
                default: format!("{class}()"),
                class: Some(class),
            }
        }
        _ => unreachable!(
            "a codec of an entry no printer lowers is refused before it renders \
             (`cbor_codec::unlowered_forms`)"
        ),
    }
}

/// A field of a record a list of the map holds, as the payload type of its
/// event schema's Python unit holds it.
struct RecordField {
    /// The field's id, the text its key is, as a Python literal.
    key: String,
    /// The attribute of the payload type that holds it.
    name: String,
    /// What it is on the wire: `bool`, `uint`, `text` or `bytes`.
    kind: &'static str,
    /// The bound of a text or a byte string, or the width of an integer.
    bound: Option<u32>,
}

/// A record the lists of the map hold, written by the module's own function.
struct Record {
    /// The payload type, as the module reaches it.
    payload: String,
    /// The module's write of one record.
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
        let (kind, bound) = cbor_codec::record_field(ty)
            .expect("`cbor_codec::record_list_unlowered` refused a field no codec writes");
        let bound = match ty {
            ValueType::Scalar(t) => t.int_bit_width(),
            _ => bound,
        };
        RecordField {
            key: serde_json::to_string(id).expect("an id is a string"),
            name: crate::forge::generator::event_schema_field_ident(id, Language::Python),
            kind,
            bound,
        }
    };
    let (_, encode_fn) =
        cbor_codec::record_codec_fns(Language::Python, &plan.model.name, &scoped.alias);
    Record {
        payload: cbor_codec::record_payload_type(Language::Python, imports, &scoped.alias),
        encode_fn,
        by_key: scoped.fields_by_key().map(field).collect(),
    }
}

/// The checks of the value `v` of `item` the encode makes before the first
/// byte, each statement at `indent`; empty where it makes none. Only a list of
/// records has any: every other entry is held in a class that holds only what
/// the encoding carries, and an enum of a signed carrier is written whatever
/// its sign (docs/adr/0101, section 2.1).
fn checks(
    plan: &CborPlan<'_>,
    imports: &[ImportContext],
    item: &CborItem<'_>,
    indent: &str,
) -> String {
    let e = item.entry;
    let raise = "raise cbor.CborOutOfRange()";
    let mut out = String::new();
    let CborShape::List { min, .. } = &item.value else {
        return out;
    };
    // Where the analysis names the refusal, the checks below are the ones that
    // write it: the lower count, and a byte string past its bound, which no
    // payload type holds. A Python payload holds a text and an integer in a
    // bare `str` and `int` too, so those are checked alike, a refusal the
    // Python value alone can reach.
    if plan.totality.refuses(RefusalKind::CborOutOfRange, &e.id) {
        plan.totality.require(RefusalKind::CborOutOfRange, &e.id);
    }
    if *min > 0 {
        out.push_str(&format!(
            "{indent}if len(v) < {min}:\n{indent}    {raise}\n"
        ));
    }
    let record = record(plan, imports, item);
    let mut each = String::new();
    for f in &record.by_key {
        let test = match (f.kind, f.bound) {
            ("text", Some(max)) => {
                format!("len(_record.{}.encode(\"utf-8\")) > {max}", f.name)
            }
            ("bytes", Some(max)) => format!("len(_record.{}) > {max}", f.name),
            ("uint", Some(bits)) => {
                format!("not 0 <= _record.{} <= {}", f.name, most_of(bits))
            }
            _ => continue,
        };
        each.push_str(&format!(
            "{indent}    if {test}:\n{indent}        {raise}\n"
        ));
    }
    if !each.is_empty() {
        out.push_str(&format!("{indent}for _record in v:\n{each}"));
    }
    out
}

/// The write of the value `v` of `item`, its key already written, each line at
/// `indent`.
fn write_value(
    l: &LangCtx,
    imports: &[ImportContext],
    plan: &CborPlan<'_>,
    item: &CborItem<'_>,
    indent: &str,
) -> String {
    let e = item.entry;
    match &item.value {
        CborShape::Bool => format!("{indent}cbor.write_bool(w, v)\n"),
        CborShape::Text { .. } => format!("{indent}cbor.write_text(w, v)\n"),
        CborShape::Bytes { .. } => format!("{indent}cbor.write_bytes(w, v)\n"),
        CborShape::Int { signed: false, .. } => format!("{indent}cbor.write_uint(w, v)\n"),
        // The integer of the variant's declared number, major type 1 below zero
        // (docs/adr/0101, section 2.1).
        CborShape::Enum(enumeration) => format!(
            "{indent}cbor.{}(w, {})\n",
            if enumeration.carrier.is_signed() {
                "write_int"
            } else {
                "write_uint"
            },
            l.codec_carrier_expr(leaf(e), "v")
        ),
        CborShape::List { elem, .. } if matches!(**elem, CborShape::Record(_)) => {
            let record = record(plan, imports, item);
            format!(
                "{indent}cbor.write_array_head(w, len(v))\n\
                 {indent}for _record in v:\n\
                 {indent}    {}(w, _record)\n",
                record.encode_fn
            )
        }
        _ => unreachable!(
            "a codec of an entry no printer lowers is refused before it renders \
             (`cbor_codec::unlowered_forms`)"
        ),
    }
}

/// The write of one record of each record the lists of `plan` hold, once: one
/// map of every field, the keys in the order of their encodings. Its value was
/// checked before the first byte, so it refuses nothing.
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
            "def {}(w: SceSink, record: {}) -> None:\n\
             \x20   \"\"\"Write one record as one map of every field, each under its id as\n\
             \x20   text, a shorter key first (RFC 8949 §4.2.1).\"\"\"\n\
             \x20   cbor.write_map_head(w, {})\n",
            record.encode_fn,
            record.payload,
            record.by_key.len()
        ));
        for f in &record.by_key {
            let write = match f.kind {
                "bool" => "write_bool",
                "text" => "write_text",
                "bytes" => "write_bytes",
                _ => "write_uint",
            };
            out.push_str(&format!(
                "    cbor.write_text(w, {})\n    cbor.{write}(w, record.{})\n",
                f.key, f.name
            ));
        }
        out.push_str("\n\n");
    }
    out
}

/// The byte case of `source`, a value of the CBOR codec `plan` at `path`
/// (docs/adr/0081 §5): its dataclass made by its constructor, which holds each
/// entry in the class of its bound or its width, and each entry compared as the
/// value holds it. A map takes no flag input and no caller's tag.
pub(crate) fn source_case(
    plan: &CborPlan<'_>,
    path: &super::source_case::CodecPath,
    source: &serde_json::Value,
) -> Result<super::source_case::SourceCase, String> {
    let elements = super::super::source_notation::cbor_elements(plan, source)?;
    super::source_case::map_or_line_case(path, elements)
}

/// The Python text of the CBOR codec `plan`, whose value's class is
/// `struct_name`.
pub(in super::super) fn render(
    plan: &CborPlan<'_>,
    l: &LangCtx,
    imports: &[ImportContext],
    struct_name: &str,
) -> CborSource {
    let mut classes = String::new();
    let mut runtime = BTreeSet::new();
    let mut hold = String::new();
    let mut declared = BTreeMap::new();
    let mut protocol = format!(
        "class {struct_name}Source(Protocol):\n\
         \x20   \"\"\"The entries a value of :class:`{struct_name}` presents, which its encode\n\
         \x20   reads (docs/adr/0081 §2): any object that presents them is a source of it.\"\"\"\n"
    );
    for item in &plan.entries {
        let e = item.entry;
        let name = l.codec_field_id(&e.id);
        let held = held(l, item, struct_name, &mut classes, &mut runtime);
        let (ty, default) = if item.required {
            (held.ty.clone(), held.default)
        } else {
            (format!("Optional[{}]", held.ty), "None".to_string())
        };
        protocol.push_str(&format!(
            "\n    @property\n    def {name}(self) -> {ty}: ...\n"
        ));
        if let Some(class) = &held.class {
            let made = if item.required {
                format!("{class}(value)")
            } else {
                format!("None if value is None else {class}(value)")
            };
            hold.push_str(&super::hold_line(&hold, &name, &made));
        }
        declared.insert(e.id.clone(), (ty, default));
    }

    // The encode: the checks, then the map head, then each entry present by
    // ascending key.
    let mut encode = format!(
        "def encode_source(src: {struct_name}Source, w: SceSink) -> None:\n\
         \x20   \"\"\"Write the map ``src`` presents into ``w``: the entries present in\n\
         \x20   ascending key order, every head in its shortest form (RFC 8949 §4.2.1).\n\
         \x20   Raises :class:`sce_forge_runtime.cbor.CborOutOfRange` before the first\n\
         \x20   byte for a value no class of an entry holds that the map cannot carry.\"\"\"\n"
    );
    for item in &plan.entries {
        let name = l.codec_field_id(&item.entry.id);
        let (open, inner) = if item.required {
            (String::new(), "    ")
        } else {
            ("    if v is not None:\n".to_string(), "        ")
        };
        let body = checks(plan, imports, item, inner);
        if !body.is_empty() {
            encode.push_str(&format!("    v = src.{name}\n{open}{body}"));
        }
    }
    let required = plan.entries.iter().filter(|item| item.required).count();
    encode.push_str(&format!("    count = {required}\n"));
    for item in plan.entries.iter().filter(|item| !item.required) {
        encode.push_str(&format!(
            "    if src.{} is not None:\n        count += 1\n",
            l.codec_field_id(&item.entry.id)
        ));
    }
    encode.push_str("    cbor.write_map_head(w, count)\n");
    for &at in &plan.by_key {
        let item = &plan.entries[at];
        let key = item.entry.key;
        encode.push_str(&format!(
            "    v = src.{}\n",
            l.codec_field_id(&item.entry.id)
        ));
        let indent = if item.required {
            "    "
        } else {
            encode.push_str("    if v is not None:\n");
            "        "
        };
        encode.push_str(&format!("{indent}cbor.write_uint(w, {key})\n"));
        encode.push_str(&write_value(l, imports, plan, item, indent));
    }
    plan.totality.assert_all_written(struct_name);

    let runtime = if runtime.is_empty() {
        String::new()
    } else {
        format!(
            "from sce_forge_runtime.bounded import {}",
            runtime.into_iter().collect::<Vec<_>>().join(", ")
        )
    };
    CborSource {
        before: format!(
            "{classes}{protocol}\n\n{}{encode}",
            record_writers(plan, imports)
        ),
        hold: super::setattr(&hold).trim_end_matches('\n').to_string(),
        declared,
        runtime,
    }
}
