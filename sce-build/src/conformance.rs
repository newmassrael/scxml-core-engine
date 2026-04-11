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
// Adding a new fixture kind requires extending `FixtureSpec` here *and* the
// corresponding `{{ kind }}` branch in every per-language harness template.

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

use crate::generator::Language;

/// Canonical primitive types used across the manifest. Each per-language
/// template maps these to its native type system.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(test, derive(schemars::JsonSchema))]
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
#[cfg_attr(test, derive(schemars::JsonSchema))]
#[serde(rename_all = "lowercase")]
pub enum CompareMode {
    Tolerance,
    Equality,
}

/// `kind = "lookup"` miss-handling policy. Mirrors `forge::model::MissPolicy`
/// but stays in the conformance manifest namespace so the harness manifest
/// parser does not have to depend on the SCE Forge model types.
///
/// `Error`: the generated function returns the language's optional type;
/// fixtures must hit a key on every input (happy-path only) and the harness
/// fragment unwraps with `.expect()` / `.value()` / `?: error()`.
///
/// `Default`: the generated function returns the raw value with a fallback;
/// fixtures may include miss inputs that test the fallback path. The
/// reference oracle simply lists the expected fallback value for those cases.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(test, derive(schemars::JsonSchema))]
#[serde(rename_all = "lowercase")]
pub enum LookupMissPolicy {
    Error,
    Default,
}

/// Scalar output descriptor for fixtures whose `expected` JSON value is a
/// single primitive.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(test, derive(schemars::JsonSchema))]
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
#[cfg_attr(test, derive(schemars::JsonSchema))]
pub struct CompoundOutput {
    pub key: String,
    pub function: String,
    #[serde(rename = "type")]
    pub ty: CanonicalType,
    pub compare: CompareMode,
}

/// Struct-return output descriptor for fixtures whose `expected` JSON value is
/// an object matching the layout of a generated return struct — a single call
/// returns multiple fields (e.g. `ValidationResult { valid, reason }`). This
/// differs from [`CompoundOutput`] in that every field comes from the *same*
/// call site rather than a distinct free function, so the fragment emits one
/// `validate(...)` / `decode(...)` and then iterates the fields to generate
/// per-field assertions. The descriptor is intentionally kept minimal — name,
/// type, compare mode — because the forge product code generator is the
/// authority on the struct layout; this manifest entry exists only so the
/// conformance fragments don't have to hard-code field names in five places.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(test, derive(schemars::JsonSchema))]
pub struct StructField {
    pub name: String,
    #[serde(rename = "type")]
    pub ty: CanonicalType,
    pub compare: CompareMode,
}

/// One fixture entry. `name` and `ref_section` are common to every kind;
/// kind-specific data lives in the `spec` tagged enum below. The
/// `#[serde(flatten)]` on `spec` means the on-disk JSON stays a single flat
/// object (`{name, ref_section, kind, args, output, ...}`) so per-language
/// Jinja2 templates can continue to read `f.args` / `f.output` / etc. without
/// having to walk a nested `f.spec.*` path.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(test, derive(schemars::JsonSchema))]
pub struct Fixture {
    pub name: String,
    pub ref_section: String,
    #[serde(flatten)]
    pub spec: FixtureSpec,
}

