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
use crate::forge::model::{CodecModel, ContentLineEntry, LineHeld, Measured, SceType};
use crate::generator::Language;

/// Whether `lang` generates a content-line codec — the one answer the
/// generator's refusal and the conformance harness's schedule both read, so a
/// fixture is run exactly where the generator admits it.
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

/// Why `lang` does not generate the content-line codec `m`, or `None` when it
/// does. Every backend generates the line record of a repeated property that
/// declares a parameter, the list of values a `sce:separator` makes of a line
/// (docs/adr/0014) and an entry that is an enum, written and read by the text of
/// its variants (docs/adr/0015), each against the same conformance vectors, so
/// none of those shapes is a ground for refusing one.
///
/// Beyond the language: C11 holds every text and list in an array the size of
/// what it holds, the deployment's capacity or the bound (docs/adr/0077 §7), so
/// a container with neither has no C11 storage — every other backend's text
/// and list grow.
pub fn refusal(lang: Language, m: &CodecModel) -> Option<String> {
    if !lowers(lang) {
        return Some(format!(
            "codec '{}' is sce:encoding=\"content-line\", which has no {lang:?} generation yet",
            m.name
        ));
    }
    if lang == Language::C11 {
        let lines = m.content_line.as_ref()?;
        if let Some(slot) = lines
            .slots()
            .into_iter()
            .find(|s| lines.entries[s.entry].storage(s.held).is_none())
        {
            return Some(format!(
                "codec '{}': content-line entry '{}' declares no {} and the deployment gives \
                 '{}' no capacity, and C11 holds what an entry holds in a fixed array of its \
                 capacity",
                m.name,
                lines.entries[slot.entry].id,
                slot.held.declaration(),
                slot.id
            ));
        }
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
        Language::Swift => render_swift(env, m, imports),
    }
}

/// What an entry's value is, as the templates branch on it.
fn kind(entry: &ContentLineEntry) -> &'static str {
    match entry.present_type() {
        SceType::String => "string",
        SceType::Bool => "bool",
        SceType::Enum(_) => "enum",
        ty if ty.is_signed() => "int",
        _ => "uint",
    }
}

/// What a template needs of an entry or parameter that is an enum, written and
/// read by the text of its variants (docs/adr/0015): the table it matches
/// against, the conversion of a carrier into the enum's own type, whether the
/// enum is open, and the expression that takes `value` to its carrier. Empty for
/// anything else.
fn enum_keys(l: &LangCtx, ty: &SceType, value: &str) -> serde_json::Map<String, serde_json::Value> {
    let mut keys = serde_json::Map::new();
    if let SceType::Enum(r) = ty {
        keys.insert("enum_alias".into(), r.alias.clone().into());
        keys.insert("enum_from".into(), l.enum_from_underlying(&r.alias).into());
        keys.insert("enum_to".into(), l.codec_carrier_expr(ty, value).into());
        keys.insert("enum_is_open".into(), l.enum_is_open(&r.alias).into());
        keys.insert("carrier_type".into(), l.enum_carrier_type(&r.alias).into());
        // A required entry starts at the first variant the enum declares, as a CBOR
        // enum field does: a closed set need not declare the zero a language would
        // start it at.
        keys.insert("enum_default".into(), l.enum_default_expr(&r.alias).into());
        // One more than the longest text, for a backend that reads a value into a
        // buffer of a size fixed at compile time: a value that fills it is longer
        // than any text, and names no variant.
        let longest = l
            .enum_texts(&r.alias)
            .iter()
            .map(|row| row.text.len())
            .max()
            .unwrap_or(0);
        keys.insert("enum_cap".into(), (longest + 1).into());
        // The conversion to the carrier as a function name, for a backend whose
        // holder is not one name: C11 reads a value out of `self->x[i]` or
        // `rec->x`, and the template writes the call over that holder.
        keys.insert(
            "enum_to_fn".into(),
            l.codec_carrier_expr(ty, "\u{1}")
                .trim_end_matches("(\u{1})")
                .to_string()
                .into(),
        );
    }
    keys
}

