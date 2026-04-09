// SPDX-License-Identifier: LGPL-2.1-or-later OR LicenseRef-SCE-Commercial
//
// SCE Forge code generator — renders kind-specific Jinja2 templates.
//
// Dispatches ForgeDocument to the appropriate template per kind and target
// language. Type mappings live here (not in the model) to preserve SRP.

use crate::filters;
use crate::forge::expr::{self, ExprTarget};
use crate::forge::model::*;
use crate::generator::{self, GeneratedOutput};
use std::path::Path;
use std::sync::LazyLock;

// ── Cross-language type mapping (SRP: lives in generator, not model) ──

/// Map SceType to C++ type name.
fn cpp_type(ty: &SceType) -> &'static str {
    match ty {
        SceType::Uint8 => "uint8_t",
        SceType::Uint16 => "uint16_t",
        SceType::Uint32 => "uint32_t",
        SceType::Uint64 => "uint64_t",
        SceType::Int8 => "int8_t",
        SceType::Int16 => "int16_t",
        SceType::Int32 => "int32_t",
        SceType::Int64 => "int64_t",
        SceType::Float32 => "float",
        SceType::Float64 => "double",
        SceType::Bool => "bool",
        SceType::String => "std::string",
        SceType::Bytes => "std::vector<uint8_t>",
    }
}

/// C++ parameter type (const ref for large types).
fn cpp_param_type(ty: &SceType) -> String {
    match ty {
        SceType::String | SceType::Bytes => format!("const {}&", cpp_type(ty)),
        _ => cpp_type(ty).to_string(),
    }
}

/// Map SceType to Kotlin type name (SCE_FORGE.md Section 3.3).
fn kotlin_type(ty: &SceType) -> &'static str {
    match ty {
        SceType::Uint8 => "UByte",
        SceType::Uint16 => "UShort",
        SceType::Uint32 => "UInt",
        SceType::Uint64 => "ULong",
        SceType::Int8 => "Byte",
        SceType::Int16 => "Short",
        SceType::Int32 => "Int",
        SceType::Int64 => "Long",
        SceType::Float32 => "Float",
        SceType::Float64 => "Double",
        SceType::Bool => "Boolean",
        SceType::String => "String",
        SceType::Bytes => "ByteArray",
    }
}

/// Kotlin conversion method for unsigned types in arithmetic/comparison context.
/// Returns None for types that support arithmetic operators natively.
fn kotlin_unsigned_conversion(ty: &SceType) -> Option<&'static str> {
    match ty {
        SceType::Uint8 | SceType::Uint16 => Some("toInt"),
        SceType::Uint32 | SceType::Uint64 => Some("toLong"),
        _ => None,
    }
}

/// Compute Kotlin expression conversions for a transform function.
/// Returns (variable_conversions, result_suffix).
///
/// Instead of shadowing parameters, the expression is modified directly:
/// - Integer inputs feeding float outputs → `.toDouble()` / `.toFloat()`
/// - Unsigned inputs feeding integer outputs → `.toInt()` / `.toLong()` (for bitwise/arithmetic)
/// - Result suffix converts back to unsigned when output is unsigned
fn kotlin_transform_conversions(
    inputs: &[ForgeField],
    output_type: &SceType,
) -> (Vec<(String, String)>, &'static str) {
    let mut conversions = Vec::new();

    let output_is_float = matches!(output_type, SceType::Float32 | SceType::Float64);
    let output_is_unsigned = matches!(
        output_type,
        SceType::Uint8 | SceType::Uint16 | SceType::Uint32 | SceType::Uint64
    );

    for inp in inputs {
        let is_unsigned = matches!(
            inp.sce_type,
            SceType::Uint8 | SceType::Uint16 | SceType::Uint32 | SceType::Uint64
        );
        let is_integer = is_unsigned
            || matches!(
                inp.sce_type,
                SceType::Int8 | SceType::Int16 | SceType::Int32 | SceType::Int64
            );

        if output_is_float && is_integer {
            let conv = if *output_type == SceType::Float32 {
                "toFloat"
            } else {
                "toDouble"
            };
            conversions.push((inp.id.clone(), conv.to_string()));
        } else if is_unsigned {
            let conv = match inp.sce_type {
                SceType::Uint8 | SceType::Uint16 => "toInt",
                _ => "toLong",
            };
            conversions.push((inp.id.clone(), conv.to_string()));
        }
    }

    // Suffix converts arithmetic result back to unsigned output type
    let has_integer_input = inputs.iter().any(|inp| {
        matches!(
            inp.sce_type,
            SceType::Uint8
                | SceType::Uint16
                | SceType::Uint32
                | SceType::Uint64
                | SceType::Int8
                | SceType::Int16
                | SceType::Int32
                | SceType::Int64
        )
    });

    let suffix = if output_is_unsigned && has_integer_input {
        match output_type {
            SceType::Uint8 => ".toUByte()",
            SceType::Uint16 => ".toUShort()",
            SceType::Uint32 => ".toUInt()",
            SceType::Uint64 => ".toULong()",
            _ => "",
        }
    } else {
        ""
    };

    (conversions, suffix)
}

/// Wrap variable references in a Kotlin expression with type conversion calls.
/// Uses word-boundary matching to avoid partial replacements.
fn kotlin_wrap_expr(expr: &str, conversions: &[(String, String)]) -> String {
    let mut result = expr.to_string();
    for (name, conv) in conversions {
        let re = regex::Regex::new(&format!(r"\b{}\b", regex::escape(name))).unwrap();
        let replacement = format!("{name}.{conv}()");
        result = re.replace_all(&result, replacement.as_str()).to_string();
    }
    result
}

/// Check if a condition expression needs unsigned-to-signed conversion.
/// Only needed when integer literals appear (e.g., `rpm > 8000`).
/// Variable-to-variable comparisons on unsigned types work natively in Kotlin.
fn kotlin_condition_needs_conversion(expr: &str) -> bool {
    let stripped = super::expr::strip_string_literals(expr);
    // Remove float literals first (e.g., 0.1, 95.0)
    static RE_FLOAT: LazyLock<regex::Regex> =
        LazyLock::new(|| regex::Regex::new(r"\d+\.\d+").unwrap());
    let no_floats = RE_FLOAT.replace_all(&stripped, " ");
    // Check for remaining integer/hex literals
    static RE_INT: LazyLock<regex::Regex> =
        LazyLock::new(|| regex::Regex::new(r"\b\d+\b|0x[0-9a-fA-F]+\b").unwrap());
    RE_INT.is_match(&no_floats)
}

/// Replace camelCase variable references with snake_case in expressions.
/// Sorts by name length descending to prevent partial replacements.
fn rust_rename_vars(expr: &str, renames: &[(String, String)]) -> String {
    let mut sorted: Vec<_> = renames.to_vec();
    sorted.sort_by(|a, b| b.0.len().cmp(&a.0.len()));

    let mut result = expr.to_string();
    for (from, to) in &sorted {
        if from == to {
            continue;
        }
        let re = regex::Regex::new(&format!(r"\b{}\b", regex::escape(from))).unwrap();
        result = re.replace_all(&result, to.as_str()).to_string();
    }
    result
}

/// Map SceType to Rust type name (SCE_FORGE.md Section 3.3).
fn rust_type(ty: &SceType) -> &'static str {
    match ty {
        SceType::Uint8 => "u8",
        SceType::Uint16 => "u16",
        SceType::Uint32 => "u32",
        SceType::Uint64 => "u64",
        SceType::Int8 => "i8",
        SceType::Int16 => "i16",
        SceType::Int32 => "i32",
        SceType::Int64 => "i64",
        SceType::Float32 => "f32",
        SceType::Float64 => "f64",
        SceType::Bool => "bool",
        SceType::String => "String",
        SceType::Bytes => "Vec<u8>",
    }
}

/// Rust parameter type (borrow for heap-allocated types).
fn rust_param_type(ty: &SceType) -> String {
    match ty {
        SceType::String => "&str".to_string(),
        SceType::Bytes => "&[u8]".to_string(),
        _ => rust_type(ty).to_string(),
    }
}

// ── Public API ─────────────────────────────────────────────────

/// Generate code from a ForgeDocument for C++ using Jinja2 templates.
pub fn generate_cpp(doc: &ForgeDocument, template_dir: &Path) -> Result<GeneratedOutput, String> {
    let forge_dir = template_dir.join("forge/cpp");
    let mut env = generator::new_env();
    generator::load_templates(&mut env, &forge_dir)?;

    let code = match doc {
        ForgeDocument::Transform(m) => render_transform_cpp(&env, m)?,
        ForgeDocument::Lookup(m) => render_lookup_cpp(&env, m)?,
        ForgeDocument::Condition(m) => render_condition_cpp(&env, m)?,
        ForgeDocument::Codec(m) => render_codec_cpp(&env, m)?,
        ForgeDocument::Validator(m) => render_validator_cpp(&env, m)?,
    };

    let filename = format!("{}.h", filters::to_snake_case(doc.name().to_string()));
    Ok(GeneratedOutput {
        files: vec![(filename, code)],
    })
}

// ── Transform rendering ────────────────────────────────────────

fn render_transform_cpp(
    env: &minijinja::Environment,
    m: &TransformModel,
) -> Result<String, String> {
    let ns = filters::to_pascal_case(m.name.clone());
    let guard = format!("SCE_FORGE_{}_H", to_upper_snake(&m.name));

    let functions: Vec<serde_json::Value> = m
        .outputs
        .iter()
        .map(|out| {
            let expr_cpp = expr::transpile(out.expr.as_deref().unwrap_or("0"), ExprTarget::Cpp)?;
            let params = m
                .inputs
                .iter()
                .map(|inp| format!("{} {}", cpp_param_type(&inp.sce_type), inp.id))
                .collect::<Vec<_>>()
                .join(", ");
            Ok(serde_json::json!({
                "ret_type": cpp_type(&out.sce_type),
                "name": format!("compute{}", filters::to_pascal_case(out.id.clone())),
                "params": params,
                "expr": expr_cpp,
            }))
        })
        .collect::<Result<_, String>>()?;

    let tmpl = env
        .get_template("transform.h.jinja2")
        .map_err(|e| format!("Template load error: {e}"))?;

    let ctx = minijinja::context! {
        guard => guard,
        namespace => ns,
        functions => minijinja::Value::from_serialize(&functions),
    };

    tmpl.render(ctx).map_err(generator::render_error)
}

// ── Lookup rendering ───────────────────────────────────────────

fn render_lookup_cpp(
    env: &minijinja::Environment,
    m: &LookupModel,
) -> Result<String, String> {
    let ns = filters::to_pascal_case(m.name.clone());
    let guard = format!("SCE_FORGE_{}_H", to_upper_snake(&m.name));
    let enum_name = filters::to_pascal_case(m.output.id.clone());
    let func_name = format!("lookup{}", filters::to_pascal_case(m.output.id.clone()));

    let entries_by_value: Vec<serde_json::Value> = m
        .entries_by_value()
        .into_iter()
        .map(|(value, keys)| {
            serde_json::json!({
                "value": value,
                "keys": keys,
            })
        })
        .collect();

    let tmpl = env
        .get_template("lookup.h.jinja2")
        .map_err(|e| format!("Template load error: {e}"))?;

    let ctx = minijinja::context! {
        guard => guard,
        namespace => ns,
        enum_name => enum_name,
        func_name => func_name,
        input_type => cpp_param_type(&m.input.sce_type),
        input_id => &m.input.id,
        unique_values => minijinja::Value::from_serialize(&m.unique_values()),
        entries_by_value => minijinja::Value::from_serialize(&entries_by_value),
        default_value => &m.default_value,
    };

    tmpl.render(ctx).map_err(generator::render_error)
}

