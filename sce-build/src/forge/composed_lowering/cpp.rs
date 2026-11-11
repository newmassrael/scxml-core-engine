// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! The C++ spelling of the walks over a composed value ([`super`])
//! (docs/adr/0086, section 7.1, the C++ row), and the checks a C++ raise seam
//! runs over a host's value before it encodes it (section 5.4).
//!
//! | `list<T>[N]` | `string[N]` / `bytes[N]` | `optional<T>` | record |
//! |---|---|---|---|
//! | `OwnedList<T, N>` | `OwnedText<N>` / `OwnedBytes<N>` | `std::optional<T>` | the machine's struct |
//!
//! Each bounded type is the forge runtime's (`sce/forge/bounded.h`), its bound
//! in its type (section 7.4), as the Rust row's are: a value past it is no value
//! of the type, so a host cannot build one, and a copy into a place of a larger
//! bound widens by type. Two places of one type are copied by the assignment,
//! which copies a list by value at every depth.

use super::*;

/// The namespace of the forge runtime's bounded types and checked
/// constructions (`sce/forge/bounded.h`, `sce/forge/algorithm.h`).
pub(crate) const CPP_FORGE: &str = "::SCE::Forge";

/// The type a C++ machine holds a list of at most `max` values held in `elem`
/// in, every place alike (docs/adr/0086, section 7.4): a variable, a record's
/// field and a payload's, so two places of one list type are one C++ type.
pub(crate) fn cpp_bounded_list(elem: &str, max: u32) -> String {
    format!("{CPP_FORGE}::OwnedList<{elem}, {max}>")
}

/// The type a C++ machine holds a text of at most `max` bytes in, every place
/// alike (docs/adr/0086, section 7.4).
pub(crate) fn cpp_text_type(max: u32) -> String {
    format!("{CPP_FORGE}::OwnedText<{max}>")
}

/// The type a C++ machine holds a byte string of at most `max` bytes in,
/// every place alike (docs/adr/0086, section 7.4).
pub(crate) fn cpp_bytes_type(max: u32) -> String {
    format!("{CPP_FORGE}::OwnedBytes<{max}>")
}

/// The C++ spellings of the walks, for the machine `machine` (section 7.1, the
/// C++ row): the runtime's `EventPayloadValue` (`common/EventPayloadLift.h`)
/// reads each constructor, and its writers write them into `carrier`.
///
/// A C++ reader fills a place and answers its refusal, so a reader here is a
/// lambda of the value and the place (`CPP_READER`), which the reader of
/// the part that holds it hands to the runtime's combinator (`readList`,
/// `readOptional`, `readRecord`); the place's type is deduced, so no reader
/// names one. A writer and a copy walk a list by a range `for` and a value
/// that may be absent by its `has_value`, each in a lambda the value is handed
/// to as its argument. A binding of a nested lambda is its own parameter or
/// local, which hides its holder's of the same name: the holder's value was
/// handed in before the inner one is bound, and each part reads its own.
pub struct CppComposed<'a> {
    pub machine: &'a str,
    pub carrier: CppCarrier,
}

/// What a C++ writer spells a composed value into: one walk of the value's
/// type, written as the carrier the place that takes it holds.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CppCarrier {
    /// The JSON text itself, by the runtime's `EventPayloadFields::json`,
    /// `list` and `record`: what the inject seam writes as the event's `data`
    /// beside the typed carrier (section 3.1).
    Json,
    /// The `ScriptValue` a `<send>` holds each param in
    /// (`StaticTarget::wire_value`) until `EventDataHelper::buildEventDataJson`
    /// writes them as one JSON object: a list as a `ScriptArray` of its
    /// elements, a record as a `ScriptObject` of its fields, whose keys the
    /// writer orders by their bytes at every depth, an absent value as
    /// `ScriptNull`, and every scalar as a `<send>`'s scalar param is (section
    /// 5.3; ARCHITECTURE.md, "JSON Object Key Order").
    Wire,
}

/// The namespace of the C++ runtime's payload reader, writers and checks
/// (`sce/include/common/EventPayloadLift.h`).
const CPP_COMMON: &str = "::SCE::Common";

/// The name a C++ reader binds the value it reads to: the first parameter of
/// [`CPP_READER`].
const CPP_READ_BINDING: &str = "sceValue";

/// The head of every C++ reader: a lambda of the value it reads and the place
/// it fills, whose type is deduced where the runtime's combinator hands it
/// one, answering the refusal of a value outside its type, empty where it
/// filled the place.
const CPP_READER: &str = "[&](const ::SCE::Common::EventPayloadValue &sceValue, auto &sceInto) \
                          -> ::SCE::Common::EventPayloadRefusal";

/// The name a C++ writer binds an element or a present value to.
const CPP_ELEMENT_BINDING: &str = "sceElement";

/// The name a C++ copy binds an element or a present value to.
const CPP_COPY_BINDING: &str = "sceCopied";

/// The method a C++ union answers the value of its tag's enum by, whatever key
/// the tag is written under (docs/adr/0092, section 7, the C++ row: `kind()`
/// from the alternative's index), on a machine's union as on a unit's.
pub(crate) const CPP_UNION_KIND: &str = "kind";

/// The member of a C++ union that holds its `std::variant` of one alternative
/// per case, on a machine's union as on a unit's.
pub(crate) const CPP_UNION_CASES: &str = "cases";

/// The member of a C++ union's alternative that holds the case's record, on a
/// machine's union as on a unit's.
pub(crate) const CPP_CASE_VALUE: &str = "value";

/// The statement a C++ union's reads end in where the narrowing proved the
/// value is in none of the cases the read takes apart: the one statement no
/// path reaches, which ends the process rather than read another case's
/// storage, as Rust's `unreachable!()` and Go's `panic` do.
pub(crate) const CPP_UNREACHED: &str = "::std::abort();";

/// The identifier of the case `variant` of a C++ union: the struct of its
/// alternative, nested in the union's struct and named as the tag's enum
/// names the variant (docs/adr/0092, section 7 items 1 and 2), on a machine's
/// union as on a unit's.
pub(crate) fn cpp_case_ident(variant: &str) -> String {
    crate::forge::enum_naming::variant_ident(crate::generator::Language::Cpp, "", variant)
}

/// The read of the record of the case `variant` where a test proved it, on a
/// machine's union as on a unit's.
pub(crate) fn cpp_case_read(variant: &str) -> String {
    format!(
        "proven{}",
        crate::filters::to_pascal_case(variant.to_string())
    )
}

/// The read of a field several cases share ([`SharedFieldDef`]) on a machine's
/// union.
fn cpp_shared_read(field: &str) -> String {
    format!(
        "provenField{}",
        crate::filters::to_pascal_case(field.to_string())
    )
}

/// The accessor of the reference a write of what the read `read` answers (a
/// case's record, a shared field) goes through (docs/adr/0092, section 6.1
/// item 6).
fn cpp_place_of(read: &str) -> String {
    format!("{read}Place")
}

/// A C++ string literal of `text`.
fn cpp_literal(text: &str) -> String {
    format!("\"{}\"", crate::filters::escape_cpp(text.to_string()))
}

/// The C++ spelling of what follows a union's value to read `member` where it
/// may be in the cases `left` ([`ComposedSpelling::union_member`]), as
/// [`union_member_of`] reads it: the tag's method; the field of the one case
/// left, through that case's proven read; or the accessor of a field several
/// cases share. `Err` as [`union_member_of`]'s.
pub(crate) fn cpp_union_member(
    union: &ScopedUnion,
    left: &[&ScopedCase],
    member: &str,
) -> Result<String, String> {
    Ok(match union_member_of(union, left, member)? {
        UnionMember::Tag => format!("{CPP_UNION_KIND}()"),
        UnionMember::Case(case) => format!("{}().{member}", cpp_case_read(&case.variant.name)),
        UnionMember::Shared => format!("{}()", cpp_shared_read(member)),
    })
}

/// The C++ read of `member` of the union value `holder`, a parameter or a local
/// of an algorithm spelled as C++ names it, whose type `ty` is the struct the
/// union's own unit declares (docs/adr/0092, section 7, the C++ row, and
/// Landing order step 5), where the value may be in the cases `left`, as
/// [`unit_union_member_of`] classifies it: the tag by `kind()`, which the
/// unit's struct answers whatever key the tag is written under; the field of
/// the one case left, through that case's proven read; or the field the cases
/// left each declare, read by a lambda called where it is written, which takes
/// each case left apart by `std::get_if` on its alternative and returns the
/// field, and whose end the narrowing proved is never reached
/// ([`CPP_UNREACHED`]). The lambda's result is deduced from its returns, which
/// [`unit_union_member_of`] proved are of one type. A case's record is the
/// payload struct its event schema's unit declares, whose fields are named as
/// the schema writes them ([`crate::forge::generator::event_schema_field_ident`]).
/// `Err` as [`unit_union_member_of`]'s.
pub(crate) fn cpp_unit_union_read(
    holder: &str,
    ty: &str,
    union: &ScopedUnion,
    left: &[&ScopedCase],
    member: &str,
) -> Result<String, String> {
    let field =
        crate::forge::generator::event_schema_field_ident(member, crate::generator::Language::Cpp);
    Ok(match unit_union_member_of(union, left, member)? {
        UnionMember::Tag => format!("{holder}.{CPP_UNION_KIND}()"),
        UnionMember::Case(case) => {
            format!("{holder}.{}().{field}", cpp_case_read(&case.variant.name))
        }
        UnionMember::Shared => {
            let arms: String = left
                .iter()
                .map(|case| {
                    format!(
                        "if (const auto *sceCase = ::std::get_if<{ty}::{}>(&{holder}.\
                         {CPP_UNION_CASES})) {{ return sceCase->{CPP_CASE_VALUE}.{field}; }} ",
                        cpp_case_ident(&case.variant.name)
                    )
                })
                .collect();
            format!("[&]() {{ {arms}{CPP_UNREACHED} }}()")
        }
    })
}

