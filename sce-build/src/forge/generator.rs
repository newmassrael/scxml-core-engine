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

    /// For stateless kinds: parameter types in positional order. For condition
    /// imports the parameters are the model's inputs; for transform imports
    /// they are the inputs; for lookup imports, a single input.
    ///
    /// Populated by `validate_and_enrich_imports` from the parsed imported
    /// ForgeDocument. Consumed by `forge::type_ctx` builders when constructing
    /// the TypeCtx for a kind that imports this alias — the inferred function
    /// signature flows into `TypeCtx::funcs`.
    ///
    /// Empty for stateful kinds and for stateless kinds before enrichment.
    /// Skipped in serialization — templates do not read this field.
    #[serde(skip)]
    pub param_types: Vec<SceType>,

    /// For stateless kinds: return type of the imported function. Transform →
    /// first output type (or `None` for multi-output); Condition → `Bool`;
    /// Lookup → output type. `None` for stateful kinds or unresolved imports.
    #[serde(skip)]
    pub ret_type: Option<SceType>,

    /// For stateful kinds: member fields exposed to user expressions as
    /// `alias_.field_name` (or equivalent member access syntax). Each entry
    /// maps a field name (as seen in the user's SCXML expression) to its
    /// concrete SceType. Empty for stateless kinds and unresolved imports.
    ///
    /// Populated by `validate_and_enrich_imports` from the imported
    /// ForgeDocument's kind-specific field list (e.g. `CodecModel.fields`,
    /// `ValidatorModel.inputs`, `FilterModel.output` + `FilterModel.input`).
    #[serde(skip)]
    pub member_field_types: Vec<(String, SceType)>,
}

/// Resolve a list of `ForgeImport` into template-ready `ImportContext`.
///
/// Uses `options` to pick up language-specific knobs (today only
/// `go_module_prefix`). Returns `Err` when an invariant required by the
/// emitter is missing or when a supplied option has an invalid shape —
/// see `validate_options` for the full rule set. Other languages
/// currently ignore `options`.
pub(crate) fn resolve_imports(
    imports: &[ForgeImport],
    lang: &crate::generator::Language,
    options: &crate::ForgeCompileOptions,
) -> Result<Vec<ImportContext>, String> {
    validate_options(imports, lang, options)?;
    Ok(imports
        .iter()
        .map(|imp| resolve_single_import(imp, lang, options))
        .collect())
}

/// Single source of truth for normalizing `go_module_prefix`. Strips
/// the trailing `/` (harmless duplication in user input like
/// `"github.com/acme/generated/"`) and returns the canonical form. Both
/// the validator and the Go emitter go through this helper so the trim
/// rule is expressed exactly once.
fn normalized_go_prefix(options: &crate::ForgeCompileOptions) -> Option<&str> {
    options
        .go_module_prefix
        .as_deref()
        .map(|p| p.trim_end_matches('/'))
}