// ── Condition rendering ────────────────────────────────────────

fn render_condition_cpp(
    env: &minijinja::Environment,
    m: &ConditionModel,
) -> Result<String, String> {
    let ns = filters::to_pascal_case(m.name.clone());
    let guard = format!("SCE_FORGE_{}_H", to_upper_snake(&m.name));
    let func_name = filters::to_camel_case(m.name.clone());

    let params = m
        .inputs
        .iter()
        .map(|inp| format!("{} {}", cpp_param_type(&inp.sce_type), inp.id))
        .collect::<Vec<_>>()
        .join(", ");

    let expr_cpp = expr::transpile(&m.expr, ExprTarget::Cpp)?;

    let tmpl = env
        .get_template("condition.h.jinja2")
        .map_err(|e| format!("Template load error: {e}"))?;

    let ctx = minijinja::context! {
        guard => guard,
        namespace => ns,
        func_name => func_name,
        params => params,
        expr => expr_cpp,
    };

    tmpl.render(ctx).map_err(generator::render_error)
}

// ── Codec rendering ────────────────────────────────────────────

fn render_codec_cpp(
    env: &minijinja::Environment,
    m: &CodecModel,
) -> Result<String, String> {
    let ns = filters::to_pascal_case(m.name.clone());
    let guard = format!("SCE_FORGE_{}_H", to_upper_snake(&m.name));
    let struct_name = filters::to_pascal_case(m.name.clone());

    // Pre-compute field info for template
    let fields: Vec<serde_json::Value> = m
        .fields
        .iter()
        .map(|f| {
            serde_json::json!({
                "id": f.id,
                "cpp_type": cpp_type(&f.sce_type),
                "decode_expr": generate_decode_expr(f, m.default_endian),
            })
        })
        .collect();

    let encode_exprs = generate_encode_exprs(&m.fields, m.default_endian);

    let tmpl = env
        .get_template("codec.h.jinja2")
        .map_err(|e| format!("Template load error: {e}"))?;

    let ctx = minijinja::context! {
        guard => guard,
        namespace => ns,
        struct_name => struct_name,
        fields => minijinja::Value::from_serialize(&fields),
        min_bytes => m.min_frame_bytes(),
        encode_exprs => minijinja::Value::from_serialize(&encode_exprs),
    };

    tmpl.render(ctx).map_err(generator::render_error)
}

// ── Codec expression generation ────────────────────────────────

/// Generate C++ decode expression for a single codec field.
fn generate_decode_expr(field: &CodecField, default_endian: Endian) -> String {
    let byte_off = field.byte_offset;
    let bit_off = field.bit_offset.unwrap_or(0);
    let endian = field.effective_endian(default_endian);

    match &field.bit_size {
        BitSize::Fixed { bits } => {
            if bit_off > 0 || *bits < 8 {
                let mask = (1u64 << bits) - 1;
                format!("static_cast<uint8_t>((raw[{byte_off}] >> {bit_off}) & 0x{mask:02X})")
            } else {
                match bits {
                    8 => format!("raw[{byte_off}]"),
                    16 => decode_multibyte(byte_off, 2, endian),
                    24 => decode_multibyte(byte_off, 3, endian),
                    32 => decode_multibyte(byte_off, 4, endian),
                    _ => format!("/* unsupported {bits}-bit decode */"),
                }
            }
        }
        BitSize::Tail => {
            format!("std::vector<uint8_t>(raw + {byte_off}, raw + len)")
        }
        BitSize::LengthRef => {
            let len_field = field.length_field.as_deref().unwrap_or("0");
            format!("std::vector<uint8_t>(raw + {byte_off}, raw + {byte_off} + {len_field})")
        }
    }
}

/// Generate multi-byte decode expression with endianness handling.
fn decode_multibyte(byte_off: u32, byte_count: u32, endian: Endian) -> String {
    let target_type = match byte_count {
        2 => "uint16_t",
        3 | 4 => "uint32_t",
        _ => "uint64_t",
    };

    let big_endian_shifts: Vec<String> = (0..byte_count)
        .map(|i| {
            let shift = (byte_count - 1 - i) * 8;
            let off = byte_off + i;
            if shift == 0 {
                format!("raw[{off}]")
            } else {
                format!("(static_cast<{target_type}>(raw[{off}]) << {shift})")
            }
        })
        .collect();

    let little_endian_shifts: Vec<String> = (0..byte_count)
        .map(|i| {
            let shift = i * 8;
            let off = byte_off + i;
            if shift == 0 {
                format!("raw[{off}]")
            } else {
                format!("(static_cast<{target_type}>(raw[{off}]) << {shift})")
            }
        })
        .collect();

    match endian {
        Endian::Big | Endian::Native => big_endian_shifts.join(" | "),
        Endian::Little => little_endian_shifts.join(" | "),
    }
}

/// Generate C++ encode byte expressions for all codec fields.
fn generate_encode_exprs(fields: &[CodecField], default_endian: Endian) -> Vec<String> {
    let mut exprs = Vec::new();

    // Group fields by byte position for sub-byte field merging
    let mut byte_groups: std::collections::BTreeMap<u32, Vec<&CodecField>> =
        std::collections::BTreeMap::new();

    for field in fields {
        if field.is_variable_length() {
            // Variable-length fields appended via insert for encode
            exprs.push(format!(
                "/* variable-length field '{}' requires manual encode */",
                field.id
            ));
        } else {
            byte_groups
                .entry(field.byte_offset)
                .or_default()
                .push(field);
        }
    }

    for (_, group) in &byte_groups {
        if group.len() == 1 {
            let field = group[0];
            encode_single_field(field, default_endian, &mut exprs);
        } else {
            // Multiple sub-byte fields at same byte offset — merge with bitwise OR
            let mut parts = Vec::new();
            for field in group {
                let bit_off = field.bit_offset.unwrap_or(0);
                let bits = field.fixed_bits().unwrap_or(8);
                let mask = (1u64 << bits) - 1;
                parts.push(format!("(({} & 0x{mask:02X}) << {bit_off})", field.id));
            }
            exprs.push(format!("static_cast<uint8_t>({})", parts.join(" | ")));
        }
    }

    exprs
}

/// Generate encode expressions for a single non-sub-byte field.
fn encode_single_field(field: &CodecField, default_endian: Endian, exprs: &mut Vec<String>) {
    let bit_off = field.bit_offset.unwrap_or(0);
    let endian = field.effective_endian(default_endian);

    match field.fixed_bits() {
        Some(8) if bit_off == 0 => {
            exprs.push(field.id.clone());
        }
        Some(bits) if bits < 8 || bit_off > 0 => {
            let mask = (1u64 << bits) - 1;
            exprs.push(format!(
                "static_cast<uint8_t>(({} & 0x{mask:02X}) << {bit_off})",
                field.id
            ));
        }
        Some(byte_count @ (16 | 24 | 32)) => {
            let n_bytes = byte_count / 8;
            let shifts: Vec<u32> = match endian {
                Endian::Big | Endian::Native => (0..n_bytes).rev().collect(),
                Endian::Little => (0..n_bytes).collect(),
            };
            for shift_byte in shifts {
                let shift = shift_byte * 8;
                if shift == 0 {
                    exprs.push(format!("static_cast<uint8_t>({} & 0xFF)", field.id));
                } else {
                    exprs.push(format!(
                        "static_cast<uint8_t>(({} >> {shift}) & 0xFF)",
                        field.id
                    ));
                }
            }
        }
        _ => exprs.push(format!("/* encode {} */", field.id)),
    }
}

// ── Validator: resolved model (rule-field association, computed once) ──

/// Range rule with its associated input field resolved.
struct ResolvedRange {
    id: String,
    sce_type: SceType,
    min: Option<String>,
    max: Option<String>,
}

/// Rate-of-change rule with its associated input field resolved.
struct ResolvedRoc {
    id: String,
    sce_type: SceType,
    max_delta: String,
}

/// Validator model with rule-to-field associations pre-resolved.
/// Eliminates repeated `inputs.iter().find()` across 5 language renderers.
struct ResolvedValidator {
    inputs: Vec<ForgeField>,
    ranges: Vec<ResolvedRange>,
    rocs: Vec<ResolvedRoc>,
    plausibility: Option<String>,
}

fn resolve_validator(m: &ValidatorModel) -> ResolvedValidator {
    let ranges = m
        .rules
        .ranges
        .iter()
        .map(|r| {
            let field = m.inputs.iter().find(|f| f.id == r.id).unwrap();
            ResolvedRange {
                id: r.id.clone(),
                sce_type: field.sce_type.clone(),
                min: r.min.clone(),
                max: r.max.clone(),
            }
        })
        .collect();

    let rocs = m
        .rules
        .rate_of_changes
        .iter()
        .map(|roc| {
            let field = m.inputs.iter().find(|f| f.id == roc.id).unwrap();
            ResolvedRoc {
                id: roc.id.clone(),
                sce_type: field.sce_type.clone(),
                max_delta: roc.max_delta.clone(),
            }
        })
        .collect();

    ResolvedValidator {
        inputs: m.inputs.clone(),
        ranges,
        rocs,
        plausibility: m.rules.plausibility.clone(),
    }
}

// ── Validator rendering ───────────────────────────────────────

fn render_validator_cpp(
    env: &minijinja::Environment,
    m: &ValidatorModel,
) -> Result<String, String> {
    let rv = resolve_validator(m);
    let ns = filters::to_pascal_case(m.name.clone());
    let guard = format!("SCE_FORGE_{}_H", to_upper_snake(&m.name));
    let struct_name = filters::to_pascal_case(m.name.clone());

    let params = rv.inputs.iter()
        .map(|inp| format!("{} {}", cpp_param_type(&inp.sce_type), inp.id))
        .collect::<Vec<_>>()
        .join(", ");

    let prev_vars: Vec<serde_json::Value> = rv.rocs.iter()
        .map(|roc| serde_json::json!({
            "type": cpp_type(&roc.sce_type),
            "name": format!("prev{}", filters::to_pascal_case(roc.id.clone())),
            "id": roc.id,
        }))
        .collect();

    let range_rules: Vec<serde_json::Value> = rv.ranges.iter()
        .map(|r| serde_json::json!({
            "id": r.id,
            "min": r.min, "max": r.max,
            "has_min": r.min.is_some(), "has_max": r.max.is_some(),
        }))
        .collect();

    let roc_rules: Vec<serde_json::Value> = rv.rocs.iter()
        .map(|roc| serde_json::json!({
            "id": roc.id,
            "max_delta": roc.max_delta,
            "prev_name": format!("prev{}", filters::to_pascal_case(roc.id.clone())),
            "type": cpp_type(&roc.sce_type),
            "is_float": roc.sce_type.is_float(),
            "is_unsigned": roc.sce_type.is_unsigned(),
        }))
        .collect();

    let plausibility_expr = match &rv.plausibility {
        Some(e) => Some(expr::transpile(e, ExprTarget::Cpp)?),
        None => None,
    };

    let tmpl = env
        .get_template("validator.h.jinja2")
        .map_err(|e| format!("Template load error: {e}"))?;

    let ctx = minijinja::context! {
        guard => guard, namespace => ns, struct_name => struct_name,
        params => params,
        prev_vars => minijinja::Value::from_serialize(&prev_vars),
        range_rules => minijinja::Value::from_serialize(&range_rules),
        roc_rules => minijinja::Value::from_serialize(&roc_rules),
        plausibility_expr => plausibility_expr,
    };

    tmpl.render(ctx).map_err(generator::render_error)
}

