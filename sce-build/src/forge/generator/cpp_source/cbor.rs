// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! The C++ printer of a CBOR codec's plan (docs/adr/0081 step 4, docs/adr/0101
//! Landing order step 4b, SCE_FORGE.md §4.6.1).
//!
//! A CBOR map derives nothing a value says: the map head counts the entries
//! present, which is the encoding's and no element. So every entry is
//! presented, as the C++ interface presents a positional codec's elements: one
//! accessor for each, read through the `source` customization points and
//! checked by `is_source`. A number, a `bool` and an enum are presented as
//! their type, and a text or a byte string within its bound (`Text`, `Bytes`;
//! docs/adr/0077 §3), each by value: the value itself for a required entry and
//! a `std::optional` for one that may be absent. A list, a record and a union
//! are lent by `const` reference, in the type a schema unit holds them in
//! (docs/adr/0101, section 5.3): the runtime's `OwnedList` of the list's bound,
//! the payload type of the record's event schema and the type of the union's
//! unit, inside a `std::optional` for an entry that may be absent.
//!
//! The encoding rule is the one the Rust printer writes (`rust_source::cbor`):
//! the map head, then each entry present in ascending key order, every head in
//! its shortest form (RFC 8949 §4.2.1), each value in its one form
//! (docs/adr/0101, section 2.1). A value below an entry's own level is read and
//! written by a walk of the plan's shape, with one read, one write and, where a
//! value of it can be refused, one check for each record and union document
//! the codec reaches, named for that document and shared wherever it is met
//! (section 5.1). What no type holds is refused where the plan's totality names
//! it, as the Rust printer refuses it: a list of fewer elements than its lower
//! count at any depth, and a record's byte string, which the schema's unit
//! holds in a `std::vector` with no bound, past its bound. Each is found before
//! the first byte is written, so a refusal leaves the sink untouched.

use super::super::*;
use super::source_readers;
use crate::forge::cbor_codec::{self, CborComposite};
use crate::forge::codec_totality::RefusalKind;
use crate::forge::composed_lowering::{
    cpp_bounded_list, cpp_bytes_type, cpp_case_ident, cpp_text_type, CPP_CASE_VALUE,
    CPP_UNION_CASES, CPP_UNREACHED,
};
use crate::forge::encode_plan::{CborEnum, CborItem, CborPlan, CborRecord, CborShape, CborUnion};
use crate::forge::model::CborEntry;
use crate::generator::Language;

/// Every form of the CBOR codec `plan` the C++ printer does not print, each a
/// clause the codec's refusal names it by. The printer prints every shape of
/// docs/adr/0101 section 5.1 ([`cbor_codec::lowers_every_entry_type`]), so a
/// codec whose types resolve is printed whole; the clauses of the shapes
/// docs/adr/0085 Landing order step 3a lowers alone
/// ([`cbor_codec::unlowered_forms`]) are named where that arm says otherwise.
/// The caller asks every plan alike, as it asks a positional one.
pub(in super::super) fn unlowered_forms(
    plan: &CborPlan<'_>,
    _imports: &[ImportContext],
) -> Vec<String> {
    if cbor_codec::lowers_every_entry_type(Language::Cpp) {
        return Vec::new();
    }
    cbor_codec::unlowered_forms(plan)
}

// ── Values below an entry's own level ───────────────────────────────────

/// Where a value stands in the type a schema unit declares, which decides the
/// type C++ holds a byte string in (docs/adr/0086, section 7.1): a record's
/// own field, bare or optional, is held as the unit holds one entry, a byte
/// string in the language's own vector, which carries no bound; every other
/// value, inside a list or held whole by an entry, in the bounded types of a
/// composition.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Held {
    Field,
    Composed,
}

/// The name a walk binds at `depth`: `base` followed by the depth, so a
/// nested walk names its own and hides none of its holder's.
fn bound_name(base: &str, depth: usize) -> String {
    format!("{base}{depth}")
}

/// `blocks`, each line of each indented by `columns` spaces and ended by a
/// newline; an empty line stays empty.
fn indented(blocks: &[String], columns: usize) -> String {
    let pad = " ".repeat(columns);
    let mut out = String::new();
    for line in blocks.iter().flat_map(|block| block.lines()) {
        if !line.is_empty() {
            out.push_str(&pad);
            out.push_str(line);
        }
        out.push('\n');
    }
    out
}

/// A C++ string literal of `text`: a key of a record's map or a union's case.
fn literal(text: &str) -> String {
    format!("\"{}\"", filters::escape_cpp(text.to_string()))
}

/// A part of a type the plan could not say, which the compile refused before
/// any printer reads the plan.
fn unresolved(what: &str) -> ! {
    unreachable!(
        "`{what}` of a CBOR entry did not resolve, which the compile refuses before any \
         printer reads the plan (`cbor_codec::unheld_value`)"
    )
}

/// The integer type of `bits` bits, of either sign where `signed`.
fn int_type(bits: u32, signed: bool) -> SceType {
    match (bits, signed) {
        (8, false) => SceType::Uint8,
        (16, false) => SceType::Uint16,
        (32, false) => SceType::Uint32,
        (_, false) => SceType::Uint64,
        (8, true) => SceType::Int8,
        (16, true) => SceType::Int16,
        (32, true) => SceType::Int32,
        (_, true) => SceType::Int64,
    }
}

/// The C++ type a value of `shape` is held in where it stands (`held`): the
/// type a schema unit holds it in (docs/adr/0086, section 7.1; docs/adr/0101,
/// section 5.3), the payload type of a record's event schema, the type of a
/// union's unit, the runtime's `OwnedList` of a list's bound, its bounded text
/// and byte string, and `std::optional` around a list's element that may be
/// absent.
fn held_type(shape: &CborShape, held: Held) -> String {
    match shape {
        CborShape::Int { bits, signed } => cpp_type(&int_type(*bits, *signed)).to_string(),
        CborShape::Float { bits: 32 } => cpp_type(&SceType::Float32).to_string(),
        CborShape::Float { .. } => cpp_type(&SceType::Float64).to_string(),
        CborShape::Bool => cpp_type(&SceType::Bool).to_string(),
        CborShape::Text { max } => cpp_text_type(*max),
        CborShape::Bytes { .. } if held == Held::Field => cpp_type(&SceType::Bytes).to_string(),
        CborShape::Bytes { max, .. } => cpp_bytes_type(*max),
        CborShape::Enum(enumeration) => enum_qualified_type(&enumeration.name, Language::Cpp),
        CborShape::Record(record) => event_schema_payload_type(&record.name, Language::Cpp),
        CborShape::Union(union) => union_unit_type(&union.name, Language::Cpp),
        CborShape::List { elem, max, .. } => {
            cpp_bounded_list(&held_type(elem, Held::Composed), *max)
        }
        CborShape::Nullable(elem) => {
            format!("std::optional<{}>", held_type(elem, Held::Composed))
        }
        CborShape::Unresolved { what } => unresolved(what),
    }
}

/// The codec's read, write and check of a value of the record or union
/// document `name` (`what` is `record` or `union`), as `[decode, encode,
/// check]`: static functions of its `Composites`, named for the document, so a
/// document met at two places, under two aliases, is read by one function.
fn composite_fns(what: &str, name: &str) -> [String; 3] {
    let document = filters::to_snake_case(name.to_string());
    [
        format!("Composites::decode_{what}_{document}"),
        format!("Composites::encode_{what}_{document}"),
        format!("Composites::check_{what}_{document}"),
    ]
}

