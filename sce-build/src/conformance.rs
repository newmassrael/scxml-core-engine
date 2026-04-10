// SPDX-License-Identifier: LGPL-2.1-or-later OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// Cross-language conformance fixture catalog.
//
// Deserializes tests/forge/conformance/fixtures.json into a typed model the
// `sce-codegen generate-conformance` subcommand renders into per-language
// test harnesses via Jinja2 templates. The manifest is the single source of
// truth for fixture structural metadata (kind, arg types, output shape);
// expected oracle values live separately in numerical_reference.json.
//
// Adding a new fixture kind requires extending `FixtureKind` here *and* the
// corresponding `{{ kind }}` branch in every per-language harness template.

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

use crate::generator::Language;

/// Canonical primitive types used across the manifest. Each per-language
/// template maps these to its native type system.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CanonicalType {
    Bool,
    I32,
    U8,
    U16,
    U32,
    F32,
    F64,
    String,
}

/// Floating-point comparison vs exact equality.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CompareMode {
    Tolerance,
    Equality,
}

/// Scalar output descriptor for fixtures whose `expected` JSON value is a
/// single primitive.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScalarOutput {
    #[serde(rename = "type")]
    pub ty: CanonicalType,
    pub compare: CompareMode,
    /// Only used by `kind = "transform"` with a single output function.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub function: Option<String>,
}

/// Compound output descriptor for fixtures whose `expected` JSON value is an
/// object with multiple named fields, each computed by a distinct free function.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompoundOutput {
    pub key: String,
    pub function: String,
    #[serde(rename = "type")]
    pub ty: CanonicalType,
    pub compare: CompareMode,
}

/// One fixture entry. The full enum varies by `kind` — fields not meaningful
/// for a given kind are `None`. Per-language templates select the relevant
/// subset via `kind` dispatch.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Fixture {
    pub name: String,
    pub kind: FixtureKind,
    pub ref_section: String,

    /// Pure-function kinds (interpolation/transform/condition/procedure).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub args: Vec<CanonicalType>,

    /// Stateful kinds (filter/observer). Single per-step input.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub input: Option<CanonicalType>,

    /// Scalar output descriptor (interpolation/condition/filter, or transform
    /// with a single output function).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub output: Option<ScalarOutput>,

    /// Compound output — transform fixtures with multiple named outputs.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub compound_outputs: Vec<CompoundOutput>,

    /// Free-function name for `kind = "condition"` (where the generated
    /// function name matches the fixture name in snake_case).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub function: Option<String>,

    /// Observer kind: ordered list of emitted event tag names.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub event_tags: Vec<String>,
}

/// The `kind` discriminator. Mirrors SCE Forge `sce:kind` attribute values
/// but is declared here so the manifest parser does not have to reach into
/// the forge model types.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum FixtureKind {
    Interpolation,
    Transform,
    Condition,
    Filter,
    Observer,
    Procedure,
}

impl FixtureKind {
    /// Lowercase string used by templates (e.g. `{% if fixture.kind == "filter" %}`).
    pub fn as_str(&self) -> &'static str {
        match self {
            FixtureKind::Interpolation => "interpolation",
            FixtureKind::Transform => "transform",
            FixtureKind::Condition => "condition",
            FixtureKind::Filter => "filter",
            FixtureKind::Observer => "observer",
            FixtureKind::Procedure => "procedure",
        }
    }
}

// ── Per-language canonical type mapping ────────────────────────
//
// Single source of truth: per-language harness templates used to define
// their own inline macros mapping "u16"/"f64"/... to their native type
// system. These helpers centralise that table so every template dispatches
// through one function per (language, canonical_type) pair. Registered as
// minijinja filters in `render_harness` below.
//
// The canonical_type argument is the lowercase string that
// `CanonicalType` serializes to; it is accepted as `&str` here so the
// filters can forward template values without allocating.

/// Map a canonical type to its Rust native type name.
pub fn rust_type_for(ty: &str) -> &'static str {
    match ty {
        "bool" => "bool",
        "i32" => "i32",
        "u8" => "u8",
        "u16" => "u16",
        "u32" => "u32",
        "f32" => "f32",
        "f64" => "f64",
        "string" => "&str",
        _ => "/* unknown canonical type */ ()",
    }
}