// ══════════════════════════════════════════════════════════════
// ── Kotlin code generation ────────────────────────────────────
// ══════════════════════════════════════════════════════════════

/// Generate code from a ForgeDocument for Kotlin using Jinja2 templates.
pub fn generate_kotlin(doc: &ForgeDocument, template_dir: &Path) -> Result<GeneratedOutput, String> {
    let forge_dir = template_dir.join("forge/kotlin");
    let mut env = generator::new_env();
    generator::load_templates(&mut env, &forge_dir)?;

    let code = match doc {
        ForgeDocument::Transform(m) => render_transform_kotlin(&env, m)?,
        ForgeDocument::Lookup(m) => render_lookup_kotlin(&env, m)?,
        ForgeDocument::Condition(m) => render_condition_kotlin(&env, m)?,
        ForgeDocument::Codec(m) => render_codec_kotlin(&env, m)?,
        ForgeDocument::Validator(m) => render_validator_kotlin(&env, m)?,
    };

    let filename = format!("{}.kt", filters::to_pascal_case(doc.name().to_string()));
    Ok(GeneratedOutput {
        files: vec![(filename, code)],
    })
}

// ── Kotlin: Transform ─────────────────────────────────────────

fn render_transform_kotlin(
    env: &minijinja::Environment,
    m: &TransformModel,
) -> Result<String, String> {
    let package = filters::to_snake_case(m.name.clone());

    let functions: Vec<serde_json::Value> = m
        .outputs
        .iter()
        .map(|out| {
            let expr_kt = expr::transpile(out.expr.as_deref().unwrap_or("0"), ExprTarget::Kotlin)?;
            let params = m
                .inputs
                .iter()
                .map(|inp| format!("{}: {}", inp.id, kotlin_type(&inp.sce_type)))
                .collect::<Vec<_>>()
                .join(", ");

            // Type-aware expression wrapping (no parameter shadowing)
            let (conversions, suffix) =
                kotlin_transform_conversions(&m.inputs, &out.sce_type);
            let mut final_expr = kotlin_wrap_expr(&expr_kt, &conversions);
            if !suffix.is_empty() {
                final_expr = format!("({final_expr}){suffix}");
            }

            Ok(serde_json::json!({
                "ret_type": kotlin_type(&out.sce_type),
                "name": format!("compute{}", filters::to_pascal_case(out.id.clone())),
                "params": params,
                "expr": final_expr,
            }))
        })
        .collect::<Result<_, String>>()?;

    let tmpl = env
        .get_template("transform.kt.jinja2")
        .map_err(|e| format!("Template load error: {e}"))?;

    let ctx = minijinja::context! {
        package => package,
        functions => minijinja::Value::from_serialize(&functions),
    };

    tmpl.render(ctx).map_err(generator::render_error)
}

// ── Kotlin: Lookup ────────────────────────────────────────────

fn render_lookup_kotlin(
    env: &minijinja::Environment,
    m: &LookupModel,
) -> Result<String, String> {
    let package = filters::to_snake_case(m.name.clone());
    let enum_name = filters::to_pascal_case(m.output.id.clone());
    let func_name = format!("lookup{}", filters::to_pascal_case(m.output.id.clone()));

    // Unsigned types need .toInt() for when-matching against Int literals
    let match_suffix = match kotlin_unsigned_conversion(&m.input.sce_type) {
        Some(conv) => format!(".{conv}()"),
        None => String::new(),
    };

    let entries_by_value: Vec<serde_json::Value> = m
        .entries_by_value()
        .into_iter()
        .map(|(value, keys)| {
            serde_json::json!({
                "value": value,
                "keys": keys,
            })
        })
        .collect();

    let tmpl = env
        .get_template("lookup.kt.jinja2")
        .map_err(|e| format!("Template load error: {e}"))?;

    let ctx = minijinja::context! {
        package => package,
        enum_name => enum_name,
        func_name => func_name,
        input_type => kotlin_type(&m.input.sce_type),
        input_id => &m.input.id,
        match_suffix => match_suffix,
        unique_values => minijinja::Value::from_serialize(&m.unique_values()),
        entries_by_value => minijinja::Value::from_serialize(&entries_by_value),
        default_value => &m.default_value,
    };

    tmpl.render(ctx).map_err(generator::render_error)
}

// ── Kotlin: Condition ─────────────────────────────────────────

fn render_condition_kotlin(
    env: &minijinja::Environment,
    m: &ConditionModel,
) -> Result<String, String> {
    let package = filters::to_snake_case(m.name.clone());
    let func_name = filters::to_camel_case(m.name.clone());

    let params = m
        .inputs
        .iter()
        .map(|inp| format!("{}: {}", inp.id, kotlin_type(&inp.sce_type)))
        .collect::<Vec<_>>()
        .join(", ");

    let mut expr_kt = expr::transpile(&m.expr, ExprTarget::Kotlin)?;

    // Only wrap unsigned inputs when the expression contains integer literal comparisons.
    // Variable-to-variable comparisons (UInt >= UInt) work natively in Kotlin.
    let has_unsigned = m.inputs.iter().any(|inp| {
        matches!(
            inp.sce_type,
            SceType::Uint8 | SceType::Uint16 | SceType::Uint32 | SceType::Uint64
        )
    });
    if has_unsigned && kotlin_condition_needs_conversion(&expr_kt) {
        let conversions: Vec<(String, String)> = m
            .inputs
            .iter()
            .filter_map(|inp| {
                kotlin_unsigned_conversion(&inp.sce_type)
                    .map(|conv| (inp.id.clone(), conv.to_string()))
            })
            .collect();
        expr_kt = kotlin_wrap_expr(&expr_kt, &conversions);
    }

    let tmpl = env
        .get_template("condition.kt.jinja2")
        .map_err(|e| format!("Template load error: {e}"))?;

    let ctx = minijinja::context! {
        package => package,
        func_name => func_name,
        params => params,
        expr => expr_kt,
    };

    tmpl.render(ctx).map_err(generator::render_error)
}

// ── Kotlin: Codec ─────────────────────────────────────────────

fn render_codec_kotlin(
    env: &minijinja::Environment,
    m: &CodecModel,
) -> Result<String, String> {
    let package = filters::to_snake_case(m.name.clone());
    let struct_name = filters::to_pascal_case(m.name.clone());

    let fields: Vec<serde_json::Value> = m
        .fields
        .iter()
        .map(|f| {
            serde_json::json!({
                "id": f.id,
                "kt_type": kotlin_type(&f.sce_type),
                "decode_expr": generate_decode_expr_kotlin(f, m.default_endian),
            })
        })
        .collect();

    let encode_exprs = generate_encode_exprs_kotlin(&m.fields, m.default_endian);

    let tmpl = env
        .get_template("codec.kt.jinja2")
        .map_err(|e| format!("Template load error: {e}"))?;

    let ctx = minijinja::context! {
        package => package,
        struct_name => struct_name,
        fields => minijinja::Value::from_serialize(&fields),
        min_bytes => m.min_frame_bytes(),
        encode_exprs => minijinja::Value::from_serialize(&encode_exprs),
    };

    tmpl.render(ctx).map_err(generator::render_error)
}

// ── Kotlin codec expression generation ────────────────────────

fn generate_decode_expr_kotlin(field: &CodecField, default_endian: Endian) -> String {
    let byte_off = field.byte_offset;
    let bit_off = field.bit_offset.unwrap_or(0);
    let endian = field.effective_endian(default_endian);

    match &field.bit_size {
        BitSize::Fixed { bits } => {
            if bit_off > 0 || *bits < 8 {
                let mask = (1u64 << bits) - 1;
                format!(
                    "((raw[{byte_off}].toInt() ushr {bit_off}) and 0x{mask:02X}).toUByte()"
                )
            } else {
                match bits {
                    8 => format!("raw[{byte_off}].toUByte()"),
                    16 => decode_multibyte_kotlin(byte_off, 2, endian),
                    24 => decode_multibyte_kotlin(byte_off, 3, endian),
                    32 => decode_multibyte_kotlin(byte_off, 4, endian),
                    _ => format!("/* unsupported {bits}-bit decode */"),
                }
            }
        }
        BitSize::Tail => {
            format!("raw.copyOfRange({byte_off}, raw.size)")
        }
        BitSize::LengthRef => {
            let len_field = field.length_field.as_deref().unwrap_or("0");
            format!("raw.copyOfRange({byte_off}, {byte_off} + {len_field}.toInt())")
        }
    }
}

fn decode_multibyte_kotlin(byte_off: u32, byte_count: u32, endian: Endian) -> String {
    let to_type = match byte_count {
        2 => "toUShort",
        3 | 4 => "toUInt",
        _ => "toULong",
    };

    let shifts: Vec<String> = match endian {
        Endian::Big | Endian::Native => (0..byte_count)
            .map(|i| {
                let shift = (byte_count - 1 - i) * 8;
                let off = byte_off + i;
                if shift == 0 {
                    format!("(raw[{off}].toInt() and 0xFF)")
                } else {
                    format!("((raw[{off}].toInt() and 0xFF) shl {shift})")
                }
            })
            .collect(),
        Endian::Little => (0..byte_count)
            .map(|i| {
                let shift = i * 8;
                let off = byte_off + i;
                if shift == 0 {
                    format!("(raw[{off}].toInt() and 0xFF)")
                } else {
                    format!("((raw[{off}].toInt() and 0xFF) shl {shift})")
                }
            })
            .collect(),
    };

    format!("({}).{to_type}()", shifts.join(" or "))
}

fn generate_encode_exprs_kotlin(fields: &[CodecField], default_endian: Endian) -> Vec<String> {
    let mut exprs = Vec::new();

    let mut byte_groups: std::collections::BTreeMap<u32, Vec<&CodecField>> =
        std::collections::BTreeMap::new();

    for field in fields {
        if field.is_variable_length() {
            exprs.push(format!(
                "/* variable-length field '{}' requires manual encode */",
                field.id
            ));
        } else {
            byte_groups
                .entry(field.byte_offset)
                .or_default()
                .push(field);
        }
    }

    for (_, group) in &byte_groups {
        if group.len() == 1 {
            encode_single_field_kotlin(group[0], default_endian, &mut exprs);
        } else {
            let mut parts = Vec::new();
            for field in group {
                let bit_off = field.bit_offset.unwrap_or(0);
                let bits = field.fixed_bits().unwrap_or(8);
                let mask = (1u64 << bits) - 1;
                parts.push(format!(
                    "({}.toInt() and 0x{mask:02X} shl {bit_off})",
                    field.id
                ));
            }
            exprs.push(format!("({}).toByte()", parts.join(" or ")));
        }
    }

    exprs
}

fn encode_single_field_kotlin(
    field: &CodecField,
    default_endian: Endian,
    exprs: &mut Vec<String>,
) {
    let bit_off = field.bit_offset.unwrap_or(0);
    let endian = field.effective_endian(default_endian);

    match field.fixed_bits() {
        Some(8) if bit_off == 0 => {
            exprs.push(format!("{}.toByte()", field.id));
        }
        Some(bits) if bits < 8 || bit_off > 0 => {
            let mask = (1u64 << bits) - 1;
            exprs.push(format!(
                "({}.toInt() and 0x{mask:02X} shl {bit_off}).toByte()",
                field.id
            ));
        }
        Some(byte_count @ (16 | 24 | 32)) => {
            let n_bytes = byte_count / 8;
            let shifts: Vec<u32> = match endian {
                Endian::Big | Endian::Native => (0..n_bytes).rev().collect(),
                Endian::Little => (0..n_bytes).collect(),
            };
            for shift_byte in shifts {
                let shift = shift_byte * 8;
                if shift == 0 {
                    exprs.push(format!("({}.toInt() and 0xFF).toByte()", field.id));
                } else {
                    exprs.push(format!(
                        "({}.toInt() ushr {shift} and 0xFF).toByte()",
                        field.id
                    ));
                }
            }
        }
        _ => exprs.push(format!("/* encode {} */", field.id)),
    }
}