/// The bare name of the function `qualified` names, as its declaration in
/// `Composites` spells it.
fn declared(qualified: &str) -> &str {
    qualified.trim_start_matches("Composites::")
}

/// The statements that read a value of `shape` standing at `held` from the
/// cursor `c` into the place `into`, which holds the value's type or an
/// optional of it: a read answers its `Decoded`, whose refusal is returned as
/// it is (docs/adr/0099, amendment of 2026-11-16). A number past its type's
/// range, a text or byte string past its bound and a list's count outside its
/// bounds are `CborOutOfRange`, the count judged before any element is read
/// (docs/adr/0101, section 3.3); a number a closed enum does not declare is
/// `UndeclaredEnumValue`; `null` stands only for a list's element that may be
/// absent. Every name it binds is suffixed by `depth`.
fn read_stmts(shape: &CborShape, into: &str, held: Held, depth: usize) -> Vec<String> {
    let read = bound_name("_r", depth);
    // A read whose `Decoded` holds the value as the place holds it.
    let whole = |call: String| {
        vec![format!(
            "auto {read} = {call};\n\
             if (!{read}) return {read}.error();\n\
             {into} = std::move(*{read});"
        )]
    };
    // A read of a text or a byte string within its bound, then held in the
    // bounded type of `ty`, whose `make` admits what the read admitted.
    let bounded = |call: String, ty: String| {
        let made = bound_name("_b", depth);
        vec![format!(
            "auto {read} = {call};\n\
             if (!{read}) return {read}.error();\n\
             auto {made} = {ty}::make(std::move(*{read}));\n\
             if (!{made}) return ::SCE::Forge::CodecError::CborOutOfRange;\n\
             {into} = std::move(*{made});"
        )]
    };
    match shape {
        CborShape::Int {
            bits: 64,
            signed: false,
        } => whole("cbor::read_uint(c)".to_string()),
        CborShape::Int { signed: false, .. } => {
            let ty = held_type(shape, held);
            vec![format!(
                "const auto {read} = cbor::read_uint_upto(c, static_cast<std::uint64_t>(std::numeric_limits<{ty}>::max()));\n\
                 if (!{read}) return {read}.error();\n\
                 {into} = static_cast<{ty}>(*{read});"
            )]
        }
        CborShape::Int { signed: true, .. } => {
            let ty = held_type(shape, held);
            vec![format!(
                "const auto {read} = cbor::read_int_within(c, static_cast<std::int64_t>(std::numeric_limits<{ty}>::min()), static_cast<std::int64_t>(std::numeric_limits<{ty}>::max()));\n\
                 if (!{read}) return {read}.error();\n\
                 {into} = static_cast<{ty}>(*{read});"
            )]
        }
        CborShape::Float { bits: 32 } => whole("cbor::read_float32(c)".to_string()),
        CborShape::Float { .. } => whole("cbor::read_float(c)".to_string()),
        CborShape::Bool => whole("cbor::read_bool(c)".to_string()),
        CborShape::Text { max } => {
            bounded(format!("cbor::read_text(c, {max})"), held_type(shape, held))
        }
        CborShape::Bytes { max, .. } if held == Held::Field => {
            whole(format!("cbor::read_bytes(c, {max})"))
        }
        CborShape::Bytes { max, .. } => bounded(
            format!("cbor::read_bytes(c, {max})"),
            held_type(shape, held),
        ),
        CborShape::Enum(enumeration) => enum_read(enumeration, into, &read),
        CborShape::Record(record) => {
            let [decode, _, _] = composite_fns("record", &record.name);
            whole(format!("{decode}(c)"))
        }
        CborShape::Union(union) => {
            let [decode, _, _] = composite_fns("union", &union.name);
            whole(format!("{decode}(c)"))
        }
        // A definite-length array, its count judged against the list's bound
        // and lower count before any element is read (docs/adr/0101, section
        // 3.3), each element read into a place of its own and appended.
        CborShape::List { elem, max, min } => {
            let (len, list, n, item) = (
                bound_name("_len", depth),
                bound_name("_list", depth),
                bound_name("_n", depth),
                bound_name("_item", depth),
            );
            let short = if *min > 0 {
                format!(" || *{len} < {min}")
            } else {
                String::new()
            };
            let element = read_stmts(elem, &item, Held::Composed, depth + 1);
            vec![format!(
                "const auto {len} = cbor::read_array_len(c);\n\
                 if (!{len}) return {len}.error();\n\
                 if (*{len} > {max}{short}) return ::SCE::Forge::CodecError::CborOutOfRange;\n\
                 {list_ty} {list};\n\
                 for (std::uint64_t {n} = 0; {n} < *{len}; ++{n}) {{\n\
                 \x20   {elem_ty} {item}{{}};\n\
                 {element}\
                 \x20   if (!{list}.push(std::move({item}))) return ::SCE::Forge::CodecError::CborOutOfRange;\n\
                 }}\n\
                 {into} = std::move({list});",
                list_ty = held_type(shape, Held::Composed),
                elem_ty = held_type(elem, Held::Composed),
                element = indented(&element, 4),
            )]
        }
        // `null` is an absent element; anything else is read as the element,
        // which refuses what is no value of it.
        CborShape::Nullable(elem) => {
            let (null, present) = (bound_name("_null", depth), bound_name("_present", depth));
            let element = read_stmts(elem, &present, Held::Composed, depth + 1);
            vec![format!(
                "const auto {null} = cbor::take_null(c);\n\
                 if (!{null}) return {null}.error();\n\
                 if (*{null}) {{\n\
                 \x20   {into} = std::nullopt;\n\
                 }} else {{\n\
                 \x20   {elem_ty} {present}{{}};\n\
                 {element}\
                 \x20   {into} = std::move({present});\n\
                 }}",
                elem_ty = held_type(elem, Held::Composed),
                element = indented(&element, 4),
            )]
        }
        CborShape::Unresolved { what } => unresolved(what),
    }
}

/// The statements that read a value of `enumeration` into `into`: the integer
/// of its carrier within the carrier's range, major type 1 below zero for a
/// signed one (docs/adr/0101, section 2.1), and the variant it declares, a
/// number a closed enum does not declare refused. `read` is the name the
/// integer is bound to.
fn enum_read(enumeration: &CborEnum, into: &str, read: &str) -> Vec<String> {
    let carrier = cpp_type(&enumeration.carrier);
    let (_, from) = enum_number_symbols(&enumeration.name, !enumeration.closed, Language::Cpp);
    let number = if enumeration.carrier.is_signed() {
        format!(
            "cbor::read_int_within(c, static_cast<std::int64_t>(std::numeric_limits<{carrier}>::min()), static_cast<std::int64_t>(std::numeric_limits<{carrier}>::max()))"
        )
    } else {
        format!(
            "cbor::read_uint_upto(c, static_cast<std::uint64_t>(std::numeric_limits<{carrier}>::max()))"
        )
    };
    let made = format!("{from}(static_cast<{carrier}>(*{read}))");
    let held = if enumeration.closed {
        let variant = format!("{read}_variant");
        format!(
            "const auto {variant} = {made};\n\
             if (!{variant}) return ::SCE::Forge::CodecError::UndeclaredEnumValue;\n\
             {into} = *{variant};"
        )
    } else {
        format!("{into} = {made};")
    };
    vec![format!(
        "const auto {read} = {number};\n\
         if (!{read}) return {read}.error();\n\
         {held}"
    )]
}