/// Build a Rust expression that unmarshals `raw` (a `serde_json::Value`
/// accessor expression) into a value of canonical type `ty`.
pub fn rust_unmarshal_expr(raw: &str, ty: &str) -> String {
    match ty {
        "bool" => format!("{raw}.as_bool().expect(\"bool\")"),
        "f64" => format!("{raw}.as_f64().expect(\"f64\")"),
        "f32" => format!("{raw}.as_f64().expect(\"f64\") as f32"),
        "i32" => format!("{raw}.as_i64().expect(\"i64\") as i32"),
        "u8" => format!("{raw}.as_u64().expect(\"u64\") as u8"),
        "u16" => format!("{raw}.as_u64().expect(\"u64\") as u16"),
        "u32" => format!("{raw}.as_u64().expect(\"u64\") as u32"),
        "string" => format!("{raw}.as_str().expect(\"string\")"),
        _ => format!("/* unknown canonical type {ty} */"),
    }
}

/// Map a canonical type to its C++ native type name.
pub fn cpp_type_for(ty: &str) -> &'static str {
    match ty {
        "bool" => "bool",
        "f64" => "double",
        "f32" => "float",
        "i32" => "std::int32_t",
        "u8" => "std::uint8_t",
        "u16" => "std::uint16_t",
        "u32" => "std::uint32_t",
        "string" => "std::string",
        _ => "/* unknown canonical type */ void",
    }
}

/// Map a canonical type to its Go native type name (distinct from the
/// SCXML-variable `to_go_type` filter in `filters.rs`, which targets a
/// different type domain).
pub fn go_type_for(ty: &str) -> &'static str {
    match ty {
        "bool" => "bool",
        "f64" => "float64",
        "f32" => "float32",
        "i32" => "int32",
        "u8" => "uint8",
        "u16" => "uint16",
        "u32" => "uint32",
        "string" => "string",
        _ => "/* unknown canonical type */ interface{}",
    }
}

/// Map a canonical type to its Kotlin native type name.
pub fn kt_type_for(ty: &str) -> &'static str {
    match ty {
        "bool" => "Boolean",
        "f64" => "Double",
        "f32" => "Float",
        "i32" => "Int",
        "u8" => "UByte",
        "u16" => "UShort",
        "u32" => "UInt",
        "string" => "String",
        _ => "/* unknown canonical type */ Any",
    }
}

/// Build a Kotlin expression that unmarshals `raw` (a `JsonElement`
/// accessor expression) into a value of canonical type `ty`.
pub fn kt_unmarshal_expr(raw: &str, ty: &str) -> String {
    match ty {
        "bool" => format!("{raw}.jsonPrimitive.boolean"),
        "f64" => format!("{raw}.jsonPrimitive.double"),
        "f32" => format!("{raw}.jsonPrimitive.double.toFloat()"),
        "i32" => format!("{raw}.jsonPrimitive.int"),
        "u8" => format!("{raw}.jsonPrimitive.int.toUByte()"),
        "u16" => format!("{raw}.jsonPrimitive.int.toUShort()"),
        "u32" => format!("{raw}.jsonPrimitive.int.toUInt()"),
        "string" => format!("{raw}.jsonPrimitive.content"),
        _ => format!("/* unknown canonical type {ty} */"),
    }
}

/// Register the conformance-specific type-mapping filters on a minijinja
/// environment. Called from `render_harness` for every language so the
/// same filter names are available to every template — but note that not
/// every filter is meaningful for every language:
///
/// * `rust_type`, `cpp_type`, `go_type`, `kt_type` — pure type-name mapping,
///   defined for all four typed targets. Python is dynamically typed and
///   never references these.
/// * `rust_unmarshal`, `kt_unmarshal` — expression builders that wrap a JSON
///   accessor. Only Rust and Kotlin need them because their JSON libraries
///   require typed extraction expressions (`as_u64() as u16`, `.jsonPrimitive
///   .int.toUShort()`). C++ uses `nlohmann::json::get<T>()` directly, Go
///   uses `json.Unmarshal` with a typed `var`, and Python's stdlib produces
///   the right Python types on `json.load`. Adding `cpp_unmarshal` /
///   `go_unmarshal` would just duplicate what `cpp_type` / `go_type` already
///   feed into native accessors at the call site.
pub fn register_conformance_filters(env: &mut minijinja::Environment) {
    env.add_filter("rust_type", |ty: String| rust_type_for(&ty).to_string());
    env.add_filter("rust_unmarshal", |raw: String, ty: String| {
        rust_unmarshal_expr(&raw, &ty)
    });
    env.add_filter("cpp_type", |ty: String| cpp_type_for(&ty).to_string());
    env.add_filter("go_type", |ty: String| go_type_for(&ty).to_string());
    env.add_filter("kt_type", |ty: String| kt_type_for(&ty).to_string());
    env.add_filter("kt_unmarshal", |raw: String, ty: String| {
        kt_unmarshal_expr(&raw, &ty)
    });
}