// ── Kotlin: Validator ────────────────────────────────────────

fn render_validator_kotlin(
    env: &minijinja::Environment,
    m: &ValidatorModel,
) -> Result<String, String> {
    let rv = resolve_validator(m);
    let package = filters::to_snake_case(m.name.clone());
    let struct_name = filters::to_pascal_case(m.name.clone());

    let params = rv.inputs.iter()
        .map(|inp| format!("{}: {}", inp.id, kotlin_type(&inp.sce_type)))
        .collect::<Vec<_>>()
        .join(", ");

    let prev_vars: Vec<serde_json::Value> = rv.rocs.iter()
        .map(|roc| {
            let kt_ty = kotlin_type(&roc.sce_type);
            serde_json::json!({
                "type": kt_ty,
                "name": format!("prev{}", filters::to_pascal_case(roc.id.clone())),
                "id": roc.id,
                "default": kotlin_default_value(kt_ty),
            })
        })
        .collect();

    let range_rules: Vec<serde_json::Value> = rv.ranges.iter()
        .map(|r| {
            let conv = kotlin_unsigned_conversion(&r.sce_type).unwrap_or("");
            serde_json::json!({
                "id": r.id,
                "min": r.min, "max": r.max,
                "has_min": r.min.is_some(), "has_max": r.max.is_some(),
                "conv": conv, "needs_conv": !conv.is_empty(),
            })
        })
        .collect();

    let roc_rules: Vec<serde_json::Value> = rv.rocs.iter()
        .map(|roc| {
            let conv = kotlin_unsigned_conversion(&roc.sce_type).unwrap_or("");
            serde_json::json!({
                "id": roc.id,
                "max_delta": roc.max_delta,
                "prev_name": format!("prev{}", filters::to_pascal_case(roc.id.clone())),
                "conv": conv, "needs_conv": !conv.is_empty(),
                "is_float": roc.sce_type.is_float(),
                "is_signed": roc.sce_type.is_signed(),
            })
        })
        .collect();

    let plausibility_expr = match &rv.plausibility {
        Some(e) => {
            let mut expr_kt = expr::transpile(e, ExprTarget::Kotlin)?;
            let has_unsigned = rv.inputs.iter().any(|f| f.sce_type.is_unsigned());
            if has_unsigned && kotlin_condition_needs_conversion(&expr_kt) {
                let conversions: Vec<(String, String)> = rv.inputs.iter()
                    .filter_map(|f| {
                        kotlin_unsigned_conversion(&f.sce_type)
                            .map(|c| (f.id.clone(), c.to_string()))
                    })
                    .collect();
                expr_kt = kotlin_wrap_expr(&expr_kt, &conversions);
            }
            Some(expr_kt)
        }
        None => None,
    };

    let tmpl = env
        .get_template("validator.kt.jinja2")
        .map_err(|e| format!("Template load error: {e}"))?;

    let ctx = minijinja::context! {
        package => package, struct_name => struct_name,
        params => params,
        prev_vars => minijinja::Value::from_serialize(&prev_vars),
        range_rules => minijinja::Value::from_serialize(&range_rules),
        roc_rules => minijinja::Value::from_serialize(&roc_rules),
        plausibility_expr => plausibility_expr,
    };

    tmpl.render(ctx).map_err(generator::render_error)
}

/// Default value for Kotlin types.
fn kotlin_default_value(kt_type: &str) -> &'static str {
    match kt_type {
        "UByte" => "0u.toUByte()",
        "UShort" => "0u.toUShort()",
        "UInt" => "0u",
        "ULong" => "0uL",
        "Byte" => "0",
        "Short" => "0",
        "Int" => "0",
        "Long" => "0L",
        "Float" => "0.0f",
        "Double" => "0.0",
        "Boolean" => "false",
        _ => "0",
    }
}

// ══════════════════════════════════════════════════════════════
// ── Rust code generation ──────────────────────────────────────
// ══════════════════════════════════════════════════════════════

/// Generate code from a ForgeDocument for Rust using Jinja2 templates.
pub fn generate_rust(doc: &ForgeDocument, template_dir: &Path) -> Result<GeneratedOutput, String> {
    let forge_dir = template_dir.join("forge/rust");
    let mut env = generator::new_env();
    generator::load_templates(&mut env, &forge_dir)?;

    let code = match doc {
        ForgeDocument::Transform(m) => render_transform_rust(&env, m)?,
        ForgeDocument::Lookup(m) => render_lookup_rust(&env, m)?,
        ForgeDocument::Condition(m) => render_condition_rust(&env, m)?,
        ForgeDocument::Codec(m) => render_codec_rust(&env, m)?,
        ForgeDocument::Validator(m) => render_validator_rust(&env, m)?,
    };

    let filename = format!("{}.rs", filters::to_snake_case(doc.name().to_string()));
    Ok(GeneratedOutput {
        files: vec![(filename, code)],
    })
}

// ── Rust: Transform ───────────────────────────────────────────

fn render_transform_rust(
    env: &minijinja::Environment,
    m: &TransformModel,
) -> Result<String, String> {
    // Build name mappings for camelCase → snake_case
    let renames: Vec<(String, String)> = m
        .inputs
        .iter()
        .map(|inp| (inp.id.clone(), filters::to_snake_case(inp.id.clone())))
        .collect();

    let functions: Vec<serde_json::Value> = m
        .outputs
        .iter()
        .map(|out| {
            let mut expr_rs =
                expr::transpile(out.expr.as_deref().unwrap_or("0"), ExprTarget::Rust)?;

            // Rename camelCase variables to snake_case in expression
            expr_rs = rust_rename_vars(&expr_rs, &renames);

            let params = m
                .inputs
                .iter()
                .map(|inp| {
                    format!(
                        "{}: {}",
                        filters::to_snake_case(inp.id.clone()),
                        rust_param_type(&inp.sce_type)
                    )
                })
                .collect::<Vec<_>>()
                .join(", ");

            // Integer inputs feeding float outputs need `as f64` cast in expression
            let needs_cast = rust_float_cast_needed(&m.inputs, &out.sce_type);
            if let Some(cast_type) = needs_cast {
                // Wrap each integer input reference with `as f64`
                for inp in &m.inputs {
                    if rust_float_cast(&inp.sce_type, &out.sce_type).is_some() {
                        let snake = filters::to_snake_case(inp.id.clone());
                        let re = regex::Regex::new(&format!(r"\b{}\b", regex::escape(&snake)))
                            .unwrap();
                        let cast_expr = format!("{snake} as {cast_type}");
                        expr_rs = re.replace_all(&expr_rs, cast_expr.as_str()).to_string();
                    }
                }
            }

            Ok(serde_json::json!({
                "ret_type": rust_type(&out.sce_type),
                "name": format!("compute_{}", filters::to_snake_case(out.id.clone())),
                "params": params,
                "expr": expr_rs,
            }))
        })
        .collect::<Result<_, String>>()?;

    let tmpl = env
        .get_template("transform.rs.jinja2")
        .map_err(|e| format!("Template load error: {e}"))?;

    let ctx = minijinja::context! {
        functions => minijinja::Value::from_serialize(&functions),
    };

    tmpl.render(ctx).map_err(generator::render_error)
}

// ── Rust: Lookup ──────────────────────────────────────────────

fn render_lookup_rust(
    env: &minijinja::Environment,
    m: &LookupModel,
) -> Result<String, String> {
    let enum_name = filters::to_pascal_case(m.output.id.clone());
    let func_name = format!("lookup_{}", filters::to_snake_case(m.output.id.clone()));
    let input_id_snake = filters::to_snake_case(m.input.id.clone());

    // Convert enum values to PascalCase for Rust convention
    let unique_values: Vec<String> = m
        .unique_values()
        .into_iter()
        .map(|v| to_rust_variant(&v))
        .collect();

    let entries_by_value: Vec<serde_json::Value> = m
        .entries_by_value()
        .into_iter()
        .map(|(value, keys)| {
            serde_json::json!({
                "value": to_rust_variant(&value),
                "keys": keys,
            })
        })
        .collect();

    let default_value = to_rust_variant(&m.default_value);

    let tmpl = env
        .get_template("lookup.rs.jinja2")
        .map_err(|e| format!("Template load error: {e}"))?;

    let ctx = minijinja::context! {
        enum_name => enum_name,
        func_name => func_name,
        input_type => rust_param_type(&m.input.sce_type),
        input_id => input_id_snake,
        unique_values => minijinja::Value::from_serialize(&unique_values),
        entries_by_value => minijinja::Value::from_serialize(&entries_by_value),
        default_value => default_value,
    };

    tmpl.render(ctx).map_err(generator::render_error)
}

// ── Rust: Condition ───────────────────────────────────────────

fn render_condition_rust(
    env: &minijinja::Environment,
    m: &ConditionModel,
) -> Result<String, String> {
    let func_name = filters::to_snake_case(m.name.clone());

    let renames: Vec<(String, String)> = m
        .inputs
        .iter()
        .map(|inp| (inp.id.clone(), filters::to_snake_case(inp.id.clone())))
        .collect();

    let params = m
        .inputs
        .iter()
        .map(|inp| {
            format!(
                "{}: {}",
                filters::to_snake_case(inp.id.clone()),
                rust_param_type(&inp.sce_type)
            )
        })
        .collect::<Vec<_>>()
        .join(", ");

    let mut expr_rs = expr::transpile(&m.expr, ExprTarget::Rust)?;
    expr_rs = rust_rename_vars(&expr_rs, &renames);

    let tmpl = env
        .get_template("condition.rs.jinja2")
        .map_err(|e| format!("Template load error: {e}"))?;

    let ctx = minijinja::context! {
        func_name => func_name,
        params => params,
        expr => expr_rs,
    };

    tmpl.render(ctx).map_err(generator::render_error)
}

// ── Rust: Codec ───────────────────────────────────────────────

fn render_codec_rust(
    env: &minijinja::Environment,
    m: &CodecModel,
) -> Result<String, String> {
    let struct_name = filters::to_pascal_case(m.name.clone());

    // Build rename mapping for camelCase field ids → snake_case
    let renames: Vec<(String, String)> = m
        .fields
        .iter()
        .map(|f| (f.id.clone(), filters::to_snake_case(f.id.clone())))
        .collect();

    let fields: Vec<serde_json::Value> = m
        .fields
        .iter()
        .map(|f| {
            serde_json::json!({
                "id": filters::to_snake_case(f.id.clone()),
                "rs_type": rust_type(&f.sce_type),
                "decode_expr": generate_decode_expr_rust(f, m.default_endian),
            })
        })
        .collect();

    // Generate encode expressions, then rename field references to snake_case
    let encode_exprs: Vec<String> = generate_encode_exprs_rust(&m.fields, m.default_endian)
        .into_iter()
        .map(|expr| rust_rename_vars(&expr, &renames))
        .collect();

    let tmpl = env
        .get_template("codec.rs.jinja2")
        .map_err(|e| format!("Template load error: {e}"))?;

    let ctx = minijinja::context! {
        struct_name => struct_name,
        fields => minijinja::Value::from_serialize(&fields),
        min_bytes => m.min_frame_bytes(),
        encode_exprs => minijinja::Value::from_serialize(&encode_exprs),
    };

    tmpl.render(ctx).map_err(generator::render_error)
}