/// The tables of the enums the codec reads an entry by, one for each enum in
/// the order an entry first names it, as a template writes them out.
fn insert_enum_tables(
    ctx: &mut serde_json::Map<String, serde_json::Value>,
    l: &LangCtx,
    m: &CodecModel,
) {
    let model = m
        .content_line
        .as_ref()
        .expect("a content-line codec carries its entries");
    let mut tables: Vec<serde_json::Value> = Vec::new();
    let mut seen: Vec<&str> = Vec::new();
    for entry in &model.entries {
        let SceType::Enum(r) = entry.present_type() else {
            continue;
        };
        if seen.contains(&r.alias.as_str()) {
            continue;
        }
        seen.push(&r.alias);
        let rows: Vec<serde_json::Value> = l
            .enum_texts(&r.alias)
            .iter()
            .map(|row| serde_json::json!({ "text": row.text, "value": row.value.to_string() }))
            .collect();
        tables.push(serde_json::json!({ "alias": r.alias, "rows": rows }));
    }
    ctx.insert("enum_tables".into(), tables.into());
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

/// A bound as the language's reader and writer take it: the number, or the
/// runtime's name for no bound where the protocol states none (docs/adr/0077
/// §7). C11 takes a bound beside a capacity, both spelled by
/// [`c11_held_keys`].
pub(crate) fn bound_literal(lang: Language, bound: Option<u32>) -> String {
    match bound {
        Some(n) => n.to_string(),
        None => match lang {
            Language::Go => "codec.Unbounded",
            Language::Cpp => "::SCE::Forge::ContentLine::kUnbounded",
            Language::Kotlin => "ContentLine.UNBOUNDED",
            Language::Python => "content_line.UNBOUNDED",
            Language::Rust => "usize::MAX",
            Language::Swift => "ContentLine.unbounded",
            Language::C11 => {
                unreachable!("a backend that holds no unbounded text renders no literal for one")
            }
        }
        .to_string(),
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
    let value_type = |e: &ContentLineEntry| match e.present_type() {
        SceType::String => string_type(e.max_size.unwrap_or(0)),
        other => l.type_name(other).into_owned(),
    };
    // A bound as this language's reader and writer take it; C11 holds a number
    // of its own for each array and reads none of these.
    let literal = |bound: Option<u32>| -> Option<String> {
        match (l.language(), bound) {
            (Language::C11, None) => None,
            (lang @ Language::C11, Some(_)) => Some(bound_literal(lang, bound)),
            (
                lang @ (Language::Rust
                | Language::Cpp
                | Language::Kotlin
                | Language::Go
                | Language::Python
                | Language::Swift),
                bound,
            ) => Some(bound_literal(lang, bound)),
        }
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
                    let mut param = serde_json::json!({
                        "name": name,
                        "local": format!("field_{name}"),
                        "param": p.param,
                        "kind": kind(p),
                        // What a template branches on is whether a line without
                        // the parameter is refused: it is of a bare type
                        // (docs/adr/0055, section 8).
                        "required": p.is_required(),
                        "max_size": p.max_size,
                        "max_size_lit": literal(p.max_size),
                        "value_type": value_type(p),
                        "default": default_of(p.present_type()),
                    });
                    // A parameter's carrier is taken from a local of the line
                    // that holds the parameter, `pv`.
                    param
                        .as_object_mut()
                        .expect("a json object")
                        .extend(enum_keys(l, p.present_type(), "pv"));
                    param
                })
                .collect();
            let name = l.codec_field_id(&e.id);
            let mut entry = serde_json::json!({
                "name": name,
                "local": format!("field_{name}"),
                "property": e.property,
                "kind": kind(e),
                "text": e.text,
                "required": e.is_required(),
                "max_size": e.max_size,
                "max_size_lit": literal(e.max_size),
                "max_count": e.max_count,
                // One value a line, on several lines.
                "is_list": e.repeated && !is_records,
                // One line holding a list of values (docs/adr/0014).
                "is_values": model.reads_values(e),
                // Several lines, each a record of its parameters and its value or
                // values (docs/adr/0014). The parameters are members of the record
                // and not of the codec, and the record is a type of its own.
                "is_records": is_records,
                "separator": e.separator,
                "max_values": e.max_values,
                "max_values_lit": literal(e.max_values),
                "record_type": is_records.then(|| format!(
                    "{struct_name}{}Line",
                    crate::filters::to_pascal_case(e.id.clone())
                )),
                "value_member": l.codec_field_id(if e.separator.is_some() { "values" } else { "value" }),
                "value_type": value_type(e),
                "bits": int_bits(e.present_type()),
                "min": int_range(e.present_type()).map(|r| r.0),
                "max": int_range(e.present_type()).map(|r| r.1),
                "default": default_of(e.present_type()),
                // What a line record or a list starts its value at, kept apart from
                // `default`, which a template reads for the field that holds the
                // record or the list.
                "value_default": default_of(e.present_type()),
                "params": params,
            });
            entry
                .as_object_mut()
                .expect("a json object")
                .extend(enum_keys(l, e.present_type(), "v"));
            entry
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
            optional_type: &|t| l.codec_optional_of(t),
            optional_default: Some("null"),
        },
    );
    insert_component(&mut ctx, m);
    insert_enum_tables(&mut ctx, &l, m);
    ctx.insert("entries".into(), entries.into());
    l.render(env, "codec_content_line", ctx)
}