/// The statement that writes `call`, a write that answers the sink's refusal,
/// returning that refusal.
fn written(call: String) -> String {
    format!("if (auto _e = {call}; _e) return _e;")
}

/// The statements that write the value of `shape` standing at `held` that the
/// expression `value` names, in its one form (docs/adr/0101, section 2.1), into
/// the sink `w`. They refuse nothing but the sink's own failure: what a value
/// can break is checked before ([`check_stmts`]). Every name they bind is
/// suffixed by `depth`.
fn write_stmts(shape: &CborShape, value: &str, held: Held, depth: usize) -> Vec<String> {
    let line = match shape {
        CborShape::Int { signed: false, .. } => written(format!(
            "cbor::write_uint(w, static_cast<std::uint64_t>({value}))"
        )),
        CborShape::Int { signed: true, .. } => written(format!(
            "cbor::write_int(w, static_cast<std::int64_t>({value}))"
        )),
        CborShape::Float { .. } => written(format!(
            "cbor::write_float(w, static_cast<double>({value}))"
        )),
        CborShape::Bool => written(format!("cbor::write_bool(w, {value})")),
        CborShape::Text { .. } => written(format!("cbor::write_text(w, {value}.inner())")),
        // A record's own byte string is the vector itself; any other is a
        // bounded byte string, which lends its vector.
        CborShape::Bytes { .. } if held == Held::Field => {
            written(format!("cbor::write_bytes(w, {value})"))
        }
        CborShape::Bytes { .. } => written(format!("cbor::write_bytes(w, {value}.inner())")),
        CborShape::Enum(enumeration) => {
            let (to, _) =
                enum_number_symbols(&enumeration.name, !enumeration.closed, Language::Cpp);
            if enumeration.carrier.is_signed() {
                written(format!(
                    "cbor::write_int(w, static_cast<std::int64_t>({to}({value})))"
                ))
            } else {
                written(format!(
                    "cbor::write_uint(w, static_cast<std::uint64_t>({to}({value})))"
                ))
            }
        }
        CborShape::Record(record) => {
            let [_, encode, _] = composite_fns("record", &record.name);
            written(format!("{encode}(w, {value})"))
        }
        CborShape::Union(union) => {
            let [_, encode, _] = composite_fns("union", &union.name);
            written(format!("{encode}(w, {value})"))
        }
        // A definite-length array of the elements in their order.
        CborShape::List { elem, .. } => {
            let item = bound_name("_item", depth);
            let inner = write_stmts(elem, &item, Held::Composed, depth + 1);
            format!(
                "{}\nfor (const auto& {item} : {value}) {{\n{}}}",
                written(format!(
                    "cbor::write_array_head(w, static_cast<std::uint64_t>({value}.size()))"
                )),
                indented(&inner, 4)
            )
        }
        // An absent element is `null`, the one item no other form takes.
        CborShape::Nullable(elem) => {
            let present = bound_name("_present", depth);
            let inner = write_stmts(elem, &present, Held::Composed, depth + 1);
            format!(
                "if (!{value}.has_value()) {{\n\
                 \x20   {}\n\
                 }} else {{\n\
                 \x20   const auto& {present} = *{value};\n\
                 {}}}",
                written("cbor::write_null(w)".to_string()),
                indented(&inner, 4)
            )
        }
        CborShape::Unresolved { what } => unresolved(what),
    };
    vec![line]
}

/// Whether a value of `shape` standing at `held` that its C++ type holds can
/// still have no encoding (docs/adr/0101, section 3.5): the one reading the
/// plan's totality makes of the shape
/// ([`crate::forge::codec_totality::cbor_value_refuses`]), so the checks are
/// written exactly where it names a refusal.
fn can_refuse(shape: &CborShape, held: Held) -> bool {
    crate::forge::codec_totality::cbor_value_refuses(shape, held == Held::Field)
}

/// Whether a record of `record` can have no encoding: a field of it can.
fn record_can_refuse(record: &CborRecord) -> bool {
    record
        .fields
        .iter()
        .any(|field| can_refuse(&field.shape, Held::Field))
}

/// Whether a value of `union` can have no encoding: the record of a case can.
fn union_can_refuse(union: &CborUnion) -> bool {
    union
        .cases
        .iter()
        .any(|case| case.record.as_ref().is_some_and(record_can_refuse))
}

/// The statements that refuse, with `refusal` (a `CodecError` spelled from its
/// enum's name), a value of `shape` standing at `held`, named by `value`, that
/// has no encoding ([`can_refuse`]); empty for one that cannot. A record's and
/// a union's own check answers its refusal, which is returned as it is.
fn check_stmts(
    shape: &CborShape,
    value: &str,
    held: Held,
    depth: usize,
    refusal: &str,
) -> Vec<String> {
    if !can_refuse(shape, held) {
        return Vec::new();
    }
    let refuse = format!("return ::SCE::Forge::{refusal};");
    match shape {
        CborShape::List { elem, min, .. } => {
            let mut lines = Vec::new();
            if *min > 0 {
                lines.push(format!("if ({value}.size() < {min}) {refuse}"));
            }
            if can_refuse(elem, Held::Composed) {
                let item = bound_name("_item", depth);
                let inner = check_stmts(elem, &item, Held::Composed, depth + 1, refusal);
                lines.push(format!(
                    "for (const auto& {item} : {value}) {{\n{}}}",
                    indented(&inner, 4)
                ));
            }
            lines
        }
        CborShape::Nullable(elem) => {
            let present = bound_name("_present", depth);
            let inner = check_stmts(elem, &present, held, depth + 1, refusal);
            vec![format!(
                "if ({value}.has_value()) {{\n\
                 \x20   const auto& {present} = *{value};\n\
                 {}}}",
                indented(&inner, 4)
            )]
        }
        CborShape::Bytes { max, .. } => vec![format!("if ({value}.size() > {max}) {refuse}")],
        CborShape::Record(record) => {
            let [_, _, check] = composite_fns("record", &record.name);
            vec![written(format!("{check}({value})"))]
        }
        CborShape::Union(union) => {
            let [_, _, check] = composite_fns("union", &union.name);
            vec![written(format!("{check}({value})"))]
        }
        CborShape::Int { .. }
        | CborShape::Float { .. }
        | CborShape::Bool
        | CborShape::Text { .. }
        | CborShape::Enum(_)
        | CborShape::Unresolved { .. } => Vec::new(),
    }
}

/// The `Composites` struct of a printed map: the read, the write and, where a
/// value of it can be refused, the check of each record and union document the
/// entries of `plan` reach (docs/adr/0101, section 5.1), each a static
/// function, so one reaches another whatever their order; empty for a map that
/// reaches none.
fn composites_struct(plan: &CborPlan<'_>, struct_name: &str) -> String {
    let reached = cbor_codec::reached_composites(plan);
    if reached.is_empty() {
        return String::new();
    }
    let functions: Vec<String> = reached
        .iter()
        .map(|composite| match composite {
            CborComposite::Record(record) => record_fns(record),
            CborComposite::Union(union) => union_fns(union),
        })
        .collect();
    format!(
        "/// The records and unions the entries of `{struct_name}` reach, at any depth: each\n\
         /// read, written and, where a value of it can be refused, checked by one function\n\
         /// named for its document and shared wherever it is met (docs/adr/0101, section 5.1).\n\
         struct Composites {{\n{}}};\n\n",
        functions.join("\n")
    )
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
        member: event_schema_field_ident(&field.id, Language::Cpp),
        key: literal(&field.id),
        ty: held_type(&field.shape, Held::Field),
    }
}

