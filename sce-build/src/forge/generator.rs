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

// ── Cross-file import resolution ──────────────────────────────────

/// Template-ready import context for a single `<sce:import>`.
/// Per-language data is computed here; templates consume it directly.
#[derive(Debug, Clone, serde::Serialize)]
pub struct ImportContext {
    /// Alias from `<sce:import as="...">`.
    pub alias: String,
    /// Kind name (e.g., "codec", "transform").
    pub kind: String,
    /// PascalCase type name for the imported struct/class (stateful kinds).
    pub type_name: String,
    /// Language-specific include/import statement (full line).
    pub include_stmt: String,
    /// Whether this kind is stateful (needs member variable).
    pub is_stateful: bool,
    /// Language-specific member variable name (e.g., "frame_" for C++).
    pub member_name: String,
    /// Language-specific member type string (may differ from type_name for C++).
    pub member_type: String,
    /// Namespace/package for the imported kind (language-specific).
    pub namespace: String,
    /// For stateless kinds: the qualified function call expression that replaces
    /// the alias in expressions. E.g., for C++ transform import:
    /// `"SCE::Generated::TransformTemperature::computeTemperature"`.
    /// Empty for stateful kinds (use member access) or when not yet resolved.
    pub qualified_call: String,
}

/// Resolve a list of ForgeImport into template-ready ImportContext.
pub fn resolve_imports(
    imports: &[ForgeImport],
    lang: &crate::generator::Language,
) -> Vec<ImportContext> {
    imports.iter().map(|imp| resolve_single_import(imp, lang)).collect()
}

/// Build template-ready import data from resolved import contexts.
/// Returns `(has_imports, all_imports_serialized, stateful_imports_serialized)`.
///
/// - `all_imports`: every import (for include/import statements in templates)
/// - `stateful_imports`: only struct-based kinds (for member variable declarations)
fn build_template_imports(
    imports: &[ImportContext],
) -> (bool, minijinja::Value, minijinja::Value) {
    let has_imports = !imports.is_empty();
    let all = minijinja::Value::from_serialize(imports);
    let stateful: Vec<&ImportContext> = imports.iter().filter(|i| i.is_stateful).collect();
    let stateful_val = minijinja::Value::from_serialize(&stateful);
    (has_imports, all, stateful_val)
}

fn resolve_single_import(
    imp: &ForgeImport,
    lang: &crate::generator::Language,
) -> ImportContext {
    let stem = Path::new(&imp.src)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or(&imp.src)
        .to_string();

    let pascal = filters::to_pascal_case(stem.clone());
    let snake = filters::to_snake_case(stem.clone());
    let is_stateful = imp.kind.needs_instance();

    match lang {
        crate::generator::Language::Cpp => {
            let ns = pascal.clone();
            let type_name = pascal.clone();
            ImportContext {
                alias: imp.alias.clone(),
                kind: imp.kind.to_string(),
                include_stmt: format!("#include \"{snake}.h\""),
                type_name: type_name.clone(),
                is_stateful,
                member_name: format!("{}_", imp.alias),
                member_type: format!("::SCE::Generated::{ns}::{type_name}"),
                namespace: format!("SCE::Generated::{ns}"),
                qualified_call: String::new(),
            }
        }
        crate::generator::Language::Kotlin => {
            // Kotlin: same package assumed — no import statement needed.
            // If cross-package imports become necessary, this must generate
            // `import com.sce.generated.{package}.{Type}` statements.
            ImportContext {
                alias: imp.alias.clone(),
                kind: imp.kind.to_string(),
                include_stmt: String::new(),
                type_name: pascal.clone(),
                is_stateful,
                member_name: imp.alias.clone(),
                member_type: pascal.clone(),
                namespace: pascal.clone(),
                qualified_call: String::new(),
            }
        }
        crate::generator::Language::Rust => {
            ImportContext {
                alias: imp.alias.clone(),
                kind: imp.kind.to_string(),
                include_stmt: format!("use super::{snake}::{pascal};"),
                type_name: pascal.clone(),
                is_stateful,
                member_name: imp.alias.clone(),
                member_type: pascal.clone(),
                namespace: snake.clone(),
                qualified_call: String::new(),
            }
        }
        crate::generator::Language::Go => {
            let go_pascal = filters::to_pascal_case(imp.alias.to_string());
            ImportContext {
                alias: imp.alias.clone(),
                kind: imp.kind.to_string(),
                include_stmt: format!("\t\"{snake}\""),
                type_name: pascal.clone(),
                is_stateful,
                member_name: go_pascal,
                member_type: format!("{snake}.{pascal}"),
                namespace: snake.clone(),
                qualified_call: String::new(),
            }
        }
        crate::generator::Language::Python => {
            ImportContext {
                alias: imp.alias.clone(),
                kind: imp.kind.to_string(),
                include_stmt: format!("from .{snake} import {pascal}"),
                type_name: pascal.clone(),
                is_stateful,
                member_name: imp.alias.clone(),
                member_type: pascal.clone(),
                namespace: snake.clone(),
                qualified_call: String::new(),
            }
        }
    }
}

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
    generate_cpp_with_imports(doc, template_dir, &[])
}

/// Generate C++ code with cross-file import support.
pub fn generate_cpp_with_imports(
    doc: &ForgeDocument,
    template_dir: &Path,
    imports: &[ImportContext],
) -> Result<GeneratedOutput, String> {
    let forge_dir = template_dir.join("forge/cpp");
    let mut env = generator::new_env();
    generator::load_templates(&mut env, &forge_dir)?;

    let code = match doc {
        ForgeDocument::Transform(m) => render_transform_cpp(&env, m, imports)?,
        ForgeDocument::Lookup(m) => render_lookup_cpp(&env, m, imports)?,
        ForgeDocument::Condition(m) => render_condition_cpp(&env, m, imports)?,
        ForgeDocument::Codec(m) => render_codec_cpp(&env, m, imports)?,
        ForgeDocument::Validator(m) => render_validator_cpp(&env, m, imports)?,
        ForgeDocument::Procedure(m) => {
            if m.is_event_driven {
                render_procedure_l2_cpp(&env, m, imports)?
            } else {
                render_procedure_cpp(&env, m, imports)?
            }
        }
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
    imports: &[ImportContext],
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

    let (has_imports, all_imports, stateful_imports) = build_template_imports(imports);

    let ctx = minijinja::context! {
        guard => guard,
        namespace => ns,
        functions => minijinja::Value::from_serialize(&functions),
        has_imports => has_imports,
        imports => stateful_imports,
        all_imports => all_imports,
    };

    tmpl.render(ctx).map_err(generator::render_error)
}

// ── Lookup rendering ───────────────────────────────────────────

fn render_lookup_cpp(
    env: &minijinja::Environment,
    m: &LookupModel,
    imports: &[ImportContext],
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

    let (has_imports, all_imports, stateful_imports) = build_template_imports(imports);

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
        has_imports => has_imports,
        imports => stateful_imports,
        all_imports => all_imports,
    };

    tmpl.render(ctx).map_err(generator::render_error)
}

// ── Condition rendering ────────────────────────────────────────

fn render_condition_cpp(
    env: &minijinja::Environment,
    m: &ConditionModel,
    imports: &[ImportContext],
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

    let (has_imports, all_imports, stateful_imports) = build_template_imports(imports);

    let ctx = minijinja::context! {
        guard => guard,
        namespace => ns,
        func_name => func_name,
        params => params,
        expr => expr_cpp,
        has_imports => has_imports,
        imports => stateful_imports,
        all_imports => all_imports,
    };

    tmpl.render(ctx).map_err(generator::render_error)
}

// ── Codec rendering ────────────────────────────────────────────

