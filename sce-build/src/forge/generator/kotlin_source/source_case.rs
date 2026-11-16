// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! A byte case of the Kotlin conformance harness (docs/adr/0081 §5): a value a
//! case states as the codec's interface presents it, made by the printed data
//! class's constructor, and for each element an expression that holds when a
//! decode of the case's bytes presents what that value does.
//!
//! The case reads the notation of `source` where every printer's case reads it
//! ([`source_members`] and the readers beside it), so one case states one value
//! in every language. A byte string or a text is made by the factory of the
//! value class of its bound, which the literal reaches by the name the printer
//! gives it ([`refinement_class`]). Every name is written whole, by the package
//! of the document that declares it, so the harness imports nothing for a
//! case. A shape the Kotlin printer does not print yet is refused by name, as
//! the Rust case refuses what it does not spell.

use super::super::rust_source::{
    byte_values, case_inputs, case_tag, enum_variant, integer_value, source_members,
    CodecPath as RustCodecPath, HeldCodecs,
};
use super::super::*;
use super::{list_class, refinement_class};
use crate::forge::encode_plan::{EncodePlan, Value};

/// Where a codec's Kotlin value is: the package of the document that declares
/// it and the data class of its value, both named after the document's name.
#[derive(Clone, Debug)]
pub(crate) struct CodecPath {
    /// The package of the document, `com.sce.generated.<snake>`.
    pub package: String,
    /// The data class of the value, the document's name in Pascal case.
    pub type_name: String,
}

impl CodecPath {
    /// The codec the document `name` generates.
    pub(crate) fn new(name: &str) -> Self {
        CodecPath {
            package: kotlin_forge_package(&filters::to_snake_case(name.to_string())),
            type_name: filters::to_pascal_case(name.to_string()),
        }
    }

    /// `name`, a declaration of the codec's file, written whole.
    fn qualified(&self, name: &str) -> String {
        format!("{}.{name}", self.package)
    }
}

/// Where a codec held by a codec is, whose Rust path a held codec's reading
/// names `held`: the package of its own document, which every Kotlin unit is
/// generated under whoever imports it.
fn held_codec(held: &RustCodecPath) -> CodecPath {
    let snake = held.module.rsplit("::").next().unwrap_or(&held.module);
    CodecPath {
        package: kotlin_forge_package(snake),
        type_name: held.type_name.clone(),
    }
}

/// The name the harness gives the value a case makes.
const VALUE: &str = "value";

/// The name the harness gives the value its decode of the case's bytes answers.
const DECODED: &str = "decoded";

/// A byte case of the Kotlin harness (docs/adr/0081 §5).
pub(crate) struct SourceCase {
    /// The value, made by its data class's constructor.
    pub literal: String,
    /// Each element by its id, with a Kotlin `Boolean` expression over
    /// `decoded` and `value` that holds when the two present the same.
    pub checks: Vec<(String, String)>,
    /// The arguments the decode is handed after its cursor: each flag input,
    /// in the order the codec declares them, as a `UByte` literal.
    pub inputs: Vec<String>,
    /// The arguments the encode is handed after its sink: the flag inputs the
    /// value does not say.
    pub encode_inputs: Vec<String>,
}

/// The byte case of `source`, a value of the codec `plan` at `path`, handed
/// the flag inputs `inputs` states. A codec whose value says an input, whose
/// case hands it a caller's tag (`tag`), or that holds a group or a variant, is
/// one the Kotlin printer does not print, and its case is refused by name; so
/// is a source that states an element the value does not present or leaves out
/// one it does.
pub(crate) fn source_case(
    plan: &EncodePlan<'_>,
    path: &CodecPath,
    source: &serde_json::Value,
    inputs: &serde_json::Value,
    tag: &serde_json::Value,
    held: &mut dyn HeldCodecs,
) -> Result<SourceCase, String> {
    // Each input as the decode is handed it, in the order the codec declares
    // them, and those the value does not say as the encode is handed them.
    let mut decode_inputs = Vec::new();
    let mut encode_inputs = Vec::new();
    for (input, value, given) in case_inputs(plan, inputs)? {
        if !given {
            return Err(format!(
                "a Kotlin case states no input the value says yet (`{}`): the Kotlin \
                 printer prints no codec whose value says one",
                input.name
            ));
        }
        decode_inputs.push(format!("{value}u"));
        encode_inputs.push(format!("{value}u"));
    }
    if case_tag(plan, source, tag)?.is_some() || plan.model.variant.is_some() {
        return Err("a Kotlin case states no variant yet: the Kotlin printer \
                    prints no codec that holds one"
            .to_string());
    }
    if !plan.groups().is_empty() {
        return Err("a Kotlin case states no group yet: the Kotlin printer \
                    prints no codec that holds one"
            .to_string());
    }
    let literal = value_literal(plan, path, source, held)?;
    let mut checks = Vec::new();
    for element in &plan.elements {
        if let Some(holds) = element_holds(element, DECODED, VALUE, 0, held)? {
            checks.push((element.field.id.clone(), holds));
        }
    }
    Ok(SourceCase {
        literal,
        checks,
        inputs: decode_inputs,
        encode_inputs,
    })
}