/// The read, the write and the check of the record `record`, as members of
/// `Composites`.
fn record_fns(record: &CborRecord) -> String {
    let [decode, encode, check] = composite_fns("record", &record.name);
    let (decode, encode, check) = (declared(&decode), declared(&encode), declared(&check));
    let payload = event_schema_payload_type(&record.name, Language::Cpp);
    let schema = &record.name;

    // The read: each field held until the map has been read to its end, so a
    // key given twice and a field that is not optional missing are told apart
    // from the value.
    let mut held = String::new();
    let mut arms = String::new();
    let mut missing = String::new();
    let mut made = String::new();
    for (index, field) in record.fields.iter().enumerate() {
        let names = field_names(field);
        let member = &names.member;
        held.push_str(&format!(
            "        std::optional<{}> held_{member};\n",
            names.ty
        ));
        let read = read_stmts(&field.shape, &format!("held_{member}"), Held::Field, 0);
        arms.push_str(&format!(
            "            {open}if (*key == {key}) {{\n\
             \x20               // A key given twice is not one map entry.\n\
             \x20               if (held_{member}.has_value()) return ::SCE::Forge::CodecError::CborDuplicateKey;\n\
             {read}",
            open = if index == 0 { "" } else { "} else " },
            key = names.key,
            read = indented(&read, 16),
        ));
        if field.optional {
            made.push_str(&format!(
                "        record.{member} = std::move(held_{member});\n"
            ));
        } else {
            missing.push_str(&format!(
                "        if (!held_{member}.has_value()) return ::SCE::Forge::CodecError::CborRequiredKeyMissing;\n"
            ));
            made.push_str(&format!(
                "        record.{member} = std::move(*held_{member});\n"
            ));
        }
    }
    let read = format!(
        "    /// Read one record of the event schema `{schema}` from `c`, into the payload\n\
         \x20   /// type the schema's own unit declares: one CBOR map of its present fields,\n\
         \x20   /// each under its id as text (docs/adr/0101, section 2.1). Its keys in any\n\
         \x20   /// order, a key the schema does not declare skipped whole, a key given twice\n\
         \x20   /// and a field that is not optional missing refused, and a value of no\n\
         \x20   /// field's type refused.\n\
         \x20   static ::SCE::Forge::Decoded<{payload}> {decode}(::SCE::Forge::SceCursor& c) {{\n\
         \x20       namespace cbor = ::SCE::Forge::Cbor;\n\
         \x20       const auto count = cbor::read_map_len(c);\n\
         \x20       if (!count) return count.error();\n\
         {held}\
         \x20       for (std::uint64_t i = 0; i < *count; ++i) {{\n\
         \x20           const auto key = cbor::read_text(c, std::nullopt);\n\
         \x20           if (!key) return key.error();\n\
         {arms}\
         \x20           }} else {{\n\
         \x20               const auto skipped = cbor::skip(c);\n\
         \x20               if (!skipped) return skipped.error();\n\
         \x20           }}\n\
         \x20       }}\n\
         {missing}\
         \x20       {payload} record;\n\
         {made}\
         \x20       return record;\n\
         \x20   }}\n"
    );

    // The write: the map head counts the fields present, then each follows in
    // the order of its key's encoding, which the plan decided.
    let required = record.fields.iter().filter(|f| !f.optional).count();
    let mut body: Vec<String> = Vec::new();
    if required == record.fields.len() {
        body.push(written(format!("cbor::write_map_head(w, {required})")));
    } else {
        body.push(format!("std::uint64_t count = {required};"));
        for field in record.fields.iter().filter(|f| f.optional) {
            body.push(format!(
                "if (record.{}.has_value()) ++count;",
                field_names(field).member
            ));
        }
        body.push(written("cbor::write_map_head(w, count)".to_string()));
    }
    for field in record.fields_by_key() {
        let names = field_names(field);
        let key = written(format!("cbor::write_text(w, {})", names.key));
        if field.optional {
            let mut inner = vec![
                format!("const auto& value = *record.{};", names.member),
                key,
            ];
            inner.extend(write_stmts(&field.shape, "value", Held::Field, 0));
            body.push(format!(
                "if (record.{}.has_value()) {{\n{}}}",
                names.member,
                indented(&inner, 4)
            ));
        } else {
            body.push(key);
            body.extend(write_stmts(
                &field.shape,
                &format!("record.{}", names.member),
                Held::Field,
                0,
            ));
        }
    }
    body.push("return std::nullopt;".to_string());
    let write = format!(
        "\n\
         \x20   /// Write one record of `{schema}` as one map of its present fields, the keys\n\
         \x20   /// in the order RFC 8949 §4.2.1 sorts their encodings: a shorter key first.\n\
         \x20   [[nodiscard]] static std::optional<::SCE::Forge::CodecError> {encode}(::SCE::Forge::SceSink& w, const {payload}& record) noexcept {{\n\
         \x20       namespace cbor = ::SCE::Forge::Cbor;\n\
         {}\
         \x20   }}\n",
        indented(&body, 8)
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
                let mut inner = vec![format!("const auto& value = *record.{};", names.member)];
                inner.extend(check_stmts(&field.shape, "value", Held::Field, 0, refusal));
                body.push(format!(
                    "if (record.{}.has_value()) {{\n{}}}",
                    names.member,
                    indented(&inner, 4)
                ));
            } else {
                body.extend(check_stmts(
                    &field.shape,
                    &format!("record.{}", names.member),
                    Held::Field,
                    0,
                    refusal,
                ));
            }
        }
        body.push("return std::nullopt;".to_string());
        format!(
            "\n\
             \x20   /// Refuse a record of `{schema}` that its unit holds and no encoding\n\
             \x20   /// carries: a list below its lower count, a byte string past its bound.\n\
             \x20   [[nodiscard]] static std::optional<::SCE::Forge::CodecError> {check}(const {payload}& record) noexcept {{\n\
             {}\
             \x20   }}\n",
            indented(&body, 8)
        )
    } else {
        String::new()
    };
    format!("{read}{write}{check}")
}

