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