/// The check of a C++ value of `ty`, written in the scope `within` names, at
/// `value`, named `path` (an expression of a `std::string`): an expression of
/// type `std::string` that is the refusal, naming the path to the place, of a
/// list short of the lower count its type states at any depth, and empty
/// where there is none (docs/adr/0086, sections 5.4 and 15.6). One call of the
/// runtime's `EventPayloadBounds::listWithin` per list that states a lower
/// count, of `list` per list whose elements hold one, and of `optional` for a
/// value that may be absent, the check of an element or a present value
/// inside the lambda the call hands it to, its parameters named by their
/// `depth`; a record is checked field by field where it stands, at the path of
/// each field below its own. `None` for a type that holds no lower count at
/// any depth.
///
/// A text's, a byte string's and a list's upper bound is its type's
/// (`OwnedText`, `OwnedBytes`, `OwnedList`; section 7.4), so a host cannot
/// build a value past one, and nothing checks it again. A lower count is held
/// by no type, as on every backend, so a C++ raise seam runs this check over
/// the payload a host hands it before it encodes it.
fn cpp_bound_check(
    ty: &ValueType,
    within: Option<&str>,
    path: &str,
    value: &str,
    depth: usize,
    records: &dyn RecordEnv,
) -> Result<Option<String>, String> {
    let bounds = format!("{CPP_COMMON}::EventPayloadBounds");
    let (inner_path, inner_value) = (format!("scePath{depth}"), format!("sceValue{depth}"));
    // The lambda the runtime hands a part to, at the part's own path.
    let part = |inner: &str| {
        format!(
            "[&](const ::std::string &{inner_path}, const auto &{inner_value}) -> ::std::string \
             {{ return {inner}; }}"
        )
    };
    Ok(match ty {
        // The bound is the type's (section 7.4).
        ValueType::Text { .. } | ValueType::Bytes { .. } => None,
        // A lower count is held whatever the elements are, an element with
        // nothing of its own to check handed to a lambda that checks nothing;
        // a list without one is walked only for an element that holds one.
        ValueType::List { elem, max } => {
            let inner =
                cpp_bound_check(elem, within, &inner_path, &inner_value, depth + 1, records)?;
            match (max.min, inner) {
                (0, None) => None,
                (0, Some(inner)) => Some(format!(
                    "{bounds}::list({path}, {value}, {}u, {})",
                    max.max,
                    part(&inner)
                )),
                (least, inner) => {
                    let each = inner.map_or_else(
                        || {
                            "[](const ::std::string &, const auto &) -> ::std::string { return {}; }"
                                .to_string()
                        },
                        |inner| part(&inner),
                    );
                    Some(format!(
                        "{bounds}::listWithin({path}, {value}, {least}u, {}u, {each})",
                        max.max
                    ))
                }
            }
        }
        // An absent value holds nothing to check; a present one is checked at
        // the place's own path, which the runtime hands the lambda.
        ValueType::Optional { elem } => {
            let Some(inner) =
                cpp_bound_check(elem, within, &inner_path, &inner_value, depth + 1, records)?
            else {
                return Ok(None);
            };
            Some(format!(
                "{bounds}::optional({path}, {value}, {})",
                part(&inner)
            ))
        }
        ValueType::Record { alias } => {
            let held = records.record(within, alias)?;
            let mut checks = Vec::new();
            for field in &held.schema.fields {
                let below = format!(
                    "{path} + \"/{}\"",
                    crate::filters::escape_cpp(field.id.clone())
                );
                if let Some(check) = cpp_bound_check(
                    &field.sce_type,
                    Some(held.alias),
                    &below,
                    &format!("{value}.{}", field.id),
                    depth,
                    records,
                )? {
                    checks.push(check);
                }
            }
            match checks.as_slice() {
                [] => None,
                [one] => Some(one.clone()),
                many => {
                    let each: String = many
                        .iter()
                        .map(|check| {
                            format!(
                                "sceRefused = {check}; if (!sceRefused.empty()) {{ return \
                                 sceRefused; }} "
                            )
                        })
                        .collect();
                    Some(format!(
                        "[&]() -> ::std::string {{ ::std::string sceRefused; {each}return \
                         sceRefused; }}()"
                    ))
                }
            }
        }
        ValueType::Scalar(_) => None,
        // The record of the case the value is in, checked as a record is, at
        // the union's own path: a field of a case stands at the union's place
        // (docs/adr/0092, section 2.1). The case is told by its alternative,
        // which the union's `std::variant` holds in the tag enum's order, so
        // the check names no type. A case whose record holds no lower count,
        // and an empty case, check nothing.
        ValueType::Union { alias } => {
            let held = records.union(within, alias)?;
            let case_value = format!("sceCase{depth}");
            let mut arms = String::new();
            for (index, case) in held.union.cases.iter().enumerate() {
                let Some(record) = &case.record else {
                    continue;
                };
                if let Some(check) = cpp_bound_check(
                    &ValueType::Record {
                        alias: record.clone(),
                    },
                    Some(held.key.as_str()),
                    path,
                    &format!("{case_value}->{CPP_CASE_VALUE}"),
                    depth + 1,
                    records,
                )? {
                    arms.push_str(&format!(
                        "if (const auto *{case_value} = ::std::get_if<{index}>(&({value}).\
                         {CPP_UNION_CASES})) {{ return {check}; }} "
                    ));
                }
            }
            (!arms.is_empty()).then(|| format!("[&]() -> ::std::string {{ {arms}return {{}}; }}()"))
        }
    })
}

/// The checks a C++ raise seam runs over the fields of the payload a host
/// hands it, each a parameter of the seam named by the field's id, before it
/// encodes any of it ([`cpp_bound_check`]): one expression of type
/// `std::string` per field whose type holds a bound at any depth, in the
/// schema's order, the path of each the field's id.
pub(crate) fn cpp_raise_checks(
    held: HeldRecord<'_>,
    records: &dyn RecordEnv,
) -> Result<Vec<String>, String> {
    let mut checks = Vec::new();
    for field in &held.schema.fields {
        let path = format!(
            "::std::string(\"{}\")",
            crate::filters::escape_cpp(field.id.clone())
        );
        if let Some(check) = cpp_bound_check(
            &field.sce_type,
            Some(held.alias),
            &path,
            &field.id,
            0,
            records,
        )? {
            checks.push(check);
        }
    }
    Ok(checks)
}

impl CppComposed<'_> {
    /// The runtime's JSON of the scalar `value`, at the spelling its type
    /// deserves (`EventPayloadFields::json`).
    fn json(value: &str) -> String {
        format!("{CPP_COMMON}::EventPayloadFields::json({value})")
    }
}