/// A Kotlin `Boolean` expression that holds when `decoded` and `value`, two
/// values of the codec that presents `element`, present the same of it, or
/// `None` where there is nothing to compare: a body always there whose codec
/// presents no element. `depth` names the values a body is compared through,
/// so a body held in a body names its own.
///
/// A value class over an array compares the array it holds by identity, and a
/// data class compares such a value class so, so a byte string is compared by
/// its content and a body element by element, down into the codecs it holds.
fn element_holds(
    element: &crate::forge::encode_plan::Element<'_>,
    decoded: &str,
    value: &str,
    depth: usize,
    held: &mut dyn HeldCodecs,
) -> Result<Option<String>, String> {
    let l = LangCtx::new(crate::generator::Language::Kotlin, &[]);
    let property = l.codec_field_id(&element.field.id);
    let (decoded, value) = (
        format!("{decoded}.{property}"),
        format!("{value}.{property}"),
    );
    let optional = element.presence.optional();
    Ok(Some(match &element.value {
        Value::Bytes(_) if optional => {
            format!("{decoded}?.bytes.contentEquals({value}?.bytes)")
        }
        Value::Bytes(_) => format!("{decoded}.bytes.contentEquals({value}.bytes)"),
        Value::Body { codec } => {
            let (d, v) = (format!("d{depth}"), format!("v{depth}"));
            let inner = held.read(codec, &mut |plan, _, held| {
                let mut holds = Vec::new();
                for element in &plan.elements {
                    holds.extend(element_holds(element, &d, &v, depth + 1, held)?);
                }
                Ok(holds.join(" && "))
            })?;
            match (inner.is_empty(), optional) {
                // The body presents nothing: only whether it is there.
                (true, false) => return Ok(None),
                (true, true) => format!("({decoded} == null) == ({value} == null)"),
                (false, false) => {
                    format!("{decoded}.let {{ {d} -> {value}.let {{ {v} -> {inner} }} }}")
                }
                (false, true) => format!(
                    "{decoded}.let {{ {d} -> {value}.let {{ {v} -> \
                     if ({d} == null || {v} == null) {d} == null && {v} == null else {inner} }} }}"
                ),
            }
        }
        // A list by its entries, each compared as a body is.
        Value::List { codec, .. } => {
            let (d, v) = (format!("d{depth}"), format!("v{depth}"));
            if optional {
                let entries = list_holds(
                    codec,
                    &format!("{d}.entries"),
                    &format!("{v}.entries"),
                    depth,
                    held,
                )?;
                format!(
                    "{decoded}.let {{ {d} -> {value}.let {{ {v} -> \
                     if ({d} == null || {v} == null) {d} == null && {v} == null else {entries} }} }}"
                )
            } else {
                let entries = list_holds(codec, &d, &v, depth, held)?;
                format!(
                    "{decoded}.entries.let {{ {d} -> {value}.entries.let {{ {v} -> {entries} }} }}"
                )
            }
        }
        _ => format!("{decoded} == {value}"),
    }))
}

/// The value of the codec `plan` at `path` that `source` states, made by its
/// data class's constructor, each element by its name.
fn value_literal(
    plan: &EncodePlan<'_>,
    path: &CodecPath,
    source: &serde_json::Value,
    held: &mut dyn HeldCodecs,
) -> Result<String, String> {
    let object = source_members(plan, source)?;
    let l = LangCtx::new(crate::generator::Language::Kotlin, &[]);
    let mut args = Vec::new();
    for element in &plan.elements {
        let id = &element.field.id;
        let json = object
            .get(id)
            .ok_or_else(|| format!("the source states no `{id}`"))?;
        let value = if json.is_null() {
            if !element.presence.optional() {
                return Err(format!(
                    "`{id}` is always there, and the source states none"
                ));
            }
            "null".to_string()
        } else {
            member_literal(element.field, &element.value, path, json, held)
                .map_err(|e| format!("`{id}`: {e}"))?
        };
        args.push(format!("{} = {value}", l.codec_field_id(id)));
    }
    Ok(format!(
        "{}({})",
        path.qualified(&path.type_name),
        args.join(", ")
    ))
}

