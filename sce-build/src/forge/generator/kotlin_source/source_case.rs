// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! A byte case of the Kotlin conformance harness (docs/adr/0081 §5): a value a
//! case states as the codec's interface presents it, made by the printed data
//! class's constructor, and for each element an expression that holds when a
//! decode of the case's bytes presents what that value does.
//!
//! The case reads the notation of `source` where every printer's case reads it
//! ([`source_members`] and the readers beside it), so one case states one value
//! in every language. A byte string is made by the factory of the value class
//! of its bound, which the literal reaches by the name the printer gives it
//! ([`refinement_class`]). Every name is written whole, by the package of the
//! document that declares it, so the harness imports nothing for a case. A
//! shape the Kotlin printer does not print yet is refused by name, as the Rust
//! case refuses what it does not spell.

use super::super::rust_source::{
    byte_values, case_inputs, case_tag, enum_variant, integer_value, source_members, HeldCodecs,
};
use super::super::*;
use super::refinement_class;
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
}

/// The byte case of `source`, a value of the codec `plan` at `path`. A codec
/// whose case hands it flag inputs (`inputs`) or a caller's tag (`tag`), or
/// that holds a group or a variant, is one the Kotlin printer does not print,
/// and its case is refused by name; so is a source that states an element the
/// value does not present or leaves out one it does.
pub(crate) fn source_case(
    plan: &EncodePlan<'_>,
    path: &CodecPath,
    source: &serde_json::Value,
    inputs: &serde_json::Value,
    tag: &serde_json::Value,
    held: &mut dyn HeldCodecs,
) -> Result<SourceCase, String> {
    if !case_inputs(plan, inputs)?.is_empty() {
        return Err("a Kotlin case hands no flag input yet: the Kotlin printer \
                    prints no codec that takes one"
            .to_string());
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
    let l = LangCtx::new(crate::generator::Language::Kotlin, &[]);
    let checks = plan
        .elements
        .iter()
        .map(|element| {
            let property = l.codec_field_id(&element.field.id);
            let (decoded, value) = (
                format!("{DECODED}.{property}"),
                format!("{VALUE}.{property}"),
            );
            // A value class over an array compares the array it holds by
            // identity, so a byte string is compared by its content.
            let holds = match &element.value {
                Value::Bytes(_) if element.presence.optional() => {
                    format!("{decoded}?.bytes.contentEquals({value}?.bytes)")
                }
                Value::Bytes(_) => format!("{decoded}.bytes.contentEquals({value}.bytes)"),
                _ => format!("{decoded} == {value}"),
            };
            (element.field.id.clone(), holds)
        })
        .collect();
    Ok(SourceCase { literal, checks })
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
        Value::Text(_) => Err(
            "a text is not spelled as a Kotlin literal yet: the Kotlin printer prints no codec \
             that holds one"
                .into(),
        ),
        Value::Body { .. } => Err(
            "a body is not spelled as a Kotlin literal yet: the Kotlin printer prints no codec \
             that holds one"
                .into(),
        ),
        Value::List { .. } => Err(
            "a list is not spelled as a Kotlin literal yet: the Kotlin printer prints no codec \
             that holds one"
                .into(),
        ),
    }
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