// ── Rust codec expression generation ──────────────────────────

fn generate_decode_expr_rust(field: &CodecField, default_endian: Endian) -> String {
    let byte_off = field.byte_offset;
    let bit_off = field.bit_offset.unwrap_or(0);
    let endian = field.effective_endian(default_endian);

    match &field.bit_size {
        BitSize::Fixed { bits } => {
            if bit_off > 0 || *bits < 8 {
                let mask = (1u64 << bits) - 1;
                format!("(raw[{byte_off}] >> {bit_off}) & 0x{mask:02X}")
            } else {
                match bits {
                    8 => format!("raw[{byte_off}]"),
                    16 => decode_multibyte_rust(byte_off, 2, endian),
                    24 => decode_multibyte_rust(byte_off, 3, endian),
                    32 => decode_multibyte_rust(byte_off, 4, endian),
                    _ => format!("/* unsupported {bits}-bit decode */"),
                }
            }
        }
        BitSize::Tail => {
            format!("raw[{byte_off}..].to_vec()")
        }
        BitSize::LengthRef => {
            let len_field = field.length_field.as_deref().unwrap_or("0");
            format!("raw[{byte_off}..{byte_off} + {len_field} as usize].to_vec()")
        }
    }
}

fn decode_multibyte_rust(byte_off: u32, byte_count: u32, endian: Endian) -> String {
    let target_type = match byte_count {
        2 => "u16",
        3 | 4 => "u32",
        _ => "u64",
    };

    let shifts: Vec<String> = match endian {
        Endian::Big | Endian::Native => (0..byte_count)
            .map(|i| {
                let shift = (byte_count - 1 - i) * 8;
                let off = byte_off + i;
                if shift == 0 {
                    format!("raw[{off}] as {target_type}")
                } else {
                    format!("((raw[{off}] as {target_type}) << {shift})")
                }
            })
            .collect(),
        Endian::Little => (0..byte_count)
            .map(|i| {
                let shift = i * 8;
                let off = byte_off + i;
                if shift == 0 {
                    format!("raw[{off}] as {target_type}")
                } else {
                    format!("((raw[{off}] as {target_type}) << {shift})")
                }
            })
            .collect(),
    };

    shifts.join(" | ")
}

fn generate_encode_exprs_rust(fields: &[CodecField], default_endian: Endian) -> Vec<String> {
    let mut exprs = Vec::new();

    let mut byte_groups: std::collections::BTreeMap<u32, Vec<&CodecField>> =
        std::collections::BTreeMap::new();

    for field in fields {
        if field.is_variable_length() {
            exprs.push(format!(
                "/* variable-length field '{}' requires manual encode */",
                field.id
            ));
        } else {
            byte_groups
                .entry(field.byte_offset)
                .or_default()
                .push(field);
        }
    }

    for (_, group) in &byte_groups {
        if group.len() == 1 {
            encode_single_field_rust(group[0], default_endian, &mut exprs);
        } else {
            let mut parts = Vec::new();
            for field in group {
                let bit_off = field.bit_offset.unwrap_or(0);
                let bits = field.fixed_bits().unwrap_or(8);
                let mask = (1u64 << bits) - 1;
                parts.push(format!(
                    "((self.{} & 0x{mask:02X}) << {bit_off})",
                    field.id
                ));
            }
            exprs.push(format!("({}) as u8", parts.join(" | ")));
        }
    }

    exprs
}

fn encode_single_field_rust(
    field: &CodecField,
    default_endian: Endian,
    exprs: &mut Vec<String>,
) {
    let bit_off = field.bit_offset.unwrap_or(0);
    let endian = field.effective_endian(default_endian);

    match field.fixed_bits() {
        Some(8) if bit_off == 0 => {
            exprs.push(format!("self.{}", field.id));
        }
        Some(bits) if bits < 8 || bit_off > 0 => {
            let mask = (1u64 << bits) - 1;
            exprs.push(format!(
                "((self.{} & 0x{mask:02X}) << {bit_off}) as u8",
                field.id
            ));
        }
        Some(byte_count @ (16 | 24 | 32)) => {
            let n_bytes = byte_count / 8;
            let shifts: Vec<u32> = match endian {
                Endian::Big | Endian::Native => (0..n_bytes).rev().collect(),
                Endian::Little => (0..n_bytes).collect(),
            };
            for shift_byte in shifts {
                let shift = shift_byte * 8;
                if shift == 0 {
                    exprs.push(format!("(self.{} & 0xFF) as u8", field.id));
                } else {
                    exprs.push(format!(
                        "(self.{} >> {shift} & 0xFF) as u8",
                        field.id
                    ));
                }
            }
        }
        _ => exprs.push(format!("/* encode {} */", field.id)),
    }
}

// ── Rust: Validator ──────────────────────────────────────────

fn render_validator_rust(
    env: &minijinja::Environment,
    m: &ValidatorModel,
) -> Result<String, String> {
    let rv = resolve_validator(m);
    let struct_name = filters::to_pascal_case(m.name.clone());

    let renames: Vec<(String, String)> = rv.inputs.iter()
        .map(|f| (f.id.clone(), filters::to_snake_case(f.id.clone())))
        .collect();

    let params = rv.inputs.iter()
        .map(|f| format!("{}: {}", filters::to_snake_case(f.id.clone()), rust_param_type(&f.sce_type)))
        .collect::<Vec<_>>()
        .join(", ");

    let prev_vars: Vec<serde_json::Value> = rv.rocs.iter()
        .map(|roc| {
            let snake = filters::to_snake_case(roc.id.clone());
            serde_json::json!({
                "type": rust_type(&roc.sce_type),
                "name": format!("prev_{snake}"),
                "id": snake,
            })
        })
        .collect();

    let range_rules: Vec<serde_json::Value> = rv.ranges.iter()
        .map(|r| serde_json::json!({
            "id": filters::to_snake_case(r.id.clone()),
            "min": r.min, "max": r.max,
            "has_min": r.min.is_some(), "has_max": r.max.is_some(),
        }))
        .collect();

    let roc_rules: Vec<serde_json::Value> = rv.rocs.iter()
        .map(|roc| {
            let snake = filters::to_snake_case(roc.id.clone());
            serde_json::json!({
                "id": snake,
                "max_delta": roc.max_delta,
                "prev_name": format!("prev_{snake}"),
                "is_unsigned": roc.sce_type.is_unsigned(),
                "is_float": roc.sce_type.is_float(),
            })
        })
        .collect();

    let plausibility_expr = match &rv.plausibility {
        Some(e) => {
            let mut expr_rs = expr::transpile(e, ExprTarget::Rust)?;
            expr_rs = rust_rename_vars(&expr_rs, &renames);
            Some(expr_rs)
        }
        None => None,
    };

    let tmpl = env
        .get_template("validator.rs.jinja2")
        .map_err(|e| format!("Template load error: {e}"))?;

    let ctx = minijinja::context! {
        struct_name => struct_name,
        params => params,
        prev_vars => minijinja::Value::from_serialize(&prev_vars),
        range_rules => minijinja::Value::from_serialize(&range_rules),
        roc_rules => minijinja::Value::from_serialize(&roc_rules),
        plausibility_expr => plausibility_expr,
    };

    tmpl.render(ctx).map_err(generator::render_error)
}

// ══════════════════════════════════════════════════════════════
// ── Go code generation ───────────────────────────────────────
// ══════════════════════════════════════════════════════════════

/// Map SceType to Go type name (SCE_FORGE.md Section 3.3).
fn go_type(ty: &SceType) -> &'static str {
    match ty {
        SceType::Uint8 => "uint8",
        SceType::Uint16 => "uint16",
        SceType::Uint32 => "uint32",
        SceType::Uint64 => "uint64",
        SceType::Int8 => "int8",
        SceType::Int16 => "int16",
        SceType::Int32 => "int32",
        SceType::Int64 => "int64",
        SceType::Float32 => "float32",
        SceType::Float64 => "float64",
        SceType::Bool => "bool",
        SceType::String => "string",
        SceType::Bytes => "[]byte",
    }
}

/// Go builtin identifiers that should not be used as variable/parameter names.
/// Keywords (func, return, etc.) are already impossible as SCXML ids.
/// Builtins (byte, string, int, etc.) compile but shadow the built-in type.
fn go_escape_builtin(name: &str) -> String {
    match name {
        "byte" | "rune" | "error" | "string" | "bool" | "int" | "uint"
        | "int8" | "int16" | "int32" | "int64"
        | "uint8" | "uint16" | "uint32" | "uint64"
        | "float32" | "float64" | "complex64" | "complex128"
        | "uintptr" | "len" | "cap" | "make" | "new" | "append" | "copy"
        | "close" | "delete" | "panic" | "recover" | "print" | "println"
        | "true" | "false" | "nil" | "iota" => format!("{name}_"),
        _ => name.to_string(),
    }
}

/// Generate code from a ForgeDocument for Go using Jinja2 templates.
pub fn generate_go(doc: &ForgeDocument, template_dir: &Path) -> Result<GeneratedOutput, String> {
    let forge_dir = template_dir.join("forge/go");
    let mut env = generator::new_env();
    generator::load_templates(&mut env, &forge_dir)?;

    let code = match doc {
        ForgeDocument::Transform(m) => render_transform_go(&env, m)?,
        ForgeDocument::Lookup(m) => render_lookup_go(&env, m)?,
        ForgeDocument::Condition(m) => render_condition_go(&env, m)?,
        ForgeDocument::Codec(m) => render_codec_go(&env, m)?,
        ForgeDocument::Validator(m) => render_validator_go(&env, m)?,
    };

    let filename = format!("{}.go", filters::to_snake_case(doc.name().to_string()));
    Ok(GeneratedOutput {
        files: vec![(filename, code)],
    })
}

// ── Go: Transform ────────────────────────────────────────────

fn render_transform_go(
    env: &minijinja::Environment,
    m: &TransformModel,
) -> Result<String, String> {
    let package = filters::to_snake_case(m.name.clone());

    // Build rename map: SCXML id → Go-safe parameter name
    let go_renames: Vec<(String, String)> = m
        .inputs
        .iter()
        .map(|inp| (inp.id.clone(), go_escape_builtin(&inp.id)))
        .collect();

    let functions: Vec<serde_json::Value> = m
        .outputs
        .iter()
        .map(|out| {
            let mut expr_go = expr::transpile(out.expr.as_deref().unwrap_or("0"), ExprTarget::Go)?;

            // Rename builtin-shadowing identifiers in expression
            for (from, to) in &go_renames {
                if from != to {
                    let re = regex::Regex::new(&format!(r"\b{}\b", regex::escape(from))).unwrap();
                    expr_go = re.replace_all(&expr_go, to.as_str()).to_string();
                }
            }

            // Integer inputs feeding float outputs need float64() cast
            let needs_cast = go_float_cast_needed(&m.inputs, &out.sce_type);
            if let Some(cast_type) = needs_cast {
                for inp in &m.inputs {
                    if go_float_cast(&inp.sce_type, &out.sce_type).is_some() {
                        let safe_name = go_escape_builtin(&inp.id);
                        let re = regex::Regex::new(&format!(r"\b{}\b", regex::escape(&safe_name)))
                            .unwrap();
                        let cast_expr = format!("{cast_type}({safe_name})");
                        expr_go = re.replace_all(&expr_go, cast_expr.as_str()).to_string();
                    }
                }
            }

            let params = m
                .inputs
                .iter()
                .map(|inp| format!("{} {}", go_escape_builtin(&inp.id), go_type(&inp.sce_type)))
                .collect::<Vec<_>>()
                .join(", ");

            Ok(serde_json::json!({
                "ret_type": go_type(&out.sce_type),
                "name": format!("Compute{}", filters::to_pascal_case(out.id.clone())),
                "orig_name": out.id,
                "params": params,
                "expr": expr_go,
            }))
        })
        .collect::<Result<_, String>>()?;

    let tmpl = env
        .get_template("transform.go.jinja2")
        .map_err(|e| format!("Template load error: {e}"))?;

    let ctx = minijinja::context! {
        package => package,
        functions => minijinja::Value::from_serialize(&functions),
    };

    tmpl.render(ctx).map_err(generator::render_error)
}