/// Kind-specific fixture data. The `#[serde(tag = "kind")]` attribute makes
/// each variant discriminate on a top-level `"kind"` string, which matches
/// what fixtures.json already stores and what the templates already read.
///
/// The Rust type system now enforces "this kind requires these fields", so
/// `Manifest::validate` no longer has to hand-check field presence for every
/// kind — it only checks cross-field semantic invariants that cannot be
/// expressed in the types (e.g. Transform needs at least one of the two
/// output shapes; Lookup's `function` is derived at runtime so users must
/// not supply it in the manifest).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(test, derive(schemars::JsonSchema))]
#[serde(tag = "kind", rename_all = "lowercase")]
pub enum FixtureSpec {
    Interpolation {
        args: Vec<CanonicalType>,
        output: ScalarOutput,
    },
    Transform {
        args: Vec<CanonicalType>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        output: Option<ScalarOutput>,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        compound_outputs: Vec<CompoundOutput>,
    },
    Condition {
        function: String,
        args: Vec<CanonicalType>,
        output: ScalarOutput,
    },
    Filter {
        input: CanonicalType,
        output: ScalarOutput,
    },
    Observer {
        input: CanonicalType,
        event_tags: Vec<String>,
    },
    Procedure {
        args: Vec<CanonicalType>,
    },
    Lookup {
        args: Vec<CanonicalType>,
        output: ScalarOutput,
        on_miss: LookupMissPolicy,
        /// Derived at harness-rendering time from the SCXML
        /// `<data sce:direction="out">` id. Must be absent in the manifest
        /// JSON — `Manifest::validate` rejects any user-supplied value so
        /// the SCXML stays the single source of truth.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        function: Option<String>,
    },
    /// Stateful data validator. Generated struct exposes
    /// `validate(args...) -> <ReturnStruct>` where `<ReturnStruct>` has the
    /// fields listed in `output` (canonical order). Internal sample memory
    /// for rate-of-change checks is implicit in the struct — not modelled in
    /// the manifest. The oracle lives in the `validators` reference section
    /// and uses stateful sequences that exercise range / ROC / plausibility
    /// branches and the documented rule that state only advances on a
    /// successful validation.
    ///
    /// Every existing forge validator returns `ValidationResult { valid, reason }`,
    /// but modelling the return shape as `Vec<StructField>` keeps the oracle
    /// format, the conformance fragments, and future multi-field codec-style
    /// kinds all sharing one descriptor instead of every fragment carrying a
    /// duplicated hard-coded list of field names.
    Validator {
        args: Vec<CanonicalType>,
        output: Vec<StructField>,
    },
    /// Byte-layout codec. Generated struct exposes `encode()` returning the
    /// raw byte vector and a static `decode(raw)` returning the typed struct
    /// (wrapped in the language's optional type). The fixture oracle holds
    /// per-case `{decoded, encoded}` pairs so every test exercises both
    /// directions and catches drift in either path.
    ///
    /// Field layout (names, canonical types, byte/bit offsets) lives in the
    /// SCXML source — the forge product code generator is the authority on
    /// it. The manifest entry mirrors only the field *names and types* so
    /// the conformance fragments can iterate them to build the decoded
    /// literal and per-field equality assertions without hard-coding names
    /// across five templates. Field names are declared in SCXML source case
    /// (camelCase from `<data id="...">`) and each per-language fragment
    /// applies its own case filter to match the generated struct's native
    /// naming convention.
    Codec {
        fields: Vec<StructField>,
    },
}

