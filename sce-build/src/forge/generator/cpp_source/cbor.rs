// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! The C++ printer of a CBOR codec's plan (docs/adr/0081 step 4, SCE_FORGE.md
//! §4.6.1).
//!
//! A CBOR map derives nothing a value says: the map head counts the entries
//! present, which is the encoding's and no element. So every entry is
//! presented, as the C++ interface presents a positional codec's elements: one
//! accessor for each, the value itself for a required entry and a
//! `std::optional` for one that may be absent, read through the `source`
//! customization points and checked by `is_source`. A text or a byte string is
//! presented within its bound (`Text`, `Bytes`; docs/adr/0077 §3), and a list
//! of records within its count as a view of the payload type its event
//! schema's own unit declares (`List`; docs/adr/0085, Landing order step 3a).
//! What no type holds is refused, where the plan's totality names it and as
//! the Rust printer refuses it: a negative value of an enum whose carrier is
//! signed, a list of fewer records than its lower count, and a record whose
//! byte string, which the schema's unit holds in a `std::vector` with no
//! bound, is past its bound. Each is found before the first byte is written,
//! so a refusal leaves the sink untouched. The encoding rule is the one the
//! Rust printer writes (`rust_source::cbor`): the map head, then each entry
//! present in ascending key order, every head in its shortest form (RFC 8949
//! §4.2.1), a list of records as a definite-length array of one map per
//! record, each field under its id as text (docs/adr/0086, section 9.2).

use super::super::*;
use super::source_readers;
use crate::forge::cbor_codec;
use crate::forge::codec_totality::RefusalKind;
use crate::forge::encode_plan::{Carrier, CborItem, CborPlan, CborValue};
use crate::forge::model::CborEntry;
use crate::forge::value_type::ValueType;
use crate::generator::Language;

/// Every form of the CBOR codec `plan` the C++ printer does not print, each a
/// clause the codec's refusal names it by: none, since every CBOR plan is
/// printed. An encode that can refuse is
/// printed with its checks before the first byte (`refusal_checks`), and a
/// record's byte string is held in the `std::vector` of the schema's unit and
/// held to its bound by those checks. The caller asks every plan alike, as it
/// asks a positional one, so a form a later plan adds is named here.
pub(in super::super) fn unlowered_forms(
    _plan: &CborPlan<'_>,
    _imports: &[ImportContext],
) -> Vec<String> {
    Vec::new()
}

/// The type a present value of the leaf entry `e` is, asked of an entry that
/// is no list of records.
fn leaf(e: &CborEntry) -> &SceType {
    e.leaf_type()
        .expect("a leaf entry has a leaf type; a list of records is asked apart")
}

/// A field of a record a printed list holds: a boolean, an unsigned integer, a
/// text or a byte string, the only fields a record a CBOR list holds has
/// (`cbor_codec::record_list_unlowered`).
struct RecordField {
    /// The field's id, the text its key is.
    key: String,
    /// The member of the payload type that holds it.
    name: String,
    /// The member's type.
    ty: String,
    /// What it is on the wire.
    kind: RecordFieldKind,
}

#[derive(Clone, Copy)]
enum RecordFieldKind {
    Bool,
    /// An unsigned integer; `wide` where it is 64 bits, which a CBOR integer
    /// holds as it is.
    Uint {
        wide: bool,
    },
    /// A text of at most `max` UTF-8 bytes, held in the `OwnedText` of that
    /// bound (docs/adr/0086, section 7.4), so its write refuses nothing.
    Text {
        max: u32,
    },
    /// A byte string of at most `max` bytes, held in the `std::vector` of the
    /// schema's unit, which carries no bound: the read holds it to `max`, and
    /// the map's encode refuses one past it before the first byte
    /// (`refusal_checks`).
    Bytes {
        max: u32,
    },
}

/// A record the lists of a printed map hold, read and written by functions of
/// the codec's own (`cbor_codec::record_codec_fns`) into the payload type its
/// event schema's own unit declares (`cbor_codec::record_payload_type`).
struct Record {
    /// The schema's name.
    schema: String,
    /// The alias the codec imports the schema under.
    alias: String,
    payload: String,
    decode_fn: String,
    encode_fn: String,
    /// Its fields in the schema's order.
    fields: Vec<RecordField>,
    /// The same fields in the order its map writes them: by the bytes of each
    /// key's encoding (`cbor_codec::record_fields_by_key`).
    by_key: Vec<RecordField>,
}