// ── Go: Lookup ───────────────────────────────────────────────

fn render_lookup_go(
    env: &minijinja::Environment,
    m: &LookupModel,
) -> Result<String, String> {
    let package = filters::to_snake_case(m.name.clone());
    let enum_name = filters::to_pascal_case(m.output.id.clone());
    let func_name = format!("Lookup{}", filters::to_pascal_case(m.output.id.clone()));
    let input_id_safe = go_escape_builtin(&m.input.id);

    let entries_by_value: Vec<serde_json::Value> = m
        .entries_by_value()
        .into_iter()
        .map(|(value, keys)| {
            serde_json::json!({
                "value": value,
                "keys": keys,
            })
        })
        .collect();

    let tmpl = env
        .get_template("lookup.go.jinja2")
        .map_err(|e| format!("Template load error: {e}"))?;

    let ctx = minijinja::context! {
        package => package,
        enum_name => enum_name,
        func_name => func_name,
        input_type => go_type(&m.input.sce_type),
        input_id => input_id_safe,
        unique_values => minijinja::Value::from_serialize(&m.unique_values()),
        entries_by_value => minijinja::Value::from_serialize(&entries_by_value),
        default_value => &m.default_value,
    };

    tmpl.render(ctx).map_err(generator::render_error)
}

// ── Go: Condition ────────────────────────────────────────────

fn render_condition_go(
    env: &minijinja::Environment,
    m: &ConditionModel,
) -> Result<String, String> {
    let package = filters::to_snake_case(m.name.clone());
    let func_name = filters::to_pascal_case(m.name.clone());

    let go_renames: Vec<(String, String)> = m
        .inputs
        .iter()
        .map(|inp| (inp.id.clone(), go_escape_builtin(&inp.id)))
        .collect();

    let params = m
        .inputs
        .iter()
        .map(|inp| format!("{} {}", go_escape_builtin(&inp.id), go_type(&inp.sce_type)))
        .collect::<Vec<_>>()
        .join(", ");

    let mut expr_go = expr::transpile(&m.expr, ExprTarget::Go)?;

    // Rename builtin-shadowing identifiers in expression
    for (from, to) in &go_renames {
        if from != to {
            let re = regex::Regex::new(&format!(r"\b{}\b", regex::escape(from))).unwrap();
            expr_go = re.replace_all(&expr_go, to.as_str()).to_string();
        }
    }

    let tmpl = env
        .get_template("condition.go.jinja2")
        .map_err(|e| format!("Template load error: {e}"))?;

    let ctx = minijinja::context! {
        package => package,
        func_name => func_name,
        params => params,
        expr => expr_go,
    };

    tmpl.render(ctx).map_err(generator::render_error)
}

// ── Go: Codec ────────────────────────────────────────────────

fn render_codec_go(
    env: &minijinja::Environment,
    m: &CodecModel,
) -> Result<String, String> {
    let package = filters::to_snake_case(m.name.clone());
    let struct_name = filters::to_pascal_case(m.name.clone());

    let fields: Vec<serde_json::Value> = m
        .fields
        .iter()
        .map(|f| {
            serde_json::json!({
                "id": filters::to_pascal_case(f.id.clone()),
                "go_type": go_type(&f.sce_type),
                "decode_expr": generate_decode_expr_go(f, m.default_endian),
            })
        })
        .collect();

    let encode_exprs = generate_encode_exprs_go(&m.fields, m.default_endian);

    let tmpl = env
        .get_template("codec.go.jinja2")
        .map_err(|e| format!("Template load error: {e}"))?;

    let ctx = minijinja::context! {
        package => package,
        struct_name => struct_name,
        fields => minijinja::Value::from_serialize(&fields),
        min_bytes => m.min_frame_bytes(),
        encode_exprs => minijinja::Value::from_serialize(&encode_exprs),
    };

    tmpl.render(ctx).map_err(generator::render_error)
}

// ── Go codec expression generation ──────────────────────────

fn generate_decode_expr_go(field: &CodecField, default_endian: Endian) -> String {
    let byte_off = field.byte_offset;
    let bit_off = field.bit_offset.unwrap_or(0);
    let endian = field.effective_endian(default_endian);

    match &field.bit_size {
        BitSize::Fixed { bits } => {
            if bit_off > 0 || *bits < 8 {
                let mask = (1u64 << bits) - 1;
                format!("(raw[{byte_off}] >> {bit_off}) & 0x{mask:02X}")
            } else {
                match bits {
                    8 => format!("raw[{byte_off}]"),
                    16 => decode_multibyte_go(byte_off, 2, endian),
                    24 => decode_multibyte_go(byte_off, 3, endian),
                    32 => decode_multibyte_go(byte_off, 4, endian),
                    _ => format!("/* unsupported {bits}-bit decode */"),
                }
            }
        }
        BitSize::Tail => {
            format!("raw[{byte_off}:]")
        }
        BitSize::LengthRef => {
            let len_field = field.length_field.as_deref().unwrap_or("0");
            format!("raw[{byte_off}:{byte_off}+int({len_field})]")
        }
    }
}

fn decode_multibyte_go(byte_off: u32, byte_count: u32, endian: Endian) -> String {
    let target_type = match byte_count {
        2 => "uint16",
        3 | 4 => "uint32",
        _ => "uint64",
    };

    let shifts: Vec<String> = match endian {
        Endian::Big | Endian::Native => (0..byte_count)
            .map(|i| {
                let shift = (byte_count - 1 - i) * 8;
                let off = byte_off + i;
                if shift == 0 {
                    format!("{target_type}(raw[{off}])")
                } else {
                    format!("{target_type}(raw[{off}])<<{shift}")
                }
            })
            .collect(),
        Endian::Little => (0..byte_count)
            .map(|i| {
                let shift = i * 8;
                let off = byte_off + i;
                if shift == 0 {
                    format!("{target_type}(raw[{off}])")
                } else {
                    format!("{target_type}(raw[{off}])<<{shift}")
                }
            })
            .collect(),
    };

    shifts.join(" | ")
}

fn generate_encode_exprs_go(fields: &[CodecField], default_endian: Endian) -> Vec<String> {
    let mut exprs = Vec::new();

    let mut byte_groups: std::collections::BTreeMap<u32, Vec<&CodecField>> =
        std::collections::BTreeMap::new();

    for field in fields {
        if field.is_variable_length() {
            exprs.push(format!(
                "/* variable-length field '{}' requires manual encode */",
                field.id
            ));
        } else {
            byte_groups
                .entry(field.byte_offset)
                .or_default()
                .push(field);
        }
    }

    for (_, group) in &byte_groups {
        if group.len() == 1 {
            encode_single_field_go(group[0], default_endian, &mut exprs);
        } else {
            let mut parts = Vec::new();
            for field in group {
                let bit_off = field.bit_offset.unwrap_or(0);
                let bits = field.fixed_bits().unwrap_or(8);
                let mask = (1u64 << bits) - 1;
                let go_field = filters::to_pascal_case(field.id.clone());
                parts.push(format!(
                    "(s.{go_field} & 0x{mask:02X}) << {bit_off}"
                ));
            }
            exprs.push(format!("byte({})", parts.join(" | ")));
        }
    }

    exprs
}

fn encode_single_field_go(
    field: &CodecField,
    default_endian: Endian,
    exprs: &mut Vec<String>,
) {
    let bit_off = field.bit_offset.unwrap_or(0);
    let endian = field.effective_endian(default_endian);
    let go_field = filters::to_pascal_case(field.id.clone());

    match field.fixed_bits() {
        Some(8) if bit_off == 0 => {
            exprs.push(format!("byte(s.{go_field})"));
        }
        Some(bits) if bits < 8 || bit_off > 0 => {
            let mask = (1u64 << bits) - 1;
            exprs.push(format!(
                "byte((s.{go_field} & 0x{mask:02X}) << {bit_off})"
            ));
        }
        Some(byte_count @ (16 | 24 | 32)) => {
            let n_bytes = byte_count / 8;
            let shifts: Vec<u32> = match endian {
                Endian::Big | Endian::Native => (0..n_bytes).rev().collect(),
                Endian::Little => (0..n_bytes).collect(),
            };
            for shift_byte in shifts {
                let shift = shift_byte * 8;
                if shift == 0 {
                    exprs.push(format!("byte(s.{go_field} & 0xFF)"));
                } else {
                    exprs.push(format!(
                        "byte(s.{go_field} >> {shift} & 0xFF)"
                    ));
                }
            }
        }
        _ => exprs.push(format!("/* encode {} */", field.id)),
    }
}

/// Determine Go cast type for integer-to-float promotion in transform expressions.
fn go_float_cast(input_type: &SceType, output_type: &SceType) -> Option<&'static str> {
    let input_is_int = matches!(
        input_type,
        SceType::Uint8
            | SceType::Uint16
            | SceType::Uint32
            | SceType::Uint64
            | SceType::Int8
            | SceType::Int16
            | SceType::Int32
            | SceType::Int64
    );
    match (input_is_int, output_type) {
        (true, SceType::Float64) => Some("float64"),
        (true, SceType::Float32) => Some("float32"),
        _ => None,
    }
}

fn go_float_cast_needed(inputs: &[ForgeField], output_type: &SceType) -> Option<&'static str> {
    inputs
        .iter()
        .find_map(|inp| go_float_cast(&inp.sce_type, output_type))
}

// ── Go: Validator ────────────────────────────────────────────

fn render_validator_go(
    env: &minijinja::Environment,
    m: &ValidatorModel,
) -> Result<String, String> {
    let rv = resolve_validator(m);
    let package = filters::to_snake_case(m.name.clone());
    let struct_name = filters::to_pascal_case(m.name.clone());

    let go_renames: Vec<(String, String)> = rv.inputs.iter()
        .map(|f| (f.id.clone(), go_escape_builtin(&f.id)))
        .collect();

    let params = rv.inputs.iter()
        .map(|f| format!("{} {}", go_escape_builtin(&f.id), go_type(&f.sce_type)))
        .collect::<Vec<_>>()
        .join(", ");

    let prev_vars: Vec<serde_json::Value> = rv.rocs.iter()
        .map(|roc| {
            let safe = go_escape_builtin(&roc.id);
            serde_json::json!({
                "type": go_type(&roc.sce_type),
                "name": format!("prev{}", filters::to_pascal_case(safe.clone())),
                "id": safe,
            })
        })
        .collect();

    let range_rules: Vec<serde_json::Value> = rv.ranges.iter()
        .map(|r| serde_json::json!({
            "id": go_escape_builtin(&r.id),
            "min": r.min, "max": r.max,
            "has_min": r.min.is_some(), "has_max": r.max.is_some(),
        }))
        .collect();

    let roc_rules: Vec<serde_json::Value> = rv.rocs.iter()
        .map(|roc| {
            let safe = go_escape_builtin(&roc.id);
            serde_json::json!({
                "id": safe,
                "max_delta": roc.max_delta,
                "prev_name": format!("prev{}", filters::to_pascal_case(safe.clone())),
                "type": go_type(&roc.sce_type),
                "is_float": roc.sce_type.is_float(),
                "is_signed": roc.sce_type.is_signed(),
            })
        })
        .collect();

    let plausibility_expr = match &rv.plausibility {
        Some(e) => {
            let mut expr_go = expr::transpile(e, ExprTarget::Go)?;
            for (from, to) in &go_renames {
                if from != to {
                    let re =
                        regex::Regex::new(&format!(r"\b{}\b", regex::escape(from))).unwrap();
                    expr_go = re.replace_all(&expr_go, to.as_str()).to_string();
                }
            }
            Some(expr_go)
        }
        None => None,
    };

    let tmpl = env
        .get_template("validator.go.jinja2")
        .map_err(|e| format!("Template load error: {e}"))?;

    let ctx = minijinja::context! {
        package => package,
        struct_name => struct_name,
        params => params,
        prev_vars => minijinja::Value::from_serialize(&prev_vars),
        range_rules => minijinja::Value::from_serialize(&range_rules),
        roc_rules => minijinja::Value::from_serialize(&roc_rules),
        plausibility_expr => plausibility_expr,
    };

    tmpl.render(ctx).map_err(generator::render_error)
}

