// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// A `sce:encoding="cbor"` codec, generated (SCE_FORGE.md §4.6.1).
//
// A CBOR codec is one map whose entries are keyed by small unsigned integers:
// order-free on decode, unknown keys skipped, written in ascending key order
// with the shortest heads (RFC 8949 §4.2.1). None of the positional codec's
// machinery — byte offsets, bit sizes, flags, present-if, repeat, TLV — has
// anything to say about it, so it is rendered here rather than through
// `render_codec`'s positional path, which never sees one.
//
// The context below carries data only — each entry's kind, key, bounds and
// type spellings — and the template writes every statement: the reads and
// writes are the runtime's CBOR primitives (`sce_forge_runtime::cbor`), so
// no CBOR is spelled in this generator either.

use crate::forge::error::{ForgeError, GenerateError};
use crate::forge::generator::{ImportContext, LangCtx};
use crate::forge::model::{CborEntry, CodecModel, SceType};

// ⚠ An entry's `name` is the backend's field identifier
// (`LangCtx::codec_field_id` — snake_case in Rust), the same spelling a
// positional codec gives its fields; the map key is `key`, never the name.
use crate::generator::Language;

/// Whether `lang` generates a CBOR codec — the one answer the generator's
/// refusal and the conformance harness's schedule both read, so a fixture
/// is run exactly where the generator admits it (the `lowers_may_fail`
/// pattern).
pub fn lowers(lang: Language) -> bool {
    matches!(
        lang,
        Language::Rust | Language::Kotlin | Language::Cpp | Language::Go
    )
}

/// Render a CBOR codec for `lang`.
///
/// A backend whose generation has not landed refuses the document by name
/// rather than emitting a type with no fields: a codec that compiled and
/// wrote an empty map would be a wrong output, and this is a missing one.
pub fn render(
    env: &minijinja::Environment,
    m: &CodecModel,
    imports: &[ImportContext],
    lang: Language,
) -> Result<String, ForgeError> {
    if !lowers(lang) {
        return Err(ForgeError::from(GenerateError::unsupported_at(
            format!(
                "codec '{}' is sce:encoding=\"cbor\", which has no {lang:?} generation yet",
                m.name
            ),
            m.source_location.clone(),
        )));
    }
    match lang {
        Language::Rust => render_rust(env, m, imports),
        Language::Kotlin => render_kotlin(env, m, imports),
        Language::Cpp => render_cpp(env, m, imports),
        Language::Go => render_go(env, m, imports),
        _ => unreachable!("`lowers` admits Rust, Kotlin, C++ and Go alone"),
    }
}

/// What an entry is on the wire, as the templates branch on it.
fn kind(entry: &CborEntry) -> &'static str {
    match entry.sce_type {
        SceType::Bool => "bool",
        SceType::String => "text",
        SceType::Bytes if entry.length.is_some() => "bytes_exact",
        SceType::Bytes => "bytes",
        SceType::Enum(_) => "enum",
        _ => "uint",
    }
}

/// The largest number an integer type holds, as the bound its entry's read
/// is held to — written into the context once so no template spells a
/// language's own constant for it.
fn int_max(ty: &SceType) -> Option<u64> {
    Some(match ty {
        SceType::Uint8 => u64::from(u8::MAX),
        SceType::Uint16 => u64::from(u16::MAX),
        SceType::Uint32 => u64::from(u32::MAX),
        SceType::Uint64 => u64::MAX,
        SceType::Int8 => i8::MAX as u64,
        SceType::Int16 => i16::MAX as u64,
        SceType::Int32 => i32::MAX as u64,
        SceType::Int64 => i64::MAX as u64,
        _ => return None,
    })
}

/// The carrier type of the enum `alias` names, as the import resolved it.
fn enum_carrier(imports: &[ImportContext], alias: &str) -> Option<SceType> {
    imports
        .iter()
        .find(|i| i.kind == "enum" && i.alias == alias)
        .and_then(|i| i.enum_underlying.clone())
}

/// The context every backend's template reads: one entry per map entry, in
/// declaration order and in key order, and the count of required entries.
/// `value_type` is this backend's type for one value of the entry; the
/// templates wrap it as their language makes an entry optional.
fn common_context(
    l: &LangCtx,
    m: &CodecModel,
    imports: &[ImportContext],
    value_type: impl Fn(&CborEntry) -> String,
) -> Result<serde_json::Map<String, serde_json::Value>, ForgeError> {
    let mut ctx = l.base_context(&m.name);
    l.insert_imports(&mut ctx, imports);
    let mut entries: Vec<serde_json::Value> = Vec::new();
    for e in &m.cbor_entries {
        let mut entry = serde_json::json!({
            "name": l.codec_field_id(&e.id),
            "key": e.key,
            "required": e.required,
            "kind": kind(e),
            "value_type": value_type(e),
            "length": e.length,
            "max_size": e.max_size,
            "max": int_max(&e.sce_type),
        });
        if let SceType::Enum(r) = &e.sce_type {
            let carrier = enum_carrier(imports, &r.alias).ok_or_else(|| {
                ForgeError::from(GenerateError::unsupported_at(
                    format!(
                        "codec '{}': entry '{}' names enum '{}', whose import resolved no carrier",
                        m.name, e.id, r.alias
                    ),
                    m.source_location.clone(),
                ))
            })?;
            entry["enum_from_underlying"] = l.enum_from_underlying(&r.alias).into();
            entry["enum_to_underlying"] = l.codec_carrier_expr(&e.sce_type, "v").into();
            entry["enum_is_open"] = l.enum_is_open(&r.alias).into();
            entry["carrier_type"] = l.type_name(&carrier).into_owned().into();
            // A signed carrier can hold a number no CBOR unsigned integer
            // carries, which encode refuses; an unsigned one cannot, and a
            // template writes no check for it (a comparison that is always
            // false is a compile warning, not a guard).
            entry["carrier_signed"] = matches!(
                carrier,
                SceType::Int8 | SceType::Int16 | SceType::Int32 | SceType::Int64
            )
            .into();
            entry["max"] = int_max(&carrier).into();
        }
        entries.push(entry);
    }
    // Written in ascending key order (RFC 8949 §4.2.1); declared in the
    // author's order.
    let mut by_key = entries.clone();
    by_key.sort_by_key(|e| e["key"].as_u64());

    let required = m.cbor_entries.iter().filter(|e| e.required).count();
    ctx.insert("required_count".into(), required.into());
    ctx.insert(
        "optional_count".into(),
        (m.cbor_entries.len() - required).into(),
    );
    ctx.insert("entries".into(), entries.into());
    ctx.insert("entries_by_key".into(), by_key.into());
    Ok(ctx)
}

