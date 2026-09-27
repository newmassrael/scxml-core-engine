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
use crate::generator::Language;

/// Whether `lang` generates a CBOR codec — the one answer the generator's
/// refusal and the conformance harness's schedule both read, so a fixture
/// is run exactly where the generator admits it (the `lowers_may_fail`
/// pattern).
pub fn lowers(lang: Language) -> bool {
    matches!(lang, Language::Rust)
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
        _ => unreachable!("`lowers` admits Rust alone"),
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

fn render_rust(
    env: &minijinja::Environment,
    m: &CodecModel,
    imports: &[ImportContext],
) -> Result<String, ForgeError> {
    let l = LangCtx::new(Language::Rust, imports);
    let mut ctx = l.base_context(&m.name);
    l.insert_imports(&mut ctx, imports);

    // A string or byte string is a view of the input, so the struct borrows
    // only when one of its entries does.
    let borrows = m
        .cbor_entries
        .iter()
        .any(|e| matches!(e.sce_type, SceType::String | SceType::Bytes));
    let mut entries: Vec<serde_json::Value> = Vec::new();
    for e in &m.cbor_entries {
        let value_type = match &e.sce_type {
            SceType::String => "&'a str".to_string(),
            SceType::Bytes => "&'a [u8]".to_string(),
            other => l.type_name(other).into_owned(),
        };
        let field_type = if e.required {
            value_type.clone()
        } else {
            format!("Option<{value_type}>")
        };
        let mut entry = serde_json::json!({
            "name": e.id,
            "key": e.key,
            "required": e.required,
            "kind": kind(e),
            "value_type": value_type,
            "field_type": field_type,
            "length": e.length,
            "max_size": e.max_size,
        });
        if let SceType::Enum(r) = &e.sce_type {
            entry["enum_from_underlying"] = l.enum_from_underlying(&r.alias).into();
            entry["enum_to_underlying"] = l.codec_carrier_expr(&e.sce_type, "v").into();
            entry["enum_is_open"] = l.enum_is_open(&r.alias).into();
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