// ══════════════════════════════════════════════════════════════
// ── Python code generation ───────────────────────────────────
// ══════════════════════════════════════════════════════════════

/// Map SceType to Python type annotation (SCE_FORGE.md Section 3.3).
fn python_type(ty: &SceType) -> &'static str {
    match ty {
        SceType::Uint8
        | SceType::Uint16
        | SceType::Uint32
        | SceType::Uint64
        | SceType::Int8
        | SceType::Int16
        | SceType::Int32
        | SceType::Int64 => "int",
        SceType::Float32 | SceType::Float64 => "float",
        SceType::Bool => "bool",
        SceType::String => "str",
        SceType::Bytes => "bytes",
    }
}

/// Generate code from a ForgeDocument for Python using Jinja2 templates.
pub fn generate_python(doc: &ForgeDocument, template_dir: &Path) -> Result<GeneratedOutput, String> {
    let forge_dir = template_dir.join("forge/python");
    let mut env = generator::new_env();
    generator::load_templates(&mut env, &forge_dir)?;

    let code = match doc {
        ForgeDocument::Transform(m) => render_transform_python(&env, m)?,
        ForgeDocument::Lookup(m) => render_lookup_python(&env, m)?,
        ForgeDocument::Condition(m) => render_condition_python(&env, m)?,
        ForgeDocument::Codec(m) => render_codec_python(&env, m)?,
        ForgeDocument::Validator(m) => render_validator_python(&env, m)?,
    };

    let filename = format!("{}.py", filters::to_snake_case(doc.name().to_string()));
    Ok(GeneratedOutput {
        files: vec![(filename, code)],
    })
}

// ── Python: Transform ────────────────────────────────────────

fn render_transform_python(
    env: &minijinja::Environment,
    m: &TransformModel,
) -> Result<String, String> {
    // Build name mappings for camelCase → snake_case
    let renames: Vec<(String, String)> = m
        .inputs
        .iter()
        .map(|inp| (inp.id.clone(), filters::to_snake_case(inp.id.clone())))
        .collect();

    let functions: Vec<serde_json::Value> = m
        .outputs
        .iter()
        .map(|out| {
            let mut expr_py =
                expr::transpile(out.expr.as_deref().unwrap_or("0"), ExprTarget::Python)?;

            // Rename camelCase variables to snake_case in expression
            expr_py = rust_rename_vars(&expr_py, &renames);

            let params = m
                .inputs
                .iter()
                .map(|inp| {
                    format!(
                        "{}: {}",
                        filters::to_snake_case(inp.id.clone()),
                        python_type(&inp.sce_type)
                    )
                })
                .collect::<Vec<_>>()
                .join(", ");

            Ok(serde_json::json!({
                "ret_type": python_type(&out.sce_type),
                "name": format!("compute_{}", filters::to_snake_case(out.id.clone())),
                "params": params,
                "expr": expr_py,
            }))
        })
        .collect::<Result<_, String>>()?;

    let tmpl = env
        .get_template("transform.py.jinja2")
        .map_err(|e| format!("Template load error: {e}"))?;

    let ctx = minijinja::context! {
        functions => minijinja::Value::from_serialize(&functions),
    };

    tmpl.render(ctx).map_err(generator::render_error)
}

// ── Python: Lookup ───────────────────────────────────────────

fn render_lookup_python(
    env: &minijinja::Environment,
    m: &LookupModel,
) -> Result<String, String> {
    let enum_name = filters::to_pascal_case(m.output.id.clone());
    let func_name = format!("lookup_{}", filters::to_snake_case(m.output.id.clone()));
    let input_id_snake = filters::to_snake_case(m.input.id.clone());

    // Pre-compute condition expression per group: `==` for single key, `in (...)` for multiple.
    // Single-element tuples need a trailing comma in Python: `(0x07,)`.
    let entries_by_value: Vec<serde_json::Value> = m
        .entries_by_value()
        .into_iter()
        .map(|(value, keys)| {
            let condition = if keys.len() == 1 {
                format!("{} == {}", input_id_snake, keys[0])
            } else {
                format!("{} in ({})", input_id_snake, keys.join(", "))
            };
            serde_json::json!({
                "value": value,
                "condition": condition,
            })
        })
        .collect();

    let tmpl = env
        .get_template("lookup.py.jinja2")
        .map_err(|e| format!("Template load error: {e}"))?;

    let ctx = minijinja::context! {
        enum_name => enum_name,
        func_name => func_name,
        input_type => python_type(&m.input.sce_type),
        input_id => input_id_snake,
        unique_values => minijinja::Value::from_serialize(&m.unique_values()),
        entries_by_value => minijinja::Value::from_serialize(&entries_by_value),
        default_value => &m.default_value,
    };

    tmpl.render(ctx).map_err(generator::render_error)
}

// ── Python: Condition ────────────────────────────────────────

fn render_condition_python(
    env: &minijinja::Environment,
    m: &ConditionModel,
) -> Result<String, String> {
    let func_name = filters::to_snake_case(m.name.clone());

    let renames: Vec<(String, String)> = m
        .inputs
        .iter()
        .map(|inp| (inp.id.clone(), filters::to_snake_case(inp.id.clone())))
        .collect();

    let params = m
        .inputs
        .iter()
        .map(|inp| {
            format!(
                "{}: {}",
                filters::to_snake_case(inp.id.clone()),
                python_type(&inp.sce_type)
            )
        })
        .collect::<Vec<_>>()
        .join(", ");

    let mut expr_py = expr::transpile(&m.expr, ExprTarget::Python)?;
    expr_py = rust_rename_vars(&expr_py, &renames);

    let tmpl = env
        .get_template("condition.py.jinja2")
        .map_err(|e| format!("Template load error: {e}"))?;

    let ctx = minijinja::context! {
        func_name => func_name,
        params => params,
        expr => expr_py,
    };

    tmpl.render(ctx).map_err(generator::render_error)
}

// ── Python: Codec ────────────────────────────────────────────

fn render_codec_python(
    env: &minijinja::Environment,
    m: &CodecModel,
) -> Result<String, String> {
    let struct_name = filters::to_pascal_case(m.name.clone());

    let fields: Vec<serde_json::Value> = m
        .fields
        .iter()
        .map(|f| {
            serde_json::json!({
                "id": filters::to_snake_case(f.id.clone()),
                "py_type": python_type(&f.sce_type),
                "decode_expr": generate_decode_expr_python(f, m.default_endian),
            })
        })
        .collect();

    // Generate encode expressions, then rename field references to snake_case
    let renames: Vec<(String, String)> = m
        .fields
        .iter()
        .map(|f| (f.id.clone(), filters::to_snake_case(f.id.clone())))
        .collect();

    let encode_exprs: Vec<String> = generate_encode_exprs_python(&m.fields, m.default_endian)
        .into_iter()
        .map(|expr| rust_rename_vars(&expr, &renames))
        .collect();

    let tmpl = env
        .get_template("codec.py.jinja2")
        .map_err(|e| format!("Template load error: {e}"))?;

    let ctx = minijinja::context! {
        struct_name => struct_name,
        fields => minijinja::Value::from_serialize(&fields),
        min_bytes => m.min_frame_bytes(),
        encode_exprs => minijinja::Value::from_serialize(&encode_exprs),
    };

    tmpl.render(ctx).map_err(generator::render_error)
}

// ── Python codec expression generation ──────────────────────

fn generate_decode_expr_python(field: &CodecField, default_endian: Endian) -> String {
    let byte_off = field.byte_offset;
    let bit_off = field.bit_offset.unwrap_or(0);
    let endian = field.effective_endian(default_endian);

    match &field.bit_size {
        BitSize::Fixed { bits } => {
            if bit_off > 0 || *bits < 8 {
                let mask = (1u64 << bits) - 1;
                format!("(raw[{byte_off}] >> {bit_off}) & 0x{mask:02X}")
            } else {
                match bits {
                    8 => format!("raw[{byte_off}]"),
                    16 => decode_multibyte_python(byte_off, 2, endian),
                    24 => decode_multibyte_python(byte_off, 3, endian),
                    32 => decode_multibyte_python(byte_off, 4, endian),
                    _ => format!("# unsupported {bits}-bit decode"),
                }
            }
        }
        BitSize::Tail => {
            format!("raw[{byte_off}:]")
        }
        BitSize::LengthRef => {
            let len_field = field.length_field.as_deref().unwrap_or("0");
            format!("raw[{byte_off}:{byte_off} + {len_field}]")
        }
    }
}

fn decode_multibyte_python(byte_off: u32, byte_count: u32, endian: Endian) -> String {
    let shifts: Vec<String> = match endian {
        Endian::Big | Endian::Native => (0..byte_count)
            .map(|i| {
                let shift = (byte_count - 1 - i) * 8;
                let off = byte_off + i;
                if shift == 0 {
                    format!("raw[{off}]")
                } else {
                    format!("(raw[{off}] << {shift})")
                }
            })
            .collect(),
        Endian::Little => (0..byte_count)
            .map(|i| {
                let shift = i * 8;
                let off = byte_off + i;
                if shift == 0 {
                    format!("raw[{off}]")
                } else {
                    format!("(raw[{off}] << {shift})")
                }
            })
            .collect(),
    };

    shifts.join(" | ")
}

fn generate_encode_exprs_python(fields: &[CodecField], default_endian: Endian) -> Vec<String> {
    let mut exprs = Vec::new();

    let mut byte_groups: std::collections::BTreeMap<u32, Vec<&CodecField>> =
        std::collections::BTreeMap::new();

    for field in fields {
        if field.is_variable_length() {
            exprs.push(format!(
                "# variable-length field '{}' requires manual encode",
                field.id
            ));
        } else {
            byte_groups
                .entry(field.byte_offset)
                .or_default()
                .push(field);
        }
    }

    for (_, group) in &byte_groups {
        if group.len() == 1 {
            encode_single_field_python(group[0], default_endian, &mut exprs);
        } else {
            let mut parts = Vec::new();
            for field in group {
                let bit_off = field.bit_offset.unwrap_or(0);
                let bits = field.fixed_bits().unwrap_or(8);
                let mask = (1u64 << bits) - 1;
                parts.push(format!(
                    "(self.{} & 0x{mask:02X}) << {bit_off}",
                    field.id
                ));
            }
            exprs.push(format!("({}) & 0xFF", parts.join(" | ")));
        }
    }

    exprs
}