fn render_kotlin(
    env: &minijinja::Environment,
    m: &CodecModel,
    imports: &[ImportContext],
) -> Result<String, ForgeError> {
    let l = LangCtx::new(Language::Kotlin, imports);
    let mut ctx = common_context(&l, m, imports, |e| l.type_name(&e.sce_type).into_owned())?;
    // A required entry starts at its type's own default — for an enum the
    // first declared variant — and is filled in by decode; an optional one
    // starts absent.
    if let Some(serde_json::Value::Array(entries)) = ctx.get_mut("entries") {
        for (entry, e) in entries.iter_mut().zip(&m.cbor_entries) {
            entry["default"] = if e.required {
                l.default_expr(&e.sce_type)
            } else {
                "null".to_string()
            }
            .into();
        }
    }
    l.render(env, "codec_cbor", ctx)
}

fn render_cpp(
    env: &minijinja::Environment,
    m: &CodecModel,
    imports: &[ImportContext],
) -> Result<String, ForgeError> {
    let l = LangCtx::new(Language::Cpp, imports);
    // Owned values, as every C++ codec holds them: a text string is a
    // `std::string` and a byte string a `std::vector<uint8_t>`.
    let mut ctx = common_context(&l, m, imports, |e| l.type_name(&e.sce_type).into_owned())?;
    // A value-initialized member holds its type's zero, which is the right
    // start for every entry but an enum's: a closed set need not declare the
    // carrier's zero, so an enum starts at its first declared variant — the
    // one default C++ codecs spell (`LangCtx::default_expr`). An optional
    // entry starts absent.
    if let Some(serde_json::Value::Array(entries)) = ctx.get_mut("entries") {
        for (entry, e) in entries.iter_mut().zip(&m.cbor_entries) {
            let value_type = entry["value_type"].as_str().unwrap_or_default().to_string();
            entry["field_type"] = if e.required {
                value_type
            } else {
                format!("std::optional<{value_type}>")
            }
            .into();
            entry["init"] = match &e.sce_type {
                SceType::Enum(r) if e.required => l.enum_default_expr(&r.alias),
                _ => String::new(),
            }
            .into();
        }
    }
    l.render(env, "codec_cbor", ctx)
}

fn render_go(
    env: &minijinja::Environment,
    m: &CodecModel,
    imports: &[ImportContext],
) -> Result<String, ForgeError> {
    let l = LangCtx::new(Language::Go, imports);
    // Owned values, as every Go codec holds them. An optional entry is a
    // pointer, so an absent byte string (`nil`) and a present empty one stay
    // two values; a required one is the value itself.
    let mut ctx = common_context(&l, m, imports, |e| l.type_name(&e.sce_type).into_owned())?;
    if let Some(serde_json::Value::Array(entries)) = ctx.get_mut("entries") {
        for (entry, e) in entries.iter_mut().zip(&m.cbor_entries) {
            let value_type = entry["value_type"].as_str().unwrap_or_default().to_string();
            entry["field_type"] = if e.required {
                value_type
            } else {
                format!("*{value_type}")
            }
            .into();
            // A required enum starts at its first declared variant, as the
            // other backends' do: a closed set need not declare the zero Go
            // would otherwise start it at.
            entry["init"] = match &e.sce_type {
                SceType::Enum(r) if e.required => l.enum_default_expr(&r.alias),
                _ => String::new(),
            }
            .into();
        }
    }
    l.render(env, "codec_cbor", ctx)
}

fn render_rust(
    env: &minijinja::Environment,
    m: &CodecModel,
    imports: &[ImportContext],
) -> Result<String, ForgeError> {
    let l = LangCtx::new(Language::Rust, imports);
    // A string or byte string is a view of the input, so the struct borrows
    // only when one of its entries does.
    let borrows = m
        .cbor_entries
        .iter()
        .any(|e| matches!(e.sce_type, SceType::String | SceType::Bytes));
    let mut ctx = common_context(&l, m, imports, |e| match &e.sce_type {
        SceType::String => "&'a str".to_string(),
        SceType::Bytes => "&'a [u8]".to_string(),
        other => l.type_name(other).into_owned(),
    })?;
    if let Some(serde_json::Value::Array(entries)) = ctx.get_mut("entries") {
        for entry in entries.iter_mut() {
            let value_type = entry["value_type"].as_str().unwrap_or_default().to_string();
            entry["field_type"] = if entry["required"].as_bool() == Some(true) {
                value_type
            } else {
                format!("Option<{value_type}>")
            }
            .into();
        }
    }
    ctx.insert("lifetime".into(), if borrows { "<'a>" } else { "" }.into());
    ctx.insert(
        "cursor_lifetime".into(),
        if borrows { "'a" } else { "'_" }.into(),
    );
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
    l.render(env, "codec_cbor", ctx)
}