/// Each record the lists of `plan` hold, once, in the order the entries first
/// name them.
fn records(plan: &CborPlan<'_>, l: &LangCtx, imports: &[ImportContext]) -> Vec<Record> {
    let scope = crate::forge::generator::import_type_scope(imports);
    let field = |(id, ty): &(String, ValueType)| -> RecordField {
        let (kind, held) = match ty {
            ValueType::Text { max } => (
                RecordFieldKind::Text { max: max.max },
                crate::forge::composed_lowering::cpp_text_type(max.max),
            ),
            // The schema's unit holds a byte string in the language's own
            // vector (`cbor_codec::record_unbounded_bytes`).
            ValueType::Bytes { max } => (
                RecordFieldKind::Bytes { max: max.max },
                l.type_name(&SceType::Bytes).into_owned(),
            ),
            ValueType::Scalar(t) => {
                let kind = match cbor_codec::record_field(ty) {
                    Some(("bool", _)) => RecordFieldKind::Bool,
                    Some(("uint", _)) => RecordFieldKind::Uint {
                        wide: *t == SceType::Uint64,
                    },
                    _ => unreachable!(
                        "a record's scalar field is a boolean or an unsigned integer \
                         (`cbor_codec::record_list_unlowered`)"
                    ),
                };
                (kind, l.type_name(t).into_owned())
            }
            _ => unreachable!(
                "a record a CBOR list holds is a map of unsigned integers, booleans, texts and \
                 byte strings (`cbor_codec::record_list_unlowered`)"
            ),
        };
        RecordField {
            key: id.clone(),
            name: crate::forge::generator::event_schema_field_ident(id, Language::Cpp),
            ty: held,
            kind,
        }
    };
    let mut out: Vec<Record> = Vec::new();
    for item in &plan.entries {
        let Some(list) = item.entry.record_list() else {
            continue;
        };
        if out.iter().any(|r| r.alias == list.record) {
            continue;
        }
        let record = scope
            .records
            .get(list.record)
            .expect("the import resolved the record (`cbor_codec::record_list_unlowered`)");
        let (decode_fn, encode_fn) =
            cbor_codec::record_codec_fns(Language::Cpp, &plan.model.name, list.record);
        out.push(Record {
            schema: record.name.clone(),
            alias: list.record.to_string(),
            payload: cbor_codec::record_payload_type(Language::Cpp, imports, list.record),
            decode_fn,
            encode_fn,
            fields: record.fields.iter().map(field).collect(),
            by_key: cbor_codec::record_fields_by_key(record)
                .into_iter()
                .map(field)
                .collect(),
        });
    }
    out
}