/// The read, the write and the check of the union `union`, as members of
/// `Composites`: a map of one entry keyed by its case's variant name, whose
/// value is the case's record or the empty map (docs/adr/0101, section 2.1).
/// A case is told by the alternative of the union's `std::variant`, each its
/// own struct (docs/adr/0092, section 7, the C++ row).
fn union_fns(union: &CborUnion) -> String {
    let [decode, encode, check] = composite_fns("union", &union.name);
    let (decode, encode, check) = (declared(&decode), declared(&encode), declared(&check));
    let ty = union_unit_type(&union.name, Language::Cpp);
    let name = &union.name;
    let alternative = |variant: &str| format!("{ty}::{}", cpp_case_ident(variant));
    let names: Vec<String> = union.cases.iter().map(|c| literal(&c.variant)).collect();

    let mut arms = String::new();
    for (at, case) in union.cases.iter().enumerate() {
        let made = alternative(&case.variant);
        match &case.record {
            Some(record) => {
                let [decode_record, _, _] = composite_fns("record", &record.name);
                arms.push_str(&format!(
                    "        case {at}: {{\n\
                     \x20           auto record = {decode_record}(c);\n\
                     \x20           if (!record) return record.error();\n\
                     \x20           return {ty}{{{made}{{std::move(*record)}}}};\n\
                     \x20       }}\n"
                ));
            }
            None => arms.push_str(&format!(
                "        case {at}: {{\n\
                 \x20           const auto empty = cbor::read_empty_case(c);\n\
                 \x20           if (!empty) return empty.error();\n\
                 \x20           return {ty}{{{made}{{}}}};\n\
                 \x20       }}\n"
            )),
        }
    }
    let read = format!(
        "    /// Read one value of the union `{name}` from `c`: a map of exactly one entry,\n\
         \x20   /// judged for its count before its key is read, keyed by a case's variant\n\
         \x20   /// name, whose value is the case's record or a map of no field it declares.\n\
         \x20   static ::SCE::Forge::Decoded<{ty}> {decode}(::SCE::Forge::SceCursor& c) {{\n\
         \x20       namespace cbor = ::SCE::Forge::Cbor;\n\
         \x20       static constexpr std::string_view kCases[] = {{{}}};\n\
         \x20       const auto at = cbor::read_variant(c, kCases);\n\
         \x20       if (!at) return at.error();\n\
         \x20       switch (*at) {{\n\
         {arms}\
         \x20       default:\n\
         \x20           // `read_variant` answers the place of a case it names, so no other\n\
         \x20           // number reaches here; a decode refuses rather than stops all the same.\n\
         \x20           return ::SCE::Forge::CodecError::CborUnknownVariant;\n\
         \x20       }}\n\
         \x20   }}\n",
        names.join(", ")
    );

    let mut cases = String::new();
    for case in &union.cases {
        let held = alternative(&case.variant);
        let variant = literal(&case.variant);
        match &case.record {
            Some(record) => {
                let [_, encode_record, _] = composite_fns("record", &record.name);
                cases.push_str(&format!(
                    "        if (const auto* held = ::std::get_if<{held}>(&value.{CPP_UNION_CASES})) {{\n\
                     \x20           if (auto _e = cbor::write_variant(w, {variant}); _e) return _e;\n\
                     \x20           return {encode_record}(w, held->{CPP_CASE_VALUE});\n\
                     \x20       }}\n"
                ));
            }
            None => cases.push_str(&format!(
                "        if (::std::holds_alternative<{held}>(value.{CPP_UNION_CASES})) {{\n\
                 \x20           if (auto _e = cbor::write_variant(w, {variant}); _e) return _e;\n\
                 \x20           return cbor::write_map_head(w, 0);\n\
                 \x20       }}\n"
            )),
        }
    }
    let write = format!(
        "\n\
         \x20   /// Write one value of the union `{name}`: the map of one entry its case's\n\
         \x20   /// variant name keys, then the case's record or the empty map.\n\
         \x20   [[nodiscard]] static std::optional<::SCE::Forge::CodecError> {encode}(::SCE::Forge::SceSink& w, const {ty}& value) noexcept {{\n\
         \x20       namespace cbor = ::SCE::Forge::Cbor;\n\
         {cases}\
         \x20       {CPP_UNREACHED}\n\
         \x20   }}\n"
    );

    let check = if union_can_refuse(union) {
        let mut cases = String::new();
        for case in &union.cases {
            let Some(record) = case.record.as_ref().filter(|r| record_can_refuse(r)) else {
                continue;
            };
            let [_, _, check_record] = composite_fns("record", &record.name);
            cases.push_str(&format!(
                "        if (const auto* held = ::std::get_if<{}>(&value.{CPP_UNION_CASES})) {{\n\
                 \x20           return {check_record}(held->{CPP_CASE_VALUE});\n\
                 \x20       }}\n",
                alternative(&case.variant)
            ));
        }
        format!(
            "\n\
             \x20   /// Refuse a value of `{name}` whose case's record has no encoding.\n\
             \x20   [[nodiscard]] static std::optional<::SCE::Forge::CodecError> {check}(const {ty}& value) noexcept {{\n\
             {cases}\
             \x20       return std::nullopt;\n\
             \x20   }}\n"
        )
    } else {
        String::new()
    };
    format!("{read}{write}{check}")
}

// ── The codec's own map ─────────────────────────────────────────────────

/// How the value presents and holds one entry.
enum Form {
    /// A text or a byte string of the codec's own map, which keeps the form it
    /// has had since docs/adr/0081 step 4 (docs/adr/0101, section 5.3):
    /// presented within its bound as `view`, held in `held`, read as `read`
    /// before it is held to its bound.
    View {
        view: String,
        held: String,
        read: String,
    },
    /// A number, a `bool` or an enum, presented, held and read as `ty`.
    Value { ty: String },
    /// A list, a record or a union, held in `ty`, the type a schema unit holds
    /// it in, and lent by `const` reference.
    Lent { ty: String },
}

/// One entry as the printed value presents and holds it.
struct Entry<'p> {
    item: &'p CborItem<'p>,
    /// Its accessor's name.
    name: String,
    form: Form,
    /// The initializer of what the value holds, where a value made by
    /// default does not hold its type's zero: an enum starts at its first
    /// declared variant.
    init: String,
}

impl Entry<'_> {
    fn required(&self) -> bool {
        self.item.required
    }

    /// Whether what the value holds owns storage of its own, which no
    /// `constexpr` makes: a text, a byte string, a list, a record or a union.
    fn owns(&self) -> bool {
        !matches!(self.form, Form::Value { .. })
    }

    /// What a source presents it as: by value, its view or its type, or an
    /// optional of it; lent, a `const` reference to its type or to an optional
    /// of it.
    fn presented(&self) -> String {
        let optional = |ty: &str| {
            if self.required() {
                ty.to_string()
            } else {
                format!("std::optional<{ty}>")
            }
        };
        match &self.form {
            Form::View { view, .. } => optional(view),
            Form::Value { ty } => optional(ty),
            Form::Lent { ty } => format!("const {}&", optional(ty)),
        }
    }

    /// The type of the local the encode takes what a source presents as: a
    /// `const` of a value, and the reference itself of what is lent.
    fn taken(&self) -> String {
        match self.form {
            Form::Lent { .. } => self.presented(),
            Form::View { .. } | Form::Value { .. } => format!("const {}", self.presented()),
        }
    }

    /// What the value holds it as.
    fn held_member(&self) -> String {
        let held = match &self.form {
            Form::View { held, .. } => held,
            Form::Value { ty } | Form::Lent { ty } => ty,
        };
        if self.required() {
            held.clone()
        } else {
            format!("std::optional<{held}>")
        }
    }

    /// What the decode reads it into before the value is made.
    fn read_type(&self) -> &str {
        match &self.form {
            Form::View { read, .. } => read,
            Form::Value { ty } | Form::Lent { ty } => ty,
        }
    }
}

/// The bound a text or a byte string of `e` is held to, as `Max` or `Max, Min`:
/// the bound its type states (docs/adr/0101, section 1.3), and an exact
/// length as both.
fn bound_args(e: &CborEntry, exact: bool) -> String {
    match (exact, e.length) {
        (true, Some(length)) => format!("{length}, {length}"),
        _ => e
            .size_bound()
            .expect("a text or byte string entry states its bound")
            .to_string(),
    }
}