impl ComposedSpelling for CppComposed<'_> {
    fn scalar_type(&self, ty: &SceType) -> String {
        crate::forge::generator::cpp_type(ty).to_string()
    }
    // The C++ row of section 7.1: the forge runtime's bounded storage, `N` on
    // the type (section 7.4), which the reader fills and a host builds through
    // the type's own checked constructions.
    fn text_type(&self, max: u32) -> String {
        cpp_text_type(max)
    }
    fn bytes_type(&self, max: u32) -> String {
        cpp_bytes_type(max)
    }
    // The one C++ spelling of a value that may be absent (docs/adr/0055,
    // section 5).
    fn optional_type(&self, elem: &str) -> String {
        format!("std::optional<{elem}>")
    }
    fn list_type(&self, elem: &str, max: u32) -> String {
        cpp_bounded_list(elem, max)
    }
    // The name a statechart's record variable of the same schema is held in
    // (`StaticTarget::record_type`), so a record a payload holds is the type a
    // variable holds.
    fn record_type(&self, alias: &str) -> String {
        format!(
            "{}{}Record",
            self.machine,
            crate::filters::to_pascal_case(alias.to_string())
        )
    }
    // Plain data by the record rule: a struct of the schema's fields in the
    // schema's order, each named as the schema names it, which is how a read
    // names it, and which a host builds by position in that order
    // (docs/adr/0059, section 5). Every upper bound is held by a member's type
    // (section 7.4); a list's lower count, which no type holds, is checked by
    // the raise seam that takes one before it sends it (section 5.4), so no
    // check is declared beside it.
    fn record_def(
        &self,
        ty: &str,
        alias: &str,
        fields: &[(String, String)],
        _checks: &[String],
    ) -> String {
        let members: String = fields
            .iter()
            .map(|(id, ty)| format!("    {ty} {id};\n"))
            .collect();
        format!(
            "/// SCE Accepted Subset §2.15: a `record:{alias}` value that holds a composed\n\
             /// value (docs/adr/0086), built by position in the schema's order; each\n\
             /// member's type holds its bound.\n\
             struct {ty} {{\n{members}}};"
        )
    }

    fn read_binding(&self) -> &'static str {
        CPP_READ_BINDING
    }
    // The width is the place's own type, which the runtime's reader is
    // instantiated at, so a field widened in its schema cannot keep being read
    // at the old width.
    fn read_scalar(&self, ty: &SceType, _value: &str) -> String {
        let read = match ty {
            SceType::Uint64 | SceType::Int64 => "readWide",
            SceType::Float32 | SceType::Float64 => "readFloating",
            SceType::Bool => "readBool",
            SceType::Uint8
            | SceType::Uint16
            | SceType::Uint32
            | SceType::Int8
            | SceType::Int16
            | SceType::Int32 => "readInteger",
            SceType::String | SceType::Bytes | SceType::Enum(_) => unreachable!(
                "a text, a byte string and an enum are cases of their own in the walk, never \
                 a scalar of a value type"
            ),
        };
        format!("{CPP_READER} {{ return sceValue.{read}(sceInto); }}")
    }
    // The variant whose declared name the wire carries; a name the enum does
    // not declare is refused with the code and pointer of the refusal.
    fn read_enum(&self, held: &MachineEnum, _value: &str) -> String {
        let arms: String = held
            .variants
            .iter()
            .map(|(declared, ident)| {
                format!(
                    "if (sceName == \"{}\") {{ sceInto = {}::{ident}; return {{}}; }} ",
                    crate::filters::escape_cpp(declared.clone()),
                    held.ty
                )
            })
            .collect();
        format!(
            "{CPP_READER} {{ ::std::string sceName; {CPP_COMMON}::EventPayloadRefusal sceNamed = \
             sceValue.readName(sceName); if (!sceNamed.empty()) {{ return sceNamed; }} {arms}return \
             sceValue.unknownVariant(sceName, \"{}\"); }}",
            crate::filters::escape_cpp(held.alias.clone())
        )
    }
    fn read_text(&self, max: u32, _value: &str) -> String {
        format!("{CPP_READER} {{ return sceValue.readText({max}u, sceInto); }}")
    }
    fn read_bytes(&self, max: u32, _value: &str) -> String {
        format!("{CPP_READER} {{ return sceValue.readBytes({max}u, sceInto); }}")
    }
    fn read_optional(&self, _value: &str, elem: &str, _elem_ty: &str) -> String {
        format!("{CPP_READER} {{ return sceValue.readOptional(sceInto, {elem}); }}")
    }
    // The bound, and a lower count where the type states one, are held before
    // any element is read (docs/adr/0086, section 15.5).
    fn read_list(&self, _value: &str, least: u32, max: u32, elem: &str, _elem_ty: &str) -> String {
        if least == 0 {
            format!("{CPP_READER} {{ return sceValue.readList({max}u, sceInto, {elem}); }}")
        } else {
            format!(
                "{CPP_READER} {{ return sceValue.readListWithin({least}u, {max}u, sceInto, {elem}); \
                 }}"
            )
        }
    }
    // Each field read into the struct in the schema's order by its own reader,
    // the first refusal returned as it is; a field a later revision added is
    // read out of the defaults where the value lacks it.
    fn read_record(
        &self,
        _value: &str,
        defaults: Option<&str>,
        _ty: &str,
        fields: &[(String, String)],
    ) -> String {
        let defaults = defaults.map_or_else(
            || "nullptr".to_string(),
            |json| format!("\"{}\"", crate::filters::escape_cpp(json.to_string())),
        );
        // A record of no fields fills nothing of its place.
        if fields.is_empty() {
            return format!(
                "{CPP_READER} {{ (void)sceInto; return sceValue.readRecord({defaults}, [](const \
                 {CPP_COMMON}::EventPayloadFields &) -> {CPP_COMMON}::EventPayloadRefusal {{ \
                 return {{}}; }}); }}"
            );
        }
        let members: String = fields
            .iter()
            .map(|(id, read)| {
                format!(
                    "sceRefused = sceFields.value(\"{}\", sceField); if (!sceRefused.empty()) {{ \
                     return sceRefused; }} sceRefused = ({read})(sceField, sceInto.{id}); if \
                     (!sceRefused.empty()) {{ return sceRefused; }} ",
                    crate::filters::escape_cpp(id.clone())
                )
            })
            .collect();
        format!(
            "{CPP_READER} {{ return sceValue.readRecord({defaults}, [&](const \
             {CPP_COMMON}::EventPayloadFields &sceFields) -> {CPP_COMMON}::EventPayloadRefusal {{ \
             {CPP_COMMON}::EventPayloadValue sceField; {CPP_COMMON}::EventPayloadRefusal \
             sceRefused; {members}return {{}}; }}); }}"
        )
    }

    fn write_binding(&self) -> &'static str {
        CPP_ELEMENT_BINDING
    }
    fn write_scalar(&self, ty: &SceType, held: &Held) -> String {
        let value = &held.text;
        let wide = matches!(ty, SceType::Uint64 | SceType::Int64);
        match self.carrier {
            // The one spelling of a payload field on the wire the inject seam
            // of a schema of entries writes too: a 64-bit integer as the text
            // of its digits.
            CppCarrier::Json if wide => Self::json(&format!("::std::to_string({value})")),
            CppCarrier::Json => Self::json(value),
            // As a scalar param is (`StaticTarget::wire_value`): an integer of
            // at most 32 bits widened to `int64_t` and a real to `double`,
            // both exactly, and a 64-bit integer as the text of its digits.
            CppCarrier::Wire => match ty {
                SceType::Uint8
                | SceType::Uint16
                | SceType::Uint32
                | SceType::Int8
                | SceType::Int16
                | SceType::Int32 => format!(
                    "ScriptValue(std::in_place_type<int64_t>, static_cast<int64_t>({value}))"
                ),
                SceType::Uint64 | SceType::Int64 => {
                    format!("ScriptValue(std::in_place_type<std::string>, std::to_string({value}))")
                }
                SceType::Float32 | SceType::Float64 => {
                    format!("ScriptValue(std::in_place_type<double>, static_cast<double>({value}))")
                }
                SceType::Bool => format!("ScriptValue(std::in_place_type<bool>, {value})"),
                SceType::String | SceType::Bytes | SceType::Enum(_) => unreachable!(
                    "a text, a byte string and an enum are cases of their own in the walk, \
                     never a scalar of a value type"
                ),
            },
        }
    }
    // The machine's enum's `sceLogName` answers the name its document
    // declares.
    fn write_enum(&self, held: &Held) -> String {
        let name = format!("::std::string(sceLogName({}))", held.text);
        match self.carrier {
            CppCarrier::Json => Self::json(&name),
            CppCarrier::Wire => format!("ScriptValue(std::in_place_type<std::string>, {name})"),
        }
    }
    fn write_text(&self, held: &Held) -> String {
        match self.carrier {
            CppCarrier::Json => Self::json(&held.text),
            CppCarrier::Wire => format!(
                "ScriptValue(std::in_place_type<std::string>, {})",
                held.text
            ),
        }
    }
    // JSON has no byte string: it crosses as its byte-exact Latin-1 text,
    // which `EventPayloadFields::json` writes a `std::vector<uint8_t>` as.
    fn write_bytes(&self, held: &Held) -> String {
        match self.carrier {
            CppCarrier::Json => Self::json(&held.text),
            CppCarrier::Wire => format!(
                "ScriptValue(std::in_place_type<std::string>, ::SCE::Latin1Bytes::textOf({}))",
                held.text
            ),
        }
    }
    // The present value where there is one, and `null` where there is none,
    // as an absent scalar param is. The value is the argument of the lambda
    // that writes it, read once, so no binding inside it names a place
    // outside.
    fn write_optional(&self, held: &Held, elem: &str) -> String {
        let (result, absent) = match self.carrier {
            CppCarrier::Json => ("::std::string", "\"null\""),
            CppCarrier::Wire => ("ScriptValue", "ScriptValue(ScriptNull{})"),
        };
        format!(
            "[&](const auto &sceOptional) -> {result} {{ if (!sceOptional.has_value()) {{ return \
             {absent}; }} const auto &{CPP_ELEMENT_BINDING} = *sceOptional; return {elem}; }}({})",
            held.text
        )
    }
    // Each element written in the list's order.
    fn write_list(&self, held: &Held, elem: &str) -> String {
        match self.carrier {
            CppCarrier::Json => format!(
                "{CPP_COMMON}::EventPayloadFields::list({}, [&](const auto &{CPP_ELEMENT_BINDING}) \
                 -> ::std::string {{ return {elem}; }})",
                held.text
            ),
            CppCarrier::Wire => format!(
                "[&](const auto &sceList) -> ScriptValue {{ auto sceItems = \
                 ::std::make_shared<ScriptArray>(); for (const auto &{CPP_ELEMENT_BINDING} : \
                 sceList) {{ sceItems->elements.push_back({elem}); }} return \
                 ScriptValue(std::in_place_type<std::shared_ptr<ScriptArray>>, \
                 ::std::move(sceItems)); }}({})",
                held.text
            ),
        }
    }
    fn write_field(&self, held: &Held, field: &str) -> String {
        format!("{}.{field}", held.text)
    }
    // The runtime writes the keys ascending by their bytes at every depth
    // (ARCHITECTURE.md, "JSON Object Key Order"), `EventPayloadFields::record`
    // and the writer of a send's typed params alike.
    fn write_record(&self, fields: &[(String, String)]) -> String {
        let members: Vec<String> = fields
            .iter()
            .map(|(id, value)| {
                format!(
                    "{{\"{}\", {value}}}",
                    crate::filters::escape_cpp(id.clone())
                )
            })
            .collect();
        let members = members.join(", ");
        match self.carrier {
            CppCarrier::Json => format!("{CPP_COMMON}::EventPayloadFields::record({{{members}}})"),
            CppCarrier::Wire => format!(
                "ScriptValue(std::in_place_type<std::shared_ptr<ScriptObject>>, \
                 ::std::make_shared<ScriptObject>(ScriptObject{{{members}}}))"
            ),
        }
    }
    // The count held first, by the `sce_failure_` a `<send>` declares for a
    // param that can fail and reads after it, then the list written.
    fn send_counted(&self, held: &Held, _field: &str, least: u32, sent: String) -> Option<String> {
        Some(format!(
            "[&]() -> ScriptValue {{ if ({}.size() < {least}u) {{ \
             sce_failure_.fail(::SCE::Forge::AlgorithmError::Precondition); }} return {sent}; }}()",
            held.text
        ))
    }

    fn copy_binding(&self) -> &'static str {
        CPP_COPY_BINDING
    }
    // C++ copies by value: an assignment of a list, a record or an optional
    // one copies every element at every depth, so the variable it is copied
    // into, which is appended to and emptied in place, shares nothing with
    // the place the value was taken from.
    fn copy_whole(&self, held: &Held, _plain: bool) -> String {
        held.text.clone()
    }
    // The fit was proven, and the widening of the runtime's bounded type
    // asserts it again by its bounds, so a copy the build did not prove does
    // not compile (docs/adr/0086, section 7.4).
    fn copy_text(&self, max: u32, held: &Held) -> String {
        format!("{}({})", cpp_text_type(max), held.text)
    }
    fn copy_bytes(&self, max: u32, held: &Held) -> String {
        format!("{}({})", cpp_bytes_type(max), held.text)
    }
    // The value is the argument of the lambda that copies it, as a writer's
    // is.
    fn copy_optional(&self, held: &Held, elem: &str, elem_ty: &str) -> String {
        format!(
            "[&](const auto &sceOptional) -> std::optional<{elem_ty}> {{ if \
             (!sceOptional.has_value()) {{ return std::nullopt; }} const auto &{CPP_COPY_BINDING} \
             = *sceOptional; return {elem}; }}({})",
            held.text
        )
    }
    // A `T` converts to a `std::optional<T>` where one is expected.
    fn copy_present(&self, value: &str) -> String {
        value.to_string()
    }
    // Each element copied into the place's element type, in a list of the
    // copied list's own bound (`widenedMap`), which the place then takes by
    // widening, its own bound being at least that one (section 7.4).
    fn copy_list(&self, held: &Held, elem: &str, elem_ty: &str) -> String {
        format!(
            "{CPP_FORGE}::widenedMap<{elem_ty}>({}, [&](const auto &{CPP_COPY_BINDING}) -> \
             {elem_ty} {{ return {elem}; }})",
            held.text
        )
    }

    // A record that holds a composition carries its saved form in two
    // functions beside its struct, the names a record of entries' saved form
    // has (`<Type>ToSaved`, `<Type>FromSaved`, `StaticTarget::record_def`), so a
    // union's saved form reads one spelling for either: an object of its
    // fields keyed by their ids in the schema's order, each written and read
    // as its type says ([`cpp_saved_writer`], [`cpp_saved_reader`]), a list
    // held to its bound and to the lower count its type states.
    fn saved_record_ext(
        &self,
        alias: &str,
        records: &dyn RecordEnv,
        enums: &BTreeMap<String, MachineEnum>,
    ) -> Result<Option<String>, String> {
        let held = records.record(None, alias)?;
        let ty = self.record_type(alias);
        let mut writes = String::new();
        let mut reads = String::new();
        for field in &held.schema.fields {
            let key = cpp_literal(&field.id);
            let refused =
                |why: String| format!("the field `{}` of `{}`: {why}", field.id, held.schema.name);
            // The fields of the record name their types in its own document's scope.
            let within = Some(held.alias);
            let write = cpp_saved_writer(&field.sce_type, within, (records, enums), self, 0)
                .map_err(refused)?;
            let read = cpp_saved_reader(&field.sce_type, within, (records, enums), self, 0)
                .map_err(refused)?;
            writes.push_str(&format!(
                "    members.set({key}, ({write})(value.{}));\n",
                field.id
            ));
            reads.push_str(&format!(
                "        {},\n",
                crate::forge::generator::cpp_positional_member(
                    &field.id,
                    &format!(
                        "({read})(::SCE::Saved::savedField(value, what, {key}), what + \".{}\")",
                        crate::filters::escape_cpp(field.id.clone())
                    )
                )
            ));
        }
        Ok(Some(format!(
            "/// `value` as a saved state writes it.\ninline ::SCE::Saved::JsonValue {ty}ToSaved(const \
             {ty} &value) {{\n    ::SCE::Saved::JsonObject members;\n{writes}    return \
             ::SCE::Saved::JsonValue(std::move(members));\n}}\n\n/// The value a saved state \
             holds, refused unless it is one.\ninline {ty} {ty}FromSaved(const \
             ::SCE::Saved::JsonValue &value, const std::string &what) {{\n    return \
             {ty}{{\n{reads}    }};\n}}"
        )))
    }
    // The union's saved form is one object, its tag beside the fields of its
    // case (docs/adr/0092, section 6.1 item 7): written by the runtime's
    // `savedWriteUnion` over the record's own `<Record>ToSaved`, and read by
    // its tag first, which names the record the rest of the object is read
    // as; a tag the union has no case for is refused naming the union.
    fn saved_union_ext(
        &self,
        def: &UnionDef,
        key: &str,
        records: &dyn RecordEnv,
    ) -> Result<Option<String>, String> {
        let tag = cpp_literal(&records.union_of_key(key)?.union.tag);
        let ty = &def.ty;
        let mut writes = String::new();
        let mut reads = String::new();
        for case in &def.cases {
            let variant = cpp_literal(&case.variant);
            let alternative = format!("{ty}::{}", case.ident);
            match &case.record {
                Some(record) => {
                    writes.push_str(&format!(
                        "    if (const auto *held = ::std::get_if<{alternative}>(&value.\
                         {CPP_UNION_CASES})) {{\n        return \
                         ::SCE::Saved::savedWriteUnion({tag}, {variant}, \
                         {record}ToSaved(held->{CPP_CASE_VALUE}));\n    }}\n"
                    ));
                    reads.push_str(&format!(
                        "    if (tag == {variant}) {{\n        return \
                         {ty}{{{alternative}{{{record}FromSaved(value, what)}}}};\n    }}\n"
                    ));
                }
                None => {
                    writes.push_str(&format!(
                        "    if (::std::holds_alternative<{alternative}>(value.{CPP_UNION_CASES})) \
                         {{\n        return ::SCE::Saved::savedWriteUnion({tag}, {variant}, \
                         ::std::nullopt);\n    }}\n"
                    ));
                    reads.push_str(&format!(
                        "    if (tag == {variant}) {{\n        return {ty}{{{alternative}{{}}}};\n    \
                         }}\n"
                    ));
                }
            }
        }
        let name = crate::filters::escape_cpp(def.name.clone());
        Ok(Some(format!(
            "/// `value` as a saved state writes it: its tag beside the fields of its case.\ninline \
             ::SCE::Saved::JsonValue {ty}ToSaved(const {ty} &value) {{\n{writes}    \
             {CPP_UNREACHED}\n}}\n\n/// The value a saved state holds, read by its tag first, \
             refused unless it is one of the\n/// cases of {name}.\ninline {ty} {ty}FromSaved(const \
             ::SCE::Saved::JsonValue &value, const std::string &what) {{\n    const std::string tag \
             = ::SCE::Saved::savedReadUnionTag(value, what, {tag});\n{reads}    \
             ::SCE::Saved::detail::refuse(\"'\" + what + \"' (\" + tag + \") is not a case of \
             {name}\");\n}}"
        )))
    }

    // The machine's own struct, beside its records (docs/adr/0092, section 7,
    // the C++ row): named by the union's document, so one union is one type of
    // the machine whichever alias names it.
    fn union_type(&self, name: &str) -> String {
        format!(
            "{}{}Union",
            self.machine,
            crate::filters::to_pascal_case(name.to_string())
        )
    }
    // The struct of the case's alternative, nested in the union's (section 7
    // items 1 and 2).
    fn union_case_ident(&self, variant: &str) -> String {
        cpp_case_ident(variant)
    }
    // The tag is read by the one method every union of a C++ unit and of a C++
    // machine answers it by, whatever key it is written under.
    fn union_tag_read(&self, _tag: &str) -> Option<String> {
        Some(CPP_UNION_KIND.to_string())
    }
    fn union_case_read(&self, variant: &str) -> String {
        cpp_case_read(variant)
    }
    fn union_shared_read(&self, field: &str) -> String {
        cpp_shared_read(field)
    }
    // A struct of one `std::variant`, an alternative per case in the tag enum's
    // order, each a struct of its own holding the case's record, or empty (section
    // 7 item 2: a `std::variant` names an alternative by its type, and two cases
    // may hold one record). Its tag is the alternative's index, so the two
    // cannot disagree. Every read the narrowing proves is a method of it, beside
    // the method of the reference a write goes through
    // ([`Self::union_member_place`]), whose other cases end the process, the one
    // place they are written ([`CPP_UNREACHED`]).
    fn union_def(&self, def: &UnionDef) -> String {
        let name = crate::filters::escape_cpp(def.name.clone());
        let ty = &def.ty;
        let mut alternatives = String::new();
        let mut kinds = String::new();
        let mut reads = String::new();
        for (index, case) in def.cases.iter().enumerate() {
            let (ident, variant) = (&case.ident, &case.variant);
            kinds.push_str(&format!(
                "        case {index}:\n            return {}::{};\n",
                def.tag_ty, case.tag_ident
            ));
            let Some(record) = &case.record else {
                alternatives.push_str(&format!(
                    "    /// The case `{variant}`, its tag alone.\n    struct {ident} {{}};\n"
                ));
                continue;
            };
            alternatives.push_str(&format!(
                "    /// The case `{variant}`, holding its record.\n    struct {ident} {{\n        \
                 {record} {CPP_CASE_VALUE};\n    }};\n"
            ));
            let (read, place) = (&case.proven, cpp_place_of(&case.proven));
            reads.push_str(&format!(
                "\n    /// The record of the case `{variant}`, read where a test of the tag proved \
                 the value is in it.\n    const {record} &{read}() const {{\n        const {ident} \
                 *held = ::std::get_if<{ident}>(&{CPP_UNION_CASES});\n        if (held == nullptr) \
                 {{\n            {CPP_UNREACHED}\n        }}\n        return \
                 held->{CPP_CASE_VALUE};\n    }}\n\n    /// The record of the case `{variant}`, \
                 written where a test of the tag proved the value is in it.\n    {record} \
                 &{place}() {{\n        {ident} *held = ::std::get_if<{ident}>(&{CPP_UNION_CASES});\n\
                 \x20       if (held == nullptr) {{\n            {CPP_UNREACHED}\n        }}\n        \
                 return held->{CPP_CASE_VALUE};\n    }}\n"
            ));
        }
        for field in &def.shared {
            let arms = |constant: &str| -> String {
                field
                    .cases
                    .iter()
                    .map(|ident| {
                        format!(
                            "        if ({constant}auto *held = ::std::get_if<{ident}>(&\
                             {CPP_UNION_CASES})) {{\n            return \
                             held->{CPP_CASE_VALUE}.{};\n        }}\n",
                            field.field
                        )
                    })
                    .collect()
            };
            let (id, accessor, fty) = (&field.field, &field.accessor, &field.ty);
            let place = cpp_place_of(accessor);
            reads.push_str(&format!(
                "\n    /// The field `{id}` the cases that declare it share, read where a test of \
                 the tag proved\n    /// the value is in one of them.\n    {fty} {accessor}() \
                 const {{\n{}        {CPP_UNREACHED}\n    }}\n\n    /// The field `{id}` the cases \
                 that declare it share, written where a test of the tag\n    /// proved the value \
                 is in one of them.\n    {fty} &{place}() {{\n{}        {CPP_UNREACHED}\n    }}\n",
                arms("const "),
                arms(""),
            ));
        }
        let listed: Vec<&str> = def.cases.iter().map(|case| case.ident.as_str()).collect();
        format!(
            "/// SCE Accepted Subset §2.15: a `union:{name}` value (docs/adr/0092): exactly one \
             case, held\n/// as the alternative of its own struct, built only whole from it \
             (section 7 items 2 and 3).\nstruct {ty} {{\n{alternatives}\n    ::std::variant<{}> \
             {CPP_UNION_CASES};\n\n    /// The case this value is in, as its tag names it.\n    \
             {} {}() const {{\n        switch ({CPP_UNION_CASES}.index()) {{\n{kinds}        \
             }}\n        {CPP_UNREACHED}\n    }}\n{reads}}};",
            listed.join(", "),
            def.tag_ty,
            def.tag_method,
        )
    }
    // The case's alternative, handed the record or nothing, as the one member
    // of the union's struct.
    fn union_value(&self, ty: &str, case: &str, record: Option<&str>) -> String {
        format!("{ty}{{{ty}::{case}{{{}}}}}", record.unwrap_or_default())
    }
    // The tag first, by the enum's reader, in the runtime's `readUnion`, which
    // refuses a value that is no object before it; then the record of the case
    // the tag names, read from the same value into the case's alternative by
    // the arm of a `switch` over the tag's enum (docs/adr/0092, section 4). The
    // tag's reader answers only a variant the enum declares, so the `switch`
    // ends in no case left.
    fn read_union(&self, _value: &str, read: &UnionRead<'_>) -> String {
        let ty = &read.ty;
        let arms: String = read
            .arms
            .iter()
            .map(|arm| {
                let alternative = format!("{ty}::{}", arm.case_ident);
                match &arm.record {
                    Some(record) => format!(
                        "case {tag_ty}::{tag}: {{ {alternative} sceCase{{}}; \
                         {CPP_COMMON}::EventPayloadRefusal sceRefused = ({record})(sceValue, \
                         sceCase.{CPP_CASE_VALUE}); if (!sceRefused.empty()) {{ return sceRefused; \
                         }} sceInto = {ty}{{::std::move(sceCase)}}; return {{}}; }} ",
                        tag_ty = read.tag_ty,
                        tag = arm.tag_ident
                    ),
                    None => format!(
                        "case {}::{}: sceInto = {ty}{{{alternative}{{}}}}; return {{}}; ",
                        read.tag_ty, arm.tag_ident
                    ),
                }
            })
            .collect();
        format!(
            "{CPP_READER} {{ return sceValue.readUnion<{tag_ty}>({}, {}, [&](const {tag_ty} \
             &sceTag, const {CPP_COMMON}::EventPayloadValue &sceValue) -> \
             {CPP_COMMON}::EventPayloadRefusal {{ switch (sceTag) {{ {arms}}} {CPP_UNREACHED} \
             }}); }}",
            cpp_literal(read.tag),
            read.tag_reader,
            tag_ty = read.tag_ty,
        )
    }
    // One object, the tag among the fields of the case (docs/adr/0092, section
    // 2.1): the runtime's `EventPayloadFields::unionCase`, which orders the keys
    // as `record` does, or the `ScriptObject` a `<send>` holds a record in with
    // the tag beside its fields, whose keys the writer orders. The case is told
    // by its alternative, its record bound in its arm as a record's writer
    // reads it.
    fn write_union(&self, held: &Held, ty: &str, tag: &str, arms: &[UnionWriteArm]) -> String {
        let result = match self.carrier {
            CppCarrier::Json => "::std::string",
            CppCarrier::Wire => "ScriptValue",
        };
        let arms: String = arms
            .iter()
            .map(|arm| {
                let fields = arm.fields.clone().unwrap_or_default();
                let written = match self.carrier {
                    CppCarrier::Json => {
                        let pairs: Vec<String> = fields
                            .iter()
                            .map(|(id, json)| format!("{{{}, {json}}}", cpp_literal(id)))
                            .collect();
                        format!(
                            "{CPP_COMMON}::EventPayloadFields::unionCase({}, {}, {{{}}})",
                            cpp_literal(tag),
                            cpp_literal(&arm.variant),
                            pairs.join(", ")
                        )
                    }
                    CppCarrier::Wire => {
                        let mut members = vec![(
                            tag.to_string(),
                            format!(
                                "ScriptValue(std::in_place_type<std::string>, \
                                 ::std::string({}))",
                                cpp_literal(&arm.variant)
                            ),
                        )];
                        members.extend(fields);
                        self.write_record(&members)
                    }
                };
                let alternative = format!("{ty}::{}", arm.case_ident);
                match &arm.fields {
                    Some(_) => format!(
                        "if (const auto *sceCase = ::std::get_if<{alternative}>(&sceUnion.\
                         {CPP_UNION_CASES})) {{ const auto &{CPP_ELEMENT_BINDING} = \
                         sceCase->{CPP_CASE_VALUE}; return {written}; }} "
                    ),
                    None => format!(
                        "if (::std::holds_alternative<{alternative}>(sceUnion.{CPP_UNION_CASES})) \
                         {{ return {written}; }} "
                    ),
                }
            })
            .collect();
        format!(
            "[&](const auto &sceUnion) -> {result} {{ {arms}{CPP_UNREACHED} }}({})",
            held.text
        )
    }
    fn union_member(
        &self,
        union: &ScopedUnion,
        left: &[&ScopedCase],
        member: &str,
    ) -> Result<String, String> {
        cpp_union_member(union, left, member)
    }
    fn union_place_of(&self, read: &str) -> String {
        cpp_place_of(read)
    }
    // A field is written in place, through the reference the method beside its
    // proven read answers: the field of the one case left, or the field the
    // cases left share.
    fn union_member_place(
        &self,
        holder: &str,
        union: &ScopedUnion,
        left: &[&ScopedCase],
        member: &str,
    ) -> Result<String, String> {
        if member == union.tag {
            return Err(format!(
                "the tag `{member}` of `union:{}` is never written alone: the case changes by a \
                 whole assignment (docs/adr/0092, section 6.1 item 6)",
                union.name
            ));
        }
        // The read stands exactly where the write does: one case left that
        // declares the field, or several that share it.
        cpp_union_member(union, left, member)?;
        Ok(match left {
            [case] => format!(
                "{holder}.{}().{member}",
                cpp_place_of(&cpp_case_read(&case.variant.name))
            ),
            _ => format!("{holder}.{}()", cpp_place_of(&cpp_shared_read(member))),
        })
    }
}