/// The `Records` struct of a printed map: for each record its lists hold, the
/// write of one record as one map of every field and the read of one back.
fn records_struct(records: &[Record], struct_name: &str) -> String {
    let mut out = format!(
        "/// The records the lists of `{struct_name}` hold, each in the payload type its\n\
         /// event schema's own unit declares and written as one CBOR map of every\n\
         /// field, each under its id as text (docs/adr/0086, section 9.2).\n\
         struct Records {{\n"
    );
    for (place, r) in records.iter().enumerate() {
        if place > 0 {
            out.push('\n');
        }
        let mut write = String::new();
        for f in &r.by_key {
            let value = match f.kind {
                RecordFieldKind::Bool => format!("cbor::write_bool(w, record.{})", f.name),
                RecordFieldKind::Uint { .. } => format!(
                    "cbor::write_uint(w, static_cast<std::uint64_t>(record.{}))",
                    f.name
                ),
                RecordFieldKind::Text { .. } => {
                    format!("cbor::write_text(w, record.{}.inner())", f.name)
                }
                // Within its bound: the map's encode refused one past it
                // before the first byte (`refusal_checks`).
                RecordFieldKind::Bytes { .. } => {
                    format!("cbor::write_bytes(w, record.{})", f.name)
                }
            };
            write.push_str(&format!(
                "        if (auto _e = cbor::write_text(w, {key:?}); _e) return _e;\n\
                 \x20       if (auto _e = {value}; _e) return _e;\n",
                key = f.key
            ));
        }
        let mut held = String::new();
        let mut reads = String::new();
        let mut present = String::new();
        let mut made = String::new();
        for (index, f) in r.fields.iter().enumerate() {
            held.push_str(&format!(
                "        std::optional<{}> held_{};\n",
                f.ty, f.name
            ));
            let read = match f.kind {
                RecordFieldKind::Bool => format!(
                    "                held_{name} = cbor::read_bool(c);\n\
                     \x20               if (!held_{name}) return std::nullopt;\n",
                    name = f.name
                ),
                RecordFieldKind::Uint { wide: true } => format!(
                    "                held_{name} = cbor::read_uint(c);\n\
                     \x20               if (!held_{name}) return std::nullopt;\n",
                    name = f.name
                ),
                RecordFieldKind::Uint { wide: false } => format!(
                    "                const auto raw = cbor::read_uint_upto(c, static_cast<std::uint64_t>(std::numeric_limits<{ty}>::max()));\n\
                     \x20               if (!raw) return std::nullopt;\n\
                     \x20               held_{name} = static_cast<{ty}>(*raw);\n",
                    name = f.name,
                    ty = f.ty
                ),
                // Read within the bound and as UTF-8, which is what the
                // field's type holds, so its `make` always makes it.
                RecordFieldKind::Text { max } => format!(
                    "                auto text = cbor::read_text(c, {max});\n\
                     \x20               if (!text) return std::nullopt;\n\
                     \x20               held_{name} = {ty}::make(std::move(*text));\n\
                     \x20               if (!held_{name}) return std::nullopt;\n",
                    name = f.name,
                    ty = f.ty
                ),
                // Read within the bound, as the Rust printer reads it, into
                // the vector the field is.
                RecordFieldKind::Bytes { max } => format!(
                    "                held_{name} = cbor::read_bytes(c, {max});\n\
                     \x20               if (!held_{name}) return std::nullopt;\n",
                    name = f.name
                ),
            };
            reads.push_str(&format!(
                "            {open}if (*key == {key:?}) {{\n\
                 \x20               // A key given twice is not one map entry.\n\
                 \x20               if (held_{name}.has_value()) return std::nullopt;\n\
                 {read}",
                open = if index == 0 { "" } else { "} else " },
                key = f.key,
                name = f.name
            ));
            present.push_str(&format!(
                "        if (!held_{}.has_value()) return std::nullopt;\n",
                f.name
            ));
            made.push_str(&format!(
                "        record.{name} = *held_{name};\n",
                name = f.name
            ));
        }
        out.push_str(&format!(
            "    /// Write one record of `{schema}` (`record:{alias}`) as one map of every\n\
             \x20   /// field, the keys in the order RFC 8949 §4.2.1 sorts their encodings: a\n\
             \x20   /// shorter key first.\n\
             \x20   [[nodiscard]] static std::optional<::SCE::Forge::CodecError> {encode}(::SCE::Forge::SceSink& w, const {payload}& record) noexcept {{\n\
             \x20       namespace cbor = ::SCE::Forge::Cbor;\n\
             \x20       if (auto _e = cbor::write_map_head(w, {count}); _e) return _e;\n\
             {write}\
             \x20       return std::nullopt;\n\
             \x20   }}\n\n\
             \x20   /// Read one record of `{schema}` from `c`: its keys in any order, a key the\n\
             \x20   /// schema does not declare skipped whole; a key given twice or a field\n\
             \x20   /// missing is `std::nullopt`.\n\
             \x20   static std::optional<{payload}> {decode}(::SCE::Forge::SceCursor& c) {{\n\
             \x20       namespace cbor = ::SCE::Forge::Cbor;\n\
             \x20       const auto count = cbor::read_map_len(c);\n\
             \x20       if (!count) return std::nullopt;\n\
             {held}\
             \x20       for (std::uint64_t i = 0; i < *count; ++i) {{\n\
             \x20           const auto key = cbor::read_text(c, std::nullopt);\n\
             \x20           if (!key) return std::nullopt;\n\
             {reads}\
             \x20           }} else if (!cbor::skip(c)) {{\n\
             \x20               return std::nullopt;\n\
             \x20           }}\n\
             \x20       }}\n\
             {present}\
             \x20       {payload} record;\n\
             {made}\
             \x20       return record;\n\
             \x20   }}\n",
            schema = r.schema,
            alias = r.alias,
            encode = r.encode_fn,
            decode = r.decode_fn,
            payload = r.payload,
            count = r.fields.len(),
        ));
    }
    out.push_str("};\n\n");
    out
}

/// One entry as the printed value presents and holds it.
struct Entry<'p> {
    item: &'p CborItem<'p>,
    /// Its accessor's name.
    name: String,
    /// What the value presents it as where it is there: a text, a byte string
    /// or a list of records as its bound's view, anything else as its type.
    view: String,
    /// What the value holds it as where it is there: a text, a byte string or
    /// a list of records in its bound's owning list.
    held: String,
    /// What the decode reads it as before it is held to its bound.
    read: String,
    /// The initializer of what the value holds, where a value made by
    /// default does not hold its type's zero: an enum starts at its first
    /// declared variant.
    init: String,
    /// For a list of records, the read and the write of one record, as
    /// `Records` declares them: `(decode, encode)`.
    record_fns: Option<(String, String)>,
}

impl Entry<'_> {
    fn required(&self) -> bool {
        self.item.required
    }

    /// Whether it is a text, a byte string or a list of records, presented as
    /// its bound's view.
    fn bounded(&self) -> bool {
        matches!(
            self.item.value,
            CborValue::Text | CborValue::Bytes { .. } | CborValue::RecordList { .. }
        )
    }

    /// What a source presents it as: its view, or an optional of it.
    fn presented(&self) -> String {
        if self.required() {
            self.view.clone()
        } else {
            format!("std::optional<{}>", self.view)
        }
    }

    fn held_member(&self) -> String {
        if self.required() {
            self.held.clone()
        } else {
            format!("std::optional<{}>", self.held)
        }
    }
}

/// The bound a text or a byte string of `e` is held to, as `Max` or `Max, Min`.
fn bound_args(e: &CborEntry, exact: bool) -> String {
    match (exact, e.length, e.max_size) {
        (true, Some(length), _) => format!("{length}, {length}"),
        (_, _, Some(max)) => max.to_string(),
        _ => "::SCE::Forge::kUnbounded".to_string(),
    }
}

