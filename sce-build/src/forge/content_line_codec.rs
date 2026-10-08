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
    matches!(lang, Language::Rust)
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
                "params": params,
            })
        })
        .collect()
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
    let mut entries = entries_context(&l, m, |n| format!("heapless::String<{n}>"));
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
    ctx.insert(
        "component".into(),
        m.content_line
            .as_ref()
            .map(|c| c.component.clone())
            .unwrap_or_default()
            .into(),
    );
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