fn render_swift(
    env: &minijinja::Environment,
    m: &CodecModel,
    imports: &[ImportContext],
) -> Result<String, ForgeError> {
    let l = LangCtx::new(Language::Swift, imports);
    let mut ctx = l.base_context(&m.name);
    l.insert_imports(&mut ctx, imports);
    // Owned values, as every Swift codec holds them: a string is a `String`, a list an `Array`, and
    // an optional entry or parameter absent is `nil`. A required entry starts at its type's own
    // default (an enum at its first declared variant) and is filled in by decode.
    let mut entries = entries_context(
        &l,
        m,
        |_| "String".to_string(),
        |ty| Some(l.default_expr(ty)),
    );
    shape_fields(
        &mut entries,
        &FieldShapes {
            list_of: &|t| format!("[{t}]"),
            list_default: Some("[]"),
            optional_type: &|t| l.codec_optional_of(t),
            optional_default: Some("nil"),
        },
    );
    // The members of the struct in declaration order, which its memberwise initializer takes: an
    // entry, then the parameters declared for it unless they are members of its line record.
    let mut fields: Vec<serde_json::Value> = Vec::new();
    for entry in &entries {
        fields.push(serde_json::json!({
            "name": entry["name"],
            "field_type": entry["field_type"],
        }));
        if entry["is_records"].as_bool() != Some(true) {
            for param in entry["params"].as_array().into_iter().flatten() {
                fields.push(serde_json::json!({
                    "name": param["name"],
                    "field_type": param["field_type"],
                }));
            }
        }
    }
    insert_component(&mut ctx, m);
    insert_enum_tables(&mut ctx, &l, m);
    ctx.insert("entries".into(), entries.into());
    ctx.insert("fields".into(), fields.into());
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
    insert_enum_tables(&mut ctx, &l, m);
    // docs/adr/0081 step 4: the codec is printed from its plan, every entry
    // presented through an accessor and encoded over any source of it.
    ctx.insert(
        "cpp_source".into(),
        crate::forge::generator::cpp_content_line_printed(m, imports).into(),
    );
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
            optional_type: &|t| l.codec_optional_of(t),
            optional_default: Some("None"),
        },
    );
    insert_component(&mut ctx, m);
    insert_enum_tables(&mut ctx, &l, m);
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
    // slice, and an optional entry or parameter an `optional.Of[T]`, absent at its
    // zero value. A field starts at its zero value, which is the right start for
    // every kind this codec holds.
    let mut entries = entries_context(&l, m, |_| "string".to_string(), |_| None);
    shape_fields(
        &mut entries,
        &FieldShapes {
            list_of: &|t| format!("[]{t}"),
            list_default: None,
            optional_type: &|t| l.codec_optional_of(t),
            optional_default: None,
        },
    );
    // `math` is imported for the bounds of an integer entry's read, and only
    // then: an unused import is a compile error in Go.
    let uses_math = entries.iter().any(|e| e["bits"].is_u64());
    ctx.insert("uses_math".into(), uses_math.into());
    insert_component(&mut ctx, m);
    insert_enum_tables(&mut ctx, &l, m);
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

/// What a C11 codec holds of `e`, of each kind it holds a variable number of
/// (docs/adr/0077 §7): `size_*` of a text, `values_*` of a line of values,
/// `count_*` of a repeated property's lines. `*_bound` is the bound as the
/// runtime's readers take it, `SIZE_MAX` where the protocol states none, and
/// `*_cap` the capacity an array is sized at and a reader holds: the
/// deployment's, or the bound. A container with neither is refused before this
/// renders ([`refusal`]).
fn c11_held_keys(item: &mut serde_json::Value, e: &ContentLineEntry) {
    for held in e.held() {
        let key = match held {
            LineHeld::Text => "size",
            LineHeld::Values => "values",
            LineHeld::Lines => "count",
        };
        item[format!("{key}_bound")] = e
            .bound(held)
            .map_or_else(|| "SIZE_MAX".to_string(), |n| format!("{n}U"))
            .into();
        item[format!("{key}_cap")] = e
            .storage(held)
            .expect("C11 refuses a container it holds no number of before it renders")
            .into();
    }
}