fn entry<'p>(
    l: &LangCtx,
    imports: &[ImportContext],
    codec: &str,
    item: &'p CborItem<'p>,
) -> Entry<'p> {
    let e = item.entry;
    if let Some(list) = e.record_list() {
        // A view of the schema's payload within the list's count, held in the
        // owning list of the same bound (docs/adr/0065 §6 R2, docs/adr/0081
        // step 4), as a positional codec holds a list of entries. Only the
        // most it holds is the type's, as Rust's `BoundedList` holds it: the
        // lower count is the encode's to refuse (`refusal_checks`), where the
        // plan's totality names it, so the two languages refuse one value.
        let payload = cbor_codec::record_payload_type(Language::Cpp, imports, list.record);
        let bound = list.max;
        return Entry {
            item,
            name: l.codec_field_id(&e.id),
            view: format!("::SCE::Forge::List<{payload}, {bound}>"),
            held: format!("::SCE::Forge::Bounded<std::vector<{payload}>, {bound}>"),
            read: format!("std::vector<{payload}>"),
            init: String::new(),
            record_fns: Some(cbor_codec::record_codec_fns(
                Language::Cpp,
                codec,
                list.record,
            )),
        };
    }
    let ty = l.type_name(leaf(e)).into_owned();
    let (view, held) = match item.value {
        CborValue::Text => {
            let bound = bound_args(e, false);
            (
                format!("::SCE::Forge::Text<{bound}>"),
                format!("::SCE::Forge::Bounded<std::string, {bound}>"),
            )
        }
        CborValue::Bytes { exact } => {
            let bound = bound_args(e, exact);
            (
                format!("::SCE::Forge::Bytes<{bound}>"),
                format!("::SCE::Forge::Bounded<std::vector<std::uint8_t>, {bound}>"),
            )
        }
        _ => (ty.clone(), ty.clone()),
    };
    let init = match leaf(e) {
        SceType::Enum(r) if item.required => l.enum_default_expr(&r.alias),
        _ => String::new(),
    };
    Entry {
        item,
        name: l.codec_field_id(&e.id),
        view,
        held,
        read: ty,
        init,
        record_fns: None,
    }
}

/// The read and the write of one record of the list `entry` holds.
fn record_fns<'e>(entry: &'e Entry<'_>) -> &'e (String, String) {
    entry
        .record_fns
        .as_ref()
        .expect("a list of records names the functions of its record")
}

/// The statement that writes `v`, the value of `entry`, after its key.
fn write_value(l: &LangCtx, entry: &Entry<'_>) -> String {
    match entry.item.value {
        CborValue::Bool => "if (auto _e = cbor::write_bool(w, v); _e) return _e;".to_string(),
        CborValue::Text => {
            "if (auto _e = cbor::write_text(w, v.inner()); _e) return _e;".to_string()
        }
        CborValue::Bytes { .. } => {
            "if (auto _e = cbor::write_bytes(w, v.inner()); _e) return _e;".to_string()
        }
        // An unsigned carrier's every value is an unsigned integer; a signed
        // one's negative value was refused before the first byte
        // (`refusal_checks`), so what is left is one.
        CborValue::Enum { .. } => format!(
            "if (auto _e = cbor::write_uint(w, static_cast<std::uint64_t>({})); _e) return _e;",
            l.codec_carrier_expr(leaf(entry.item.entry), "v")
        ),
        CborValue::Uint { .. } => {
            "if (auto _e = cbor::write_uint(w, static_cast<std::uint64_t>(v)); _e) return _e;"
                .to_string()
        }
        // A definite-length array of the records, each the map `Records`
        // writes (docs/adr/0086, section 9.2). The view holds the list within
        // its count, and what it does not hold was refused before the first
        // byte (`refusal_checks`), so nothing is checked here.
        CborValue::RecordList { .. } => format!(
            "if (auto _e = cbor::write_array_head(w, static_cast<std::uint64_t>(v.size())); _e) return _e;\n\
             \x20           for (const auto& _record : v) {{\n\
             \x20               if (auto _e = Records::{}(w, _record); _e) return _e;\n\
             \x20           }}",
            record_fns(entry).1
        ),
    }
}

/// How a statement about the value of `entry` opens, and what the value is
/// read as inside it: a required entry in a block of its own, and one the map
/// may lack only where the value holds it.
fn present(entry: &Entry<'_>) -> (String, String) {
    if entry.required() {
        ("        {\n".to_string(), format!("_e_{}", entry.name))
    } else {
        (
            format!("        if (_e_{}) {{\n", entry.name),
            format!("*_e_{}", entry.name),
        )
    }
}