/// The C++ function value that writes a value of `ty` as a saved state does (a
/// callable of the value answering `::SCE::Saved::JsonValue`): a number, a
/// truth value, a text and a byte string by the runtime's writer of its type,
/// an enum, a record and a union by their own `<Type>ToSaved`, a value that may
/// be absent as null or as its present value, and a list element by element.
/// `within` is the scope the type is written in, `depth` names the parameter of
/// each nested lambda, so none hides its holder's. `Err` for a byte string
/// inside a list, which a saved state would write as an array where it writes a
/// byte string as its text.
fn cpp_saved_writer(
    ty: &ValueType,
    within: Option<&str>,
    names: (&dyn RecordEnv, &BTreeMap<String, MachineEnum>),
    spelling: &CppComposed<'_>,
    depth: usize,
) -> Result<String, String> {
    let (records, enums) = names;
    let element = format!("sceSaved{depth}");
    Ok(match ty {
        ValueType::Scalar(SceType::Enum(_))
        | ValueType::Record { .. }
        | ValueType::Union { .. } => {
            format!(
                "{}ToSaved",
                machine_type(ty, within, records, enums, spelling)?
            )
        }
        ValueType::Scalar(scalar) => format!(
            "::SCE::Saved::savedWrite{}",
            crate::filters::to_pascal_case(scalar.as_attr())
        ),
        ValueType::Text { .. } => "::SCE::Saved::savedWriteString".to_string(),
        ValueType::Bytes { .. } => "::SCE::Saved::savedWriteBytes".to_string(),
        ValueType::Optional { elem } => format!(
            "[](const auto &{element}) {{ return ::SCE::Saved::savedWriteOptional({element}, {}); \
             }}",
            cpp_saved_writer(elem, within, names, spelling, depth + 1)?
        ),
        ValueType::List { elem, .. } => {
            if matches!(**elem, ValueType::Bytes { .. }) {
                return Err(
                    "a byte string inside a list, which a saved state has no text for".to_string(),
                );
            }
            format!(
                "[](const auto &{element}) {{ return ::SCE::Saved::savedWriteList({element}, {}); \
                 }}",
                cpp_saved_writer(elem, within, names, spelling, depth + 1)?
            )
        }
    })
}