/// How a C11 codec holds `item`, the context of an entry `e` or of one of its
/// parameters, by the member paths its template reads and writes it through. The
/// value of one that may be absent (`optional`) is the `value` of the struct
/// `sce/forge/optional.h` declares for its element, present when its `present`
/// member is true (docs/adr/0055, section 8), and the declaration that struct needs
/// first is added to `declarations` once:
///
/// - `value_ref`: the member that holds the value of a number, a `bool` or an enum;
/// - `data_ref`: the member that holds the data written to a line — the buffer of
///   a string, and the value itself for any other kind;
/// - `len_ref`: the member that holds the length of a string held beside its
///   buffer, none for one held as every text of the machine is, ended by its
///   terminator;
/// - `present_ref`: the member that says a value that may be absent is present.
fn c11_hold(
    l: &LangCtx,
    item: &mut serde_json::Value,
    e: &ContentLineEntry,
    optional: bool,
    declarations: &mut Vec<String>,
) {
    c11_held_keys(item, e);
    let name = item["name"].as_str().unwrap_or_default().to_string();
    let value = if optional {
        let held = l.c11_codec_optional(e.present_type(), e.storage(LineHeld::Text));
        if !held.declaration.is_empty() && !declarations.contains(&held.declaration) {
            declarations.push(held.declaration);
        }
        item["field_type"] = held.type_name.into();
        format!("{name}.value")
    } else {
        name.clone()
    };
    let (data, len) = match (e.present_type(), optional) {
        (SceType::String, true) => (format!("{value}.data"), None),
        (SceType::String, false) => (name.clone(), Some(format!("{name}_len"))),
        _ => (value.clone(), None),
    };
    item["optional_c"] = optional.into();
    item["value_ref"] = value.into();
    item["data_ref"] = data.into();
    item["len_ref"] = len.into();
    item["present_ref"] = format!("{name}.present").into();
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
    // A required string is a fixed array the size of its bound beside its
    // length, and a list an array of them beside a count. An entry or parameter
    // that may be absent is the struct `sce/forge/optional.h` declares for its
    // element (docs/adr/0055, section 5), the one a statechart's variable of the
    // same type is held in: no allocation, as the rest of the C11 runtime. An
    // entry with no bound is refused before this renders ([`refusal`]), so
    // every array here has one.
    let mut entries = entries_context(&l, m, |_| "char".to_string(), |_| None);
    let codec_snake = crate::filters::to_snake_case(m.name.clone());
    let content = m
        .content_line
        .as_ref()
        .expect("a content-line codec carries its entries");
    let mut declarations: Vec<String> = Vec::new();
    for (entry, e) in entries
        .iter_mut()
        .zip(content.entries.iter().filter(|e| e.param.is_none()))
    {
        // A list, a list of values and a line record are held by the shapes
        // their own members give them, never by the struct of one value.
        let alone = ["is_list", "is_values", "is_records"]
            .iter()
            .all(|key| entry[*key].as_bool() != Some(true));
        c11_hold(&l, entry, e, !e.is_required() && alone, &mut declarations);
        if let Some(serde_json::Value::Array(params)) = entry.get_mut("params") {
            for (param, p) in params.iter_mut().zip(content.parameters_of(e)) {
                c11_hold(&l, param, p, !p.is_required(), &mut declarations);
            }
        }
        // The parameters an encode of an absent property must find absent, by
        // the member each is present in.
        let optional_params: Vec<serde_json::Value> = entry["params"]
            .as_array()
            .map(|params| {
                params
                    .iter()
                    .filter(|p| p["required"].as_bool() != Some(true))
                    .map(|p| serde_json::json!({ "present_ref": p["present_ref"] }))
                    .collect()
            })
            .unwrap_or_default();
        entry["optional_params"] = optional_params.into();
        // A line record is a typedef of its own, named from the codec and the entry
        // as the codec's own typedef is (`<codec>_t`).
        if entry["is_records"].as_bool() == Some(true) {
            entry["record_type"] = format!(
                "{codec_snake}_{}_line_t",
                entry["name"].as_str().unwrap_or_default()
            )
            .into();
        }
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
    ctx.insert("optional_declarations".into(), declarations.into());
    insert_component(&mut ctx, m);
    insert_enum_tables(&mut ctx, &l, m);
    // One buffer serves every enum the codec reads, so it is the size of the
    // widest: one more than the longest text of any of them.
    let word_cap = m
        .content_line
        .as_ref()
        .into_iter()
        .flat_map(|c| c.entries.iter())
        .filter_map(|e| match e.present_type() {
            SceType::Enum(r) => l
                .enum_texts(&r.alias)
                .iter()
                .map(|row| row.text.len())
                .max(),
            _ => None,
        })
        .max()
        .map_or(0, |longest| longest + 1);
    ctx.insert("enum_word_cap".into(), word_cap.into());
    ctx.insert("entries".into(), entries.into());
    // What this node writes at most: each text and list at what it holds, the
    // deployment's capacity where it gives one.
    let max_encoded_bytes = max_encoded_bytes_with(
        m,
        &|alias: &str| {
            l.enum_texts(alias)
                .iter()
                .map(|row| row.text.len() as u64)
                .max()
                .unwrap_or(0)
        },
        Measured::Held,
    )
    .expect("C11 refuses a content-line entry it holds no number of before it renders");
    ctx.insert("max_encoded_bytes".into(), max_encoded_bytes.into());
    l.render(env, "codec_content_line", ctx)
}

/// The most bytes an entry's value, or a parameter's, takes on a line, before
/// escapes and folds. A string is held to its `sce:max-size`; an enum is written
/// as the text of a variant, so the longest of its texts (docs/adr/0015) —
/// without it a codec whose enum has a long text would be handed a buffer too
/// small for it, which `…_MAX_BYTES` promises never happens.
///
/// `longest_text` answers, for the alias of an enum the codec imports, the
/// length in bytes of the longest text of its variants. `None` for a string
/// with nothing to measure it at ([`measure`]).
fn value_width(
    longest_text: &dyn Fn(&str) -> u64,
    entry: &ContentLineEntry,
    measured: Measured,
) -> Option<u64> {
    match entry.present_type() {
        SceType::Enum(r) => Some(longest_text(&r.alias)),
        _ => measure(entry, LineHeld::Text, measured).map(u64::from),
    }
}

/// What `held` of `entry` is measured at: its bound, or what this node holds of
/// it (docs/adr/0077 §7). `None` where there is nothing to measure it at.
fn measure(entry: &ContentLineEntry, held: LineHeld, measured: Measured) -> Option<u32> {
    match measured {
        Measured::Bound => entry.bound(held),
        Measured::Held => entry.storage(held),
    }
}

/// The most bytes `m` encodes to, folds and escapes included: a bound a C11
/// caller sizes an `encode_to_buf` buffer by. Every entry present at its
/// largest — a TEXT with every character escaped, a list full — and every
/// logical line cut as often as a cut can fall (a line carries at least 71
/// payload octets between cuts, the widest unit being four).
///
/// The texts of an enum do not depend on the language a codec is rendered for
/// (an enum document gives each variant one wire text), so the bound is taken
/// over `longest_text`, the lookup of the longest text of an imported enum, and
/// not over a language's import table. The C11 renderer and the build-wide
/// pass that judges a broadcast log's element
/// ([`crate::forge::codec_size`]) both read this one function.
///
/// Measured at the bound, it is the largest frame of the language; held, the
/// largest this node writes, which a C11 caller sizes a buffer by. `None` where
/// an entry has nothing to measure it at: at the bound, one the protocol bounds
/// by nothing ([`ContentLineEntry::unbounded`]), and held, one the deployment
/// gives no capacity besides; a caller that needs a number refuses the codec by
/// that entry.
pub(crate) fn max_encoded_bytes_with(
    m: &CodecModel,
    longest_text: &dyn Fn(&str) -> u64,
    measured: Measured,
) -> Option<u64> {
    let Some(model) = m.content_line.as_ref() else {
        return Some(0);
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
            logical += 1 + name + 1 + 2 + value_width(longest_text, param, measured)?;
        }
        let value = match entry.present_type() {
            // An enum's text is letters, digits and hyphens: nothing is escaped.
            SceType::Enum(_) => value_width(longest_text, entry, measured)?,
            SceType::String => {
                // A line that holds a list carries every value at its largest and
                // a separator between each (docs/adr/0014); the separator is not
                // escaped, so it counts as one octet.
                let values = if entry.separator.is_some() {
                    u64::from(measure(entry, LineHeld::Values, measured)?)
                } else {
                    1
                };
                value_width(longest_text, entry, measured)?
                    * if entry.text { 2 } else { 1 }
                    * values
                    + (values - 1)
            }
            SceType::Bool => 5,
            _ => 20,
        };
        logical += 1 + value;
        let physical = logical + 2 + 3 * (logical / 70 + 1);
        let lines = if entry.repeated {
            u64::from(measure(entry, LineHeld::Lines, measured)?)
        } else {
            1
        };
        total += physical * lines;
    }
    Some(total)
}