fn render_codec_cpp(
    env: &minijinja::Environment,
    m: &CodecModel,
    imports: &[ImportContext],
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

    let (has_imports, all_imports, stateful_imports) = build_template_imports(imports);

    let ctx = minijinja::context! {
        guard => guard,
        namespace => ns,
        struct_name => struct_name,
        fields => minijinja::Value::from_serialize(&fields),
        min_bytes => m.min_frame_bytes(),
        encode_exprs => minijinja::Value::from_serialize(&encode_exprs),
        has_imports => has_imports,
        imports => stateful_imports,
        all_imports => all_imports,
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
    imports: &[ImportContext],
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

    // Build import alias rename map for expressions (stateless → qualified call)
    let import_renames: std::collections::HashMap<&str, &str> = imports
        .iter()
        .filter(|i| !i.is_stateful && !i.qualified_call.is_empty())
        .map(|i| (i.alias.as_str(), i.qualified_call.as_str()))
        .collect();

    let plausibility_expr = match &rv.plausibility {
        Some(e) if import_renames.is_empty() => Some(expr::transpile(e, ExprTarget::Cpp)?),
        Some(e) => Some(expr::transpile_with_renames(e, ExprTarget::Cpp, &import_renames)?),
        None => None,
    };

    let tmpl = env
        .get_template("validator.h.jinja2")
        .map_err(|e| format!("Template load error: {e}"))?;

    // Cross-file imports
    let (has_imports, all_imports, stateful_imports) = build_template_imports(imports);

    let ctx = minijinja::context! {
        guard => guard, namespace => ns, struct_name => struct_name,
        params => params,
        prev_vars => minijinja::Value::from_serialize(&prev_vars),
        range_rules => minijinja::Value::from_serialize(&range_rules),
        roc_rules => minijinja::Value::from_serialize(&roc_rules),
        plausibility_expr => plausibility_expr,
        has_imports => has_imports,
        imports => stateful_imports,
        all_imports => all_imports,
    };

    tmpl.render(ctx).map_err(generator::render_error)
}

// ══════════════════════════════════════════════════════════════
// ── Kotlin code generation ────────────────────────────────────
// ══════════════════════════════════════════════════════════════

/// Generate code from a ForgeDocument for Kotlin using Jinja2 templates.
pub fn generate_kotlin(doc: &ForgeDocument, template_dir: &Path) -> Result<GeneratedOutput, String> {
    generate_kotlin_with_imports(doc, template_dir, &[])
}

/// Generate Kotlin code with cross-file import support.
pub fn generate_kotlin_with_imports(
    doc: &ForgeDocument,
    template_dir: &Path,
    imports: &[ImportContext],
) -> Result<GeneratedOutput, String> {
    let forge_dir = template_dir.join("forge/kotlin");
    let mut env = generator::new_env();
    generator::load_templates(&mut env, &forge_dir)?;

    let code = match doc {
        ForgeDocument::Transform(m) => render_transform_kotlin(&env, m, imports)?,
        ForgeDocument::Lookup(m) => render_lookup_kotlin(&env, m, imports)?,
        ForgeDocument::Condition(m) => render_condition_kotlin(&env, m, imports)?,
        ForgeDocument::Codec(m) => render_codec_kotlin(&env, m, imports)?,
        ForgeDocument::Validator(m) => render_validator_kotlin(&env, m, imports)?,
        ForgeDocument::Procedure(m) => {
            if m.is_event_driven {
                render_procedure_l2_kotlin(&env, m, imports)?
            } else {
                render_procedure_kotlin(&env, m, imports)?
            }
        }
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
    imports: &[ImportContext],
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

    let (has_imports, all_imports, stateful_imports) = build_template_imports(imports);

    let ctx = minijinja::context! {
        package => package,
        functions => minijinja::Value::from_serialize(&functions),
        has_imports => has_imports,
        imports => stateful_imports,
        all_imports => all_imports,
    };

    tmpl.render(ctx).map_err(generator::render_error)
}

// ── Kotlin: Lookup ────────────────────────────────────────────

fn render_lookup_kotlin(
    env: &minijinja::Environment,
    m: &LookupModel,
    imports: &[ImportContext],
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

    let (has_imports, all_imports, stateful_imports) = build_template_imports(imports);

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
        has_imports => has_imports,
        imports => stateful_imports,
        all_imports => all_imports,
    };

    tmpl.render(ctx).map_err(generator::render_error)
}

// ── Kotlin: Condition ─────────────────────────────────────────

fn render_condition_kotlin(
    env: &minijinja::Environment,
    m: &ConditionModel,
    imports: &[ImportContext],
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

    let (has_imports, all_imports, stateful_imports) = build_template_imports(imports);

    let ctx = minijinja::context! {
        package => package,
        func_name => func_name,
        params => params,
        expr => expr_kt,
        has_imports => has_imports,
        imports => stateful_imports,
        all_imports => all_imports,
    };

    tmpl.render(ctx).map_err(generator::render_error)
}

// ── Kotlin: Codec ─────────────────────────────────────────────

fn render_codec_kotlin(
    env: &minijinja::Environment,
    m: &CodecModel,
    imports: &[ImportContext],
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

    let (has_imports, all_imports, stateful_imports) = build_template_imports(imports);

    let ctx = minijinja::context! {
        package => package,
        struct_name => struct_name,
        fields => minijinja::Value::from_serialize(&fields),
        min_bytes => m.min_frame_bytes(),
        encode_exprs => minijinja::Value::from_serialize(&encode_exprs),
        has_imports => has_imports,
        imports => stateful_imports,
        all_imports => all_imports,
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
    imports: &[ImportContext],
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

    // Build import alias rename map for expressions (stateless → qualified call)
    let import_renames: std::collections::HashMap<&str, &str> = imports
        .iter()
        .filter(|i| !i.is_stateful && !i.qualified_call.is_empty())
        .map(|i| (i.alias.as_str(), i.qualified_call.as_str()))
        .collect();

    let plausibility_expr = match &rv.plausibility {
        Some(e) => {
            let mut expr_kt = if import_renames.is_empty() {
                expr::transpile(e, ExprTarget::Kotlin)?
            } else {
                expr::transpile_with_renames(e, ExprTarget::Kotlin, &import_renames)?
            };
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

    // Cross-file imports
    let (has_imports, all_imports, stateful_imports) = build_template_imports(imports);

    let ctx = minijinja::context! {
        package => package, struct_name => struct_name,
        params => params,
        prev_vars => minijinja::Value::from_serialize(&prev_vars),
        range_rules => minijinja::Value::from_serialize(&range_rules),
        roc_rules => minijinja::Value::from_serialize(&roc_rules),
        plausibility_expr => plausibility_expr,
        has_imports => has_imports,
        imports => stateful_imports,
        all_imports => all_imports,
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
    generate_rust_with_imports(doc, template_dir, &[])
}

/// Generate Rust code with cross-file import support.
pub fn generate_rust_with_imports(
    doc: &ForgeDocument,
    template_dir: &Path,
    imports: &[ImportContext],
) -> Result<GeneratedOutput, String> {
    let forge_dir = template_dir.join("forge/rust");
    let mut env = generator::new_env();
    generator::load_templates(&mut env, &forge_dir)?;

    let code = match doc {
        ForgeDocument::Transform(m) => render_transform_rust(&env, m, imports)?,
        ForgeDocument::Lookup(m) => render_lookup_rust(&env, m, imports)?,
        ForgeDocument::Condition(m) => render_condition_rust(&env, m, imports)?,
        ForgeDocument::Codec(m) => render_codec_rust(&env, m, imports)?,
        ForgeDocument::Validator(m) => render_validator_rust(&env, m, imports)?,
        ForgeDocument::Procedure(m) => {
            if m.is_event_driven {
                render_procedure_l2_rust(&env, m, imports)?
            } else {
                render_procedure_rust(&env, m, imports)?
            }
        }
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
    imports: &[ImportContext],
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

    let (has_imports, all_imports, stateful_imports) = build_template_imports(imports);

    let ctx = minijinja::context! {
        functions => minijinja::Value::from_serialize(&functions),
        has_imports => has_imports,
        imports => stateful_imports,
        all_imports => all_imports,
    };

    tmpl.render(ctx).map_err(generator::render_error)
}

// ── Rust: Lookup ──────────────────────────────────────────────

fn render_lookup_rust(
    env: &minijinja::Environment,
    m: &LookupModel,
    imports: &[ImportContext],
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

    let (has_imports, all_imports, stateful_imports) = build_template_imports(imports);

    let ctx = minijinja::context! {
        enum_name => enum_name,
        func_name => func_name,
        input_type => rust_param_type(&m.input.sce_type),
        input_id => input_id_snake,
        unique_values => minijinja::Value::from_serialize(&unique_values),
        entries_by_value => minijinja::Value::from_serialize(&entries_by_value),
        default_value => default_value,
        has_imports => has_imports,
        imports => stateful_imports,
        all_imports => all_imports,
    };

    tmpl.render(ctx).map_err(generator::render_error)
}

// ── Rust: Condition ───────────────────────────────────────────

fn render_condition_rust(
    env: &minijinja::Environment,
    m: &ConditionModel,
    imports: &[ImportContext],
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

    let (has_imports, all_imports, stateful_imports) = build_template_imports(imports);

    let ctx = minijinja::context! {
        func_name => func_name,
        params => params,
        expr => expr_rs,
        has_imports => has_imports,
        imports => stateful_imports,
        all_imports => all_imports,
    };

    tmpl.render(ctx).map_err(generator::render_error)
}

// ── Rust: Codec ───────────────────────────────────────────────

fn render_codec_rust(
    env: &minijinja::Environment,
    m: &CodecModel,
    imports: &[ImportContext],
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

    let (has_imports, all_imports, stateful_imports) = build_template_imports(imports);

    let ctx = minijinja::context! {
        struct_name => struct_name,
        fields => minijinja::Value::from_serialize(&fields),
        min_bytes => m.min_frame_bytes(),
        encode_exprs => minijinja::Value::from_serialize(&encode_exprs),
        has_imports => has_imports,
        imports => stateful_imports,
        all_imports => all_imports,
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
    imports: &[ImportContext],
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

    // Build import alias rename map for expressions (stateless → qualified call)
    let import_renames: std::collections::HashMap<&str, &str> = imports
        .iter()
        .filter(|i| !i.is_stateful && !i.qualified_call.is_empty())
        .map(|i| (i.alias.as_str(), i.qualified_call.as_str()))
        .collect();

    let plausibility_expr = match &rv.plausibility {
        Some(e) => {
            let mut expr_rs = if import_renames.is_empty() {
                expr::transpile(e, ExprTarget::Rust)?
            } else {
                expr::transpile_with_renames(e, ExprTarget::Rust, &import_renames)?
            };
            expr_rs = rust_rename_vars(&expr_rs, &renames);
            Some(expr_rs)
        }
        None => None,
    };

    let tmpl = env
        .get_template("validator.rs.jinja2")
        .map_err(|e| format!("Template load error: {e}"))?;

    // Cross-file imports
    let (has_imports, all_imports, stateful_imports) = build_template_imports(imports);

    let ctx = minijinja::context! {
        struct_name => struct_name,
        params => params,
        prev_vars => minijinja::Value::from_serialize(&prev_vars),
        range_rules => minijinja::Value::from_serialize(&range_rules),
        roc_rules => minijinja::Value::from_serialize(&roc_rules),
        plausibility_expr => plausibility_expr,
        has_imports => has_imports,
        imports => stateful_imports,
        all_imports => all_imports,
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
    generate_go_with_imports(doc, template_dir, &[])
}

/// Generate Go code with cross-file import support.
pub fn generate_go_with_imports(
    doc: &ForgeDocument,
    template_dir: &Path,
    imports: &[ImportContext],
) -> Result<GeneratedOutput, String> {
    let forge_dir = template_dir.join("forge/go");
    let mut env = generator::new_env();
    generator::load_templates(&mut env, &forge_dir)?;

    let code = match doc {
        ForgeDocument::Transform(m) => render_transform_go(&env, m, imports)?,
        ForgeDocument::Lookup(m) => render_lookup_go(&env, m, imports)?,
        ForgeDocument::Condition(m) => render_condition_go(&env, m, imports)?,
        ForgeDocument::Codec(m) => render_codec_go(&env, m, imports)?,
        ForgeDocument::Validator(m) => render_validator_go(&env, m, imports)?,
        ForgeDocument::Procedure(m) => {
            if m.is_event_driven {
                render_procedure_l2_go(&env, m, imports)?
            } else {
                render_procedure_go(&env, m, imports)?
            }
        }
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
    imports: &[ImportContext],
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

    let (has_imports, all_imports, stateful_imports) = build_template_imports(imports);

    let ctx = minijinja::context! {
        package => package,
        functions => minijinja::Value::from_serialize(&functions),
        has_imports => has_imports,
        imports => stateful_imports,
        all_imports => all_imports,
    };

    tmpl.render(ctx).map_err(generator::render_error)
}

// ── Go: Lookup ───────────────────────────────────────────────

fn render_lookup_go(
    env: &minijinja::Environment,
    m: &LookupModel,
    imports: &[ImportContext],
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

    let (has_imports, all_imports, stateful_imports) = build_template_imports(imports);

    let ctx = minijinja::context! {
        package => package,
        enum_name => enum_name,
        func_name => func_name,
        input_type => go_type(&m.input.sce_type),
        input_id => input_id_safe,
        unique_values => minijinja::Value::from_serialize(&m.unique_values()),
        entries_by_value => minijinja::Value::from_serialize(&entries_by_value),
        default_value => &m.default_value,
        has_imports => has_imports,
        imports => stateful_imports,
        all_imports => all_imports,
    };

    tmpl.render(ctx).map_err(generator::render_error)
}

// ── Go: Condition ────────────────────────────────────────────

fn render_condition_go(
    env: &minijinja::Environment,
    m: &ConditionModel,
    imports: &[ImportContext],
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

    let (has_imports, all_imports, stateful_imports) = build_template_imports(imports);

    let ctx = minijinja::context! {
        package => package,
        func_name => func_name,
        params => params,
        expr => expr_go,
        has_imports => has_imports,
        imports => stateful_imports,
        all_imports => all_imports,
    };

    tmpl.render(ctx).map_err(generator::render_error)
}

// ── Go: Codec ────────────────────────────────────────────────

fn render_codec_go(
    env: &minijinja::Environment,
    m: &CodecModel,
    imports: &[ImportContext],
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

    let (has_imports, all_imports, stateful_imports) = build_template_imports(imports);

    let ctx = minijinja::context! {
        package => package,
        struct_name => struct_name,
        fields => minijinja::Value::from_serialize(&fields),
        min_bytes => m.min_frame_bytes(),
        encode_exprs => minijinja::Value::from_serialize(&encode_exprs),
        has_imports => has_imports,
        imports => stateful_imports,
        all_imports => all_imports,
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
    imports: &[ImportContext],
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

    // Build import alias rename map for expressions (stateless → qualified call)
    let import_renames: std::collections::HashMap<&str, &str> = imports
        .iter()
        .filter(|i| !i.is_stateful && !i.qualified_call.is_empty())
        .map(|i| (i.alias.as_str(), i.qualified_call.as_str()))
        .collect();

    let plausibility_expr = match &rv.plausibility {
        Some(e) => {
            let mut expr_go = if import_renames.is_empty() {
                expr::transpile(e, ExprTarget::Go)?
            } else {
                expr::transpile_with_renames(e, ExprTarget::Go, &import_renames)?
            };
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

    // Cross-file imports
    let (has_imports, all_imports, stateful_imports) = build_template_imports(imports);

    let ctx = minijinja::context! {
        package => package,
        struct_name => struct_name,
        params => params,
        prev_vars => minijinja::Value::from_serialize(&prev_vars),
        range_rules => minijinja::Value::from_serialize(&range_rules),
        roc_rules => minijinja::Value::from_serialize(&roc_rules),
        plausibility_expr => plausibility_expr,
        has_imports => has_imports,
        imports => stateful_imports,
        all_imports => all_imports,
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
    generate_python_with_imports(doc, template_dir, &[])
}

/// Generate Python code with cross-file import support.
pub fn generate_python_with_imports(
    doc: &ForgeDocument,
    template_dir: &Path,
    imports: &[ImportContext],
) -> Result<GeneratedOutput, String> {
    let forge_dir = template_dir.join("forge/python");
    let mut env = generator::new_env();
    generator::load_templates(&mut env, &forge_dir)?;

    let code = match doc {
        ForgeDocument::Transform(m) => render_transform_python(&env, m, imports)?,
        ForgeDocument::Lookup(m) => render_lookup_python(&env, m, imports)?,
        ForgeDocument::Condition(m) => render_condition_python(&env, m, imports)?,
        ForgeDocument::Codec(m) => render_codec_python(&env, m, imports)?,
        ForgeDocument::Validator(m) => render_validator_python(&env, m, imports)?,
        ForgeDocument::Procedure(m) => {
            if m.is_event_driven {
                render_procedure_l2_python(&env, m, imports)?
            } else {
                render_procedure_python(&env, m, imports)?
            }
        }
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
    imports: &[ImportContext],
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

    let (has_imports, all_imports, stateful_imports) = build_template_imports(imports);

    let ctx = minijinja::context! {
        functions => minijinja::Value::from_serialize(&functions),
        has_imports => has_imports,
        imports => stateful_imports,
        all_imports => all_imports,
    };

    tmpl.render(ctx).map_err(generator::render_error)
}

// ── Python: Lookup ───────────────────────────────────────────

fn render_lookup_python(
    env: &minijinja::Environment,
    m: &LookupModel,
    imports: &[ImportContext],
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

    let (has_imports, all_imports, stateful_imports) = build_template_imports(imports);

    let ctx = minijinja::context! {
        enum_name => enum_name,
        func_name => func_name,
        input_type => python_type(&m.input.sce_type),
        input_id => input_id_snake,
        unique_values => minijinja::Value::from_serialize(&m.unique_values()),
        entries_by_value => minijinja::Value::from_serialize(&entries_by_value),
        default_value => &m.default_value,
        has_imports => has_imports,
        imports => stateful_imports,
        all_imports => all_imports,
    };

    tmpl.render(ctx).map_err(generator::render_error)
}

// ── Python: Condition ────────────────────────────────────────

fn render_condition_python(
    env: &minijinja::Environment,
    m: &ConditionModel,
    imports: &[ImportContext],
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

    let (has_imports, all_imports, stateful_imports) = build_template_imports(imports);

    let ctx = minijinja::context! {
        func_name => func_name,
        params => params,
        expr => expr_py,
        has_imports => has_imports,
        imports => stateful_imports,
        all_imports => all_imports,
    };

    tmpl.render(ctx).map_err(generator::render_error)
}

// ── Python: Codec ────────────────────────────────────────────

fn render_codec_python(
    env: &minijinja::Environment,
    m: &CodecModel,
    imports: &[ImportContext],
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

    let (has_imports, all_imports, stateful_imports) = build_template_imports(imports);

    let ctx = minijinja::context! {
        struct_name => struct_name,
        fields => minijinja::Value::from_serialize(&fields),
        min_bytes => m.min_frame_bytes(),
        encode_exprs => minijinja::Value::from_serialize(&encode_exprs),
        has_imports => has_imports,
        imports => stateful_imports,
        all_imports => all_imports,
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
    imports: &[ImportContext],
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

    // Build import alias rename map for expressions (stateless → qualified call)
    let import_renames: std::collections::HashMap<&str, &str> = imports
        .iter()
        .filter(|i| !i.is_stateful && !i.qualified_call.is_empty())
        .map(|i| (i.alias.as_str(), i.qualified_call.as_str()))
        .collect();

    let plausibility_expr = match &rv.plausibility {
        Some(e) => {
            let mut expr_py = if import_renames.is_empty() {
                expr::transpile(e, ExprTarget::Python)?
            } else {
                expr::transpile_with_renames(e, ExprTarget::Python, &import_renames)?
            };
            expr_py = rust_rename_vars(&expr_py, &renames);
            Some(expr_py)
        }
        None => None,
    };

    let tmpl = env
        .get_template("validator.py.jinja2")
        .map_err(|e| format!("Template load error: {e}"))?;

    // Cross-file imports
    let (has_imports, all_imports, stateful_imports) = build_template_imports(imports);

    let ctx = minijinja::context! {
        struct_name => struct_name,
        params => params,
        prev_vars => minijinja::Value::from_serialize(&prev_vars),
        range_rules => minijinja::Value::from_serialize(&range_rules),
        roc_rules => minijinja::Value::from_serialize(&roc_rules),
        plausibility_expr => plausibility_expr,
        has_imports => has_imports,
        imports => stateful_imports,
        all_imports => all_imports,
    };

    tmpl.render(ctx).map_err(generator::render_error)
}

// ══════════════════════════════════════════════════════════════
// ── Procedure: resolved model (state-index mapping, computed once) ──
// ══════════════════════════════════════════════════════════════

/// A transition with its target pre-resolved to a state index.
struct ResolvedTransition {
    target_index: usize,
    /// Guard expression, pre-transpiled to the target language.
    cond: Option<String>,
}

/// A state with its transitions pre-resolved.
struct ResolvedProcedureState {
    id: String,
    index: usize,
    is_final: bool,
    transitions: Vec<ResolvedTransition>,
}

/// Procedure model with state-to-index associations pre-resolved.
/// Eliminates repeated name lookups across 5 language renderers.
struct ResolvedProcedure {
    inputs: Vec<ForgeField>,
    states: Vec<ResolvedProcedureState>,
    initial_index: usize,
    state_count: usize,
}

fn resolve_procedure(m: &ProcedureModel, target: ExprTarget) -> Result<ResolvedProcedure, String> {
    // Build name→index map
    let index_map: std::collections::BTreeMap<&str, usize> = m
        .states
        .iter()
        .enumerate()
        .map(|(i, s)| (s.id.as_str(), i))
        .collect();

    let initial_index = *index_map
        .get(m.initial.as_str())
        .ok_or_else(|| format!("Initial state '{}' not found", m.initial))?;

    let states = m
        .states
        .iter()
        .enumerate()
        .map(|(i, s)| {
            let transitions = s
                .transitions
                .iter()
                .map(|tr| {
                    let target_index = *index_map
                        .get(tr.target.as_str())
                        .ok_or_else(|| format!("Transition target '{}' not found", tr.target))?;
                    let cond = match &tr.cond {
                        Some(c) => Some(expr::transpile(c, target)?),
                        None => None,
                    };
                    Ok(ResolvedTransition {
                        target_index,
                        cond,
                    })
                })
                .collect::<Result<Vec<_>, String>>()?;

            Ok(ResolvedProcedureState {
                id: s.id.clone(),
                index: i,
                is_final: s.is_final,
                transitions,
            })
        })
        .collect::<Result<Vec<_>, String>>()?;

    Ok(ResolvedProcedure {
        inputs: m.inputs.clone(),
        states,
        initial_index,
        state_count: m.states.len(),
    })
}

/// Build common procedure data structures for template rendering.
fn build_procedure_data(rp: &ResolvedProcedure) -> (Vec<serde_json::Value>, Vec<serde_json::Value>, Vec<usize>) {
    let state_names: Vec<serde_json::Value> = rp
        .states
        .iter()
        .map(|s| serde_json::json!(s.id))
        .collect();

    let non_final_states: Vec<serde_json::Value> = rp
        .states
        .iter()
        .filter(|s| !s.is_final)
        .map(|s| {
            let transitions: Vec<serde_json::Value> = s
                .transitions
                .iter()
                .map(|tr| {
                    serde_json::json!({
                        "target_index": tr.target_index,
                        "has_cond": tr.cond.is_some(),
                        "cond": tr.cond.as_deref().unwrap_or(""),
                    })
                })
                .collect();
            serde_json::json!({
                "index": s.index,
                "id": s.id,
                "transitions": transitions,
            })
        })
        .collect();

    let final_indices: Vec<usize> = rp
        .states
        .iter()
        .filter(|s| s.is_final)
        .map(|s| s.index)
        .collect();

    (state_names, non_final_states, final_indices)
}

/// Build a final-state check expression for the given operator (e.g., "||", "or").
fn final_check_expr(final_indices: &[usize], var: &str, op: &str) -> String {
    final_indices
        .iter()
        .map(|i| format!("{var} == {i}"))
        .collect::<Vec<_>>()
        .join(&format!(" {op} "))
}

// ── Procedure: C++ ──────────────────────────────────────────

fn render_procedure_cpp(
    env: &minijinja::Environment,
    m: &ProcedureModel,
    imports: &[ImportContext],
) -> Result<String, String> {
    let rp = resolve_procedure(m, ExprTarget::Cpp)?;
    let ns = filters::to_pascal_case(m.name.clone());
    let guard = format!("SCE_FORGE_{}_H", to_upper_snake(&m.name));
    let struct_name = filters::to_pascal_case(m.name.clone());

    let params = rp
        .inputs
        .iter()
        .map(|inp| format!("{} {}", cpp_param_type(&inp.sce_type), inp.id))
        .collect::<Vec<_>>()
        .join(", ");

    let (state_names, non_final_states, final_indices) = build_procedure_data(&rp);

    let state_enum: Vec<serde_json::Value> = rp
        .states
        .iter()
        .map(|s| {
            serde_json::json!({
                "name": to_upper_snake(&s.id),
                "index": s.index,
            })
        })
        .collect();

    let final_check = final_check_expr(&final_indices, "s", "||");

    let tmpl = env
        .get_template("procedure.h.jinja2")
        .map_err(|e| format!("Template load error: {e}"))?;

    let (has_imports, all_imports, stateful_imports) = build_template_imports(imports);

    let ctx = minijinja::context! {
        guard => guard, namespace => ns, struct_name => struct_name,
        params => params,
        state_enum => minijinja::Value::from_serialize(&state_enum),
        state_names => minijinja::Value::from_serialize(&state_names),
        state_count => rp.state_count,
        initial_index => rp.initial_index,
        non_final_states => minijinja::Value::from_serialize(&non_final_states),
        final_check => final_check,
        has_imports => has_imports,
        imports => stateful_imports,
        all_imports => all_imports,
    };

    tmpl.render(ctx).map_err(generator::render_error)
}

// ── Procedure Level 2: C++ (event-driven, StaticExecutionEngine) ──

fn render_procedure_l2_cpp(
    env: &minijinja::Environment,
    m: &ProcedureModel,
    imports: &[ImportContext],
) -> Result<String, String> {
    let pascal = filters::to_pascal_case(m.name.clone());
    let guard = format!("SCE_FORGE_{}_L2_H", to_upper_snake(&m.name));
    let policy_name = format!("{}Policy", &pascal);

    // Build state enum
    let state_enum: Vec<serde_json::Value> = m
        .states
        .iter()
        .enumerate()
        .map(|(i, s)| {
            serde_json::json!({
                "name": filters::to_pascal_case(s.id.clone()),
                "index": i,
            })
        })
        .collect();

    // Collect unique events: original SCXML string → PascalCase enum name.
    // BTreeMap orders by raw SCXML event string (key).
    let mut event_raw_to_pascal: std::collections::BTreeMap<String, String> = std::collections::BTreeMap::new();
    event_raw_to_pascal.insert("ok".to_string(), "Ok".to_string());
    event_raw_to_pascal.insert("fail".to_string(), "Fail".to_string());
    for s in &m.states {
        for tr in &s.transitions {
            if let Some(ev) = &tr.event {
                event_raw_to_pascal
                    .entry(ev.clone())
                    .or_insert_with(|| filters::to_pascal_case(ev.clone()));
            }
        }
    }

    // Build event enum data: PascalCase enum variant name + original SCXML event string.
    // Deduplicate by PascalCase name (multiple raw strings could map to same enum variant).
    let mut seen_pascal: std::collections::BTreeSet<String> = std::collections::BTreeSet::new();
    let event_enum: Vec<serde_json::Value> = event_raw_to_pascal
        .iter()
        .filter(|(_, pascal)| seen_pascal.insert((*pascal).clone()))
        .enumerate()
        .map(|(i, (raw, pascal))| {
            serde_json::json!({
                "name": pascal,
                "index": i + 1,
                "event_name": raw,
            })
        })
        .collect();

    // Build event name → enum name map keyed by ORIGINAL SCXML event string.
    // This ensures transition matching works for any casing convention
    // (e.g., "ok", "REQUEST_COMPLETE", "requestComplete" all map correctly).
    let event_name_map: &std::collections::BTreeMap<String, String> = &event_raw_to_pascal;

    // Build input field data
    let input_fields: Vec<serde_json::Value> = m
        .inputs
        .iter()
        .map(|f| {
            serde_json::json!({
                "id": f.id,
                "cpp_type": cpp_type(&f.sce_type),
                "cpp_param_type": cpp_param_type(&f.sce_type),
                "setter_name": filters::to_pascal_case(f.id.clone()),
            })
        })
        .collect();

    // Build internal field data
    let internal_fields: Vec<serde_json::Value> = m
        .internals
        .iter()
        .map(|f| {
            let default_val = f.expr.as_ref().map(|e| {
                expr::transpile(e, ExprTarget::Cpp).unwrap_or_else(|_| e.clone())
            });
            serde_json::json!({
                "id": f.id,
                "cpp_type": cpp_type(&f.sce_type),
                "default_value": default_val,
            })
        })
        .collect();

    // Initial state
    let initial_state = filters::to_pascal_case(m.initial.clone());

    // Build variable name list for expression rewriting (input + internal names)
    let var_name_strings: Vec<String> = m
        .inputs
        .iter()
        .chain(m.internals.iter())
        .map(|f| f.id.clone())
        .collect();
    let var_names: Vec<&str> = var_name_strings.iter().map(|s| s.as_str()).collect();

    // Pre-build rename maps once (CR#4: avoid per-expression HashMap rebuild)
    let mut owned_rename_map = build_rename_map(&var_names);
    // Add import alias renames: `frame` → `frame_` for C++ member access
    for imp in imports {
        if imp.is_stateful {
            owned_rename_map.insert(&imp.alias, imp.member_name.clone());
        }
    }
    let rename_map: std::collections::HashMap<&str, &str> = owned_rename_map
        .iter()
        .map(|(k, v)| (*k, v.as_str()))
        .collect();
    let mut owned_assign_rename_map = owned_rename_map.clone();
    owned_assign_rename_map.insert("_event.data", "pendingEventData_".to_string());
    let assign_rename_map: std::collections::HashMap<&str, &str> = owned_assign_rename_map
        .iter()
        .map(|(k, v)| (*k, v.as_str()))
        .collect();

    // Final states
    let final_states: Vec<serde_json::Value> = m
        .states
        .iter()
        .filter(|s| s.is_final)
        .map(|s| {
            serde_json::json!({
                "name": filters::to_pascal_case(s.id.clone()),
                "id": s.id,
            })
        })
        .collect();

    // States with onentry sends
    let states_with_entry: Vec<serde_json::Value> = m
        .states
        .iter()
        .filter(|s| !s.on_entry_sends.is_empty())
        .map(|s| {
            let sends: Vec<serde_json::Value> = s
                .on_entry_sends
                .iter()
                .map(|send| {
                    let addr_expr = send.addr.as_ref().map(|a| {
                        transpile_l2_expr(a, ExprTarget::Cpp, &rename_map)
                    });
                    let payload_expr = send.payload.as_ref().map(|p| {
                        transpile_l2_expr(p, ExprTarget::Cpp, &rename_map)
                    });
                    serde_json::json!({
                        "service": send.service,
                        "subfunc": send.subfunc,
                        "has_addr": send.addr.is_some(),
                        "addr_expr": addr_expr.unwrap_or_default(),
                        "payload": send.payload.is_some(),
                        "payload_expr": payload_expr.unwrap_or_default(),
                    })
                })
                .collect();
            serde_json::json!({
                "name": filters::to_pascal_case(s.id.clone()),
                "sends": sends,
            })
        })
        .collect();

    // Final states with done data
    let final_states_with_donedata: Vec<serde_json::Value> = m
        .states
        .iter()
        .filter(|s| s.is_final && !s.done_params.is_empty())
        .map(|s| {
            let done_params: Vec<serde_json::Value> = s
                .done_params
                .iter()
                .map(|p| {
                    let transpiled = transpile_l2_expr(&p.expr, ExprTarget::Cpp, &rename_map);
                    serde_json::json!({
                        "name": p.name,
                        "expr": transpiled,
                    })
                })
                .collect();
            serde_json::json!({
                "name": filters::to_pascal_case(s.id.clone()),
                "done_params": done_params,
            })
        })
        .collect();

    // Non-final states with transitions
    let non_final_states: Vec<serde_json::Value> = m
        .states
        .iter()
        .filter(|s| !s.is_final)
        .map(|s| {
            let transitions: Vec<serde_json::Value> = s
                .transitions
                .iter()
                .enumerate()
                .map(|(idx, tr)| {
                    let event_enum_name = tr.event.as_ref().map(|ev| {
                        event_name_map
                            .get(ev)
                            .cloned()
                            .unwrap_or_else(|| filters::to_pascal_case(ev.clone()))
                    });
                    let cond_transpiled = tr.cond.as_ref().map(|c| {
                        transpile_l2_expr(c, ExprTarget::Cpp, &rename_map)
                    });
                    serde_json::json!({
                        "index": idx,
                        "has_event": tr.event.is_some(),
                        "event_name": tr.event.as_deref().unwrap_or(""),
                        "event_enum": event_enum_name.unwrap_or_default(),
                        "has_cond": tr.cond.is_some(),
                        "cond": cond_transpiled.unwrap_or_default(),
                        "target_name": filters::to_pascal_case(tr.target.clone()),
                        "has_assigns": !tr.assigns.is_empty(),
                    })
                })
                .collect();
            serde_json::json!({
                "name": filters::to_pascal_case(s.id.clone()),
                "transitions": transitions,
            })
        })
        .collect();

    // Build type map for assign type checking (variable name → SceType)
    let type_map: std::collections::HashMap<&str, &SceType> = m
        .inputs
        .iter()
        .chain(m.internals.iter())
        .map(|f| (f.id.as_str(), &f.sce_type))
        .collect();

    // States that have transitions with assigns
    let states_with_assigns: Vec<serde_json::Value> = m
        .states
        .iter()
        .filter(|s| s.transitions.iter().any(|tr| !tr.assigns.is_empty()))
        .map(|s| {
            let assign_transitions: Vec<serde_json::Value> = s
                .transitions
                .iter()
                .enumerate()
                .filter(|(_, tr)| !tr.assigns.is_empty())
                .map(|(idx, tr)| {
                    let assigns: Vec<serde_json::Value> = tr
                        .assigns
                        .iter()
                        .map(|a| {
                            let transpiled = transpile_l2_expr(&a.expr, ExprTarget::Cpp, &assign_rename_map);
                            // Type-aware wrapping: if target is bytes and source is string-like,
                            // generate explicit conversion
                            let wrapped = match type_map.get(a.location.as_str()) {
                                Some(SceType::Bytes) if a.expr.trim() == "_event.data" => {
                                    // string → vector<uint8_t> conversion (exact _event.data match only)
                                    format!("std::vector<uint8_t>({transpiled}.begin(), {transpiled}.end())")
                                }
                                _ => transpiled,
                            };
                            serde_json::json!({
                                "location": a.location,
                                "expr": wrapped,
                            })
                        })
                        .collect();
                    serde_json::json!({
                        "index": idx,
                        "assigns": assigns,
                    })
                })
                .collect();
            serde_json::json!({
                "name": filters::to_pascal_case(s.id.clone()),
                "assign_transitions": assign_transitions,
            })
        })
        .collect();

    // Collect raw sce:payload expressions for header dependency comment (CR#6)
    let payload_exprs: Vec<String> = m
        .states
        .iter()
        .flat_map(|s| s.on_entry_sends.iter())
        .filter_map(|send| send.payload.clone())
        .collect();
    let has_external_deps = !payload_exprs.is_empty();

    let tmpl = env
        .get_template("procedure_l2.h.jinja2")
        .map_err(|e| format!("Template load error: {e}"))?;

    // Cross-file imports: stateful imports become member variables
    let (has_imports, all_imports, stateful_imports) = build_template_imports(imports);

    let ctx = minijinja::context! {
        guard => guard,
        namespace => &pascal,
        policy_name => policy_name,
        class_name => &pascal,
        pascal_name => &pascal,
        state_enum => minijinja::Value::from_serialize(&state_enum),
        event_enum => minijinja::Value::from_serialize(&event_enum),
        input_fields => minijinja::Value::from_serialize(&input_fields),
        internal_fields => minijinja::Value::from_serialize(&internal_fields),
        initial_state => initial_state,
        final_states => minijinja::Value::from_serialize(&final_states),
        states_with_entry => minijinja::Value::from_serialize(&states_with_entry),
        final_states_with_donedata => minijinja::Value::from_serialize(&final_states_with_donedata),
        non_final_states => minijinja::Value::from_serialize(&non_final_states),
        states_with_assigns => minijinja::Value::from_serialize(&states_with_assigns),
        has_external_deps => has_external_deps,
        payload_exprs => minijinja::Value::from_serialize(&payload_exprs),
        has_imports => has_imports,
        imports => stateful_imports,
        all_imports => all_imports,
    };

    tmpl.render(ctx).map_err(generator::render_error)
}

/// Build a rename map from datamodel variable names to policy member names.
/// `retryCount` → `retryCount_`. Variables not in the map (e.g., `_event`) are left as-is.
fn build_rename_map<'a>(var_names: &'a [&'a str]) -> std::collections::HashMap<&'a str, String> {
    var_names
        .iter()
        .map(|name| (*name, format!("{}_", name)))
        .collect()
}

/// Transpile a Level 2 procedure expression with a pre-built rename map.
/// On failure, emits a C++ comment with the error for compile-time visibility.
fn transpile_l2_expr(raw: &str, target: ExprTarget, renames: &std::collections::HashMap<&str, &str>) -> String {
    match expr::transpile_with_renames(raw, target, renames) {
        Ok(result) => result,
        Err(e) => format!("/* SCE_TRANSPILE_ERROR: {} */ {}", e, raw),
    }
}

// ── Procedure: Kotlin ───────────────────────────────────────

fn render_procedure_kotlin(
    env: &minijinja::Environment,
    m: &ProcedureModel,
    imports: &[ImportContext],
) -> Result<String, String> {
    let rp = resolve_procedure(m, ExprTarget::Kotlin)?;
    let package = filters::to_snake_case(m.name.clone());
    let struct_name = filters::to_pascal_case(m.name.clone());

    let params = rp
        .inputs
        .iter()
        .map(|inp| format!("{}: {}", inp.id, kotlin_type(&inp.sce_type)))
        .collect::<Vec<_>>()
        .join(", ");

    let (state_names, _, final_indices) = build_procedure_data(&rp);

    // Apply unsigned conversion to guard expressions if needed
    let has_unsigned = rp.inputs.iter().any(|f| f.sce_type.is_unsigned());
    let non_final_states: Vec<serde_json::Value> = rp
        .states
        .iter()
        .filter(|s| !s.is_final)
        .map(|s| {
            let transitions: Vec<serde_json::Value> = s
                .transitions
                .iter()
                .map(|tr| {
                    let cond = match &tr.cond {
                        Some(c) if has_unsigned && kotlin_condition_needs_conversion(c) => {
                            let conversions: Vec<(String, String)> = rp
                                .inputs
                                .iter()
                                .filter_map(|f| {
                                    kotlin_unsigned_conversion(&f.sce_type)
                                        .map(|cv| (f.id.clone(), cv.to_string()))
                                })
                                .collect();
                            Some(kotlin_wrap_expr(c, &conversions))
                        }
                        Some(c) => Some(c.clone()),
                        None => None,
                    };
                    serde_json::json!({
                        "target_index": tr.target_index,
                        "has_cond": cond.is_some(),
                        "cond": cond.unwrap_or_default(),
                    })
                })
                .collect();
            serde_json::json!({
                "index": s.index,
                "id": s.id,
                "transitions": transitions,
            })
        })
        .collect();

    let final_check = final_check_expr(&final_indices, "current", "||");

    let tmpl = env
        .get_template("procedure.kt.jinja2")
        .map_err(|e| format!("Template load error: {e}"))?;

    let (has_imports, all_imports, stateful_imports) = build_template_imports(imports);

    let ctx = minijinja::context! {
        package => package, struct_name => struct_name,
        params => params,
        state_names => minijinja::Value::from_serialize(&state_names),
        state_count => rp.state_count,
        initial_index => rp.initial_index,
        non_final_states => minijinja::Value::from_serialize(&non_final_states),
        final_check => final_check,
        has_imports => has_imports,
        imports => stateful_imports,
        all_imports => all_imports,
    };

    tmpl.render(ctx).map_err(generator::render_error)
}

// ── Procedure: Rust ─────────────────────────────────────────

fn render_procedure_rust(
    env: &minijinja::Environment,
    m: &ProcedureModel,
    imports: &[ImportContext],
) -> Result<String, String> {
    let rp = resolve_procedure(m, ExprTarget::Rust)?;
    let struct_name = filters::to_pascal_case(m.name.clone());

    let renames: Vec<(String, String)> = rp
        .inputs
        .iter()
        .map(|f| (f.id.clone(), filters::to_snake_case(f.id.clone())))
        .collect();

    let params = rp
        .inputs
        .iter()
        .map(|f| {
            format!(
                "{}: {}",
                filters::to_snake_case(f.id.clone()),
                rust_param_type(&f.sce_type)
            )
        })
        .collect::<Vec<_>>()
        .join(", ");

    let state_names: Vec<serde_json::Value> = rp
        .states
        .iter()
        .map(|s| serde_json::json!(s.id))
        .collect();

    // Apply variable renames to guard expressions
    let non_final_states: Vec<serde_json::Value> = rp
        .states
        .iter()
        .filter(|s| !s.is_final)
        .map(|s| {
            let transitions: Vec<serde_json::Value> = s
                .transitions
                .iter()
                .map(|tr| {
                    let cond = tr.cond.as_ref().map(|c| rust_rename_vars(c, &renames));
                    serde_json::json!({
                        "target_index": tr.target_index,
                        "has_cond": cond.is_some(),
                        "cond": cond.unwrap_or_default(),
                    })
                })
                .collect();
            serde_json::json!({
                "index": s.index,
                "id": s.id,
                "transitions": transitions,
            })
        })
        .collect();

    let final_indices: Vec<usize> = rp
        .states
        .iter()
        .filter(|s| s.is_final)
        .map(|s| s.index)
        .collect();

    let final_check = final_check_expr(&final_indices, "current", "||");

    let tmpl = env
        .get_template("procedure.rs.jinja2")
        .map_err(|e| format!("Template load error: {e}"))?;

    let (has_imports, all_imports, stateful_imports) = build_template_imports(imports);

    let ctx = minijinja::context! {
        struct_name => struct_name,
        params => params,
        state_names => minijinja::Value::from_serialize(&state_names),
        state_count => rp.state_count,
        initial_index => rp.initial_index,
        non_final_states => minijinja::Value::from_serialize(&non_final_states),
        final_check => final_check,
        has_imports => has_imports,
        imports => stateful_imports,
        all_imports => all_imports,
    };

    tmpl.render(ctx).map_err(generator::render_error)
}

// ── Procedure: Go ───────────────────────────────────────────

fn render_procedure_go(
    env: &minijinja::Environment,
    m: &ProcedureModel,
    imports: &[ImportContext],
) -> Result<String, String> {
    let rp = resolve_procedure(m, ExprTarget::Go)?;
    let package = filters::to_snake_case(m.name.clone());
    let struct_name = filters::to_pascal_case(m.name.clone());

    let go_renames: Vec<(String, String)> = rp
        .inputs
        .iter()
        .map(|f| (f.id.clone(), go_escape_builtin(&f.id)))
        .collect();

    let params = rp
        .inputs
        .iter()
        .map(|f| format!("{} {}", go_escape_builtin(&f.id), go_type(&f.sce_type)))
        .collect::<Vec<_>>()
        .join(", ");

    let state_names: Vec<serde_json::Value> = rp
        .states
        .iter()
        .map(|s| serde_json::json!(s.id))
        .collect();

    // Apply Go variable renames to guard expressions
    let non_final_states: Vec<serde_json::Value> = rp
        .states
        .iter()
        .filter(|s| !s.is_final)
        .map(|s| {
            let transitions: Vec<serde_json::Value> = s
                .transitions
                .iter()
                .map(|tr| {
                    let cond = tr.cond.as_ref().map(|c| {
                        let mut expr = c.clone();
                        for (from, to) in &go_renames {
                            if from != to {
                                let re = regex::Regex::new(&format!(
                                    r"\b{}\b",
                                    regex::escape(from)
                                ))
                                .unwrap();
                                expr = re.replace_all(&expr, to.as_str()).to_string();
                            }
                        }
                        expr
                    });
                    serde_json::json!({
                        "target_index": tr.target_index,
                        "has_cond": cond.is_some(),
                        "cond": cond.unwrap_or_default(),
                    })
                })
                .collect();
            serde_json::json!({
                "index": s.index,
                "id": s.id,
                "transitions": transitions,
            })
        })
        .collect();

    let final_indices: Vec<usize> = rp
        .states
        .iter()
        .filter(|s| s.is_final)
        .map(|s| s.index)
        .collect();

    let final_check = final_check_expr(&final_indices, "current", "||");

    let tmpl = env
        .get_template("procedure.go.jinja2")
        .map_err(|e| format!("Template load error: {e}"))?;

    let (has_imports, all_imports, stateful_imports) = build_template_imports(imports);

    let ctx = minijinja::context! {
        package => package, struct_name => struct_name,
        params => params,
        state_names => minijinja::Value::from_serialize(&state_names),
        state_count => rp.state_count,
        initial_index => rp.initial_index,
        non_final_states => minijinja::Value::from_serialize(&non_final_states),
        final_check => final_check,
        has_imports => has_imports,
        imports => stateful_imports,
        all_imports => all_imports,
    };

    tmpl.render(ctx).map_err(generator::render_error)
}

// ── Procedure: Python ───────────────────────────────────────

fn render_procedure_python(
    env: &minijinja::Environment,
    m: &ProcedureModel,
    imports: &[ImportContext],
) -> Result<String, String> {
    let rp = resolve_procedure(m, ExprTarget::Python)?;
    let struct_name = filters::to_pascal_case(m.name.clone());

    let renames: Vec<(String, String)> = rp
        .inputs
        .iter()
        .map(|f| (f.id.clone(), filters::to_snake_case(f.id.clone())))
        .collect();

    let params = rp
        .inputs
        .iter()
        .map(|f| {
            format!(
                "{}: {}",
                filters::to_snake_case(f.id.clone()),
                python_type(&f.sce_type)
            )
        })
        .collect::<Vec<_>>()
        .join(", ");

    let state_names: Vec<serde_json::Value> = rp
        .states
        .iter()
        .map(|s| serde_json::json!(s.id))
        .collect();

    // Apply variable renames to guard expressions
    let non_final_states: Vec<serde_json::Value> = rp
        .states
        .iter()
        .filter(|s| !s.is_final)
        .map(|s| {
            let transitions: Vec<serde_json::Value> = s
                .transitions
                .iter()
                .map(|tr| {
                    let cond = tr.cond.as_ref().map(|c| rust_rename_vars(c, &renames));
                    serde_json::json!({
                        "target_index": tr.target_index,
                        "has_cond": cond.is_some(),
                        "cond": cond.unwrap_or_default(),
                    })
                })
                .collect();
            serde_json::json!({
                "index": s.index,
                "id": s.id,
                "transitions": transitions,
            })
        })
        .collect();

    let final_indices: Vec<usize> = rp
        .states
        .iter()
        .filter(|s| s.is_final)
        .map(|s| s.index)
        .collect();

    let final_check = final_check_expr(&final_indices, "current", "or");

    let tmpl = env
        .get_template("procedure.py.jinja2")
        .map_err(|e| format!("Template load error: {e}"))?;

    let (has_imports, all_imports, stateful_imports) = build_template_imports(imports);

    let ctx = minijinja::context! {
        struct_name => struct_name,
        params => params,
        state_names => minijinja::Value::from_serialize(&state_names),
        state_count => rp.state_count,
        initial_index => rp.initial_index,
        non_final_states => minijinja::Value::from_serialize(&non_final_states),
        final_check => final_check,
        has_imports => has_imports,
        imports => stateful_imports,
        all_imports => all_imports,
    };

    tmpl.render(ctx).map_err(generator::render_error)
}

// ── Procedure Level 2: shared helpers ───────────────────────────

/// Common L2 procedure data shared across all language renderers.
struct L2Common {
    state_enum: Vec<serde_json::Value>,
    event_enum: Vec<serde_json::Value>,
    event_name_map: std::collections::BTreeMap<String, String>,
    initial_state: String,
    final_states: Vec<serde_json::Value>,
    payload_exprs: Vec<String>,
    has_external_deps: bool,
}

/// Build language-independent L2 procedure data (state/event enums, final states).
fn build_l2_common(m: &ProcedureModel) -> L2Common {
    let state_enum: Vec<serde_json::Value> = m
        .states
        .iter()
        .enumerate()
        .map(|(i, s)| {
            serde_json::json!({
                "name": filters::to_pascal_case(s.id.clone()),
                "index": i,
            })
        })
        .collect();

    let mut event_raw_to_pascal: std::collections::BTreeMap<String, String> =
        std::collections::BTreeMap::new();
    event_raw_to_pascal.insert("ok".to_string(), "Ok".to_string());
    event_raw_to_pascal.insert("fail".to_string(), "Fail".to_string());
    for s in &m.states {
        for tr in &s.transitions {
            if let Some(ev) = &tr.event {
                event_raw_to_pascal
                    .entry(ev.clone())
                    .or_insert_with(|| filters::to_pascal_case(ev.clone()));
            }
        }
    }

    let mut seen_pascal: std::collections::BTreeSet<String> = std::collections::BTreeSet::new();
    let event_enum: Vec<serde_json::Value> = event_raw_to_pascal
        .iter()
        .filter(|(_, pascal)| seen_pascal.insert((*pascal).clone()))
        .enumerate()
        .map(|(i, (raw, pascal))| {
            serde_json::json!({
                "name": pascal,
                "index": i + 1,
                "event_name": raw,
            })
        })
        .collect();

    let initial_state = filters::to_pascal_case(m.initial.clone());

    let final_states: Vec<serde_json::Value> = m
        .states
        .iter()
        .filter(|s| s.is_final)
        .map(|s| {
            serde_json::json!({
                "name": filters::to_pascal_case(s.id.clone()),
                "id": s.id,
            })
        })
        .collect();

    let payload_exprs: Vec<String> = m
        .states
        .iter()
        .flat_map(|s| s.on_entry_sends.iter())
        .filter_map(|send| send.payload.clone())
        .collect();
    let has_external_deps = !payload_exprs.is_empty();

    L2Common {
        state_enum,
        event_enum,
        event_name_map: event_raw_to_pascal,
        initial_state,
        final_states,
        payload_exprs,
        has_external_deps,
    }
}

/// Build non-final state transition data for L2 templates.
/// `cond_transform` allows Kotlin to apply unsigned conversion to guard expressions.
fn build_l2_non_final_states(
    m: &ProcedureModel,
    target: ExprTarget,
    rename_map: &std::collections::HashMap<&str, &str>,
    event_name_map: &std::collections::BTreeMap<String, String>,
    cond_transform: Option<&dyn Fn(&str) -> String>,
) -> Vec<serde_json::Value> {
    m.states
        .iter()
        .filter(|s| !s.is_final)
        .map(|s| {
            let transitions: Vec<serde_json::Value> = s
                .transitions
                .iter()
                .enumerate()
                .map(|(idx, tr)| {
                    let event_enum_name = tr.event.as_ref().map(|ev| {
                        event_name_map
                            .get(ev)
                            .cloned()
                            .unwrap_or_else(|| filters::to_pascal_case(ev.clone()))
                    });
                    let cond_transpiled = tr.cond.as_ref().map(|c| {
                        let base = transpile_l2_expr(c, target, rename_map);
                        match cond_transform {
                            Some(f) => f(&base),
                            None => base,
                        }
                    });
                    serde_json::json!({
                        "index": idx,
                        "has_event": tr.event.is_some(),
                        "event_name": tr.event.as_deref().unwrap_or(""),
                        "event_enum": event_enum_name.unwrap_or_default(),
                        "has_cond": tr.cond.is_some(),
                        "cond": cond_transpiled.unwrap_or_default(),
                        "target_name": filters::to_pascal_case(tr.target.clone()),
                        "has_assigns": !tr.assigns.is_empty(),
                    })
                })
                .collect();
            serde_json::json!({
                "name": filters::to_pascal_case(s.id.clone()),
                "transitions": transitions,
            })
        })
        .collect()
}

/// Build states with onentry sends for L2 templates.
/// `payload_rename_map` allows Rust to borrow non-Copy types in payload expressions.
fn build_l2_states_with_entry(
    m: &ProcedureModel,
    target: ExprTarget,
    rename_map: &std::collections::HashMap<&str, &str>,
    payload_rename_map: Option<&std::collections::HashMap<&str, &str>>,
) -> Vec<serde_json::Value> {
    let payload_map = payload_rename_map.unwrap_or(rename_map);
    m.states
        .iter()
        .filter(|s| !s.on_entry_sends.is_empty())
        .map(|s| {
            let sends: Vec<serde_json::Value> = s
                .on_entry_sends
                .iter()
                .map(|send| {
                    let addr_expr = send
                        .addr
                        .as_ref()
                        .map(|a| transpile_l2_expr(a, target, rename_map));
                    let payload_expr = send
                        .payload
                        .as_ref()
                        .map(|p| transpile_l2_expr(p, target, payload_map));
                    serde_json::json!({
                        "service": send.service,
                        "subfunc": send.subfunc,
                        "has_addr": send.addr.is_some(),
                        "addr_expr": addr_expr.unwrap_or_default(),
                        "payload": send.payload.is_some(),
                        "payload_expr": payload_expr.unwrap_or_default(),
                    })
                })
                .collect();
            serde_json::json!({
                "name": filters::to_pascal_case(s.id.clone()),
                "sends": sends,
            })
        })
        .collect()
}

/// Build final states with donedata for L2 templates.
fn build_l2_final_states_with_donedata(
    m: &ProcedureModel,
    target: ExprTarget,
    rename_map: &std::collections::HashMap<&str, &str>,
) -> Vec<serde_json::Value> {
    m.states
        .iter()
        .filter(|s| s.is_final && !s.done_params.is_empty())
        .map(|s| {
            let done_params: Vec<serde_json::Value> = s
                .done_params
                .iter()
                .map(|p| {
                    let transpiled = transpile_l2_expr(&p.expr, target, rename_map);
                    serde_json::json!({
                        "name": p.name,
                        "expr": transpiled,
                    })
                })
                .collect();
            serde_json::json!({
                "name": filters::to_pascal_case(s.id.clone()),
                "done_params": done_params,
            })
        })
        .collect()
}

/// Build states that have transitions with assigns for L2 templates.
fn build_l2_states_with_assigns(
    m: &ProcedureModel,
    target: ExprTarget,
    assign_rename_map: &std::collections::HashMap<&str, &str>,
    type_map: &std::collections::HashMap<&str, &SceType>,
    location_transform: impl Fn(&str) -> String,
    bytes_wrap: impl Fn(&str, &str) -> String,
) -> Vec<serde_json::Value> {
    m.states
        .iter()
        .filter(|s| s.transitions.iter().any(|tr| !tr.assigns.is_empty()))
        .map(|s| {
            let assign_transitions: Vec<serde_json::Value> = s
                .transitions
                .iter()
                .enumerate()
                .filter(|(_, tr)| !tr.assigns.is_empty())
                .map(|(idx, tr)| {
                    let assigns: Vec<serde_json::Value> = tr
                        .assigns
                        .iter()
                        .map(|a| {
                            let transpiled =
                                transpile_l2_expr(&a.expr, target, assign_rename_map);
                            let wrapped = match type_map.get(a.location.as_str()) {
                                Some(SceType::Bytes) if a.expr.trim() == "_event.data" => {
                                    bytes_wrap(&transpiled, &a.location)
                                }
                                _ => transpiled,
                            };
                            serde_json::json!({
                                "location": location_transform(&a.location),
                                "expr": wrapped,
                            })
                        })
                        .collect();
                    serde_json::json!({
                        "index": idx,
                        "assigns": assigns,
                    })
                })
                .collect();
            serde_json::json!({
                "name": filters::to_pascal_case(s.id.clone()),
                "assign_transitions": assign_transitions,
            })
        })
        .collect()
}

/// Build the type map (variable name → SceType) for assign type checking.
fn build_l2_type_map<'a>(m: &'a ProcedureModel) -> std::collections::HashMap<&'a str, &'a SceType> {
    m.inputs
        .iter()
        .chain(m.internals.iter())
        .map(|f| (f.id.as_str(), &f.sce_type))
        .collect()
}

/// Default zero-value for Kotlin types.
fn kotlin_default(ty: &SceType) -> &'static str {
    match ty {
        SceType::Uint8 => "0.toUByte()",
        SceType::Uint16 => "0.toUShort()",
        SceType::Uint32 => "0u",
        SceType::Uint64 => "0uL",
        SceType::Int8 | SceType::Int16 | SceType::Int32 => "0",
        SceType::Int64 => "0L",
        SceType::Float32 => "0.0f",
        SceType::Float64 => "0.0",
        SceType::Bool => "false",
        SceType::String => "\"\"",
        SceType::Bytes => "byteArrayOf()",
    }
}

/// Default zero-value for Rust types.
fn rust_default(ty: &SceType) -> &'static str {
    match ty {
        SceType::Uint8 | SceType::Uint16 | SceType::Uint32 | SceType::Uint64 => "0",
        SceType::Int8 | SceType::Int16 | SceType::Int32 | SceType::Int64 => "0",
        SceType::Float32 | SceType::Float64 => "0.0",
        SceType::Bool => "false",
        SceType::String => "String::new()",
        SceType::Bytes => "Vec::new()",
    }
}

/// Default zero-value for Python types.
fn python_default(ty: &SceType) -> &'static str {
    match ty {
        SceType::Uint8 | SceType::Uint16 | SceType::Uint32 | SceType::Uint64 => "0",
        SceType::Int8 | SceType::Int16 | SceType::Int32 | SceType::Int64 => "0",
        SceType::Float32 | SceType::Float64 => "0.0",
        SceType::Bool => "False",
        SceType::String => "\"\"",
        SceType::Bytes => "b\"\"",
    }
}

// ── Procedure Level 2: Kotlin ───────────────────────────────────

fn render_procedure_l2_kotlin(
    env: &minijinja::Environment,
    m: &ProcedureModel,
    imports: &[ImportContext],
) -> Result<String, String> {
    let pascal = filters::to_pascal_case(m.name.clone());
    let package = filters::to_snake_case(m.name.clone());
    let common = build_l2_common(m);

    // Input fields
    let input_fields: Vec<serde_json::Value> = m
        .inputs
        .iter()
        .map(|f| {
            serde_json::json!({
                "id": f.id,
                "kt_type": kotlin_type(&f.sce_type),
                "setter_name": filters::to_pascal_case(f.id.clone()),
                "default_value": kotlin_default(&f.sce_type),
            })
        })
        .collect();

    // Internal fields
    let internal_fields: Vec<serde_json::Value> = m
        .internals
        .iter()
        .map(|f| {
            let default_val = f
                .expr
                .as_ref()
                .map(|e| expr::transpile(e, ExprTarget::Kotlin).unwrap_or_else(|_| e.clone()))
                .unwrap_or_else(|| kotlin_default(&f.sce_type).to_string());
            serde_json::json!({
                "id": f.id,
                "kt_type": kotlin_type(&f.sce_type),
                "default_value": default_val,
            })
        })
        .collect();

    // Rename map: Kotlin only renames _event.data → pendingEventData
    let owned_rename: std::collections::HashMap<&str, String> =
        std::collections::HashMap::from([("_event.data", "pendingEventData".to_string())]);
    let rename_map: std::collections::HashMap<&str, &str> = owned_rename
        .iter()
        .map(|(k, v)| (*k, v.as_str()))
        .collect();

    // Build assign rename map (same as rename_map for Kotlin)
    let assign_rename_map = rename_map.clone();

    let type_map = build_l2_type_map(m);
    let states_with_entry = build_l2_states_with_entry(m, ExprTarget::Kotlin, &rename_map, None);
    let final_states_with_donedata =
        build_l2_final_states_with_donedata(m, ExprTarget::Kotlin, &rename_map);

    // Non-final states: apply Kotlin unsigned conversion to guards if needed
    let has_unsigned = m
        .inputs
        .iter()
        .chain(m.internals.iter())
        .any(|f| f.sce_type.is_unsigned());
    let unsigned_conversions: Vec<(String, String)> = m
        .inputs
        .iter()
        .chain(m.internals.iter())
        .filter_map(|f| {
            kotlin_unsigned_conversion(&f.sce_type).map(|cv| (f.id.clone(), cv.to_string()))
        })
        .collect();

    let kt_cond_transform = |base: &str| -> String {
        if has_unsigned && kotlin_condition_needs_conversion(base) {
            kotlin_wrap_expr(base, &unsigned_conversions)
        } else {
            base.to_string()
        }
    };
    let non_final_states = build_l2_non_final_states(
        m,
        ExprTarget::Kotlin,
        &rename_map,
        &common.event_name_map,
        Some(&kt_cond_transform),
    );

    let states_with_assigns = build_l2_states_with_assigns(
        m,
        ExprTarget::Kotlin,
        &assign_rename_map,
        &type_map,
        |loc| loc.to_string(),
        |transpiled, _| format!("{transpiled}.toByteArray()"),
    );

    let tmpl = env
        .get_template("procedure_l2.kt.jinja2")
        .map_err(|e| format!("Template load error: {e}"))?;

    // Cross-file imports
    let (has_imports, all_imports, stateful_imports) = build_template_imports(imports);

    let ctx = minijinja::context! {
        package => package,
        class_name => &pascal,
        pascal_name => &pascal,
        state_enum => minijinja::Value::from_serialize(&common.state_enum),
        event_enum => minijinja::Value::from_serialize(&common.event_enum),
        input_fields => minijinja::Value::from_serialize(&input_fields),
        internal_fields => minijinja::Value::from_serialize(&internal_fields),
        initial_state => common.initial_state,
        final_states => minijinja::Value::from_serialize(&common.final_states),
        states_with_entry => minijinja::Value::from_serialize(&states_with_entry),
        final_states_with_donedata => minijinja::Value::from_serialize(&final_states_with_donedata),
        non_final_states => minijinja::Value::from_serialize(&non_final_states),
        states_with_assigns => minijinja::Value::from_serialize(&states_with_assigns),
        has_external_deps => common.has_external_deps,
        payload_exprs => minijinja::Value::from_serialize(&common.payload_exprs),
        has_imports => has_imports,
        imports => stateful_imports,
        all_imports => all_imports,
    };

    tmpl.render(ctx).map_err(generator::render_error)
}

// ── Procedure Level 2: Rust ─────────────────────────────────────

fn render_procedure_l2_rust(
    env: &minijinja::Environment,
    m: &ProcedureModel,
    imports: &[ImportContext],
) -> Result<String, String> {
    let pascal = filters::to_pascal_case(m.name.clone());
    let snake = filters::to_snake_case(m.name.clone());
    let common = build_l2_common(m);

    // Build rename map: varName → self.var_name
    let var_name_strings: Vec<String> = m
        .inputs
        .iter()
        .chain(m.internals.iter())
        .map(|f| f.id.clone())
        .collect();
    let owned_rename: std::collections::HashMap<&str, String> = var_name_strings
        .iter()
        .map(|name| {
            (
                name.as_str(),
                format!("self.{}", filters::to_snake_case(name.clone())),
            )
        })
        .collect();
    // Add import alias renames: `frame` → `self.frame` for Rust member access
    let mut owned_rename_with_event = owned_rename;
    for imp in imports {
        if imp.is_stateful {
            owned_rename_with_event
                .insert(&imp.alias, format!("self.{}", imp.member_name));
        }
    }
    owned_rename_with_event.insert("_event.data", "self.pending_event_data".to_string());
    let rename_map: std::collections::HashMap<&str, &str> = owned_rename_with_event
        .iter()
        .map(|(k, v)| (*k, v.as_str()))
        .collect();
    let assign_rename_map = rename_map.clone();

    // Input fields
    let input_fields: Vec<serde_json::Value> = m
        .inputs
        .iter()
        .map(|f| {
            let snake_id = filters::to_snake_case(f.id.clone());
            let (setter_conv, rs_param_type) = match f.sce_type {
                SceType::String => ("value.to_string()".to_string(), "&str".to_string()),
                SceType::Bytes => ("value.to_vec()".to_string(), "&[u8]".to_string()),
                _ => ("value".to_string(), rust_type(&f.sce_type).to_string()),
            };
            serde_json::json!({
                "id": snake_id,
                "rs_type": rust_type(&f.sce_type),
                "rs_param_type": rs_param_type,
                "setter_name": snake_id,
                "setter_conv": setter_conv,
                "param_name": snake_id,
                "default_value": rust_default(&f.sce_type),
            })
        })
        .collect();

    // Internal fields
    let internal_fields: Vec<serde_json::Value> = m
        .internals
        .iter()
        .map(|f| {
            let snake_id = filters::to_snake_case(f.id.clone());
            let default_val = f
                .expr
                .as_ref()
                .map(|e| expr::transpile(e, ExprTarget::Rust).unwrap_or_else(|_| e.clone()))
                .unwrap_or_else(|| rust_default(&f.sce_type).to_string());
            serde_json::json!({
                "id": snake_id,
                "rs_type": rust_type(&f.sce_type),
                "default_value": default_val,
            })
        })
        .collect();

    let type_map = build_l2_type_map(m);

    // Payload rename map: borrow Bytes/String fields to prevent move in fn args.
    // e.g., computeKey(self.seed) → computeKey(&self.seed) for Vec<u8> fields.
    let mut owned_payload_rename: std::collections::HashMap<&str, String> = var_name_strings
        .iter()
        .map(|name| {
            let snake = filters::to_snake_case(name.clone());
            let ty = type_map.get(name.as_str());
            let value = match ty {
                Some(SceType::Bytes) | Some(SceType::String) => format!("&self.{}", snake),
                _ => format!("self.{}", snake),
            };
            (name.as_str(), value)
        })
        .chain(std::iter::once(("_event.data", "self.pending_event_data".to_string())))
        .collect();
    // Add import alias renames to payload map
    for imp in imports {
        if imp.is_stateful {
            owned_payload_rename
                .insert(&imp.alias, format!("self.{}", imp.member_name));
        }
    }
    let payload_rename_map: std::collections::HashMap<&str, &str> = owned_payload_rename
        .iter()
        .map(|(k, v)| (*k, v.as_str()))
        .collect();

    let states_with_entry =
        build_l2_states_with_entry(m, ExprTarget::Rust, &rename_map, Some(&payload_rename_map));
    let final_states_with_donedata =
        build_l2_final_states_with_donedata(m, ExprTarget::Rust, &rename_map);
    let non_final_states = build_l2_non_final_states(
        m,
        ExprTarget::Rust,
        &rename_map,
        &common.event_name_map,
        None,
    );
    let states_with_assigns = build_l2_states_with_assigns(
        m,
        ExprTarget::Rust,
        &assign_rename_map,
        &type_map,
        |loc| format!("self.{}", filters::to_snake_case(loc.to_string())),
        |transpiled, _| format!("{transpiled}.as_bytes().to_vec()"),
    );

    let tmpl = env
        .get_template("procedure_l2.rs.jinja2")
        .map_err(|e| format!("Template load error: {e}"))?;

    // Cross-file imports
    let (has_imports, all_imports, stateful_imports) = build_template_imports(imports);

    let ctx = minijinja::context! {
        struct_name => &pascal,
        snake_name => snake,
        state_enum => minijinja::Value::from_serialize(&common.state_enum),
        event_enum => minijinja::Value::from_serialize(&common.event_enum),
        input_fields => minijinja::Value::from_serialize(&input_fields),
        internal_fields => minijinja::Value::from_serialize(&internal_fields),
        initial_state => common.initial_state,
        final_states => minijinja::Value::from_serialize(&common.final_states),
        states_with_entry => minijinja::Value::from_serialize(&states_with_entry),
        final_states_with_donedata => minijinja::Value::from_serialize(&final_states_with_donedata),
        non_final_states => minijinja::Value::from_serialize(&non_final_states),
        states_with_assigns => minijinja::Value::from_serialize(&states_with_assigns),
        has_external_deps => common.has_external_deps,
        payload_exprs => minijinja::Value::from_serialize(&common.payload_exprs),
        has_imports => has_imports,
        imports => stateful_imports,
        all_imports => all_imports,
    };

    tmpl.render(ctx).map_err(generator::render_error)
}

// ── Procedure Level 2: Go ───────────────────────────────────────

fn render_procedure_l2_go(
    env: &minijinja::Environment,
    m: &ProcedureModel,
    imports: &[ImportContext],
) -> Result<String, String> {
    let pascal = filters::to_pascal_case(m.name.clone());
    let package = filters::to_snake_case(m.name.clone());
    let common = build_l2_common(m);

    // Build rename map: varName → p.varName (Go struct field access)
    let var_name_strings: Vec<String> = m
        .inputs
        .iter()
        .chain(m.internals.iter())
        .map(|f| f.id.clone())
        .collect();
    let owned_rename: std::collections::HashMap<&str, String> = var_name_strings
        .iter()
        .map(|name| {
            (
                name.as_str(),
                format!("p.{}", go_escape_builtin(name)),
            )
        })
        .collect();
    // Add import alias renames: `frame` → `p.Frame` for Go struct field access
    let mut owned_rename_with_event = owned_rename;
    for imp in imports {
        if imp.is_stateful {
            owned_rename_with_event
                .insert(&imp.alias, format!("p.{}", imp.member_name));
        }
    }
    owned_rename_with_event.insert("_event.data", "p.pendingEventData".to_string());
    let rename_map: std::collections::HashMap<&str, &str> = owned_rename_with_event
        .iter()
        .map(|(k, v)| (*k, v.as_str()))
        .collect();
    let assign_rename_map = rename_map.clone();

    // Determine if fmt import is needed (for addr string conversion)
    let needs_fmt = m
        .states
        .iter()
        .flat_map(|s| s.on_entry_sends.iter())
        .any(|send| send.addr.is_some() || send.payload.is_some());

    // Input fields
    let input_fields: Vec<serde_json::Value> = m
        .inputs
        .iter()
        .map(|f| {
            let go_id = go_escape_builtin(&f.id);
            serde_json::json!({
                "id": go_id,
                "raw_id": f.id,
                "go_type": go_type(&f.sce_type),
                "setter_name": filters::to_pascal_case(f.id.clone()),
                "param_id": go_id,
            })
        })
        .collect();

    // Internal fields
    let internal_fields: Vec<serde_json::Value> = m
        .internals
        .iter()
        .map(|f| {
            let go_id = go_escape_builtin(&f.id);
            let default_val = f
                .expr
                .as_ref()
                .map(|e| expr::transpile(e, ExprTarget::Go).unwrap_or_else(|_| e.clone()));
            serde_json::json!({
                "id": go_id,
                "go_type": go_type(&f.sce_type),
                "has_default": default_val.is_some(),
                "default_value": default_val.unwrap_or_default(),
            })
        })
        .collect();

    let type_map = build_l2_type_map(m);
    let states_with_entry = build_l2_states_with_entry(m, ExprTarget::Go, &rename_map, None);
    let final_states_with_donedata =
        build_l2_final_states_with_donedata(m, ExprTarget::Go, &rename_map);
    let non_final_states = build_l2_non_final_states(
        m,
        ExprTarget::Go,
        &rename_map,
        &common.event_name_map,
        None,
    );
    let states_with_assigns = build_l2_states_with_assigns(
        m,
        ExprTarget::Go,
        &assign_rename_map,
        &type_map,
        |loc| format!("p.{}", go_escape_builtin(loc)),
        |transpiled, _| format!("[]byte({transpiled})"),
    );

    let tmpl = env
        .get_template("procedure_l2.go.jinja2")
        .map_err(|e| format!("Template load error: {e}"))?;

    // Cross-file imports
    let (has_imports, all_imports, stateful_imports) = build_template_imports(imports);

    let ctx = minijinja::context! {
        package => package,
        class_name => &pascal,
        pascal_name => &pascal,
        needs_fmt => needs_fmt,
        state_enum => minijinja::Value::from_serialize(&common.state_enum),
        event_enum => minijinja::Value::from_serialize(&common.event_enum),
        input_fields => minijinja::Value::from_serialize(&input_fields),
        internal_fields => minijinja::Value::from_serialize(&internal_fields),
        initial_state => common.initial_state,
        final_states => minijinja::Value::from_serialize(&common.final_states),
        states_with_entry => minijinja::Value::from_serialize(&states_with_entry),
        final_states_with_donedata => minijinja::Value::from_serialize(&final_states_with_donedata),
        non_final_states => minijinja::Value::from_serialize(&non_final_states),
        states_with_assigns => minijinja::Value::from_serialize(&states_with_assigns),
        has_external_deps => common.has_external_deps,
        payload_exprs => minijinja::Value::from_serialize(&common.payload_exprs),
        has_imports => has_imports,
        imports => stateful_imports,
        all_imports => all_imports,
    };

    tmpl.render(ctx).map_err(generator::render_error)
}

// ── Procedure Level 2: Python ───────────────────────────────────

fn render_procedure_l2_python(
    env: &minijinja::Environment,
    m: &ProcedureModel,
    imports: &[ImportContext],
) -> Result<String, String> {
    let pascal = filters::to_pascal_case(m.name.clone());
    let snake = filters::to_snake_case(m.name.clone());
    let common = build_l2_common(m);

    // Build rename map: varName → self._var_name
    let var_name_strings: Vec<String> = m
        .inputs
        .iter()
        .chain(m.internals.iter())
        .map(|f| f.id.clone())
        .collect();
    let owned_rename: std::collections::HashMap<&str, String> = var_name_strings
        .iter()
        .map(|name| {
            (
                name.as_str(),
                format!("self._{}", filters::to_snake_case(name.clone())),
            )
        })
        .collect();
    // Add import alias renames: `frame` → `self.frame` for Python member access
    let mut owned_rename_with_event = owned_rename;
    for imp in imports {
        if imp.is_stateful {
            owned_rename_with_event
                .insert(&imp.alias, format!("self.{}", imp.member_name));
        }
    }
    owned_rename_with_event.insert("_event.data", "self._pending_event_data".to_string());
    let rename_map: std::collections::HashMap<&str, &str> = owned_rename_with_event
        .iter()
        .map(|(k, v)| (*k, v.as_str()))
        .collect();
    let assign_rename_map = rename_map.clone();

    // Input fields
    let input_fields: Vec<serde_json::Value> = m
        .inputs
        .iter()
        .map(|f| {
            let snake_id = filters::to_snake_case(f.id.clone());
            serde_json::json!({
                "snake_id": snake_id,
                "py_type": python_type(&f.sce_type),
                "default_value": python_default(&f.sce_type),
            })
        })
        .collect();

    // Internal fields
    let internal_fields: Vec<serde_json::Value> = m
        .internals
        .iter()
        .map(|f| {
            let snake_id = filters::to_snake_case(f.id.clone());
            let default_val = f
                .expr
                .as_ref()
                .map(|e| expr::transpile(e, ExprTarget::Python).unwrap_or_else(|_| e.clone()))
                .unwrap_or_else(|| python_default(&f.sce_type).to_string());
            serde_json::json!({
                "snake_id": snake_id,
                "py_type": python_type(&f.sce_type),
                "default_value": default_val,
            })
        })
        .collect();

    let type_map = build_l2_type_map(m);
    let states_with_entry = build_l2_states_with_entry(m, ExprTarget::Python, &rename_map, None);
    let final_states_with_donedata =
        build_l2_final_states_with_donedata(m, ExprTarget::Python, &rename_map);
    let non_final_states = build_l2_non_final_states(
        m,
        ExprTarget::Python,
        &rename_map,
        &common.event_name_map,
        None,
    );
    let states_with_assigns = build_l2_states_with_assigns(
        m,
        ExprTarget::Python,
        &assign_rename_map,
        &type_map,
        |loc| format!("self._{}", filters::to_snake_case(loc.to_string())),
        |transpiled, _| format!("{transpiled}.encode()"),
    );

    let tmpl = env
        .get_template("procedure_l2.py.jinja2")
        .map_err(|e| format!("Template load error: {e}"))?;

    // Cross-file imports
    let (has_imports, all_imports, stateful_imports) = build_template_imports(imports);

    let ctx = minijinja::context! {
        class_name => &pascal,
        snake_name => snake,
        state_enum => minijinja::Value::from_serialize(&common.state_enum),
        event_enum => minijinja::Value::from_serialize(&common.event_enum),
        input_fields => minijinja::Value::from_serialize(&input_fields),
        internal_fields => minijinja::Value::from_serialize(&internal_fields),
        initial_state => common.initial_state,
        final_states => minijinja::Value::from_serialize(&common.final_states),
        states_with_entry => minijinja::Value::from_serialize(&states_with_entry),
        final_states_with_donedata => minijinja::Value::from_serialize(&final_states_with_donedata),
        non_final_states => minijinja::Value::from_serialize(&non_final_states),
        states_with_assigns => minijinja::Value::from_serialize(&states_with_assigns),
        has_external_deps => common.has_external_deps,
        payload_exprs => minijinja::Value::from_serialize(&common.payload_exprs),
        has_imports => has_imports,
        imports => stateful_imports,
        all_imports => all_imports,
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