/// The C++ function value that reads a saved value back as a value of `ty`, as
/// [`cpp_saved_writer`] writes it (a callable of the value and the name a
/// refusal gives it): a text and a byte string into the type that holds the
/// bound the type states, a list to its bound and to the lower count its type
/// states, an enum, a record and a union by their own `<Type>FromSaved`, a
/// value that may be absent read absent from null. Each refusal is the
/// runtime's `StateRefusal`. `within` and `depth` as [`cpp_saved_writer`]'s.
fn cpp_saved_reader(
    ty: &ValueType,
    within: Option<&str>,
    names: (&dyn RecordEnv, &BTreeMap<String, MachineEnum>),
    spelling: &CppComposed<'_>,
    depth: usize,
) -> Result<String, String> {
    let (records, enums) = names;
    let (saved, what) = (format!("sceSaved{depth}"), format!("sceWhat{depth}"));
    let head = format!("[](const ::SCE::Saved::JsonValue &{saved}, const std::string &{what})");
    Ok(match ty {
        ValueType::Scalar(SceType::Enum(_))
        | ValueType::Record { .. }
        | ValueType::Union { .. } => {
            format!(
                "{}FromSaved",
                machine_type(ty, within, records, enums, spelling)?
            )
        }
        ValueType::Scalar(scalar) => format!(
            "::SCE::Saved::savedRead{}",
            crate::filters::to_pascal_case(scalar.as_attr())
        ),
        ValueType::Text { max } => format!("::SCE::Saved::savedOwnedText<{}>()", max.max),
        ValueType::Bytes { max } => format!("::SCE::Saved::savedOwnedBytes<{}>()", max.max),
        ValueType::Optional { elem } => format!(
            "{head} {{ return ::SCE::Saved::savedReadOptional({saved}, {what}, {}); }}",
            cpp_saved_reader(elem, within, names, spelling, depth + 1)?
        ),
        ValueType::List { elem, max } => format!(
            "{head} {{ return ::SCE::Saved::savedReadOwnedListWithin<{}>({saved}, {what}, {}u, \
             {}); }}",
            max.max,
            max.min,
            cpp_saved_reader(elem, within, names, spelling, depth + 1)?
        ),
    })
}

#[cfg(test)]
mod tests {
    use super::super::test_support::*;
    use super::*;