/// The runtime's codec module, as a generated Rust codec spells it.
const RUST_RUNTIME: &str = "::sce_forge_runtime::codec";

/// What a storage profile holds of a content-line codec (docs/adr/0077 §7): a
/// tag for each text and each list, as the storage macro takes them
/// ([`crate::forge::generator::rust_storage_field`]), each held by `Inline` at
/// its bound, and the tags of the texts, which a decode builds into.
pub(crate) struct RustStorage {
    pub(crate) fields: Vec<serde_json::Value>,
    /// The tag of each text, the type a decode asks to be a `SceStr`.
    pub(crate) text_tags: Vec<String>,
}

/// The field id the tag of one value of the list `e` is named by: a text in a
/// list has a tag of its own, apart from the list's.
fn value_id(e: &ContentLineEntry) -> String {
    e.value_slot_id()
}

/// The field id the tag of a line record's list of values is named by.
fn values_id(e: &ContentLineEntry) -> String {
    e.values_slot_id()
}

/// The tags of the content-line codec `m` ([`RustStorage`]), one for each
/// container its storing form holds
/// ([`crate::forge::model::ContentLineModel::slots`]), each held at
/// the deployment's capacity or the bound: a text's `sce:max-size`, a list of
/// lines' `sce:max-count`, a line of values' `sce:max-values`.
pub(crate) fn rust_storage(m: &CodecModel) -> RustStorage {
    use crate::forge::generator::{rust_storage_field, rust_storage_tag};
    use crate::forge::model::LineHeld;
    let mut storage = RustStorage {
        fields: Vec::new(),
        text_tags: Vec::new(),
    };
    let Some(model) = m.content_line.as_ref() else {
        return storage;
    };
    for slot in model.slots() {
        let held = model.entries[slot.entry].storage(slot.held);
        let kind = if slot.held == LineHeld::Text {
            storage.text_tags.push(rust_storage_tag(&m.name, &slot.id));
            "Str"
        } else {
            "List"
        };
        storage
            .fields
            .push(rust_storage_field(&m.name, &slot.id, kind, held));
    }
    storage
}

