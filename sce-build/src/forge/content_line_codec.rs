// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// A `sce:encoding="content-line"` codec, generated (SCE_FORGE.md §4.6.4,
// docs/adr/0010).
//
// A content-line codec is one RFC 5545 component: `BEGIN:<component>`, the
// declared properties as folded content lines, `END:<component>`. None of the
// positional codec's machinery — byte offsets, bit sizes, flags, present-if,
// repeat, TLV — has anything to say about it, so it is rendered here rather
// than through `render_codec`'s positional path, which never sees one.
//
// The context below carries data only — each entry's property, kind, bounds and
// type spellings — and the template writes every statement: the line grammar
// (unfolding, TEXT escapes, quoting, folding) is the runtime's
// (`sce_forge_runtime::content_line`), so none of it is spelled in this
// generator either.
//
// A backend generates it in its own commit, as the CBOR codec was
// (`forge::cbor_codec`): a codec that compiled and wrote an empty component
// would be a wrong output, and a missing one is refused by name until then.

use crate::forge::error::{ForgeError, GenerateError};
use crate::forge::generator::{ImportContext, LangCtx};
use crate::forge::model::{CodecModel, ContentLineEntry, SceType};
use crate::generator::Language;

/// Whether `lang` generates a content-line codec — the one answer the
/// generator's refusal and the conformance harness's schedule both read, so a
/// fixture is run exactly where the generator admits it.
pub fn lowers(lang: Language) -> bool {
    matches!(lang, Language::Rust | Language::Kotlin | Language::Cpp)
}

/// Why `lang` does not generate the content-line codec `m`, or `None` when it
/// does.
pub fn refusal(lang: Language, m: &CodecModel) -> Option<String> {
    if !lowers(lang) {
        return Some(format!(
            "codec '{}' is sce:encoding=\"content-line\", which has no {lang:?} generation yet",
            m.name
        ));
    }
    None
}

/// Render a content-line codec for `lang`.
pub fn render(
    env: &minijinja::Environment,
    m: &CodecModel,
    imports: &[ImportContext],
    lang: Language,
) -> Result<String, ForgeError> {
    if let Some(why) = refusal(lang, m) {
        return Err(ForgeError::from(GenerateError::unsupported_at(
            why,
            m.source_location.clone(),
        )));
    }
    match lang {
        Language::Rust => render_rust(env, m, imports),
        Language::Kotlin => render_kotlin(env, m, imports),
        Language::Cpp => render_cpp(env, m, imports),
        _ => unreachable!("`refusal` admits only a backend that has a render"),
    }
}

/// What an entry's value is, as the templates branch on it.
fn kind(entry: &ContentLineEntry) -> &'static str {
    match &entry.sce_type {
        SceType::String => "string",
        SceType::Bool => "bool",
        ty if ty.is_signed() => "int",
        _ => "uint",
    }
}

/// The value entries of `m` in declaration order, each with the parameters
/// declared for its property, as the context the templates read. `value_type`
/// is this backend's type for one value of an entry (`string_type` spells a
/// string of at most `n` bytes); the callers wrap it as their language makes an
/// entry optional or a list.
fn entries_context(
    l: &LangCtx,
    m: &CodecModel,
    string_type: impl Fn(u32) -> String,
    default_of: impl Fn(&SceType) -> Option<String>,
) -> Vec<serde_json::Value> {
    let model = m
        .content_line
        .as_ref()
        .expect("a content-line codec carries its entries");
    let value_type = |e: &ContentLineEntry| match &e.sce_type {
        SceType::String => string_type(e.max_size.unwrap_or(0)),
        other => l.type_name(other).into_owned(),
    };
    model
        .entries
        .iter()
        .filter(|e| e.param.is_none())
        .map(|e| {
            let params: Vec<serde_json::Value> = model
                .entries
                .iter()
                .filter(|p| p.param.is_some() && p.property.eq_ignore_ascii_case(&e.property))
                .map(|p| {
                    let name = l.codec_field_id(&p.id);
                    serde_json::json!({
                        "name": name,
                        "local": format!("field_{name}"),
                        "param": p.param,
                        "required": p.required,
                        "max_size": p.max_size,
                        "value_type": value_type(p),
                        "default": default_of(&p.sce_type),
                    })
                })
                .collect();
            let name = l.codec_field_id(&e.id);
            serde_json::json!({
                "name": name,
                "local": format!("field_{name}"),
                "property": e.property,
                "kind": kind(e),
                "text": e.text,
                "required": e.required,
                "max_size": e.max_size,
                "max_count": e.max_count,
                "is_list": e.max_count.is_some(),
                "value_type": value_type(e),
                "default": default_of(&e.sce_type),
                "params": params,
            })
        })
        .collect()
}

/// The component the codec reads and writes, as the name between `BEGIN:` and
/// `END:`.
fn insert_component(ctx: &mut serde_json::Map<String, serde_json::Value>, m: &CodecModel) {
    ctx.insert(
        "component".into(),
        m.content_line
            .as_ref()
            .map(|c| c.component.clone())
            .unwrap_or_default()
            .into(),
    );
}

fn render_kotlin(
    env: &minijinja::Environment,
    m: &CodecModel,
    imports: &[ImportContext],
) -> Result<String, ForgeError> {
    let l = LangCtx::new(Language::Kotlin, imports);
    let mut ctx = l.base_context(&m.name);
    l.insert_imports(&mut ctx, imports);
    // Owned values, as every Kotlin codec holds them: a string is a `String`, a
    // list a `MutableList`, and an optional entry or parameter absent is `null`.
    // A required entry starts at its type's own default and is filled in by
    // decode.
    let mut entries = entries_context(
        &l,
        m,
        |_| "String".to_string(),
        |ty| Some(l.default_expr(ty)),
    );
    shape_fields(
        &mut entries,
        &FieldShapes {
            list_type: "MutableList<String>",
            list_default: Some("mutableListOf()"),
            optional_type: &|t| format!("{t}?"),
            optional_default: Some("null"),
        },
    );
    insert_component(&mut ctx, m);
    ctx.insert("entries".into(), entries.into());
    l.render(env, "codec_content_line", ctx)
}

