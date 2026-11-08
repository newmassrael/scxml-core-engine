// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
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
use crate::forge::generator::{
    insert_rust_storage, rust_storage_field, rust_storage_tag, rust_storage_trait, ImportContext,
    LangCtx, ORIGIN_PARAM, OWNED_IN_ORIGIN_FN,
};
use crate::forge::model::{CborEntry, CodecModel, Measured, SceType};

// ⚠ An entry's `name` is the backend's field identifier
// (`LangCtx::codec_field_id` — snake_case in Rust), the same spelling a
// positional codec gives its fields; the map key is `key`, never the name.
use crate::generator::Language;

/// Whether `lang` generates a CBOR codec — the one answer the generator's
/// refusal and the conformance harness's schedule both read, so a fixture
/// is run exactly where the generator admits it (the `lowers_may_fail`
/// pattern).
pub fn lowers(lang: Language) -> bool {
    match lang {
        Language::Rust
        | Language::Kotlin
        | Language::Cpp
        | Language::Go
        | Language::Python
        | Language::C11
        | Language::Swift => true,
    }
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
    if lang == Language::Swift {
        if let Some(what) = crate::forge::generator::swift_cbor_unlowered(m) {
            return Some(format!(
                "codec '{}': {what} has no Swift lowering yet",
                m.name
            ));
        }
    }
    if lang == Language::C11 {
        if let Some(e) = m.cbor_entries.iter().find(|e| {
            matches!(e.present_type(), SceType::String | SceType::Bytes)
                && e.storage_bytes().is_none()
        }) {
            return Some(format!(
                "codec '{}': CBOR entry '{}' declares neither sce:max-size nor sce:length \
                 and the deployment gives it no capacity, and C11 holds a string in a fixed \
                 array of its capacity",
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
        Language::Swift => render_swift(env, m, imports),
    }
}

/// What an entry is on the wire, as the templates branch on it.
pub(crate) fn kind(entry: &CborEntry) -> &'static str {
    match entry.present_type() {
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

/// Whether a string or byte-string entry of the CBOR codec `m` has neither a
/// bound nor a capacity: the protocol states no `sce:max-size` or `sce:length`
/// of it and the deployment gives it none, so no container of a fixed size holds
/// it (docs/adr/0077 §7).
///
/// What a backend that stores in fixed containers reads to refuse the codec,
/// and what the broadcast-log fact
/// [`crate::BroadcastLogElementFacts::cbor_entry_unbounded`] is read from: a
/// log's Rust slot holds its element at `Inline`.
pub fn has_unbounded_text_or_bytes(m: &CodecModel) -> bool {
    m.cbor_entries.iter().any(|e| {
        matches!(e.present_type(), SceType::String | SceType::Bytes) && e.storage_bytes().is_none()
    })
}

/// Whether the Rust form of the CBOR codec `m` has an owned mirror,
/// `{Name}Owned<S>` (SCE_FORGE.md §4.6.1): it decodes a string or a byte string to
/// a view of the buffer, so the map borrows, and a mirror is what a consumer that
/// keeps a decoded value past its buffer copies it into. The mirror holds each
/// such entry in the profile's container for its tag, so a map whose entry has
/// no bound has one as every other map does, and `Inline` alone does not answer
/// for that entry (docs/adr/0077 §7). A map of scalars borrows nothing and is its
/// own owned form.
pub fn rust_owned_mirror(m: &CodecModel) -> bool {
    m.cbor_entries
        .iter()
        .any(|e| matches!(e.present_type(), SceType::String | SceType::Bytes))
}

/// The storage-profile parameter of the Rust owned mirror: the generic parameter
/// every `{Name}Owned<S>` is declared over, the one the positional codec's mirror
/// uses (`STORAGE_PARAM` in the generator).
pub(crate) const RUST_STORAGE_PARAM: &str = "S";

/// The module path the generated Rust codec reaches the runtime's storage policy
/// through, spelled as the positional codec's mirror spells it (`RUNTIME_CODEC` in
/// the generator).
pub(crate) const RUST_RUNTIME_CODEC: &str = "::sce_forge_runtime::codec";

/// The bounds a string or byte string entry's value is held to by its Rust type
/// (docs/adr/0065 §6 R1, docs/adr/0077 §3), as the const arguments of `Bytes`
/// or `Text` after the lifetime: `sce:max-size` for a string or byte string
/// that states one, `{ usize::MAX }` for one that states none, so every string
/// has one shape as a positional field's does (§7), and `sce:length` as both
/// the most and the fewest for an exact byte string. `None` for any other entry.
pub(crate) fn rust_bound_args(e: &CborEntry) -> Option<String> {
    match kind(e) {
        "text" | "bytes" => Some(
            e.max_size
                .map_or_else(|| "{ usize::MAX }".to_string(), |max| max.to_string()),
        ),
        "bytes_exact" => e.length.map(|n| format!("{n}, {n}")),
        _ => None,
    }
}

/// The Rust type of the present value of `e` borrowed for `lifetime`: a `Text`
/// or `Bytes` of its bounds ([`rust_bound_args`]), and any other entry as the
/// language names it. The one spelling the decoded map and its source interface
/// share.
pub(crate) fn rust_view_value_type(l: &LangCtx, e: &CborEntry, lifetime: &str) -> String {
    let text = matches!(e.present_type(), SceType::String);
    match (e.present_type(), rust_bound_args(e)) {
        (SceType::String | SceType::Bytes, Some(bounds)) => format!(
            "{RUST_RUNTIME_CODEC}::{}<{lifetime}, {bounds}>",
            if text { "Text" } else { "Bytes" }
        ),
        (other, _) => l.type_name(other).into_owned(),
    }
}

/// The type the Rust owned mirror holds the present value of `e` as: a string in
/// the profile's text container and a byte string in its byte container, each
/// the profile's for the entry's tag, and any other entry as the view holds it.
///
/// The container is held inside a `Bounded` of the entry's bounds, as a
/// positional codec's is (docs/adr/0077 §3), so the mirror's source read lends
/// it with its bounds and checks nothing.
pub(crate) fn rust_owned_value_type(l: &LangCtx, m: &CodecModel, e: &CborEntry) -> String {
    let kind = match e.present_type() {
        SceType::String => "Str",
        SceType::Bytes => "Bytes",
        other => return l.type_name(other).into_owned(),
    };
    let tag = rust_storage_tag(&m.name, &e.id);
    let container =
        format!("<{RUST_STORAGE_PARAM} as {RUST_RUNTIME_CODEC}::Holds{kind}<{tag}>>::{kind}");
    let bounds = rust_bound_args(e).expect("a string or byte string entry has bounds");
    format!("{RUST_RUNTIME_CODEC}::Bounded<{container}, {bounds}>")
}

/// The string and byte string entries of the map `m` as the storage macro takes
/// them ([`rust_storage_field`]): each one a profile answers for, held by
/// `Inline` at what a storing form holds of it ([`CborEntry::storage_bytes`]),
/// and by no `Inline` impl where that is nothing. A map with no owned mirror
/// ([`rust_owned_mirror`]) declares none.
pub(crate) fn rust_storage_fields(m: &CodecModel) -> Vec<serde_json::Value> {
    if !rust_owned_mirror(m) {
        return Vec::new();
    }
    m.cbor_entries
        .iter()
        .filter_map(|e| {
            let kind = match e.present_type() {
                SceType::String => "Str",
                SceType::Bytes => "Bytes",
                _ => return None,
            };
            Some(rust_storage_field(&m.name, &e.id, kind, e.storage_bytes()))
        })
        .collect()
}

/// The expression that builds the present value of `e` for the Rust owned mirror
/// from the view's field `field` and the origin parameter: a string or a byte
/// string through the profile's `OriginStorage` (`str_from` or `bytes_from`, at the
/// entry's capacity), an entry that may be absent built only when present, and any
/// other entry moved as `try_into_owned_in` moves it. A fallible build is written
/// with its `?`, so the expression is the field's value.
fn rust_owned_origin_expr(m: &CodecModel, e: &CborEntry, field: &str) -> String {
    let (constructor, held) = match e.present_type() {
        SceType::String => ("str_from", "from_built_text"),
        SceType::Bytes => ("bytes_from", "from_built"),
        _ => return field.to_string(),
    };
    // The profile builds the container from the view's bytes, and the bounds
    // check the length it built once (`Bounded::from_built`).
    let tag = rust_storage_tag(&m.name, &e.id);
    let build = |arg: &str| {
        format!(
            "{RUST_RUNTIME_CODEC}::Bounded::{held}(<{RUST_STORAGE_PARAM} as {RUST_RUNTIME_CODEC}::OriginStorage>::{constructor}::<{tag}>({ORIGIN_PARAM}, {arg}.into_inner()))",
        )
    };
    if e.is_required() {
        format!("{}?", build(field))
    } else {
        format!("{field}.map(|_v| {}).transpose()?", build("_v"))
    }
}

/// The carrier type of the enum `alias` names, as the import resolved it.
pub(crate) fn enum_carrier(imports: &[ImportContext], alias: &str) -> Option<SceType> {
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
            // What a template branches on is whether the map must hold the key:
            // the entry is of a bare type (docs/adr/0055, section 8).
            "required": e.is_required(),
            "kind": kind(e),
            "value_type": value_type(e),
            "length": e.length,
            "max_size": e.max_size,
            "max": int_max(e.present_type()),
        });
        if let SceType::Enum(r) = e.present_type() {
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
            entry["enum_to_underlying"] = l.codec_carrier_expr(e.present_type(), "v").into();
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
    let required = m.cbor_entries.iter().filter(|e| e.is_required()).count();
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
    let mut ctx = common_context(&l, m, imports, |e| {
        l.type_name(e.present_type()).into_owned()
    })?;
    // A required entry starts at its type's own default — for an enum the
    // first declared variant — and is filled in by decode; an optional one
    // starts absent.
    if let Some(serde_json::Value::Array(entries)) = ctx.get_mut("entries") {
        for (entry, e) in entries.iter_mut().zip(&m.cbor_entries) {
            entry["default"] = if e.is_required() {
                l.default_expr(e.present_type())
            } else {
                "null".to_string()
            }
            .into();
        }
    }
    render_in_key_order(&l, env, ctx)
}

fn render_swift(
    env: &minijinja::Environment,
    m: &CodecModel,
    imports: &[ImportContext],
) -> Result<String, ForgeError> {
    let l = LangCtx::new(Language::Swift, imports);
    // Owned values, as every Swift codec holds them: a text string is a `String` and a byte string a
    // `[UInt8]`. A required entry starts at its type's zero, an enum at its first declared variant,
    // and is filled in by decode; an optional one starts absent, so `{Name}()` is the infallible
    // initializer every codec offers.
    let mut ctx = common_context(&l, m, imports, |e| {
        l.type_name(e.present_type()).into_owned()
    })?;
    if let Some(serde_json::Value::Array(entries)) = ctx.get_mut("entries") {
        for (entry, e) in entries.iter_mut().zip(&m.cbor_entries) {
            entry["default"] = if !e.is_required() {
                "nil".to_string()
            } else {
                match (kind(e), e.present_type()) {
                    ("bool", _) => "false".to_string(),
                    ("text", _) => "\"\"".to_string(),
                    ("bytes" | "bytes_exact", _) => "[]".to_string(),
                    ("enum", SceType::Enum(r)) => l.enum_default_expr(&r.alias),
                    _ => "0".to_string(),
                }
            }
            .into();
            // The common context spells the conversion of an enum's value as the variable `v`; the
            // Swift template names the value it writes `value`.
            if let SceType::Enum(r) = e.present_type() {
                entry["enum_to_underlying"] =
                    l.codec_carrier_expr(e.present_type(), "value").into();
                // An open enum is a struct over its carrier that holds any number, whose
                // `fromUnderlying` is total; a closed enum answers nil for a number no variant
                // declares, which a decode refuses.
                entry["enum_is_open"] = l.enum_is_open(&r.alias).into();
            }
            // A negative number is no CBOR unsigned integer, which encode refuses and does not wrap.
            entry["signed"] = matches!(
                e.present_type(),
                SceType::Int8 | SceType::Int16 | SceType::Int32 | SceType::Int64
            )
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
    let mut ctx = common_context(&l, m, imports, |e| {
        l.type_name(e.present_type()).into_owned()
    })?;
    // docs/adr/0081 step 4: a map whose plan the C++ printer lowers is printed
    // from it, its entries presented through accessors and encoded over any
    // source; any other keeps the template's struct whole.
    if let Some(printed) = crate::forge::generator::cpp_cbor_printed(m, imports) {
        ctx.insert("cpp_source".into(), printed.into());
    }
    // A value-initialized member holds its type's zero, which is the right
    // start for every entry but an enum's: a closed set need not declare the
    // carrier's zero, so an enum starts at its first declared variant — the
    // one default C++ codecs spell (`LangCtx::default_expr`). An optional
    // entry starts absent.
    if let Some(serde_json::Value::Array(entries)) = ctx.get_mut("entries") {
        for (entry, e) in entries.iter_mut().zip(&m.cbor_entries) {
            let value_type = entry["value_type"].as_str().unwrap_or_default().to_string();
            entry["field_type"] = if e.is_required() {
                value_type
            } else {
                l.codec_optional_of(&value_type)
            }
            .into();
            entry["init"] = match e.present_type() {
                SceType::Enum(r) if e.is_required() => l.enum_default_expr(&r.alias),
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
    // Owned values, as every Go codec holds them. An optional entry is the one
    // optional type (docs/adr/0055, section 5), so an absent byte string and a
    // present empty one stay two values; a required one is the value itself.
    let mut ctx = common_context(&l, m, imports, |e| {
        l.type_name(e.present_type()).into_owned()
    })?;
    if let Some(serde_json::Value::Array(entries)) = ctx.get_mut("entries") {
        for (entry, e) in entries.iter_mut().zip(&m.cbor_entries) {
            let value_type = entry["value_type"].as_str().unwrap_or_default().to_string();
            let optional = l.codec_optional_of(&value_type);
            // A decode holds every entry it reads as an optional value until
            // the map has been read to its end: a key given twice, and a
            // required key that was never given, are told from the value.
            entry["held_type"] = optional.clone().into();
            entry["field_type"] = if e.is_required() {
                value_type
            } else {
                optional
            }
            .into();
            // A required enum starts at its first declared variant, as the
            // other backends' do: a closed set need not declare the zero Go
            // would otherwise start it at.
            entry["init"] = match e.present_type() {
                SceType::Enum(r) if e.is_required() => l.enum_default_expr(&r.alias),
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
    let mut ctx = common_context(&l, m, imports, |e| {
        l.type_name(e.present_type()).into_owned()
    })?;
    crate::forge::generator::insert_c_codec_symbols(&mut ctx, &m.name);
    // A required text or byte string is a fixed array of its capacity beside
    // its length (the capacity is the deployment's or the bound, and present:
    // `refusal` saw to it), an exact-length one an array of that length alone.
    // A read refuses a length past the bound as a frame of no protocol and one
    // past the capacity as one this node cannot hold (docs/adr/0077 §7), so the
    // bound is written apart, `SIZE_MAX` where the protocol states none. An entry that may be
    // absent is the struct `sce/forge/optional.h` declares for its element
    // (docs/adr/0055, section 5), the one a statechart's variable of the same
    // type is held in: its value is the member `value`, a text the buffer of
    // its bound and a byte string that buffer and its length. The template
    // reads each entry through the member paths set here, so the one place
    // that knows how an entry is held is this one.
    let mut declarations: Vec<String> = Vec::new();
    if let Some(serde_json::Value::Array(entries)) = ctx.get_mut("entries") {
        for (entry, e) in entries.iter_mut().zip(&m.cbor_entries) {
            let name = entry["name"].as_str().unwrap_or_default().to_string();
            let capacity = e.storage_bytes();
            entry["capacity"] = capacity.into();
            entry["bound"] = e
                .max_size
                .map_or_else(|| "SIZE_MAX".to_string(), |max| format!("{max}U"))
                .into();
            let optional = !e.is_required();
            let value = if optional {
                let held = l.c11_codec_optional(e.present_type(), capacity);
                if !held.declaration.is_empty() && !declarations.contains(&held.declaration) {
                    declarations.push(held.declaration);
                }
                entry["field_type"] = held.type_name.into();
                format!("{name}.value")
            } else {
                name.clone()
            };
            let (data, len) = match (e.present_type(), optional) {
                (SceType::String, true) => (format!("{value}.data"), None),
                (SceType::Bytes, true) => (format!("{value}.data"), Some(format!("{value}.len"))),
                (SceType::String | SceType::Bytes, false) => {
                    (name.clone(), Some(format!("{name}_len")))
                }
                _ => (value.clone(), None),
            };
            entry["value_ref"] = value.clone().into();
            entry["data_ref"] = data.into();
            entry["len_ref"] = len.into();
            entry["present_ref"] = format!("{name}.present").into();
            // C11's encode reads the value in place rather than binding it,
            // so the conversion is spelled over the struct member.
            if matches!(e.present_type(), SceType::Enum(_)) {
                entry["enum_to_underlying"] = l
                    .codec_carrier_expr(e.present_type(), &format!("self->{value}"))
                    .into();
            }
        }
    }
    ctx.insert("optional_declarations".into(), declarations.into());
    // What this node's encode writes at most: each string at what it holds,
    // the deployment's capacity where it gives one.
    let max_encoded_bytes = max_encoded_bytes(m, Measured::Held)
        .expect("C11 refuses a CBOR string it holds no number of before it renders");
    ctx.insert("max_encoded_bytes".into(), max_encoded_bytes.into());
    render_in_key_order(&l, env, ctx)
}

/// The most bytes `m` encodes to: its map head and, for every entry, its key
/// and the longest value it admits, measured as `measured` says — at the
/// bound, the largest frame of the language; held, a size C11 callers size an
/// `encode_to_buf` buffer by.
///
/// `None` where a string or byte string has nothing to measure it at: no bound
/// and, held, no capacity. The build-wide pass that judges a broadcast log's
/// element ([`crate::forge::codec_size`]) refuses such an entry by name before it
/// asks, and C11 refuses it before it renders.
pub(crate) fn max_encoded_bytes(m: &CodecModel, measured: Measured) -> Option<u64> {
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
    let mut total = head(m.cbor_entries.len() as u64);
    for e in &m.cbor_entries {
        let key = head(u64::from(e.key));
        let value = match e.present_type() {
            SceType::Bool => 1,
            SceType::String | SceType::Bytes => {
                let n = u64::from(match measured {
                    Measured::Bound => e.length.or(e.max_size),
                    Measured::Held => e.storage_bytes(),
                }?);
                head(n) + n
            }
            _ => 9,
        };
        total += key + value;
    }
    Some(total)
}

fn render_python(
    env: &minijinja::Environment,
    m: &CodecModel,
    imports: &[ImportContext],
) -> Result<String, ForgeError> {
    let l = LangCtx::new(Language::Python, imports);
    let mut ctx = common_context(&l, m, imports, |e| {
        l.type_name(e.present_type()).into_owned()
    })?;
    // A required entry starts at its type's own default — an enum at its
    // first declared variant — and an optional one absent (`None`).
    if let Some(serde_json::Value::Array(entries)) = ctx.get_mut("entries") {
        for (entry, e) in entries.iter_mut().zip(&m.cbor_entries) {
            let value_type = entry["value_type"].as_str().unwrap_or_default().to_string();
            entry["field_type"] = if e.is_required() {
                value_type
            } else {
                l.codec_optional_of(&value_type)
            }
            .into();
            entry["default"] = if e.is_required() {
                l.default_expr(e.present_type())
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
            .any(|e| matches!(e.present_type(), SceType::Enum(r) if !l.enum_is_open(&r.alias)))
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
    render_in_key_order(&l, env, rust_context(&l, m, imports)?)
}

/// The origin-aware projection of the Rust owned mirror, `try_into_owned_in_origin`
/// (`--owned-origin`), as the text that follows the module the default output is,
/// or `None` for a map with no mirror, which has no projection to add.
///
/// A template of its own, rendered from the context the module is, because the
/// caller appends it after the codec's source interface: the module without the
/// option is then a prefix of the module with it, the property every other
/// generated owned form holds. The signature, the names and the origin parameter
/// are the positional codec's (`OWNED_IN_ORIGIN_FN`, `ORIGIN_PARAM`).
pub fn render_rust_owned_origin(
    env: &minijinja::Environment,
    m: &CodecModel,
    imports: &[ImportContext],
) -> Result<Option<String>, ForgeError> {
    if !rust_owned_mirror(m) {
        return Ok(None);
    }
    let l = LangCtx::new(Language::Rust, imports);
    let mut ctx = rust_context(&l, m, imports)?;
    let origin_storage = format!("{RUST_RUNTIME_CODEC}::OriginStorage");
    let storage = rust_storage_trait(&m.name);
    ctx.insert("owned_in_origin_fn".into(), OWNED_IN_ORIGIN_FN.into());
    ctx.insert("origin_param".into(), ORIGIN_PARAM.into());
    ctx.insert(
        "origin_ref_type".into(),
        format!("&<{RUST_STORAGE_PARAM} as {origin_storage}>::Origin").into(),
    );
    ctx.insert(
        "origin_generics_impl".into(),
        format!("<{RUST_STORAGE_PARAM}: {storage} + {origin_storage}>").into(),
    );
    if let Some(serde_json::Value::Array(entries)) = ctx.get_mut("entries") {
        for (entry, e) in entries.iter_mut().zip(&m.cbor_entries) {
            let field = format!("self.{}", entry["name"].as_str().unwrap_or_default());
            entry["owned_origin_expr"] = rust_owned_origin_expr(m, e, &field).into();
        }
    }
    l.render(env, "codec_cbor_origin", ctx).map(Some)
}

/// The context the Rust module is rendered from, with the owned mirror's names and
/// each entry's owned type when the map has a mirror.
fn rust_context(
    l: &LangCtx,
    m: &CodecModel,
    imports: &[ImportContext],
) -> Result<serde_json::Map<String, serde_json::Value>, ForgeError> {
    // A string or byte string is a view of the input, so the struct borrows
    // only when one of its entries does.
    let borrows = m
        .cbor_entries
        .iter()
        .any(|e| matches!(e.present_type(), SceType::String | SceType::Bytes));
    let mut ctx = common_context(l, m, imports, |e| rust_view_value_type(l, e, "'a"))?;
    // docs/adr/0065: whether the encode can refuse a value, which decides what
    // `encode` and `encode_to_vec` return.
    ctx.insert(
        "encode_may_fail".into(),
        crate::forge::codec_totality::may_fail(m, imports).into(),
    );
    if let Some(serde_json::Value::Array(entries)) = ctx.get_mut("entries") {
        for (entry, e) in entries.iter_mut().zip(&m.cbor_entries) {
            let value_type = entry["value_type"].as_str().unwrap_or_default().to_string();
            entry["field_type"] = if entry["required"].as_bool() == Some(true) {
                value_type
            } else {
                l.codec_optional_of(&value_type)
            }
            .into();
            // A string or byte string held within its bounds is made through
            // the type's one door once read, and the door's refusal is the
            // error the read gives for the same value (docs/adr/0077 §3).
            if rust_bound_args(e).is_some() {
                let refusal = if kind(e) == "bytes_exact" {
                    "CborWrongLength"
                } else {
                    "CborOutOfRange"
                };
                entry["bounded_refusal"] = refusal.into();
            }
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
    // The owned mirror: only a map that borrows and can name a capacity for each
    // string and byte string it holds has one (`rust_owned_mirror`). The names are
    // the positional codec's, so a consumer reaches either mirror the same way.
    let mirror = rust_owned_mirror(m);
    ctx.insert("emit_owned".into(), mirror.into());
    if mirror {
        let storage = rust_storage_trait(&m.name);
        insert_rust_storage(&mut ctx, &m.name, rust_storage_fields(m), &[]);
        // The mirror defaults to the build's profile only where `Inline`, that
        // profile on the heap-free tier, holds the map: a struct's default is
        // checked against its bound (docs/adr/0077 §7).
        let default_holds = crate::forge::generator::codec_inline_holds(m, imports);
        ctx.insert("default_profile_holds".into(), default_holds.into());
        let default = if default_holds {
            format!(" = {RUST_RUNTIME_CODEC}::DefaultStorage")
        } else {
            String::new()
        };
        ctx.insert(
            "owned_generics_decl".into(),
            format!("<{RUST_STORAGE_PARAM}: {storage}{default}>").into(),
        );
        ctx.insert(
            "owned_generics_impl".into(),
            format!("<{RUST_STORAGE_PARAM}: {storage}>").into(),
        );
        ctx.insert(
            "owned_generics_args".into(),
            format!("<{RUST_STORAGE_PARAM}>").into(),
        );
        let name = crate::filters::to_pascal_case(m.name.clone());
        ctx.insert(
            "owned_in_generics".into(),
            format!("<{RUST_STORAGE_PARAM}: {name}From<'a>>").into(),
        );
        ctx.insert(
            "codec_storage_path".into(),
            format!("for<'x> {name}From<'x>").into(),
        );
        ctx.insert(
            "codec_owned_derives_attr".into(),
            crate::rust_derive_policy::RustDeriveCategory::CodecStruct
                .derives_attr()
                .into(),
        );
        if let Some(serde_json::Value::Array(entries)) = ctx.get_mut("entries") {
            for (entry, e) in entries.iter_mut().zip(&m.cbor_entries) {
                let owned = rust_owned_value_type(l, m, e);
                entry["storage_tag"] = rust_storage_tag(&m.name, &e.id).into();
                entry["owned_field_type"] = if e.is_required() {
                    owned
                } else {
                    l.codec_optional_of(&owned)
                }
                .into();
            }
        }
    }
    Ok(ctx)
}