/// A bound as the `Option<usize>` a content-line reader takes.
fn rust_bound_arg(bound: Option<u32>) -> String {
    bound.map_or_else(|| "None".to_string(), |n| format!("Some({n})"))
}

fn render_rust(
    env: &minijinja::Environment,
    m: &CodecModel,
    imports: &[ImportContext],
) -> Result<String, ForgeError> {
    use crate::forge::generator::rust_storage_tag;
    let l = LangCtx::new(Language::Rust, imports);
    let mut ctx = l.base_context(&m.name);
    l.insert_imports(&mut ctx, imports);
    let model = m
        .content_line
        .as_ref()
        .expect("a content-line codec carries its entries");
    // docs/adr/0065: whether the encode can refuse a value, which decides what
    // `encode` and `encode_to_vec` return.
    ctx.insert(
        "encode_may_fail".into(),
        crate::forge::codec_totality::may_fail(m, imports).into(),
    );
    // A value is held at a storage profile, each text and list in what the
    // profile answers for its tag (docs/adr/0077 §7). A content line is
    // unescaped and unfolded as it is read, so nothing borrows the frame.
    let str_ty = |id: &str| {
        format!(
            "<S as {RUST_RUNTIME}::HoldsStr<{}>>::Str",
            rust_storage_tag(&m.name, id)
        )
    };
    let list_ty = |id: &str, entry: &str| {
        format!(
            "<S as {RUST_RUNTIME}::HoldsList<{}>>::List<{entry}>",
            rust_storage_tag(&m.name, id)
        )
    };
    let mut entries = entries_context(&l, m, |_| String::new(), |_| None);
    let values = model.entries.iter().filter(|e| e.param.is_none());
    for (entry, e) in entries.iter_mut().zip(values) {
        let is_text = e.present_type() == &SceType::String;
        let plain = l.type_name(e.present_type()).into_owned();
        let required = e.is_required();
        // The type of one value of the entry, where it is one of a list.
        let one = if is_text {
            str_ty(&value_id(e))
        } else {
            plain.clone()
        };
        entry["max_size_arg"] = rust_bound_arg(e.max_size).into();
        entry["max_values_arg"] = rust_bound_arg(e.max_values).into();
        // A decode counts the lines of a repeated property where a count is
        // judged: past its `sce:max-count`, or none where it is required. The
        // list may hold fewer than were read, past a profile's capacity.
        entry["counts_lines"] = (e.repeated && (e.max_count.is_some() || e.is_required())).into();
        // The type a text read for this entry is built in: one of the list, or
        // the entry's own where it holds one value.
        entry["str_type"] = one.clone().into();
        let (field_type, local_type, local_init) = if model.reads_records(e) {
            // A line record is a struct of its own, and the field a list of
            // them. What its value member holds is one value, or the list a
            // separator makes.
            let record = format!("{}In<S>", entry["record_type"].as_str().unwrap_or_default());
            entry["value_field_type"] = if e.separator.is_some() {
                list_ty(&values_id(e), &one)
            } else {
                one.clone()
            }
            .into();
            entry["value_str_type"] = one.clone().into();
            let list = list_ty(&e.id, &record);
            (list.clone(), list, "Default::default()".to_string())
        } else if model.reads_values(e) {
            // One line holding a list: the field is the list, and its local an
            // option, so a second line of the property is told from the first.
            let list = list_ty(&e.id, &one);
            (list.clone(), l.codec_optional_of(&list), "None".to_string())
        } else if e.repeated {
            let list = list_ty(&e.id, &one);
            (list.clone(), list, "Default::default()".to_string())
        } else {
            let value = if is_text { str_ty(&e.id) } else { plain };
            entry["str_type"] = value.clone().into();
            let optional = l.codec_optional_of(&value);
            if required {
                (value, optional, "None".to_string())
            } else {
                (optional.clone(), optional, "None".to_string())
            }
        };
        entry["field_type"] = field_type.into();
        entry["local_type"] = local_type.into();
        entry["local_init"] = local_init.into();
        let params = model.parameters_of(e);
        if let Some(serde_json::Value::Array(rows)) = entry.get_mut("params") {
            for (param, p) in rows.iter_mut().zip(params) {
                let value = if p.present_type() == &SceType::String {
                    str_ty(&p.id)
                } else {
                    l.type_name(p.present_type()).into_owned()
                };
                param["str_type"] = value.clone().into();
                param["value_type"] = value.clone().into();
                param["max_size_arg"] = rust_bound_arg(p.max_size).into();
                param["field_type"] = if p.is_required() {
                    value
                } else {
                    l.codec_optional_of(&value)
                }
                .into();
            }
        }
    }
    let storage = rust_storage(m);
    let name = crate::filters::to_pascal_case(m.name.clone());
    ctx.insert(
        "text_bounds".into(),
        storage
            .text_tags
            .iter()
            .map(|tag| {
                format!("<S as {RUST_RUNTIME}::HoldsStr<{tag}>>::Str: {RUST_RUNTIME}::SceStr")
            })
            .collect::<Vec<_>>()
            .join(", ")
            .into(),
    );
    crate::forge::generator::insert_rust_storage(&mut ctx, &m.name, storage.fields, &[]);
    ctx.insert(
        "default_profile_holds".into(),
        crate::forge::generator::codec_inline_holds(m, imports).into(),
    );
    ctx.insert("storage_trait".into(), format!("{name}Storage").into());
    insert_component(&mut ctx, m);
    insert_enum_tables(&mut ctx, &l, m);
    ctx.insert("entries".into(), entries.into());
    ctx.insert(
        "codec_struct_derives_attr".into(),
        crate::rust_derive_policy::RustDeriveCategory::CodecStruct
            .derives_attr()
            .into(),
    );
    l.render(env, "codec_content_line", ctx)
}