fn encode_single_field_python(
    field: &CodecField,
    default_endian: Endian,
    exprs: &mut Vec<String>,
) {
    let bit_off = field.bit_offset.unwrap_or(0);
    let endian = field.effective_endian(default_endian);

    match field.fixed_bits() {
        Some(8) if bit_off == 0 => {
            exprs.push(format!("self.{} & 0xFF", field.id));
        }
        Some(bits) if bits < 8 || bit_off > 0 => {
            let mask = (1u64 << bits) - 1;
            exprs.push(format!(
                "(self.{} & 0x{mask:02X}) << {bit_off} & 0xFF",
                field.id
            ));
        }
        Some(byte_count @ (16 | 24 | 32)) => {
            let n_bytes = byte_count / 8;
            let shifts: Vec<u32> = match endian {
                Endian::Big | Endian::Native => (0..n_bytes).rev().collect(),
                Endian::Little => (0..n_bytes).collect(),
            };
            for shift_byte in shifts {
                let shift = shift_byte * 8;
                if shift == 0 {
                    exprs.push(format!("self.{} & 0xFF", field.id));
                } else {
                    exprs.push(format!(
                        "(self.{} >> {shift}) & 0xFF",
                        field.id
                    ));
                }
            }
        }
        _ => exprs.push(format!("# encode {}", field.id)),
    }
}

// ── Python: Validator ────────────────────────────────────────

fn render_validator_python(
    env: &minijinja::Environment,
    m: &ValidatorModel,
) -> Result<String, String> {
    let rv = resolve_validator(m);
    let struct_name = filters::to_pascal_case(m.name.clone());

    let renames: Vec<(String, String)> = rv.inputs.iter()
        .map(|f| (f.id.clone(), filters::to_snake_case(f.id.clone())))
        .collect();

    let params = rv.inputs.iter()
        .map(|f| format!("{}: {}", filters::to_snake_case(f.id.clone()), python_type(&f.sce_type)))
        .collect::<Vec<_>>()
        .join(", ");

    let prev_vars: Vec<serde_json::Value> = rv.rocs.iter()
        .map(|roc| {
            let snake = filters::to_snake_case(roc.id.clone());
            serde_json::json!({
                "name": format!("prev_{snake}"),
                "id": snake,
                "is_float": roc.sce_type.is_float(),
            })
        })
        .collect();

    let range_rules: Vec<serde_json::Value> = rv.ranges.iter()
        .map(|r| serde_json::json!({
            "id": filters::to_snake_case(r.id.clone()),
            "min": r.min, "max": r.max,
            "has_min": r.min.is_some(), "has_max": r.max.is_some(),
        }))
        .collect();

    let roc_rules: Vec<serde_json::Value> = rv.rocs.iter()
        .map(|roc| {
            let snake = filters::to_snake_case(roc.id.clone());
            serde_json::json!({
                "id": snake,
                "max_delta": roc.max_delta,
                "prev_name": format!("prev_{snake}"),
            })
        })
        .collect();

    let plausibility_expr = match &rv.plausibility {
        Some(e) => {
            let mut expr_py = expr::transpile(e, ExprTarget::Python)?;
            expr_py = rust_rename_vars(&expr_py, &renames);
            Some(expr_py)
        }
        None => None,
    };

    let tmpl = env
        .get_template("validator.py.jinja2")
        .map_err(|e| format!("Template load error: {e}"))?;

    let ctx = minijinja::context! {
        struct_name => struct_name,
        params => params,
        prev_vars => minijinja::Value::from_serialize(&prev_vars),
        range_rules => minijinja::Value::from_serialize(&range_rules),
        roc_rules => minijinja::Value::from_serialize(&roc_rules),
        plausibility_expr => plausibility_expr,
    };

    tmpl.render(ctx).map_err(generator::render_error)
}

// ── Inline kind rendering (policy struct member functions) ─────
//
// Inline kinds live inside the policy struct — they access datamodel
// member variables directly via `this->`. This is distinct from standalone
// kinds, which are namespace-scoped free functions with explicit parameters.

/// Render all inline kinds as C++ policy struct member code.
/// Output is indented for embedding inside a struct body.
pub fn render_inline_kinds_cpp(
    kinds: &[InlineKind],
) -> Result<String, String> {
    let mut fragments = Vec::new();
    for kind in kinds {
        let code = render_single_inline_kind_member(kind)?;
        fragments.push(code);
    }
    Ok(fragments.join("\n"))
}

/// Render a single inline kind as a policy struct member.
fn render_single_inline_kind_member(kind: &InlineKind) -> Result<String, String> {
    match &kind.data {
        InlineKindData::Transform { inputs: _, expr, output_type } => {
            render_inline_transform_member(&kind.id, expr, output_type)
        }
        InlineKindData::Lookup { input_id, entries, default_value } => {
            render_inline_lookup_member(&kind.id, input_id, entries, default_value)
        }
        InlineKindData::Condition { expr } => {
            render_inline_condition_member(&kind.id, expr)
        }
        InlineKindData::Codec { fields, default_endian } => {
            render_inline_codec_member(&kind.id, fields, *default_endian)
        }
    }
}

/// Inline transform: const member function returning computed value from member variables.
fn render_inline_transform_member(
    id: &str,
    raw_expr: &str,
    output_type: &SceType,
) -> Result<String, String> {
    let expr_cpp = expr::transpile(raw_expr, ExprTarget::Cpp)?;
    let func_name = format!("compute{}", filters::to_pascal_case(id.to_string()));
    let ret_type = cpp_type(output_type);

    Ok(format!(
        "    // SCE Forge: Inline transform '{id}'\n\
         \x20   [[nodiscard]] {ret_type} {func_name}() const {{\n\
         \x20       return {expr_cpp};\n\
         \x20   }}"
    ))
}

/// Inline lookup: nested enum + const member function with switch.
fn render_inline_lookup_member(
    id: &str,
    input_id: &str,
    entries: &[LookupEntry],
    default_value: &str,
) -> Result<String, String> {
    let enum_name = filters::to_pascal_case(id.to_string());
    let func_name = format!("lookup{}", filters::to_pascal_case(id.to_string()));

    // Collect unique values preserving order
    let mut seen = std::collections::BTreeSet::new();
    let mut unique_values = Vec::new();
    for entry in entries {
        if seen.insert(entry.value.clone()) {
            unique_values.push(entry.value.clone());
        }
    }

    // Group entries by value
    let mut map: std::collections::BTreeMap<String, Vec<String>> =
        std::collections::BTreeMap::new();
    for entry in entries {
        map.entry(entry.value.clone())
            .or_default()
            .push(entry.key.clone());
    }

    let mut code = String::new();
    code.push_str(&format!(
        "    // SCE Forge: Inline lookup '{id}'\n\
         \x20   enum class {enum_name} {{ {} }};\n\n",
        unique_values.join(", ")
    ));

    // Static lookup function (takes explicit parameter — lookup input is external)
    code.push_str(&format!(
        "    static {enum_name} {func_name}(uint32_t {input_id}) {{\n\
         \x20       switch ({input_id}) {{\n"
    ));

    for (value, keys) in &map {
        for key in keys {
            code.push_str(&format!("        case {key}:\n"));
        }
        code.push_str(&format!("            return {enum_name}::{value};\n"));
    }

    code.push_str(&format!(
        "        default: return {enum_name}::{default_value};\n\
         \x20       }}\n\
         \x20   }}"
    ));

    Ok(code)
}

/// Inline condition: const member function returning bool from member variables.
fn render_inline_condition_member(
    id: &str,
    raw_expr: &str,
) -> Result<String, String> {
    let expr_cpp = expr::transpile(raw_expr, ExprTarget::Cpp)?;
    let func_name = filters::to_camel_case(id.to_string());

    Ok(format!(
        "    // SCE Forge: Inline condition '{id}'\n\
         \x20   [[nodiscard]] bool {func_name}() const {{\n\
         \x20       return {expr_cpp};\n\
         \x20   }}"
    ))
}

/// Inline codec: nested struct with static decode/encode methods.
fn render_inline_codec_member(
    id: &str,
    codec_fields: &[CodecField],
    default_endian: Endian,
) -> Result<String, String> {
    let struct_name = filters::to_pascal_case(id.to_string());

    // Compute min frame bytes
    let mut min_bytes = 0u32;
    for f in codec_fields {
        if let Some(bits) = f.fixed_bits() {
            let end = f.byte_offset + (bits + 7) / 8;
            min_bytes = min_bytes.max(end);
        }
    }

    let mut code = String::new();
    code.push_str(&format!("    // SCE Forge: Inline codec '{id}'\n"));
    code.push_str(&format!("    struct {struct_name} {{\n"));

    // Field declarations
    for f in codec_fields {
        code.push_str(&format!(
            "        {} {};\n",
            cpp_type(&f.sce_type),
            f.id
        ));
    }

    // decode
    code.push_str(&format!(
        "\n        static std::optional<{struct_name}> decode(const uint8_t* raw, size_t len) {{\n\
         \x20           if (len < {min_bytes}) return std::nullopt;\n\
         \x20           return {struct_name}{{\n"
    ));
    for f in codec_fields {
        let decode = generate_decode_expr(f, default_endian);
        code.push_str(&format!("                .{} = {},\n", f.id, decode));
    }
    code.push_str("            };\n        }\n");

    // encode
    let encode_exprs = generate_encode_exprs(codec_fields, default_endian);
    code.push_str("\n        std::vector<uint8_t> encode() const {\n            return {\n");
    for (i, expr_str) in encode_exprs.iter().enumerate() {
        let comma = if i < encode_exprs.len() - 1 { "," } else { "" };
        code.push_str(&format!("                {expr_str}{comma}\n"));
    }
    code.push_str("            };\n        }\n");

    code.push_str("    };");

    Ok(code)
}

// ── Naming helpers (delegating to filters where possible) ──────

fn to_upper_snake(s: &str) -> String {
    filters::to_snake_case(s.to_string()).to_uppercase()
}

/// Convert all-uppercase identifiers to PascalCase for Rust enum variants.
/// "STOP" → "Stop", "RUNNING" → "Running", "ENGINE_START" → "EngineStart".
/// Mixed-case input is delegated to filters::to_pascal_case.
fn to_rust_variant(s: &str) -> String {
    if s.chars().all(|c| c.is_ascii_uppercase() || c == '_' || c.is_ascii_digit()) {
        s.split('_')
            .filter(|p| !p.is_empty())
            .map(|word| {
                let mut chars = word.chars();
                match chars.next() {
                    Some(c) => {
                        let mut out = c.to_uppercase().to_string();
                        out.extend(chars.map(|c| c.to_ascii_lowercase()));
                        out
                    }
                    None => String::new(),
                }
            })
            .collect()
    } else {
        filters::to_pascal_case(s.to_string())
    }
}

/// Determine Rust cast type for integer-to-float promotion in transform expressions.
/// Returns Some("f64") when an integer input feeds a float output.
fn rust_float_cast(input_type: &SceType, output_type: &SceType) -> Option<&'static str> {
    let input_is_int = matches!(
        input_type,
        SceType::Uint8
            | SceType::Uint16
            | SceType::Uint32
            | SceType::Uint64
            | SceType::Int8
            | SceType::Int16
            | SceType::Int32
            | SceType::Int64
    );
    match (input_is_int, output_type) {
        (true, SceType::Float64) => Some("f64"),
        (true, SceType::Float32) => Some("f32"),
        _ => None,
    }
}

/// Check if any integer input needs a float cast for the given output type.
fn rust_float_cast_needed(inputs: &[ForgeField], output_type: &SceType) -> Option<&'static str> {
    inputs
        .iter()
        .find_map(|inp| rust_float_cast(&inp.sce_type, output_type))
}
