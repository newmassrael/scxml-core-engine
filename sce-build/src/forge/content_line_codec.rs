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
    matches!(
        lang,
        Language::Rust
            | Language::Kotlin
            | Language::Cpp
            | Language::Go
            | Language::Python
            | Language::C11
    )
}

/// Whether `lang` generates the line record of a repeated property that declares
/// a parameter, and the list of values a `sce:separator` makes of a line
/// (docs/adr/0014). Each backend turns this on in the commit that generates it,
/// against the conformance vectors: a codec that compiled and dropped a parameter
/// would be the silent loss the decision exists to end.
pub fn lowers_line_records(lang: Language) -> bool {
    match lang {
        Language::Python => true,
        Language::Rust | Language::Kotlin | Language::Cpp | Language::Go | Language::C11 => false,
    }
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
    if m.content_line
        .as_ref()
        .is_some_and(|c| c.uses_line_records())
        && !lowers_line_records(lang)
    {
        return Some(format!(
            "codec '{}' reads a line of a repeated property as a record, or a line as a list \
             of values (sce:separator), which has no {lang:?} generation yet (docs/adr/0014)",
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
        Language::Go => render_go(env, m, imports),
        Language::Python => render_python(env, m, imports),
        Language::C11 => render_c(env, m, imports),
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

/// The width in bits of an integer type, as the bound its entry's read is held
/// to — written into the context once so no template spells a width.
fn int_bits(ty: &SceType) -> Option<u32> {
    Some(match ty {
        SceType::Uint8 | SceType::Int8 => 8,
        SceType::Uint16 | SceType::Int16 => 16,
        SceType::Uint32 | SceType::Int32 => 32,
        SceType::Uint64 | SceType::Int64 => 64,
        _ => return None,
    })
}

/// The smallest and largest number an integer type holds, as the bounds its
/// entry's read and write are held to — for a backend whose integer carries no
/// width of its own.
fn int_range(ty: &SceType) -> Option<(i64, u64)> {
    Some(match ty {
        SceType::Uint8 => (0, u64::from(u8::MAX)),
        SceType::Uint16 => (0, u64::from(u16::MAX)),
        SceType::Uint32 => (0, u64::from(u32::MAX)),
        SceType::Uint64 => (0, u64::MAX),
        SceType::Int8 => (i64::from(i8::MIN), i8::MAX as u64),
        SceType::Int16 => (i64::from(i16::MIN), i16::MAX as u64),
        SceType::Int32 => (i64::from(i32::MIN), i32::MAX as u64),
        SceType::Int64 => (i64::MIN, i64::MAX as u64),
        _ => return None,
    })
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
    let struct_name = l
        .base_context(&m.name)
        .get("struct_name")
        .and_then(serde_json::Value::as_str)
        .unwrap_or_default()
        .to_string();
    model
        .entries
        .iter()
        .filter(|e| e.param.is_none())
        .map(|e| {
            let is_records = model.reads_records(e);
            let params: Vec<serde_json::Value> = model
                .parameters_of(e)
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
                // One value a line, on several lines.
                "is_list": e.max_count.is_some() && !is_records,
                // One line holding a list of values (docs/adr/0014).
                "is_values": model.reads_values(e),
                // Several lines, each a record of its parameters and its value or
                // values (docs/adr/0014). The parameters are members of the record
                // and not of the codec, and the record is a type of its own.
                "is_records": is_records,
                "separator": e.separator,
                "max_values": e.max_values,
                "record_type": is_records.then(|| format!(
                    "{struct_name}{}Line",
                    crate::filters::to_pascal_case(e.id.clone())
                )),
                "value_member": if e.separator.is_some() { "values" } else { "value" },
                "value_type": value_type(e),
                "bits": int_bits(&e.sce_type),
                "min": int_range(&e.sce_type).map(|r| r.0),
                "max": int_range(&e.sce_type).map(|r| r.1),
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
            list_of: &|t| format!("MutableList<{t}>"),
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
            list_of: &|t| format!("std::vector<{t}>"),
            list_default: None,
            optional_type: &|t| format!("std::optional<{t}>"),
            optional_default: None,
        },
    );
    insert_component(&mut ctx, m);
    ctx.insert("entries".into(), entries.into());
    l.render(env, "codec_content_line", ctx)
}

fn render_python(
    env: &minijinja::Environment,
    m: &CodecModel,
    imports: &[ImportContext],
) -> Result<String, ForgeError> {
    let l = LangCtx::new(Language::Python, imports);
    let mut ctx = l.base_context(&m.name);
    l.insert_imports(&mut ctx, imports);
    // Owned values, as every Python codec holds them: a `str`, a `List[str]`
    // that each instance starts empty, and an optional entry or parameter
    // `None`. A required entry starts at its type's own default. A Python `int`
    // has no width, so an integer entry carries its bounds for the template to
    // hold.
    let mut entries = entries_context(&l, m, |_| "str".to_string(), |ty| Some(l.default_expr(ty)));
    shape_fields(
        &mut entries,
        &FieldShapes {
            list_of: &|t| format!("List[{t}]"),
            list_default: Some("field(default_factory=list)"),
            optional_type: &|t| format!("Optional[{t}]"),
            optional_default: Some("None"),
        },
    );
    insert_component(&mut ctx, m);
    ctx.insert("entries".into(), entries.into());
    l.render(env, "codec_content_line", ctx)
}

fn render_go(
    env: &minijinja::Environment,
    m: &CodecModel,
    imports: &[ImportContext],
) -> Result<String, ForgeError> {
    let l = LangCtx::new(Language::Go, imports);
    let mut ctx = l.base_context(&m.name);
    l.insert_imports(&mut ctx, imports);
    // Owned values, as every Go codec holds them: a string is a `string`, a list a
    // slice, and an optional entry or parameter a pointer, nil when absent. A
    // field starts at its zero value, which is the right start for every kind
    // this codec holds.
    let mut entries = entries_context(&l, m, |_| "string".to_string(), |_| None);
    shape_fields(
        &mut entries,
        &FieldShapes {
            list_of: &|t| format!("[]{t}"),
            list_default: None,
            optional_type: &|t| format!("*{t}"),
            optional_default: None,
        },
    );
    // `math` is imported for the bounds of an integer entry's read, and only
    // then: an unused import is a compile error in Go.
    let uses_math = entries.iter().any(|e| e["bits"].is_u64());
    ctx.insert("uses_math".into(), uses_math.into());
    insert_component(&mut ctx, m);
    ctx.insert("entries".into(), entries.into());
    l.render(env, "codec_content_line", ctx)
}

/// How a backend spells an entry that is a list or is optional, and what such
/// a field starts at, for [`shape_fields`].
struct FieldShapes<'a> {
    /// The type of a list of `t`: of values, or of the line records of a repeated
    /// property (docs/adr/0014).
    list_of: &'a dyn Fn(&str) -> String,
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
        if entry["is_records"].as_bool() == Some(true) {
            let record_type = entry["record_type"]
                .as_str()
                .unwrap_or_default()
                .to_string();
            entry["field_type"] = (shapes.list_of)(&record_type).into();
            entry["default"] = shapes.list_default.into();
        } else if entry["is_list"].as_bool() == Some(true)
            || entry["is_values"].as_bool() == Some(true)
        {
            entry["field_type"] = (shapes.list_of)(&value_type).into();
            entry["default"] = shapes.list_default.into();
        } else if !required {
            entry["field_type"] = (shapes.optional_type)(&value_type).into();
            entry["default"] = shapes.optional_default.into();
        } else {
            entry["field_type"] = value_type.into();
        }
        if let Some(serde_json::Value::Array(params)) = entry.get_mut("params") {
            for param in params {
                let value_type = param["value_type"].as_str().unwrap_or_default().to_string();
                if param["required"].as_bool() == Some(true) {
                    param["field_type"] = value_type.into();
                } else {
                    param["field_type"] = (shapes.optional_type)(&value_type).into();
                    param["default"] = shapes.optional_default.into();
                }
            }
        }
        mark_optional_params(entry);
    }
}

/// List, as `optional_params`, the parameters of `entry` that may be absent:
/// an encode of an absent property must check that none of them is given.
fn mark_optional_params(entry: &mut serde_json::Value) {
    let optional: Vec<serde_json::Value> = entry["params"]
        .as_array()
        .map(|params| {
            params
                .iter()
                .filter(|p| p["required"].as_bool() != Some(true))
                .map(|p| serde_json::json!({ "name": p["name"] }))
                .collect()
        })
        .unwrap_or_default();
    entry["optional_params"] = optional.into();
}

fn render_c(
    env: &minijinja::Environment,
    m: &CodecModel,
    imports: &[ImportContext],
) -> Result<String, ForgeError> {
    let l = LangCtx::new(Language::C11, imports);
    let mut ctx = l.base_context(&m.name);
    l.insert_imports(&mut ctx, imports);
    crate::forge::generator::insert_c_codec_symbols(&mut ctx, &m.name);
    // A string is a fixed array the size of its bound beside its length, a list
    // an array of them beside a count, and an optional entry or parameter a
    // `<name>_present` flag beside its value: no allocation, as the rest of the
    // C11 runtime. Every string has its bound (`sce:max-size` is required of
    // one), so C11 refuses nothing the other backends admit.
    let mut entries = entries_context(&l, m, |_| "char".to_string(), |_| None);
    for entry in &mut entries {
        mark_optional_params(entry);
        // The bounds of an integer entry's read, as C literals: a 64-bit literal
        // needs its width said, and the smallest int64 has no literal at all.
        let (min, max) = (entry["min"].as_i64(), entry["max"].as_u64());
        if let (Some(min), Some(max)) = (min, max) {
            entry["max_expr"] = if entry["kind"] == "uint" {
                format!("UINT64_C({max})")
            } else {
                format!("INT64_C({max})")
            }
            .into();
            entry["min_expr"] = if min == i64::MIN {
                "INT64_MIN".to_string()
            } else {
                format!("INT64_C({min})")
            }
            .into();
        }
    }
    insert_component(&mut ctx, m);
    ctx.insert("entries".into(), entries.into());
    ctx.insert("max_encoded_bytes".into(), max_encoded_bytes(m).into());
    l.render(env, "codec_content_line", ctx)
}

/// The most bytes `m` encodes to, folds and escapes included: a bound a C11
/// caller sizes an `encode_to_buf` buffer by. Every entry present at its
/// largest — a TEXT with every character escaped, a list full — and every
/// logical line cut as often as a cut can fall (a line carries at least 71
/// payload octets between cuts, the widest unit being four).
fn max_encoded_bytes(m: &CodecModel) -> u64 {
    let Some(model) = m.content_line.as_ref() else {
        return 0;
    };
    let component = model.component.len() as u64;
    // `BEGIN:` + component + CRLF, `END:` + component + CRLF.
    let mut total = 2 * component + 6 + 4 + 4;
    for entry in model.entries.iter().filter(|e| e.param.is_none()) {
        let mut logical = entry.property.len() as u64;
        for param in model
            .entries
            .iter()
            .filter(|p| p.param.is_some() && p.property.eq_ignore_ascii_case(&entry.property))
        {
            // `;` + name + `=` + the quotes + the value.
            let name = param.param.as_deref().map_or(0, str::len) as u64;
            logical += 1 + name + 1 + 2 + u64::from(param.max_size.unwrap_or(0));
        }
        let value = match &entry.sce_type {
            SceType::String => {
                u64::from(entry.max_size.unwrap_or(0)) * if entry.text { 2 } else { 1 }
            }
            SceType::Bool => 5,
            _ => 20,
        };
        logical += 1 + value;
        let physical = logical + 2 + 3 * (logical / 70 + 1);
        total += physical * u64::from(entry.max_count.unwrap_or(1));
    }
    total
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