/// Validate `options` against the per-language invariants the emitter
/// relies on. Keeps all option-rejection logic in one place so the
/// `resolve_single_import` arms can treat their inputs as already-sane.
fn validate_options(
    imports: &[ForgeImport],
    lang: &crate::generator::Language,
    options: &crate::ForgeCompileOptions,
) -> Result<(), String> {
    if matches!(lang, crate::generator::Language::Go) && !imports.is_empty() {
        match normalized_go_prefix(options) {
            None => {
                return Err(
                    "<sce:import> with language=go requires \
                     ForgeCompileOptions.go_module_prefix. Go module-qualified \
                     imports have no valid bare form; set this field to the \
                     go.mod module path that hosts the generated packages \
                     (e.g. \"github.com/acme/project/generated\")."
                        .to_string(),
                );
            }
            Some(trimmed) if trimmed.is_empty() => {
                return Err(
                    "ForgeCompileOptions.go_module_prefix is empty; \
                     supply a non-empty Go module path such as \
                     \"github.com/acme/project/generated\"."
                        .to_string(),
                );
            }
            Some(trimmed) if trimmed.chars().any(char::is_whitespace) => {
                let raw = options.go_module_prefix.as_deref().unwrap_or("");
                return Err(format!(
                    "ForgeCompileOptions.go_module_prefix {raw:?} \
                     contains whitespace; Go import paths may not \
                     contain spaces or tabs."
                ));
            }
            Some(_) => {}
        }
    }
    Ok(())
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
    options: &crate::ForgeCompileOptions,
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
                param_types: Vec::new(),
                ret_type: None,
                member_field_types: Vec::new(),
            }
        }
        crate::generator::Language::Kotlin => {
            // Every imported kind lives in its own sibling package
            // (`com.sce.generated.<snake>`), so both stateful and stateless
            // imports need an explicit import statement — a wildcard import
            // brings the class name (for stateful) or free functions (for
            // stateless) into unqualified scope. The earlier "stateful imports
            // assume same package" assumption silently produced uncompilable
            // Kotlin goldens because the generated procedure file referenced
            // the imported class by bare name with no import in scope.
            let include_stmt = format!("import com.sce.generated.{snake}.*");
            ImportContext {
                alias: imp.alias.clone(),
                kind: imp.kind.to_string(),
                include_stmt,
                type_name: pascal.clone(),
                is_stateful,
                member_name: imp.alias.clone(),
                member_type: pascal.clone(),
                namespace: pascal.clone(),
                qualified_call: String::new(),
                param_types: Vec::new(),
                ret_type: None,
                member_field_types: Vec::new(),
            }
        }
        crate::generator::Language::Rust => {
            // Stateful kinds generate a Pascal-named struct — import the type
            // directly so the `<alias>: PascalType` member declaration resolves.
            // Stateless kinds generate free functions (`pub fn compute_*`) with
            // no type wrapper; importing `use super::snake::Pascal;` would pull
            // in a non-existent symbol. Import the module path instead so the
            // `build_qualified_call` output `snake::compute_*(...)` resolves.
            let include_stmt = if is_stateful {
                format!("use super::{snake}::{pascal};")
            } else {
                format!("use super::{snake};")
            };
            ImportContext {
                alias: imp.alias.clone(),
                kind: imp.kind.to_string(),
                include_stmt,
                type_name: pascal.clone(),
                is_stateful,
                member_name: imp.alias.clone(),
                member_type: pascal.clone(),
                namespace: snake.clone(),
                qualified_call: String::new(),
                param_types: Vec::new(),
                ret_type: None,
                member_field_types: Vec::new(),
            }
        }
        crate::generator::Language::Go => {
            // `resolve_imports` rejects Go imports without a module
            // prefix up front, so reaching this branch with `None` is an
            // internal invariant violation — unwrap with an explicit
            // message so the panic carries the bug's location rather
            // than an opaque `Option::unwrap` trace.
            let prefix = normalized_go_prefix(options)
                .expect("resolve_imports must validate go_module_prefix before reaching Go arm");
            let import_path = format!("{prefix}/{snake}");
            let go_pascal = filters::to_pascal_case(imp.alias.to_string());
            ImportContext {
                alias: imp.alias.clone(),
                kind: imp.kind.to_string(),
                include_stmt: format!("\t\"{import_path}\""),
                type_name: pascal.clone(),
                is_stateful,
                member_name: go_pascal,
                member_type: format!("{snake}.{pascal}"),
                namespace: snake.clone(),
                qualified_call: String::new(),
                param_types: Vec::new(),
                ret_type: None,
                member_field_types: Vec::new(),
            }
        }
        crate::generator::Language::Python => {
            // Stateful kinds expose a dataclass — a `from .snake import Pascal`
            // brings the class name into scope for the `self.alias: Pascal =
            // Pascal()` member declaration. Stateless kinds only emit free
            // functions; the Pascal name has no class. Import the module
            // instead so the `build_qualified_call` output `snake.func(...)`
            // resolves at the call site.
            let include_stmt = if is_stateful {
                format!("from .{snake} import {pascal}")
            } else {
                format!("from . import {snake}")
            };
            ImportContext {
                alias: imp.alias.clone(),
                kind: imp.kind.to_string(),
                include_stmt,
                type_name: pascal.clone(),
                is_stateful,
                member_name: imp.alias.clone(),
                member_type: pascal.clone(),
                namespace: snake.clone(),
                qualified_call: String::new(),
                param_types: Vec::new(),
                ret_type: None,
                member_field_types: Vec::new(),
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

/// Kotlin conversion method suffix for unsigned-to-signed narrowing, used by
/// non-expression template fields (lookup `when` clauses, validator range
/// bounds). Expression-level coercion is handled by the typed emitter in
/// `forge::expr::emit_kotlin`.
fn kotlin_unsigned_conversion(ty: &SceType) -> Option<&'static str> {
    match ty {
        SceType::Uint8 | SceType::Uint16 => Some("toInt"),
        SceType::Uint32 | SceType::Uint64 => Some("toLong"),
        _ => None,
    }
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

// ── Per-language literal formatters ───────────────────────────
//
// Convert a raw value text from SCXML (e.g. `"100"`, `"0.25"`, `"true"`) into
// a language-correct literal of the requested SceType. Used by lookup const
// arrays where the same fixture must compile in five target languages whose
// literal grammar varies (Kotlin requires `u` suffix for unsigned, Float
// needs `f` suffix; Rust accepts bare numerics in typed array context but
// needs `.0` for float promotion when the source text is an integer).
//
// The text is trusted to already parse as the requested type — the parser
// catches malformed values upstream. These helpers only adapt syntax, not
// semantics.

/// True if `n` is a textual integer (no decimal point or exponent).
fn looks_like_int(n: &str) -> bool {
    !n.contains('.') && !n.contains('e') && !n.contains('E')
}

fn rust_literal(text: &str, ty: &SceType) -> String {
    match ty {
        SceType::Float32 if looks_like_int(text) => format!("{text}.0_f32"),
        SceType::Float64 if looks_like_int(text) => format!("{text}.0"),
        SceType::Float32 => format!("{text}_f32"),
        SceType::Float64 => text.to_string(),
        SceType::Bool | SceType::String => text.to_string(),
        _ => text.to_string(),
    }
}

fn cpp_literal(text: &str, ty: &SceType) -> String {
    match ty {
        SceType::Float32 if looks_like_int(text) => format!("{text}.0f"),
        SceType::Float32 => format!("{text}f"),
        SceType::Float64 if looks_like_int(text) => format!("{text}.0"),
        _ => text.to_string(),
    }
}

fn go_literal(text: &str, ty: &SceType) -> String {
    // Go's untyped constants auto-convert in typed array context, but emit
    // explicit `.0` for float literals to match the cross-language style and
    // keep manual review readable.
    match ty {
        SceType::Float32 | SceType::Float64 if looks_like_int(text) => format!("{text}.0"),
        _ => text.to_string(),
    }
}

fn kotlin_literal(text: &str, ty: &SceType) -> String {
    match ty {
        SceType::Uint8 | SceType::Uint16 | SceType::Uint32 | SceType::Uint64 => {
            // `100u.toUByte()` etc. — `u` suffix marks the literal as unsigned,
            // then narrow to the exact type. Kotlin has no UByte/UShort literal
            // form so the conversion is mandatory.
            let suffix = match ty {
                SceType::Uint8 => "toUByte",
                SceType::Uint16 => "toUShort",
                SceType::Uint32 => "toUInt",
                SceType::Uint64 => "toULong",
                _ => unreachable!(),
            };
            format!("{text}u.{suffix}()")
        }
        SceType::Int8 => format!("({text}).toByte()"),
        SceType::Int16 => format!("({text}).toShort()"),
        SceType::Int64 if looks_like_int(text) => format!("{text}L"),
        SceType::Float32 if looks_like_int(text) => format!("{text}.0f"),
        SceType::Float32 => format!("{text}f"),
        SceType::Float64 if looks_like_int(text) => format!("{text}.0"),
        SceType::String => format!("\"{text}\""),
        _ => text.to_string(),
    }
}

fn python_literal(text: &str, ty: &SceType) -> String {
    match ty {
        SceType::Float32 | SceType::Float64 if looks_like_int(text) => format!("{text}.0"),
        SceType::String => format!("'{text}'"),
        SceType::Bool => {
            // Python uses Title-cased booleans.
            match text {
                "true" => "True".to_string(),
                "false" => "False".to_string(),
                _ => text.to_string(),
            }
        }
        _ => text.to_string(),
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
        ForgeDocument::Procedure(m) => render_procedure_cpp(&env, m, imports)?,
        ForgeDocument::Filter(m) => render_filter(&env, m, imports, crate::generator::Language::Cpp)?,
        ForgeDocument::Interpolation(m) => render_interpolation(&env, m, imports, crate::generator::Language::Cpp)?,
        ForgeDocument::Timer(m) => render_timer(&env, m, imports, crate::generator::Language::Cpp)?,
        ForgeDocument::Observer(m) => render_observer(&env, m, imports, crate::generator::Language::Cpp)?,
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

    let type_ctx = crate::forge::type_ctx::transform(m, imports);
    let empty_renames = std::collections::HashMap::new();

    let functions: Vec<serde_json::Value> = m
        .outputs
        .iter()
        .map(|out| {
            let expected = crate::forge::types::InferredType::from_sce_type(&out.sce_type);
            let expr_cpp = expr::transpile_typed(
                out.expr.as_deref().unwrap_or("0"),
                ExprTarget::Cpp,
                &type_ctx,
                &empty_renames,
                expected,
            )?;
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

    let output_is_string = m.output_is_string();
    let on_miss_error = m.miss_policy.is_error();

    let (entries_by_value, unique_values, default_value) = if output_is_string {
        let ebv: Vec<serde_json::Value> = m
            .entries_by_value()
            .into_iter()
            .map(|(value, keys)| serde_json::json!({"value": value, "keys": keys}))
            .collect();
        let uv = m.unique_values();
        let dv = match &m.miss_policy {
            MissPolicy::Default(s) => s.clone(),
            MissPolicy::Error => String::new(),
        };
        (ebv, uv, dv)
    } else {
        (Vec::new(), Vec::new(), String::new())
    };

    let (keys_literal, values_literal, default_literal) = if !output_is_string {
        let kl: Vec<String> = m
            .entries
            .iter()
            .map(|e| cpp_literal(&e.key, &m.input.sce_type))
            .collect();
        let vl: Vec<String> = m
            .entries
            .iter()
            .map(|e| cpp_literal(&e.value, &m.output.sce_type))
            .collect();
        let dl = match &m.miss_policy {
            MissPolicy::Default(s) => cpp_literal(s, &m.output.sce_type),
            MissPolicy::Error => String::new(),
        };
        (kl, vl, dl)
    } else {
        (Vec::new(), Vec::new(), String::new())
    };

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
        value_type => cpp_param_type(&m.output.sce_type),
        input_id => &m.input.id,
        unique_values => minijinja::Value::from_serialize(&unique_values),
        entries_by_value => minijinja::Value::from_serialize(&entries_by_value),
        default_value => default_value,
        default_literal => default_literal,
        output_is_string => output_is_string,
        on_miss_error => on_miss_error,
        keys_literal => minijinja::Value::from_serialize(&keys_literal),
        values_literal => minijinja::Value::from_serialize(&values_literal),
        n => m.entries.len(),
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

    let type_ctx = crate::forge::type_ctx::condition(m, imports);
    let expr_cpp = expr::transpile_typed(
        &m.expr,
        ExprTarget::Cpp,
        &type_ctx,
        &std::collections::HashMap::new(),
        crate::forge::types::InferredType::Bool,
    )?;

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

impl ResolvedRange {
    /// Canonical (snake_case) form of the rule's identifier, used as the
    /// fragment in error reason strings (`{reason_id}_out_of_range`). Lives
    /// here on the resolved struct rather than at each generator call site
    /// so the cross-language byte-parity invariant — every language emits
    /// the same reason string for the same rule — is enforced in one place.
    fn canonical_reason_id(&self) -> String {
        filters::to_snake_case(self.id.clone())
    }
}

impl ResolvedRoc {
    /// Canonical (snake_case) form of the rule's identifier, used as the
    /// fragment in error reason strings
    /// (`{reason_id}_rate_of_change_exceeded`). See
    /// [`ResolvedRange::canonical_reason_id`] for the cross-language rationale.
    fn canonical_reason_id(&self) -> String {
        filters::to_snake_case(self.id.clone())
    }
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

    // `id` stays in source (target-language) case for the local parameter
    // reference; `reason_id` is the canonical snake_case form derived in one
    // place (`ResolvedRange::canonical_reason_id`) so the cross-language
    // byte-parity invariant on reason strings is enforced at the model edge
    // rather than duplicated across 5 generator arms.
    let range_rules: Vec<serde_json::Value> = rv.ranges.iter()
        .map(|r| serde_json::json!({
            "id": r.id,
            "reason_id": r.canonical_reason_id(),
            "min": r.min, "max": r.max,
            "has_min": r.min.is_some(), "has_max": r.max.is_some(),
        }))
        .collect();

    let roc_rules: Vec<serde_json::Value> = rv.rocs.iter()
        .map(|roc| serde_json::json!({
            "id": roc.id,
            "reason_id": roc.canonical_reason_id(),
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

    let type_ctx = crate::forge::type_ctx::validator(m, imports);
    let plausibility_expr = match &rv.plausibility {
        Some(e) => Some(expr::transpile_typed(
            e,
            ExprTarget::Cpp,
            &type_ctx,
            &import_renames,
            crate::forge::types::InferredType::Bool,
        )?),
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
        ForgeDocument::Procedure(m) => render_procedure_kotlin(&env, m, imports)?,
        ForgeDocument::Filter(m) => render_filter(&env, m, imports, crate::generator::Language::Kotlin)?,
        ForgeDocument::Interpolation(m) => render_interpolation(&env, m, imports, crate::generator::Language::Kotlin)?,
        ForgeDocument::Timer(m) => render_timer(&env, m, imports, crate::generator::Language::Kotlin)?,
        ForgeDocument::Observer(m) => render_observer(&env, m, imports, crate::generator::Language::Kotlin)?,
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

    let type_ctx = crate::forge::type_ctx::transform(m, imports);
    let empty_renames = std::collections::HashMap::new();

    let functions: Vec<serde_json::Value> = m
        .outputs
        .iter()
        .map(|out| {
            let expected = crate::forge::types::InferredType::from_sce_type(&out.sce_type);
            let final_expr = expr::transpile_typed(
                out.expr.as_deref().unwrap_or("0"),
                ExprTarget::Kotlin,
                &type_ctx,
                &empty_renames,
                expected,
            )?;
            let params = m
                .inputs
                .iter()
                .map(|inp| format!("{}: {}", inp.id, kotlin_type(&inp.sce_type)))
                .collect::<Vec<_>>()
                .join(", ");

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

    let output_is_string = m.output_is_string();
    let on_miss_error = m.miss_policy.is_error();

    let (entries_by_value, unique_values, default_value) = if output_is_string {
        let ebv: Vec<serde_json::Value> = m
            .entries_by_value()
            .into_iter()
            .map(|(value, keys)| serde_json::json!({"value": value, "keys": keys}))
            .collect();
        let uv = m.unique_values();
        let dv = match &m.miss_policy {
            MissPolicy::Default(s) => s.clone(),
            MissPolicy::Error => String::new(),
        };
        (ebv, uv, dv)
    } else {
        (Vec::new(), Vec::new(), String::new())
    };

    let (keys_literal, values_literal, default_literal) = if !output_is_string {
        let kl: Vec<String> = m
            .entries
            .iter()
            .map(|e| kotlin_literal(&e.key, &m.input.sce_type))
            .collect();
        let vl: Vec<String> = m
            .entries
            .iter()
            .map(|e| kotlin_literal(&e.value, &m.output.sce_type))
            .collect();
        let dl = match &m.miss_policy {
            MissPolicy::Default(s) => kotlin_literal(s, &m.output.sce_type),
            MissPolicy::Error => String::new(),
        };
        (kl, vl, dl)
    } else {
        (Vec::new(), Vec::new(), String::new())
    };

    let tmpl = env
        .get_template("lookup.kt.jinja2")
        .map_err(|e| format!("Template load error: {e}"))?;

    let (has_imports, all_imports, stateful_imports) = build_template_imports(imports);

    let ctx = minijinja::context! {
        package => package,
        enum_name => enum_name,
        func_name => func_name,
        input_type => kotlin_type(&m.input.sce_type),
        value_type => kotlin_type(&m.output.sce_type),
        input_id => &m.input.id,
        match_suffix => match_suffix,
        unique_values => minijinja::Value::from_serialize(&unique_values),
        entries_by_value => minijinja::Value::from_serialize(&entries_by_value),
        default_value => default_value,
        default_literal => default_literal,
        output_is_string => output_is_string,
        on_miss_error => on_miss_error,
        keys_literal => minijinja::Value::from_serialize(&keys_literal),
        values_literal => minijinja::Value::from_serialize(&values_literal),
        n => m.entries.len(),
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

    let type_ctx = crate::forge::type_ctx::condition(m, imports);
    let expr_kt = expr::transpile_typed(
        &m.expr,
        ExprTarget::Kotlin,
        &type_ctx,
        &std::collections::HashMap::new(),
        crate::forge::types::InferredType::Bool,
    )?;

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
                "kt_default": kotlin_default(&f.sce_type),
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

    // `reason_id` is the canonical reason-string fragment derived from
    // `ResolvedRange::canonical_reason_id` — single source of truth shared
    // across all 5 languages.
    let range_rules: Vec<serde_json::Value> = rv.ranges.iter()
        .map(|r| {
            let conv = kotlin_unsigned_conversion(&r.sce_type).unwrap_or("");
            serde_json::json!({
                "id": r.id,
                "reason_id": r.canonical_reason_id(),
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
                "reason_id": roc.canonical_reason_id(),
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

    let type_ctx = crate::forge::type_ctx::validator(m, imports);
    let plausibility_expr = match &rv.plausibility {
        Some(e) => Some(expr::transpile_typed(
            e,
            ExprTarget::Kotlin,
            &type_ctx,
            &import_renames,
            crate::forge::types::InferredType::Bool,
        )?),
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
        ForgeDocument::Procedure(m) => render_procedure_rust(&env, m, imports)?,
        ForgeDocument::Filter(m) => render_filter(&env, m, imports, crate::generator::Language::Rust)?,
        ForgeDocument::Interpolation(m) => render_interpolation(&env, m, imports, crate::generator::Language::Rust)?,
        ForgeDocument::Timer(m) => render_timer(&env, m, imports, crate::generator::Language::Rust)?,
        ForgeDocument::Observer(m) => render_observer(&env, m, imports, crate::generator::Language::Rust)?,
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
    let type_ctx = crate::forge::type_ctx::transform(m, imports);
    let empty_renames = std::collections::HashMap::new();

    let functions: Vec<serde_json::Value> = m
        .outputs
        .iter()
        .map(|out| {
            let expected = crate::forge::types::InferredType::from_sce_type(&out.sce_type);
            let expr_rs = expr::transpile_typed(
                out.expr.as_deref().unwrap_or("0"),
                ExprTarget::Rust,
                &type_ctx,
                &empty_renames,
                expected,
            )?;

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

    let output_is_string = m.output_is_string();
    let on_miss_error = m.miss_policy.is_error();

    // Per-strategy bindings: only the chosen branch's data is computed; the
    // unused branch passes empty placeholders that the template's
    // {% if output_is_string %} gate never reads.
    let (unique_values, entries_by_value, default_value) = if output_is_string {
        let uv: Vec<String> = m
            .unique_values()
            .into_iter()
            .map(|v| to_rust_variant(&v))
            .collect();
        let ebv: Vec<serde_json::Value> = m
            .entries_by_value()
            .into_iter()
            .map(|(value, keys)| {
                serde_json::json!({
                    "value": to_rust_variant(&value),
                    "keys": keys,
                })
            })
            .collect();
        let dv = match &m.miss_policy {
            MissPolicy::Default(s) => to_rust_variant(s),
            MissPolicy::Error => String::new(),
        };
        (uv, ebv, dv)
    } else {
        (Vec::new(), Vec::new(), String::new())
    };

    let (keys_literal, values_literal, default_literal) = if !output_is_string {
        let kl: Vec<String> = m
            .entries
            .iter()
            .map(|e| rust_literal(&e.key, &m.input.sce_type))
            .collect();
        let vl: Vec<String> = m
            .entries
            .iter()
            .map(|e| rust_literal(&e.value, &m.output.sce_type))
            .collect();
        let dl = match &m.miss_policy {
            MissPolicy::Default(s) => rust_literal(s, &m.output.sce_type),
            MissPolicy::Error => String::new(),
        };
        (kl, vl, dl)
    } else {
        (Vec::new(), Vec::new(), String::new())
    };

    let tmpl = env
        .get_template("lookup.rs.jinja2")
        .map_err(|e| format!("Template load error: {e}"))?;

    let (has_imports, all_imports, stateful_imports) = build_template_imports(imports);

    let ctx = minijinja::context! {
        enum_name => enum_name,
        func_name => func_name,
        input_type => rust_param_type(&m.input.sce_type),
        value_type => rust_param_type(&m.output.sce_type),
        input_id => input_id_snake,
        unique_values => minijinja::Value::from_serialize(&unique_values),
        entries_by_value => minijinja::Value::from_serialize(&entries_by_value),
        default_value => default_value,
        default_literal => default_literal,
        output_is_string => output_is_string,
        on_miss_error => on_miss_error,
        keys_literal => minijinja::Value::from_serialize(&keys_literal),
        values_literal => minijinja::Value::from_serialize(&values_literal),
        n => m.entries.len(),
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

    let type_ctx = crate::forge::type_ctx::condition(m, imports);
    let expr_rs = expr::transpile_typed(
        &m.expr,
        ExprTarget::Rust,
        &type_ctx,
        &std::collections::HashMap::new(),
        crate::forge::types::InferredType::Bool,
    )?;

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

    let encode_exprs: Vec<String> = generate_encode_exprs_rust(&m.fields, m.default_endian);

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
                filters::to_snake_case(field.id.clone())
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
                let name = filters::to_snake_case(field.id.clone());
                parts.push(format!(
                    "((self.{name} & 0x{mask:02X}) << {bit_off})"
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
    let name = filters::to_snake_case(field.id.clone());

    match field.fixed_bits() {
        Some(8) if bit_off == 0 => {
            exprs.push(format!("self.{name}"));
        }
        Some(bits) if bits < 8 || bit_off > 0 => {
            let mask = (1u64 << bits) - 1;
            exprs.push(format!(
                "((self.{name} & 0x{mask:02X}) << {bit_off}) as u8"
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
                    exprs.push(format!("(self.{name} & 0xFF) as u8"));
                } else {
                    exprs.push(format!(
                        "(self.{name} >> {shift} & 0xFF) as u8"
                    ));
                }
            }
        }
        _ => exprs.push(format!("/* encode {name} */")),
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

    // Rust local-var convention is snake_case, which happens to coincide
    // with the canonical reason fragment — but they remain semantically
    // distinct fields so the template renders the same `rule.reason_id`
    // expression as the other 4 languages and `ResolvedRange` stays the
    // single source of truth for reason-string casing.
    let range_rules: Vec<serde_json::Value> = rv.ranges.iter()
        .map(|r| {
            let snake = filters::to_snake_case(r.id.clone());
            serde_json::json!({
                "id": snake,
                "reason_id": r.canonical_reason_id(),
                "min": r.min, "max": r.max,
                "has_min": r.min.is_some(), "has_max": r.max.is_some(),
            })
        })
        .collect();

    let roc_rules: Vec<serde_json::Value> = rv.rocs.iter()
        .map(|roc| {
            let snake = filters::to_snake_case(roc.id.clone());
            serde_json::json!({
                "id": snake.clone(),
                "reason_id": roc.canonical_reason_id(),
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

    let type_ctx = crate::forge::type_ctx::validator(m, imports);
    let plausibility_expr = match &rv.plausibility {
        Some(e) => Some(expr::transpile_typed(
            e,
            ExprTarget::Rust,
            &type_ctx,
            &import_renames,
            crate::forge::types::InferredType::Bool,
        )?),
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
        ForgeDocument::Procedure(m) => render_procedure_go(&env, m, imports)?,
        ForgeDocument::Filter(m) => render_filter(&env, m, imports, crate::generator::Language::Go)?,
        ForgeDocument::Interpolation(m) => render_interpolation(&env, m, imports, crate::generator::Language::Go)?,
        ForgeDocument::Timer(m) => render_timer(&env, m, imports, crate::generator::Language::Go)?,
        ForgeDocument::Observer(m) => render_observer(&env, m, imports, crate::generator::Language::Go)?,
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

    // Rename map: SCXML id → Go-safe parameter name (for builtins like `len`, `new`)
    let go_rename_strings: Vec<(String, String)> = m
        .inputs
        .iter()
        .map(|inp| (inp.id.clone(), go_escape_builtin(&inp.id)))
        .filter(|(f, t)| f != t)
        .collect();
    let go_renames: std::collections::HashMap<&str, &str> = go_rename_strings
        .iter()
        .map(|(f, t)| (f.as_str(), t.as_str()))
        .collect();

    let type_ctx = crate::forge::type_ctx::transform(m, imports);

    let functions: Vec<serde_json::Value> = m
        .outputs
        .iter()
        .map(|out| {
            let expected = crate::forge::types::InferredType::from_sce_type(&out.sce_type);
            let expr_go = expr::transpile_typed(
                out.expr.as_deref().unwrap_or("0"),
                ExprTarget::Go,
                &type_ctx,
                &go_renames,
                expected,
            )?;

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

    let output_is_string = m.output_is_string();
    let on_miss_error = m.miss_policy.is_error();

    let (entries_by_value, unique_values, default_value) = if output_is_string {
        let ebv: Vec<serde_json::Value> = m
            .entries_by_value()
            .into_iter()
            .map(|(value, keys)| serde_json::json!({"value": value, "keys": keys}))
            .collect();
        let uv = m.unique_values();
        let dv = match &m.miss_policy {
            MissPolicy::Default(s) => s.clone(),
            MissPolicy::Error => String::new(),
        };
        (ebv, uv, dv)
    } else {
        (Vec::new(), Vec::new(), String::new())
    };

    let (keys_literal, values_literal, default_literal) = if !output_is_string {
        let kl: Vec<String> = m
            .entries
            .iter()
            .map(|e| go_literal(&e.key, &m.input.sce_type))
            .collect();
        let vl: Vec<String> = m
            .entries
            .iter()
            .map(|e| go_literal(&e.value, &m.output.sce_type))
            .collect();
        let dl = match &m.miss_policy {
            MissPolicy::Default(s) => go_literal(s, &m.output.sce_type),
            MissPolicy::Error => String::new(),
        };
        (kl, vl, dl)
    } else {
        (Vec::new(), Vec::new(), String::new())
    };

    let tmpl = env
        .get_template("lookup.go.jinja2")
        .map_err(|e| format!("Template load error: {e}"))?;

    let (has_imports, all_imports, stateful_imports) = build_template_imports(imports);

    let ctx = minijinja::context! {
        package => package,
        enum_name => enum_name,
        func_name => func_name,
        input_type => go_type(&m.input.sce_type),
        value_type => go_type(&m.output.sce_type),
        input_id => input_id_safe,
        unique_values => minijinja::Value::from_serialize(&unique_values),
        entries_by_value => minijinja::Value::from_serialize(&entries_by_value),
        default_value => default_value,
        default_literal => default_literal,
        output_is_string => output_is_string,
        on_miss_error => on_miss_error,
        keys_literal => minijinja::Value::from_serialize(&keys_literal),
        values_literal => minijinja::Value::from_serialize(&values_literal),
        n => m.entries.len(),
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

    let go_rename_strings: Vec<(String, String)> = m
        .inputs
        .iter()
        .map(|inp| (inp.id.clone(), go_escape_builtin(&inp.id)))
        .filter(|(f, t)| f != t)
        .collect();
    let go_renames: std::collections::HashMap<&str, &str> = go_rename_strings
        .iter()
        .map(|(f, t)| (f.as_str(), t.as_str()))
        .collect();

    let params = m
        .inputs
        .iter()
        .map(|inp| format!("{} {}", go_escape_builtin(&inp.id), go_type(&inp.sce_type)))
        .collect::<Vec<_>>()
        .join(", ");

    let type_ctx = crate::forge::type_ctx::condition(m, imports);
    let expr_go = expr::transpile_typed(
        &m.expr,
        ExprTarget::Go,
        &type_ctx,
        &go_renames,
        crate::forge::types::InferredType::Bool,
    )?;

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

// ── Go: Validator ────────────────────────────────────────────

fn render_validator_go(
    env: &minijinja::Environment,
    m: &ValidatorModel,
    imports: &[ImportContext],
) -> Result<String, String> {
    let rv = resolve_validator(m);
    let package = filters::to_snake_case(m.name.clone());
    let struct_name = filters::to_pascal_case(m.name.clone());

    let go_rename_strings: Vec<(String, String)> = rv.inputs.iter()
        .map(|f| (f.id.clone(), go_escape_builtin(&f.id)))
        .filter(|(f, t)| f != t)
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

    // Go's `id` stays in source case (camelCase, escaped against Go builtins)
    // for the local parameter reference; `reason_id` is the canonical form
    // from `ResolvedRange::canonical_reason_id`. Single source of truth, no
    // per-language drift possible.
    let range_rules: Vec<serde_json::Value> = rv.ranges.iter()
        .map(|r| serde_json::json!({
            "id": go_escape_builtin(&r.id),
            "reason_id": r.canonical_reason_id(),
            "min": r.min, "max": r.max,
            "has_min": r.min.is_some(), "has_max": r.max.is_some(),
        }))
        .collect();

    let roc_rules: Vec<serde_json::Value> = rv.rocs.iter()
        .map(|roc| {
            let safe = go_escape_builtin(&roc.id);
            serde_json::json!({
                "id": safe,
                "reason_id": roc.canonical_reason_id(),
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

    let type_ctx = crate::forge::type_ctx::validator(m, imports);
    let mut combined_renames: std::collections::HashMap<&str, &str> = go_rename_strings
        .iter()
        .map(|(f, t)| (f.as_str(), t.as_str()))
        .collect();
    for (k, v) in &import_renames {
        combined_renames.insert(*k, *v);
    }
    let plausibility_expr = match &rv.plausibility {
        Some(e) => Some(expr::transpile_typed(
            e,
            ExprTarget::Go,
            &type_ctx,
            &combined_renames,
            crate::forge::types::InferredType::Bool,
        )?),
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
        ForgeDocument::Procedure(m) => render_procedure_python(&env, m, imports)?,
        ForgeDocument::Filter(m) => render_filter(&env, m, imports, crate::generator::Language::Python)?,
        ForgeDocument::Interpolation(m) => render_interpolation(&env, m, imports, crate::generator::Language::Python)?,
        ForgeDocument::Timer(m) => render_timer(&env, m, imports, crate::generator::Language::Python)?,
        ForgeDocument::Observer(m) => render_observer(&env, m, imports, crate::generator::Language::Python)?,
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
    let type_ctx = crate::forge::type_ctx::transform(m, imports);
    let empty_renames = std::collections::HashMap::new();

    let functions: Vec<serde_json::Value> = m
        .outputs
        .iter()
        .map(|out| {
            let expected = crate::forge::types::InferredType::from_sce_type(&out.sce_type);
            let expr_py = expr::transpile_typed(
                out.expr.as_deref().unwrap_or("0"),
                ExprTarget::Python,
                &type_ctx,
                &empty_renames,
                expected,
            )?;

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

    let output_is_string = m.output_is_string();
    let on_miss_error = m.miss_policy.is_error();

    // Pre-compute condition expression per group: `==` for single key, `in (...)` for multiple.
    // Single-element tuples need a trailing comma in Python: `(0x07,)`.
    let (entries_by_value, unique_values, default_value) = if output_is_string {
        let ebv: Vec<serde_json::Value> = m
            .entries_by_value()
            .into_iter()
            .map(|(value, keys)| {
                let condition = if keys.len() == 1 {
                    format!("{} == {}", input_id_snake, keys[0])
                } else {
                    format!("{} in ({})", input_id_snake, keys.join(", "))
                };
                serde_json::json!({"value": value, "condition": condition})
            })
            .collect();
        let uv = m.unique_values();
        let dv = match &m.miss_policy {
            MissPolicy::Default(s) => s.clone(),
            MissPolicy::Error => String::new(),
        };
        (ebv, uv, dv)
    } else {
        (Vec::new(), Vec::new(), String::new())
    };

    let (keys_literal, values_literal, default_literal) = if !output_is_string {
        let kl: Vec<String> = m
            .entries
            .iter()
            .map(|e| python_literal(&e.key, &m.input.sce_type))
            .collect();
        let vl: Vec<String> = m
            .entries
            .iter()
            .map(|e| python_literal(&e.value, &m.output.sce_type))
            .collect();
        let dl = match &m.miss_policy {
            MissPolicy::Default(s) => python_literal(s, &m.output.sce_type),
            MissPolicy::Error => String::new(),
        };
        (kl, vl, dl)
    } else {
        (Vec::new(), Vec::new(), String::new())
    };

    let tmpl = env
        .get_template("lookup.py.jinja2")
        .map_err(|e| format!("Template load error: {e}"))?;

    let (has_imports, all_imports, stateful_imports) = build_template_imports(imports);

    let ctx = minijinja::context! {
        enum_name => enum_name,
        func_name => func_name,
        input_type => python_type(&m.input.sce_type),
        value_type => python_type(&m.output.sce_type),
        input_id => input_id_snake,
        unique_values => minijinja::Value::from_serialize(&unique_values),
        entries_by_value => minijinja::Value::from_serialize(&entries_by_value),
        default_value => default_value,
        default_literal => default_literal,
        output_is_string => output_is_string,
        on_miss_error => on_miss_error,
        keys_literal => minijinja::Value::from_serialize(&keys_literal),
        values_literal => minijinja::Value::from_serialize(&values_literal),
        n => m.entries.len(),
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

    let type_ctx = crate::forge::type_ctx::condition(m, imports);
    let expr_py = expr::transpile_typed(
        &m.expr,
        ExprTarget::Python,
        &type_ctx,
        &std::collections::HashMap::new(),
        crate::forge::types::InferredType::Bool,
    )?;

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
                "default_value": python_default(&f.sce_type),
                "decode_expr": generate_decode_expr_python(f, m.default_endian),
            })
        })
        .collect();

    let encode_exprs: Vec<String> = generate_encode_exprs_python(&m.fields, m.default_endian);

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
                filters::to_snake_case(field.id.clone())
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
                let name = filters::to_snake_case(field.id.clone());
                parts.push(format!(
                    "(self.{name} & 0x{mask:02X}) << {bit_off}"
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
    let name = filters::to_snake_case(field.id.clone());

    match field.fixed_bits() {
        Some(8) if bit_off == 0 => {
            exprs.push(format!("self.{name} & 0xFF"));
        }
        Some(bits) if bits < 8 || bit_off > 0 => {
            let mask = (1u64 << bits) - 1;
            exprs.push(format!(
                "(self.{name} & 0x{mask:02X}) << {bit_off} & 0xFF"
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
                    exprs.push(format!("self.{name} & 0xFF"));
                } else {
                    exprs.push(format!(
                        "(self.{name} >> {shift}) & 0xFF"
                    ));
                }
            }
        }
        _ => exprs.push(format!("# encode {name}")),
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

    // Python local-var convention is snake_case, which happens to coincide
    // with the canonical reason fragment — but they remain semantically
    // distinct fields so the template's `rule.reason_id` use is uniform
    // across all 5 languages and `ResolvedRange` stays the single source of
    // truth for reason-string casing.
    let range_rules: Vec<serde_json::Value> = rv.ranges.iter()
        .map(|r| {
            let snake = filters::to_snake_case(r.id.clone());
            serde_json::json!({
                "id": snake,
                "reason_id": r.canonical_reason_id(),
                "min": r.min, "max": r.max,
                "has_min": r.min.is_some(), "has_max": r.max.is_some(),
            })
        })
        .collect();

    let roc_rules: Vec<serde_json::Value> = rv.rocs.iter()
        .map(|roc| {
            let snake = filters::to_snake_case(roc.id.clone());
            serde_json::json!({
                "id": snake.clone(),
                "reason_id": roc.canonical_reason_id(),
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

    let type_ctx = crate::forge::type_ctx::validator(m, imports);
    let plausibility_expr = match &rv.plausibility {
        Some(e) => Some(expr::transpile_typed(
            e,
            ExprTarget::Python,
            &type_ctx,
            &import_renames,
            crate::forge::types::InferredType::Bool,
        )?),
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

// ── Procedure: C++ ──────────────────────────────────────────

fn render_procedure_cpp(
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

    // <sce:helper> DI closure members (C++ std::function). Initialised to a
    // fail-fast lambda that throws std::runtime_error with a clear "helper
    // not set" message — a default-constructed std::function would throw
    // std::bad_function_call at invoke time, giving the caller zero context
    // about which helper was missing or which setter to call. See the Rust
    // / Python / Go / Kotlin branches above for the matching fail-fast
    // rationale.
    let helper_fields: Vec<serde_json::Value> = m
        .helpers
        .iter()
        .map(|h| {
            let params_ty: Vec<String> =
                h.args.iter().map(cpp_param_type).collect();
            let ret_ty = cpp_type(&h.returns);
            let function_type = format!(
                "std::function<{}({})>",
                ret_ty,
                params_ty.join(", "),
            );
            let setter_name = filters::to_pascal_case(h.name.clone());
            // Typed lambda signature matching the function_type. Param names
            // are generated _argN so unused-parameter warnings stay quiet.
            let lambda_params: Vec<String> = h
                .args
                .iter()
                .enumerate()
                .map(|(i, a)| format!("{} _arg{i}", cpp_param_type(a)))
                .collect();
            let default_impl = format!(
                "[]({}) -> {} {{ throw std::runtime_error(\"helper '{}' not set — call set{}() before runToCompletion()\"); }}",
                lambda_params.join(", "),
                ret_ty,
                h.name,
                setter_name,
            );
            serde_json::json!({
                "id": h.name,                                     // user-visible name
                "member_name": format!("{}_", h.name),            // trailing-underscore member
                "setter_name": setter_name,
                "function_type": function_type,
                "default_impl": default_impl,
            })
        })
        .collect();

    // Build the typed context once — every expression in this render
    // function (internal defaults, guards, assigns, sends, donedata) sees
    // the same set of procedure inputs/internals as identifiers.
    let procedure_type_ctx = crate::forge::type_ctx::procedure(m, imports);
    let empty_procedure_renames = std::collections::HashMap::new();
    let internal_fields: Vec<serde_json::Value> = m
        .internals
        .iter()
        .map(|f| {
            let expected = crate::forge::types::InferredType::from_sce_type(&f.sce_type);
            let default_val = f.expr.as_ref().map(|e| {
                expr::transpile_typed(
                    e,
                    ExprTarget::Cpp,
                    &procedure_type_ctx,
                    &empty_procedure_renames,
                    expected,
                )
                .unwrap_or_else(|_| e.clone())
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
    // Method-level rename entries for stateful imports: `frame.encode` →
    // `frame_.encode` (C++). Site-owned Vec keeps the qualified keys alive so
    // the `HashMap<&str, String>` can borrow them. See
    // `stateful_import_method_renames` for the rationale.
    let cpp_method_renames =
        stateful_import_method_renames(imports, &generator::Language::Cpp);
    for (k, v) in &cpp_method_renames {
        owned_rename_map.insert(k.as_str(), v.clone());
    }
    let cpp_field_renames =
        stateful_import_field_renames(imports, &generator::Language::Cpp);
    for (k, v) in &cpp_field_renames {
        owned_rename_map.insert(k.as_str(), v.clone());
    }
    let cpp_helper_rename_pairs: Vec<(String, String)> = m
        .helpers
        .iter()
        .map(|h| (h.name.clone(), format!("{}_", h.name)))
        .collect();
    for (k, v) in &cpp_helper_rename_pairs {
        owned_rename_map.insert(k.as_str(), v.clone());
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
                        transpile_procedure_expr(
                            a,
                            ExprTarget::Cpp,
                            &procedure_type_ctx,
                            &rename_map,
                            crate::forge::types::InferredType::Unknown,
                        )
                    });
                    let payload_expr = send.payload.as_ref().map(|p| {
                        transpile_procedure_expr(
                            p,
                            ExprTarget::Cpp,
                            &procedure_type_ctx,
                            &rename_map,
                            crate::forge::types::InferredType::Unknown,
                        )
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
                    let transpiled = transpile_procedure_expr(
                        &p.expr,
                        ExprTarget::Cpp,
                        &procedure_type_ctx,
                        &rename_map,
                        crate::forge::types::InferredType::Unknown,
                    );
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
                        transpile_procedure_expr(
                            c,
                            ExprTarget::Cpp,
                            &procedure_type_ctx,
                            &rename_map,
                            crate::forge::types::InferredType::Bool,
                        )
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

    let states_with_assigns = build_procedure_states_with_assigns(
        m,
        ExprTarget::Cpp,
        &procedure_type_ctx,
        &assign_rename_map,
    );

    // Collect raw sce:payload expressions for header dependency comment (CR#6)
    let payload_exprs: Vec<String> = m
        .states
        .iter()
        .flat_map(|s| s.on_entry_sends.iter())
        .filter_map(|send| send.payload.clone())
        .collect();
    let has_external_deps = !payload_exprs.is_empty();

    let tmpl = env
        .get_template("procedure.h.jinja2")
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
        helper_fields => minijinja::Value::from_serialize(&helper_fields),
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

/// Compute method-level rename entries for every stateful import, so the
/// expression rename pass can collapse `alias.method` Member nodes into a
/// target-language-native call fragment.
///
/// **Why this exists**: `rename_identifiers` handles `Member{Ident(obj), prop}`
/// by looking up the full `"obj.prop"` path in the rename map, and only
/// falls back to renaming `obj` alone when the qualified path is absent.
/// Without qualified entries for each imported kind's public methods, the
/// property name (`encode`, `decode`, `update`, ...) flows through verbatim,
/// which is wrong for Go (PascalCase exports: `Encode`, `Decode`) and any
/// future language whose stateful kind method names diverge from the
/// source-level SCXML spelling. The 4 languages that happen to use
/// lowercase method names emit byte-identical output with or without this
/// helper; Go's Encode/Decode is the motivating consumer.
///
/// Returns `(qualified_source_path, target_expansion)` pairs whose
/// qualified paths (e.g. `"frame.encode"`) match the shape the rename pass
/// forms from `Member{Ident("frame"), "encode"}`. Callers own the returned
/// `Vec<(String, String)>` so its borrowed keys can feed into the existing
/// `HashMap<&str, String>` rename maps at each procedure generator site.
fn stateful_import_method_renames(
    imports: &[ImportContext],
    language: &generator::Language,
) -> Vec<(String, String)> {
    let mut out = Vec::new();
    for imp in imports {
        if !imp.is_stateful {
            continue;
        }
        match imp.kind.as_str() {
            "codec" => {
                // Codec exposes `encode()` on an instance; `decode(raw)` is a
                // package-level free function in Go (`DecodeCodecSimpleFrame`)
                // and a static/class method in C++/Kotlin/Python/Rust, so the
                // mapping shape diverges per language. Until a fixture
                // actually uses `alias.decode(...)`, we only emit `encode`
                // entries — adding `decode` without a load-bearing consumer
                // risks baking the wrong Go expansion into the helper. Grow
                // this list when the first decode-using fixture lands.
                for method in ["encode"] {
                    let qualified_key = format!("{}.{}", imp.alias, method);
                    // Per-language expansions mirror the member-access
                    // prefix each procedure template actually emits:
                    //   C++      `{member}_.method()`        no `this->`
                    //   Kotlin   `{member}.method()`         no prefix
                    //   Rust     `self.{member}.method()`    `self.`
                    //   Go       `p.{Member}.Method()`       `p.`, PascalCase
                    //   Python   `self.{member}.method()`    `self.`
                    let expansion = match language {
                        generator::Language::Cpp | generator::Language::Kotlin => {
                            format!("{}.{}", imp.member_name, method)
                        }
                        generator::Language::Rust | generator::Language::Python => {
                            format!("self.{}.{}", imp.member_name, method)
                        }
                        generator::Language::Go => {
                            let target_method = filters::to_pascal_case(method.to_string());
                            format!("p.{}.{}", imp.member_name, target_method)
                        }
                    };
                    out.push((qualified_key, expansion));
                }
            }
            // Other stateful kinds (filter, observer, validator, procedure)
            // expose their own method APIs (`update`, `validate`, ...), but
            // no conformance fixture currently imports them as stateful
            // aliases inside another kind. Adding entries here without a
            // load-bearing consumer would be dormant infrastructure — grow
            // this arm when a future fixture imports a filter/observer
            // method call and the byte golden fails to compile.
            _ => {}
        }
    }
    out
}

/// Compute per-field rename entries for every stateful import's publicly
/// accessible data members. This is the field-access counterpart to
/// [`stateful_import_method_renames`] which handles method calls.
///
/// For each `(alias, field)` pair discovered via enrichment in
/// [`validate_and_enrich_imports`], the helper produces a
/// `"{alias}.{field}"` → `"<target-specific member access>"` entry so the
/// rename pass collapses `Member{Ident(alias), field}` into a `Raw` node
/// with the correct target-language spelling (snake_case for Rust/Python,
/// PascalCase for Go, verbatim for C++/Kotlin).
fn stateful_import_field_renames(
    imports: &[ImportContext],
    language: &generator::Language,
) -> Vec<(String, String)> {
    let mut out = Vec::new();
    for imp in imports {
        if !imp.is_stateful {
            continue;
        }
        for (qualified_key, _) in &imp.member_field_types {
            // qualified_key is already `"alias.field"` (see lib.rs enrichment).
            // Extract the bare field name for per-language case conversion.
            let field = match qualified_key.split_once('.') {
                Some((_, f)) => f,
                None => continue,
            };
            let expansion = match language {
                generator::Language::Cpp => {
                    format!("{}.{}", imp.member_name, field)
                }
                generator::Language::Kotlin => {
                    format!("{}.{}", imp.member_name, field)
                }
                generator::Language::Rust => {
                    let snake_field = filters::to_snake_case(field.to_string());
                    format!("self.{}.{}", imp.member_name, snake_field)
                }
                generator::Language::Go => {
                    let pascal_field = filters::to_pascal_case(field.to_string());
                    format!("p.{}.{}", imp.member_name, pascal_field)
                }
                generator::Language::Python => {
                    let snake_field = filters::to_snake_case(field.to_string());
                    format!("self.{}.{}", imp.member_name, snake_field)
                }
            };
            out.push((qualified_key.clone(), expansion));
        }
    }
    out
}

/// Transpile a procedure expression with a pre-built rename map and
/// type context. On failure, emits a C++ comment with the error for
/// compile-time visibility.
///
/// `expected` drives top-level coercion — pass `InferredType::Bool` for
/// guard conditions, the target field type for assignments, and
/// `InferredType::Unknown` for payloads/sends where the consumer accepts
/// any value.
fn transpile_procedure_expr(
    raw: &str,
    target: ExprTarget,
    type_ctx: &crate::forge::types::TypeCtx<'_>,
    renames: &std::collections::HashMap<&str, &str>,
    expected: crate::forge::types::InferredType,
) -> String {
    match expr::transpile_typed(raw, target, type_ctx, renames, expected) {
        Ok(result) => result,
        Err(e) => format!("/* SCE_TRANSPILE_ERROR: {} */ {}", e, raw),
    }
}

// ── Procedure: Rust ─────────────────────────────────────────

// ── Procedure: shared helpers ───────────────────────────────

/// Common procedure data shared across all language renderers.
struct ProcedureCommon {
    state_enum: Vec<serde_json::Value>,
    event_enum: Vec<serde_json::Value>,
    event_name_map: std::collections::BTreeMap<String, String>,
    initial_state: String,
    final_states: Vec<serde_json::Value>,
    payload_exprs: Vec<String>,
    has_external_deps: bool,
}

/// Build language-independent procedure data (state/event enums, final states).
fn build_procedure_common(m: &ProcedureModel) -> ProcedureCommon {
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

    ProcedureCommon {
        state_enum,
        event_enum,
        event_name_map: event_raw_to_pascal,
        initial_state,
        final_states,
        payload_exprs,
        has_external_deps,
    }
}

/// Build non-final state transition data for procedure templates.
fn build_procedure_non_final_states(
    m: &ProcedureModel,
    target: ExprTarget,
    type_ctx: &crate::forge::types::TypeCtx<'_>,
    rename_map: &std::collections::HashMap<&str, &str>,
    event_name_map: &std::collections::BTreeMap<String, String>,
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
                        transpile_procedure_expr(
                            c,
                            target,
                            type_ctx,
                            rename_map,
                            crate::forge::types::InferredType::Bool,
                        )
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

/// Build states with onentry sends for procedure templates.
fn build_procedure_states_with_entry(
    m: &ProcedureModel,
    target: ExprTarget,
    type_ctx: &crate::forge::types::TypeCtx<'_>,
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
                    let addr_expr = send.addr.as_ref().map(|a| {
                        transpile_procedure_expr(
                            a,
                            target,
                            type_ctx,
                            rename_map,
                            crate::forge::types::InferredType::Unknown,
                        )
                    });
                    let payload_expr = send.payload.as_ref().map(|p| {
                        transpile_procedure_expr(
                            p,
                            target,
                            type_ctx,
                            payload_map,
                            crate::forge::types::InferredType::Unknown,
                        )
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
        .collect()
}

/// Build final states with donedata for procedure templates.
fn build_procedure_final_states_with_donedata(
    m: &ProcedureModel,
    target: ExprTarget,
    type_ctx: &crate::forge::types::TypeCtx<'_>,
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
                    let transpiled = transpile_procedure_expr(
                        &p.expr,
                        target,
                        type_ctx,
                        rename_map,
                        crate::forge::types::InferredType::Unknown,
                    );
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

/// Build states that have transitions with assigns for procedure templates.
///
/// Both sides of every assignment flow through the expression pipeline:
///   * **LHS** (`a.location`): via [`expr::transpile_lvalue`] — validates
///     the shape is a legal lvalue (bare ident or single-level member),
///     runs the full `tokenize → parse → infer → rename → emit` pass, and
///     returns the inferred type.
///   * **RHS** (`a.expr`): via [`transpile_procedure_expr`] as before, using
///     the LHS's inferred type as the `expected` parameter to drive coercion.
///
/// This replaces the earlier design where LHS was transformed by per-language
/// closures (`location_transform`) operating on the raw string — a path that
/// bypassed inference, renaming, and emission, and broke on any location
/// grammar beyond bare top-level identifiers.
fn build_procedure_states_with_assigns(
    m: &ProcedureModel,
    target: ExprTarget,
    type_ctx: &crate::forge::types::TypeCtx<'_>,
    assign_rename_map: &std::collections::HashMap<&str, &str>,
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
                            let (location_emitted, lhs_ty) = expr::transpile_lvalue(
                                &a.location,
                                target,
                                type_ctx,
                                assign_rename_map,
                            )
                            .unwrap_or_else(|e| {
                                (
                                    format!("/* SCE_LVALUE_ERROR: {} */ {}", e, a.location),
                                    crate::forge::types::InferredType::Unknown,
                                )
                            });
                            let transpiled = transpile_procedure_expr(
                                &a.expr,
                                target,
                                type_ctx,
                                assign_rename_map,
                                lhs_ty,
                            );
                            let wrapped = if matches!(lhs_ty, crate::forge::types::InferredType::Bytes)
                                && a.expr.trim() == "_event.data"
                            {
                                bytes_wrap_for(target, &transpiled)
                            } else {
                                transpiled
                            };
                            serde_json::json!({
                                "location": location_emitted,
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

/// Language-specific wrapping for `_event.data` assignment to a Bytes-typed
/// lvalue. Each target language has its own idiom for converting a string
/// (the pending event data) into the native byte container.
fn bytes_wrap_for(target: ExprTarget, transpiled: &str) -> String {
    match target {
        ExprTarget::Cpp => {
            format!("std::vector<uint8_t>({transpiled}.begin(), {transpiled}.end())")
        }
        ExprTarget::Kotlin => format!("{transpiled}.toByteArray()"),
        ExprTarget::Rust => format!("{transpiled}.as_bytes().to_vec()"),
        ExprTarget::Go => format!("[]byte({transpiled})"),
        ExprTarget::Python => format!("{transpiled}.encode()"),
    }
}

/// Build the type map (variable name → SceType) for assign type checking.
fn build_procedure_type_map<'a>(m: &'a ProcedureModel) -> std::collections::HashMap<&'a str, &'a SceType> {
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

// ── Procedure: Kotlin ───────────────────────────────────────

fn render_procedure_kotlin(
    env: &minijinja::Environment,
    m: &ProcedureModel,
    imports: &[ImportContext],
) -> Result<String, String> {
    let pascal = filters::to_pascal_case(m.name.clone());
    let package = filters::to_snake_case(m.name.clone());
    let common = build_procedure_common(m);

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

    // <sce:helper> DI closure members (Kotlin function-type properties).
    // Initialised to a fail-fast lambda using `error("...")` (throws
    // IllegalStateException) rather than a zero-value closure — matching the
    // Rust / C++ / Python / Go branches.
    let helper_fields: Vec<serde_json::Value> = m
        .helpers
        .iter()
        .map(|h| {
            let params_ty: Vec<String> = h
                .args
                .iter()
                .map(|a| kotlin_type(a).to_string())
                .collect();
            let ret_ty = kotlin_type(&h.returns);
            let function_type = format!(
                "({}) -> {}",
                params_ty.join(", "),
                ret_ty,
            );
            let setter_name = filters::to_pascal_case(h.name.clone());
            let placeholder_args = (0..h.args.len())
                .map(|i| format!("_arg{i}"))
                .collect::<Vec<_>>()
                .join(", ");
            let default_impl = format!(
                "{{ {placeholder_args} -> error(\"helper '{}' not set — call set{}() before runToCompletion()\") }}",
                h.name,
                setter_name,
            );
            serde_json::json!({
                "id": h.name,
                "setter_name": setter_name,
                "function_type": function_type,
                "default_impl": default_impl,
            })
        })
        .collect();

    let procedure_type_ctx = crate::forge::type_ctx::procedure(m, imports);
    let empty_procedure_renames = std::collections::HashMap::new();

    // Internal fields
    let internal_fields: Vec<serde_json::Value> = m
        .internals
        .iter()
        .map(|f| {
            let expected = crate::forge::types::InferredType::from_sce_type(&f.sce_type);
            let default_val = f
                .expr
                .as_ref()
                .map(|e| expr::transpile_typed(
                    e,
                    ExprTarget::Kotlin,
                    &procedure_type_ctx,
                    &empty_procedure_renames,
                    expected,
                ).unwrap_or_else(|_| e.clone()))
                .unwrap_or_else(|| kotlin_default(&f.sce_type).to_string());
            serde_json::json!({
                "id": f.id,
                "kt_type": kotlin_type(&f.sce_type),
                "default_value": default_val,
            })
        })
        .collect();

    // Rename map: Kotlin only renames _event.data → pendingEventData, plus
    // stateful-import method entries so `alias.encode` collapses cleanly
    // (byte-identical to the current verbatim path for codec, since Kotlin
    // codec methods are already lowercase — the entries exist so future
    // Kotlin-specific method casing has a single source of truth).
    //
    // Note: <sce:helper> declarations do NOT need rename entries here. Kotlin
    // function-type class properties are directly invokable via `operator fun
    // invoke`, and bare `computeKey(seed)` inside a class method body resolves
    // through the implicit `this` receiver to `this.computeKey(seed)`. The
    // expression pipeline's type inference picks up the helper's signature
    // from `ctx.funcs` (seeded by type_ctx::insert_procedure_helpers), so no
    // syntactic rewriting at the rename pass is required for Kotlin.
    let mut owned_rename: std::collections::HashMap<&str, String> =
        std::collections::HashMap::from([("_event.data", "pendingEventData".to_string())]);
    let kotlin_method_renames =
        stateful_import_method_renames(imports, &generator::Language::Kotlin);
    for (k, v) in &kotlin_method_renames {
        owned_rename.insert(k.as_str(), v.clone());
    }
    let kotlin_field_renames =
        stateful_import_field_renames(imports, &generator::Language::Kotlin);
    for (k, v) in &kotlin_field_renames {
        owned_rename.insert(k.as_str(), v.clone());
    }
    let rename_map: std::collections::HashMap<&str, &str> = owned_rename
        .iter()
        .map(|(k, v)| (*k, v.as_str()))
        .collect();

    let assign_rename_map = rename_map.clone();

    let states_with_entry =
        build_procedure_states_with_entry(m, ExprTarget::Kotlin, &procedure_type_ctx, &rename_map, None);
    let final_states_with_donedata =
        build_procedure_final_states_with_donedata(m, ExprTarget::Kotlin, &procedure_type_ctx, &rename_map);

    let non_final_states = build_procedure_non_final_states(
        m,
        ExprTarget::Kotlin,
        &procedure_type_ctx,
        &rename_map,
        &common.event_name_map,
    );

    let states_with_assigns = build_procedure_states_with_assigns(
        m,
        ExprTarget::Kotlin,
        &procedure_type_ctx,
        &assign_rename_map,
    );

    let tmpl = env
        .get_template("procedure.kt.jinja2")
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
        helper_fields => minijinja::Value::from_serialize(&helper_fields),
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

// ── Procedure: Rust ─────────────────────────────────────────

fn render_procedure_rust(
    env: &minijinja::Environment,
    m: &ProcedureModel,
    imports: &[ImportContext],
) -> Result<String, String> {
    let pascal = filters::to_pascal_case(m.name.clone());
    let snake = filters::to_snake_case(m.name.clone());
    let common = build_procedure_common(m);

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
    // Method-level rename entries for stateful imports (Rust expansion:
    // `self.{member}.{method}`). Site-owned Vec keeps qualified keys alive.
    let rust_method_renames =
        stateful_import_method_renames(imports, &generator::Language::Rust);
    for (k, v) in &rust_method_renames {
        owned_rename_with_event.insert(k.as_str(), v.clone());
    }
    let rust_field_renames =
        stateful_import_field_renames(imports, &generator::Language::Rust);
    for (k, v) in &rust_field_renames {
        owned_rename_with_event.insert(k.as_str(), v.clone());
    }
    // <sce:helper> rename entries: every declared helper call site collapses
    // to `(self.helper_name)(...)`. The extra parens are required so Rust
    // parses the closure field access as the callee of the invocation,
    // disambiguating from a `self.helper_name(...)` method call.
    let helper_rename_pairs: Vec<(String, String)> = m
        .helpers
        .iter()
        .map(|h| {
            (
                h.name.clone(),
                format!("(self.{})", filters::to_snake_case(h.name.clone())),
            )
        })
        .collect();
    for (k, v) in &helper_rename_pairs {
        owned_rename_with_event.insert(k.as_str(), v.clone());
    }
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

    // <sce:helper> DI closure members. Each declared helper becomes a field
    // of type `Box<dyn Fn(...) -> ...>` initialised to a fail-fast sentinel
    // (panics with a clear "helper not set" message when invoked without a
    // prior setter call), plus a public setter accepting any
    // `Fn(...) -> ... + 'static`. Call sites in expressions dispatch through
    // the rename map as `(self.helper_name)(args)`. Fail-fast instead of
    // silently returning a zero/empty value: a helper inside an expression
    // has no sensible no-op semantic (unlike `serviceHandler` which can
    // legitimately skip a send), so an unset helper is a programming bug
    // that must surface immediately rather than produce wrong numbers.
    let helper_fields: Vec<serde_json::Value> = m
        .helpers
        .iter()
        .map(|h| {
            let snake = filters::to_snake_case(h.name.clone());
            let setter_name = format!("set_{}", snake);
            let params_ty: Vec<String> =
                h.args.iter().map(rust_param_type).collect();
            let ret_ty = rust_type(&h.returns);
            let closure_type = format!(
                "Box<dyn Fn({}) -> {}>",
                params_ty.join(", "),
                ret_ty,
            );
            let setter_param_type = format!(
                "impl Fn({}) -> {} + 'static",
                params_ty.join(", "),
                ret_ty,
            );
            let placeholder_args = (0..h.args.len())
                .map(|i| format!("_arg{i}"))
                .collect::<Vec<_>>()
                .join(", ");
            let default_impl = format!(
                "Box::new(|{placeholder_args}| panic!(\"helper '{}' not set — call {}() before run_to_completion()\"))",
                h.name,
                setter_name,
            );
            serde_json::json!({
                "id": snake,
                "setter_name": setter_name,
                "closure_type": closure_type,
                "setter_param_type": setter_param_type,
                "default_impl": default_impl,
            })
        })
        .collect();

    let procedure_type_ctx = crate::forge::type_ctx::procedure(m, imports);
    let empty_procedure_renames = std::collections::HashMap::new();

    // Internal fields
    let internal_fields: Vec<serde_json::Value> = m
        .internals
        .iter()
        .map(|f| {
            let snake_id = filters::to_snake_case(f.id.clone());
            let expected = crate::forge::types::InferredType::from_sce_type(&f.sce_type);
            let default_val = f
                .expr
                .as_ref()
                .map(|e| expr::transpile_typed(
                    e,
                    ExprTarget::Rust,
                    &procedure_type_ctx,
                    &empty_procedure_renames,
                    expected,
                ).unwrap_or_else(|_| e.clone()))
                .unwrap_or_else(|| rust_default(&f.sce_type).to_string());
            serde_json::json!({
                "id": snake_id,
                "rs_type": rust_type(&f.sce_type),
                "default_value": default_val,
            })
        })
        .collect();

    let type_map = build_procedure_type_map(m);

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
    // Method-level rename entries for stateful imports (same Rust expansion
    // as `rename_map` — `rust_method_renames` is reused so the key strings
    // outlive both HashMaps).
    for (k, v) in &rust_method_renames {
        owned_payload_rename.insert(k.as_str(), v.clone());
    }
    for (k, v) in &rust_field_renames {
        owned_payload_rename.insert(k.as_str(), v.clone());
    }
    for (k, v) in &helper_rename_pairs {
        owned_payload_rename.insert(k.as_str(), v.clone());
    }
    let payload_rename_map: std::collections::HashMap<&str, &str> = owned_payload_rename
        .iter()
        .map(|(k, v)| (*k, v.as_str()))
        .collect();

    let states_with_entry = build_procedure_states_with_entry(
        m,
        ExprTarget::Rust,
        &procedure_type_ctx,
        &rename_map,
        Some(&payload_rename_map),
    );
    let final_states_with_donedata =
        build_procedure_final_states_with_donedata(m, ExprTarget::Rust, &procedure_type_ctx, &rename_map);
    let non_final_states = build_procedure_non_final_states(
        m,
        ExprTarget::Rust,
        &procedure_type_ctx,
        &rename_map,
        &common.event_name_map,
    );
    let states_with_assigns = build_procedure_states_with_assigns(
        m,
        ExprTarget::Rust,
        &procedure_type_ctx,
        &assign_rename_map,
    );

    let tmpl = env
        .get_template("procedure.rs.jinja2")
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
        helper_fields => minijinja::Value::from_serialize(&helper_fields),
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

// ── Procedure: Go ───────────────────────────────────────────

fn render_procedure_go(
    env: &minijinja::Environment,
    m: &ProcedureModel,
    imports: &[ImportContext],
) -> Result<String, String> {
    let pascal = filters::to_pascal_case(m.name.clone());
    let package = filters::to_snake_case(m.name.clone());
    let common = build_procedure_common(m);

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
    // Method-level rename entries for stateful imports. Go is the only
    // language whose codec methods are PascalCase exports (`Encode` /
    // `Decode`), so this is the load-bearing consumer for the helper: the
    // existing byte golden `p.Frame.encode()` fails to compile and must
    // become `p.Frame.Encode()`.
    let go_method_renames =
        stateful_import_method_renames(imports, &generator::Language::Go);
    for (k, v) in &go_method_renames {
        owned_rename_with_event.insert(k.as_str(), v.clone());
    }
    let go_field_renames =
        stateful_import_field_renames(imports, &generator::Language::Go);
    for (k, v) in &go_field_renames {
        owned_rename_with_event.insert(k.as_str(), v.clone());
    }
    // <sce:helper> rename entries — Go func-field members accessed via the
    // struct receiver `p.helperName(...)`. The helper name is kept in its
    // source camelCase casing (unexported — state machine private field).
    let go_helper_rename_pairs: Vec<(String, String)> = m
        .helpers
        .iter()
        .map(|h| {
            (
                h.name.clone(),
                format!("p.{}", go_escape_builtin(&h.name)),
            )
        })
        .collect();
    for (k, v) in &go_helper_rename_pairs {
        owned_rename_with_event.insert(k.as_str(), v.clone());
    }
    let rename_map: std::collections::HashMap<&str, &str> = owned_rename_with_event
        .iter()
        .map(|(k, v)| (*k, v.as_str()))
        .collect();
    let assign_rename_map = rename_map.clone();

    // Determine if fmt import is needed. The Go template uses fmt.Sprint()
    // only for addr stringification; payload now flows through as raw
    // `[]byte` without any conversion, so payload-only procedures no
    // longer pull in fmt.
    let needs_fmt = m
        .states
        .iter()
        .flat_map(|s| s.on_entry_sends.iter())
        .any(|send| send.addr.is_some());

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

    // <sce:helper> DI closure members (Go func fields). Go uses
    // constructor-injection (no setters): the helper is a required
    // positional parameter on `newPolicy` / `Execute`. A missing arg is a
    // compile error; a nil arg is swapped in-place for a fail-fast closure
    // that panics with a clear "helper not set" message, so no call site
    // bypasses the checked contract.
    let helper_fields: Vec<serde_json::Value> = m
        .helpers
        .iter()
        .map(|h| {
            let escaped_id = go_escape_builtin(&h.name);
            let params_ty: Vec<String> =
                h.args.iter().map(|a| go_type(a).to_string()).collect();
            let ret_ty = go_type(&h.returns);
            let function_type = format!(
                "func({}) {}",
                params_ty.join(", "),
                ret_ty,
            );
            let placeholder_args = (0..h.args.len())
                .map(|i| format!("_arg{i} {}", params_ty[i]))
                .collect::<Vec<_>>()
                .join(", ");
            // Nil-replacement closure emitted in newPolicy when the caller
            // passes nil — same fail-fast shape as the other 4 languages'
            // default_impl, adapted to Go's constructor-injection model.
            let default_impl = format!(
                "func({placeholder_args}) {ret_ty} {{ panic(\"helper '{}' passed nil to Execute — pass a non-nil func({}) {} argument\") }}",
                h.name,
                params_ty.join(", "),
                ret_ty,
            );
            serde_json::json!({
                "id": escaped_id,
                "function_type": function_type,
                "default_impl": default_impl,
            })
        })
        .collect();

    let procedure_type_ctx = crate::forge::type_ctx::procedure(m, imports);
    let empty_procedure_renames = std::collections::HashMap::new();

    // Internal fields
    let internal_fields: Vec<serde_json::Value> = m
        .internals
        .iter()
        .map(|f| {
            let go_id = go_escape_builtin(&f.id);
            let expected = crate::forge::types::InferredType::from_sce_type(&f.sce_type);
            let default_val = f.expr.as_ref().map(|e| {
                expr::transpile_typed(
                    e,
                    ExprTarget::Go,
                    &procedure_type_ctx,
                    &empty_procedure_renames,
                    expected,
                )
                .unwrap_or_else(|_| e.clone())
            });
            serde_json::json!({
                "id": go_id,
                "go_type": go_type(&f.sce_type),
                "has_default": default_val.is_some(),
                "default_value": default_val.unwrap_or_default(),
            })
        })
        .collect();

    let states_with_entry =
        build_procedure_states_with_entry(m, ExprTarget::Go, &procedure_type_ctx, &rename_map, None);
    let final_states_with_donedata =
        build_procedure_final_states_with_donedata(m, ExprTarget::Go, &procedure_type_ctx, &rename_map);
    let non_final_states = build_procedure_non_final_states(
        m,
        ExprTarget::Go,
        &procedure_type_ctx,
        &rename_map,
        &common.event_name_map,
    );
    let states_with_assigns = build_procedure_states_with_assigns(
        m,
        ExprTarget::Go,
        &procedure_type_ctx,
        &assign_rename_map,
    );

    let tmpl = env
        .get_template("procedure.go.jinja2")
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
        helper_fields => minijinja::Value::from_serialize(&helper_fields),
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

// ── Procedure: Python ───────────────────────────────────────

fn render_procedure_python(
    env: &minijinja::Environment,
    m: &ProcedureModel,
    imports: &[ImportContext],
) -> Result<String, String> {
    let pascal = filters::to_pascal_case(m.name.clone());
    let snake = filters::to_snake_case(m.name.clone());
    let common = build_procedure_common(m);

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
    // Method-level rename entries for stateful imports (Python expansion:
    // `self.{member}.{method}`).
    let python_method_renames =
        stateful_import_method_renames(imports, &generator::Language::Python);
    for (k, v) in &python_method_renames {
        owned_rename_with_event.insert(k.as_str(), v.clone());
    }
    let python_field_renames =
        stateful_import_field_renames(imports, &generator::Language::Python);
    for (k, v) in &python_field_renames {
        owned_rename_with_event.insert(k.as_str(), v.clone());
    }
    // <sce:helper> rename entries — Python instance-method-level helpers use
    // the standard `self._name` prefix, matching the datamodel field naming
    // convention. Bare `computeKey(x)` inside a method body would not resolve
    // to a class field, so the rename is load-bearing.
    let python_helper_rename_pairs: Vec<(String, String)> = m
        .helpers
        .iter()
        .map(|h| {
            (
                h.name.clone(),
                format!("self._{}", filters::to_snake_case(h.name.clone())),
            )
        })
        .collect();
    for (k, v) in &python_helper_rename_pairs {
        owned_rename_with_event.insert(k.as_str(), v.clone());
    }
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

    // <sce:helper> DI closure members (Python typing.Callable). Initialised
    // to a fail-fast sentinel produced by the module-level
    // `_unset_helper_raiser` factory that the template emits when helpers
    // are present — Python lambdas cannot contain a raise statement, so the
    // factory returns a nested `def` that raises RuntimeError with context.
    // Matches the Rust / C++ / Go / Kotlin fail-fast rationale.
    let helper_fields: Vec<serde_json::Value> = m
        .helpers
        .iter()
        .map(|h| {
            let snake = filters::to_snake_case(h.name.clone());
            let setter_name = format!("set_{}", snake);
            let params_ty: Vec<String> = h
                .args
                .iter()
                .map(|a| python_type(a).to_string())
                .collect();
            let ret_ty = python_type(&h.returns);
            let callable_type = format!(
                "Callable[[{}], {}]",
                params_ty.join(", "),
                ret_ty,
            );
            let default_impl = format!(
                "_unset_helper_raiser({:?}, {:?})",
                h.name, setter_name,
            );
            serde_json::json!({
                "snake_id": snake,
                "setter_name": setter_name,
                "callable_type": callable_type,
                "default_impl": default_impl,
            })
        })
        .collect();

    let procedure_type_ctx = crate::forge::type_ctx::procedure(m, imports);
    let empty_procedure_renames = std::collections::HashMap::new();

    // Internal fields
    let internal_fields: Vec<serde_json::Value> = m
        .internals
        .iter()
        .map(|f| {
            let snake_id = filters::to_snake_case(f.id.clone());
            let expected = crate::forge::types::InferredType::from_sce_type(&f.sce_type);
            let default_val = f
                .expr
                .as_ref()
                .map(|e| expr::transpile_typed(
                    e,
                    ExprTarget::Python,
                    &procedure_type_ctx,
                    &empty_procedure_renames,
                    expected,
                ).unwrap_or_else(|_| e.clone()))
                .unwrap_or_else(|| python_default(&f.sce_type).to_string());
            serde_json::json!({
                "snake_id": snake_id,
                "py_type": python_type(&f.sce_type),
                "default_value": default_val,
            })
        })
        .collect();

    let states_with_entry =
        build_procedure_states_with_entry(m, ExprTarget::Python, &procedure_type_ctx, &rename_map, None);
    let final_states_with_donedata =
        build_procedure_final_states_with_donedata(m, ExprTarget::Python, &procedure_type_ctx, &rename_map);
    let non_final_states = build_procedure_non_final_states(
        m,
        ExprTarget::Python,
        &procedure_type_ctx,
        &rename_map,
        &common.event_name_map,
    );
    let states_with_assigns = build_procedure_states_with_assigns(
        m,
        ExprTarget::Python,
        &procedure_type_ctx,
        &assign_rename_map,
    );

    let tmpl = env
        .get_template("procedure.py.jinja2")
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
        helper_fields => minijinja::Value::from_serialize(&helper_fields),
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
///
/// Inline kinds are embedded in a statechart's `<data>` element and reference
/// the enclosing statechart's member variables, whose types we do not have
/// direct access to in this rendering path. We therefore build an empty
/// TypeCtx and rely on C++'s implicit numeric conversions — the emitted
/// `return` statement will be type-checked by the host compiler.
fn render_inline_transform_member(
    id: &str,
    raw_expr: &str,
    output_type: &SceType,
) -> Result<String, String> {
    let empty_ctx = crate::forge::type_ctx::empty();
    let empty_renames = std::collections::HashMap::new();
    let expected = crate::forge::types::InferredType::from_sce_type(output_type);
    let expr_cpp = expr::transpile_typed(
        raw_expr,
        ExprTarget::Cpp,
        &empty_ctx,
        &empty_renames,
        expected,
    )?;
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
    let empty_ctx = crate::forge::type_ctx::empty();
    let empty_renames = std::collections::HashMap::new();
    let expr_cpp = expr::transpile_typed(
        raw_expr,
        ExprTarget::Cpp,
        &empty_ctx,
        &empty_renames,
        crate::forge::types::InferredType::Bool,
    )?;
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

// ══════════════════════════════════════════════════════════════
// ── Phase 3: unified render functions (language-parameterized) ──
// ══════════════════════════════════════════════════════════════

/// Language-specific helpers for Phase 3 kind rendering.
/// Eliminates per-language duplication across filter/interpolation/timer/observer.
struct Phase3Lang {
    lang: crate::generator::Language,
}

impl Phase3Lang {
    fn new(lang: crate::generator::Language) -> Self {
        Self { lang }
    }

    fn type_name(&self, ty: &SceType) -> &'static str {
        match self.lang {
            crate::generator::Language::Cpp => cpp_type(ty),
            crate::generator::Language::Kotlin => kotlin_type(ty),
            crate::generator::Language::Rust => rust_type(ty),
            crate::generator::Language::Go => go_type(ty),
            crate::generator::Language::Python => python_type(ty),
        }
    }

    fn param_str(&self, fields: &[ForgeField]) -> String {
        fields.iter()
            .map(|f| {
                let id = match self.lang {
                    crate::generator::Language::Rust | crate::generator::Language::Python =>
                        filters::to_snake_case(f.id.clone()),
                    _ => f.id.clone(),
                };
                match self.lang {
                    crate::generator::Language::Cpp => format!("{} {}", cpp_param_type(&f.sce_type), id),
                    crate::generator::Language::Kotlin => format!("{}: {}", id, kotlin_type(&f.sce_type)),
                    crate::generator::Language::Rust => format!("{}: {}", id, rust_type(&f.sce_type)),
                    crate::generator::Language::Go => format!("{} {}", id, go_type(&f.sce_type)),
                    crate::generator::Language::Python => format!("{}: {}", id, python_type(&f.sce_type)),
                }
            })
            .collect::<Vec<_>>()
            .join(", ")
    }

    fn template_ext(&self) -> &'static str {
        match self.lang {
            crate::generator::Language::Cpp => "h",
            crate::generator::Language::Kotlin => "kt",
            crate::generator::Language::Rust => "rs",
            crate::generator::Language::Go => "go",
            crate::generator::Language::Python => "py",
        }
    }

    fn expr_target(&self) -> ExprTarget {
        match self.lang {
            crate::generator::Language::Cpp => ExprTarget::Cpp,
            crate::generator::Language::Kotlin => ExprTarget::Kotlin,
            crate::generator::Language::Rust => ExprTarget::Rust,
            crate::generator::Language::Go => ExprTarget::Go,
            crate::generator::Language::Python => ExprTarget::Python,
        }
    }

    /// Base context fields common to all Phase 3 kinds.
    fn base_context(&self, name: &str) -> serde_json::Map<String, serde_json::Value> {
        let mut m = serde_json::Map::new();
        let struct_name = filters::to_pascal_case(name.to_string());
        m.insert("struct_name".into(), struct_name.clone().into());
        match self.lang {
            crate::generator::Language::Cpp => {
                m.insert("guard".into(), format!("SCE_FORGE_{}_H", to_upper_snake(name)).into());
                m.insert("namespace".into(), struct_name.into());
            }
            crate::generator::Language::Go => {
                m.insert("package_name".into(), filters::to_snake_case(name.to_string()).into());
            }
            crate::generator::Language::Kotlin => {
                m.insert("package".into(), filters::to_snake_case(name.to_string()).into());
            }
            _ => {}
        }
        m
    }

    /// Event name formatting per language convention.
    fn event_name(&self, s: &str) -> String {
        match self.lang {
            crate::generator::Language::Go => filters::to_pascal_case(s.to_string()),
            _ => to_upper_snake(s),
        }
    }
}

fn render_phase3(
    env: &minijinja::Environment,
    template_name: &str,
    ctx: serde_json::Map<String, serde_json::Value>,
) -> Result<String, String> {
    let tmpl = env
        .get_template(template_name)
        .map_err(|e| format!("Template load error: {e}"))?;
    let value = minijinja::Value::from_serialize(&ctx);
    tmpl.render(value).map_err(generator::render_error)
}

// ── Filter (unified) ──────────────────────────────────────────

fn render_filter(
    env: &minijinja::Environment,
    m: &FilterModel,
    imports: &[ImportContext],
    lang: crate::generator::Language,
) -> Result<String, String> {
    let l = Phase3Lang::new(lang);
    let mut ctx = l.base_context(&m.name);
    let (has_imports, all_imports, stateful_imports) = build_template_imports(imports);

    ctx.insert("filter_type".into(), m.filter_type.as_str().into());
    ctx.insert("input_id".into(), m.input.id.clone().into());
    ctx.insert("input_type".into(), l.type_name(&m.input.sce_type).into());
    ctx.insert("output_type".into(), l.type_name(&m.output.sce_type).into());
    ctx.insert("window".into(), serde_json::json!(m.window));
    ctx.insert("alpha".into(), serde_json::json!(m.alpha));
    ctx.insert("has_imports".into(), has_imports.into());
    ctx.insert("imports".into(), serde_json::to_value(&stateful_imports).unwrap_or_default());
    ctx.insert("all_imports".into(), serde_json::to_value(&all_imports).unwrap_or_default());

    render_phase3(env, &format!("filter.{}.jinja2", l.template_ext()), ctx)
}

// ── Interpolation (unified) ───────────────────────────────────

fn render_interpolation(
    env: &minijinja::Environment,
    m: &InterpolationModel,
    imports: &[ImportContext],
    lang: crate::generator::Language,
) -> Result<String, String> {
    let l = Phase3Lang::new(lang);
    let mut ctx = l.base_context(&m.name);
    let (has_imports, all_imports, stateful_imports) = build_template_imports(imports);

    let axes: Vec<serde_json::Value> = m.axes.iter().map(|a| {
        let var_name = match lang {
            crate::generator::Language::Go =>
                format!("axis{}", filters::to_pascal_case(a.input_id.clone())),
            _ => format!("AXIS_{}", a.input_id.to_uppercase()),
        };
        serde_json::json!({
            "input_id": a.input_id,
            "var_name": var_name,
            "breakpoints": a.breakpoints,
            "size": a.breakpoints.len(),
        })
    }).collect();

    let is_bilinear = m.method == InterpolationMethod::Bilinear;
    let rows = m.axes[0].breakpoints.len();
    let cols = if is_bilinear { m.axes[1].breakpoints.len() } else { 0 };

    ctx.insert("is_bilinear".into(), is_bilinear.into());
    ctx.insert("axes".into(), serde_json::json!(axes));
    ctx.insert("values".into(), serde_json::json!(m.values));
    ctx.insert("rows".into(), rows.into());
    ctx.insert("cols".into(), cols.into());
    ctx.insert("output_type".into(), l.type_name(&m.output.sce_type).into());
    ctx.insert("params".into(), l.param_str(&m.inputs).into());
    ctx.insert("out_of_bounds".into(), m.out_of_bounds.as_str().into());
    ctx.insert("has_imports".into(), has_imports.into());
    ctx.insert("imports".into(), serde_json::to_value(&stateful_imports).unwrap_or_default());
    ctx.insert("all_imports".into(), serde_json::to_value(&all_imports).unwrap_or_default());

    render_phase3(env, &format!("interpolation.{}.jinja2", l.template_ext()), ctx)
}

// ── Timer (unified) ───────────────────────────────────────────

fn render_timer(
    env: &minijinja::Environment,
    m: &TimerModel,
    imports: &[ImportContext],
    lang: crate::generator::Language,
) -> Result<String, String> {
    let l = Phase3Lang::new(lang);
    let mut ctx = l.base_context(&m.name);
    let (has_imports, all_imports, stateful_imports) = build_template_imports(imports);

    let timers: Vec<serde_json::Value> = m.timers.iter().map(|t| {
        let callback = t.event.as_deref()
            .or(t.on_timeout.as_deref())
            .unwrap_or(&t.id);
        serde_json::json!({
            "id": t.id,
            "id_pascal": filters::to_pascal_case(t.id.clone()),
            "id_snake": filters::to_snake_case(t.id.clone()),
            "timer_type": t.timer_type.as_str(),
            "time_ms": t.time_ms,
            "event": t.event,
            "on_timeout": t.on_timeout,
            "callback": callback,
            "callback_pascal": filters::to_pascal_case(callback.to_string()),
            "callback_snake": filters::to_snake_case(callback.to_string()),
            "is_periodic": t.timer_type == TimerType::Periodic,
        })
    }).collect();

    // Deduplicate callbacks: two timers may target the same handler method,
    // but the handler trait/concept lists each method exactly once. Insertion-
    // order preserved (BTreeMap keyed by encounter index) so output is stable.
    let mut seen = std::collections::BTreeSet::new();
    let unique_callbacks: Vec<serde_json::Value> = m.timers.iter()
        .filter_map(|t| {
            let callback = t.event.as_deref()
                .or(t.on_timeout.as_deref())
                .unwrap_or(&t.id)
                .to_string();
            if seen.insert(callback.clone()) {
                Some(serde_json::json!({
                    "callback": callback.clone(),
                    "callback_pascal": filters::to_pascal_case(callback.clone()),
                    "callback_snake": filters::to_snake_case(callback),
                }))
            } else {
                None
            }
        })
        .collect();

    ctx.insert("timers".into(), serde_json::json!(timers));
    ctx.insert("unique_callbacks".into(), serde_json::json!(unique_callbacks));
    ctx.insert("has_imports".into(), has_imports.into());
    ctx.insert("imports".into(), serde_json::to_value(&stateful_imports).unwrap_or_default());
    ctx.insert("all_imports".into(), serde_json::to_value(&all_imports).unwrap_or_default());

    render_phase3(env, &format!("timer.{}.jinja2", l.template_ext()), ctx)
}

// ── Observer (unified) ────────────────────────────────────────

fn render_observer(
    env: &minijinja::Environment,
    m: &ObserverModel,
    imports: &[ImportContext],
    lang: crate::generator::Language,
) -> Result<String, String> {
    let l = Phase3Lang::new(lang);
    let mut ctx = l.base_context(&m.name);
    let (has_imports, all_imports, stateful_imports) = build_template_imports(imports);

    let obs_type_ctx = crate::forge::type_ctx::observer(m, imports);
    let obs_empty_renames = std::collections::HashMap::new();

    let monitors: Vec<serde_json::Value> = m.monitors.iter().map(|mon| {
        let enter_expr = expr::transpile_typed(
            &mon.enter_expr,
            l.expr_target(),
            &obs_type_ctx,
            &obs_empty_renames,
            crate::forge::types::InferredType::Bool,
        )
        .unwrap_or_default();
        let leave_expr = mon.leave_expr.as_ref().map(|e| {
            expr::transpile_typed(
                e,
                l.expr_target(),
                &obs_type_ctx,
                &obs_empty_renames,
                crate::forge::types::InferredType::Bool,
            )
            .unwrap_or_default()
        });

        let active_var = match lang {
            crate::generator::Language::Cpp => format!("{}Active_", mon.id),
            crate::generator::Language::Kotlin => format!("{}Active", mon.id),
            crate::generator::Language::Go => format!("{}Active", mon.id),
            crate::generator::Language::Rust | crate::generator::Language::Python =>
                format!("{}_active", filters::to_snake_case(mon.id.clone())),
        };

        serde_json::json!({
            "id": mon.id,
            "active_var": active_var,
            "enter_expr": enter_expr,
            "leave_expr": leave_expr,
            "has_leave": mon.leave_expr.is_some(),
            "on_enter": mon.on_enter,
            "on_leave": mon.on_leave,
            "has_on_leave": mon.on_leave.is_some(),
            "event_enter": l.event_name(&mon.on_enter),
            "event_leave": mon.on_leave.as_ref().map(|s| l.event_name(s)),
        })
    }).collect();

    let mut events = Vec::new();
    for mon in &m.monitors {
        events.push(l.event_name(&mon.on_enter));
        if let Some(ref on_leave) = mon.on_leave {
            events.push(l.event_name(on_leave));
        }
    }

    ctx.insert("params".into(), l.param_str(&m.inputs).into());
    ctx.insert("monitors".into(), serde_json::json!(monitors));
    ctx.insert("events".into(), serde_json::json!(events));
    ctx.insert("has_event_domain".into(), m.event_domain.is_some().into());
    ctx.insert("event_domain".into(), serde_json::json!(m.event_domain));
    ctx.insert("has_imports".into(), has_imports.into());
    ctx.insert("imports".into(), serde_json::to_value(&stateful_imports).unwrap_or_default());
    ctx.insert("all_imports".into(), serde_json::to_value(&all_imports).unwrap_or_default());

    render_phase3(env, &format!("observer.{}.jinja2", l.template_ext()), ctx)
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