/// The statements that refuse the value `v` of `entry` where the encoding
/// cannot carry it, or `None` where its type holds all the encoding asks. The
/// encode runs them before the first byte is written, so a refusal leaves the
/// sink untouched (docs/adr/0065 §2). Each refusal is one the plan's totality
/// names at the entry, and the one the Rust printer writes there
/// (`rust_source::cbor`): `CodecError::CborOutOfRange` for a negative value of
/// an enum whose carrier is signed, which no unsigned integer carries; for a
/// list of fewer records than its lower count, which the view of the list
/// does not hold; and for a record whose byte string is past its bound, which
/// the schema's unit holds in a `std::vector` with no bound.
fn refusal_checks(
    l: &LangCtx,
    imports: &[ImportContext],
    plan: &CborPlan<'_>,
    entry: &Entry<'_>,
) -> Option<String> {
    let e = entry.item.entry;
    let refuse = || {
        format!(
            "return ::SCE::Forge::{};",
            plan.totality.require(RefusalKind::CborOutOfRange, &e.id)
        )
    };
    let mut checks: Vec<String> = Vec::new();
    match entry.item.value {
        CborValue::Enum {
            carrier: Carrier::Signed,
        } => checks.push(format!(
            "if ({} < 0) {}",
            l.codec_carrier_expr(leaf(e), "v"),
            refuse()
        )),
        CborValue::RecordList { min } => {
            if min > 0 {
                checks.push(format!("if (v.size() < {min}) {}", refuse()));
            }
            let alias = e
                .record_list()
                .map(|list| list.record)
                .expect("a list entry holds a list of records");
            let unbounded = crate::forge::generator::import_type_scope(imports)
                .records
                .get(alias)
                .map(|record| cbor_codec::record_unbounded_bytes(&record.fields))
                .unwrap_or_default();
            if !unbounded.is_empty() {
                let refusal = refuse();
                let mut each = String::new();
                for (id, max) in &unbounded {
                    let field =
                        crate::forge::generator::event_schema_field_ident(id, Language::Cpp);
                    each.push_str(&format!(
                        "                if (_record.{field}.size() > {max}) {refusal}\n"
                    ));
                }
                checks.push(format!(
                    "for (const auto& _record : v) {{\n{each}            }}"
                ));
            }
        }
        _ => {}
    }
    if checks.is_empty() {
        return None;
    }
    let (open, value) = present(entry);
    let body: String = checks
        .iter()
        .map(|check| format!("            {check}\n"))
        .collect();
    Some(format!(
        "{open}\
         \x20           const auto& v = {value};\n\
         {body}\
         \x20       }}\n"
    ))
}

/// The statements that read the value of `entry` into its local, which the
/// caller has checked is still empty; a refusal returns `std::nullopt` from
/// the decode, this backend's decode convention.
fn read_value(l: &LangCtx, imports: &[ImportContext], entry: &Entry<'_>) -> String {
    let e = entry.item.entry;
    let limit =
        |max: Option<u32>| max.map_or_else(|| "std::nullopt".to_string(), |m| m.to_string());
    let read = match entry.item.value {
        CborValue::Bool => "auto v = cbor::read_bool(c);".to_string(),
        CborValue::Text => format!("auto v = cbor::read_text(c, {});", limit(e.max_size)),
        CborValue::Bytes { exact: true } => format!(
            "auto v = cbor::read_bytes_exact(c, {});",
            e.length.expect("an exact byte string states its length")
        ),
        CborValue::Bytes { exact: false } => {
            format!("auto v = cbor::read_bytes(c, {});", limit(e.max_size))
        }
        CborValue::Enum { .. } => {
            let SceType::Enum(r) = leaf(e) else {
                unreachable!("an enum entry is of an enum type")
            };
            let carrier = crate::forge::cbor_codec::enum_carrier(imports, &r.alias)
                .expect("the import resolved the enum's carrier");
            let carrier = l.type_name(&carrier);
            let made = format!(
                "{}(static_cast<{carrier}>(*raw))",
                l.enum_from_underlying(&r.alias)
            );
            let v = if l.enum_is_open(&r.alias) {
                format!("std::optional<{}> v = {made};", entry.read)
            } else {
                format!("auto v = {made};")
            };
            format!(
                "const auto raw = cbor::read_uint_upto(c, static_cast<std::uint64_t>(std::numeric_limits<{carrier}>::max()));\n\
                 \x20               if (!raw) return std::nullopt;\n\
                 \x20               {v}"
            )
        }
        CborValue::Uint { wide: true } => "auto v = cbor::read_uint(c);".to_string(),
        CborValue::Uint { wide: false } => format!(
            "const auto raw = cbor::read_uint_upto(c, static_cast<std::uint64_t>(std::numeric_limits<{ty}>::max()));\n\
             \x20               if (!raw) return std::nullopt;\n\
             \x20               std::optional<{ty}> v = static_cast<{ty}>(*raw);",
            ty = entry.read
        ),
        // A definite-length array within the list's count, refused before any
        // record is read where it is past it, each item one record's map.
        CborValue::RecordList { min } => {
            let list = e
                .record_list()
                .expect("a list of records entry holds a list of records");
            let short = if min > 0 {
                format!(" || *_len < {min}")
            } else {
                String::new()
            };
            format!(
                "const auto _len = cbor::read_array_len(c);\n\
                 \x20               if (!_len || *_len > {max}{short}) return std::nullopt;\n\
                 \x20               std::optional<{read}> v{{std::in_place}};\n\
                 \x20               for (std::uint64_t _n = 0; _n < *_len; ++_n) {{\n\
                 \x20                   auto _record = Records::{decode}(c);\n\
                 \x20                   if (!_record) return std::nullopt;\n\
                 \x20                   v->push_back(std::move(*_record));\n\
                 \x20               }}",
                max = list.max,
                read = entry.read,
                decode = record_fns(entry).0
            )
        }
    };
    format!(
        "                {read}\n\
         \x20               if (!v) return std::nullopt;\n\
         \x20               {name} = std::move(*v);\n",
        name = entry.name
    )
}