/// Top-level manifest document.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Manifest {
    pub version: u32,
    #[serde(default)]
    pub description: String,
    pub fixtures: Vec<Fixture>,
}

impl Manifest {
    /// Load and parse a manifest file from disk.
    pub fn load(path: &Path) -> Result<Self, String> {
        let text = std::fs::read_to_string(path)
            .map_err(|e| format!("cannot read manifest at {}: {e}", path.display()))?;
        let manifest: Manifest = serde_json::from_str(&text)
            .map_err(|e| format!("invalid manifest JSON at {}: {e}", path.display()))?;
        manifest.validate()?;
        Ok(manifest)
    }

    /// Structural checks that cannot be expressed in the type system alone:
    /// fixture names must be unique, kinds must agree with field presence.
    ///
    /// Note: this is the **only** runtime validator for the manifest. The
    /// `tests/forge/conformance/fixtures.schema.json` document is purely
    /// for editor / IDE / CI-linter integration via the `$schema` reference
    /// in `fixtures.json`; it is never consulted at codegen time. The Rust
    /// type system + this function are the source of truth at runtime.
    pub fn validate(&self) -> Result<(), String> {
        use std::collections::HashSet;
        let mut seen = HashSet::new();
        for f in &self.fixtures {
            if !seen.insert(f.name.clone()) {
                return Err(format!("duplicate fixture name: {}", f.name));
            }
            match f.kind {
                FixtureKind::Filter | FixtureKind::Observer => {
                    if f.input.is_none() {
                        return Err(format!(
                            "fixture {}: kind={} requires `input`",
                            f.name,
                            f.kind.as_str()
                        ));
                    }
                    if !f.args.is_empty() {
                        return Err(format!(
                            "fixture {}: stateful kinds use `input`, not `args`",
                            f.name
                        ));
                    }
                }
                FixtureKind::Interpolation
                | FixtureKind::Condition
                | FixtureKind::Procedure => {
                    if f.args.is_empty() {
                        return Err(format!(
                            "fixture {}: kind={} requires non-empty `args`",
                            f.name,
                            f.kind.as_str()
                        ));
                    }
                }
                FixtureKind::Transform => {
                    if f.args.is_empty() {
                        return Err(format!(
                            "fixture {}: transform requires `args`",
                            f.name
                        ));
                    }
                    if f.output.is_none() && f.compound_outputs.is_empty() {
                        return Err(format!(
                            "fixture {}: transform needs `output` or `compound_outputs`",
                            f.name
                        ));
                    }
                }
            }
            match f.kind {
                FixtureKind::Observer => {
                    if f.event_tags.is_empty() {
                        return Err(format!(
                            "fixture {}: observer requires `event_tags`",
                            f.name
                        ));
                    }
                }
                FixtureKind::Condition => {
                    if f.function.is_none() {
                        return Err(format!(
                            "fixture {}: condition requires `function`",
                            f.name
                        ));
                    }
                }
                _ => {}
            }
        }
        Ok(())
    }
}

/// Per-language harness filesystem layout. Returned as a single bundle by
/// [`harness_layout`] so adding a new language is one match arm in one
/// function instead of three siblings that can drift out of sync.
pub struct HarnessLayout {
    /// Output filename written under the build's chosen output directory.
    pub output_filename: &'static str,
    /// Subdirectory of `tools/codegen/templates/` containing this language's
    /// conformance scaffold and `kinds/` fragments.
    pub template_subdir: &'static str,
    /// Top-level scaffold filename inside `template_subdir`.
    pub template_filename: &'static str,
}