/// One value of the abstract type `value` of the element `field` of the codec
/// at `path`, as a Kotlin expression the data class's constructor takes: a
/// number as a literal of the element's type, a byte string as a value of the
/// class of its bound, and a value of an enum as the variant of the enum's
/// class.
fn member_literal(
    field: &CodecField,
    value: &Value<'_>,
    path: &CodecPath,
    json: &serde_json::Value,
    held: &mut dyn HeldCodecs,
) -> Result<String, String> {
    match value {
        Value::Scalar(ty) if ty.int_bit_width().is_some() => {
            integer_literal(integer_value(json)?, ty)
        }
        // A byte as the signed number of its bits, which `byteArrayOf` takes.
        Value::Bytes(_) => {
            let bytes: Vec<String> = byte_values(json)?
                .iter()
                .map(|b| (*b as i8).to_string())
                .collect();
            Ok(format!(
                "{}.of(byteArrayOf({}))!!",
                path.qualified(&refinement_class(
                    &path.type_name,
                    &field.id,
                    &field.sce_type
                )),
                bytes.join(", ")
            ))
        }
        Value::Scalar(SceType::Enum(enum_ref)) => {
            let (held_enum, variant) = enum_variant(held, &enum_ref.alias, json)?;
            let ty = kotlin_qualified(
                &held_enum.name,
                &filters::to_pascal_case(held_enum.name.clone()),
            );
            Ok(crate::forge::enum_naming::variant_ref(
                crate::generator::Language::Kotlin,
                &ty,
                &held_enum.name,
                variant,
            ))
        }
        Value::Scalar(_) => Err("a float or a bool is not spelled as a Kotlin literal yet".into()),
        Value::Text(_) => {
            let text = json
                .as_str()
                .ok_or_else(|| "text is a JSON string".to_string())?;
            Ok(format!(
                "{}.of({})!!",
                path.qualified(&refinement_class(
                    &path.type_name,
                    &field.id,
                    &field.sce_type
                )),
                string_literal(text)
            ))
        }
        // The held codec's data class, reached by its document's package, as
        // the printed codec's encode reaches it.
        Value::Body { codec } => held.read(codec, &mut |plan, held_path, held| {
            value_literal(plan, &held_codec(held_path), json, held)
        }),
        // Each entry its codec's data class, reached as a body's is, in the
        // value class of the list's bound, made by its factory.
        Value::List { codec, law, .. } if law.is_none() => {
            let entries = json
                .as_array()
                .ok_or_else(|| "a list is an array of its entries".to_string())?;
            let mut items = Vec::new();
            for entry in entries {
                items.push(held.read(codec, &mut |plan, held_path, held| {
                    value_literal(plan, &held_codec(held_path), entry, held)
                })?);
            }
            Ok(format!(
                "{}.of(listOf({}))!!",
                path.qualified(&list_class(&path.type_name, &field.id)),
                items.join(", ")
            ))
        }
        Value::List { .. } => Err(
            "a list held to a law is not spelled as a Kotlin literal yet: the Kotlin printer \
             prints no codec that holds one"
                .into(),
        ),
    }
}

/// A Kotlin `Boolean` expression that holds when the lists `decoded` and
/// `value`, of sources of the codec imported as `codec`, hold as many entries
/// and each the same as the other's at its place, compared element by element
/// through the names `depth` gives them.
fn list_holds(
    codec: &str,
    decoded: &str,
    value: &str,
    depth: usize,
    held: &mut dyn HeldCodecs,
) -> Result<String, String> {
    let (i, d, v) = (
        format!("i{depth}"),
        format!("d{}", depth + 1),
        format!("v{}", depth + 1),
    );
    let inner = held.read(codec, &mut |plan, _, held| {
        let mut holds = Vec::new();
        for element in &plan.elements {
            holds.extend(element_holds(element, &d, &v, depth + 2, held)?);
        }
        Ok(holds.join(" && "))
    })?;
    let sizes = format!("{decoded}.size == {value}.size");
    Ok(if inner.is_empty() {
        sizes
    } else {
        format!(
            "{sizes} && {decoded}.indices.all {{ {i} -> {decoded}[{i}].let {{ {d} -> \
             {value}[{i}].let {{ {v} -> {inner} }} }} }}"
        )
    })
}