impl FixtureSpec {
    /// Lowercase discriminator string, used by this module's validate/render
    /// logic when it needs a stable identifier without pattern-matching the
    /// full variant. Matches the `#[serde(tag)]` value so it also matches
    /// what `{{ f.kind }}` renders inside the Jinja2 templates.
    pub fn kind_str(&self) -> &'static str {
        match self {
            FixtureSpec::Interpolation { .. } => "interpolation",
            FixtureSpec::Transform { .. } => "transform",
            FixtureSpec::Condition { .. } => "condition",
            FixtureSpec::Filter { .. } => "filter",
            FixtureSpec::Observer { .. } => "observer",
            FixtureSpec::Procedure { .. } => "procedure",
            FixtureSpec::Lookup { .. } => "lookup",
            FixtureSpec::Validator { .. } => "validator",
            FixtureSpec::Codec { .. } => "codec",
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
#[cfg_attr(test, derive(schemars::JsonSchema))]
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

    /// Semantic cross-field checks the tagged-enum type system cannot express.
    ///
    /// Field-presence invariants ("filter requires `input`", "lookup requires
    /// `on_miss`", "observer requires `event_tags`", etc.) are now enforced
    /// by `FixtureSpec`'s per-variant struct fields — deserialization fails
    /// with a clear serde error before this function runs. What remains here
    /// is anything that the type system alone cannot check:
    ///
    /// * fixture names must be unique across the catalog,
    /// * `transform` fixtures must carry at least one of the two output
    ///   shapes (`output` scalar OR non-empty `compound_outputs`),
    /// * `validator` fixtures must declare at least one output field — an
    ///   empty `output` would render a test body with no assertions and
    ///   silently pass regardless of the generated struct's behaviour,
    /// * `lookup` fixtures must not preset `function` — it is derived from
    ///   the SCXML output id at harness-rendering time, and letting users
    ///   supply it in the manifest would invite the SCXML/manifest pair to
    ///   drift out of sync.
    ///
    /// This is still the **only** runtime validator for the manifest. The
    /// `tests/forge/conformance/fixtures.schema.json` document is purely for
    /// editor / IDE / CI-linter integration via the `$schema` reference in
    /// `fixtures.json`; it is never consulted at codegen time.
    pub fn validate(&self) -> Result<(), String> {
        use std::collections::HashSet;
        let mut seen = HashSet::new();
        for f in &self.fixtures {
            if !seen.insert(f.name.clone()) {
                return Err(format!("duplicate fixture name: {}", f.name));
            }
            match &f.spec {
                FixtureSpec::Transform {
                    output,
                    compound_outputs,
                    ..
                } => {
                    if output.is_none() && compound_outputs.is_empty() {
                        return Err(format!(
                            "fixture {}: transform needs `output` or `compound_outputs`",
                            f.name
                        ));
                    }
                }
                FixtureSpec::Lookup { function, .. } => {
                    if function.is_some() {
                        return Err(format!(
                            "fixture {}: lookup `function` is derived from \
                             the SCXML output id; remove it from fixtures.json",
                            f.name
                        ));
                    }
                }
                FixtureSpec::Validator { output, .. } => {
                    if output.is_empty() {
                        return Err(format!(
                            "fixture {}: validator requires at least one \
                             `output` field — an empty output would render \
                             an assertion-free test body",
                            f.name
                        ));
                    }
                }
                FixtureSpec::Codec { fields, .. } => {
                    if fields.is_empty() {
                        return Err(format!(
                            "fixture {}: codec requires at least one `fields` \
                             entry — an empty field list would render an \
                             assertion-free round-trip test body",
                            f.name
                        ));
                    }
                }
                FixtureSpec::Interpolation { .. }
                | FixtureSpec::Condition { .. }
                | FixtureSpec::Filter { .. }
                | FixtureSpec::Observer { .. }
                | FixtureSpec::Procedure { .. } => {}
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

/// Parse a `<scxml sce:kind="lookup">` source file and return the
/// `<data sce:direction="out">` field's id (in source case). The conformance
/// harness uses this to derive the generated lookup function name without
/// duplicating it in fixtures.json — the SCXML file is the single source of
/// truth.
fn read_lookup_output_id(scxml_path: &Path) -> Result<String, String> {
    let text = std::fs::read_to_string(scxml_path)
        .map_err(|e| format!("cannot read {}: {e}", scxml_path.display()))?;
    let doc = roxmltree::Document::parse(&text)
        .map_err(|e| format!("invalid SCXML at {}: {e}", scxml_path.display()))?;
    let scxml_ns = "http://www.w3.org/2005/07/scxml";
    let sce_ns = crate::forge::model::SCE_NAMESPACE;
    let datamodel = doc
        .root_element()
        .children()
        .find(|n| n.is_element() && n.tag_name().name() == "datamodel" && n.tag_name().namespace() == Some(scxml_ns))
        .ok_or_else(|| format!("{}: missing <datamodel>", scxml_path.display()))?;
    for data in datamodel.children().filter(|n| n.is_element() && n.tag_name().name() == "data") {
        let dir = data.attribute((sce_ns, "direction")).unwrap_or("");
        if dir == "out" {
            return data
                .attribute("id")
                .map(|s| s.to_string())
                .ok_or_else(|| format!("{}: <data sce:direction=\"out\"> missing id", scxml_path.display()));
        }
    }
    Err(format!(
        "{}: no <data sce:direction=\"out\"> found",
        scxml_path.display()
    ))
}

/// Render the per-language conformance harness from a manifest, returning
/// the rendered source code.
///
/// `resource_dir` is the directory containing the per-fixture `<name>.scxml`
/// files. The harness generator parses each lookup fixture's SCXML to derive
/// its generated function name (avoiding the SCXML/manifest duplication
/// footgun where the user has to keep both in sync).
///
/// This is the shared entry point used by both the `sce-codegen
/// generate-conformance` subcommand and any language's build system calling
/// sce-build in-process (e.g. the Rust `build.rs`).
pub fn render_harness(
    manifest: &Manifest,
    language: Language,
    template_base: &Path,
    resource_dir: &Path,
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

    // Clone fixtures so we can fill the derived `function` field for lookup
    // entries from the SCXML output id without mutating the manifest.
    let mut fixtures = manifest.fixtures.clone();
    for f in fixtures.iter_mut() {
        if let FixtureSpec::Lookup { function, .. } = &mut f.spec {
            let scxml_path = resource_dir.join(format!("{}.scxml", f.name));
            let output_id = read_lookup_output_id(&scxml_path)?;
            // Function naming convention: lookup_<output_id_snake>. Per-language
            // case conversion (camel/pascal) is applied by template filters.
            *function = Some(format!(
                "lookup_{}",
                crate::filters::to_snake_case(output_id)
            ));
        }
    }

    // Per-kind presence flags let per-language harness scaffolds pull in
    // kind-specific runtime imports (e.g. Go `forge` package, Kotlin
    // `sce-kotlin-runtime` types) only when the fixture set actually uses
    // that kind, keeping unused-import warnings and transitive deps scoped.
    let has_procedure = fixtures
        .iter()
        .any(|f| matches!(f.spec, FixtureSpec::Procedure { .. }));

    let ctx = minijinja::context! {
        fixtures => minijinja::Value::from_serialize(&fixtures),
        has_procedure => has_procedure,
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

    /// Drift guard between the Rust `Manifest`/`FixtureSpec` type hierarchy
    /// and the checked-in `tests/forge/conformance/fixtures.schema.json`.
    ///
    /// Prior to this test the schema was hand-maintained and could silently
    /// lag behind type changes — the validator session added a field and the
    /// schema update was forgotten for a full session. Now every type in the
    /// manifest hierarchy derives `JsonSchema` (gated on `cfg(test)` so the
    /// dependency stays in dev-deps), and this test re-derives the schema on
    /// every run and fails with a diff if the checked-in file drifts.
    ///
    /// Bootstrap / refresh: run `UPDATE_EXPECT=1 cargo test -p sce-build
    /// schema_drift_guard` to rewrite the file from the current types. The
    /// reviewer inspects the diff and commits alongside whatever type change
    /// triggered the update — the invariant is *not* that the schema is
    /// frozen, it is that the schema matches the Rust types at every commit.
    #[test]
    fn schema_drift_guard() {
        let schema = schemars::schema_for!(Manifest);
        let generated =
            serde_json::to_string_pretty(&schema).expect("schema must serialize to JSON");
        // Trailing newline so the file is POSIX-friendly and `git diff`
        // doesn't flag a "no newline at end of file" on every write.
        let generated = format!("{generated}\n");

        let path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../tests/forge/conformance/fixtures.schema.json");

        if std::env::var_os("UPDATE_EXPECT").is_some() {
            std::fs::write(&path, &generated)
                .unwrap_or_else(|e| panic!("write {}: {e}", path.display()));
            return;
        }

        let checked_in = std::fs::read_to_string(&path)
            .unwrap_or_else(|e| panic!("read {}: {e}", path.display()));
        assert_eq!(
            checked_in, generated,
            "\nfixtures.schema.json is out of sync with the Rust types in \
             sce-build/src/conformance.rs. Run:\n    \
             UPDATE_EXPECT=1 cargo test -p sce-build schema_drift_guard\n\
             and commit the regenerated file.\n"
        );
    }
}