/// The text of the printed CBOR codec `plan` inside its namespace: the
/// readers of its entries, the trait a source meets, the encode over any
/// source, and the value, made by its constructor and read by its accessors.
pub(in super::super) fn render(
    plan: &CborPlan<'_>,
    l: &LangCtx,
    imports: &[ImportContext],
    struct_name: &str,
) -> String {
    let entries: Vec<Entry<'_>> = plan
        .entries
        .iter()
        .map(|item| entry(l, imports, &plan.model.name, item))
        .collect();
    let names: Vec<&str> = entries.iter().map(|e| e.name.as_str()).collect();
    let mut out = String::new();

    out.push_str(&format!(
        "/// The entries a value of `{struct_name}` presents, each read by name: the\n\
         /// member function where the type has one, otherwise a function the type's own\n\
         /// namespace declares (docs/adr/0081, step 4).\n\
         namespace source {{\n{}}}  // namespace source\n\n",
        source_readers(&names)
    ));

    let read = |e: &Entry<'_>| format!("source::{}(std::declval<const S&>())", e.name);
    let reads: Vec<String> = entries
        .iter()
        .map(|e| format!("decltype({})", read(e)))
        .collect();
    let converts: Vec<String> = entries
        .iter()
        .map(|e| {
            format!(
                "std::is_convertible<decltype({}), {}>",
                read(e),
                e.presented()
            )
        })
        .collect();
    let nothrow: Vec<String> = entries
        .iter()
        .map(|e| {
            format!(
                "noexcept(::SCE::Forge::Detail::converts_to<{}>({}))",
                e.presented(),
                read(e)
            )
        })
        .collect();
    out.push_str(&format!(
        "/// Whether `S` presents every entry of `{struct_name}`, each convertible to\n\
         /// its type (a C++17 trait, ADR 0059 §3).\n\
         template <class S, class = void>\n\
         struct is_source : std::false_type {{}};\n\
         template <class S>\n\
         struct is_source<S, std::void_t<{}>>\n\
         \x20   : std::conjunction<{}> {{}};\n\n\
         /// Whether reading every entry of a source `S` throws nothing.\n\
         template <class S, class = void>\n\
         struct is_nothrow_source : std::false_type {{}};\n\
         template <class S>\n\
         struct is_nothrow_source<S, std::enable_if_t<is_source<S>::value>>\n\
         \x20   : std::bool_constant<{}> {{}};\n\n",
        reads.join(", "),
        converts.join(", "),
        nothrow.join(" && ")
    ));

    // The encode: each entry read once and taken as what it is presented as,
    // the map head counting the ones present, then each in ascending key
    // order.
    let mut encode = String::new();
    for e in &entries {
        encode.push_str(&format!(
            "        const auto& _r_{name} = source::{name}(src);\n\
             \x20       const {ty} _e_{name} = _r_{name};\n",
            name = e.name,
            ty = e.presented()
        ));
    }
    // What the encoding cannot carry is refused before the first byte, each
    // entry's in the order the encode would meet it: ascending key, as the
    // Rust encode's count over a sink that only counts meets it.
    for &index in &plan.by_key {
        if let Some(checks) = refusal_checks(l, imports, plan, &entries[index]) {
            encode.push_str(&checks);
        }
    }
    let required = plan.required();
    let optional: Vec<&Entry<'_>> = entries.iter().filter(|e| !e.required()).collect();
    if optional.is_empty() {
        encode.push_str(&format!(
            "        const std::uint64_t _count = {required};\n"
        ));
    } else {
        encode.push_str(&format!("        std::uint64_t _count = {required};\n"));
        for e in &optional {
            encode.push_str(&format!("        if (_e_{}) ++_count;\n", e.name));
        }
    }
    encode.push_str("        if (auto _e = cbor::write_map_head(w, _count); _e) return _e;\n");
    for &index in &plan.by_key {
        let e = &entries[index];
        let (open, value) = present(e);
        encode.push_str(&format!(
            "{open}\
             \x20           const auto& v = {value};\n\
             \x20           if (auto _e = cbor::write_uint(w, {key}); _e) return _e;\n\
             \x20           {write}\n\
             \x20       }}\n",
            key = e.item.entry.key,
            write = write_value(l, e)
        ));
    }
    // The records the lists hold, read and written before the encode and the
    // value that read and write them.
    let held_records = records(plan, l, imports);
    if !held_records.is_empty() {
        out.push_str(&records_struct(&held_records, struct_name));
    }
    let total = plan.totality.is_total();
    let refuses = if total {
        "It refuses nothing; what it returns is\n\
         /// the sink's own failure, and it throws only what a read of `src` throws."
    } else {
        "A value the\n\
         /// encoding cannot carry is refused with `CodecError::CborOutOfRange` before\n\
         /// the first byte is written, so the sink is left untouched; otherwise what\n\
         /// it returns is the sink's own failure, and it throws only what a read of\n\
         /// `src` throws."
    };
    out.push_str(&format!(
        "/// The encode of `{struct_name}` over any source of it: the map head counting\n\
         /// the entries present, then each in ascending key order, every head in its\n\
         /// shortest form (RFC 8949 §4.2.1). {refuses}\n\
         struct Wire {{\n\
         \x20   template <class S>\n\
         \x20   [[nodiscard]] static std::optional<::SCE::Forge::CodecError> encode_source(const S& src, ::SCE::Forge::SceSink& w) noexcept(is_nothrow_source<S>::value) {{\n\
         \x20       static_assert(is_source<S>::value, \"S presents every entry of {struct_name}: {}\");\n\
         \x20       namespace cbor = ::SCE::Forge::Cbor;\n\
         {encode}\
         \x20       return std::nullopt;\n\
         \x20   }}\n\
         }};\n\n",
        names.join(", ")
    ));

    out.push_str(&value_class(l, imports, &entries, struct_name, total));
    // Every refusal the totality analysis names is one the encode writes
    // (docs/adr/0065 §2).
    plan.totality.assert_all_written(struct_name);
    out
}