/// Canonical layout (output name, template directory, scaffold name) for
/// the given language. Single source of truth: every consumer that used to
/// call the legacy `harness_filename` / `template_subdir` / `template_filename`
/// trio now goes through this one function.
pub fn harness_layout(language: Language) -> HarnessLayout {
    match language {
        Language::Rust => HarnessLayout {
            output_filename: "numerical_conformance.rs",
            template_subdir: "forge/rust/conformance",
            template_filename: "harness.rs.jinja2",
        },
        // Not test_-prefixed so the committed shim at
        // sce-forge-runtime/python/tests/test_numerical_conformance.py can
        // import from this module without name collision during unittest
        // discovery.
        Language::Python => HarnessLayout {
            output_filename: "conformance_generated.py",
            template_subdir: "forge/python/conformance",
            template_filename: "harness.py.jinja2",
        },
        Language::Go => HarnessLayout {
            output_filename: "numerical_conformance_test.go",
            template_subdir: "forge/go/conformance",
            template_filename: "harness.go.jinja2",
        },
        Language::Kotlin => HarnessLayout {
            output_filename: "NumericalConformanceTest.kt",
            template_subdir: "forge/kotlin/conformance",
            template_filename: "harness.kt.jinja2",
        },
        Language::Cpp => HarnessLayout {
            output_filename: "numerical_conformance_test.cpp",
            template_subdir: "forge/cpp/conformance",
            template_filename: "harness.cpp.jinja2",
        },
    }
}

/// Backwards-compatible wrapper. Kept so existing call sites in
/// `sce_codegen.rs` and `sce-forge-runtime/rust/build.rs` continue to
/// compile; new code should call [`harness_layout`] directly.
pub fn harness_filename(language: Language) -> &'static str {
    harness_layout(language).output_filename
}

/// Render the per-language conformance harness from a manifest, returning
/// the rendered source code.
///
/// This is the shared entry point used by both the `sce-codegen
/// generate-conformance` subcommand and any language's build system calling
/// sce-build in-process (e.g. the Rust `build.rs`).
pub fn render_harness(
    manifest: &Manifest,
    language: Language,
    template_base: &Path,
) -> Result<String, String> {
    let layout = harness_layout(language);
    let template_dir: PathBuf = template_base.join(layout.template_subdir);
    if !template_dir.exists() {
        return Err(format!(
            "conformance template directory not found: {}",
            template_dir.display()
        ));
    }

    // Match the product-template environment settings exactly
    // (`trim_blocks` + `lstrip_blocks` + Python-Jinja2 method compat) so
    // per-kind `{% include %}` fragments lay out predictably without every
    // line needing a manual `{%- -%}` dash.
    let mut env = crate::generator::new_env();
    // Case-conversion filters are shared with the forge product templates so
    // the naming conventions stay aligned between product and test code.
    crate::filters::register_filters(&mut env);
    crate::filters::register_go_filters(&mut env);
    // Conformance type-mapping filters: every per-language template uses the
    // same filter names (`rust_type`, `cpp_type`, `go_type`, `kt_type`, plus
    // the `*_unmarshal` variants) so the canonical-type table lives in one
    // place (see `register_conformance_filters`) instead of being duplicated
    // as inline macros inside each harness template.
    register_conformance_filters(&mut env);
    crate::generator::load_templates(&mut env, &template_dir)?;

    let tmpl_name = layout.template_filename;
    let tmpl = env
        .get_template(tmpl_name)
        .map_err(|e| format!("template {tmpl_name} not found: {e}"))?;

    let ctx = minijinja::context! {
        fixtures => minijinja::Value::from_serialize(&manifest.fixtures),
    };

    tmpl.render(ctx)
        .map_err(|e| format!("render template {tmpl_name}: {e}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn loads_real_manifest() {
        let path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../tests/forge/conformance/fixtures.json");
        let m = Manifest::load(&path).expect("manifest must load and validate");
        assert_eq!(m.version, 1);
        assert!(
            !m.fixtures.is_empty(),
            "manifest should contain at least one fixture"
        );
    }
}