/// `text` as a Kotlin string literal of the same text: a quote, a backslash and
/// a `$`, which would start a template, escaped, and every control character
/// written as its UTF-16 unit, as Kotlin has no escape for most of them.
fn string_literal(text: &str) -> String {
    let mut literal = String::from("\"");
    for c in text.chars() {
        match c {
            '"' => literal.push_str("\\\""),
            '\\' => literal.push_str("\\\\"),
            '$' => literal.push_str("\\$"),
            c if c.is_control() => literal.push_str(&format!("\\u{:04X}", c as u32)),
            c => literal.push(c),
        }
    }
    literal.push('"');
    literal
}

/// `n` as a Kotlin literal of the integer type `ty`, which the constructor's
/// parameter gives the literal: an unsigned literal for an unsigned type, and
/// a conversion of a signed one for a type no literal of its own spells.
fn integer_literal(n: i128, ty: &SceType) -> Result<String, String> {
    let fits = |least: i128, most: i128| {
        if (least..=most).contains(&n) {
            Ok(())
        } else {
            Err(format!("{n} is no value of a `{}`", ty.as_attr()))
        }
    };
    Ok(match ty {
        SceType::Uint8 => {
            fits(0, u8::MAX.into())?;
            format!("{n}u")
        }
        SceType::Uint16 => {
            fits(0, u16::MAX.into())?;
            format!("{n}u")
        }
        SceType::Uint32 => {
            fits(0, u32::MAX.into())?;
            format!("{n}u")
        }
        SceType::Uint64 => {
            fits(0, u64::MAX.into())?;
            format!("{n}uL")
        }
        SceType::Int8 => {
            fits(i8::MIN.into(), i8::MAX.into())?;
            format!("({n}).toByte()")
        }
        SceType::Int16 => {
            fits(i16::MIN.into(), i16::MAX.into())?;
            format!("({n}).toShort()")
        }
        SceType::Int32 => {
            fits(i32::MIN.into(), i32::MAX.into())?;
            n.to_string()
        }
        SceType::Int64 => {
            fits(i64::MIN.into(), i64::MAX.into())?;
            format!("{n}L")
        }
        _ => return Err(format!("a `{}` is no integer", ty.as_attr())),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A text reaches the harness as a Kotlin literal of the same text: what
    /// would end the literal, escape, or start a template is escaped, and a
    /// control character is written as its unit.
    ///
    /// Turns red where `string_literal` stops escaping a `$`, a quote or a
    /// backslash, or writes a control character raw.
    #[test]
    fn a_text_is_spelled_as_a_kotlin_literal_of_the_same_text() {
        assert_eq!(string_literal("abc"), "\"abc\"");
        assert_eq!(string_literal("a\"b\\c$d"), "\"a\\\"b\\\\c\\$d\"");
        assert_eq!(string_literal("\u{0c}\n"), "\"\\u000C\\u000A\"");
        assert_eq!(string_literal("h\u{e9}llo"), "\"h\u{e9}llo\"");
    }

    /// A number is a literal of the element's type, which the parameter gives
    /// it, and one its type does not hold is refused by name.
    ///
    /// Turns red where `integer_literal` spells a 64-bit unsigned number without
    /// its `uL` suffix, a signed byte without its conversion, or lets a number
    /// past its type through.
    #[test]
    fn a_number_is_a_literal_of_its_elements_type() {
        assert_eq!(integer_literal(255, &SceType::Uint8).as_deref(), Ok("255u"));
        assert_eq!(
            integer_literal(18_446_744_073_709_551_615, &SceType::Uint64).as_deref(),
            Ok("18446744073709551615uL")
        );
        assert_eq!(
            integer_literal(-1, &SceType::Int8).as_deref(),
            Ok("(-1).toByte()")
        );
        assert_eq!(integer_literal(-7, &SceType::Int64).as_deref(), Ok("-7L"));
        assert!(integer_literal(256, &SceType::Uint8).is_err());
        assert!(integer_literal(-1, &SceType::Uint32).is_err());
    }
}