fn entry<'p>(l: &LangCtx, item: &'p CborItem<'p>) -> Entry<'p> {
    let e = item.entry;
    let form = match &item.value {
        CborShape::Text { .. } => {
            let bound = bound_args(e, false);
            Form::View {
                view: format!("::SCE::Forge::Text<{bound}>"),
                held: format!("::SCE::Forge::Bounded<std::string, {bound}>"),
                read: cpp_type(&SceType::String).to_string(),
            }
        }
        CborShape::Bytes { exact, .. } => {
            let bound = bound_args(e, *exact);
            Form::View {
                view: format!("::SCE::Forge::Bytes<{bound}>"),
                held: format!("::SCE::Forge::Bounded<std::vector<std::uint8_t>, {bound}>"),
                read: cpp_type(&SceType::Bytes).to_string(),
            }
        }
        shape @ (CborShape::List { .. } | CborShape::Record(_) | CborShape::Union(_)) => {
            Form::Lent {
                ty: held_type(shape, Held::Composed),
            }
        }
        shape => Form::Value {
            ty: held_type(shape, Held::Composed),
        },
    };
    let init = match &item.value {
        CborShape::Enum(enumeration) if item.required => l.enum_default_expr(&enumeration.alias),
        _ => String::new(),
    };
    Entry {
        item,
        name: l.codec_field_id(&e.id),
        form,
        init,
    }
}

/// The statements that write `v`, the value of `entry`, after its key: a text
/// and a byte string the view holds within its bound, and any other value in
/// its one form ([`write_stmts`]).
fn write_value(entry: &Entry<'_>) -> Vec<String> {
    match (&entry.form, &entry.item.value) {
        (Form::View { .. }, CborShape::Text { .. }) => {
            vec![written("cbor::write_text(w, v.inner())".to_string())]
        }
        (Form::View { .. }, _) => vec![written("cbor::write_bytes(w, v.inner())".to_string())],
        (Form::Value { .. } | Form::Lent { .. }, shape) => {
            write_stmts(shape, "v", Held::Composed, 0)
        }
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
/// (`rust_source::cbor`): `CodecError::CborOutOfRange` for a list of fewer
/// elements than its lower count, which its type does not hold, and for a
/// record whose byte string is past its bound, which the schema's unit holds
/// in a `std::vector` with no bound, each at any depth.
fn refusal_checks(plan: &CborPlan<'_>, entry: &Entry<'_>) -> Option<String> {
    let shape = &entry.item.value;
    if !can_refuse(shape, Held::Composed) {
        return None;
    }
    let refusal = plan
        .totality
        .require(RefusalKind::CborOutOfRange, &entry.item.entry.id);
    let checks = check_stmts(shape, "v", Held::Composed, 0, &refusal);
    let (open, value) = present(entry);
    Some(format!(
        "{open}\
         \x20           const auto& v = {value};\n\
         {}\
         \x20       }}\n",
        indented(&checks, 12)
    ))
}

/// The statements that read the value of `entry` into its local, which the
/// caller has checked is still empty: a text or a byte string as the view
/// takes it, and any other value in its one form ([`read_stmts`]).
fn read_value(entry: &Entry<'_>) -> String {
    let e = entry.item.entry;
    let name = &entry.name;
    let limit = || {
        e.size_bound()
            .expect("a text or byte string entry states its bound")
            .to_string()
    };
    // A read whose `Decoded` holds the value as the local holds it.
    let whole = |read: String| {
        format!(
            "                auto v = {read};\n\
             \x20               if (!v) return v.error();\n\
             \x20               {name} = std::move(*v);\n"
        )
    };
    match (&entry.form, &entry.item.value) {
        (Form::View { .. }, CborShape::Text { .. }) => {
            whole(format!("cbor::read_text(c, {})", limit()))
        }
        (Form::View { .. }, CborShape::Bytes { exact: true, .. }) => whole(format!(
            "cbor::read_bytes_exact(c, {})",
            e.length.expect("an exact byte string states its length")
        )),
        (Form::View { .. }, _) => whole(format!("cbor::read_bytes(c, {})", limit())),
        (Form::Value { .. } | Form::Lent { .. }, shape) => {
            indented(&read_stmts(shape, name, Held::Composed, 0), 16)
        }
    }
}

/// The byte case of `source`, a value of the CBOR codec `plan` at `path`
/// (docs/adr/0081 §5): the value made by its constructor, which takes the
/// entries in declaration order (`value_class`), a text and a byte string each
/// made within its bound by `make` of the view its accessor presents it as, and
/// each entry compared through its accessor. A map takes no flag input and no
/// caller's tag.
pub(crate) fn source_case(
    plan: &CborPlan<'_>,
    path: &super::source_case::CodecPath,
    source: &serde_json::Value,
) -> Result<super::source_case::SourceCase, String> {
    let elements = super::super::source_notation::cbor_elements(plan, source)?;
    super::source_case::map_or_line_case(path, elements, super::source_case::Texts::Bounded)
}

/// The text of the printed CBOR codec `plan` inside its namespace: the
/// readers of its entries, the trait a source meets, the reads, writes and
/// checks of the records and unions it reaches, the encode over any source,
/// and the value, made by its constructor and read by its accessors.
pub(in super::super) fn render(
    plan: &CborPlan<'_>,
    l: &LangCtx,
    _imports: &[ImportContext],
    struct_name: &str,
) -> String {
    let entries: Vec<Entry<'_>> = plan.entries.iter().map(|item| entry(l, item)).collect();
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
             \x20       {ty} _e_{name} = _r_{name};\n",
            name = e.name,
            ty = e.taken()
        ));
    }
    // What the encoding cannot carry is refused before the first byte, each
    // entry's in the order the encode would meet it: ascending key, as the
    // Rust encode's count over a sink that only counts meets it.
    for &index in &plan.by_key {
        if let Some(checks) = refusal_checks(plan, &entries[index]) {
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
             {write}\
             \x20       }}\n",
            key = e.item.entry.key,
            write = indented(&write_value(e), 12)
        ));
    }
    // The records and unions the entries reach, read, written and checked
    // before the encode and the value that call them.
    out.push_str(&composites_struct(plan, struct_name));
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

    out.push_str(&value_class(&entries, &plan.by_key, struct_name, total));
    // Every refusal the totality analysis names is one the encode writes
    // (docs/adr/0065 §2).
    plan.totality.assert_all_written(struct_name);
    out
}