    const CPP: CppComposed<'static> = CppComposed {
        machine: "M",
        carrier: CppCarrier::Json,
    };

    /// The C++ row of the one table of what each backend lowers of a composed
    /// value: a payload read at any depth, a list taken whole and a composed
    /// value sent (docs/adr/0086, sections 5 and 6), and a union held, read by
    /// its tag, lifted and saved (docs/adr/0092, section 7); no variable of a
    /// record that holds a composition yet.
    #[test]
    fn the_cpp_row_lowers_every_composed_position_but_a_composed_record_variable() {
        use crate::forge::event_schema_check::ComposedLowering;
        assert_eq!(
            ComposedLowering::of(crate::generator::Language::Cpp),
            ComposedLowering {
                name: "cpp",
                reads: true,
                copies: true,
                unions: true,
                records: false,
                sends: true,
            }
        );
    }

    /// The head of every C++ reader, as the tests below spell what they
    /// expect of one.
    const R: &str = "[&](const ::SCE::Common::EventPayloadValue &sceValue, auto &sceInto) -> \
                     ::SCE::Common::EventPayloadRefusal";

    /// The C++ row of section 7.1: a text, a byte string and a list are held
    /// in the runtime's type of each, its bound in the type (section 7.4), a
    /// value that may be absent is `std::optional` over its type, a record is
    /// the machine's struct, at any depth; no check is declared beside a
    /// record, whose members' types hold every bound.
    #[test]
    fn a_cpp_machine_holds_each_constructor_in_its_own_type_at_any_depth() {
        let held = held();
        let enums = enums();
        let spelled =
            |text: &str| machine_type(&ty(text), None, &held, &enums, &CPP).expect("a type");
        assert_eq!(spelled("uint16"), "uint16_t");
        assert_eq!(spelled("string[8]"), "::SCE::Forge::OwnedText<8>");
        assert_eq!(spelled("bytes[4]"), "::SCE::Forge::OwnedBytes<4>");
        assert_eq!(spelled("optional<int64>"), "std::optional<int64_t>");
        assert_eq!(
            spelled("list<list<uint8>[2]>[3]"),
            "::SCE::Forge::OwnedList<::SCE::Forge::OwnedList<uint8_t, 2>, 3>"
        );
        assert_eq!(
            spelled("list<enum:Level>[2]"),
            "::SCE::Forge::OwnedList<MLevelEnum, 2>"
        );
        assert_eq!(
            spelled("list<record:Item>[3]"),
            "::SCE::Forge::OwnedList<MItemRecord, 3>"
        );
        assert_eq!(
            spelled("optional<list<uint8>[2]>"),
            "std::optional<::SCE::Forge::OwnedList<uint8_t, 2>>"
        );
        assert_eq!(
            spelled("list<optional<uint16>>[2]"),
            "::SCE::Forge::OwnedList<std::optional<uint16_t>, 2>"
        );
        let members = record_members("Item", &held, &enums, &CPP).expect("members");
        let checks = record_checks("Item", &held, &CPP).expect("checks");
        assert!(checks.is_empty(), "{checks:?}");
        let def = CPP.record_def("MItemRecord", "Item", &members, &checks);
        assert!(
            def.ends_with(
                "struct MItemRecord {\n    uint32_t id;\n    MLevelEnum level;\n    \
                 std::optional<int64_t> due;\n    ::SCE::Forge::OwnedBytes<4> blob;\n    \
                 ::SCE::Forge::OwnedList<::SCE::Forge::OwnedText<8>, 4> tags;\n};"
            ),
            "{def}"
        );
    }

    /// One lambda of the value and the place per constructor, each calling
    /// the runtime's reader of its constructor, the reader of a part handed to
    /// the combinator of its holder; a bound and a lower count handed to the
    /// list that holds them, a record read field by field into its struct with
    /// its defaults.
    #[test]
    fn a_cpp_reader_is_one_lambda_per_constructor() {
        let held = held();
        let enums = enums();
        let read = |text: &str| reader(&ty(text), None, &held, &enums, &CPP, "v").expect("read");
        let whole = format!("{R} {{ return sceValue.readInteger(sceInto); }}");
        assert_eq!(read("uint8"), whole);
        assert_eq!(read("int16"), whole);
        let wide = format!("{R} {{ return sceValue.readWide(sceInto); }}");
        assert_eq!(read("uint64"), wide);
        assert_eq!(read("int64"), wide);
        assert_eq!(
            read("bool"),
            format!("{R} {{ return sceValue.readBool(sceInto); }}")
        );
        assert_eq!(
            read("float32"),
            format!("{R} {{ return sceValue.readFloating(sceInto); }}")
        );
        assert_eq!(
            read("string[8]"),
            format!("{R} {{ return sceValue.readText(8u, sceInto); }}")
        );
        assert_eq!(
            read("bytes[4]"),
            format!("{R} {{ return sceValue.readBytes(4u, sceInto); }}")
        );
        assert_eq!(
            read("optional<int64>"),
            format!("{R} {{ return sceValue.readOptional(sceInto, {wide}); }}")
        );
        assert_eq!(
            read("list<list<uint8>[2]>[3]"),
            format!(
                "{R} {{ return sceValue.readList(3u, sceInto, {R} {{ return \
                 sceValue.readList(2u, sceInto, {whole}); }}); }}"
            )
        );
        assert_eq!(
            read("list<list<uint8>[1..2]>[2..3]"),
            format!(
                "{R} {{ return sceValue.readListWithin(2u, 3u, sceInto, {R} {{ return \
                 sceValue.readListWithin(1u, 2u, sceInto, {whole}); }}); }}"
            )
        );
        assert_eq!(
            read("enum:Level"),
            format!(
                "{R} {{ ::std::string sceName; ::SCE::Common::EventPayloadRefusal sceNamed = \
                 sceValue.readName(sceName); if (!sceNamed.empty()) {{ return sceNamed; }} if \
                 (sceName == \"low\") {{ sceInto = MLevelEnum::Low; return {{}}; }} if (sceName \
                 == \"high\") {{ sceInto = MLevelEnum::High; return {{}}; }} return \
                 sceValue.unknownVariant(sceName, \"Level\"); }}"
            )
        );
        let item = read("record:Item");
        assert!(
            item.starts_with(&format!(
                "{R} {{ return sceValue.readRecord(nullptr, [&](const \
                 ::SCE::Common::EventPayloadFields &sceFields) -> \
                 ::SCE::Common::EventPayloadRefusal {{ ::SCE::Common::EventPayloadValue \
                 sceField; ::SCE::Common::EventPayloadRefusal sceRefused; sceRefused = \
                 sceFields.value(\"id\", sceField); if (!sceRefused.empty()) {{ return \
                 sceRefused; }} sceRefused = ({whole})(sceField, sceInto.id); if \
                 (!sceRefused.empty()) {{ return sceRefused; }} "
            )),
            "{item}"
        );
        assert!(
            item.ends_with(&format!(
                "sceRefused = ({R} {{ return sceValue.readList(4u, sceInto, {R} {{ return \
                 sceValue.readText(8u, sceInto); }}); }})(sceField, sceInto.tags); if \
                 (!sceRefused.empty()) {{ return sceRefused; }} return {{}}; }}); }}"
            )),
            "{item}"
        );
        // A list the second revision added reads empty where a delivery lacks it.
        let batch = read("record:Batch");
        assert!(
            batch.starts_with(&format!(
                "{R} {{ return sceValue.readRecord(\"{{\\\"late\\\":[]}}\", "
            )),
            "{batch}"
        );
    }

    /// The JSON a value is spelled as, one call of the runtime's writers per
    /// constructor: a list by `EventPayloadFields::list`, a value that may be
    /// absent handed to a lambda that writes `null` where it is absent, a
    /// record by its field ids, which the runtime orders.
    #[test]
    fn a_cpp_writer_is_one_runtime_call_per_constructor() {
        let held = held();
        let write =
            |text: &str, at: Held| writer(&ty(text), None, &held, &CPP, &at).expect("write");
        let j = "::SCE::Common::EventPayloadFields::json";
        assert_eq!(write("uint8", Held::place("x.n")), format!("{j}(x.n)"));
        assert_eq!(
            write("int64", Held::place("x.n")),
            format!("{j}(::std::to_string(x.n))")
        );
        assert_eq!(write("bytes[4]", Held::place("x.b")), format!("{j}(x.b)"));
        assert_eq!(
            write("enum:Level", Held::reference("sceElement")),
            format!("{j}(::std::string(sceLogName(sceElement)))")
        );
        assert_eq!(
            write("list<optional<uint16>>[2]", Held::place("x.maybe")),
            format!(
                "::SCE::Common::EventPayloadFields::list(x.maybe, [&](const auto &sceElement) -> \
                 ::std::string {{ return [&](const auto &sceOptional) -> ::std::string {{ if \
                 (!sceOptional.has_value()) {{ return \"null\"; }} const auto &sceElement = \
                 *sceOptional; return {j}(sceElement); }}(sceElement); }})"
            )
        );
        let items = write("list<record:Item>[3]", Held::place("x.items"));
        for part in [
            "::SCE::Common::EventPayloadFields::list(x.items, [&](const auto &sceElement) -> \
             ::std::string { return ::SCE::Common::EventPayloadFields::record({{\"id\", "
                .to_string(),
            format!("{{\"id\", {j}(sceElement.id)}}, "),
            format!("{{\"level\", {j}(::std::string(sceLogName(sceElement.level)))}}, "),
            format!(
                "{{\"due\", [&](const auto &sceOptional) -> ::std::string {{ if \
                 (!sceOptional.has_value()) {{ return \"null\"; }} const auto &sceElement = \
                 *sceOptional; return {j}(::std::to_string(sceElement)); }}(sceElement.due)}}, "
            ),
            format!("{{\"blob\", {j}(sceElement.blob)}}, "),
            format!(
                "{{\"tags\", ::SCE::Common::EventPayloadFields::list(sceElement.tags, [&](const \
                 auto &sceElement) -> ::std::string {{ return {j}(sceElement); }})}}}}); }})"
            ),
        ] {
            assert!(items.contains(&part), "expected `{part}` in:\n{items}");
        }
    }

    /// The value a C++ `<send>` holds a composed param in: a `ScriptValue`
    /// holding a `ScriptArray`, a `ScriptObject` whose keys the writer orders,
    /// `ScriptNull` where a value is absent, and each scalar as a scalar param
    /// is (a narrow integer widened to `int64_t`, a 64-bit one as its digits, a
    /// real as a `double`, a byte string as its Latin-1 text, an enum value as
    /// its declared name).
    #[test]
    fn a_cpp_send_writes_each_constructor_into_the_value_it_holds_params_in() {
        let held = held();
        let wire = CppComposed {
            machine: "M",
            carrier: CppCarrier::Wire,
        };
        let write =
            |text: &str, at: Held| writer(&ty(text), None, &held, &wire, &at).expect("write");
        assert_eq!(
            write("uint8", Held::place("x.n")),
            "ScriptValue(std::in_place_type<int64_t>, static_cast<int64_t>(x.n))"
        );
        assert_eq!(
            write("int64", Held::place("x.n")),
            "ScriptValue(std::in_place_type<std::string>, std::to_string(x.n))"
        );
        assert_eq!(
            write("float32", Held::place("x.n")),
            "ScriptValue(std::in_place_type<double>, static_cast<double>(x.n))"
        );
        assert_eq!(
            write("bool", Held::place("x.b")),
            "ScriptValue(std::in_place_type<bool>, x.b)"
        );
        let inner = "[&](const auto &sceList) -> ScriptValue { auto sceItems = \
                     ::std::make_shared<ScriptArray>(); for (const auto &sceElement : sceList) { \
                     sceItems->elements.push_back(ScriptValue(std::in_place_type<int64_t>, \
                     static_cast<int64_t>(sceElement))); } return \
                     ScriptValue(std::in_place_type<std::shared_ptr<ScriptArray>>, \
                     ::std::move(sceItems)); }(sceElement)";
        assert_eq!(
            write("list<list<uint8>[2]>[2]", Held::place("x.grid")),
            format!(
                "[&](const auto &sceList) -> ScriptValue {{ auto sceItems = \
                 ::std::make_shared<ScriptArray>(); for (const auto &sceElement : sceList) {{ \
                 sceItems->elements.push_back({inner}); }} return \
                 ScriptValue(std::in_place_type<std::shared_ptr<ScriptArray>>, \
                 ::std::move(sceItems)); }}(x.grid)"
            )
        );
        assert_eq!(
            write("optional<list<uint8>[2]>", Held::place("x.spare")),
            format!(
                "[&](const auto &sceOptional) -> ScriptValue {{ if (!sceOptional.has_value()) {{ \
                 return ScriptValue(ScriptNull{{}}); }} const auto &sceElement = *sceOptional; \
                 return {inner}; }}(x.spare)"
            )
        );
        let items = write("list<record:Item>[3]", Held::place("x.items"));
        for part in [
            "sceItems->elements.push_back(ScriptValue(std::in_place_type<std::shared_ptr<\
             ScriptObject>>, ::std::make_shared<ScriptObject>(ScriptObject{{\"id\", \
             ScriptValue(std::in_place_type<int64_t>, static_cast<int64_t>(sceElement.id))}, ",
            "{\"level\", ScriptValue(std::in_place_type<std::string>, \
             ::std::string(sceLogName(sceElement.level)))}, ",
            "{\"blob\", ScriptValue(std::in_place_type<std::string>, \
             ::SCE::Latin1Bytes::textOf(sceElement.blob))}, ",
        ] {
            assert!(items.contains(part), "expected `{part}` in:\n{items}");
        }
        // The writers write the `ScriptValue` a send holds its params in.
        assert_eq!(wire.send_param("x".to_string()), "x");
        // A list sent where its field asks a lower count it does not prove is
        // held to the count before it is written (docs/adr/0086, section 15.6).
        assert_eq!(
            wire.send_counted(&Held::place("taking_"), "ops", 1, "W".to_string()),
            Some(
                "[&]() -> ScriptValue { if (taking_.size() < 1u) { \
                 sce_failure_.fail(::SCE::Forge::AlgorithmError::Precondition); } return W; }()"
                    .to_string()
            )
        );
    }

    /// Two places of one type are copied by the assignment, which copies a
    /// list by value at every depth; where the bounds differ, a list is mapped
    /// element by element into the place's element type and widened into the
    /// place's bound, and a text is widened, the runtime's types asserting the
    /// fit the build proved (docs/adr/0086, section 7.4).
    #[test]
    fn a_cpp_copy_of_a_value_that_fits_widens_into_the_place() {
        let held = held();
        let enums = enums();
        let copy = |from: &str, to: &str| {
            copier(
                &ty(from),
                None,
                &ty(to),
                None,
                &held,
                &enums,
                &CPP,
                &Held::place("x"),
            )
        };
        assert_eq!(
            copy("list<uint8>[2]", "list<uint8>[2]"),
            Ok("x".to_string())
        );
        assert_eq!(
            copy("list<uint8>[2]", "list<uint8>[4]"),
            Ok(
                "::SCE::Forge::widenedMap<uint8_t>(x, [&](const auto &sceCopied) -> uint8_t { \
                 return sceCopied; })"
                    .to_string()
            )
        );
        assert_eq!(
            copy("list<string[4]>[2]", "list<string[8]>[3]"),
            Ok(
                "::SCE::Forge::widenedMap<::SCE::Forge::OwnedText<8>>(x, [&](const auto \
                 &sceCopied) -> ::SCE::Forge::OwnedText<8> { return \
                 ::SCE::Forge::OwnedText<8>(sceCopied); })"
                    .to_string()
            )
        );
        assert_eq!(
            copy("list<record:Item>[2]", "list<record:Item>[3]"),
            Ok(
                "::SCE::Forge::widenedMap<MItemRecord>(x, [&](const auto &sceCopied) -> \
                 MItemRecord { return sceCopied; })"
                    .to_string()
            )
        );
        assert_eq!(
            copy("list<uint8>[2]", "list<optional<uint8>>[2]"),
            Ok(
                "::SCE::Forge::widenedMap<std::optional<uint8_t>>(x, [&](const auto &sceCopied) \
                 -> std::optional<uint8_t> { return sceCopied; })"
                    .to_string()
            )
        );
    }

    /// A C++ raise seam checks of a host's value only what no type holds, a
    /// list's lower count (docs/adr/0086, sections 5.4 and 7.4): one check per
    /// list that states one, an element's or a present value's inside the
    /// lambda the runtime hands it to, each parameter named by its depth, and
    /// a list without one walked only for an element that holds one. A text,
    /// a byte string and a list's upper bound are their types', so a payload
    /// of no lower count is not checked at all.
    #[test]
    fn a_cpp_raise_seam_checks_each_lower_count_before_it_encodes() {
        let held = held();
        let check = |text: &str, id: &str| {
            cpp_bound_check(
                &ty(text),
                None,
                &format!("::std::string(\"{id}\")"),
                id,
                0,
                &held,
            )
            .expect("a check")
        };
        let b = "::SCE::Common::EventPayloadBounds";
        let nothing = "[](const ::std::string &, const auto &) -> ::std::string { return {}; }";
        assert_eq!(check("uint8", "n"), None);
        assert_eq!(check("enum:Level", "level"), None);
        assert_eq!(check("optional<int64>", "due"), None);
        assert_eq!(check("string[4]", "note"), None);
        assert_eq!(check("bytes[4]", "blob"), None);
        assert_eq!(check("list<list<string[4]>[2]>[3]", "grid"), None);
        assert_eq!(
            check("list<list<uint8>[2]>[2..3]", "grid"),
            Some(format!(
                "{b}::listWithin(::std::string(\"grid\"), grid, 2u, 3u, {nothing})"
            ))
        );
        assert_eq!(
            check("list<list<uint8>[1..2]>[3]", "grid"),
            Some(format!(
                "{b}::list(::std::string(\"grid\"), grid, 3u, [&](const ::std::string \
                 &scePath0, const auto &sceValue0) -> ::std::string {{ return \
                 {b}::listWithin(scePath0, sceValue0, 1u, 2u, {nothing}); }})"
            ))
        );
        assert_eq!(
            check("optional<list<string[4]>[1..2]>", "spare"),
            Some(format!(
                "{b}::optional(::std::string(\"spare\"), spare, [&](const ::std::string \
                 &scePath0, const auto &sceValue0) -> ::std::string {{ return \
                 {b}::listWithin(scePath0, sceValue0, 1u, 2u, {nothing}); }})"
            ))
        );
        let batch = held.record(None, "Batch").expect("Batch");
        assert_eq!(
            cpp_raise_checks(batch, &held).expect("checks"),
            Vec::<String>::new(),
            "every bound of the batch is its types'"
        );
    }

    /// The case of the shape union named `name`.
    fn case<'s>(shapes: &'s Shapes, name: &str) -> &'s ScopedCase {
        shapes
            .union
            .cases
            .iter()
            .find(|case| case.variant.name == name)
            .expect("a case")
    }

    /// A union is the machine's own struct at any depth, named by its document
    /// (docs/adr/0092, section 7, the C++ row), its tag read by `kind()`
    /// whatever key it is written under, and a value built whole from the
    /// alternative of its case.
    #[test]
    fn a_cpp_machine_holds_a_union_in_its_own_struct() {
        let shapes = shapes();
        let kinds = kinds();
        assert_eq!(
            machine_type(&ty("union:Shape"), None, &shapes, &kinds, &CPP),
            Ok("MShapeUnion".to_string())
        );
        assert_eq!(
            machine_type(
                &ty("list<optional<union:Shape>>[3]"),
                None,
                &shapes,
                &kinds,
                &CPP
            ),
            Ok("::SCE::Forge::OwnedList<std::optional<MShapeUnion>, 3>".to_string())
        );
        assert_eq!(CPP.union_tag_read("type"), Some("kind".to_string()));
        assert_eq!(
            CPP.union_value("MShapeUnion", "Dot", None),
            "MShapeUnion{MShapeUnion::Dot{}}"
        );
        assert_eq!(
            CPP.union_value("MShapeUnion", "Circle", Some("r")),
            "MShapeUnion{MShapeUnion::Circle{r}}"
        );
    }

    /// The declaration: one alternative struct per case in the tag enum's
    /// order, the record of a case its `value`, an empty case empty; the
    /// `std::variant` of them; `kind()` from the alternative's index; a proven
    /// read per case that holds a record beside the reference a write goes
    /// through, each ending the process in another case; and one accessor for
    /// `radius`, which `circle` and `disc` share.
    #[test]
    fn a_cpp_union_is_declared_with_its_alternatives_and_its_proven_reads() {
        let def = union_definition(SHAPE_KEY, &shapes(), &kinds(), &CPP).expect("declared");
        assert_eq!(def.ty, "MShapeUnion");
        assert_eq!(def.tag_method, "kind");
        assert_eq!(
            def.shared,
            vec![SharedFieldDef {
                field: "radius".to_string(),
                accessor: "provenFieldRadius".to_string(),
                ty: "uint16_t".to_string(),
                cases: vec!["Circle".to_string(), "Disc".to_string()],
            }]
        );
        let text = CPP.union_def(&def);
        for fragment in [
            "struct MShapeUnion {\n    /// The case `dot`, its tag alone.\n    struct Dot {};\n",
            "    struct Circle {\n        MRadiusRecord value;\n    };\n",
            "\n    ::std::variant<Dot, Circle, Disc> cases;\n",
            "    MKindEnum kind() const {\n        switch (cases.index()) {\n        case 0:\n            \
             return MKindEnum::Dot;\n        case 1:\n            return MKindEnum::Circle;\n",
            "    const MRadiusRecord &provenCircle() const {\n        const Circle *held = \
             ::std::get_if<Circle>(&cases);\n        if (held == nullptr) {\n            \
             ::std::abort();\n        }\n        return held->value;\n    }\n",
            "    MRadiusRecord &provenCirclePlace() {\n        Circle *held = \
             ::std::get_if<Circle>(&cases);\n        if (held == nullptr) {\n",
            "    uint16_t provenFieldRadius() const {\n        if (const auto *held = \
             ::std::get_if<Circle>(&cases)) {\n            return held->value.radius;\n        }\n",
            "    uint16_t &provenFieldRadiusPlace() {\n        if (auto *held = \
             ::std::get_if<Circle>(&cases)) {\n            return held->value.radius;\n        }\n",
        ] {
            assert!(text.contains(fragment), "expected `{fragment}` in:\n{text}");
        }
        // An empty case holds no record and has nothing to read.
        assert!(!text.contains("provenDot"), "{text}");
    }

    /// A read of a union's member (docs/adr/0092, section 6.1): the tag by its
    /// method, a field of the one case left through that case's proven read,
    /// and a field two cases share through its accessor; a write goes through
    /// the reference the method beside each answers, and the tag is never
    /// written alone.
    #[test]
    fn a_cpp_proven_read_and_write_are_spelled_by_the_cases_left() {
        let shapes = shapes();
        let union = &shapes.union;
        let all: Vec<&ScopedCase> = union.cases.iter().collect();
        let both = [case(&shapes, "circle"), case(&shapes, "disc")];
        assert_eq!(cpp_union_member(union, &all, "kind"), Ok("kind()".into()));
        assert_eq!(
            cpp_union_member(union, &[case(&shapes, "disc")], "radius"),
            Ok("provenDisc().radius".into())
        );
        assert_eq!(
            CPP.union_member(union, &both, "radius"),
            Ok("provenFieldRadius()".into())
        );
        assert!(cpp_union_member(union, &[case(&shapes, "dot")], "radius").is_err());
        assert_eq!(
            CPP.union_member_place("v_s", union, &[case(&shapes, "disc")], "radius"),
            Ok("v_s.provenDiscPlace().radius".into())
        );
        assert_eq!(
            CPP.union_member_place("v_s", union, &both, "radius"),
            Ok("v_s.provenFieldRadiusPlace()".into())
        );
        let tag = CPP
            .union_member_place("v_s", union, &all, "kind")
            .expect_err("the tag is never written alone");
        assert!(tag.contains("never written alone"), "{tag}");
    }

    /// An algorithm's union is its unit's struct (docs/adr/0092, Landing order
    /// step 5): the tag by `kind()`, the field of the one case left by that
    /// case's proven read, a field several cases declare by a lambda that takes
    /// each case left apart by its alternative and ends the process past them,
    /// and no read where no case left declares the field.
    #[test]
    fn an_algorithms_union_is_read_on_its_units_struct() {
        let shapes = shapes();
        let union = &shapes.union;
        let read = |left: &[&ScopedCase], member: &str| {
            cpp_unit_union_read("shape", "U", union, left, member)
        };
        let both = [case(&shapes, "circle"), case(&shapes, "disc")];
        assert_eq!(read(&[], "kind"), Ok("shape.kind()".to_string()));
        assert_eq!(
            read(&[case(&shapes, "circle")], "radius"),
            Ok("shape.provenCircle().radius".to_string())
        );
        assert_eq!(
            read(&both, "radius"),
            Ok(
                "[&]() { if (const auto *sceCase = ::std::get_if<U::Circle>(&shape.cases)) { \
                 return sceCase->value.radius; } if (const auto *sceCase = \
                 ::std::get_if<U::Disc>(&shape.cases)) { return sceCase->value.radius; } \
                 ::std::abort(); }()"
                    .to_string()
            )
        );
        assert!(read(&[], "radius").is_err());
        assert!(read(&[case(&shapes, "dot"), case(&shapes, "circle")], "radius").is_err());
    }

    /// The reader reads the tag by the enum's reader inside the runtime's
    /// `readUnion`, and then the record of the case it names from the same
    /// value into the case's alternative, each case an arm of a `switch` over
    /// the tag's enum; the writer tells the case by its alternative and writes
    /// the tag among the case's fields, as the JSON text and as the value a
    /// `<send>` holds; a union is copied by the assignment.
    #[test]
    fn a_cpp_union_is_read_tag_first_written_by_its_case_and_copied_by_value() {
        let shapes = shapes();
        let kinds = kinds();
        let read = reader(&ty("union:Shape"), None, &shapes, &kinds, &CPP, "v").expect("read");
        for fragment in [
            format!("{R} {{ return sceValue.readUnion<MKindEnum>(\"kind\", {R} {{ "),
            "[&](const MKindEnum &sceTag, const ::SCE::Common::EventPayloadValue &sceValue) -> \
             ::SCE::Common::EventPayloadRefusal { switch (sceTag) { case MKindEnum::Dot: sceInto \
             = MShapeUnion{MShapeUnion::Dot{}}; return {}; case MKindEnum::Circle: { \
             MShapeUnion::Circle sceCase{}; ::SCE::Common::EventPayloadRefusal sceRefused = "
                .to_string(),
            "(sceValue, sceCase.value); if (!sceRefused.empty()) { return sceRefused; } sceInto = \
             MShapeUnion{::std::move(sceCase)}; return {}; } "
                .to_string(),
            "} ::std::abort(); }); }".to_string(),
        ] {
            assert!(
                read.contains(&fragment),
                "expected `{fragment}` in:\n{read}"
            );
        }
        let json =
            writer(&ty("union:Shape"), None, &shapes, &CPP, &Held::place("x.s")).expect("write");
        assert_eq!(
            json,
            "[&](const auto &sceUnion) -> ::std::string { if \
             (::std::holds_alternative<MShapeUnion::Dot>(sceUnion.cases)) { return \
             ::SCE::Common::EventPayloadFields::unionCase(\"kind\", \"dot\", {}); } if (const auto \
             *sceCase = ::std::get_if<MShapeUnion::Circle>(&sceUnion.cases)) { const auto \
             &sceElement = sceCase->value; return \
             ::SCE::Common::EventPayloadFields::unionCase(\"kind\", \"circle\", {{\"radius\", \
             ::SCE::Common::EventPayloadFields::json(sceElement.radius)}}); } if (const auto \
             *sceCase = ::std::get_if<MShapeUnion::Disc>(&sceUnion.cases)) { const auto \
             &sceElement = sceCase->value; return \
             ::SCE::Common::EventPayloadFields::unionCase(\"kind\", \"disc\", {{\"radius\", \
             ::SCE::Common::EventPayloadFields::json(sceElement.radius)}}); } ::std::abort(); \
             }(x.s)"
        );
        let wire = CppComposed {
            machine: "M",
            carrier: CppCarrier::Wire,
        };
        let sent = writer(
            &ty("union:Shape"),
            None,
            &shapes,
            &wire,
            &Held::place("x.s"),
        )
        .expect("write");
        assert!(
            sent.contains(
                "ScriptObject{{\"kind\", ScriptValue(std::in_place_type<std::string>, \
                 ::std::string(\"circle\"))}, {\"radius\", \
                 ScriptValue(std::in_place_type<int64_t>, \
                 static_cast<int64_t>(sceElement.radius))}}"
            ),
            "{sent}"
        );
        let copy = |from: &str, to: &str| {
            copier(
                &ty(from),
                None,
                &ty(to),
                None,
                &shapes,
                &kinds,
                &CPP,
                &Held::place("x"),
            )
        };
        assert_eq!(copy("union:Shape", "union:Shape"), Ok("x".to_string()));
        assert_eq!(
            copy("optional<union:Shape>", "optional<union:Shape>"),
            Ok("x".to_string())
        );
        assert_eq!(
            copy("union:Shape", "optional<union:Shape>"),
            Ok("x".to_string())
        );
    }

    /// A union is saved as its tag beside the fields of its case, written by
    /// the runtime's `savedWriteUnion` over the record's own `ToSaved`, and
    /// read back by its tag first (docs/adr/0092, section 6.1 item 7); a tag no
    /// case names is refused naming the union.
    #[test]
    fn a_cpp_union_is_saved_by_its_tag_beside_its_case() {
        let shapes = shapes();
        let def = union_definition(SHAPE_KEY, &shapes, &kinds(), &CPP).expect("declared");
        let text = CPP
            .saved_union_ext(&def, SHAPE_KEY, &shapes)
            .expect("spelled")
            .expect("a C++ union carries its saved form");
        for fragment in [
            "inline ::SCE::Saved::JsonValue MShapeUnionToSaved(const MShapeUnion &value) {\n    if \
             (::std::holds_alternative<MShapeUnion::Dot>(value.cases)) {\n        return \
             ::SCE::Saved::savedWriteUnion(\"kind\", \"dot\", ::std::nullopt);\n    }\n    if \
             (const auto *held = ::std::get_if<MShapeUnion::Circle>(&value.cases)) {\n        \
             return ::SCE::Saved::savedWriteUnion(\"kind\", \"circle\", \
             MRadiusRecordToSaved(held->value));\n    }\n",
            "inline MShapeUnion MShapeUnionFromSaved(const ::SCE::Saved::JsonValue &value, const \
             std::string &what) {\n    const std::string tag = \
             ::SCE::Saved::savedReadUnionTag(value, what, \"kind\");\n",
            "    if (tag == \"dot\") {\n        return MShapeUnion{MShapeUnion::Dot{}};\n    }\n",
            "    if (tag == \"circle\") {\n        return \
             MShapeUnion{MShapeUnion::Circle{MRadiusRecordFromSaved(value, what)}};\n    }\n",
            "::SCE::Saved::detail::refuse(\"'\" + what + \"' (\" + tag + \") is not a case of \
             shape\");",
        ] {
            assert!(text.contains(fragment), "expected `{fragment}` in:\n{text}");
        }
    }

    /// A composed record's saved form: each field written by the runtime's
    /// writer of its type and read back into the type that holds every bound
    /// its type states, a list's lower count included (docs/adr/0086, section
    /// 15.6), by position in the schema's order.
    #[test]
    fn a_cpp_composed_record_is_saved_field_by_field_by_its_types() {
        let held = held();
        let enums = enums();
        let text = CPP
            .saved_record_ext("Item", &held, &enums)
            .expect("spelled")
            .expect("a C++ composed record carries its saved form");
        for fragment in [
            "inline ::SCE::Saved::JsonValue MItemRecordToSaved(const MItemRecord &value) {\n    \
             ::SCE::Saved::JsonObject members;\n",
            "    members.set(\"id\", (::SCE::Saved::savedWriteUint32)(value.id));\n",
            "    members.set(\"level\", (MLevelEnumToSaved)(value.level));\n",
            "    members.set(\"due\", ([](const auto &sceSaved0) { return \
             ::SCE::Saved::savedWriteOptional(sceSaved0, ::SCE::Saved::savedWriteInt64); \
             })(value.due));\n",
            "inline MItemRecord MItemRecordFromSaved(const ::SCE::Saved::JsonValue &value, const \
             std::string &what) {\n    return MItemRecord{\n",
            "        /*blob=*/(::SCE::Saved::savedOwnedBytes<4>())(::SCE::Saved::savedField(value, \
             what, \"blob\"), what + \".blob\"),\n",
            "        /*tags=*/([](const ::SCE::Saved::JsonValue &sceSaved0, const std::string \
             &sceWhat0) { return ::SCE::Saved::savedReadOwnedListWithin<4>(sceSaved0, sceWhat0, \
             0u, ::SCE::Saved::savedOwnedText<8>()); })(::SCE::Saved::savedField(value, what, \
             \"tags\"), what + \".tags\"),\n",
        ] {
            assert!(text.contains(fragment), "expected `{fragment}` in:\n{text}");
        }
    }

    /// A union a raise seam is handed is checked by the record of the case it
    /// is in, at the union's own path; one whose cases hold no lower count
    /// checks nothing, and a union the statechart does not hold is refused.
    #[test]
    fn a_cpp_raise_seam_checks_a_union_by_its_case() {
        let shapes = shapes();
        assert_eq!(
            cpp_bound_check(&ty("union:Shape"), None, "path", "one", 0, &shapes),
            Ok(None)
        );
        assert!(cpp_bound_check(&ty("union:Other"), None, "path", "one", 0, &shapes).is_err());
    }
}