fn render_cpp(
    env: &minijinja::Environment,
    m: &CodecModel,
    imports: &[ImportContext],
) -> Result<String, ForgeError> {
    let l = LangCtx::new(Language::Cpp, imports);
    let mut ctx = l.base_context(&m.name);
    l.insert_imports(&mut ctx, imports);
    // Owned values, as every C++ codec holds them: a string is a `std::string`, a
    // list a `std::vector`, and an optional entry or parameter a `std::optional`.
    // A member is value-initialized, which is the right start for every kind
    // this codec holds: an empty string, a zero, `false`.
    let mut entries = entries_context(&l, m, |_| "std::string".to_string(), |_| None);
    shape_fields(
        &mut entries,
        &FieldShapes {
            list_type: "std::vector<std::string>",
            list_default: None,
            optional_type: &|t| format!("std::optional<{t}>"),
            optional_default: None,
        },
    );
    insert_component(&mut ctx, m);
    ctx.insert("entries".into(), entries.into());
    l.render(env, "codec_content_line", ctx)
}

/// How a backend spells an entry that is a list or is optional, and what such
/// a field starts at, for [`shape_fields`].
struct FieldShapes<'a> {
    list_type: &'a str,
    /// The start of a list, when the backend spells one.
    list_default: Option<&'a str>,
    /// The type of a value that may be absent.
    optional_type: &'a dyn Fn(&str) -> String,
    /// What an absent value starts at, when the backend spells one.
    optional_default: Option<&'a str>,
}

/// Give each entry and each of its parameters the type of the field that holds
/// it — the value's type when it is required, the backend's optional spelling
/// when it is not, its list type for a list — and list the parameters that may be
/// absent, which an encode must check against an absent property. A required
/// value keeps the `default` the context gave it.
fn shape_fields(entries: &mut [serde_json::Value], shapes: &FieldShapes<'_>) {
    for entry in entries {
        let value_type = entry["value_type"].as_str().unwrap_or_default().to_string();
        let required = entry["required"].as_bool() == Some(true);
        if entry["is_list"].as_bool() == Some(true) {
            entry["field_type"] = shapes.list_type.into();
            entry["default"] = shapes.list_default.into();
        } else if !required {
            entry["field_type"] = (shapes.optional_type)(&value_type).into();
            entry["default"] = shapes.optional_default.into();
        } else {
            entry["field_type"] = value_type.into();
        }
        let mut optional_params = Vec::new();
        if let Some(serde_json::Value::Array(params)) = entry.get_mut("params") {
            for param in params {
                let value_type = param["value_type"].as_str().unwrap_or_default().to_string();
                if param["required"].as_bool() == Some(true) {
                    param["field_type"] = value_type.into();
                } else {
                    param["field_type"] = (shapes.optional_type)(&value_type).into();
                    param["default"] = shapes.optional_default.into();
                    optional_params.push(serde_json::json!({ "name": param["name"] }));
                }
            }
        }
        entry["optional_params"] = optional_params.into();
    }
}

fn render_rust(
    env: &minijinja::Environment,
    m: &CodecModel,
    imports: &[ImportContext],
) -> Result<String, ForgeError> {
    let l = LangCtx::new(Language::Rust, imports);
    let mut ctx = l.base_context(&m.name);
    l.insert_imports(&mut ctx, imports);
    // A value lives in the bounded storage its `sce:max-size` names, so neither
    // direction allocates and the struct borrows nothing from its input.
    let mut entries = entries_context(&l, m, |n| format!("heapless::String<{n}>"), |_| None);
    for entry in &mut entries {
        let value_type = entry["value_type"].as_str().unwrap_or_default().to_string();
        let required = entry["required"].as_bool() == Some(true);
        let (field_type, local_type, local_init) = if entry["is_list"].as_bool() == Some(true) {
            let list = format!(
                "heapless::Vec<{value_type}, {}>",
                entry["max_count"].as_u64().unwrap_or(0)
            );
            (list.clone(), list, "heapless::Vec::new()".to_string())
        } else if required {
            (
                value_type.clone(),
                format!("Option<{value_type}>"),
                "None".to_string(),
            )
        } else {
            let optional = format!("Option<{value_type}>");
            (optional.clone(), optional, "None".to_string())
        };
        entry["field_type"] = field_type.into();
        entry["local_type"] = local_type.into();
        entry["local_init"] = local_init.into();
        if let Some(serde_json::Value::Array(params)) = entry.get_mut("params") {
            for param in params {
                let value_type = param["value_type"].as_str().unwrap_or_default().to_string();
                param["field_type"] = if param["required"].as_bool() == Some(true) {
                    value_type
                } else {
                    format!("Option<{value_type}>")
                }
                .into();
            }
        }
    }
    insert_component(&mut ctx, m);
    ctx.insert("entries".into(), entries.into());
    ctx.insert(
        "codec_struct_derives_attr".into(),
        format!(
            "#[derive(Default, {})]",
            crate::rust_derive_policy::RustDeriveCategory::CodecStruct
                .derives()
                .join(", ")
        )
        .into(),
    );
    l.render(env, "codec_content_line", ctx)
}
