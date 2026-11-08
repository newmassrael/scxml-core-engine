// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! The C++ spelling of the walks over a composed value ([`super`])
//! (docs/adr/0086, section 7.1, the C++ row), and the checks a C++ raise seam
//! runs over a host's value before it encodes it (section 5.4).
//!
//! | `list<T>[N]` | `string[N]` / `bytes[N]` | `optional<T>` | record |
//! |---|---|---|---|
//! | `std::vector<T>` | `std::string` / `std::vector<uint8_t>` | `std::optional<T>` | the machine's struct |
//!
//! No C++ type carries a bound yet (section 7.4, last paragraph), so two
//! places whose types differ only in their bounds are one type, and a value
//! that fits is copied as it is, by the assignment, which copies a list by
//! value at every depth.

use super::*;

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

/// Why a C++ walk meets no union: the one decision of what is lowered refuses
/// a union on C++ before any walk reaches one
/// ([`crate::forge::event_schema_check::ComposedLowering::unions`]).
const CPP_HOLDS_NO_UNION: &str = "a C++ machine holds no union yet (docs/adr/0092, section 7): \
     the one decision of what is lowered refuses a union on C++ before any walk reaches one";

/// The check of a C++ value of `ty`, written in the scope `within` names, at
/// `value`, named `path` (an expression of a `std::string`): an expression of
/// type `std::string` that is the refusal, naming the path to the place, of a
/// value past a bound its type declares at any depth, and empty where there
/// is none (docs/adr/0086, section 5.4). One call of the runtime's
/// `EventPayloadBounds::text`, `bytes`, `list` or `listWithin` per
/// constructor that holds a bound, and of `optional` for a value that may be
/// absent, the check of an element or a present value inside the lambda the
/// call hands it to, its parameters named by their `depth`; a record is
/// checked field by field where it stands, at the path of each field below
/// its own. `None` for a type that holds no bound at any depth: a number, an
/// enum, an optional one of them and a record of those.
///
/// A C++ struct has no constructor that could refuse a value, so a C++ raise
/// seam runs this check over the payload a host hands it before it encodes
/// it, a record a list holds included.
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
        ValueType::Text { max } => Some(format!("{bounds}::text({path}, {value}, {}u)", max.max)),
        ValueType::Bytes { max } => Some(format!("{bounds}::bytes({path}, {value}, {}u)", max.max)),
        // The count is held whatever the elements are; an element with
        // nothing of its own to check is handed to a lambda that checks
        // nothing.
        ValueType::List { elem, max } => {
            let each =
                match cpp_bound_check(elem, within, &inner_path, &inner_value, depth + 1, records)?
                {
                    Some(inner) => part(&inner),
                    None => {
                        "[](const ::std::string &, const auto &) -> ::std::string { return {}; }"
                            .to_string()
                    }
                };
            Some(if max.min == 0 {
                format!("{bounds}::list({path}, {value}, {}u, {each})", max.max)
            } else {
                format!(
                    "{bounds}::listWithin({path}, {value}, {}u, {}u, {each})",
                    max.min, max.max
                )
            })
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
        ValueType::Union { .. } => return Err(CPP_HOLDS_NO_UNION.to_string()),
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
    // The C++ row of section 7.1: no type has a bound as its parameter yet
    // (section 7.4, last paragraph), so the reader holds a value to its bound
    // where it is read, and a raise seam where a host hands it one.
    fn text_type(&self, _max: u32) -> String {
        "std::string".to_string()
    }
    fn bytes_type(&self, _max: u32) -> String {
        "std::vector<uint8_t>".to_string()
    }
    // The one C++ spelling of a value that may be absent (docs/adr/0055,
    // section 5).
    fn optional_type(&self, elem: &str) -> String {
        format!("std::optional<{elem}>")
    }
    fn list_type(&self, elem: &str, _max: u32) -> String {
        format!("std::vector<{elem}>")
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
    // (docs/adr/0059, section 5). A C++ struct has no constructor that could
    // refuse a value, so the raise seam that takes one holds it to every bound
    // its type declares before it sends it (docs/adr/0086, section 5.4), and
    // no check is declared beside it.
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
             /// value (docs/adr/0086), built by position in the schema's order; a raise\n\
             /// seam holds it to every bound its type declares before it sends it.\n\
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
    // A text and a byte string are one C++ type whatever their bound, so a
    // walk that copies by type never reaches these; the value is the copy.
    fn copy_text(&self, _max: u32, held: &Held) -> String {
        held.text.clone()
    }
    fn copy_bytes(&self, _max: u32, held: &Held) -> String {
        held.text.clone()
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
    fn copy_list(&self, held: &Held, elem: &str, elem_ty: &str) -> String {
        format!(
            "[&](const auto &sceList) -> std::vector<{elem_ty}> {{ std::vector<{elem_ty}> \
             sceItems; sceItems.reserve(sceList.size()); for (const auto &{CPP_COPY_BINDING} : \
             sceList) {{ sceItems.push_back({elem}); }} return sceItems; }}({})",
            held.text
        )
    }

    // A C++ machine holds no union yet: the gate refuses one by name before a
    // walk reaches it, so each spelling below either names what it would be or
    // says why there is none.
    fn union_type(&self, name: &str) -> String {
        format!(
            "{}{}Union",
            self.machine,
            crate::filters::to_pascal_case(name.to_string())
        )
    }
    fn union_case_ident(&self, variant: &str) -> String {
        crate::forge::enum_naming::variant_ident(crate::generator::Language::Cpp, "", variant)
    }
    fn union_tag_read(&self, _tag: &str) -> Option<String> {
        None
    }
    fn union_case_read(&self, variant: &str) -> String {
        format!(
            "proven{}",
            crate::filters::to_pascal_case(variant.to_string())
        )
    }
    fn union_shared_read(&self, field: &str) -> String {
        format!(
            "provenField{}",
            crate::filters::to_pascal_case(field.to_string())
        )
    }
    fn union_def(&self, _def: &UnionDef) -> String {
        unreachable!("{CPP_HOLDS_NO_UNION}")
    }
    fn union_value(&self, _ty: &str, _case: &str, _record: Option<&str>) -> String {
        unreachable!("{CPP_HOLDS_NO_UNION}")
    }
    fn read_union(&self, _value: &str, _read: &UnionRead<'_>) -> String {
        unreachable!("{CPP_HOLDS_NO_UNION}")
    }
    fn write_union(&self, _held: &Held, _ty: &str, _tag: &str, _arms: &[UnionWriteArm]) -> String {
        unreachable!("{CPP_HOLDS_NO_UNION}")
    }
    fn union_member(
        &self,
        _union: &ScopedUnion,
        _left: &[&ScopedCase],
        _member: &str,
    ) -> Result<String, String> {
        Err(CPP_HOLDS_NO_UNION.to_string())
    }
    fn union_place_of(&self, read: &str) -> String {
        format!("{read}Place")
    }
    fn union_member_place(
        &self,
        _holder: &str,
        _union: &ScopedUnion,
        _left: &[&ScopedCase],
        _member: &str,
    ) -> Result<String, String> {
        Err(CPP_HOLDS_NO_UNION.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::super::test_support::*;
    use super::*;

    const CPP: CppComposed<'static> = CppComposed {
        machine: "M",
        carrier: CppCarrier::Json,
    };

    /// The head of every C++ reader, as the tests below spell what they
    /// expect of one.
    const R: &str = "[&](const ::SCE::Common::EventPayloadValue &sceValue, auto &sceInto) -> \
                     ::SCE::Common::EventPayloadRefusal";

    /// The C++ row of section 7.1: no type carries a bound, a value that may
    /// be absent is `std::optional` over its type, a record is the machine's
    /// struct, at any depth; no check is declared beside a record, since a C++
    /// struct has no constructor that could refuse a value.
    #[test]
    fn a_cpp_machine_holds_each_constructor_in_its_own_type_at_any_depth() {
        let held = held();
        let enums = enums();
        let spelled =
            |text: &str| machine_type(&ty(text), None, &held, &enums, &CPP).expect("a type");
        assert_eq!(spelled("uint16"), "uint16_t");
        assert_eq!(spelled("string[8]"), "std::string");
        assert_eq!(spelled("bytes[4]"), "std::vector<uint8_t>");
        assert_eq!(spelled("optional<int64>"), "std::optional<int64_t>");
        assert_eq!(
            spelled("list<list<uint8>[2]>[3]"),
            "std::vector<std::vector<uint8_t>>"
        );
        assert_eq!(spelled("list<enum:Level>[2]"), "std::vector<MLevelEnum>");
        assert_eq!(spelled("list<record:Item>[3]"), "std::vector<MItemRecord>");
        assert_eq!(
            spelled("optional<list<uint8>[2]>"),
            "std::optional<std::vector<uint8_t>>"
        );
        assert_eq!(
            spelled("list<optional<uint16>>[2]"),
            "std::vector<std::optional<uint16_t>>"
        );
        let members = record_members("Item", &held, &enums, &CPP).expect("members");
        let checks = record_checks("Item", &held, &CPP).expect("checks");
        assert!(checks.is_empty(), "{checks:?}");
        let def = CPP.record_def("MItemRecord", "Item", &members, &checks);
        assert!(
            def.ends_with(
                "struct MItemRecord {\n    uint32_t id;\n    MLevelEnum level;\n    \
                 std::optional<int64_t> due;\n    std::vector<uint8_t> blob;\n    \
                 std::vector<std::string> tags;\n};"
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
    }

    /// Two places whose types differ only in their bounds are one C++ type,
    /// and a C++ assignment copies a list by value at every depth, so a value
    /// that fits is copied as it is; where the types differ, each part is
    /// copied into the place's type by a lambda the value is handed to.
    #[test]
    fn a_cpp_copy_of_a_value_that_fits_is_the_value() {
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
            copy("list<uint8>[2]", "list<uint8>[4]"),
            Ok("x".to_string())
        );
        assert_eq!(
            copy("list<string[4]>[2]", "list<string[8]>[3]"),
            Ok("x".to_string())
        );
        assert_eq!(
            copy("list<record:Item>[2]", "list<record:Item>[3]"),
            Ok("x".to_string())
        );
        assert_eq!(
            copy("list<uint8>[2]", "list<optional<uint8>>[2]"),
            Ok(
                "[&](const auto &sceList) -> std::vector<std::optional<uint8_t>> { \
                std::vector<std::optional<uint8_t>> sceItems; \
                sceItems.reserve(sceList.size()); for (const auto &sceCopied : sceList) { \
                sceItems.push_back(sceCopied); } return sceItems; }(x)"
                    .to_string()
            )
        );
    }

    /// A C++ raise seam holds a host's value to every bound its type declares
    /// (docs/adr/0086, section 5.4): one check per constructor that holds a
    /// bound, an element's or a present value's inside the lambda the runtime
    /// hands it to, each parameter named by its depth; a lower count is held
    /// too; a record a list holds is checked field by field at the path below
    /// the element; a number, an enum and an optional one of them are not
    /// checked.
    #[test]
    fn a_cpp_raise_seam_checks_each_bound_before_it_encodes() {
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
        assert_eq!(
            check("list<list<uint8>[2]>[2..3]", "grid"),
            Some(format!(
                "{b}::listWithin(::std::string(\"grid\"), grid, 2u, 3u, [&](const ::std::string \
                 &scePath0, const auto &sceValue0) -> ::std::string {{ return \
                 {b}::list(scePath0, sceValue0, 2u, {nothing}); }})"
            ))
        );
        assert_eq!(
            check("optional<list<string[4]>[2]>", "spare"),
            Some(format!(
                "{b}::optional(::std::string(\"spare\"), spare, [&](const ::std::string \
                 &scePath0, const auto &sceValue0) -> ::std::string {{ return \
                 {b}::list(scePath0, sceValue0, 2u, [&](const ::std::string &scePath1, const \
                 auto &sceValue1) -> ::std::string {{ return {b}::text(scePath1, sceValue1, \
                 4u); }}); }})"
            ))
        );
        let batch = held.record(None, "Batch").expect("Batch");
        assert_eq!(
            cpp_raise_checks(batch, &held).expect("checks"),
            vec![
                format!(
                    "{b}::list(::std::string(\"items\"), items, 3u, [&](const ::std::string \
                     &scePath0, const auto &sceValue0) -> ::std::string {{ return [&]() -> \
                     ::std::string {{ ::std::string sceRefused; sceRefused = \
                     {b}::bytes(scePath0 + \"/blob\", sceValue0.blob, 4u); if \
                     (!sceRefused.empty()) {{ return sceRefused; }} sceRefused = \
                     {b}::list(scePath0 + \"/tags\", sceValue0.tags, 4u, [&](const \
                     ::std::string &scePath1, const auto &sceValue1) -> ::std::string {{ return \
                     {b}::text(scePath1, sceValue1, 8u); }}); if (!sceRefused.empty()) {{ return \
                     sceRefused; }} return sceRefused; }}(); }})"
                ),
                format!(
                    "{b}::list(::std::string(\"grid\"), grid, 2u, [&](const ::std::string \
                     &scePath0, const auto &sceValue0) -> ::std::string {{ return \
                     {b}::list(scePath0, sceValue0, 2u, {nothing}); }})"
                ),
                format!("{b}::list(::std::string(\"late\"), late, 2u, {nothing})"),
            ]
        );
    }

    /// A C++ walk names no union: the one decision of what is lowered refuses
    /// one on C++ first, and the walk that would declare or check one says why
    /// there is none.
    #[test]
    fn a_cpp_walk_reads_no_union() {
        let shapes = shapes();
        assert!(union_definition(SHAPE_KEY, &shapes, &kinds(), &CPP).is_err());
        let all: Vec<&ScopedCase> = shapes.union.cases.iter().collect();
        assert!(CPP
            .union_member(&shapes.union, &all, "kind")
            .unwrap_err()
            .contains("holds no union"));
        assert!(
            cpp_bound_check(&ty("union:Shape"), None, "path", "one", 0, &shapes)
                .unwrap_err()
                .contains("holds no union")
        );
    }
}