/// The value: made by its constructor, each entry read by its accessor,
/// decoded from a map and encoded as any source is. `total` is whether the
/// encode refuses no value, which the vector its bytes are returned in says.
fn value_class(
    l: &LangCtx,
    imports: &[ImportContext],
    entries: &[Entry<'_>],
    struct_name: &str,
    total: bool,
) -> String {
    let params: Vec<String> = entries
        .iter()
        .map(|e| format!("{} {}", e.presented(), e.name))
        .collect();
    let inits: Vec<String> = entries
        .iter()
        .map(|e| format!("{}_({})", e.name, e.name))
        .collect();
    let owns = entries.iter().any(Entry::bounded);
    let (constexpr, noexcept) = if owns {
        ("", "")
    } else {
        ("constexpr ", " noexcept")
    };
    let mut out = format!(
        "/// One CBOR map (SCE_FORGE.md §4.6.1). Decode takes the keys in any order and\n\
         /// skips one this codec does not declare; encode writes the entries present in\n\
         /// ascending key order, every head in its shortest form. A value made by\n\
         /// default holds each required entry at its type's default and each optional\n\
         /// one absent.\n\
         struct {struct_name} {{\n\
         \x20   {struct_name}() = default;\n\
         \x20   /// The value of each entry it presents, in declaration order (ADR 0059 §5).\n\
         \x20   {constexpr}{struct_name}({}){noexcept}\n\
         \x20       : {} {{}}\n",
        params.join(", "),
        inits.join(", ")
    );
    for e in entries {
        let key = e.item.entry.key;
        let body = if e.bounded() && !e.required() {
            format!(
                "if (!{name}_) {{ return std::nullopt; }} return {view}(*{name}_);",
                name = e.name,
                view = e.view
            )
        } else {
            format!("return {}_;", e.name)
        };
        let constexpr = if e.bounded() { "" } else { "constexpr " };
        let doc = if e.required() {
            format!("The entry with key {key}.")
        } else {
            format!("The entry with key {key}, where the map holds it.")
        };
        out.push_str(&format!(
            "    /// {doc}\n\
             \x20   [[nodiscard]] {constexpr}{ty} {name}() const noexcept {{ {body} }}\n",
            ty = e.presented(),
            name = e.name
        ));
    }

    // The decode: each entry read once in whatever order the map holds it,
    // a key given twice refused, an unknown key skipped; a required entry
    // missing refused; a text or a byte string then held to its bound, which
    // the read already held it within.
    let mut decode = String::new();
    for e in entries {
        decode.push_str(&format!("        std::optional<{}> {};\n", e.read, e.name));
    }
    decode.push_str(
        "        for (std::uint64_t i = 0; i < *count; ++i) {\n\
         \x20           const auto key = cbor::read_uint(c);\n\
         \x20           if (!key) return std::nullopt;\n\
         \x20           switch (*key) {\n",
    );
    let mut by_key: Vec<&Entry<'_>> = entries.iter().collect();
    by_key.sort_by_key(|e| e.item.entry.key);
    for e in &by_key {
        decode.push_str(&format!(
            "            case {key}: {{\n\
             \x20               // A key given twice is not one map entry.\n\
             \x20               if ({name}.has_value()) return std::nullopt;\n\
             {read}\
             \x20               break;\n\
             \x20           }}\n",
            key = e.item.entry.key,
            name = e.name,
            read = read_value(l, imports, e)
        ));
    }
    decode.push_str(
        "            default:\n\
         \x20               if (!cbor::skip(c)) return std::nullopt;\n\
         \x20               break;\n\
         \x20           }\n\
         \x20       }\n",
    );
    for e in entries.iter().filter(|e| e.required()) {
        decode.push_str(&format!(
            "        if (!{}.has_value()) return std::nullopt;\n",
            e.name
        ));
    }
    let mut args: Vec<String> = Vec::new();
    for e in entries {
        if !e.bounded() {
            args.push(if e.required() {
                format!("/*{name}=*/*{name}", name = e.name)
            } else {
                format!("/*{name}=*/{name}", name = e.name)
            });
            continue;
        }
        if e.required() {
            decode.push_str(&format!(
                "        const auto _b_{name} = {view}::make(*{name});\n\
                 \x20       if (!_b_{name}) return std::nullopt;\n",
                name = e.name,
                view = e.view
            ));
            args.push(format!("/*{name}=*/*_b_{name}", name = e.name));
        } else {
            decode.push_str(&format!(
                "        std::optional<{view}> _b_{name};\n\
                 \x20       if ({name}) {{\n\
                 \x20           _b_{name} = {view}::make(*{name});\n\
                 \x20           if (!_b_{name}) return std::nullopt;\n\
                 \x20       }}\n",
                name = e.name,
                view = e.view
            ));
            args.push(format!("/*{name}=*/_b_{name}", name = e.name));
        }
    }
    // The vector of the bytes is an optional only where the encode can refuse
    // the value, as the positional and content-line values answer theirs.
    let (refuses, encoded, refused, to_vec) = if total {
        (
            "It refuses nothing; what it returns is the sink's own failure.",
            "std::vector<std::uint8_t>",
            "",
            "\x20       (void)encode(sink);\n\
             \x20       return out;\n",
        )
    } else {
        (
            "A value the encoding cannot carry is refused before the\n\
             \x20   /// first byte is written; otherwise what it returns is the sink's own failure.",
            "std::optional<std::vector<std::uint8_t>>",
            "; `std::nullopt` when encode refuses the value",
            "\x20       if (encode(sink)) {\n\
             \x20           return std::nullopt;\n\
             \x20       }\n\
             \x20       return out;\n",
        )
    };
    out.push_str(&format!(
        "\n\
         \x20   /// Decode one map from `cursor`. On success the cursor advances past it;\n\
         \x20   /// `std::nullopt` on any refusal, with the cursor left where it was.\n\
         \x20   static std::optional<{struct_name}> decode(::SCE::Forge::SceCursor& cursor) {{\n\
         \x20       ::SCE::Forge::SceCursor c = cursor;\n\
         \x20       auto value = decode_at(c);\n\
         \x20       if (value) {{\n\
         \x20           cursor = c;\n\
         \x20       }}\n\
         \x20       return value;\n\
         \x20   }}\n\n\
         \x20   /// Encode this map into `w`: the encode over any source, this value one of\n\
         \x20   /// them. {refuses}\n\
         \x20   [[nodiscard]] std::optional<::SCE::Forge::CodecError> encode(::SCE::Forge::SceSink& w) const noexcept {{\n\
         \x20       return Wire::encode_source(*this, w);\n\
         \x20   }}\n\n\
         \x20   /// `encode` into a new vector{refused}.\n\
         \x20   [[nodiscard]] {encoded} encode_to_vec() const {{\n\
         \x20       std::vector<std::uint8_t> out;\n\
         \x20       ::SCE::Forge::VectorSink sink(out);\n\
         {to_vec}\
         \x20   }}\n\n\
         private:\n\
         \x20   static std::optional<{struct_name}> decode_at(::SCE::Forge::SceCursor& c) {{\n\
         \x20       namespace cbor = ::SCE::Forge::Cbor;\n\
         \x20       const auto count = cbor::read_map_len(c);\n\
         \x20       if (!count) return std::nullopt;\n\
         {decode}\
         \x20       return {struct_name}{{\n\
         {}\
         \x20       }};\n\
         \x20   }}\n\n",
        args.iter()
            .map(|a| format!("            {a},\n"))
            .collect::<String>()
    ));
    for e in entries {
        let init = if e.init.is_empty() || !e.required() {
            "{}".to_string()
        } else {
            format!("{{{}}}", e.init)
        };
        out.push_str(&format!("    {} {}_{init};\n", e.held_member(), e.name));
    }
    out.push_str("};\n");
    out
}
