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
        Language::Rust
            | Language::Kotlin
            | Language::Cpp
            | Language::Go
            | Language::Python
            | Language::C11
    )
}

/// Why `lang` does not generate the CBOR codec `m`, or `None` when it does —
/// the one answer the generator's refusal and the conformance harness's
/// schedule both read, so a fixture is run exactly where it generates.
///
/// Beyond the language: C11 holds a text or byte string in a fixed array
/// the size of its bound, so an entry with neither `sce:max-size` nor
/// `sce:length` has no C11 storage — every other backend's string grows.
pub fn refusal(lang: Language, m: &CodecModel) -> Option<String> {
    if !lowers(lang) {
        return Some(format!(
            "codec '{}' is sce:encoding=\"cbor\", which has no {lang:?} generation yet",
            m.name
        ));
    }
    if lang == Language::C11 {
        if let Some(e) = m.cbor_entries.iter().find(|e| {
            matches!(e.sce_type, SceType::String | SceType::Bytes)
                && e.max_size.is_none()
                && e.length.is_none()
        }) {
            return Some(format!(
                "codec '{}': CBOR entry '{}' declares neither sce:max-size nor sce:length, \
                 and C11 holds a string in a fixed array the size of its bound",
                m.name, e.id
            ));
        }
    }
    None
}

/// Render a CBOR codec for `lang`.
///
/// A backend that cannot generate the document refuses it by name rather
/// than emitting a type with no fields: a codec that compiled and wrote an
/// empty map would be a wrong output, and this is a missing one.
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
    let required = m.cbor_entries.iter().filter(|e| e.required).count();
    ctx.insert("required_count".into(), required.into());
    ctx.insert(
        "optional_count".into(),
        (m.cbor_entries.len() - required).into(),
    );
    ctx.insert("entries".into(), entries.into());
    Ok(ctx)
}

/// Render `ctx` with `entries_by_key` beside `entries`: the same entries in
/// ascending key order, which is the order encode writes them (RFC 8949
/// §4.2.1) — `entries` keeps the author's declaration order. Taken here,
/// after a backend has added its own fields to each entry, so the two lists
/// never disagree about what an entry carries.
fn render_in_key_order(
    l: &LangCtx,
    env: &minijinja::Environment,
    mut ctx: serde_json::Map<String, serde_json::Value>,
) -> Result<String, ForgeError> {
    let mut by_key = match ctx.get("entries") {
        Some(serde_json::Value::Array(entries)) => entries.clone(),
        _ => Vec::new(),
    };
    by_key.sort_by_key(|e| e["key"].as_u64());
    ctx.insert("entries_by_key".into(), by_key.into());
    l.render(env, "codec_cbor", ctx)
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
    render_in_key_order(&l, env, ctx)
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
    render_in_key_order(&l, env, ctx)
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
    render_in_key_order(&l, env, ctx)
}

fn render_c(
    env: &minijinja::Environment,
    m: &CodecModel,
    imports: &[ImportContext],
) -> Result<String, ForgeError> {
    let l = LangCtx::new(Language::C11, imports);
    let mut ctx = common_context(&l, m, imports, |e| l.type_name(&e.sce_type).into_owned())?;
    crate::forge::generator::insert_c_codec_symbols(&mut ctx, &m.name);
    // A text or byte string is a fixed array the size of its bound beside
    // its length (the bound is present: `refusal` saw to it), an exact-length
    // one an array of that length alone; an optional entry carries a
    // `<name>_present` flag beside its value.
    if let Some(serde_json::Value::Array(entries)) = ctx.get_mut("entries") {
        for (entry, e) in entries.iter_mut().zip(&m.cbor_entries) {
            entry["capacity"] = e.length.or(e.max_size).into();
            // C11's encode reads the value in place rather than binding it,
            // so the conversion is spelled over the struct member.
            if matches!(e.sce_type, SceType::Enum(_)) {
                let member = format!("self->{}", entry["name"].as_str().unwrap_or_default());
                entry["enum_to_underlying"] = l.codec_carrier_expr(&e.sce_type, &member).into();
            }
        }
    }
    ctx.insert("max_encoded_bytes".into(), max_encoded_bytes(m).into());
    render_in_key_order(&l, env, ctx)
}

/// The most bytes `m` encodes to: its map head and, for every entry, its key
/// and the longest value its declaration admits — a bound C11 callers size
/// an `encode_to_buf` buffer by.
fn max_encoded_bytes(m: &CodecModel) -> u64 {
    // The length of a head carrying `value`, in its shortest form.
    fn head(value: u64) -> u64 {
        match value {
            0..=23 => 1,
            24..=0xFF => 2,
            0x100..=0xFFFF => 3,
            0x1_0000..=0xFFFF_FFFF => 5,
            _ => 9,
        }
    }
    let entries = head(m.cbor_entries.len() as u64);
    entries
        + m.cbor_entries
            .iter()
            .map(|e| {
                let key = head(u64::from(e.key));
                let value = match e.sce_type {
                    SceType::Bool => 1,
                    SceType::String | SceType::Bytes => {
                        let n = u64::from(e.length.or(e.max_size).unwrap_or(0));
                        head(n) + n
                    }
                    _ => 9,
                };
                key + value
            })
            .sum::<u64>()
}

fn render_python(
    env: &minijinja::Environment,
    m: &CodecModel,
    imports: &[ImportContext],
) -> Result<String, ForgeError> {
    let l = LangCtx::new(Language::Python, imports);
    let mut ctx = common_context(&l, m, imports, |e| l.type_name(&e.sce_type).into_owned())?;
    // A required entry starts at its type's own default — an enum at its
    // first declared variant — and an optional one absent (`None`).
    if let Some(serde_json::Value::Array(entries)) = ctx.get_mut("entries") {
        for (entry, e) in entries.iter_mut().zip(&m.cbor_entries) {
            let value_type = entry["value_type"].as_str().unwrap_or_default().to_string();
            entry["field_type"] = if e.required {
                value_type
            } else {
                format!("Optional[{value_type}]")
            }
            .into();
            entry["default"] = if e.required {
                l.default_expr(&e.sce_type)
            } else {
                "None".to_string()
            }
            .into();
        }
    }
    ctx.insert(
        "has_closed_enum".into(),
        m.cbor_entries
            .iter()
            .any(|e| matches!(&e.sce_type, SceType::Enum(r) if !l.enum_is_open(&r.alias)))
            .into(),
    );
    render_in_key_order(&l, env, ctx)
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
    render_in_key_order(&l, env, ctx)
}