/// The value: made by its constructor, each entry read by its accessor,
/// decoded from a map and encoded as any source is. `by_key` is the order of
/// `entries` by key, the plan's ([`CborPlan::by_key`]), and `total` whether the
/// encode refuses no value, which the vector its bytes are returned in says.
fn value_class(entries: &[Entry<'_>], by_key: &[usize], struct_name: &str, total: bool) -> String {
    let params: Vec<String> = entries
        .iter()
        .map(|e| format!("{} {}", e.presented(), e.name))
        .collect();
    let inits: Vec<String> = entries
        .iter()
        .map(|e| format!("{}_({})", e.name, e.name))
        .collect();
    let owns = entries.iter().any(Entry::owns);
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
        let body = match &e.form {
            Form::View { view, .. } if !e.required() => format!(
                "if (!{name}_) {{ return std::nullopt; }} return {view}(*{name}_);",
                name = e.name
            ),
            _ => format!("return {}_;", e.name),
        };
        let constexpr = if e.owns() { "" } else { "constexpr " };
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
        decode.push_str(&format!(
            "        std::optional<{}> {};\n",
            e.read_type(),
            e.name
        ));
    }
    decode.push_str(
        "        for (std::uint64_t i = 0; i < *count; ++i) {\n\
         \x20           const auto key = cbor::read_uint(c);\n\
         \x20           if (!key) return key.error();\n\
         \x20           switch (*key) {\n",
    );
    for &at in by_key {
        let e = &entries[at];
        decode.push_str(&format!(
            "            case {key}: {{\n\
             \x20               // A key given twice is not one map entry.\n\
             \x20               if ({name}.has_value()) return ::SCE::Forge::CodecError::CborDuplicateKey;\n\
             {read}\
             \x20               break;\n\
             \x20           }}\n",
            key = e.item.entry.key,
            name = e.name,
            read = read_value(e)
        ));
    }
    decode.push_str(
        "            default: {\n\
         \x20               const auto skipped = cbor::skip(c);\n\
         \x20               if (!skipped) return skipped.error();\n\
         \x20               break;\n\
         \x20           }\n\
         \x20           }\n\
         \x20       }\n",
    );
    for e in entries.iter().filter(|e| e.required()) {
        decode.push_str(&format!(
            "        if (!{}.has_value()) return ::SCE::Forge::CodecError::CborRequiredKeyMissing;\n",
            e.name
        ));
    }
    let mut args: Vec<String> = Vec::new();
    for e in entries {
        let Form::View { view, .. } = &e.form else {
            args.push(if e.required() {
                format!("/*{name}=*/*{name}", name = e.name)
            } else {
                format!("/*{name}=*/{name}", name = e.name)
            });
            continue;
        };
        if e.required() {
            decode.push_str(&format!(
                "        const auto _b_{name} = {view}::make(*{name});\n\
                 \x20       if (!_b_{name}) return ::SCE::Forge::CodecError::CborOutOfRange;\n",
                name = e.name
            ));
            args.push(format!("/*{name}=*/*_b_{name}", name = e.name));
        } else {
            decode.push_str(&format!(
                "        std::optional<{view}> _b_{name};\n\
                 \x20       if ({name}) {{\n\
                 \x20           _b_{name} = {view}::make(*{name});\n\
                 \x20           if (!_b_{name}) return ::SCE::Forge::CodecError::CborOutOfRange;\n\
                 \x20       }}\n",
                name = e.name
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
         \x20   /// a refusal answers the `CodecError` that refused the map\n\
         \x20   /// (docs/adr/0099, amendment of 2026-11-16), with the cursor left where it was.\n\
         \x20   static ::SCE::Forge::Decoded<{struct_name}> decode(::SCE::Forge::SceCursor& cursor) {{\n\
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
         \x20   static ::SCE::Forge::Decoded<{struct_name}> decode_at(::SCE::Forge::SceCursor& c) {{\n\
         \x20       namespace cbor = ::SCE::Forge::Cbor;\n\
         \x20       const auto count = cbor::read_map_len(c);\n\
         \x20       if (!count) return count.error();\n\
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

    fn bytes(max: u32) -> CborShape {
        CborShape::Bytes { max, exact: false }
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
            vec![field("voucher_code", "bytes[4]", bytes(4))],
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

    /// Each of `expected` is in `code`.
    fn holds_all(code: &str, expected: &[&str]) {
        for expected in expected {
            assert!(code.contains(expected), "missing `{expected}`:\n{code}");
        }
    }

    // Red if `held_type` holds a record's own byte string bounded, or a list,
    // a record or a union in another type than its unit's.
    #[test]
    fn a_value_is_held_in_the_type_a_schema_unit_holds_it_in() {
        assert_eq!(held_type(&bytes(4), Held::Field), "std::vector<uint8_t>");
        assert_eq!(
            held_type(&list(bytes(4), 2, 0), Held::Composed),
            "::SCE::Forge::OwnedList<::SCE::Forge::OwnedBytes<4>, 2>"
        );
        assert_eq!(
            held_type(
                &list(CborShape::Nullable(Box::new(int(8, true))), 3, 0),
                Held::Composed
            ),
            "::SCE::Forge::OwnedList<std::optional<int8_t>, 3>"
        );
        assert_eq!(
            held_type(&CborShape::Float { bits: 32 }, Held::Composed),
            "float"
        );
        assert_eq!(
            held_type(&color(), Held::Composed),
            "SCE::Generated::PaintColor::PaintColor"
        );
        assert_eq!(
            held_type(&CborShape::Union(payment()), Held::Composed),
            "SCE::Generated::OrderPayment::OrderPayment"
        );
        assert_eq!(
            held_type(
                &CborShape::Record(record("order_customer", Vec::new())),
                Held::Composed
            ),
            "SCE::Generated::OrderCustomer::OrderCustomerPayload"
        );
    }

    // Red if a signed integer is read as an unsigned one, a narrow one read
    // past its type's range, or a closed enum's undeclared number admitted.
    #[test]
    fn a_number_is_read_within_its_type_and_written_in_either_sign() {
        let read = read_stmts(&int(16, true), "x", Held::Composed, 0).join("\n");
        holds_all(
            &read,
            &[
                "const auto _r0 = cbor::read_int_within(c, \
                 static_cast<std::int64_t>(std::numeric_limits<int16_t>::min()), \
                 static_cast<std::int64_t>(std::numeric_limits<int16_t>::max()));",
                "if (!_r0) return _r0.error();",
                "x = static_cast<int16_t>(*_r0);",
            ],
        );
        assert_eq!(
            read_stmts(&CborShape::Float { bits: 32 }, "x", Held::Composed, 0),
            ["auto _r0 = cbor::read_float32(c);\nif (!_r0) return _r0.error();\nx = std::move(*_r0);"]
        );
        assert_eq!(
            write_stmts(&int(16, true), "record.n", Held::Field, 0),
            ["if (auto _e = cbor::write_int(w, static_cast<std::int64_t>(record.n)); _e) return _e;"]
        );
        assert_eq!(
            write_stmts(&int(64, false), "_item0", Held::Composed, 1),
            ["if (auto _e = cbor::write_uint(w, static_cast<std::uint64_t>(_item0)); _e) return _e;"]
        );
        assert_eq!(
            write_stmts(&CborShape::Float { bits: 32 }, "v", Held::Composed, 0),
            ["if (auto _e = cbor::write_float(w, static_cast<double>(v)); _e) return _e;"]
        );
        // An enum is the integer of its declared number, the variant it names
        // read back and a number a closed enum does not declare refused.
        holds_all(
            &read_stmts(&color(), "x", Held::Composed, 0).join("\n"),
            &[
                "const auto _r0 = cbor::read_uint_upto(c, \
                 static_cast<std::uint64_t>(std::numeric_limits<uint8_t>::max()));",
                "const auto _r0_variant = \
                 SCE::Generated::PaintColor::from_underlying(static_cast<uint8_t>(*_r0));",
                "if (!_r0_variant) return ::SCE::Forge::CodecError::UndeclaredEnumValue;",
                "x = *_r0_variant;",
            ],
        );
        assert_eq!(
            write_stmts(&color(), "record.color", Held::Field, 0),
            [
                "if (auto _e = cbor::write_uint(w, static_cast<std::uint64_t>(\
              SCE::Generated::PaintColor::to_underlying(record.color))); _e) return _e;"
            ]
        );
    }

    // Red if a list's count is not judged before its elements are read, or an
    // absent element is written as anything but `null`.
    #[test]
    fn a_list_is_read_within_its_counts_and_an_absent_element_is_null() {
        let shape = list(CborShape::Nullable(Box::new(int(16, false))), 3, 1);
        holds_all(
            &read_stmts(&shape, "v", Held::Composed, 0).join("\n"),
            &[
                "const auto _len0 = cbor::read_array_len(c);",
                "if (*_len0 > 3 || *_len0 < 1) return ::SCE::Forge::CodecError::CborOutOfRange;",
                "::SCE::Forge::OwnedList<std::optional<uint16_t>, 3> _list0;",
                "    std::optional<uint16_t> _item0{};\n",
                "    const auto _null1 = cbor::take_null(c);\n",
                "        _item0 = std::nullopt;\n",
                "        uint16_t _present1{};\n",
                "    if (!_list0.push(std::move(_item0))) return \
                 ::SCE::Forge::CodecError::CborOutOfRange;\n",
                "v = std::move(_list0);",
            ],
        );
        holds_all(
            &write_stmts(&shape, "v", Held::Composed, 0).join("\n"),
            &[
                "if (auto _e = cbor::write_array_head(w, static_cast<std::uint64_t>(v.size())); \
                 _e) return _e;",
                "for (const auto& _item0 : v) {\n",
                "    if (!_item0.has_value()) {\n        \
                 if (auto _e = cbor::write_null(w); _e) return _e;\n    } else {\n        \
                 const auto& _present1 = *_item0;\n        \
                 if (auto _e = cbor::write_uint(w, static_cast<std::uint64_t>(_present1)); _e) \
                 return _e;\n    }\n",
            ],
        );
        // A list in a list reads and writes its own, named apart.
        let nested = list(list(int(8, false), 2, 0), 2, 0);
        assert!(read_stmts(&nested, "v", Held::Composed, 0)[0]
            .contains("    const auto _len1 = cbor::read_array_len(c);\n"));
        assert!(write_stmts(&nested, "v", Held::Composed, 0)[0]
            .contains("    for (const auto& _item1 : _item0) {\n"));
    }

    // Red if `can_refuse` reads another rule than the totality's, or a check
    // is written where no value of the shape can be refused.
    #[test]
    fn a_lower_count_and_a_fields_byte_string_are_checked_and_nothing_else() {
        // A byte string inside a list is held to its bound by its type.
        assert!(!can_refuse(&list(bytes(4), 2, 0), Held::Composed));
        assert!(can_refuse(&bytes(4), Held::Field));
        assert!(can_refuse(&list(int(8, false), 2, 1), Held::Composed));
        assert!(!can_refuse(&CborShape::Text { max: 8 }, Held::Field));
        assert!(can_refuse(&CborShape::Union(payment()), Held::Composed));
        assert_eq!(
            check_stmts(&list(int(8, false), 2, 1), "v", Held::Composed, 0, "E"),
            ["if (v.size() < 1) return ::SCE::Forge::E;"]
        );
        assert!(check_stmts(&list(bytes(4), 2, 0), "v", Held::Composed, 0, "E").is_empty());
        assert_eq!(
            check_stmts(&bytes(4), "record.blob", Held::Field, 0, "E"),
            ["if (record.blob.size() > 4) return ::SCE::Forge::E;"]
        );
        // A union's check is its document's, called and its refusal returned.
        assert_eq!(
            check_stmts(
                &list(CborShape::Union(payment()), 3, 0),
                "v",
                Held::Composed,
                0,
                "E"
            ),
            ["for (const auto& _item0 : v) {\n    \
              if (auto _e = Composites::check_union_order_payment(_item0); _e) return _e;\n}"]
        );
    }

    // Red if a record's map is written in the schema's order rather than the
    // plan's order of its keys, or an absent field is written at all.
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
        holds_all(
            &code,
            &[
                "static ::SCE::Forge::Decoded<SCE::Generated::OrderLine::OrderLinePayload> \
                 decode_record_order_line(::SCE::Forge::SceCursor& c) {",
                "        std::optional<::SCE::Forge::OwnedText<8>> held_note;\n",
                "            } else if (*key == \"note\") {\n",
                "        if (!held_quantity.has_value()) return \
                 ::SCE::Forge::CodecError::CborRequiredKeyMissing;\n",
                "        record.note = std::move(held_note);\n",
                "        record.quantity = std::move(*held_quantity);\n",
                "[[nodiscard]] static std::optional<::SCE::Forge::CodecError> \
                 encode_record_order_line(::SCE::Forge::SceSink& w, \
                 const SCE::Generated::OrderLine::OrderLinePayload& record) noexcept {",
                // An absent field is left out of the map and its count.
                "        std::uint64_t count = 3;\n        if (record.note.has_value()) ++count;\n",
                "        if (record.note.has_value()) {\n            \
                 const auto& value = *record.note;\n            \
                 if (auto _e = cbor::write_text(w, \"note\"); _e) return _e;\n            \
                 if (auto _e = cbor::write_text(w, value.inner()); _e) return _e;\n        }\n",
            ],
        );
        // The plan's order of the keys' encodings: `sku`, `note`, `color`,
        // `quantity`, whatever the order the schema declares them in.
        let write = code
            .split("encode_record_order_line(")
            .nth(1)
            .expect("the record has a write");
        let at = |key: &str| {
            write
                .find(&format!("cbor::write_text(w, \"{key}\")"))
                .unwrap_or_else(|| panic!("the record writes `{key}`:\n{write}"))
        };
        assert!(at("sku") < at("note") && at("note") < at("color") && at("color") < at("quantity"));
        // Nothing of it is refused, so it has no check.
        assert!(!code.contains("check_record_order_line"), "{code}");
    }

    // Red if a union is written with its tag beside its fields rather than as
    // one entry named by its case, or its empty case as anything but `a0`.
    #[test]
    fn a_union_is_one_entry_named_by_its_case_and_an_empty_case_is_the_empty_map() {
        let code = union_fns(&payment());
        let ty = "SCE::Generated::OrderPayment::OrderPayment";
        let decode = format!(
            "static ::SCE::Forge::Decoded<{ty}> decode_union_order_payment(\
             ::SCE::Forge::SceCursor& c) {{"
        );
        let card = format!("            return {ty}{{{ty}::Card{{std::move(*record)}}}};\n");
        let cash = format!("            return {ty}{{{ty}::Cash{{}}}};\n");
        let write_cash = format!(
            "        if (::std::holds_alternative<{ty}::Cash>(value.cases)) {{\n            \
             if (auto _e = cbor::write_variant(w, \"cash\"); _e) return _e;\n            \
             return cbor::write_map_head(w, 0);\n        }}\n"
        );
        let write_card = format!(
            "        if (const auto* held = ::std::get_if<{ty}::Card>(&value.cases)) {{\n            \
             if (auto _e = cbor::write_variant(w, \"card\"); _e) return _e;\n            \
             return Composites::encode_record_card_payment(w, held->value);\n        }}\n"
        );
        // Only the `voucher` case's record can be refused.
        let check_voucher = format!(
            "        if (const auto* held = ::std::get_if<{ty}::Voucher>(&value.cases)) {{\n            \
             return Composites::check_record_voucher_payment(held->value);\n        }}\n"
        );
        holds_all(
            &code,
            &[
                decode.as_str(),
                "static constexpr std::string_view kCases[] = {\"card\", \"cash\", \"voucher\"};",
                "const auto at = cbor::read_variant(c, kCases);",
                "        case 0: {\n            \
                 auto record = Composites::decode_record_card_payment(c);\n",
                card.as_str(),
                "            const auto empty = cbor::read_empty_case(c);\n",
                cash.as_str(),
                "return ::SCE::Forge::CodecError::CborUnknownVariant;",
                write_cash.as_str(),
                write_card.as_str(),
                check_voucher.as_str(),
            ],
        );
        assert!(!code.contains("check_record_card_payment"), "{code}");
    }
}
