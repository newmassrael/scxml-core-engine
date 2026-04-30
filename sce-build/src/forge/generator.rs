// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// SCE Forge code generator — renders kind-specific Jinja2 templates.
//
// Dispatches ForgeDocument to the appropriate template per kind and target
// language. Type mappings live here (not in the model) to preserve SRP.

use crate::filters;
use crate::forge::error::{ForgeError, GenerateError};
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

    /// For stateful kinds: member method signatures, keyed by qualified
    /// `"{alias}.{method}"` name. Each entry carries parameter types and a
    /// return type so that `infer_types` can propagate return types through
    /// member-call expressions like `frame.encode()`.
    ///
    /// Populated by `validate_and_enrich_imports` from the imported
    /// ForgeDocument's kind-specific method inventory (e.g. Codec →
    /// `encode()` returns `Bytes`). Only instance methods are registered
    /// here; static factory methods like `decode(raw)` are type-level calls
    /// and do not appear as `alias.method()` in user expressions.
    #[serde(skip)]
    pub member_method_sigs: Vec<(String, Vec<SceType>, SceType)>,
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
) -> Result<Vec<ImportContext>, ForgeError> {
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
) -> Result<(), ForgeError> {
    if matches!(lang, crate::generator::Language::Go) && !imports.is_empty() {
        match normalized_go_prefix(options) {
            None => {
                return Err(GenerateError::InvalidConfig(
                    "<sce:import> with language=go requires \
                     ForgeCompileOptions.go_module_prefix. Go module-qualified \
                     imports have no valid bare form; set this field to the \
                     go.mod module path that hosts the generated packages \
                     (e.g. \"github.com/acme/project/generated\")."
                        .to_string(),
                )
                .into());
            }
            Some(trimmed) if trimmed.is_empty() => {
                return Err(GenerateError::InvalidConfig(
                    "ForgeCompileOptions.go_module_prefix is empty; \
                     supply a non-empty Go module path such as \
                     \"github.com/acme/project/generated\"."
                        .to_string(),
                )
                .into());
            }
            Some(trimmed) if trimmed.chars().any(char::is_whitespace) => {
                let raw = options.go_module_prefix.as_deref().unwrap_or("");
                return Err(GenerateError::InvalidConfig(format!(
                    "ForgeCompileOptions.go_module_prefix {raw:?} \
                     contains whitespace; Go import paths may not \
                     contain spaces or tabs."
                ))
                .into());
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
                member_method_sigs: Vec::new(),
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
                member_method_sigs: Vec::new(),
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
                member_method_sigs: Vec::new(),
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
                member_method_sigs: Vec::new(),
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
                member_method_sigs: Vec::new(),
            }
        }
        crate::generator::Language::C11 => {
            // RFC §5.J.1: C11 cross-file imports use plain `#include "<snake>.h"`.
            // No namespace concept exists; the module name is encoded as a
            // function prefix at every callsite (see `build_qualified_call`).
            // The shape mirrors C++ but routes through the M2+ C11 emitter.
            ImportContext {
                alias: imp.alias.clone(),
                kind: imp.kind.to_string(),
                include_stmt: format!("#include \"{snake}.h\""),
                type_name: pascal.clone(),
                is_stateful,
                member_name: format!("{}_", imp.alias),
                member_type: snake.clone(),
                namespace: snake.clone(),
                qualified_call: String::new(),
                param_types: Vec::new(),
                ret_type: None,
                member_field_types: Vec::new(),
                member_method_sigs: Vec::new(),
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

/// Map SceType to C11 type name (RFC §5.J.2 F2). All types are stdint
/// fixed-width integers, plain `bool` (from `<stdbool.h>`), or IEEE
/// `float`/`double`. String/Bytes are out of scope for Phase A — the
/// transform fixtures do not exercise them; Phase B's codec arms add
/// the heap-free byte-array handling.
fn c_type(ty: &SceType) -> &'static str {
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
        // String / Bytes flow through Phase B+ (codec & condition kinds).
        // Returning a stable placeholder lets the match be exhaustive
        // without adding a panic site — the Phase-A transform pipeline
        // never reaches these arms because the fixture set is purely
        // numeric.
        SceType::String => "const char *",
        SceType::Bytes => "const uint8_t *",
    }
}

/// C11 parameter type. For Phase A's numeric-only transform fixtures
/// this is the same as `c_type` — strings and bytes (which would need
/// length-paired pointer pairs) are deferred to Phase B+.
fn c_param_type(ty: &SceType) -> &'static str {
    c_type(ty)
}

/// C11 literal formatter. Mirrors `cpp_literal` exactly for the shape
/// Phase A exercises (decimal-integer-to-float `.0` promotion, `f`
/// suffix for Float32). C and C++ accept the same literal grammar at
/// this level.
fn c_literal(text: &str, ty: &SceType) -> String {
    match ty {
        SceType::Float32 if looks_like_int(text) => format!("{text}.0f"),
        SceType::Float32 => format!("{text}f"),
        SceType::Float64 if looks_like_int(text) => format!("{text}.0"),
        _ => text.to_string(),
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

// ── Runtime dependency annotation ────────────────────────────────

/// Inject `runtime_dep` into the Jinja2 environment so every template can
/// reference `{{ runtime_dep }}` in its header comment.
fn inject_runtime_dep_global(env: &mut minijinja::Environment, doc: &ForgeDocument) {
    env.add_global("runtime_dep", doc.runtime_dep().to_string());
}

// ── Public API ─────────────────────────────────────────────────

/// Generate code from a ForgeDocument for C++ using Jinja2 templates.
pub fn generate_cpp(doc: &ForgeDocument, template_dir: &Path) -> Result<GeneratedOutput, ForgeError> {
    generate_cpp_with_imports(doc, template_dir, &[])
}

/// Generate C++ code with cross-file import support.
pub fn generate_cpp_with_imports(
    doc: &ForgeDocument,
    template_dir: &Path,
    imports: &[ImportContext],
) -> Result<GeneratedOutput, ForgeError> {
    let forge_dir = template_dir.join("forge/cpp");
    let mut env = generator::new_env();
    generator::load_templates(&mut env, &forge_dir)?;
    inject_runtime_dep_global(&mut env, doc);

    let code = match doc {
        ForgeDocument::Transform(m) => render_transform(&env, m, imports, crate::generator::Language::Cpp)?,
        ForgeDocument::Lookup(m) => render_lookup(&env, m, imports, crate::generator::Language::Cpp)?,
        ForgeDocument::Condition(m) => render_condition(&env, m, imports, crate::generator::Language::Cpp)?,
        ForgeDocument::Codec(m) => render_codec(&env, m, imports, crate::generator::Language::Cpp)?,
        ForgeDocument::Validator(m) => render_validator(&env, m, imports, crate::generator::Language::Cpp)?,
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

// ── Transform rendering (unified) ─────────────────────────────

fn render_transform(
    env: &minijinja::Environment,
    m: &TransformModel,
    imports: &[ImportContext],
    lang: crate::generator::Language,
) -> Result<String, ForgeError> {
    use crate::generator::Language;
    let l = LangCtx::new(lang);

    let go_renames = l.go_rename_pairs(m.inputs.iter().map(|f| f.id.as_str()));
    let renames = rename_map(&go_renames);

    let type_ctx = crate::forge::type_ctx::transform(m, imports);
    let params = l.param_str(&m.inputs);

    let functions: Vec<serde_json::Value> = m
        .outputs
        .iter()
        .map(|out| {
            let expected = crate::forge::types::InferredType::from_sce_type(&out.sce_type);
            let expr_val = expr::transpile_typed(
                out.expr.as_deref().unwrap_or("0"),
                l.expr_target(),
                &type_ctx,
                &renames,
                expected,
            )?;

            let fn_name = match lang {
                Language::Go =>
                    format!("Compute{}", filters::to_pascal_case(out.id.clone())),
                Language::Rust | Language::Python =>
                    format!("compute_{}", filters::to_snake_case(out.id.clone())),
                // RFC §5.J.2 §3 D1 (mirroring Lookup): C11 has a flat scope,
                // so fully-qualify the exported function with `<m.name>_` to
                // keep two transforms whose output ids collide (e.g. both
                // `temperature`) from clashing in a single TU. This also
                // matches what `build_qualified_call` produces at every
                // cross-file callsite (`{namespace}_{discover_primary_function}`),
                // so `crossfile_validator_transform` and any other future
                // C11 transform import resolves to the same symbol the
                // generated header declares.
                Language::C11 =>
                    format!(
                        "{}_compute_{}",
                        filters::to_snake_case(m.name.clone()),
                        filters::to_snake_case(out.id.clone()),
                    ),
                _ =>
                    format!("compute{}", filters::to_pascal_case(out.id.clone())),
            };

            let mut obj = serde_json::Map::new();
            obj.insert("ret_type".into(), l.type_name(&out.sce_type).into());
            obj.insert("name".into(), fn_name.into());
            obj.insert("params".into(), params.clone().into());
            obj.insert("expr".into(), expr_val.into());
            if matches!(lang, Language::Go) {
                obj.insert("orig_name".into(), out.id.clone().into());
            }

            Ok(serde_json::Value::Object(obj))
        })
        .collect::<Result<_, ForgeError>>()?;

    let mut ctx = l.base_context(&m.name);
    ctx.insert("functions".into(), serde_json::json!(functions));
    l.insert_imports(&mut ctx, imports);

    l.render(env, "transform", ctx)
}

// ── Lookup rendering (unified) ────────────────────────────────

fn render_lookup(
    env: &minijinja::Environment,
    m: &LookupModel,
    imports: &[ImportContext],
    lang: crate::generator::Language,
) -> Result<String, ForgeError> {
    use crate::generator::Language;
    let l = LangCtx::new(lang);

    let enum_name = filters::to_pascal_case(m.output.id.clone());
    let func_name = match lang {
        Language::Go =>
            format!("Lookup{}", filters::to_pascal_case(m.output.id.clone())),
        Language::Rust | Language::Python =>
            format!("lookup_{}", filters::to_snake_case(m.output.id.clone())),
        // RFC §5.J.2 §3 D1: C11 has a flat scope, so fully-qualify with the
        // fixture name to keep two lookups whose output ids collide
        // (e.g. both `status`) from clashing in a single TU.
        Language::C11 =>
            format!(
                "{}_{}",
                filters::to_snake_case(m.name.clone()),
                filters::to_snake_case(m.output.id.clone()),
            ),
        _ =>
            format!("lookup{}", filters::to_pascal_case(m.output.id.clone())),
    };
    let input_id = l.local_id(&m.input.id);

    let output_is_string = m.output_is_string();
    let on_miss_error = m.miss_policy.is_error();

    // String-enum strategy: entries grouped by output value.
    let (entries_by_value, unique_values, default_value) = if output_is_string {
        let raw_ebv = m.entries_by_value();

        let ebv: Vec<serde_json::Value> = match lang {
            Language::Python => {
                // Python template expects a `condition` expression per group.
                raw_ebv.into_iter().map(|(value, keys)| {
                    let condition = if keys.len() == 1 {
                        format!("{input_id} == {}", keys[0])
                    } else {
                        format!("{input_id} in ({})", keys.join(", "))
                    };
                    serde_json::json!({"value": value, "condition": condition})
                }).collect()
            }
            Language::Rust => {
                raw_ebv.into_iter().map(|(value, keys)| {
                    serde_json::json!({"value": to_rust_variant(&value), "keys": keys})
                }).collect()
            }
            _ => {
                raw_ebv.into_iter()
                    .map(|(value, keys)| serde_json::json!({"value": value, "keys": keys}))
                    .collect()
            }
        };

        let uv: Vec<String> = match lang {
            Language::Rust => m.unique_values().into_iter().map(|v| to_rust_variant(&v)).collect(),
            _ => m.unique_values(),
        };

        let dv = match &m.miss_policy {
            MissPolicy::Default(s) => match lang {
                Language::Rust => to_rust_variant(s),
                _ => s.clone(),
            },
            MissPolicy::Error => String::new(),
        };
        (ebv, uv, dv)
    } else {
        (Vec::new(), Vec::new(), String::new())
    };

    // Numeric strategy: parallel key/value arrays with language-specific literals.
    let (keys_literal, values_literal, default_literal) = if !output_is_string {
        let kl: Vec<String> = m.entries.iter()
            .map(|e| l.literal(&e.key, &m.input.sce_type))
            .collect();
        let vl: Vec<String> = m.entries.iter()
            .map(|e| l.literal(&e.value, &m.output.sce_type))
            .collect();
        let dl = match &m.miss_policy {
            MissPolicy::Default(s) => l.literal(s, &m.output.sce_type),
            MissPolicy::Error => String::new(),
        };
        (kl, vl, dl)
    } else {
        (Vec::new(), Vec::new(), String::new())
    };

    let mut ctx = l.base_context(&m.name);
    ctx.insert("enum_name".into(), enum_name.into());
    ctx.insert("func_name".into(), func_name.clone().into());
    ctx.insert("input_type".into(), l.param_type(&m.input.sce_type).into());
    ctx.insert("value_type".into(), l.param_type(&m.output.sce_type).into());
    ctx.insert("input_id".into(), input_id.into());
    ctx.insert("unique_values".into(), serde_json::json!(unique_values));
    ctx.insert("entries_by_value".into(), serde_json::json!(entries_by_value));
    ctx.insert("default_value".into(), default_value.into());
    ctx.insert("default_literal".into(), default_literal.into());
    ctx.insert("output_is_string".into(), output_is_string.into());
    ctx.insert("on_miss_error".into(), on_miss_error.into());
    ctx.insert("keys_literal".into(), serde_json::json!(keys_literal));
    ctx.insert("values_literal".into(), serde_json::json!(values_literal));
    ctx.insert("n".into(), m.entries.len().into());

    // Kotlin-specific: unsigned-to-signed conversion for when-match expressions.
    if matches!(lang, Language::Kotlin) {
        let match_suffix = match kotlin_unsigned_conversion(&m.input.sce_type) {
            Some(conv) => format!(".{conv}()"),
            None => String::new(),
        };
        ctx.insert("match_suffix".into(), match_suffix.into());
    }

    // C11 (RFC §5.J.2 §3 D1): fully-qualified flat-scope identifiers derived
    // from `func_name` (already `<m.name>_<output_id>`). Variant prefix is
    // its UPPER_SNAKE form; sibling helpers / arrays append a stable suffix.
    // Variant list and value-name switch arms are joined here rather than
    // inside the template — minijinja's trim_blocks collapses inline
    // `{% endif %}` newlines and would emit all variants on one line.
    if matches!(lang, Language::C11) {
        let prefix = to_upper_snake(&func_name);
        let variants_block: String = unique_values
            .iter()
            .map(|v| format!("    {prefix}_{v}"))
            .collect::<Vec<_>>()
            .join(",\n");
        let value_name_arms: String = unique_values
            .iter()
            .map(|v| format!("        case {prefix}_{v}: return \"{v}\";"))
            .collect::<Vec<_>>()
            .join("\n");
        ctx.insert("c_typedef_name".into(), format!("{func_name}_t").into());
        ctx.insert("c_variant_prefix".into(), prefix.into());
        ctx.insert("c_value_name_func".into(), format!("{func_name}_name").into());
        ctx.insert("c_keys_array_name".into(), format!("{func_name}_keys").into());
        ctx.insert("c_values_array_name".into(), format!("{func_name}_values").into());
        ctx.insert("c_variants_block".into(), variants_block.into());
        ctx.insert("c_value_name_arms".into(), value_name_arms.into());
    }

    l.insert_imports(&mut ctx, imports);
    l.render(env, "lookup", ctx)
}

// ── Condition rendering (unified) ─────────────────────────────

fn render_condition(
    env: &minijinja::Environment,
    m: &ConditionModel,
    imports: &[ImportContext],
    lang: crate::generator::Language,
) -> Result<String, ForgeError> {
    use crate::generator::Language;
    let l = LangCtx::new(lang);

    let go_renames = l.go_rename_pairs(m.inputs.iter().map(|f| f.id.as_str()));
    let renames = rename_map(&go_renames);

    let func_name = match lang {
        Language::Go => filters::to_pascal_case(m.name.clone()),
        Language::Rust | Language::Python | Language::C11 =>
            filters::to_snake_case(m.name.clone()),
        _ => filters::to_camel_case(m.name.clone()),
    };

    let params = l.param_str(&m.inputs);

    let type_ctx = crate::forge::type_ctx::condition(m, imports);
    let expr_val = expr::transpile_typed(
        &m.expr,
        l.expr_target(),
        &type_ctx,
        &renames,
        crate::forge::types::InferredType::Bool,
    )?;

    let mut ctx = l.base_context(&m.name);
    ctx.insert("func_name".into(), func_name.into());
    ctx.insert("params".into(), params.into());
    ctx.insert("expr".into(), expr_val.into());
    l.insert_imports(&mut ctx, imports);

    l.render(env, "condition", ctx)
}

// ── Codec rendering (unified) ─────────────────────────────────

fn render_codec(
    env: &minijinja::Environment,
    m: &CodecModel,
    imports: &[ImportContext],
    lang: crate::generator::Language,
) -> Result<String, ForgeError> {
    let l = LangCtx::new(lang);
    let type_key = l.codec_type_key();

    let fields: Vec<serde_json::Value> = m
        .fields
        .iter()
        .map(|f| {
            let mut obj = serde_json::Map::new();
            obj.insert("id".into(), l.codec_field_id(&f.id).into());
            obj.insert(type_key.into(), l.type_name(&f.sce_type).into());
            obj.insert("decode_expr".into(), generate_decode_expr(f, m.default_endian, lang).into());
            if matches!(lang, crate::generator::Language::Kotlin) {
                obj.insert("kt_default".into(), kotlin_default(&f.sce_type).into());
            }
            if matches!(lang, crate::generator::Language::Python) {
                obj.insert("default_value".into(), python_default(&f.sce_type).into());
            }
            serde_json::Value::Object(obj)
        })
        .collect();

    let encode_exprs = generate_encode_exprs(&m.fields, m.default_endian, lang);

    let mut ctx = l.base_context(&m.name);
    ctx.insert("fields".into(), serde_json::json!(fields));
    ctx.insert("min_bytes".into(), m.min_frame_bytes().into());
    ctx.insert("encode_exprs".into(), serde_json::json!(encode_exprs));

    // C11 (RFC §5.J.2 §3 D2): full-qual flat-scope identifiers.
    // Decode = α (`bool fn(raw, len, *out)`); encode = β (return-by-value
    // `<name>_encoded_t { bytes[MAX]; len }`). MAX collapses to MIN for
    // fixed-only fixtures (Phase B-3 set); Tail/LengthRef fixtures will
    // surface a max-side computation when their first fixture lands.
    if matches!(lang, crate::generator::Language::C11) {
        let snake = filters::to_snake_case(m.name.clone());
        let upper = to_upper_snake(&m.name);
        ctx.insert("c_struct_typedef".into(), format!("{snake}_t").into());
        ctx.insert("c_encoded_typedef".into(), format!("{snake}_encoded_t").into());
        ctx.insert("c_decode_func".into(), format!("{snake}_decode").into());
        ctx.insert("c_encode_func".into(), format!("{snake}_encode").into());
        ctx.insert("c_max_bytes_macro".into(), format!("{upper}_MAX_BYTES").into());
        ctx.insert("c_min_bytes_macro".into(), format!("{upper}_MIN_BYTES").into());
    }

    l.insert_imports(&mut ctx, imports);

    l.render(env, "codec", ctx)
}

// ── Codec expression generation (unified) ─────────────────────

/// Generate decode expression for a single codec field.
fn generate_decode_expr(
    field: &CodecField,
    default_endian: Endian,
    lang: crate::generator::Language,
) -> String {
    use crate::generator::Language;
    let byte_off = field.byte_offset;
    let bit_off = field.bit_offset.unwrap_or(0);
    let endian = field.effective_endian(default_endian);

    match &field.bit_size {
        BitSize::Fixed { bits } => {
            if bit_off > 0 || *bits < 8 {
                let mask = (1u64 << bits) - 1;
                match lang {
                    Language::Cpp =>
                        format!("static_cast<uint8_t>((raw[{byte_off}] >> {bit_off}) & 0x{mask:02X})"),
                    Language::Kotlin =>
                        format!("((raw[{byte_off}].toInt() ushr {bit_off}) and 0x{mask:02X}).toUByte()"),
                    Language::C11 =>
                        format!("(uint8_t)((raw[{byte_off}] >> {bit_off}) & 0x{mask:02X})"),
                    _ =>
                        format!("(raw[{byte_off}] >> {bit_off}) & 0x{mask:02X}"),
                }
            } else {
                match bits {
                    8 => match lang {
                        Language::Kotlin => format!("raw[{byte_off}].toUByte()"),
                        _ => format!("raw[{byte_off}]"),
                    },
                    16 => decode_multibyte_unified(byte_off, 2, endian, lang),
                    24 => decode_multibyte_unified(byte_off, 3, endian, lang),
                    32 => decode_multibyte_unified(byte_off, 4, endian, lang),
                    _ => match lang {
                        Language::Python => format!("# unsupported {bits}-bit decode"),
                        _ => format!("/* unsupported {bits}-bit decode */"),
                    },
                }
            }
        }
        BitSize::Tail => match lang {
            Language::Cpp =>
                format!("std::vector<uint8_t>(raw + {byte_off}, raw + len)"),
            Language::Kotlin =>
                format!("raw.copyOfRange({byte_off}, raw.size)"),
            Language::Rust =>
                format!("raw[{byte_off}..].to_vec()"),
            Language::Go | Language::Python =>
                format!("raw[{byte_off}:]"),
            Language::C11 => unimplemented!(
                "C11 codec BitSize::Tail emitter is RFC \u{00A7}5.J.1 M3+ work \
                 (codec DSL emitter follows lookup vertical slice)"
            ),
        },
        BitSize::LengthRef => {
            let len_field = field.length_field.as_deref().unwrap_or("0");
            match lang {
                Language::Cpp =>
                    format!("std::vector<uint8_t>(raw + {byte_off}, raw + {byte_off} + {len_field})"),
                Language::Kotlin =>
                    format!("raw.copyOfRange({byte_off}, {byte_off} + {len_field}.toInt())"),
                Language::Rust =>
                    format!("raw[{byte_off}..{byte_off} + {len_field} as usize].to_vec()"),
                Language::Go =>
                    format!("raw[{byte_off}:{byte_off}+int({len_field})]"),
                Language::Python =>
                    format!("raw[{byte_off}:{byte_off} + {len_field}]"),
                Language::C11 => unimplemented!(
                    "C11 codec BitSize::LengthRef emitter is RFC \u{00A7}5.J.1 M3+ work"
                ),
            }
        }
    }
}

/// Generate multi-byte decode expression with endianness handling.
fn decode_multibyte_unified(
    byte_off: u32,
    byte_count: u32,
    endian: Endian,
    lang: crate::generator::Language,
) -> String {
    use crate::generator::Language;

    // Build shift expressions for the appropriate endian ordering.
    let make_shifts = |le: bool| -> Vec<String> {
        (0..byte_count)
            .map(|i| {
                let shift = if le { i * 8 } else { (byte_count - 1 - i) * 8 };
                let off = byte_off + i;
                match lang {
                    Language::Cpp => {
                        let target = match byte_count { 2 => "uint16_t", 3 | 4 => "uint32_t", _ => "uint64_t" };
                        if shift == 0 { format!("raw[{off}]") }
                        else { format!("(static_cast<{target}>(raw[{off}]) << {shift})") }
                    }
                    Language::Kotlin => {
                        if shift == 0 { format!("(raw[{off}].toInt() and 0xFF)") }
                        else { format!("((raw[{off}].toInt() and 0xFF) shl {shift})") }
                    }
                    Language::Rust => {
                        let target = match byte_count { 2 => "u16", 3 | 4 => "u32", _ => "u64" };
                        if shift == 0 { format!("raw[{off}] as {target}") }
                        else { format!("((raw[{off}] as {target}) << {shift})") }
                    }
                    Language::Go => {
                        let target = match byte_count { 2 => "uint16", 3 | 4 => "uint32", _ => "uint64" };
                        if shift == 0 { format!("{target}(raw[{off}])") }
                        else { format!("{target}(raw[{off}])<<{shift}") }
                    }
                    Language::Python => {
                        if shift == 0 { format!("raw[{off}]") }
                        else { format!("(raw[{off}] << {shift})") }
                    }
                    Language::C11 => {
                        let target = match byte_count { 2 => "uint16_t", 3 | 4 => "uint32_t", _ => "uint64_t" };
                        if shift == 0 { format!("raw[{off}]") }
                        else { format!("(({target})raw[{off}] << {shift})") }
                    }
                }
            })
            .collect()
    };

    let shifts = match endian {
        Endian::Big | Endian::Native => make_shifts(false),
        Endian::Little => make_shifts(true),
    };

    let sep = match lang {
        Language::Kotlin => " or ",
        _ => " | ",
    };

    let joined = shifts.join(sep);

    // Kotlin wraps in conversion call.
    if matches!(lang, Language::Kotlin) {
        let to_type = match byte_count {
            2 => "toUShort",
            3 | 4 => "toUInt",
            _ => "toULong",
        };
        format!("({joined}).{to_type}()")
    } else {
        joined
    }
}

/// Generate encode byte expressions for all codec fields.
fn generate_encode_exprs(
    fields: &[CodecField],
    default_endian: Endian,
    lang: crate::generator::Language,
) -> Vec<String> {
    let l = LangCtx::new(lang);
    let mut exprs = Vec::new();

    let mut byte_groups: std::collections::BTreeMap<u32, Vec<&CodecField>> =
        std::collections::BTreeMap::new();

    for field in fields {
        if field.is_variable_length() {
            exprs.push(l.codec_comment(
                &format!("variable-length field '{}' requires manual encode", field.id)
            ));
        } else {
            byte_groups.entry(field.byte_offset).or_default().push(field);
        }
    }

    for (_, group) in &byte_groups {
        if group.len() == 1 {
            encode_single_field_unified(group[0], default_endian, &mut exprs, lang);
        } else {
            let mut parts = Vec::new();
            for field in group {
                let bit_off = field.bit_offset.unwrap_or(0);
                let bits = field.fixed_bits().unwrap_or(8);
                let mask = (1u64 << bits) - 1;
                let field_ref = l.codec_field_ref(&l.codec_field_id(&field.id));
                match lang {
                    crate::generator::Language::Kotlin =>
                        parts.push(format!("({field_ref}.toInt() and 0x{mask:02X} shl {bit_off})")),
                    crate::generator::Language::Cpp
                    | crate::generator::Language::Rust
                    | crate::generator::Language::C11 =>
                        parts.push(format!("(({field_ref} & 0x{mask:02X}) << {bit_off})")),
                    _ =>
                        parts.push(format!("({field_ref} & 0x{mask:02X}) << {bit_off}")),
                }
            }
            let sep = match lang { crate::generator::Language::Kotlin => " or ", _ => " | " };
            let merged = parts.join(sep);
            exprs.push(l.codec_to_byte(&merged));
        }
    }

    exprs
}

/// Generate encode expressions for a single non-sub-byte field.
fn encode_single_field_unified(
    field: &CodecField,
    default_endian: Endian,
    exprs: &mut Vec<String>,
    lang: crate::generator::Language,
) {
    use crate::generator::Language;
    let l = LangCtx::new(lang);
    let name = l.codec_field_id(&field.id);
    let field_ref = l.codec_field_ref(&name);
    let bit_off = field.bit_offset.unwrap_or(0);
    let endian = field.effective_endian(default_endian);

    match field.fixed_bits() {
        Some(8) if bit_off == 0 => {
            match lang {
                Language::Cpp => exprs.push(field_ref),
                Language::Kotlin => exprs.push(format!("{field_ref}.toByte()")),
                Language::Rust => exprs.push(field_ref),
                Language::Go => exprs.push(format!("byte({field_ref})")),
                Language::Python => exprs.push(format!("{field_ref} & 0xFF")),
                // C11 (β encode shape): field_ref already includes `self->`,
                // and the value is a uint8_t so no width cast is required.
                Language::C11 => exprs.push(field_ref),
            }
        }
        Some(bits) if bits < 8 || bit_off > 0 => {
            let mask = (1u64 << bits) - 1;
            let inner = match lang {
                Language::Kotlin =>
                    format!("{field_ref}.toInt() and 0x{mask:02X} shl {bit_off}"),
                _ =>
                    format!("({field_ref} & 0x{mask:02X}) << {bit_off}"),
            };
            exprs.push(l.codec_to_byte(&inner));
        }
        Some(byte_count @ (16 | 24 | 32)) => {
            let n_bytes = byte_count / 8;
            let shifts: Vec<u32> = match endian {
                Endian::Big | Endian::Native => (0..n_bytes).rev().collect(),
                Endian::Little => (0..n_bytes).collect(),
            };
            for shift_byte in shifts {
                let shift = shift_byte * 8;
                let expr = match lang {
                    Language::Cpp => {
                        if shift == 0 {
                            format!("static_cast<uint8_t>({field_ref} & 0xFF)")
                        } else {
                            format!("static_cast<uint8_t>(({field_ref} >> {shift}) & 0xFF)")
                        }
                    }
                    Language::Kotlin => {
                        if shift == 0 {
                            format!("({field_ref}.toInt() and 0xFF).toByte()")
                        } else {
                            format!("({field_ref}.toInt() ushr {shift} and 0xFF).toByte()")
                        }
                    }
                    Language::Rust => {
                        if shift == 0 {
                            format!("(self.{name} & 0xFF) as u8")
                        } else {
                            format!("(self.{name} >> {shift} & 0xFF) as u8")
                        }
                    }
                    Language::Go => {
                        if shift == 0 {
                            format!("byte(s.{name} & 0xFF)")
                        } else {
                            format!("byte(s.{name} >> {shift} & 0xFF)")
                        }
                    }
                    Language::Python => {
                        if shift == 0 {
                            format!("self.{name} & 0xFF")
                        } else {
                            format!("(self.{name} >> {shift}) & 0xFF")
                        }
                    }
                    Language::C11 => {
                        if shift == 0 {
                            format!("(uint8_t)(self->{name} & 0xFF)")
                        } else {
                            format!("(uint8_t)((self->{name} >> {shift}) & 0xFF)")
                        }
                    }
                };
                exprs.push(expr);
            }
        }
        _ => exprs.push(l.codec_comment(&format!("encode {name}"))),
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

fn resolve_validator(m: &ValidatorModel) -> Result<ResolvedValidator, ForgeError> {
    let available_ids: Vec<&str> = m.inputs.iter().map(|f| f.id.as_str()).collect();

    let mut ranges = Vec::new();
    for r in &m.rules.ranges {
        let field = m.inputs.iter().find(|f| f.id == r.id).ok_or_else(|| {
            ForgeError::Validation(crate::forge::error::ValidationError::InvalidReference {
                kind: crate::forge::model::ForgeKind::Validator,
                name: r.id.clone(),
                what: "input field for range rule".into(),
                available: available_ids.join(", "),
            })
        })?;
        ranges.push(ResolvedRange {
            id: r.id.clone(),
            sce_type: field.sce_type.clone(),
            min: r.min.clone(),
            max: r.max.clone(),
        });
    }

    let mut rocs = Vec::new();
    for roc in &m.rules.rate_of_changes {
        let field = m.inputs.iter().find(|f| f.id == roc.id).ok_or_else(|| {
            ForgeError::Validation(crate::forge::error::ValidationError::InvalidReference {
                kind: crate::forge::model::ForgeKind::Validator,
                name: roc.id.clone(),
                what: "input field for rate-of-change rule".into(),
                available: available_ids.join(", "),
            })
        })?;
        rocs.push(ResolvedRoc {
            id: roc.id.clone(),
            sce_type: field.sce_type.clone(),
            max_delta: roc.max_delta.clone(),
        });
    }

    Ok(ResolvedValidator {
        inputs: m.inputs.clone(),
        ranges,
        rocs,
        plausibility: m.rules.plausibility.clone(),
    })
}

// ── Validator rendering (unified) ────────────────────────────

fn render_validator(
    env: &minijinja::Environment,
    m: &ValidatorModel,
    imports: &[ImportContext],
    lang: crate::generator::Language,
) -> Result<String, ForgeError> {
    use crate::generator::Language;
    let l = LangCtx::new(lang);
    let rv = resolve_validator(m)?;

    let params = l.param_str(&rv.inputs);

    // prev_vars: superset of all per-language fields.
    let prev_vars: Vec<serde_json::Value> = rv.rocs.iter()
        .map(|roc| {
            let local = l.local_id(&roc.id);
            let ty_str = l.type_name(&roc.sce_type);
            let mut obj = serde_json::Map::new();
            obj.insert("type".into(), ty_str.into());
            obj.insert("name".into(), l.prev_name(&roc.id).into());
            obj.insert("id".into(), local.into());
            obj.insert("is_float".into(), roc.sce_type.is_float().into());
            if matches!(lang, Language::Kotlin) {
                obj.insert("default".into(), kotlin_default_value(ty_str).into());
            }
            serde_json::Value::Object(obj)
        })
        .collect();

    // range_rules: `reason_id` from single source of truth (ResolvedRange).
    let range_rules: Vec<serde_json::Value> = rv.ranges.iter()
        .map(|r| {
            let mut obj = serde_json::Map::new();
            obj.insert("id".into(), l.local_id(&r.id).into());
            obj.insert("reason_id".into(), r.canonical_reason_id().into());
            obj.insert("min".into(), serde_json::json!(r.min));
            obj.insert("max".into(), serde_json::json!(r.max));
            obj.insert("has_min".into(), r.min.is_some().into());
            obj.insert("has_max".into(), r.max.is_some().into());
            // Unsigned typing flag — needed by the C template to elide
            // lower-bound checks where `min == "0"` and the field type is
            // unsigned, since `unsigned < 0` is tautologically false and
            // gcc -Wtype-limits would surface a -Werror in the C11 build.
            // cpp/Rust/Go/Kotlin/Python builds either don't run -Werror
            // here or don't carry an equivalent diagnostic, so they emit
            // the same redundant comparison as before.
            obj.insert("is_unsigned".into(), r.sce_type.is_unsigned().into());
            if matches!(lang, Language::Kotlin) {
                let conv = kotlin_unsigned_conversion(&r.sce_type).unwrap_or("");
                obj.insert("conv".into(), conv.into());
                obj.insert("needs_conv".into(), (!conv.is_empty()).into());
            }
            serde_json::Value::Object(obj)
        })
        .collect();

    // roc_rules: superset of per-language fields; Kotlin conv folded in.
    let roc_rules: Vec<serde_json::Value> = rv.rocs.iter()
        .map(|roc| {
            let local = l.local_id(&roc.id);
            let mut obj = serde_json::Map::new();
            obj.insert("id".into(), local.into());
            obj.insert("reason_id".into(), roc.canonical_reason_id().into());
            obj.insert("max_delta".into(), roc.max_delta.clone().into());
            obj.insert("prev_name".into(), l.prev_name(&roc.id).into());
            obj.insert("type".into(), l.type_name(&roc.sce_type).into());
            obj.insert("is_float".into(), roc.sce_type.is_float().into());
            obj.insert("is_unsigned".into(), roc.sce_type.is_unsigned().into());
            obj.insert("is_signed".into(), roc.sce_type.is_signed().into());
            if matches!(lang, Language::Kotlin) {
                let conv = kotlin_unsigned_conversion(&roc.sce_type).unwrap_or("");
                obj.insert("conv".into(), conv.into());
                obj.insert("needs_conv".into(), (!conv.is_empty()).into());
            }
            serde_json::Value::Object(obj)
        })
        .collect();

    // Build import alias rename map for expressions (stateless → qualified call).
    let import_renames: std::collections::HashMap<&str, &str> = imports
        .iter()
        .filter(|i| !i.is_stateful && !i.qualified_call.is_empty())
        .map(|i| (i.alias.as_str(), i.qualified_call.as_str()))
        .collect();

    // Go: merge import renames with builtin escape renames.
    let go_renames = l.go_rename_pairs(rv.inputs.iter().map(|f| f.id.as_str()));
    let mut combined_renames = rename_map(&go_renames);
    for (k, v) in &import_renames {
        combined_renames.insert(*k, *v);
    }
    let expr_renames = if matches!(lang, Language::Go) {
        &combined_renames
    } else {
        &import_renames
    };

    let type_ctx = crate::forge::type_ctx::validator(m, imports);
    let plausibility_expr = match &rv.plausibility {
        Some(e) => Some(expr::transpile_typed(
            e,
            l.expr_target(),
            &type_ctx,
            expr_renames,
            crate::forge::types::InferredType::Bool,
        )?),
        None => None,
    };

    let mut ctx = l.base_context(&m.name);
    ctx.insert("params".into(), params.into());
    ctx.insert("prev_vars".into(), serde_json::json!(prev_vars));
    ctx.insert("range_rules".into(), serde_json::json!(range_rules));
    ctx.insert("roc_rules".into(), serde_json::json!(roc_rules));
    ctx.insert("plausibility_expr".into(), serde_json::json!(plausibility_expr));

    // C11 (RFC §5.J.2 §3 Phase C V1b): per-fixture flat-scope typedef + V2c
    // mixed calling convention. Stateless validators (no rocs) emit a free
    // function `<snake>_validate(args)`; stateful validators emit a state
    // struct + pointer-passing `<snake>_validate(<snake>_t *self, args)`,
    // mirroring the cpp shape but in C-idiomatic form (no member functions,
    // no zero-field structs which would violate -Wpedantic).
    if matches!(lang, Language::C11) {
        let snake = filters::to_snake_case(m.name.clone());
        ctx.insert("c_result_typedef".into(), format!("{snake}_result_t").into());
        ctx.insert("c_state_typedef".into(), format!("{snake}_t").into());
        ctx.insert("c_validate_func".into(), format!("{snake}_validate").into());
        ctx.insert("c_has_state".into(), (!prev_vars.is_empty()).into());
    }

    l.insert_imports(&mut ctx, imports);

    l.render(env, "validator", ctx)
}

// ══════════════════════════════════════════════════════════════
// ── Kotlin code generation ────────────────────────────────────
// ══════════════════════════════════════════════════════════════

/// Generate code from a ForgeDocument for Kotlin using Jinja2 templates.
pub fn generate_kotlin(doc: &ForgeDocument, template_dir: &Path) -> Result<GeneratedOutput, ForgeError> {
    generate_kotlin_with_imports(doc, template_dir, &[])
}

/// Generate Kotlin code with cross-file import support.
pub fn generate_kotlin_with_imports(
    doc: &ForgeDocument,
    template_dir: &Path,
    imports: &[ImportContext],
) -> Result<GeneratedOutput, ForgeError> {
    let forge_dir = template_dir.join("forge/kotlin");
    let mut env = generator::new_env();
    generator::load_templates(&mut env, &forge_dir)?;
    inject_runtime_dep_global(&mut env, doc);

    let code = match doc {
        ForgeDocument::Transform(m) => render_transform(&env, m, imports, crate::generator::Language::Kotlin)?,
        ForgeDocument::Lookup(m) => render_lookup(&env, m, imports, crate::generator::Language::Kotlin)?,
        ForgeDocument::Condition(m) => render_condition(&env, m, imports, crate::generator::Language::Kotlin)?,
        ForgeDocument::Codec(m) => render_codec(&env, m, imports, crate::generator::Language::Kotlin)?,
        ForgeDocument::Validator(m) => render_validator(&env, m, imports, crate::generator::Language::Kotlin)?,
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
pub fn generate_rust(doc: &ForgeDocument, template_dir: &Path) -> Result<GeneratedOutput, ForgeError> {
    generate_rust_with_imports(doc, template_dir, &[])
}

/// Generate Rust code with cross-file import support.
pub fn generate_rust_with_imports(
    doc: &ForgeDocument,
    template_dir: &Path,
    imports: &[ImportContext],
) -> Result<GeneratedOutput, ForgeError> {
    let forge_dir = template_dir.join("forge/rust");
    let mut env = generator::new_env();
    generator::load_templates(&mut env, &forge_dir)?;
    inject_runtime_dep_global(&mut env, doc);

    let code = match doc {
        ForgeDocument::Transform(m) => render_transform(&env, m, imports, crate::generator::Language::Rust)?,
        ForgeDocument::Lookup(m) => render_lookup(&env, m, imports, crate::generator::Language::Rust)?,
        ForgeDocument::Condition(m) => render_condition(&env, m, imports, crate::generator::Language::Rust)?,
        ForgeDocument::Codec(m) => render_codec(&env, m, imports, crate::generator::Language::Rust)?,
        ForgeDocument::Validator(m) => render_validator(&env, m, imports, crate::generator::Language::Rust)?,
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
pub fn generate_go(doc: &ForgeDocument, template_dir: &Path) -> Result<GeneratedOutput, ForgeError> {
    generate_go_with_imports(doc, template_dir, &[])
}

/// Generate Go code with cross-file import support.
pub fn generate_go_with_imports(
    doc: &ForgeDocument,
    template_dir: &Path,
    imports: &[ImportContext],
) -> Result<GeneratedOutput, ForgeError> {
    let forge_dir = template_dir.join("forge/go");
    let mut env = generator::new_env();
    generator::load_templates(&mut env, &forge_dir)?;
    inject_runtime_dep_global(&mut env, doc);

    let code = match doc {
        ForgeDocument::Transform(m) => render_transform(&env, m, imports, crate::generator::Language::Go)?,
        ForgeDocument::Lookup(m) => render_lookup(&env, m, imports, crate::generator::Language::Go)?,
        ForgeDocument::Condition(m) => render_condition(&env, m, imports, crate::generator::Language::Go)?,
        ForgeDocument::Codec(m) => render_codec(&env, m, imports, crate::generator::Language::Go)?,
        ForgeDocument::Validator(m) => render_validator(&env, m, imports, crate::generator::Language::Go)?,
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
pub fn generate_python(doc: &ForgeDocument, template_dir: &Path) -> Result<GeneratedOutput, ForgeError> {
    generate_python_with_imports(doc, template_dir, &[])
}

/// Generate Python code with cross-file import support.
pub fn generate_python_with_imports(
    doc: &ForgeDocument,
    template_dir: &Path,
    imports: &[ImportContext],
) -> Result<GeneratedOutput, ForgeError> {
    let forge_dir = template_dir.join("forge/python");
    let mut env = generator::new_env();
    generator::load_templates(&mut env, &forge_dir)?;
    inject_runtime_dep_global(&mut env, doc);

    let code = match doc {
        ForgeDocument::Transform(m) => render_transform(&env, m, imports, crate::generator::Language::Python)?,
        ForgeDocument::Lookup(m) => render_lookup(&env, m, imports, crate::generator::Language::Python)?,
        ForgeDocument::Condition(m) => render_condition(&env, m, imports, crate::generator::Language::Python)?,
        ForgeDocument::Codec(m) => render_codec(&env, m, imports, crate::generator::Language::Python)?,
        ForgeDocument::Validator(m) => render_validator(&env, m, imports, crate::generator::Language::Python)?,
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

// ══════════════════════════════════════════════════════════════
// ── C11 code generation (RFC §5.J.2) ────────────────────────
// ══════════════════════════════════════════════════════════════
//
// Phase A scope: `Transform` kind only. All other ForgeDocument
// variants return a precise GenerateError that names the deferring
// phase (matches `forge_phase3_complete.md` discipline of failing
// loud at codegen time, not at compile time of stale generated code).

/// Generate code from a ForgeDocument for C11 using Jinja2 templates.
pub fn generate_c11(doc: &ForgeDocument, template_dir: &Path) -> Result<GeneratedOutput, ForgeError> {
    generate_c11_with_imports(doc, template_dir, &[])
}

/// Generate C11 code with cross-file import support.
///
/// Phase A landed Transform; Phase B added Condition, Lookup, Codec;
/// Phase C lifts Validator. Procedure/Filter/Interpolation/Timer/
/// Observer remain `GenerateError::UnsupportedFeature` until their
/// phase, so an operator who points `--language c11` at a fixture in
/// scope for a future phase sees a single-line "deferred to Phase X"
/// diagnostic instead of an `unimplemented!` panic.
pub fn generate_c11_with_imports(
    doc: &ForgeDocument,
    template_dir: &Path,
    imports: &[ImportContext],
) -> Result<GeneratedOutput, ForgeError> {
    let forge_dir = template_dir.join("forge/c");
    let mut env = generator::new_env();
    generator::load_templates(&mut env, &forge_dir)?;
    inject_runtime_dep_global(&mut env, doc);

    let code = match doc {
        ForgeDocument::Transform(m) => render_transform(&env, m, imports, crate::generator::Language::C11)?,
        ForgeDocument::Condition(m) => render_condition(&env, m, imports, crate::generator::Language::C11)?,
        ForgeDocument::Lookup(m) => render_lookup(&env, m, imports, crate::generator::Language::C11)?,
        ForgeDocument::Codec(m) => render_codec(&env, m, imports, crate::generator::Language::C11)?,
        ForgeDocument::Validator(m) => render_validator(&env, m, imports, crate::generator::Language::C11)?,
        ForgeDocument::Procedure(m) => {
            if m.is_l2() {
                return Err(GenerateError::UnsupportedFeature(
                    "C11 forge codegen for L2 procedures (with <sce:helper> / \
                     internal <data> / <onentry><send> / <donedata>) is RFC \
                     \u{00A7}5.J.2 Phase D-2/D-3 work — D-1 ships only the L1 \
                     guard-only path."
                        .into(),
                )
                .into());
            }
            render_procedure_c(&env, m, imports)?
        }
        ForgeDocument::Filter(_)
        | ForgeDocument::Interpolation(_)
        | ForgeDocument::Timer(_)
        | ForgeDocument::Observer(_) => {
            return Err(GenerateError::UnsupportedFeature(
                "C11 forge codegen for kinds Filter/Interpolation/Timer/Observer is \
                 RFC \u{00A7}5.J.2 Phase E work."
                    .into(),
            )
            .into());
        }
    };

    let filename = format!("{}.h", filters::to_snake_case(doc.name().to_string()));
    Ok(GeneratedOutput {
        files: vec![(filename, code)],
    })
}

// ── Procedure: C++ ──────────────────────────────────────────

fn render_procedure_cpp(
    env: &minijinja::Environment,
    m: &ProcedureModel,
    imports: &[ImportContext],
) -> Result<String, ForgeError> {
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
    //
    // RFC `claudedocs/rfc-forge-bytes-bounded.md` §3 B4: `error.execution`
    // is always emitted in the cpp procedure Event enum so the
    // assign-time cap-check codegen can raise it through the shared
    // run_procedure() loop's normal transition machinery, even when the
    // current fixture has no explicit `<transition event="error.execution">`
    // (in which case processTransition simply returns nullopt and the
    // procedure terminates uncompleted — W3C-correct).
    let mut event_raw_to_pascal: std::collections::BTreeMap<String, String> = std::collections::BTreeMap::new();
    event_raw_to_pascal.insert("error.execution".to_string(), "ErrorExecution".to_string());
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
        .map_err(|e| GenerateError::TemplateLoad(e.to_string()))?;

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

    Ok(tmpl.render(ctx).map_err(generator::render_error)?)
}

// ── Procedure: C11 (D-1 L1 only — RFC §5.J.2 §3.D) ──────────
//
// L1 procedures are pure guard-only diamond flows: no `<sce:helper>`,
// no internal `<data>`, no `<onentry><send>`, no `<donedata>`. The
// emit shape is a single `static inline` execute function returning
// a `<name>_result_t` record (`completed` + `final_state` C string),
// driving a flat `switch`/`case` over a `<name>_state_t` enum inside
// a 1000-iteration safety loop.
//
// L2 (D-2/D-3) is rejected at the dispatcher (`generate_c11_with_imports`)
// with a precise error pointing at the relevant sub-phase. This
// function therefore needs no helper / send / donedata / assign
// branches — every fixture it sees has empty `helpers`, empty
// `internals`, no `on_entry_sends`, and no `done_params`.
fn render_procedure_c(
    env: &minijinja::Environment,
    m: &ProcedureModel,
    imports: &[ImportContext],
) -> Result<String, ForgeError> {
    let snake = filters::to_snake_case(m.name.clone());
    let upper = to_upper_snake(&m.name);
    let guard = format!("SCE_FORGE_{}_H", &upper);

    let state_enum: Vec<serde_json::Value> = m
        .states
        .iter()
        .enumerate()
        .map(|(i, s)| {
            serde_json::json!({
                "enum_name": format!("{}_STATE_{}", upper, to_upper_snake(&s.id)),
                "id": s.id,
                "is_final": s.is_final,
                "index": i,
            })
        })
        .collect();

    // Input parameters: snake_case ids in C, native types via c_param_type.
    let input_fields: Vec<serde_json::Value> = m
        .inputs
        .iter()
        .map(|f| {
            serde_json::json!({
                "id": filters::to_snake_case(f.id.clone()),
                "c_param_type": c_param_type(&f.sce_type),
            })
        })
        .collect();
    let params = if input_fields.is_empty() {
        "void".to_string()
    } else {
        m.inputs
            .iter()
            .map(|f| {
                format!(
                    "{} {}",
                    c_param_type(&f.sce_type),
                    filters::to_snake_case(f.id.clone())
                )
            })
            .collect::<Vec<_>>()
            .join(", ")
    };

    // Build identifier rename map: SCXML source ids → snake_case parameter
    // names so the typed expression pipeline emits matching C identifiers.
    let var_name_strings: Vec<String> = m.inputs.iter().map(|f| f.id.clone()).collect();
    let snake_owned: Vec<String> = m
        .inputs
        .iter()
        .map(|f| filters::to_snake_case(f.id.clone()))
        .collect();
    let mut owned_rename: std::collections::HashMap<&str, String> =
        std::collections::HashMap::new();
    for (raw, snk) in var_name_strings.iter().zip(snake_owned.iter()) {
        owned_rename.insert(raw.as_str(), snk.clone());
    }
    let rename_map: std::collections::HashMap<&str, &str> = owned_rename
        .iter()
        .map(|(k, v)| (*k, v.as_str()))
        .collect();

    let procedure_type_ctx = crate::forge::type_ctx::procedure(m, imports);

    // Non-final states: ordered transition list with transpiled C guards.
    // L1 procedures have no event-driven transitions and no transition
    // assigns — every transition is either an unconditional `target` or
    // a guarded `cond`+`target` pair.
    let non_final_states: Vec<serde_json::Value> = m
        .states
        .iter()
        .filter(|s| !s.is_final)
        .map(|s| {
            let transitions: Vec<serde_json::Value> = s
                .transitions
                .iter()
                .map(|tr| {
                    let cond_transpiled = tr.cond.as_ref().map(|c| {
                        transpile_procedure_expr(
                            c,
                            ExprTarget::C,
                            &procedure_type_ctx,
                            &rename_map,
                            crate::forge::types::InferredType::Bool,
                        )
                    });
                    let target_enum = format!(
                        "{}_STATE_{}",
                        upper,
                        to_upper_snake(&tr.target),
                    );
                    serde_json::json!({
                        "has_cond": tr.cond.is_some(),
                        "cond": cond_transpiled.unwrap_or_default(),
                        "target_enum": target_enum,
                    })
                })
                .collect();
            serde_json::json!({
                "enum_name": format!("{}_STATE_{}", upper, to_upper_snake(&s.id)),
                "transitions": transitions,
            })
        })
        .collect();

    let final_states: Vec<serde_json::Value> = m
        .states
        .iter()
        .filter(|s| s.is_final)
        .map(|s| {
            serde_json::json!({
                "enum_name": format!("{}_STATE_{}", upper, to_upper_snake(&s.id)),
                "id": s.id,
            })
        })
        .collect();

    let initial_state_enum = format!(
        "{}_STATE_{}",
        upper,
        to_upper_snake(&m.initial),
    );
    let result_typedef = format!("{}_result_t", &snake);
    let state_typedef = format!("{}_state_t", &snake);
    let execute_func = format!("{}_execute", &snake);

    let tmpl = env
        .get_template("procedure.h.jinja2")
        .map_err(|e| GenerateError::TemplateLoad(e.to_string()))?;

    let ctx = minijinja::context! {
        guard => guard,
        state_typedef => state_typedef,
        result_typedef => result_typedef,
        execute_func => execute_func,
        params => params,
        initial_state_enum => initial_state_enum,
        state_enum => minijinja::Value::from_serialize(&state_enum),
        non_final_states => minijinja::Value::from_serialize(&non_final_states),
        final_states => minijinja::Value::from_serialize(&final_states),
        input_fields => minijinja::Value::from_serialize(&input_fields),
    };

    Ok(tmpl.render(ctx).map_err(generator::render_error)?)
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
                        generator::Language::C11 => unimplemented!(
                            "C11 stateful import method rename is RFC \u{00A7}5.J.1 M3+ work"
                        ),
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
                generator::Language::C11 => unimplemented!(
                    "C11 stateful import field rename is RFC \u{00A7}5.J.1 M3+ work"
                ),
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
    // RFC `claudedocs/rfc-forge-bytes-bounded.md` §3 B4: bytes-typed
    // slot id → resolved cap. Only the cpp branch consumes these
    // fields today (commit 3a). Other backends ignore the extra JSON
    // properties; their per-language commits land later (commits
    // 3b/3c/3d/3e per RFC §8 split).
    let bytes_slot_caps: std::collections::HashMap<&str, u32> = m
        .inputs
        .iter()
        .chain(m.internals.iter())
        .filter(|f| matches!(f.sce_type, crate::forge::model::SceType::Bytes))
        .map(|f| {
            (
                f.id.as_str(),
                crate::forge::limits::resolve_bytes_max(f.max_size),
            )
        })
        .collect();
    let cap_check_target = matches!(target, ExprTarget::Cpp);

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
                            // Cap-check fires when (a) the destination
                            // slot is bytes-typed with a known cap and
                            // (b) the current target language has its
                            // procedure runtime wired for the
                            // error.execution raise path. cpp is the
                            // first such backend.
                            let slot_cap = bytes_slot_caps.get(a.location.as_str()).copied();
                            let is_bytes_with_cap = cap_check_target
                                && slot_cap.is_some()
                                && matches!(
                                    lhs_ty,
                                    crate::forge::types::InferredType::Bytes
                                );
                            serde_json::json!({
                                "location": location_emitted,
                                "expr": wrapped,
                                "is_bytes_with_cap": is_bytes_with_cap,
                                "cap": slot_cap.unwrap_or(0),
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
        ExprTarget::C => unimplemented!(
            "C11 bytes_wrap_for: procedure kind is RFC \u{00A7}5.J.2 Phase D work"
        ),
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
) -> Result<String, ForgeError> {
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
        .map_err(|e| GenerateError::TemplateLoad(e.to_string()))?;

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

    Ok(tmpl.render(ctx).map_err(generator::render_error)?)
}

// ── Procedure: Rust ─────────────────────────────────────────

fn render_procedure_rust(
    env: &minijinja::Environment,
    m: &ProcedureModel,
    imports: &[ImportContext],
) -> Result<String, ForgeError> {
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
        .map_err(|e| GenerateError::TemplateLoad(e.to_string()))?;

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

    Ok(tmpl.render(ctx).map_err(generator::render_error)?)
}

// ── Procedure: Go ───────────────────────────────────────────

fn render_procedure_go(
    env: &minijinja::Environment,
    m: &ProcedureModel,
    imports: &[ImportContext],
) -> Result<String, ForgeError> {
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
        .map_err(|e| GenerateError::TemplateLoad(e.to_string()))?;

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

    Ok(tmpl.render(ctx).map_err(generator::render_error)?)
}

// ── Procedure: Python ───────────────────────────────────────

fn render_procedure_python(
    env: &minijinja::Environment,
    m: &ProcedureModel,
    imports: &[ImportContext],
) -> Result<String, ForgeError> {
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
        .map_err(|e| GenerateError::TemplateLoad(e.to_string()))?;

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

    Ok(tmpl.render(ctx).map_err(generator::render_error)?)
}

// ── Inline kind rendering (policy struct member functions) ─────
//
// Inline kinds live inside the policy struct — they access datamodel
// member variables directly via `this->`. This is distinct from standalone
// kinds, which are namespace-scoped free functions with explicit parameters.

/// Output of inline kind rendering: type definitions and member functions.
/// Rust and Go require types (enums, structs) at module/package level,
/// while C++ and Kotlin support nested types inside a class/struct body.
pub struct InlineKindCode {
    /// Top-level type definitions (enums, structs) — populated for Rust/Go
    /// where types cannot be nested inside impl/struct blocks.
    /// Empty for C++ and Kotlin.
    pub type_defs: String,
    /// Member functions and (for C++/Kotlin) nested type definitions.
    pub member_fns: String,
}

/// Render all inline kinds for a given target language.
/// `machine_name` is the PascalCase policy name (needed for Go receiver types).
pub fn render_inline_kinds(
    kinds: &[InlineKind],
    lang: crate::generator::Language,
    machine_name: &str,
) -> Result<InlineKindCode, ForgeError> {
    let l = LangCtx::new(lang);
    let mut type_defs = Vec::new();
    let mut member_fns = Vec::new();

    for kind in kinds {
        let (td, mf) = render_single_inline_kind(kind, &l, machine_name)?;
        if !td.is_empty() {
            type_defs.push(td);
        }
        member_fns.push(mf);
    }

    Ok(InlineKindCode {
        type_defs: type_defs.join("\n"),
        member_fns: member_fns.join("\n"),
    })
}

/// Dispatch a single inline kind to its type-specific renderer.
fn render_single_inline_kind(
    kind: &InlineKind,
    l: &LangCtx,
    machine_name: &str,
) -> Result<(String, String), ForgeError> {
    match &kind.data {
        InlineKindData::Transform { inputs: _, expr, output_type } => {
            render_inline_transform_member(&kind.id, expr, output_type, l, machine_name)
        }
        InlineKindData::Lookup { input_id, entries, default_value } => {
            render_inline_lookup_member(&kind.id, input_id, entries, default_value, l)
        }
        InlineKindData::Condition { expr } => {
            render_inline_condition_member(&kind.id, expr, l, machine_name)
        }
        InlineKindData::Codec { fields, default_endian } => {
            render_inline_codec_member(&kind.id, fields, *default_endian, l)
        }
    }
}

/// Build identifier→member-access renames for languages that require explicit
/// `self.` (Rust) or `p.` (Go) prefixes when accessing policy struct fields.
/// C++ and Kotlin use implicit member access, so no renames are needed.
fn build_member_renames(
    raw_expr: &str,
    l: &LangCtx,
) -> Result<Vec<(String, String)>, ForgeError> {
    use crate::generator::Language;
    match l.lang {
        Language::Cpp | Language::Kotlin | Language::Python => Ok(Vec::new()),
        Language::Rust => {
            let idents = expr::extract_free_idents(raw_expr)?;
            Ok(idents
                .into_iter()
                .map(|id| {
                    let target = format!("self.{}", filters::to_snake_case(id.clone()));
                    (id, target)
                })
                .collect())
        }
        Language::Go => {
            let idents = expr::extract_free_idents(raw_expr)?;
            Ok(idents
                .into_iter()
                .map(|id| {
                    let target =
                        format!("p.{}", go_escape_builtin(&filters::to_camel_case(id.clone())));
                    (id, target)
                })
                .collect())
        }
        Language::C11 => unimplemented!(
            "C11 inline-kind member renames are RFC \u{00A7}5.J.1 M3+ work \
             (statechart emitter follows lookup vertical slice)"
        ),
    }
}

/// Inline transform: member function returning computed value from policy fields.
///
/// Inline kinds reference the enclosing statechart's member variables. For C++
/// and Kotlin, implicit member access works directly. For Rust and Go, we build
/// identifier renames to insert `self.` / `p.` prefixes. The empty TypeCtx
/// means we rely on the host compiler for final type checking.
fn render_inline_transform_member(
    id: &str,
    raw_expr: &str,
    output_type: &SceType,
    l: &LangCtx,
    machine_name: &str,
) -> Result<(String, String), ForgeError> {
    use crate::generator::Language;
    let empty_ctx = crate::forge::type_ctx::empty();
    let expected = crate::forge::types::InferredType::from_sce_type(output_type);

    let member_renames = build_member_renames(raw_expr, l)?;
    let renames = rename_map(&member_renames);

    let transpiled = expr::transpile_typed(
        raw_expr,
        l.expr_target(),
        &empty_ctx,
        &renames,
        expected,
    )?;

    let ret_type = l.type_name(output_type);

    let code = match l.lang {
        Language::Cpp => {
            let func_name = format!("compute{}", filters::to_pascal_case(id.to_string()));
            format!(
                "    // SCE Forge: Inline transform '{id}'\n\
                 \x20   [[nodiscard]] {ret_type} {func_name}() const {{\n\
                 \x20       return {transpiled};\n\
                 \x20   }}"
            )
        }
        Language::Kotlin => {
            let func_name = format!("compute{}", filters::to_pascal_case(id.to_string()));
            format!(
                "    // SCE Forge: Inline transform '{id}'\n\
                 \x20   fun {func_name}(): {ret_type} = {transpiled}"
            )
        }
        Language::Rust => {
            let func_name = format!("compute_{}", filters::to_snake_case(id.to_string()));
            format!(
                "    // SCE Forge: Inline transform '{id}'\n\
                 \x20   pub fn {func_name}(&self) -> {ret_type} {{\n\
                 \x20       {transpiled}\n\
                 \x20   }}"
            )
        }
        Language::Go => {
            let func_name = format!("Compute{}", filters::to_pascal_case(id.to_string()));
            format!(
                "// SCE Forge: Inline transform '{id}'\n\
                 func (p *{machine_name}Policy) {func_name}() {ret_type} {{\n\
                 \treturn {transpiled}\n\
                 }}"
            )
        }
        Language::Python => {
            let func_name = format!("compute_{}", filters::to_snake_case(id.to_string()));
            format!(
                "    # SCE Forge: Inline transform '{id}'\n\
                 \x20   def {func_name}(self) -> {ret_type}:\n\
                 \x20       return {transpiled}"
            )
        }
        Language::C11 => unimplemented!(
            "C11 inline transform emitter is RFC \u{00A7}5.J.1 M3+ work"
        ),
    };

    Ok((String::new(), code))
}

/// Inline lookup: enum type + lookup function with switch/match/when.
/// For C++/Kotlin the enum is nested inside the member code. For Rust/Go
/// the enum goes to type_defs (module/package level).
fn render_inline_lookup_member(
    id: &str,
    input_id: &str,
    entries: &[LookupEntry],
    default_value: &str,
    l: &LangCtx,
) -> Result<(String, String), ForgeError> {
    use crate::generator::Language;
    let enum_name = filters::to_pascal_case(id.to_string());

    // Collect unique values preserving order
    let mut seen = std::collections::BTreeSet::new();
    let mut unique_values = Vec::new();
    for entry in entries {
        if seen.insert(entry.value.clone()) {
            unique_values.push(entry.value.clone());
        }
    }

    // Group entries by value for switch/match arms
    let mut map: std::collections::BTreeMap<String, Vec<String>> =
        std::collections::BTreeMap::new();
    for entry in entries {
        map.entry(entry.value.clone())
            .or_default()
            .push(entry.key.clone());
    }

    match l.lang {
        Language::Cpp => {
            let func_name = format!("lookup{}", filters::to_pascal_case(id.to_string()));
            let mut code = String::new();
            code.push_str(&format!(
                "    // SCE Forge: Inline lookup '{id}'\n\
                 \x20   enum class {enum_name} {{ {} }};\n\n",
                unique_values.join(", ")
            ));
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
            Ok((String::new(), code))
        }

        Language::Kotlin => {
            let func_name = format!("lookup{}", filters::to_pascal_case(id.to_string()));
            let mut code = String::new();
            code.push_str(&format!(
                "    // SCE Forge: Inline lookup '{id}'\n\
                 \x20   enum class {enum_name} {{ {} }}\n\n",
                unique_values.join(", ")
            ));
            code.push_str(&format!(
                "    fun {func_name}({input_id}: Int): {enum_name} = when ({input_id}) {{\n"
            ));
            for (value, keys) in &map {
                let keys_str = keys.join(", ");
                code.push_str(&format!(
                    "        {keys_str} -> {enum_name}.{value}\n"
                ));
            }
            code.push_str(&format!(
                "        else -> {enum_name}.{default_value}\n\
                 \x20   }}"
            ));
            Ok((String::new(), code))
        }

        Language::Rust => {
            // Rust enum variants use PascalCase (e.g. OFF → Off)
            let rust_variant = |v: &str| -> String {
                let mut chars = v.chars();
                match chars.next() {
                    Some(c) => {
                        let rest: String = chars.collect::<String>().to_lowercase();
                        format!("{}{rest}", c.to_uppercase().next().unwrap_or(c))
                    }
                    None => String::new(),
                }
            };

            let func_name = format!("lookup_{}", filters::to_snake_case(id.to_string()));

            // Type definition (module level)
            let mut type_def = String::new();
            type_def.push_str(&format!(
                "// SCE Forge: Inline lookup '{id}'\n\
                 #[derive(Debug, Clone, Copy, PartialEq)]\n\
                 pub enum {enum_name} {{\n"
            ));
            for v in &unique_values {
                type_def.push_str(&format!("    {},\n", rust_variant(v)));
            }
            type_def.push_str("}");

            // Function (impl block)
            let input_snake = filters::to_snake_case(input_id.to_string());
            let mut code = String::new();
            code.push_str(&format!(
                "    // SCE Forge: Inline lookup '{id}'\n\
                 \x20   pub fn {func_name}({input_snake}: u32) -> {enum_name} {{\n\
                 \x20       match {input_snake} {{\n"
            ));
            for (value, keys) in &map {
                let keys_str = keys.join(" | ");
                code.push_str(&format!(
                    "            {keys_str} => {enum_name}::{},\n",
                    rust_variant(value)
                ));
            }
            code.push_str(&format!(
                "            _ => {enum_name}::{},\n\
                 \x20       }}\n\
                 \x20   }}",
                rust_variant(default_value)
            ));
            Ok((type_def, code))
        }

        Language::Go => {
            let func_name = format!("Lookup{}", filters::to_pascal_case(id.to_string()));

            // Type + const block (package level)
            let mut type_def = String::new();
            type_def.push_str(&format!(
                "// SCE Forge: Inline lookup '{id}'\n\
                 type {enum_name} int\n\n\
                 const (\n"
            ));
            for (i, v) in unique_values.iter().enumerate() {
                if i == 0 {
                    type_def.push_str(&format!(
                        "\t{enum_name}{v} {enum_name} = iota\n"
                    ));
                } else {
                    type_def.push_str(&format!(
                        "\t{enum_name}{v}\n"
                    ));
                }
            }
            type_def.push(')');

            // Package-level function (no receiver — pure lookup)
            let input_camel = go_escape_builtin(&filters::to_camel_case(input_id.to_string()));
            let mut code = String::new();
            code.push_str(&format!(
                "// SCE Forge: Inline lookup '{id}'\n\
                 func {func_name}({input_camel} uint32) {enum_name} {{\n\
                 \tswitch {input_camel} {{\n"
            ));
            for (value, keys) in &map {
                for key in keys {
                    code.push_str(&format!("\tcase {key}:\n"));
                }
                code.push_str(&format!("\t\treturn {enum_name}{value}\n"));
            }
            code.push_str(&format!(
                "\tdefault:\n\
                 \t\treturn {enum_name}{default_value}\n\
                 \t}}\n\
                 }}"
            ));
            Ok((type_def, code))
        }

        Language::Python => {
            let func_name = format!("lookup_{}", filters::to_snake_case(id.to_string()));
            let input_snake = filters::to_snake_case(input_id.to_string());
            let mut code = String::new();
            code.push_str(&format!(
                "    # SCE Forge: Inline lookup '{id}'\n\
                 \x20   class {enum_name}:\n"
            ));
            for (i, v) in unique_values.iter().enumerate() {
                code.push_str(&format!("        {v} = {i}\n"));
            }
            code.push_str(&format!(
                "\n    @staticmethod\n\
                 \x20   def {func_name}({input_snake}: int) -> '{enum_name}':\n\
                 \x20       _map = {{"
            ));
            for (value, keys) in &map {
                for key in keys {
                    code.push_str(&format!("{key}: {enum_name}.{value}, "));
                }
            }
            code.push_str(&format!(
                "}}\n\
                 \x20       return _map.get({input_snake}, {enum_name}.{default_value})"
            ));
            Ok((String::new(), code))
        }
        Language::C11 => unimplemented!(
            "C11 inline lookup emitter is RFC \u{00A7}5.J.1 M2+ work \
             (lookup vertical slice is the M2 milestone — replace this arm \
             when the lookup.h.jinja2/lookup.c.jinja2 templates land)"
        ),
    }
}

/// Inline condition: member function returning bool from policy fields.
fn render_inline_condition_member(
    id: &str,
    raw_expr: &str,
    l: &LangCtx,
    machine_name: &str,
) -> Result<(String, String), ForgeError> {
    use crate::generator::Language;
    let empty_ctx = crate::forge::type_ctx::empty();

    let member_renames = build_member_renames(raw_expr, l)?;
    let renames = rename_map(&member_renames);

    let transpiled = expr::transpile_typed(
        raw_expr,
        l.expr_target(),
        &empty_ctx,
        &renames,
        crate::forge::types::InferredType::Bool,
    )?;

    let code = match l.lang {
        Language::Cpp => {
            let func_name = filters::to_camel_case(id.to_string());
            format!(
                "    // SCE Forge: Inline condition '{id}'\n\
                 \x20   [[nodiscard]] bool {func_name}() const {{\n\
                 \x20       return {transpiled};\n\
                 \x20   }}"
            )
        }
        Language::Kotlin => {
            let func_name = filters::to_camel_case(id.to_string());
            format!(
                "    // SCE Forge: Inline condition '{id}'\n\
                 \x20   fun {func_name}(): Boolean = {transpiled}"
            )
        }
        Language::Rust => {
            let func_name = filters::to_snake_case(id.to_string());
            format!(
                "    // SCE Forge: Inline condition '{id}'\n\
                 \x20   pub fn {func_name}(&self) -> bool {{\n\
                 \x20       {transpiled}\n\
                 \x20   }}"
            )
        }
        Language::Go => {
            let func_name = filters::to_pascal_case(id.to_string());
            format!(
                "// SCE Forge: Inline condition '{id}'\n\
                 func (p *{machine_name}Policy) {func_name}() bool {{\n\
                 \treturn {transpiled}\n\
                 }}"
            )
        }
        Language::Python => {
            let func_name = filters::to_snake_case(id.to_string());
            format!(
                "    # SCE Forge: Inline condition '{id}'\n\
                 \x20   def {func_name}(self) -> bool:\n\
                 \x20       return {transpiled}"
            )
        }
        Language::C11 => unimplemented!(
            "C11 inline condition emitter is RFC \u{00A7}5.J.1 M3+ work"
        ),
    };

    Ok((String::new(), code))
}

/// Inline codec: struct with decode/encode methods.
/// For C++/Kotlin, the struct is nested inside member code.
/// For Rust/Go, the struct and its methods go to type_defs.
fn render_inline_codec_member(
    id: &str,
    codec_fields: &[CodecField],
    default_endian: Endian,
    l: &LangCtx,
) -> Result<(String, String), ForgeError> {
    use crate::generator::Language;
    let struct_name = filters::to_pascal_case(id.to_string());

    // Compute min frame bytes
    let mut min_bytes = 0u32;
    for f in codec_fields {
        if let Some(bits) = f.fixed_bits() {
            let end = f.byte_offset + (bits + 7) / 8;
            min_bytes = min_bytes.max(end);
        }
    }

    match l.lang {
        Language::Cpp => {
            let mut code = String::new();
            code.push_str(&format!("    // SCE Forge: Inline codec '{id}'\n"));
            code.push_str(&format!("    struct {struct_name} {{\n"));
            for f in codec_fields {
                code.push_str(&format!(
                    "        {} {};\n",
                    cpp_type(&f.sce_type),
                    f.id
                ));
            }
            code.push_str(&format!(
                "\n        static std::optional<{struct_name}> decode(const uint8_t* raw, size_t len) {{\n\
                 \x20           if (len < {min_bytes}) return std::nullopt;\n\
                 \x20           return {struct_name}{{\n"
            ));
            for f in codec_fields {
                let decode = generate_decode_expr(f, default_endian, Language::Cpp);
                code.push_str(&format!("                .{} = {},\n", f.id, decode));
            }
            code.push_str("            };\n        }\n");
            let encode_exprs =
                generate_encode_exprs(codec_fields, default_endian, Language::Cpp);
            code.push_str(
                "\n        std::vector<uint8_t> encode() const {\n            return {\n",
            );
            for (i, expr_str) in encode_exprs.iter().enumerate() {
                let comma = if i < encode_exprs.len() - 1 { "," } else { "" };
                code.push_str(&format!("                {expr_str}{comma}\n"));
            }
            code.push_str("            };\n        }\n");
            code.push_str("    };");
            Ok((String::new(), code))
        }

        Language::Kotlin => {
            let mut code = String::new();
            code.push_str(&format!("    // SCE Forge: Inline codec '{id}'\n"));
            code.push_str(&format!("    data class {struct_name}(\n"));
            for (i, f) in codec_fields.iter().enumerate() {
                let comma = if i < codec_fields.len() - 1 { "," } else { "" };
                code.push_str(&format!(
                    "        val {}: {}{comma}\n",
                    f.id,
                    kotlin_type(&f.sce_type)
                ));
            }
            code.push_str("    ) {\n        companion object {\n");
            code.push_str(&format!(
                "            fun decode(raw: ByteArray): {struct_name}? {{\n\
                 \x20               if (raw.size < {min_bytes}) return null\n\
                 \x20               return {struct_name}(\n"
            ));
            for f in codec_fields {
                let decode = generate_decode_expr(f, default_endian, Language::Kotlin);
                code.push_str(&format!("                    {},\n", decode));
            }
            code.push_str("                )\n            }\n        }\n");
            let encode_exprs =
                generate_encode_exprs(codec_fields, default_endian, Language::Kotlin);
            code.push_str(
                "        fun encode(): ByteArray = byteArrayOf(\n",
            );
            for (i, expr_str) in encode_exprs.iter().enumerate() {
                let comma = if i < encode_exprs.len() - 1 { "," } else { "" };
                code.push_str(&format!("            {expr_str}{comma}\n"));
            }
            code.push_str("        )\n    }");
            Ok((String::new(), code))
        }

        Language::Rust => {
            let mut type_def = String::new();
            type_def.push_str(&format!("// SCE Forge: Inline codec '{id}'\n"));
            type_def.push_str(&format!("#[derive(Debug, Clone)]\npub struct {struct_name} {{\n"));
            for f in codec_fields {
                let field_id = filters::to_snake_case(f.id.clone());
                type_def.push_str(&format!(
                    "    pub {}: {},\n",
                    field_id,
                    rust_type(&f.sce_type)
                ));
            }
            type_def.push_str("}\n\n");
            type_def.push_str(&format!("impl {struct_name} {{\n"));
            type_def.push_str(&format!(
                "    pub fn decode(raw: &[u8]) -> Option<Self> {{\n\
                 \x20       if raw.len() < {min_bytes} {{ return None; }}\n\
                 \x20       Some(Self {{\n"
            ));
            for f in codec_fields {
                let decode = generate_decode_expr(f, default_endian, Language::Rust);
                let field_id = filters::to_snake_case(f.id.clone());
                type_def.push_str(&format!("            {field_id}: {decode},\n"));
            }
            type_def.push_str("        })\n    }\n\n");
            let encode_exprs =
                generate_encode_exprs(codec_fields, default_endian, Language::Rust);
            type_def.push_str("    pub fn encode(&self) -> Vec<u8> {\n        vec![\n");
            for (i, expr_str) in encode_exprs.iter().enumerate() {
                let comma = if i < encode_exprs.len() - 1 { "," } else { "" };
                type_def.push_str(&format!("            {expr_str}{comma}\n"));
            }
            type_def.push_str("        ]\n    }\n}");
            Ok((type_def, String::new()))
        }

        Language::Go => {
            let mut type_def = String::new();
            type_def.push_str(&format!("// SCE Forge: Inline codec '{id}'\n"));
            type_def.push_str(&format!("type {struct_name} struct {{\n"));
            for f in codec_fields {
                let field_id = filters::to_pascal_case(f.id.clone());
                type_def.push_str(&format!(
                    "\t{} {}\n",
                    field_id,
                    go_type(&f.sce_type)
                ));
            }
            type_def.push_str("}\n\n");
            type_def.push_str(&format!(
                "func Decode{struct_name}(raw []byte) (*{struct_name}, error) {{\n\
                 \tif len(raw) < {min_bytes} {{\n\
                 \t\treturn nil, fmt.Errorf(\"frame too short: %d < {min_bytes}\", len(raw))\n\
                 \t}}\n\
                 \treturn &{struct_name}{{\n"
            ));
            for f in codec_fields {
                let decode = generate_decode_expr(f, default_endian, Language::Go);
                let field_id = filters::to_pascal_case(f.id.clone());
                type_def.push_str(&format!("\t\t{field_id}: {decode},\n"));
            }
            type_def.push_str("\t}, nil\n}\n\n");
            let encode_exprs =
                generate_encode_exprs(codec_fields, default_endian, Language::Go);
            type_def.push_str(&format!(
                "func (s *{struct_name}) Encode() []byte {{\n\treturn []byte{{\n"
            ));
            for (i, expr_str) in encode_exprs.iter().enumerate() {
                let comma = if i < encode_exprs.len() - 1 { "," } else { "" };
                type_def.push_str(&format!("\t\t{expr_str}{comma}\n"));
            }
            type_def.push_str("\t}\n}");
            Ok((type_def, String::new()))
        }

        Language::Python => {
            let mut code = String::new();
            code.push_str(&format!("    # SCE Forge: Inline codec '{id}'\n"));
            code.push_str(&format!("    class {struct_name}:\n"));
            code.push_str("        def __init__(self");
            for f in codec_fields {
                let field_id = filters::to_snake_case(f.id.clone());
                code.push_str(&format!(", {field_id}: {}", python_type(&f.sce_type)));
            }
            code.push_str("):\n");
            for f in codec_fields {
                let field_id = filters::to_snake_case(f.id.clone());
                code.push_str(&format!("            self.{field_id} = {field_id}\n"));
            }
            code.push_str(&format!(
                "\n        @staticmethod\n\
                 \x20       def decode(raw: bytes) -> '{struct_name} | None':\n\
                 \x20           if len(raw) < {min_bytes}:\n\
                 \x20               return None\n\
                 \x20           return {struct_name}(\n"
            ));
            for f in codec_fields {
                let decode = generate_decode_expr(f, default_endian, Language::Python);
                code.push_str(&format!("                {decode},\n"));
            }
            code.push_str("            )\n");
            let encode_exprs =
                generate_encode_exprs(codec_fields, default_endian, Language::Python);
            code.push_str("        def encode(self) -> bytes:\n            return bytes([\n");
            for (i, expr_str) in encode_exprs.iter().enumerate() {
                let comma = if i < encode_exprs.len() - 1 { "," } else { "" };
                code.push_str(&format!("                {expr_str}{comma}\n"));
            }
            code.push_str("            ])");
            Ok((String::new(), code))
        }
        Language::C11 => unimplemented!(
            "C11 inline codec emitter is RFC \u{00A7}5.J.1 M3+ work \
             (codec DSL emitter follows lookup vertical slice)"
        ),
    }
}

// ══════════════════════════════════════════════════════════════
// ── Phase 3: unified render functions (language-parameterized) ──
// ══════════════════════════════════════════════════════════════

/// Language-specific helpers for Phase 3 kind rendering.
/// Eliminates per-language duplication across filter/interpolation/timer/observer.
/// Language-aware helper for template context construction.
///
/// Centralises type mapping, identifier casing, parameter formatting, and
/// template routing so that per-kind render functions are language-agnostic.
struct LangCtx {
    lang: crate::generator::Language,
}

impl LangCtx {
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
            crate::generator::Language::C11 => c_type(ty),
        }
    }

    /// Parameter type for function signatures (uses references/borrows for
    /// heap-allocated types in C++ and Rust).
    fn param_type(&self, ty: &SceType) -> String {
        match self.lang {
            crate::generator::Language::Cpp => cpp_param_type(ty),
            crate::generator::Language::Rust => rust_param_type(ty),
            crate::generator::Language::C11 => c_param_type(ty).to_string(),
            _ => self.type_name(ty).to_string(),
        }
    }

    /// Format a full parameter list string from fields.
    fn param_str(&self, fields: &[ForgeField]) -> String {
        fields.iter()
            .map(|f| self.format_param(&f.id, &f.sce_type))
            .collect::<Vec<_>>()
            .join(", ")
    }

    /// Format a single parameter: handles language-specific id casing, type
    /// placement order, and reference/borrow semantics.
    fn format_param(&self, id: &str, ty: &SceType) -> String {
        match self.lang {
            crate::generator::Language::Cpp =>
                format!("{} {}", cpp_param_type(ty), id),
            crate::generator::Language::Kotlin =>
                format!("{}: {}", id, kotlin_type(ty)),
            crate::generator::Language::Rust =>
                format!("{}: {}", filters::to_snake_case(id.to_string()), rust_param_type(ty)),
            crate::generator::Language::Go =>
                format!("{} {}", go_escape_builtin(id), go_type(ty)),
            crate::generator::Language::Python =>
                format!("{}: {}", filters::to_snake_case(id.to_string()), python_type(ty)),
            crate::generator::Language::C11 =>
                format!("{} {}", c_param_type(ty), filters::to_snake_case(id.to_string())),
        }
    }

    /// Language-specific identifier for local variables / parameters.
    fn local_id(&self, id: &str) -> String {
        match self.lang {
            crate::generator::Language::Rust
            | crate::generator::Language::Python
            | crate::generator::Language::C11 =>
                filters::to_snake_case(id.to_string()),
            crate::generator::Language::Go =>
                go_escape_builtin(id),
            _ => id.to_string(),
        }
    }

    fn template_ext(&self) -> &'static str {
        match self.lang {
            crate::generator::Language::Cpp => "h",
            crate::generator::Language::Kotlin => "kt",
            crate::generator::Language::Rust => "rs",
            crate::generator::Language::Go => "go",
            crate::generator::Language::Python => "py",
            // C11 forge templates emit `.h` + `.c` pairs (RFC §5.J.1).
            // The single-extension contract LangCtx assumes here is the
            // header — the M2+ lookup vertical slice will introduce a
            // companion `template_body_ext()` (or equivalent shape) for
            // the `.c` half. Until then this arm is unreachable because
            // generate_c11(...) does not exist.
            crate::generator::Language::C11 => "h",
        }
    }

    fn expr_target(&self) -> ExprTarget {
        match self.lang {
            crate::generator::Language::Cpp => ExprTarget::Cpp,
            crate::generator::Language::Kotlin => ExprTarget::Kotlin,
            crate::generator::Language::Rust => ExprTarget::Rust,
            crate::generator::Language::Go => ExprTarget::Go,
            crate::generator::Language::Python => ExprTarget::Python,
            crate::generator::Language::C11 => ExprTarget::C,
        }
    }

    /// Base context fields common to all kinds (guard, namespace, package).
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
                m.insert("package".into(), filters::to_snake_case(name.to_string()).into());
            }
            crate::generator::Language::Kotlin => {
                m.insert("package".into(), filters::to_snake_case(name.to_string()).into());
            }
            crate::generator::Language::C11 => {
                // C has no namespace concept — only the include guard differs
                // from Cpp, dropping the C++ name-mangling-sensitive tail.
                m.insert("guard".into(), format!("SCE_FORGE_{}_H", to_upper_snake(name)).into());
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

    /// Build Go rename pairs for builtin-colliding identifiers.
    /// Returns empty vec for non-Go languages.
    fn go_rename_pairs<'a, I: Iterator<Item = &'a str>>(&self, ids: I) -> Vec<(String, String)> {
        if !matches!(self.lang, crate::generator::Language::Go) {
            return Vec::new();
        }
        ids.map(|id| (id.to_string(), go_escape_builtin(id)))
            .filter(|(f, t)| f != t)
            .collect()
    }


    /// Language-specific literal formatting for typed constant arrays.
    fn literal(&self, val: &str, ty: &SceType) -> String {
        match self.lang {
            crate::generator::Language::Cpp => cpp_literal(val, ty),
            crate::generator::Language::Kotlin => kotlin_literal(val, ty),
            crate::generator::Language::Rust => rust_literal(val, ty),
            crate::generator::Language::Go => go_literal(val, ty),
            crate::generator::Language::Python => python_literal(val, ty),
            crate::generator::Language::C11 => c_literal(val, ty),
        }
    }

    /// Load a kind template by name (e.g. "transform" → "transform.h.jinja2").
    fn load_template<'a>(
        &self,
        env: &'a minijinja::Environment,
        kind: &str,
    ) -> Result<minijinja::Template<'a, 'a>, ForgeError> {
        let name = format!("{}.{}.jinja2", kind, self.template_ext());
        env.get_template(&name).map_err(|e| GenerateError::TemplateLoad(e.to_string()).into())
    }

    /// Render a template from a serde_json::Map context.
    fn render(
        &self,
        env: &minijinja::Environment,
        kind: &str,
        ctx: serde_json::Map<String, serde_json::Value>,
    ) -> Result<String, ForgeError> {
        let tmpl = self.load_template(env, kind)?;
        let value = minijinja::Value::from_serialize(&ctx);
        Ok(tmpl.render(value).map_err(generator::render_error)?)
    }

    /// Insert standard import fields into a context map.
    fn insert_imports(
        &self,
        ctx: &mut serde_json::Map<String, serde_json::Value>,
        imports: &[ImportContext],
    ) {
        let (has_imports, all_imports, stateful_imports) = build_template_imports(imports);
        ctx.insert("has_imports".into(), has_imports.into());
        ctx.insert("imports".into(), serde_json::to_value(&stateful_imports).unwrap_or_default());
        ctx.insert("all_imports".into(), serde_json::to_value(&all_imports).unwrap_or_default());
    }

    // ── Codec-specific helpers ──────────────────────────────────

    /// Template-facing type key for codec fields (e.g. "cpp_type", "kt_type").
    fn codec_type_key(&self) -> &'static str {
        match self.lang {
            crate::generator::Language::Cpp => "cpp_type",
            crate::generator::Language::Kotlin => "kt_type",
            crate::generator::Language::Rust => "rs_type",
            crate::generator::Language::Go => "go_type",
            crate::generator::Language::Python => "py_type",
            crate::generator::Language::C11 => "c_type",
        }
    }

    /// Codec field ID: Go PascalCase, Rust/Python/C11 snake_case, others as-is.
    fn codec_field_id(&self, id: &str) -> String {
        match self.lang {
            crate::generator::Language::Go => filters::to_pascal_case(id.to_string()),
            crate::generator::Language::Rust
            | crate::generator::Language::Python
            | crate::generator::Language::C11 =>
                filters::to_snake_case(id.to_string()),
            _ => id.to_string(),
        }
    }

    /// Self/receiver prefix for codec encode field references.
    fn codec_field_ref(&self, name: &str) -> String {
        match self.lang {
            crate::generator::Language::Rust | crate::generator::Language::Python =>
                format!("self.{name}"),
            crate::generator::Language::Go =>
                format!("s.{name}"),
            // C11's encode is a free function `encode(const struct_t *self)`
            // so member access goes through the pointer with `->`.
            crate::generator::Language::C11 =>
                format!("self->{name}"),
            _ => name.to_string(),
        }
    }

    /// Cast expression to byte (uint8) for encode.
    fn codec_to_byte(&self, expr: &str) -> String {
        match self.lang {
            crate::generator::Language::Cpp =>
                format!("static_cast<uint8_t>({expr})"),
            crate::generator::Language::Kotlin =>
                format!("({expr}).toByte()"),
            crate::generator::Language::Rust =>
                format!("({expr}) as u8"),
            crate::generator::Language::Go =>
                format!("byte({expr})"),
            crate::generator::Language::Python =>
                format!("({expr}) & 0xFF"),
            crate::generator::Language::C11 =>
                format!("(uint8_t)({expr})"),
        }
    }

    /// Comment syntax for unsupported/manual code.
    fn codec_comment(&self, text: &str) -> String {
        match self.lang {
            crate::generator::Language::Python => format!("# {text}"),
            _ => format!("/* {text} */"),
        }
    }

    /// Validator previous-value variable name per language convention.
    fn prev_name(&self, id: &str) -> String {
        match self.lang {
            crate::generator::Language::Rust
            | crate::generator::Language::Python
            | crate::generator::Language::C11 =>
                format!("prev_{}", filters::to_snake_case(id.to_string())),
            _ =>
                format!("prev{}", filters::to_pascal_case(self.local_id(id))),
        }
    }
}

/// Build a rename HashMap from pre-computed (original, escaped) pairs.
fn rename_map(pairs: &[(String, String)]) -> std::collections::HashMap<&str, &str> {
    pairs.iter().map(|(f, t)| (f.as_str(), t.as_str())).collect()
}

fn render_phase3(
    env: &minijinja::Environment,
    template_name: &str,
    ctx: serde_json::Map<String, serde_json::Value>,
) -> Result<String, ForgeError> {
    let tmpl = env
        .get_template(template_name)
        .map_err(|e| GenerateError::TemplateLoad(e.to_string()))?;
    let value = minijinja::Value::from_serialize(&ctx);
    Ok(tmpl.render(value).map_err(generator::render_error)?)
}

// ── Filter (unified) ──────────────────────────────────────────

fn render_filter(
    env: &minijinja::Environment,
    m: &FilterModel,
    imports: &[ImportContext],
    lang: crate::generator::Language,
) -> Result<String, ForgeError> {
    let l = LangCtx::new(lang);
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
) -> Result<String, ForgeError> {
    let l = LangCtx::new(lang);
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
) -> Result<String, ForgeError> {
    let l = LangCtx::new(lang);
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
) -> Result<String, ForgeError> {
    let l = LangCtx::new(lang);
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
            crate::generator::Language::C11 => unimplemented!(
                "C11 observer active_var is RFC \u{00A7}5.J.1 M3+ work"
            ),
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

// ══════════════════════════════════════════════════════════════
// ── Unit tests ───────────────────────────────────────────────
// ══════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;
    use crate::forge::model::{ForgeKind, SceType};

    // ── Type mapping: cpp ────────────────────────────────────

    #[test]
    fn cpp_type_all_variants() {
        assert_eq!(cpp_type(&SceType::Uint8), "uint8_t");
        assert_eq!(cpp_type(&SceType::Uint16), "uint16_t");
        assert_eq!(cpp_type(&SceType::Uint32), "uint32_t");
        assert_eq!(cpp_type(&SceType::Uint64), "uint64_t");
        assert_eq!(cpp_type(&SceType::Int8), "int8_t");
        assert_eq!(cpp_type(&SceType::Int16), "int16_t");
        assert_eq!(cpp_type(&SceType::Int32), "int32_t");
        assert_eq!(cpp_type(&SceType::Int64), "int64_t");
        assert_eq!(cpp_type(&SceType::Float32), "float");
        assert_eq!(cpp_type(&SceType::Float64), "double");
        assert_eq!(cpp_type(&SceType::Bool), "bool");
        assert_eq!(cpp_type(&SceType::String), "std::string");
        assert_eq!(cpp_type(&SceType::Bytes), "std::vector<uint8_t>");
    }

    #[test]
    fn cpp_param_type_references_large_types() {
        assert_eq!(cpp_param_type(&SceType::String), "const std::string&");
        assert_eq!(cpp_param_type(&SceType::Bytes), "const std::vector<uint8_t>&");
    }

    #[test]
    fn cpp_param_type_value_for_primitives() {
        assert_eq!(cpp_param_type(&SceType::Int32), "int32_t");
        assert_eq!(cpp_param_type(&SceType::Bool), "bool");
        assert_eq!(cpp_param_type(&SceType::Float64), "double");
    }

    // ── Type mapping: kotlin ─────────────────────────────────

    #[test]
    fn kotlin_type_all_variants() {
        assert_eq!(kotlin_type(&SceType::Uint8), "UByte");
        assert_eq!(kotlin_type(&SceType::Uint16), "UShort");
        assert_eq!(kotlin_type(&SceType::Uint32), "UInt");
        assert_eq!(kotlin_type(&SceType::Uint64), "ULong");
        assert_eq!(kotlin_type(&SceType::Int8), "Byte");
        assert_eq!(kotlin_type(&SceType::Int16), "Short");
        assert_eq!(kotlin_type(&SceType::Int32), "Int");
        assert_eq!(kotlin_type(&SceType::Int64), "Long");
        assert_eq!(kotlin_type(&SceType::Float32), "Float");
        assert_eq!(kotlin_type(&SceType::Float64), "Double");
        assert_eq!(kotlin_type(&SceType::Bool), "Boolean");
        assert_eq!(kotlin_type(&SceType::String), "String");
        assert_eq!(kotlin_type(&SceType::Bytes), "ByteArray");
    }

    #[test]
    fn kotlin_unsigned_conversion_narrowing() {
        assert_eq!(kotlin_unsigned_conversion(&SceType::Uint8), Some("toInt"));
        assert_eq!(kotlin_unsigned_conversion(&SceType::Uint16), Some("toInt"));
        assert_eq!(kotlin_unsigned_conversion(&SceType::Uint32), Some("toLong"));
        assert_eq!(kotlin_unsigned_conversion(&SceType::Uint64), Some("toLong"));
    }

    #[test]
    fn kotlin_unsigned_conversion_none_for_signed() {
        assert_eq!(kotlin_unsigned_conversion(&SceType::Int32), None);
        assert_eq!(kotlin_unsigned_conversion(&SceType::Float64), None);
        assert_eq!(kotlin_unsigned_conversion(&SceType::Bool), None);
        assert_eq!(kotlin_unsigned_conversion(&SceType::String), None);
    }

    // ── Type mapping: rust ───────────────────────────────────

    #[test]
    fn rust_type_all_variants() {
        assert_eq!(rust_type(&SceType::Uint8), "u8");
        assert_eq!(rust_type(&SceType::Uint16), "u16");
        assert_eq!(rust_type(&SceType::Uint32), "u32");
        assert_eq!(rust_type(&SceType::Uint64), "u64");
        assert_eq!(rust_type(&SceType::Int8), "i8");
        assert_eq!(rust_type(&SceType::Int16), "i16");
        assert_eq!(rust_type(&SceType::Int32), "i32");
        assert_eq!(rust_type(&SceType::Int64), "i64");
        assert_eq!(rust_type(&SceType::Float32), "f32");
        assert_eq!(rust_type(&SceType::Float64), "f64");
        assert_eq!(rust_type(&SceType::Bool), "bool");
        assert_eq!(rust_type(&SceType::String), "String");
        assert_eq!(rust_type(&SceType::Bytes), "Vec<u8>");
    }

    #[test]
    fn rust_param_type_borrows_heap_types() {
        assert_eq!(rust_param_type(&SceType::String), "&str");
        assert_eq!(rust_param_type(&SceType::Bytes), "&[u8]");
    }

    #[test]
    fn rust_param_type_value_for_primitives() {
        assert_eq!(rust_param_type(&SceType::Int32), "i32");
        assert_eq!(rust_param_type(&SceType::Float64), "f64");
        assert_eq!(rust_param_type(&SceType::Bool), "bool");
    }

    // ── Type mapping: go ─────────────────────────────────────

    #[test]
    fn go_type_all_variants() {
        assert_eq!(go_type(&SceType::Uint8), "uint8");
        assert_eq!(go_type(&SceType::Uint16), "uint16");
        assert_eq!(go_type(&SceType::Uint32), "uint32");
        assert_eq!(go_type(&SceType::Uint64), "uint64");
        assert_eq!(go_type(&SceType::Int8), "int8");
        assert_eq!(go_type(&SceType::Int16), "int16");
        assert_eq!(go_type(&SceType::Int32), "int32");
        assert_eq!(go_type(&SceType::Int64), "int64");
        assert_eq!(go_type(&SceType::Float32), "float32");
        assert_eq!(go_type(&SceType::Float64), "float64");
        assert_eq!(go_type(&SceType::Bool), "bool");
        assert_eq!(go_type(&SceType::String), "string");
        assert_eq!(go_type(&SceType::Bytes), "[]byte");
    }

    // ── Type mapping: python ─────────────────────────────────

    #[test]
    fn python_type_collapses_integers() {
        assert_eq!(python_type(&SceType::Uint8), "int");
        assert_eq!(python_type(&SceType::Int64), "int");
        assert_eq!(python_type(&SceType::Uint64), "int");
    }

    #[test]
    fn python_type_collapses_floats() {
        assert_eq!(python_type(&SceType::Float32), "float");
        assert_eq!(python_type(&SceType::Float64), "float");
    }

    #[test]
    fn python_type_special() {
        assert_eq!(python_type(&SceType::Bool), "bool");
        assert_eq!(python_type(&SceType::String), "str");
        assert_eq!(python_type(&SceType::Bytes), "bytes");
    }

    // ── go_escape_builtin ────────────────────────────────────

    #[test]
    fn go_escape_builtins() {
        assert_eq!(go_escape_builtin("byte"), "byte_");
        assert_eq!(go_escape_builtin("string"), "string_");
        assert_eq!(go_escape_builtin("int"), "int_");
        assert_eq!(go_escape_builtin("len"), "len_");
        assert_eq!(go_escape_builtin("make"), "make_");
        assert_eq!(go_escape_builtin("true"), "true_");
        assert_eq!(go_escape_builtin("nil"), "nil_");
        assert_eq!(go_escape_builtin("iota"), "iota_");
    }

    #[test]
    fn go_escape_non_builtin_unchanged() {
        assert_eq!(go_escape_builtin("myVar"), "myVar");
        assert_eq!(go_escape_builtin("temperature"), "temperature");
        assert_eq!(go_escape_builtin("rpm"), "rpm");
    }

    // ── looks_like_int ───────────────────────────────────────

    #[test]
    fn looks_like_int_positive() {
        assert!(looks_like_int("100"));
        assert!(looks_like_int("0"));
        assert!(looks_like_int("-42"));
    }

    #[test]
    fn looks_like_int_negative() {
        assert!(!looks_like_int("1.5"));
        assert!(!looks_like_int("1e10"));
        assert!(!looks_like_int("2E3"));
        assert!(!looks_like_int("0.0"));
    }

    // ── Literal formatters ───────────────────────────────────

    #[test]
    fn rust_literal_float32_from_int() {
        assert_eq!(rust_literal("100", &SceType::Float32), "100.0_f32");
    }

    #[test]
    fn rust_literal_float32_from_float() {
        assert_eq!(rust_literal("1.5", &SceType::Float32), "1.5_f32");
    }

    #[test]
    fn rust_literal_float64_from_int() {
        assert_eq!(rust_literal("100", &SceType::Float64), "100.0");
    }

    #[test]
    fn rust_literal_float64_from_float() {
        assert_eq!(rust_literal("1.5", &SceType::Float64), "1.5");
    }

    #[test]
    fn rust_literal_integer_passthrough() {
        assert_eq!(rust_literal("42", &SceType::Int32), "42");
    }

    #[test]
    fn cpp_literal_float32_from_int() {
        assert_eq!(cpp_literal("100", &SceType::Float32), "100.0f");
    }

    #[test]
    fn cpp_literal_float32_from_float() {
        assert_eq!(cpp_literal("1.5", &SceType::Float32), "1.5f");
    }

    #[test]
    fn cpp_literal_float64_from_int() {
        assert_eq!(cpp_literal("100", &SceType::Float64), "100.0");
    }

    #[test]
    fn cpp_literal_integer_passthrough() {
        assert_eq!(cpp_literal("42", &SceType::Int32), "42");
    }

    #[test]
    fn go_literal_float_from_int() {
        assert_eq!(go_literal("100", &SceType::Float32), "100.0");
        assert_eq!(go_literal("100", &SceType::Float64), "100.0");
    }

    #[test]
    fn go_literal_float_from_float() {
        assert_eq!(go_literal("1.5", &SceType::Float64), "1.5");
    }

    #[test]
    fn go_literal_integer_passthrough() {
        assert_eq!(go_literal("42", &SceType::Int32), "42");
    }

    #[test]
    fn kotlin_literal_unsigned_types() {
        assert_eq!(kotlin_literal("100", &SceType::Uint8), "100u.toUByte()");
        assert_eq!(kotlin_literal("200", &SceType::Uint16), "200u.toUShort()");
        assert_eq!(kotlin_literal("300", &SceType::Uint32), "300u.toUInt()");
        assert_eq!(kotlin_literal("400", &SceType::Uint64), "400u.toULong()");
    }

    #[test]
    fn kotlin_literal_signed_narrow() {
        assert_eq!(kotlin_literal("42", &SceType::Int8), "(42).toByte()");
        assert_eq!(kotlin_literal("42", &SceType::Int16), "(42).toShort()");
    }

    #[test]
    fn kotlin_literal_long() {
        assert_eq!(kotlin_literal("100", &SceType::Int64), "100L");
    }

    #[test]
    fn kotlin_literal_float() {
        assert_eq!(kotlin_literal("100", &SceType::Float32), "100.0f");
        assert_eq!(kotlin_literal("1.5", &SceType::Float32), "1.5f");
        assert_eq!(kotlin_literal("100", &SceType::Float64), "100.0");
    }

    #[test]
    fn kotlin_literal_string() {
        assert_eq!(kotlin_literal("hello", &SceType::String), "\"hello\"");
    }

    #[test]
    fn python_literal_float_from_int() {
        assert_eq!(python_literal("100", &SceType::Float32), "100.0");
        assert_eq!(python_literal("100", &SceType::Float64), "100.0");
    }

    #[test]
    fn python_literal_string() {
        assert_eq!(python_literal("hello", &SceType::String), "'hello'");
    }

    #[test]
    fn python_literal_bool() {
        assert_eq!(python_literal("true", &SceType::Bool), "True");
        assert_eq!(python_literal("false", &SceType::Bool), "False");
    }

    #[test]
    fn python_literal_integer_passthrough() {
        assert_eq!(python_literal("42", &SceType::Int32), "42");
    }

    // ── normalized_go_prefix ─────────────────────────────────

    #[test]
    fn go_prefix_strips_trailing_slash() {
        let opts = crate::ForgeCompileOptions {
            go_module_prefix: Some("github.com/acme/gen/".to_string()),
        };
        assert_eq!(normalized_go_prefix(&opts), Some("github.com/acme/gen"));
    }

    #[test]
    fn go_prefix_no_trailing_slash() {
        let opts = crate::ForgeCompileOptions {
            go_module_prefix: Some("github.com/acme/gen".to_string()),
        };
        assert_eq!(normalized_go_prefix(&opts), Some("github.com/acme/gen"));
    }

    #[test]
    fn go_prefix_none() {
        let opts = crate::ForgeCompileOptions {
            go_module_prefix: None,
        };
        assert_eq!(normalized_go_prefix(&opts), None);
    }

    #[test]
    fn go_prefix_multiple_trailing_slashes() {
        let opts = crate::ForgeCompileOptions {
            go_module_prefix: Some("github.com/acme///".to_string()),
        };
        assert_eq!(normalized_go_prefix(&opts), Some("github.com/acme"));
    }

    // ── validate_options ─────────────────────────────────────

    #[test]
    fn validate_go_with_imports_missing_prefix() {
        let imports = vec![ForgeImport {
            src: "transform.scxml".to_string(),
            kind: ForgeKind::Transform,
            alias: "t".to_string(),
            line: None,
        }];
        let opts = crate::ForgeCompileOptions { go_module_prefix: None };
        let result = validate_options(&imports, &crate::generator::Language::Go, &opts);
        assert!(result.is_err());
    }

    #[test]
    fn validate_go_with_imports_empty_prefix() {
        let imports = vec![ForgeImport {
            src: "transform.scxml".to_string(),
            kind: ForgeKind::Transform,
            alias: "t".to_string(),
            line: None,
        }];
        let opts = crate::ForgeCompileOptions {
            go_module_prefix: Some("".to_string()),
        };
        let result = validate_options(&imports, &crate::generator::Language::Go, &opts);
        assert!(result.is_err());
    }

    #[test]
    fn validate_go_with_imports_whitespace_prefix() {
        let imports = vec![ForgeImport {
            src: "transform.scxml".to_string(),
            kind: ForgeKind::Transform,
            alias: "t".to_string(),
            line: None,
        }];
        let opts = crate::ForgeCompileOptions {
            go_module_prefix: Some("github.com/acme /gen".to_string()),
        };
        let result = validate_options(&imports, &crate::generator::Language::Go, &opts);
        assert!(result.is_err());
    }

    #[test]
    fn validate_go_with_imports_valid_prefix() {
        let imports = vec![ForgeImport {
            src: "transform.scxml".to_string(),
            kind: ForgeKind::Transform,
            alias: "t".to_string(),
            line: None,
        }];
        let opts = crate::ForgeCompileOptions {
            go_module_prefix: Some("github.com/acme/gen".to_string()),
        };
        let result = validate_options(&imports, &crate::generator::Language::Go, &opts);
        assert!(result.is_ok());
    }

    #[test]
    fn validate_go_no_imports_no_prefix_ok() {
        let opts = crate::ForgeCompileOptions { go_module_prefix: None };
        let result = validate_options(&[], &crate::generator::Language::Go, &opts);
        assert!(result.is_ok());
    }

    #[test]
    fn validate_non_go_ignores_prefix() {
        let imports = vec![ForgeImport {
            src: "transform.scxml".to_string(),
            kind: ForgeKind::Transform,
            alias: "t".to_string(),
            line: None,
        }];
        let opts = crate::ForgeCompileOptions { go_module_prefix: None };
        let result = validate_options(&imports, &crate::generator::Language::Cpp, &opts);
        assert!(result.is_ok());
    }

    // ── resolve_single_import ────────────────────────────────

    fn test_import() -> ForgeImport {
        ForgeImport {
            src: "temperature_transform.scxml".to_string(),
            kind: ForgeKind::Transform,
            alias: "temp".to_string(),
            line: None,
        }
    }

    fn stateful_import() -> ForgeImport {
        ForgeImport {
            src: "simple_codec.scxml".to_string(),
            kind: ForgeKind::Codec,
            alias: "frame".to_string(),
            line: None,
        }
    }

    #[test]
    fn resolve_import_cpp_stateless() {
        let imp = test_import();
        let opts = crate::ForgeCompileOptions::default();
        let ctx = resolve_single_import(&imp, &crate::generator::Language::Cpp, &opts);
        assert_eq!(ctx.alias, "temp");
        assert_eq!(ctx.include_stmt, "#include \"temperature_transform.h\"");
        assert!(!ctx.is_stateful);
        assert_eq!(ctx.namespace, "SCE::Generated::TemperatureTransform");
    }

    #[test]
    fn resolve_import_cpp_stateful() {
        let imp = stateful_import();
        let opts = crate::ForgeCompileOptions::default();
        let ctx = resolve_single_import(&imp, &crate::generator::Language::Cpp, &opts);
        assert!(ctx.is_stateful);
        assert_eq!(ctx.member_name, "frame_");
        assert_eq!(ctx.member_type, "::SCE::Generated::SimpleCodec::SimpleCodec");
    }

    #[test]
    fn resolve_import_kotlin() {
        let imp = test_import();
        let opts = crate::ForgeCompileOptions::default();
        let ctx = resolve_single_import(&imp, &crate::generator::Language::Kotlin, &opts);
        assert_eq!(ctx.include_stmt, "import com.sce.generated.temperature_transform.*");
        assert_eq!(ctx.type_name, "TemperatureTransform");
    }

    #[test]
    fn resolve_import_rust_stateless() {
        let imp = test_import();
        let opts = crate::ForgeCompileOptions::default();
        let ctx = resolve_single_import(&imp, &crate::generator::Language::Rust, &opts);
        // Stateless: import module path, not type
        assert_eq!(ctx.include_stmt, "use super::temperature_transform;");
        assert!(!ctx.is_stateful);
    }

    #[test]
    fn resolve_import_rust_stateful() {
        let imp = stateful_import();
        let opts = crate::ForgeCompileOptions::default();
        let ctx = resolve_single_import(&imp, &crate::generator::Language::Rust, &opts);
        // Stateful: import the type directly
        assert_eq!(ctx.include_stmt, "use super::simple_codec::SimpleCodec;");
        assert!(ctx.is_stateful);
    }

    #[test]
    fn resolve_import_go() {
        let imp = test_import();
        let opts = crate::ForgeCompileOptions {
            go_module_prefix: Some("github.com/acme/gen".to_string()),
        };
        let ctx = resolve_single_import(&imp, &crate::generator::Language::Go, &opts);
        assert_eq!(
            ctx.include_stmt,
            "\t\"github.com/acme/gen/temperature_transform\""
        );
        assert_eq!(ctx.namespace, "temperature_transform");
    }

    #[test]
    fn resolve_import_python_stateless() {
        let imp = test_import();
        let opts = crate::ForgeCompileOptions::default();
        let ctx = resolve_single_import(&imp, &crate::generator::Language::Python, &opts);
        assert_eq!(ctx.include_stmt, "from . import temperature_transform");
    }

    #[test]
    fn resolve_import_python_stateful() {
        let imp = stateful_import();
        let opts = crate::ForgeCompileOptions::default();
        let ctx = resolve_single_import(&imp, &crate::generator::Language::Python, &opts);
        assert_eq!(ctx.include_stmt, "from .simple_codec import SimpleCodec");
    }

    // ── to_upper_snake ───────────────────────────────────────

    #[test]
    fn upper_snake_from_pascal() {
        assert_eq!(to_upper_snake("EngineStart"), "ENGINE_START");
    }

    #[test]
    fn upper_snake_from_camel() {
        assert_eq!(to_upper_snake("gearPosition"), "GEAR_POSITION");
    }

    #[test]
    fn upper_snake_from_snake() {
        assert_eq!(to_upper_snake("gear_position"), "GEAR_POSITION");
    }

    // ── to_rust_variant ──────────────────────────────────────

    #[test]
    fn rust_variant_from_uppercase() {
        assert_eq!(to_rust_variant("STOP"), "Stop");
        assert_eq!(to_rust_variant("RUNNING"), "Running");
        assert_eq!(to_rust_variant("ENGINE_START"), "EngineStart");
    }

    #[test]
    fn rust_variant_from_mixed_case() {
        assert_eq!(to_rust_variant("engineStart"), "EngineStart");
    }

    #[test]
    fn rust_variant_single_char() {
        assert_eq!(to_rust_variant("A"), "A");
    }

    #[test]
    fn rust_variant_with_digits() {
        assert_eq!(to_rust_variant("GEAR_1"), "Gear1");
    }

    // ── build_template_imports ────────────────────────────────

    #[test]
    fn template_imports_empty() {
        let (has, _all, _stateful) = build_template_imports(&[]);
        assert!(!has);
    }

    #[test]
    fn template_imports_stateful_filter() {
        let imports = vec![
            ImportContext {
                alias: "t".to_string(),
                kind: "transform".to_string(),
                is_stateful: false,
                include_stmt: String::new(),
                type_name: String::new(),
                member_name: String::new(),
                member_type: String::new(),
                namespace: String::new(),
                qualified_call: String::new(),
                param_types: Vec::new(),
                ret_type: None,
                member_field_types: Vec::new(),
                member_method_sigs: Vec::new(),
            },
            ImportContext {
                alias: "c".to_string(),
                kind: "codec".to_string(),
                is_stateful: true,
                include_stmt: String::new(),
                type_name: String::new(),
                member_name: String::new(),
                member_type: String::new(),
                namespace: String::new(),
                qualified_call: String::new(),
                param_types: Vec::new(),
                ret_type: None,
                member_field_types: Vec::new(),
                member_method_sigs: Vec::new(),
            },
        ];
        let (has, _all, _stateful) = build_template_imports(&imports);
        assert!(has);
    }
}

