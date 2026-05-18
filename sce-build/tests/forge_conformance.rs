// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// SCE Forge conformance tests — verifies kind codegen output against golden references.
//
// Each test: parse SCXML -> generate C++ -> compare against expected output.
// Expected outputs are in tests/forge/expected/ and serve as golden references.

use std::path::Path;

/// Project root (sce-build is at <root>/sce-build).
fn project_root() -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("sce-build must be in project root")
        .to_path_buf()
}

fn resource_dir() -> std::path::PathBuf {
    project_root().join("tests/forge/resources")
}

fn expected_dir() -> std::path::PathBuf {
    project_root().join("tests/forge/expected")
}

/// Placeholder Go module prefix used for every product-golden Go test.
/// Picked from the IANA-reserved `example.com` domain so generated
/// `import "example.com/sce-forge/..."` lines are unmistakably synthetic
/// and deterministic. Real consumers (the sce-forge-runtime Go harness)
/// pass their own module root via `--go-module-prefix`.
const GOLDEN_GO_MODULE_PREFIX: &str = "example.com/sce-forge";

/// Per-language defaults for product-golden forge codegen.
///
/// Each language gets its own branch here so that when
/// `ForgeCompileOptions` grows a new knob (e.g. a future
/// `rust_crate_prefix`), the only edit needed is one line in this
/// factory — individual test cases do not care about option
/// construction and never need to be touched.
fn golden_options(language: sce_build::generator::Language) -> sce_build::ForgeCompileOptions {
    let mut opts = sce_build::ForgeCompileOptions::default();
    if matches!(language, sce_build::generator::Language::Go) {
        opts.go_module_prefix = Some(GOLDEN_GO_MODULE_PREFIX.to_string());
    }
    opts
}

/// Generate code from a standalone forge SCXML for a specific language and compare
/// against expected output.
fn assert_standalone_forge_lang(
    scxml_name: &str,
    expected_filename: &str,
    language: sce_build::generator::Language,
) {
    let scxml_path = resource_dir().join(format!("{scxml_name}.scxml"));
    let content = std::fs::read_to_string(&scxml_path)
        .unwrap_or_else(|e| panic!("Cannot read {}: {e}", scxml_path.display()));

    let stem = scxml_name;
    let base_dir = scxml_path.parent().unwrap();
    let options = golden_options(language);
    let output = sce_build::compile_forge_with_imports(
        &content,
        sce_build::DocumentLabel::symmetric(stem),
        language,
        base_dir,
        &options,
    )
    .unwrap_or_else(|e| panic!("Forge codegen failed for {scxml_name} ({language:?}): {e}"));

    assert!(!output.files.is_empty(), "No output for {scxml_name}");

    let (_, generated) = &output.files[0];
    let expected_path = expected_dir().join(expected_filename);

    // Golden update mode: when UPDATE_GOLDEN=1 is set, overwrite the expected
    // file with the freshly generated output instead of comparing. Used after
    // intentional emitter changes (e.g. cosmetic refactors of the typed AST
    // pipeline) to refresh stale goldens. Requires manual review of the diff
    // before committing.
    if std::env::var("UPDATE_GOLDEN").is_ok() {
        std::fs::write(&expected_path, generated.trim().to_string() + "\n")
            .unwrap_or_else(|e| panic!("Cannot write {}: {e}", expected_path.display()));
        return;
    }

    let expected = std::fs::read_to_string(&expected_path)
        .unwrap_or_else(|e| panic!("Cannot read expected {}: {e}", expected_path.display()));

    assert_eq!(
        generated.trim(),
        expected.trim(),
        "Output mismatch for {scxml_name} ({language:?})\n--- expected: {}\n+++ generated",
        expected_path.display()
    );
}

/// Generate C++ from a standalone forge SCXML and compare against expected output.
fn assert_standalone_forge(scxml_name: &str, expected_filename: &str) {
    assert_standalone_forge_lang(
        scxml_name,
        expected_filename,
        sce_build::generator::Language::Cpp,
    );
}

/// Generate Kotlin from a standalone forge SCXML and compare against expected output.
fn assert_standalone_forge_kotlin(scxml_name: &str, expected_filename: &str) {
    assert_standalone_forge_lang(
        scxml_name,
        expected_filename,
        sce_build::generator::Language::Kotlin,
    );
}

/// Generate Rust from a standalone forge SCXML and compare against expected output.
fn assert_standalone_forge_rust(scxml_name: &str, expected_filename: &str) {
    assert_standalone_forge_lang(
        scxml_name,
        expected_filename,
        sce_build::generator::Language::Rust,
    );
}

/// RFC §5.B B2-test-vector trunk: assert that a forge codegen run
/// produces a per-fixture sidecar file (e.g. `<fixture>_test.rs`) as
/// the second entry in `output.files`, and that its content matches
/// the checked-in golden under `tests/forge/expected/`. Mirrors the
/// `assert_standalone_forge_lang` shape but indexes into the sidecar
/// position so the existing primary-file goldens stay byte-stable.
fn assert_sidecar_forge_lang(
    scxml_name: &str,
    expected_filename: &str,
    language: sce_build::generator::Language,
) {
    let scxml_path = resource_dir().join(format!("{scxml_name}.scxml"));
    let content = std::fs::read_to_string(&scxml_path)
        .unwrap_or_else(|e| panic!("Cannot read {}: {e}", scxml_path.display()));
    let stem = scxml_name;
    let base_dir = scxml_path.parent().unwrap();
    let options = golden_options(language);
    let output = sce_build::compile_forge_with_imports(
        &content,
        sce_build::DocumentLabel::symmetric(stem),
        language,
        base_dir,
        &options,
    )
    .unwrap_or_else(|e| panic!("Forge codegen failed for {scxml_name} ({language:?}): {e}"));

    assert!(
        output.files.len() >= 2,
        "expected sidecar emission for {scxml_name} ({language:?}); output.files.len()={}",
        output.files.len()
    );
    let (_, generated) = &output.files[1];
    let expected_path = expected_dir().join(expected_filename);
    if std::env::var("UPDATE_GOLDEN").is_ok() {
        std::fs::write(&expected_path, generated.trim().to_string() + "\n")
            .unwrap_or_else(|e| panic!("Cannot write {}: {e}", expected_path.display()));
        return;
    }
    let expected = std::fs::read_to_string(&expected_path)
        .unwrap_or_else(|e| panic!("Cannot read expected {}: {e}", expected_path.display()));
    assert_eq!(
        generated.trim(),
        expected.trim(),
        "Sidecar mismatch for {scxml_name} ({language:?})\n--- expected: {}\n+++ generated",
        expected_path.display()
    );
}

/// RFC §5.B B5-θ codec test-vector trunk gate-rejection helper.
/// Asserts that compiling the named codec SCXML for the given
/// language yields exactly one output file (the primary codec
/// header / module) — i.e. no sidecar emission. The 4 trunk-gated
/// backends (Cpp / Kotlin / Go / Python) take this path until per-
/// language B5-θ closures land; once a closure lifts the gate, the
/// matching `forge_codec_..._<lang>_no_sidecar_until_closure` test
/// rotates to a positive `assert_sidecar_forge_lang(...)` call.
fn assert_no_codec_sidecar_until_closure(
    scxml_name: &str,
    language: sce_build::generator::Language,
) {
    let scxml_path = resource_dir().join(format!("{scxml_name}.scxml"));
    let content = std::fs::read_to_string(&scxml_path)
        .unwrap_or_else(|e| panic!("Cannot read {}: {e}", scxml_path.display()));
    let stem = scxml_name;
    let base_dir = scxml_path.parent().unwrap();
    let options = golden_options(language);
    let output = sce_build::compile_forge_with_imports(
        &content,
        sce_build::DocumentLabel::symmetric(stem),
        language,
        base_dir,
        &options,
    )
    .unwrap_or_else(|e| {
        panic!(
            "Trunk gate expects primary codec to compile cleanly for \
             {scxml_name} ({language:?}); got error: {e}"
        )
    });
    assert_eq!(
        output.files.len(),
        1,
        "B5-θ trunk gate: codec '{scxml_name}' on {language:?} must emit exactly one file \
         (primary codec, no sidecar). Got {} files: {:?}",
        output.files.len(),
        output
            .files
            .iter()
            .map(|(n, _)| n.as_str())
            .collect::<Vec<_>>()
    );
}

/// Strip path-dependent comment lines for comparison.
/// Handles `// From: ...` (C++/Rust/Go) and `// Source: ...` (Kotlin).
fn normalize_for_comparison(code: &str) -> String {
    code.lines()
        .filter(|line| !line.starts_with("// From:") && !line.starts_with("// Source:"))
        .collect::<Vec<_>>()
        .join("\n")
}

/// Generate C++ from a statechart with inline kinds and verify against full-file golden.
/// C++ uses the pre-existing full-file golden comparison.
fn assert_inline_kinds_cpp(scxml_name: &str) {
    let scxml_path = resource_dir().join(format!("{scxml_name}.scxml"));
    let tdir = sce_build::find_template_dir_for(sce_build::generator::Language::Cpp);

    let output = sce_build::compile_scxml_lang(
        scxml_path.to_str().unwrap(),
        &tdir,
        sce_build::generator::Language::Cpp,
    )
    .unwrap_or_else(|e| panic!("Statechart codegen (Cpp) failed for {scxml_name}: {e}"));

    let header = &output.files[0].1;
    assert!(
        header.contains("SCE Forge: Inline"),
        "Inline kind code missing in {scxml_name} (Cpp) output"
    );

    let expected_path = expected_dir().join(format!("{scxml_name}_sm.h"));
    if std::env::var("UPDATE_GOLDEN").is_ok() {
        let absolute = scxml_path.to_string_lossy().into_owned();
        let relative = format!("tests/forge/resources/{scxml_name}.scxml");
        let normalized = header.replace(&absolute, &relative);
        std::fs::write(&expected_path, normalized)
            .unwrap_or_else(|e| panic!("Cannot write {}: {e}", expected_path.display()));
        return;
    }
    if expected_path.exists() {
        let expected = std::fs::read_to_string(&expected_path).unwrap();
        assert_eq!(
            normalize_for_comparison(header).trim(),
            normalize_for_comparison(&expected).trim(),
            "Output mismatch for {scxml_name} (Cpp)\n--- expected: {}\n+++ generated",
            expected_path.display()
        );
    }
}

// ── Inline kind multi-language test infrastructure ──────────────
//
// Three-layer verification:
//   1. Fragment golden — render_inline_kinds() output compared against
//      small, stable golden files (~15 lines). Decoupled from template
//      changes; only breaks when the inline kind renderer itself changes.
//   2. Structural assertions — language-specific pattern checks that
//      verify idiomatic output (e.g. Rust `self.` prefix, Go receiver).
//   3. Compile gate — syn::parse_file() on the full generated Rust
//      statechart proves the rendered code is syntactically valid when
//      embedded in a real file context.

/// Render inline kinds directly and compare against fragment goldens,
/// then verify structural correctness and template integration.
fn assert_inline_kinds_lang(scxml_name: &str, lang: sce_build::generator::Language) {
    use sce_build::forge::generator::{render_inline_kinds, InlineKindCode};

    let scxml_path = resource_dir().join(format!("{scxml_name}.scxml"));

    // ── Parse model ────────────────────────────────────────────
    let mut parser = sce_build::parser::SCXMLParser::new();
    let model = parser
        .parse_file(scxml_path.to_str().unwrap())
        .unwrap_or_else(|e| panic!("Parse failed for {scxml_name}: {e}"));

    let machine_name = sce_build::filters::to_pascal_case(model.name.clone());

    assert!(
        !model.inline_kinds.is_empty(),
        "{scxml_name} must have inline kinds for this test"
    );

    // ── Layer 1: Fragment golden ───────────────────────────────
    let InlineKindCode {
        type_defs,
        member_fns,
    } = render_inline_kinds(&model.inline_kinds, lang, &machine_name)
        .unwrap_or_else(|e| panic!("render_inline_kinds({lang:?}) failed: {e}"));

    let lang_tag = match lang {
        sce_build::generator::Language::Kotlin => "kt",
        sce_build::generator::Language::Rust => "rs",
        sce_build::generator::Language::Go => "go",
        sce_build::generator::Language::C11 => "c",
        _ => panic!("unsupported language for inline kind lang test"),
    };

    // Member functions golden (always present)
    let fns_golden_path = expected_dir().join(format!("{scxml_name}_inline_fns.{lang_tag}.golden"));
    if std::env::var("UPDATE_GOLDEN").is_ok() {
        std::fs::write(&fns_golden_path, member_fns.trim().to_string() + "\n")
            .unwrap_or_else(|e| panic!("Cannot write {}: {e}", fns_golden_path.display()));
    } else if fns_golden_path.exists() {
        let expected = std::fs::read_to_string(&fns_golden_path).unwrap();
        assert_eq!(
            member_fns.trim(),
            expected.trim(),
            "Fragment mismatch: member_fns ({lang:?})\n--- expected: {}\n+++ generated",
            fns_golden_path.display()
        );
    } else {
        panic!(
            "Fragment golden not found: {}  (run with UPDATE_GOLDEN=1)",
            fns_golden_path.display()
        );
    }

    // Type definitions golden (Rust/Go only)
    if !type_defs.is_empty() {
        let types_golden_path =
            expected_dir().join(format!("{scxml_name}_inline_types.{lang_tag}.golden"));
        if std::env::var("UPDATE_GOLDEN").is_ok() {
            std::fs::write(&types_golden_path, type_defs.trim().to_string() + "\n")
                .unwrap_or_else(|e| panic!("Cannot write {}: {e}", types_golden_path.display()));
        } else if types_golden_path.exists() {
            let expected = std::fs::read_to_string(&types_golden_path).unwrap();
            assert_eq!(
                type_defs.trim(),
                expected.trim(),
                "Fragment mismatch: type_defs ({lang:?})\n--- expected: {}\n+++ generated",
                types_golden_path.display()
            );
        } else {
            panic!(
                "Fragment golden not found: {}  (run with UPDATE_GOLDEN=1)",
                types_golden_path.display()
            );
        }
    }

    // ── Layer 2: Structural assertions ─────────────────────────
    assert_inline_structural(&member_fns, &type_defs, lang, scxml_name);

    // ── Layer 3: Template integration + compile gate ───────────
    let tdir = sce_build::find_template_dir_for(lang);
    let output = sce_build::compile_scxml_lang(scxml_path.to_str().unwrap(), &tdir, lang)
        .unwrap_or_else(|e| panic!("Statechart codegen ({lang:?}) failed for {scxml_name}: {e}"));

    let full_code = &output.files[0].1;

    // Verify fragments are actually embedded in the generated statechart
    assert!(
        full_code.contains("SCE Forge: Inline"),
        "Inline kind marker missing in full statechart ({lang:?})"
    );

    // Compile gate: Rust only (syn is available; Go/Kotlin would need external toolchains)
    if matches!(lang, sce_build::generator::Language::Rust) {
        syn::parse_file(full_code).unwrap_or_else(|e| {
            panic!("Generated Rust statechart with inline kinds fails syn parse: {e}")
        });
    }
}

/// Language-specific structural assertions on inline kind output.
/// Verifies idiomatic patterns that byte-comparison alone cannot catch
/// (e.g. a missing `self.` prefix would still match a stale golden).
fn assert_inline_structural(
    member_fns: &str,
    type_defs: &str,
    lang: sce_build::generator::Language,
    scxml_name: &str,
) {
    match scxml_name {
        "inline_mixed" => assert_inline_mixed_structural(member_fns, type_defs, lang),
        "inline_codec" => assert_inline_codec_structural(member_fns, type_defs, lang),
        _ => {} // Other fixtures rely on byte-golden comparison alone.
    }
}

fn assert_inline_mixed_structural(
    member_fns: &str,
    type_defs: &str,
    lang: sce_build::generator::Language,
) {
    use sce_build::generator::Language;
    match lang {
        Language::Kotlin => {
            // Nested enum
            assert!(
                member_fns.contains("enum class RpmStatus"),
                "Kotlin: missing nested enum class"
            );
            // when expression
            assert!(
                member_fns.contains("= when ("),
                "Kotlin: missing when expression in lookup"
            );
            // Kotlin-idiomatic function signatures
            assert!(
                member_fns.contains("fun isReady(): Boolean ="),
                "Kotlin: missing condition function"
            );
            assert!(
                member_fns.contains("fun computeToFahrenheit(): Double ="),
                "Kotlin: missing transform function"
            );
        }
        Language::Rust => {
            // Module-level enum in type_defs
            assert!(
                type_defs.contains("pub enum RpmStatus"),
                "Rust: missing enum in type_defs"
            );
            assert!(
                type_defs.contains("#[derive(Debug, Clone, Copy, PartialEq)]"),
                "Rust: missing derives on enum"
            );
            // self. prefix for member access
            assert!(
                member_fns.contains("self."),
                "Rust: missing self. prefix for member access"
            );
            // Idiomatic signatures
            assert!(
                member_fns.contains("pub fn is_ready(&self) -> bool"),
                "Rust: missing condition function signature"
            );
            assert!(
                member_fns.contains("pub fn compute_to_fahrenheit(&self) -> f64"),
                "Rust: missing transform function signature"
            );
            // match expression in lookup
            assert!(
                member_fns.contains("match raw"),
                "Rust: missing match in lookup"
            );
        }
        Language::Go => {
            // Package-level type in type_defs
            assert!(
                type_defs.contains("type RpmStatus int"),
                "Go: missing type in type_defs"
            );
            assert!(
                type_defs.contains("RpmStatus = iota"),
                "Go: missing iota const block"
            );
            // p. receiver prefix for member access
            assert!(
                member_fns.contains("p."),
                "Go: missing p. receiver prefix for member access"
            );
            // Exported method with receiver
            assert!(
                member_fns.contains("func (p *"),
                "Go: missing receiver method"
            );
            // Package-level lookup (no receiver)
            assert!(
                member_fns.contains("func LookupRpmStatus("),
                "Go: missing package-level lookup function"
            );
        }
        Language::C11 => {
            // RFC §5.J.2 Phase F. Verifies idiomatic C11 emit shape that
            // byte-comparison alone cannot catch (e.g. a missing `_st->`
            // prefix or wrong typedef shape would still match a stale
            // golden written from a buggy renderer).

            // Top-level enum typedef (no nesting in C)
            assert!(
                member_fns.contains("typedef enum"),
                "C11: missing typedef enum for lookup"
            );
            assert!(
                member_fns.contains("} inline_mixed_rpm_status_t;"),
                "C11: missing snake_case typedef name"
            );
            // Prefixed enum constants (no namespacing in C)
            assert!(
                member_fns.contains("INLINE_MIXED_RPM_STATUS_OFF"),
                "C11: missing prefixed enum constant"
            );
            assert!(
                member_fns.contains("INLINE_MIXED_RPM_STATUS_RUNNING"),
                "C11: missing prefixed enum constant"
            );
            // _st-> member access (procedure D14a mirror)
            assert!(
                member_fns.contains("_st->"),
                "C11: missing _st-> prefix for policy member access"
            );
            // Free-standing static inline functions
            assert!(
                member_fns.contains("static inline bool inline_mixed_is_ready("),
                "C11: missing condition function signature"
            );
            assert!(
                member_fns.contains("static inline double inline_mixed_compute_to_fahrenheit("),
                "C11: missing transform function signature"
            );
            assert!(
                member_fns.contains(
                    "static inline inline_mixed_rpm_status_t inline_mixed_lookup_rpm_status("
                ),
                "C11: missing lookup function signature"
            );
            // const policy pointer parameter
            assert!(
                member_fns.contains("(const inline_mixed_policy_t *_st)"),
                "C11: missing const policy pointer parameter"
            );
        }
        _ => {}
    }
}

/// Structural assertions for `inline_codec.scxml` (Phase F-2). The codec
/// inline kind emits a payload struct + (de)serialization pair — these
/// idiomatic patterns differ enough across languages that byte-compare
/// alone would miss e.g. a forgotten `companion object` in Kotlin or a
/// missing `_encoded_t` envelope in C11.
fn assert_inline_codec_structural(
    member_fns: &str,
    type_defs: &str,
    lang: sce_build::generator::Language,
) {
    use sce_build::generator::Language;
    match lang {
        Language::Kotlin => {
            assert!(
                member_fns.contains("data class Frame("),
                "Kotlin: missing data class for inline codec"
            );
            assert!(
                member_fns.contains("companion object"),
                "Kotlin: missing companion object hosting decode"
            );
            assert!(
                member_fns.contains("fun decode(cursor: com.sce.forge.runtime.SceCursor): Frame?"),
                "Kotlin: missing cursor-based decode signature"
            );
            assert!(
                member_fns.contains("fun encode(): ByteArray = byteArrayOf("),
                "Kotlin: missing encode signature"
            );
        }
        Language::Rust => {
            assert!(
                type_defs.contains("pub struct Frame"),
                "Rust: missing pub struct in type_defs"
            );
            assert!(
                type_defs.contains("#[derive(Debug, Clone)]"),
                "Rust: missing derives on codec struct"
            );
            assert!(
                type_defs.contains(
                    "pub fn decode(cursor: &mut ::sce_forge_runtime::codec::SceCursor<'_>) -> \
                 Result<Self, ::sce_forge_runtime::codec::CodecError>"
                ),
                "Rust: missing cursor-based decode signature"
            );
            assert!(
                type_defs.contains("pub fn encode(&self) -> Vec<u8>"),
                "Rust: missing encode signature"
            );
        }
        Language::Go => {
            assert!(
                type_defs.contains("type Frame struct"),
                "Go: missing struct in type_defs"
            );
            assert!(
                type_defs.contains("func DecodeFrame(cursor *codec.SceCursor) (*Frame, error)"),
                "Go: missing cursor-based exported Decode function"
            );
            assert!(
                type_defs.contains("func (s *Frame) Encode() []byte"),
                "Go: missing receiver Encode method"
            );
        }
        Language::C11 => {
            assert!(
                member_fns.contains("#define INLINE_CODEC_FRAME_MIN_BYTES 4"),
                "C11: missing min-bytes macro"
            );
            assert!(
                member_fns.contains("} inline_codec_frame_t;"),
                "C11: missing payload typedef"
            );
            assert!(
                member_fns.contains("} inline_codec_frame_encoded_t;"),
                "C11: missing encoded envelope typedef"
            );
            assert!(
                member_fns.contains(
                    "static inline sce_forge_codec_status_t inline_codec_frame_decode(\
                 sce_forge_cursor_t *cursor, inline_codec_frame_t *out)"
                ),
                "C11: missing cursor-based decode signature"
            );
            assert!(
                member_fns.contains(
                    "static inline inline_codec_frame_encoded_t \
                 inline_codec_frame_encode(const inline_codec_frame_t *self)"
                ),
                "C11: missing encode signature"
            );
            // self->{snake} member access on encode side
            assert!(
                member_fns.contains("self->msg_id"),
                "C11: missing self-> prefix on encode field access"
            );
        }
        _ => {}
    }
}

// ── Algorithm conformance (RFC §5.A, Phase A3) ────────────────

/// RFC §5.B B2-test-vector Cpp closure: the algorithm body itself
/// stays byte-stable against its prior golden — the closure only
/// adds a sidecar emission, so the primary algorithm output stays
/// identical to the pre-test-vector form.
#[test]
fn forge_cpp_algorithm_crc16() {
    assert_standalone_forge("algorithm_crc16", "algorithm_crc16.h");
}

/// RFC §5.B B2-test-vector Cpp closure: pin the per-fixture sidecar
/// (`<fixture>_test.h`) emitted next to the algorithm header. The
/// Cpp conformance harness folds the returned failure count into
/// `g_failures` from main() (mirrors the C11 contract).
#[test]
fn forge_cpp_algorithm_crc16_test_vector_sidecar() {
    assert_sidecar_forge_lang(
        "algorithm_crc16",
        "algorithm_crc16_test.h",
        sce_build::generator::Language::Cpp,
    );
}

// ── RFC §5.B B5-θ codec test-vector sidecars (Rust + C11 trunk) ───
//
// 3 fixtures × 2 backends = 6 positive sidecar emissions; each row
// builds the codec struct from declared `<sce:decoded>` field
// values, encodes → asserts byte parity vs the row's `hex`, decodes
// → asserts every field round-trips. Cpp / Kotlin / Go / Python
// stay in the gate-rejection bucket (12 negative tests below) until
// per-language B5-θ closures lift the trunk gate.

#[test]
fn forge_rust_codec_zenoh_close_test_vector_sidecar() {
    assert_sidecar_forge_lang(
        "codec_zenoh_close",
        "codec_zenoh_close_test.rs",
        sce_build::generator::Language::Rust,
    );
}

#[test]
fn forge_c11_codec_zenoh_close_test_vector_sidecar() {
    assert_sidecar_forge_lang(
        "codec_zenoh_close",
        "codec_zenoh_close_test.c.h",
        sce_build::generator::Language::C11,
    );
}

#[test]
fn forge_rust_codec_zenoh_frame_test_vector_sidecar() {
    assert_sidecar_forge_lang(
        "codec_zenoh_frame",
        "codec_zenoh_frame_test.rs",
        sce_build::generator::Language::Rust,
    );
}

#[test]
fn forge_c11_codec_zenoh_frame_test_vector_sidecar() {
    assert_sidecar_forge_lang(
        "codec_zenoh_frame",
        "codec_zenoh_frame_test.c.h",
        sce_build::generator::Language::C11,
    );
}

#[test]
fn forge_rust_codec_zenoh_locator_test_vector_sidecar() {
    assert_sidecar_forge_lang(
        "codec_zenoh_locator",
        "codec_zenoh_locator_test.rs",
        sce_build::generator::Language::Rust,
    );
}

#[test]
fn forge_c11_codec_zenoh_locator_test_vector_sidecar() {
    assert_sidecar_forge_lang(
        "codec_zenoh_locator",
        "codec_zenoh_locator_test.c.h",
        sce_build::generator::Language::C11,
    );
}

// ── B5-θ trunk gate-rejection: 4 backends × 3 fixtures = 12 ────
//
// Each test asserts that the named codec compiles cleanly to its
// primary file but emits NO sidecar on the targeted backend. When
// per-language closures land, each test rotates to its matching
// `assert_sidecar_forge_lang(...)` call (mirrors B1-β / B5-γ /
// B5-ε / B5-ζ trunk-then-closures rotation pattern).

#[test]
fn forge_codec_zenoh_close_cpp_no_sidecar_until_closure() {
    assert_no_codec_sidecar_until_closure("codec_zenoh_close", sce_build::generator::Language::Cpp);
}

#[test]
fn forge_codec_zenoh_close_kotlin_no_sidecar_until_closure() {
    assert_no_codec_sidecar_until_closure(
        "codec_zenoh_close",
        sce_build::generator::Language::Kotlin,
    );
}

#[test]
fn forge_codec_zenoh_close_go_no_sidecar_until_closure() {
    assert_no_codec_sidecar_until_closure("codec_zenoh_close", sce_build::generator::Language::Go);
}

#[test]
fn forge_codec_zenoh_close_python_no_sidecar_until_closure() {
    assert_no_codec_sidecar_until_closure(
        "codec_zenoh_close",
        sce_build::generator::Language::Python,
    );
}

#[test]
fn forge_codec_zenoh_frame_cpp_no_sidecar_until_closure() {
    assert_no_codec_sidecar_until_closure("codec_zenoh_frame", sce_build::generator::Language::Cpp);
}

#[test]
fn forge_codec_zenoh_frame_kotlin_no_sidecar_until_closure() {
    assert_no_codec_sidecar_until_closure(
        "codec_zenoh_frame",
        sce_build::generator::Language::Kotlin,
    );
}

#[test]
fn forge_codec_zenoh_frame_go_no_sidecar_until_closure() {
    assert_no_codec_sidecar_until_closure("codec_zenoh_frame", sce_build::generator::Language::Go);
}

#[test]
fn forge_codec_zenoh_frame_python_no_sidecar_until_closure() {
    assert_no_codec_sidecar_until_closure(
        "codec_zenoh_frame",
        sce_build::generator::Language::Python,
    );
}

#[test]
fn forge_codec_zenoh_locator_cpp_no_sidecar_until_closure() {
    assert_no_codec_sidecar_until_closure(
        "codec_zenoh_locator",
        sce_build::generator::Language::Cpp,
    );
}

#[test]
fn forge_codec_zenoh_locator_kotlin_no_sidecar_until_closure() {
    assert_no_codec_sidecar_until_closure(
        "codec_zenoh_locator",
        sce_build::generator::Language::Kotlin,
    );
}

#[test]
fn forge_codec_zenoh_locator_go_no_sidecar_until_closure() {
    assert_no_codec_sidecar_until_closure(
        "codec_zenoh_locator",
        sce_build::generator::Language::Go,
    );
}

#[test]
fn forge_codec_zenoh_locator_python_no_sidecar_until_closure() {
    assert_no_codec_sidecar_until_closure(
        "codec_zenoh_locator",
        sce_build::generator::Language::Python,
    );
}

// ── B5-ι sidecar emit (Rust + C11) + 4 backend gate ─────────────
//
// codec_zenoh_fragment carries inline `<sce:test-vector>` rows so the
// per-fixture sidecar `*_test.rs` / `*_test.c.h` is emitted on Rust +
// C11. cpp/kotlin/go/python remain gated until B5-θ closures land
// (mirrors the close/frame/locator rotation pattern from B5-θ trunk).
// codec_zenoh_decl_final has an empty body and no test vectors, so it emits
// no sidecar on any backend. codec_zenoh_open_body's gated cookie
// pair precludes inline `<sce:test-vector>` until B5-θ-optional lands
// absent-vs-present markers (same constraint as codec_zenoh_scout).

#[test]
fn forge_rust_codec_zenoh_fragment_test_vector_sidecar() {
    assert_sidecar_forge_lang(
        "codec_zenoh_fragment",
        "codec_zenoh_fragment_test.rs",
        sce_build::generator::Language::Rust,
    );
}

#[test]
fn forge_c11_codec_zenoh_fragment_test_vector_sidecar() {
    assert_sidecar_forge_lang(
        "codec_zenoh_fragment",
        "codec_zenoh_fragment_test.c.h",
        sce_build::generator::Language::C11,
    );
}

#[test]
fn forge_codec_zenoh_fragment_cpp_no_sidecar_until_closure() {
    assert_no_codec_sidecar_until_closure(
        "codec_zenoh_fragment",
        sce_build::generator::Language::Cpp,
    );
}

#[test]
fn forge_codec_zenoh_fragment_kotlin_no_sidecar_until_closure() {
    assert_no_codec_sidecar_until_closure(
        "codec_zenoh_fragment",
        sce_build::generator::Language::Kotlin,
    );
}

#[test]
fn forge_codec_zenoh_fragment_go_no_sidecar_until_closure() {
    assert_no_codec_sidecar_until_closure(
        "codec_zenoh_fragment",
        sce_build::generator::Language::Go,
    );
}

#[test]
fn forge_codec_zenoh_fragment_python_no_sidecar_until_closure() {
    assert_no_codec_sidecar_until_closure(
        "codec_zenoh_fragment",
        sce_build::generator::Language::Python,
    );
}

// ── RFC §5.B B5-κ Surface L sidecar emit (primitive demo) ──────
//
// codec_length_ref_dotted_basic carries inline `<sce:test-vector>`
// rows, so the per-fixture sidecar emits on Rust + C11. cpp/kotlin/
// go/python remain gated until B5-θ closures land (rotating gate
// pattern from B5-θ trunk). codec_zenoh_scout has no inline
// test-vectors (B5-θ doesn't yet support absent-vs-present markers
// for present-if codecs — `<sce:test-vector>` was rejected until that
// closure), so its sidecar is uniformly empty across all 6 backends.

#[test]
fn forge_rust_codec_length_ref_dotted_basic_test_vector_sidecar() {
    assert_sidecar_forge_lang(
        "codec_length_ref_dotted_basic",
        "codec_length_ref_dotted_basic_test.rs",
        sce_build::generator::Language::Rust,
    );
}

#[test]
fn forge_c11_codec_length_ref_dotted_basic_test_vector_sidecar() {
    assert_sidecar_forge_lang(
        "codec_length_ref_dotted_basic",
        "codec_length_ref_dotted_basic_test.c.h",
        sce_build::generator::Language::C11,
    );
}

#[test]
fn forge_codec_length_ref_dotted_basic_cpp_no_sidecar_until_closure() {
    assert_no_codec_sidecar_until_closure(
        "codec_length_ref_dotted_basic",
        sce_build::generator::Language::Cpp,
    );
}

#[test]
fn forge_codec_length_ref_dotted_basic_kotlin_no_sidecar_until_closure() {
    assert_no_codec_sidecar_until_closure(
        "codec_length_ref_dotted_basic",
        sce_build::generator::Language::Kotlin,
    );
}

#[test]
fn forge_codec_length_ref_dotted_basic_go_no_sidecar_until_closure() {
    assert_no_codec_sidecar_until_closure(
        "codec_length_ref_dotted_basic",
        sce_build::generator::Language::Go,
    );
}

#[test]
fn forge_codec_length_ref_dotted_basic_python_no_sidecar_until_closure() {
    assert_no_codec_sidecar_until_closure(
        "codec_length_ref_dotted_basic",
        sce_build::generator::Language::Python,
    );
}

// ── §5.F build-time const-fold (Phase A4-β — host interpreter) ──

/// RFC §5.F α-residual contract: parser still produces the
/// `<sce:fold>` IR shape that the host interpreter consumes in β.
/// Pins three structural contracts the evaluator depends on:
///   1. `array<u16, N>` outer-type syntax accepted (Rust-style alias).
///   2. `<sce:fold range="START..END" as="i" elem-type="...">` body
///      parses with the algorithm-statement vocabulary inside it.
///   3. `<sce:yield expr="..."/>` is the fold's terminal child.
#[test]
fn forge_algorithm_const_fold_smoke_parses() {
    use sce_build::forge::model::{AlgorithmConstType, ForgeDocument, SceType};
    use sce_build::forge::parser::parse_forge;
    use sce_build::DocumentLabel;

    let scxml_path = resource_dir().join("algorithm_const_fold_smoke.scxml");
    let content = std::fs::read_to_string(&scxml_path).expect("read fixture");

    let doc = parse_forge(
        &content,
        DocumentLabel::symmetric("algorithm_const_fold_smoke"),
    )
    .expect("fold-form const must parse cleanly")
    .expect("fixture is sce:kind=\"algorithm\"");
    let alg = match doc {
        ForgeDocument::Algorithm(m) => m,
        other => panic!("expected Algorithm doc, got {:?}", other.kind()),
    };

    assert_eq!(alg.consts.len(), 1, "fixture declares exactly one const");
    let c = &alg.consts[0];
    assert_eq!(c.name, "DOUBLED");
    assert!(
        c.compute_at_build,
        "compute_at_build must be set when <sce:fold> body is present"
    );
    assert!(
        c.init.is_none(),
        "fold-form const must not carry init=; got {:?}",
        c.init
    );

    match &c.sce_type {
        AlgorithmConstType::Array { elem, len } => {
            assert_eq!(
                *elem,
                SceType::Uint16,
                "Rust-style `u16` alias must map to Uint16"
            );
            assert_eq!(*len, 4, "array<u16, 4> declared length");
        }
        other => panic!("fold-form const must carry array shape, got {other:?}"),
    }

    let fold = c.fold.as_ref().expect("fold-form const carries FoldBody");
    assert_eq!(fold.range_start, 0);
    assert_eq!(fold.range_end, 4);
    assert_eq!(fold.iter_var, "i");
    assert_eq!(fold.elem_type, SceType::Uint16);
    assert_eq!(fold.yield_expr, "doubled");
    assert_eq!(
        fold.body.len(),
        1,
        "fixture's fold body has one <sce:var> before <sce:yield>"
    );
}

/// RFC §5.F β happy-path: codegen on the smoke fixture lifts the
/// fold-form const to a per-language static array literal. Two
/// language goldens (Rust + Cpp) pin the §5.J.5 emit syntax — the
/// host interpreter is single-source, so the underlying numeric
/// data is byte-equivalent across backends by construction.
#[test]
fn forge_const_fold_smoke_emits_rust() {
    assert_standalone_forge_rust(
        "algorithm_const_fold_smoke",
        "algorithm_const_fold_smoke.rs",
    );
}

#[test]
fn forge_const_fold_smoke_emits_cpp() {
    assert_standalone_forge("algorithm_const_fold_smoke", "algorithm_const_fold_smoke.h");
}

/// RFC §5.F β acceptance fixture: CRC16-CCITT-FALSE in
/// **table form** with a 256-entry build-time-evaluated table. Two
/// language goldens pin the cross-backend emit shape; the
/// underlying `crc16_ccitt_false_table_matches_reference` unit
/// test in `forge::const_fold` proves byte-equivalence with the
/// canonical CRC16-CCITT-FALSE reference table at the
/// host-interpreter layer (RFC §A6 cross-language runtime
/// equivalence remains scheduled for A4-A5-A6).
#[test]
fn forge_algorithm_crc16_table_rust() {
    assert_standalone_forge_rust("algorithm_crc16_table", "algorithm_crc16_table.rs");
}

#[test]
fn forge_algorithm_crc16_table_cpp() {
    assert_standalone_forge("algorithm_crc16_table", "algorithm_crc16_table.h");
}

/// RFC §5.F γ wire codes: each of the three `algorithm/const-*`
/// diagnostics maps a §5.F failure mode onto a typed
/// [`GenerateError`] variant. β shipped these as
/// `generate/unsupported-feature` slug payloads; γ promotes them to
/// first-class wire codes so consumers dispatch on the structured
/// `ForgeError` variant rather than substring-matching the message.
///
/// `algorithm/const-fold-budget-exceeded` — an iteration budget set
/// below the fold's element count surfaces as `ConstFoldBudgetExceeded`
/// with the configured `budget` quoted verbatim. Pins the
/// `--const-fold-budget=N` plumbing through
/// `ForgeCompileOptions::const_fold_budget`.
#[test]
fn forge_const_fold_budget_exceeded_rejects_oversized_fold() {
    use sce_build::forge::error::{ForgeError, GenerateError};

    let scxml_path = resource_dir().join("algorithm_const_fold_smoke.scxml");
    let content = std::fs::read_to_string(&scxml_path).expect("read fixture");

    let opts = sce_build::ForgeCompileOptions {
        const_fold_budget: Some(2), // smoke fixture has 4 elements
        ..Default::default()
    };
    let result = sce_build::compile_forge_with_imports(
        &content,
        sce_build::DocumentLabel::symmetric("algorithm_const_fold_smoke"),
        sce_build::generator::Language::Rust,
        scxml_path.parent().unwrap(),
        &opts,
    );
    let err = match result {
        Ok(_) => panic!("budget=2 must reject the smoke fixture's 4-element fold"),
        Err(e) => e,
    };
    assert!(
        matches!(
            err.error,
            ForgeError::Generate(GenerateError::ConstFoldBudgetExceeded { budget: 2, .. })
        ),
        "budget-exceeded error must surface as the typed variant; got: {err:?}"
    );
}

/// `algorithm/const-not-foldable` — fold body referencing an
/// identifier outside fold scope cannot reduce to a build-time value.
#[test]
fn forge_const_not_foldable_rejects_unscoped_ident() {
    use sce_build::forge::error::{ForgeError, GenerateError};

    let scxml_path = resource_dir().join("algorithm_const_not_foldable.scxml");
    let content = std::fs::read_to_string(&scxml_path).expect("read fixture");

    let result = sce_build::compile_forge_with_imports(
        &content,
        sce_build::DocumentLabel::symmetric("algorithm_const_not_foldable"),
        sce_build::generator::Language::Rust,
        scxml_path.parent().unwrap(),
        &sce_build::ForgeCompileOptions::default(),
    );
    let err = match result {
        Ok(_) => panic!("unscoped ident in fold body must reject"),
        Err(e) => e,
    };
    assert!(
        matches!(
            err.error,
            ForgeError::Generate(GenerateError::ConstNotFoldable { ref detail, .. })
                if detail.contains("unbound_outer")
        ),
        "unscoped-ident failure must surface as ConstNotFoldable naming the offender; got: {err:?}"
    );
}

/// `algorithm/const-yield-type-mismatch` — a fold whose yield
/// expression produces a float cannot be coerced into a uint16 slot.
#[test]
fn forge_const_yield_type_mismatch_rejects_float_into_uint() {
    use sce_build::forge::error::{ForgeError, GenerateError};
    use sce_build::forge::model::SceType;

    let scxml_path = resource_dir().join("algorithm_const_yield_type_mismatch.scxml");
    let content = std::fs::read_to_string(&scxml_path).expect("read fixture");

    let result = sce_build::compile_forge_with_imports(
        &content,
        sce_build::DocumentLabel::symmetric("algorithm_const_yield_type_mismatch"),
        sce_build::generator::Language::Rust,
        scxml_path.parent().unwrap(),
        &sce_build::ForgeCompileOptions::default(),
    );
    let err = match result {
        Ok(_) => panic!("float-into-uint16 yield must reject"),
        Err(e) => e,
    };
    assert!(
        matches!(
            err.error,
            ForgeError::Generate(GenerateError::ConstYieldTypeMismatch {
                expected: SceType::Uint16,
                ref actual,
                ..
            }) if actual == "float"
        ),
        "float-yield must surface as typed ConstYieldTypeMismatch with expected=Uint16; got: {err:?}"
    );
}

// ── Transform conformance (3 tests) ────────────────────────────

#[test]
fn forge_transform_temperature() {
    assert_standalone_forge("transform_temperature", "transform_temperature.h");
}

#[test]
fn forge_transform_multi_output() {
    assert_standalone_forge("transform_multi_output", "transform_multi_output.h");
}

#[test]
fn forge_transform_bitwise() {
    assert_standalone_forge("transform_bitwise", "transform_bitwise.h");
}

// ── Lookup conformance (3 tests) ──────────────────────────────

#[test]
fn forge_lookup_engine_status() {
    assert_standalone_forge("lookup_engine_status", "lookup_engine_status.h");
}

#[test]
fn forge_lookup_gear_position() {
    assert_standalone_forge("lookup_gear_position", "lookup_gear_position.h");
}

#[test]
fn forge_lookup_single_default() {
    assert_standalone_forge("lookup_single_default", "lookup_single_default.h");
}

// Numeric-output lookup with on-miss=error: shares the lookup kind but
// triggers the parallel-array codegen branch instead of enum dispatch.
#[test]
fn forge_lookup_alarm_code() {
    assert_standalone_forge("lookup_alarm_code", "lookup_alarm_code.h");
}

#[test]
fn forge_lookup_state_action() {
    assert_standalone_forge("lookup_state_action", "lookup_state_action.h");
}

#[test]
fn forge_lookup_unit_scale() {
    assert_standalone_forge("lookup_unit_scale", "lookup_unit_scale.h");
}

#[test]
fn forge_lookup_severity_default() {
    assert_standalone_forge("lookup_severity_default", "lookup_severity_default.h");
}

// ── Condition conformance (3 tests) ───────────────────────────

#[test]
fn forge_condition_programming() {
    assert_standalone_forge("condition_programming", "condition_programming.h");
}

#[test]
fn forge_condition_threshold() {
    assert_standalone_forge("condition_threshold", "condition_threshold.h");
}

#[test]
fn forge_condition_range() {
    assert_standalone_forge("condition_range", "condition_range.h");
}

// ── Codec conformance (3 tests) ──────────────────────────────

#[test]
fn forge_codec_simple_frame() {
    assert_standalone_forge("codec_simple_frame", "codec_simple_frame.h");
}

#[test]
fn forge_codec_little_endian() {
    assert_standalone_forge("codec_little_endian", "codec_little_endian.h");
}

#[test]
fn forge_codec_subbyte() {
    assert_standalone_forge("codec_subbyte", "codec_subbyte.h");
}

#[test]
fn forge_codec_tail() {
    assert_standalone_forge("codec_tail", "codec_tail.h");
}

#[test]
fn forge_codec_length_ref() {
    assert_standalone_forge("codec_length_ref", "codec_length_ref.h");
}

#[test]
fn forge_codec_vle_zint_u64() {
    assert_standalone_forge("codec_vle_zint_u64", "codec_vle_zint_u64.h");
}

// ── RFC §5.B B5-prep Zenoh transport-message body codecs ────
// First reachable downstream consumers of B1-B4 primitives from
// the watching-zenoh authoring path (RFC §7 Phase B sequence).
// Pure composition — no new IR / parser / template surface.
// Validates that B-phase primitives compose for actual Zenoh
// transport-message body shapes. Upstream parity:
// `_z_close_encode` / `_z_frame_encode` in zenoh-pico
// src/protocol/codec/transport.c. KeepAlive's empty body defers
// — `parser.rs` rejects zero-field codecs (EmptyCollection),
// which is its own primitive concern.

#[test]
fn forge_codec_zenoh_close_cpp() {
    assert_standalone_forge("codec_zenoh_close", "codec_zenoh_close.h");
}

#[test]
fn forge_codec_zenoh_frame_cpp() {
    assert_standalone_forge("codec_zenoh_frame", "codec_zenoh_frame.h");
}

// ── RFC §5.B B5-ι cross-codec composition (Cpp) ───────────────
// Cross-codec composition fixtures from the watching-zenoh authoring
// path (RFC §7 Phase B). Pure composition of B1-B5 primitives — no
// new IR / parser / template surface. Each fixture mirrors an upstream
// zenoh-pico encoder; see the SCXML resource header for line refs.

#[test]
fn forge_codec_zenoh_open_body_cpp() {
    assert_standalone_forge("codec_zenoh_open_body", "codec_zenoh_open_body.h");
}

#[test]
fn forge_codec_zenoh_init_body_cpp() {
    assert_standalone_forge("codec_zenoh_init_body", "codec_zenoh_init_body.h");
}

#[test]
fn forge_codec_zenoh_join_cpp() {
    assert_standalone_forge("codec_zenoh_join", "codec_zenoh_join.h");
}

#[test]
fn forge_codec_zenoh_fragment_cpp() {
    assert_standalone_forge("codec_zenoh_fragment", "codec_zenoh_fragment.h");
}

#[test]
fn forge_codec_zenoh_decl_final_cpp() {
    assert_standalone_forge("codec_zenoh_decl_final", "codec_zenoh_decl_final.h");
}

// ── RFC §5.B B5-κ Surface L dotted-path length-field (cpp) ─────
// Author writes `sce:length-field="<carrier>.<flag>"` to source the
// byte count of a length-ref payload from a multi-bit flag subfield
// inside a flags-bearing carrier — mirrors B1-δ present-if's
// dotted-path grammar exactly. First reachable consumer:
// codec_zenoh_scout where zenoh-pico packs zid_len_m1 into cbyte's
// high nibble alongside what / I bits.

#[test]
fn forge_codec_length_ref_dotted_basic_cpp() {
    assert_standalone_forge(
        "codec_length_ref_dotted_basic",
        "codec_length_ref_dotted_basic.h",
    );
}

#[test]
fn forge_codec_zenoh_scout_cpp() {
    assert_standalone_forge("codec_zenoh_scout", "codec_zenoh_scout.h");
}

// ── RFC §5.B B5-α multi-bit flag accessor + empty-codec lift ──
// `codec_qos_byte` mirrors zenoh's `_z_n_qos_t._val`: a uint8 carrier
// with five bit-ranges (priority:3 + reliable:1 + congestion:2 +
// express:1 + reserved:1). Multi-bit ranges (priority/congestion)
// emit uint8-typed get/set; single-bit ranges (reliable/express/
// reserved) keep the bool shape from B1-γ. `codec_zenoh_keep_alive`
// is the first reachable consumer of the empty-body lift — the
// surrounding 1-byte transport-message header (MID + flags) marks
// the message; the body codec emits zero wire bytes.

#[test]
fn forge_codec_qos_byte_cpp() {
    assert_standalone_forge("codec_qos_byte", "codec_qos_byte.h");
}

#[test]
fn forge_codec_zenoh_keep_alive_cpp() {
    assert_standalone_forge("codec_zenoh_keep_alive", "codec_zenoh_keep_alive.h");
}

// ── RFC §5.B B1-γ flags primitive ────────────────────────────
// `codec_flags_basic` declares a uint8 carrier `header` with four
// named bits; codegen emits per-flag get/set accessors while the wire
// layout is unchanged from a plain uint8 field. The fixture mirrors
// the Zenoh Fragment header (reliable / more / drop / first) — see
// watching-zenoh/docs/wire-spec-subset.md §4.2 for the upstream shape.

#[test]
fn forge_codec_flags_basic() {
    assert_standalone_forge("codec_flags_basic", "codec_flags_basic.h");
}

// ── RFC §5.B B1-δ present-if primitive (trunk) ──────────────
// `codec_present_if_basic` declares a uint8 flags carrier with one
// named bit `has_seq` followed by a uint16 BE field gated on that
// bit. Trunk ships Rust + Cpp byte-stable goldens; Kotlin / Go /
// C11 / Python land in B1-δ closures (each errors with
// `generate/unsupported-feature` until then). The fixture round-
// trips two oracle frames: `set_has_seq(true) + seq=Some(0xCAFE)`
// → `0x01 0xCA 0xFE`; `set_has_seq(false) + seq=None` → `0x00`.

#[test]
fn forge_codec_present_if_basic() {
    assert_standalone_forge("codec_present_if_basic", "codec_present_if_basic.h");
}

// ── RFC §5.B B5-λ present-if negation primitive (Cpp) ───────
// `codec_present_if_negation` mirrors `codec_present_if_basic`'s
// shape but inverts the polarity: the trailing seq field is gated
// on `!flags.absent_seq` so it's present iff the named bit is
// CLEAR. Encode/decode emit `(carrier & mask) == 0` instead of
// `!= 0`. Closes the grammar gap that previously forced
// specialized codec authoring (e.g. body-when-A=0 carve-out for
// codec_zenoh_open_ack) when wire layouts use "set means absent"
// semantics. v1 trunk ships byte-stable goldens on all 6
// backends in one commit (parser change is a single `op`
// substitution per language; no per-language closure phasing).

#[test]
fn forge_codec_present_if_negation() {
    assert_standalone_forge("codec_present_if_negation", "codec_present_if_negation.h");
}

// ── RFC §5.B Y3 atomic 2b-ii present-if disjunction primitive (Cpp) ──
// `codec_present_if_disjunction` mirrors the basic / negation
// fixtures' shape but uses an OR chain — the trailing seq field is
// gated on `flags.wants_a || flags.wants_b`. Each clause contributes
// its own `(carrier & MASK) != 0` test, joined by the per-language
// logical-OR token (`||` for 5 backends, `or` for Python). Closes
// the deferral comment on `PresentIfPredicate` ("Conjunction and
// equality remain deferred to later B-stages when a reachable
// consumer surfaces") for the disjunction half — zenoh-pico interest
// is the surfaced consumer (`_Z_INTEREST_NOT_FINAL_MASK = (CURRENT
// | FUTURE)`, interest.h:35), exercised at codec level by
// codec_zenoh_interest in the same atomic.

#[test]
fn forge_codec_present_if_disjunction() {
    assert_standalone_forge(
        "codec_present_if_disjunction",
        "codec_present_if_disjunction.h",
    );
}

// ── RFC §5.B Y3 atomic 2b-ii peek-byte peek-byte primitive (Cpp) ──
// `codec_variant_peek_basic` exercises `<sce:peek-byte>` as a child
// of `<sce:variant>` — the cursor's NEXT byte is read without
// advancing and dispatched on a named bit-range; the arm body
// decoder reads that same byte as its own first wire byte. Models
// the Zenoh response/request body MID dispatch shape per
// `network.c:347-364`. The cross-codec validator (this atomic)
// enforces that each arm body's first `<sce:flags>` field at
// byte_offset=0 declares peek-byte's named flags identically. Arm
// body codecs `codec_peek_arm_a` / `codec_peek_arm_b` ship as
// standalone fixtures so each is byte-golden-pinned independently
// (the demo composes them via `<sce:import>`).

#[test]
fn forge_codec_peek_arm_a() {
    assert_standalone_forge("codec_peek_arm_a", "codec_peek_arm_a.h");
}

#[test]
fn forge_codec_peek_arm_b() {
    assert_standalone_forge("codec_peek_arm_b", "codec_peek_arm_b.h");
}

#[test]
fn forge_codec_variant_peek_basic() {
    assert_standalone_forge("codec_variant_peek_basic", "codec_variant_peek_basic.h");
}

// ── RFC §5.B Y3 atomic 2b-ii peek-byte first realistic peek-byte
// consumer: codec_zenoh_response (Cpp). Network-layer envelope
// wrapping reply/err inner bodies — the inner body's own header
// MID (Z_REPLY = 0x04 / Z_ERR = 0x05) identifies which arm body
// codec consumes the trailing wire bytes. Mirrors zenoh-pico
// `_z_response_encode/decode` (network.c). Composes Y3 atomic 2b-i
// sub-codecs (msg_reply / msg_err) under the new peek-byte
// primitive. Q4 envelope-only fidelity — inner wire detail strips
// (VLE-LSB-as-flag, bit-shifted-length-prefix, source_info nested
// ext) defer to follow-up atomic 2b-iii-{α,β,γ}.

#[test]
fn forge_codec_zenoh_response() {
    assert_standalone_forge("codec_zenoh_response", "codec_zenoh_response.h");
}

// ── RFC §5.B B2-β present-if + variable-length (Cpp) ─────────
// B2-β lifts the v1 BitSize::Fixed-only restriction so present-if
// can gate Tail / LengthRef / Vle bit-sizes. Each fixture pairs
// a uint8 flag carrier with a single gated variable-length field;
// the encoded frame is exactly 1 byte when the flag is clear and
// expands by the payload width when set.

#[test]
fn forge_codec_present_if_tail_cpp() {
    assert_standalone_forge("codec_present_if_tail", "codec_present_if_tail.h");
}

#[test]
fn forge_codec_present_if_length_ref_cpp() {
    assert_standalone_forge(
        "codec_present_if_length_ref",
        "codec_present_if_length_ref.h",
    );
}

#[test]
fn forge_codec_present_if_vle_cpp() {
    assert_standalone_forge("codec_present_if_vle", "codec_present_if_vle.h");
}

// ── RFC §5.B B2 repeat primitive (trunk, Cpp) ─────────────────
// `codec_repeat_basic` decodes a one-byte uint8 count prefix then
// iterates the imported `codec_repeat_elem` codec `num_frags` times.
// `codec_until_eof_basic` skips the count prefix and greedily
// consumes elements until cursor exhaustion. Both share the same
// element body (`codec_repeat_elem`, uint16 BE seq id). v1 trunk
// ships Rust + Cpp goldens; Kotlin / Go / C11 / Python land in
// per-language closures. The element body fixture also doubles as
// a standalone byte-stable reference so syn / cpp parser gates
// compile each fixture in isolation; `super::...` (Rust) /
// `::SCE::Generated::...` (Cpp) qualified paths resolve at link
// time when all three goldens are in scope.

#[test]
fn forge_codec_repeat_elem_cpp() {
    assert_standalone_forge("codec_repeat_elem", "codec_repeat_elem.h");
}

#[test]
fn forge_codec_repeat_basic_cpp() {
    assert_standalone_forge("codec_repeat_basic", "codec_repeat_basic.h");
}

#[test]
fn forge_codec_until_eof_basic_cpp() {
    assert_standalone_forge("codec_until_eof_basic", "codec_until_eof_basic.h");
}

// ── RFC §5.B B5-μ repeat-with-present-if (Wire RFC Phase B X1) ─
// `codec_repeat_present_if_basic` is the LOCAL-scope primitive demo
// of `<sce:repeat sce:present-if="<carrier>.<flag>"/>`. Mirrors B5-κ
// Surface L's two-fixture trunk pattern (primitive + realistic
// consumer); the realistic consumer is `codec_zenoh_hello`.
// Co-gating contract: the count field `num_elems` MUST carry the
// IDENTICAL `sce:present-if="carrier.has_list"` predicate.
// Wrap shape per language: Option<Vec> / std::optional<vector> /
// MutableList? / Optional[List]; Go uses bare slice nilness; C11
// keeps carrier-bit-as-truth.

#[test]
fn forge_codec_repeat_present_if_basic_cpp() {
    assert_standalone_forge(
        "codec_repeat_present_if_basic",
        "codec_repeat_present_if_basic.h",
    );
}

// `codec_zenoh_hello` is the realistic PARENT-scope consumer of B5-μ
// — mirrors zenoh-pico `_z_hello_encode` (message.c:646-664) +
// `_Z_FLAG_T_HELLO_L = 0x20` at bit 5. Body's locator-list (VLE
// num_locators + repeat<codec_zenoh_locator>) co-gates on the parent
// header's L flag. Closes the final remaining Batch 1 fixture.

#[test]
fn forge_codec_zenoh_hello_cpp() {
    assert_standalone_forge("codec_zenoh_hello", "codec_zenoh_hello.h");
}

// ── Wire RFC Phase B Y0a foundation: gated String + length-ref ─
// Lifts the B5-ζ Surface H deferral on `sce:type="string"` +
// `sce:present-if` (parser.rs:1583+ pre-Y0a) so zenoh-pico's
// wireexpr (message.c:115-145 — VLE id + has_suffix-gated UTF-8
// suffix) can be authored faithfully. Mirrors B5-μ X1's two-fixture
// trunk pattern: `codec_present_if_string` is the LOCAL-scope
// primitive demo (carrier.has_text gates uint8 text_len + UTF-8
// text) and `codec_zenoh_wireexpr` is the PARENT-scope realistic
// body codec (parent.N gates VLE suffix_len + UTF-8 suffix; N flag
// at bit 5 mirrors `_Z_DECL_KEXPR_FLAG_N = 0x20` and equivalents
// across all 8 declare sub-types). 15+ Y1/Y3/Y4 consumers will
// embed wireexpr via cross-codec parent-flag composition (B5-γ).

#[test]
fn forge_codec_present_if_string_cpp() {
    assert_standalone_forge("codec_present_if_string", "codec_present_if_string.h");
}

#[test]
fn forge_codec_zenoh_wireexpr_cpp() {
    assert_standalone_forge("codec_zenoh_wireexpr", "codec_zenoh_wireexpr.h");
}

// ── RFC §5.B Y0c — single-codec embed primitive ─────────────────────
// Closes the Y1 prerequisite gap surfaced before authoring Wire RFC
// Phase B Y1: the codec DSL had no way to compose a single imported
// codec as an inline field. Repeat / TLV-chain / variant arm all use
// codecs as container element types; embed is the missing "single
// occurrence, always present" composition primitive that mirrors
// zenoh-pico's `_z_X_t` struct fields holding a sub-struct of another
// typed codec (`_z_decl_kexpr_t._keyexpr` is the first reachable
// consumer — Y1 ships 4 such consumers via this primitive).
//
// `codec_embed_basic` locks the wire-transparent emit shape across
// 6 backends (`uint8 tag` + embedded `codec_zenoh_locator`). Y1's
// realistic consumers compose this with B5-γ parent-flag threading
// (Case B: pass-through `parent_flags`) for the wireexpr embed.

#[test]
fn forge_codec_embed_basic_cpp() {
    assert_standalone_forge("codec_embed_basic", "codec_embed_basic.h");
}

// ── RFC §5.B Wire RFC Phase B Y1 — declare/undeclare family (4
// wireexpr-bearing decls + 1 trivial undecl). All compose Y0a's
// codec_zenoh_wireexpr through Y0c embed grammar. The four
// wireexpr-bearing decls (decl_keyexpr / decl_subscriber /
// decl_queryable / decl_token) declare matching
// `<sce:requires-parent-flags carrier="header">` blocks so the body's
// parent_flags param threads through to wireexpr verbatim (Case B
// pass-through). decl_queryable additionally inlines a parent.Z-gated
// queryable_info ext (ext_type uint8 + ext_value VLE u64 packed by
// the host). undecl_keyexpr is a trivial single-VLE-id body.
//
// Y1 omits undecl_subscriber/queryable/token: they carry an optional
// ext_keyexpr via TLV ext envelope (length-prefixed outer + inline-
// suffix-by-implicit-length inner — new design surface scoped to Y0b).

#[test]
fn forge_codec_zenoh_decl_kexpr_cpp() {
    assert_standalone_forge("codec_zenoh_decl_kexpr", "codec_zenoh_decl_kexpr.h");
}

#[test]
fn forge_codec_zenoh_decl_subscriber_cpp() {
    assert_standalone_forge(
        "codec_zenoh_decl_subscriber",
        "codec_zenoh_decl_subscriber.h",
    );
}

#[test]
fn forge_codec_zenoh_decl_queryable_cpp() {
    assert_standalone_forge("codec_zenoh_decl_queryable", "codec_zenoh_decl_queryable.h");
}

#[test]
fn forge_codec_zenoh_decl_token_cpp() {
    assert_standalone_forge("codec_zenoh_decl_token", "codec_zenoh_decl_token.h");
}

#[test]
fn forge_codec_zenoh_undecl_kexpr_cpp() {
    assert_standalone_forge("codec_zenoh_undecl_kexpr", "codec_zenoh_undecl_kexpr.h");
}

// ── RFC §5.B Wire RFC Phase B Y0b — TLV envelope foundation ────
// `codec_zenoh_decl_ext_keyexpr_inner` + `codec_zenoh_decl_ext_keyexpr`
// lock the two Y0b embed-attribute lifts (sce:length-from + sce:
// present-if) as a primitive demo pair, with three undecl_*
// realistic consumers exercising the parent.Z-gated optional embed
// shape. Mirrors zenoh-pico declarations.c:38-50 + 90-104 verbatim.

#[test]
fn forge_codec_zenoh_decl_ext_keyexpr_inner_cpp() {
    assert_standalone_forge(
        "codec_zenoh_decl_ext_keyexpr_inner",
        "codec_zenoh_decl_ext_keyexpr_inner.h",
    );
}

#[test]
fn forge_codec_zenoh_decl_ext_keyexpr_cpp() {
    assert_standalone_forge(
        "codec_zenoh_decl_ext_keyexpr",
        "codec_zenoh_decl_ext_keyexpr.h",
    );
}

#[test]
fn forge_codec_zenoh_undecl_subscriber_cpp() {
    assert_standalone_forge(
        "codec_zenoh_undecl_subscriber",
        "codec_zenoh_undecl_subscriber.h",
    );
}

#[test]
fn forge_codec_zenoh_undecl_queryable_cpp() {
    assert_standalone_forge(
        "codec_zenoh_undecl_queryable",
        "codec_zenoh_undecl_queryable.h",
    );
}

#[test]
fn forge_codec_zenoh_undecl_token_cpp() {
    assert_standalone_forge("codec_zenoh_undecl_token", "codec_zenoh_undecl_token.h");
}

// ── RFC §5.B Wire RFC Phase B Y2 — _encode_ext envelope family ──
// `codec_zenoh_source_info` is the second realistic consumer of B5-κ
// dotted-path length-field + B5-δ length-arith (after codec_zenoh_scout),
// modelling `_z_source_info_encode/decode` (message.c:196-242). Its
// `_encode_ext` envelope (message.c:243-254) and the bare timestamp's
// `_z_timestamp_encode_ext` (message.c:95-100) are the only two
// `_encode_ext` slots in upstream zenoh-pico — both compose the Y0b
// length-bound embed (`5a2c8afa`) over an existing body codec.

#[test]
fn forge_codec_zenoh_source_info_cpp() {
    assert_standalone_forge("codec_zenoh_source_info", "codec_zenoh_source_info.h");
}

#[test]
fn forge_codec_zenoh_source_info_ext_cpp() {
    assert_standalone_forge(
        "codec_zenoh_source_info_ext",
        "codec_zenoh_source_info_ext.h",
    );
}

#[test]
fn forge_codec_zenoh_timestamp_ext_cpp() {
    assert_standalone_forge("codec_zenoh_timestamp_ext", "codec_zenoh_timestamp_ext.h");
}

// ── RFC §5.B B3 TLV chain primitive (Cpp/Rust trunk) ────────
// `codec_tlv_chain_basic` declares a TLV chain bounded at max-depth=8
// with on-overflow="reject". MCU-class — Cpp/Kotlin/Go/Python all
// typed-reject with codegen/mcu-class-kind-on-non-mcu-language.
// The trunk's "Cpp/Rust trunk" naming is consistent with B1-β/δ +
// B2-α: "trunk" = first emit pair to land. For B3 the pair is
// **Rust + C11** (not Rust + Cpp) because the kind is MCU-class —
// Cpp would never be a valid emit target. Cpp gets a rejection test
// instead. The Rust-only `forge_codec_tlv_chain_basic_rust` lives
// under the assert_standalone_forge_rust shape; the C11 counterpart
// is `forge_c11_codec_tlv_chain_basic`.

#[test]
fn forge_codec_tlv_entry_cpp() {
    // Plain entry codec (no MCU-only sub-features) — ships on all 6
    // backends. The cpp golden anchors the entry shape; the Rust
    // counterpart on the next test exercises the same fixture under
    // the stream-correct length-ref shape introduced in B3-α.
    assert_standalone_forge("codec_tlv_entry", "codec_tlv_entry.h");
}

#[test]
fn forge_codec_tlv_entry_rust() {
    assert_standalone_forge_rust("codec_tlv_entry", "codec_tlv_entry.rs");
}

#[test]
fn forge_codec_tlv_chain_basic_rust() {
    assert_standalone_forge_rust("codec_tlv_chain_basic", "codec_tlv_chain_basic.rs");
}

// ── RFC §5.B B5-ε surface G — TLV chain entry body keyed by carrier bits ─
// `codec_zenoh_ext_envelope` carries a `<sce:tlv-chain>` of
// `codec_zenoh_ext_entry` entries; each entry's body shape is selected
// at runtime by the `enc` bit-range of its 1-byte header (mirrors
// zenoh-pico `_z_msg_ext_decode_iter` + `_z_msg_ext_unknown_body_decode`).
// Surface G freebie: no IR change. The `tlv_chain_body_alias` resolver
// already accepts any imported codec; the chain decode helper invokes
// the body codec's uniform `<codec>::decode(cursor)` /
// `<codec>_decode(cursor, *out)` signature; a variant codec emits
// exactly that surface, so a TLV chain of variant entries works
// transparently. The transitive `codec_max_bytes` enrichment fix
// landed alongside this lift (lib.rs `compute_codec_recursive_max_bytes`)
// is what closes the C11 silent-truncation gap when the variant entry's
// worst-case arm body exceeds the prefix size.

#[test]
fn forge_codec_zenoh_ext_unit_rust() {
    assert_standalone_forge_rust("codec_zenoh_ext_unit", "codec_zenoh_ext_unit.rs");
}

#[test]
fn forge_codec_zenoh_ext_zint_rust() {
    assert_standalone_forge_rust("codec_zenoh_ext_zint", "codec_zenoh_ext_zint.rs");
}

#[test]
fn forge_codec_zenoh_ext_zbuf_rust() {
    assert_standalone_forge_rust("codec_zenoh_ext_zbuf", "codec_zenoh_ext_zbuf.rs");
}

#[test]
fn forge_codec_zenoh_ext_entry_rust() {
    assert_standalone_forge_rust("codec_zenoh_ext_entry", "codec_zenoh_ext_entry.rs");
}

#[test]
fn forge_codec_zenoh_ext_envelope_rust() {
    assert_standalone_forge_rust("codec_zenoh_ext_envelope", "codec_zenoh_ext_envelope.rs");
}

// RFC §5.B B5-ε closures: cpp/kotlin/go/python now emit TLV chain via
// the host-language list shape (std::vector / MutableList / []T /
// List). The previous gate-rejection test on `codec_zenoh_ext_envelope`
// was retired alongside the analog on `codec_tlv_chain_basic` —
// positive byte-golden tests cover all 6 backends below.

#[test]
fn forge_codec_tlv_chain_basic_cpp() {
    assert_standalone_forge("codec_tlv_chain_basic", "codec_tlv_chain_basic.h");
}

#[test]
fn forge_codec_tlv_chain_basic_kotlin() {
    assert_standalone_forge_kotlin("codec_tlv_chain_basic", "CodecTlvChainBasic.kt");
}

#[test]
fn forge_codec_tlv_chain_basic_go() {
    assert_standalone_forge_go("codec_tlv_chain_basic", "codec_tlv_chain_basic.go");
}

#[test]
fn forge_codec_tlv_chain_basic_python() {
    assert_standalone_forge_python("codec_tlv_chain_basic", "codec_tlv_chain_basic.py");
}

// ── RFC §5.B Wire RFC Phase B Y3 atomic 2a — tlv-chain-with-present-if
// `codec_tlv_chain_present_if_basic` is the minimal demo for the
// `<sce:tlv-chain sce:present-if="P">` lift (mirrors codec_repeat_
// present_if_basic for B5-μ). Single-bit local carrier gates the
// chain; entries decode via the existing codec_tlv_entry. Locks the
// gated-list host wrap shape across all 6 backends (Option<Vec<T>> /
// std::optional<vector> / MutableList<T>? / bare []T (Go slice
// nilness) / Optional[List[T]] / C11 carrier-bit-as-truth +
// `_len = 0`).

#[test]
fn forge_codec_tlv_chain_present_if_basic_cpp() {
    assert_standalone_forge(
        "codec_tlv_chain_present_if_basic",
        "codec_tlv_chain_present_if_basic.h",
    );
}

#[test]
fn forge_codec_tlv_chain_present_if_basic_kotlin() {
    assert_standalone_forge_kotlin(
        "codec_tlv_chain_present_if_basic",
        "CodecTlvChainPresentIfBasic.kt",
    );
}

#[test]
fn forge_codec_tlv_chain_present_if_basic_rust() {
    assert_standalone_forge_rust(
        "codec_tlv_chain_present_if_basic",
        "codec_tlv_chain_present_if_basic.rs",
    );
}

#[test]
fn forge_codec_tlv_chain_present_if_basic_go() {
    assert_standalone_forge_go(
        "codec_tlv_chain_present_if_basic",
        "codec_tlv_chain_present_if_basic.go",
    );
}

#[test]
fn forge_codec_tlv_chain_present_if_basic_python() {
    assert_standalone_forge_python(
        "codec_tlv_chain_present_if_basic",
        "codec_tlv_chain_present_if_basic.py",
    );
}

#[test]
fn forge_c11_codec_tlv_chain_present_if_basic() {
    assert_standalone_forge_c(
        "codec_tlv_chain_present_if_basic",
        "codec_tlv_chain_present_if_basic.c.h",
    );
}

// ── RFC §5.B Wire RFC Phase B Y3 atomic 2a — zenoh-specific demo
// `codec_zenoh_query` mirrors zenoh-pico `_z_query_encode/decode`
// (message.c:394-505). Used as the request body variant arm for
// Y_QUERY (atomic 2b). Y3 atomic 2a uses it as the first realistic
// consumer of tlv-chain-with-present-if; the chain is the codec's
// last field so terminate-on stays at exhaust-or-depth (entry-flag
// termination exercises in Y3 atomic 2b consumers like
// codec_zenoh_request whose body variant follows the chain).

#[test]
fn forge_codec_zenoh_query_cpp() {
    assert_standalone_forge("codec_zenoh_query", "codec_zenoh_query.h");
}

#[test]
fn forge_codec_zenoh_query_kotlin() {
    assert_standalone_forge_kotlin("codec_zenoh_query", "CodecZenohQuery.kt");
}

#[test]
fn forge_codec_zenoh_query_rust() {
    assert_standalone_forge_rust("codec_zenoh_query", "codec_zenoh_query.rs");
}

#[test]
fn forge_codec_zenoh_query_go() {
    assert_standalone_forge_go("codec_zenoh_query", "codec_zenoh_query.go");
}

#[test]
fn forge_codec_zenoh_query_python() {
    assert_standalone_forge_python("codec_zenoh_query", "codec_zenoh_query.py");
}

#[test]
fn forge_c11_codec_zenoh_query() {
    assert_standalone_forge_c("codec_zenoh_query", "codec_zenoh_query.c.h");
}

// ── RFC §5.B Wire RFC Phase B Y3 atomic 2b — sub-codec atomic
// `codec_zenoh_reply` mirrors zenoh-pico `_z_reply_encode/decode`
// (message.c:507-543) at envelope-level wire fidelity. Response body
// variant arm for Z_REPLY (atomic 2b consumer codec_zenoh_response).
// First realistic consumer of Y3 atomic 1's entry-flag chain
// termination — chain is followed by the push_body embed so
// exhaust-or-depth would not detect the body boundary.

#[test]
fn forge_codec_zenoh_reply_cpp() {
    assert_standalone_forge("codec_zenoh_reply", "codec_zenoh_reply.h");
}

#[test]
fn forge_codec_zenoh_reply_kotlin() {
    assert_standalone_forge_kotlin("codec_zenoh_reply", "CodecZenohReply.kt");
}

#[test]
fn forge_codec_zenoh_reply_rust() {
    assert_standalone_forge_rust("codec_zenoh_reply", "codec_zenoh_reply.rs");
}

#[test]
fn forge_codec_zenoh_reply_go() {
    assert_standalone_forge_go("codec_zenoh_reply", "codec_zenoh_reply.go");
}

#[test]
fn forge_codec_zenoh_reply_python() {
    assert_standalone_forge_python("codec_zenoh_reply", "codec_zenoh_reply.py");
}

#[test]
fn forge_c11_codec_zenoh_reply() {
    assert_standalone_forge_c("codec_zenoh_reply", "codec_zenoh_reply.c.h");
}

// ── RFC §5.B Wire RFC Phase B Y3 atomic 2b — sub-codec atomic
// `codec_zenoh_err` mirrors zenoh-pico `_z_err_encode/decode`
// (message.c:545-595) at envelope-level wire fidelity. Response body
// variant arm for Z_ERR. Encoding wire is the Q1(b) simplification
// (inline VLE u32 encoding_id, schema-less subset) — full encoding
// wire (VLE-LSB-as-flag) deferred to a follow-up atomic.

#[test]
fn forge_codec_zenoh_err_cpp() {
    assert_standalone_forge("codec_zenoh_err", "codec_zenoh_err.h");
}

#[test]
fn forge_codec_zenoh_err_kotlin() {
    assert_standalone_forge_kotlin("codec_zenoh_err", "CodecZenohErr.kt");
}

#[test]
fn forge_codec_zenoh_err_rust() {
    assert_standalone_forge_rust("codec_zenoh_err", "codec_zenoh_err.rs");
}

#[test]
fn forge_codec_zenoh_err_go() {
    assert_standalone_forge_go("codec_zenoh_err", "codec_zenoh_err.go");
}

#[test]
fn forge_codec_zenoh_err_python() {
    assert_standalone_forge_python("codec_zenoh_err", "codec_zenoh_err.py");
}

#[test]
fn forge_c11_codec_zenoh_err() {
    assert_standalone_forge_c("codec_zenoh_err", "codec_zenoh_err.c.h");
}

// ── RFC §5.B Wire RFC Phase B Y3 atomic 2b — sub-codec atomic
// `codec_zenoh_interest_body` mirrors zenoh-pico `_z_interest_encode/
// decode` (interest.c:41-91) at envelope-level wire fidelity (not-is_final
// case only — is_final gate lives in the outer envelope codec_zenoh_interest).
// First fixture where local 1B flags carrier serves as the parent_flags
// for an embedded wireexpr (the upstream bit-reuse trick — N/M occupy
// the cleared CURRENT/FUTURE positions per _Z_INTEREST_FLAG_COPY_MASK).

#[test]
fn forge_codec_zenoh_interest_body_cpp() {
    assert_standalone_forge("codec_zenoh_interest_body", "codec_zenoh_interest_body.h");
}

#[test]
fn forge_codec_zenoh_interest_body_kotlin() {
    assert_standalone_forge_kotlin("codec_zenoh_interest_body", "CodecZenohInterestBody.kt");
}

#[test]
fn forge_codec_zenoh_interest_body_rust() {
    assert_standalone_forge_rust("codec_zenoh_interest_body", "codec_zenoh_interest_body.rs");
}

#[test]
fn forge_codec_zenoh_interest_body_go() {
    assert_standalone_forge_go("codec_zenoh_interest_body", "codec_zenoh_interest_body.go");
}

#[test]
fn forge_codec_zenoh_interest_body_python() {
    assert_standalone_forge_python("codec_zenoh_interest_body", "codec_zenoh_interest_body.py");
}

#[test]
fn forge_c11_codec_zenoh_interest_body() {
    assert_standalone_forge_c("codec_zenoh_interest_body", "codec_zenoh_interest_body.c.h");
}

// ── RFC §5.B Wire RFC Phase B Y3 atomic 2b — sub-codec atomic
// `codec_zenoh_declaration` mirrors zenoh-pico `_z_declaration_encode/
// decode` (declarations.c:137-180) — 9-arm dispatcher on first-byte
// MID 5-bit. Used by codec_zenoh_declare as the body embed after the
// network-level header + ext chain. Each arm wires up an existing Y1
// per-MID body codec (decl/undecl × keyexpr/subscriber/queryable/token
// + decl_final).

#[test]
fn forge_codec_zenoh_declaration_cpp() {
    assert_standalone_forge("codec_zenoh_declaration", "codec_zenoh_declaration.h");
}

#[test]
fn forge_codec_zenoh_declaration_kotlin() {
    assert_standalone_forge_kotlin("codec_zenoh_declaration", "CodecZenohDeclaration.kt");
}

#[test]
fn forge_codec_zenoh_declaration_rust() {
    assert_standalone_forge_rust("codec_zenoh_declaration", "codec_zenoh_declaration.rs");
}

#[test]
fn forge_codec_zenoh_declaration_go() {
    assert_standalone_forge_go("codec_zenoh_declaration", "codec_zenoh_declaration.go");
}

#[test]
fn forge_codec_zenoh_declaration_python() {
    assert_standalone_forge_python("codec_zenoh_declaration", "codec_zenoh_declaration.py");
}

#[test]
fn forge_c11_codec_zenoh_declaration() {
    assert_standalone_forge_c("codec_zenoh_declaration", "codec_zenoh_declaration.c.h");
}

// ── RFC §5.B Wire RFC Phase B Y3 atomic 2b-iii — sub-codec atomic
// `codec_zenoh_timestamp` mirrors zenoh-pico `_z_timestamp_encode/decode`
// (message.c:86-112) verbatim: VLE u64 time + length-prefixed zid bytes
// (max 16 per `ZENOH_ID_SIZE`). Foundational sub-codec for the T-gated
// timestamp embed inside codec_zenoh_msg_put / codec_zenoh_msg_del. No
// new SCE primitives — full upstream wire fidelity via existing
// length-ref bytes pattern.

#[test]
fn forge_codec_zenoh_timestamp_cpp() {
    assert_standalone_forge("codec_zenoh_timestamp", "codec_zenoh_timestamp.h");
}

#[test]
fn forge_codec_zenoh_timestamp_kotlin() {
    assert_standalone_forge_kotlin("codec_zenoh_timestamp", "CodecZenohTimestamp.kt");
}

#[test]
fn forge_codec_zenoh_timestamp_rust() {
    assert_standalone_forge_rust("codec_zenoh_timestamp", "codec_zenoh_timestamp.rs");
}

#[test]
fn forge_codec_zenoh_timestamp_go() {
    assert_standalone_forge_go("codec_zenoh_timestamp", "codec_zenoh_timestamp.go");
}

#[test]
fn forge_codec_zenoh_timestamp_python() {
    assert_standalone_forge_python("codec_zenoh_timestamp", "codec_zenoh_timestamp.py");
}

#[test]
fn forge_c11_codec_zenoh_timestamp() {
    assert_standalone_forge_c("codec_zenoh_timestamp", "codec_zenoh_timestamp.c.h");
}

// ── RFC §5.B Wire RFC Phase B Y3 atomic 2b-iii-α — full upstream
// wire fidelity for `_z_encoding_encode/decode` (codec.c:356-381).
// `codec_zenoh_encoding` is the second consumer of the VLE+flags
// composition primitive (first was codec_ext_encoding_info — the
// composition has shipped goldens since B4). Closes 3 of the 4
// deferrals listed in codec_ext_encoding_info's preamble: #2 VLE u64
// schema_len (not u8), #3 gated schema_len (upstream-binary parity),
// #4 string typing (not bytes). Deferral #1 (derived `id =
// packed_id >> 1` accessor) stays — host derives the real id via
// the same idiom every other Zenoh codec uses (raw carrier byte +
// named bit accessors; see codec_zenoh_request.header u8 with N/M/Z
// accessors). First reachable consumer = codec_zenoh_msg_put /
// codec_zenoh_err E-gated embed (replaces the prior Q1(b)
// inline VLE u32 encoding_id field).

#[test]
fn forge_codec_zenoh_encoding_cpp() {
    assert_standalone_forge("codec_zenoh_encoding", "codec_zenoh_encoding.h");
}

#[test]
fn forge_codec_zenoh_encoding_kotlin() {
    assert_standalone_forge_kotlin("codec_zenoh_encoding", "CodecZenohEncoding.kt");
}

#[test]
fn forge_codec_zenoh_encoding_rust() {
    assert_standalone_forge_rust("codec_zenoh_encoding", "codec_zenoh_encoding.rs");
}

#[test]
fn forge_codec_zenoh_encoding_go() {
    assert_standalone_forge_go("codec_zenoh_encoding", "codec_zenoh_encoding.go");
}

#[test]
fn forge_codec_zenoh_encoding_python() {
    assert_standalone_forge_python("codec_zenoh_encoding", "codec_zenoh_encoding.py");
}

#[test]
fn forge_c11_codec_zenoh_encoding() {
    assert_standalone_forge_c("codec_zenoh_encoding", "codec_zenoh_encoding.c.h");
}

// ── RFC §5.B Wire RFC Phase B Y3 atomic 2b-iii — sub-codec atomic
// `codec_zenoh_msg_put` mirrors zenoh-pico `_z_put_encode/decode`
// (message.c:369-379) which delegates to `_z_push_body_encode/decode`
// (lines 257-348). Request body variant arm for Z_PUT (MID 0x01).
// Encoding wire is the Q1(b) simplification (inline VLE u32
// encoding_id placeholder, schema-less subset only) mirroring
// codec_zenoh_err verbatim — full encoding wire (VLE-LSB-as-flag
// per `_z_encoding_encode` codec.c:356-367) deferred to atomic
// 2b-iii-α follow-up.

#[test]
fn forge_codec_zenoh_msg_put_cpp() {
    assert_standalone_forge("codec_zenoh_msg_put", "codec_zenoh_msg_put.h");
}

#[test]
fn forge_codec_zenoh_msg_put_kotlin() {
    assert_standalone_forge_kotlin("codec_zenoh_msg_put", "CodecZenohMsgPut.kt");
}

#[test]
fn forge_codec_zenoh_msg_put_rust() {
    assert_standalone_forge_rust("codec_zenoh_msg_put", "codec_zenoh_msg_put.rs");
}

#[test]
fn forge_codec_zenoh_msg_put_go() {
    assert_standalone_forge_go("codec_zenoh_msg_put", "codec_zenoh_msg_put.go");
}

#[test]
fn forge_codec_zenoh_msg_put_python() {
    assert_standalone_forge_python("codec_zenoh_msg_put", "codec_zenoh_msg_put.py");
}

#[test]
fn forge_c11_codec_zenoh_msg_put() {
    assert_standalone_forge_c("codec_zenoh_msg_put", "codec_zenoh_msg_put.c.h");
}

// ── RFC §5.B Wire RFC Phase B Y3 atomic 2b-iii — sub-codec atomic
// `codec_zenoh_msg_del` mirrors zenoh-pico `_z_del_encode/decode`
// (message.c:381-391) which delegates to `_z_push_body_encode/decode`
// (lines 257-348, !_is_put branch). Request body variant arm for
// Z_DEL (MID 0x02). DEL has no encoding (bit 6 truly upstream-
// reserved, declared as X@6 mirroring msg_reply X@6 precedent) and
// no payload (push_body_encode line 299-301 is `_is_put`-only).

#[test]
fn forge_codec_zenoh_msg_del_cpp() {
    assert_standalone_forge("codec_zenoh_msg_del", "codec_zenoh_msg_del.h");
}

#[test]
fn forge_codec_zenoh_msg_del_kotlin() {
    assert_standalone_forge_kotlin("codec_zenoh_msg_del", "CodecZenohMsgDel.kt");
}

#[test]
fn forge_codec_zenoh_msg_del_rust() {
    assert_standalone_forge_rust("codec_zenoh_msg_del", "codec_zenoh_msg_del.rs");
}

#[test]
fn forge_codec_zenoh_msg_del_go() {
    assert_standalone_forge_go("codec_zenoh_msg_del", "codec_zenoh_msg_del.go");
}

#[test]
fn forge_codec_zenoh_msg_del_python() {
    assert_standalone_forge_python("codec_zenoh_msg_del", "codec_zenoh_msg_del.py");
}

#[test]
fn forge_c11_codec_zenoh_msg_del() {
    assert_standalone_forge_c("codec_zenoh_msg_del", "codec_zenoh_msg_del.c.h");
}

// ── RFC §5.B Wire RFC Phase B Y3 atomic 2b-iii — first realistic
// peek-byte primitive consumer with msg_put / msg_del / query body
// arms (atomic 2b-i sub-codecs + atomic 2a sub-codec composing through
// atomic 2b-ii peek-byte primitive). Mirrors zenoh-pico
// `_z_request_encode/decode` (network.c:113-238). Header carries N@5
// + M@6 + Z@7 verbatim upstream (`_Z_FLAG_N_REQUEST_N=0x20` /
// `_Z_FLAG_N_REQUEST_M=0x40` per network.h:70-71) — distinct from the
// pre-fix codec_zenoh_response which had inverted N/M (corrected in
// the preceding commit).

#[test]
fn forge_codec_zenoh_request_cpp() {
    assert_standalone_forge("codec_zenoh_request", "codec_zenoh_request.h");
}

#[test]
fn forge_codec_zenoh_request_kotlin() {
    assert_standalone_forge_kotlin("codec_zenoh_request", "CodecZenohRequest.kt");
}

#[test]
fn forge_codec_zenoh_request_rust() {
    assert_standalone_forge_rust("codec_zenoh_request", "codec_zenoh_request.rs");
}

#[test]
fn forge_codec_zenoh_request_go() {
    assert_standalone_forge_go("codec_zenoh_request", "codec_zenoh_request.go");
}

#[test]
fn forge_codec_zenoh_request_python() {
    assert_standalone_forge_python("codec_zenoh_request", "codec_zenoh_request.py");
}

#[test]
fn forge_c11_codec_zenoh_request() {
    assert_standalone_forge_c("codec_zenoh_request", "codec_zenoh_request.c.h");
}

// ── RFC §5.B Wire RFC Phase B Y3 atomic 2b-iv — minimal-shape consumer
// `codec_zenoh_response_final` mirrors zenoh-pico
// `_z_response_final_encode/decode` (network.c:368-386). Header is
// MID 0x1a + Z@7 only (encoder writes Z=0 always; decoder accepts Z=1
// and skips ext chain). Wire = header + VLE u64 request_id + Z-gated
// chain.

#[test]
fn forge_codec_zenoh_response_final_cpp() {
    assert_standalone_forge("codec_zenoh_response_final", "codec_zenoh_response_final.h");
}

#[test]
fn forge_codec_zenoh_response_final_kotlin() {
    assert_standalone_forge_kotlin("codec_zenoh_response_final", "CodecZenohResponseFinal.kt");
}

#[test]
fn forge_codec_zenoh_response_final_rust() {
    assert_standalone_forge_rust(
        "codec_zenoh_response_final",
        "codec_zenoh_response_final.rs",
    );
}

#[test]
fn forge_codec_zenoh_response_final_go() {
    assert_standalone_forge_go(
        "codec_zenoh_response_final",
        "codec_zenoh_response_final.go",
    );
}

#[test]
fn forge_codec_zenoh_response_final_python() {
    assert_standalone_forge_python(
        "codec_zenoh_response_final",
        "codec_zenoh_response_final.py",
    );
}

#[test]
fn forge_c11_codec_zenoh_response_final() {
    assert_standalone_forge_c(
        "codec_zenoh_response_final",
        "codec_zenoh_response_final.c.h",
    );
}

// ── RFC §5.B B5 strict closure — peer of codec_transport_envelope at
// the network layer. `codec_zenoh_network_envelope` mirrors zenoh-pico
// `_z_network_message_decode` (network.c:630-668). 7-arm peek-byte
// dispatcher over MID bits 0..4 (0x19..0x1f); each arm body is a
// standalone codec that reads byte 0 as its own header. The default
// arm catches MIDs 0x00..0x18 unused by zenoh-pico's network namespace.
// No new SCE primitives required — peek-byte + variant + import all
// proven by `codec_zenoh_request`.

#[test]
fn forge_codec_zenoh_network_envelope_cpp() {
    assert_standalone_forge(
        "codec_zenoh_network_envelope",
        "codec_zenoh_network_envelope.h",
    );
}

#[test]
fn forge_codec_zenoh_network_envelope_kotlin() {
    assert_standalone_forge_kotlin(
        "codec_zenoh_network_envelope",
        "CodecZenohNetworkEnvelope.kt",
    );
}

#[test]
fn forge_codec_zenoh_network_envelope_rust() {
    assert_standalone_forge_rust(
        "codec_zenoh_network_envelope",
        "codec_zenoh_network_envelope.rs",
    );
}

#[test]
fn forge_codec_zenoh_network_envelope_go() {
    assert_standalone_forge_go(
        "codec_zenoh_network_envelope",
        "codec_zenoh_network_envelope.go",
    );
}

#[test]
fn forge_codec_zenoh_network_envelope_python() {
    assert_standalone_forge_python(
        "codec_zenoh_network_envelope",
        "codec_zenoh_network_envelope.py",
    );
}

#[test]
fn forge_c11_codec_zenoh_network_envelope() {
    assert_standalone_forge_c(
        "codec_zenoh_network_envelope",
        "codec_zenoh_network_envelope.c.h",
    );
}

// ── RFC §5.B Wire RFC Phase B Y3 atomic 2b-iv — multi-arm own-field
// variant on header ENC bits. `codec_zenoh_oam` mirrors zenoh-pico
// `_z_oam_encode/decode` (network.c:488-579). Header carries MID 0x1f
// + ENC[5..7) 2-bit subfield + Z@7. Body variant on `header.enc`
// reuses the existing codec_zenoh_ext_{unit,zint,zbuf} leaves —
// 0x00→ext_unit (empty), 0x01→ext_zint (VLE u64), 0x02→ext_zbuf
// (length-prefixed bytes), default→ext_unit (defensive recovery
// matching upstream's _Z_ERR_GENERIC reject path at network.c:506-508
// with most-benign zero-body shape).

#[test]
fn forge_codec_zenoh_oam_cpp() {
    assert_standalone_forge("codec_zenoh_oam", "codec_zenoh_oam.h");
}

#[test]
fn forge_codec_zenoh_oam_kotlin() {
    assert_standalone_forge_kotlin("codec_zenoh_oam", "CodecZenohOam.kt");
}

#[test]
fn forge_codec_zenoh_oam_rust() {
    assert_standalone_forge_rust("codec_zenoh_oam", "codec_zenoh_oam.rs");
}

#[test]
fn forge_codec_zenoh_oam_go() {
    assert_standalone_forge_go("codec_zenoh_oam", "codec_zenoh_oam.go");
}

#[test]
fn forge_codec_zenoh_oam_python() {
    assert_standalone_forge_python("codec_zenoh_oam", "codec_zenoh_oam.py");
}

#[test]
fn forge_c11_codec_zenoh_oam() {
    assert_standalone_forge_c("codec_zenoh_oam", "codec_zenoh_oam.c.h");
}

// ── RFC §5.B Wire RFC Phase B Y3 atomic 2b-iv — single-embed consumer
// of the 9-arm codec_zenoh_declaration dispatcher. `codec_zenoh_declare`
// mirrors zenoh-pico `_z_declare_encode/decode` (network.c:388-450).
// Header carries MID 0x1e + I@5 (gates VLE u32 interest_id) + Z@7
// (gates ext chain). Inner body is codec_zenoh_declaration embed
// (atomic 2b-i sub-codec, 9-arm dispatcher). interest_id type = u32
// per upstream `_z_zint32_decode` at network.c:441.

#[test]
fn forge_codec_zenoh_declare_cpp() {
    assert_standalone_forge("codec_zenoh_declare", "codec_zenoh_declare.h");
}

#[test]
fn forge_codec_zenoh_declare_kotlin() {
    assert_standalone_forge_kotlin("codec_zenoh_declare", "CodecZenohDeclare.kt");
}

#[test]
fn forge_codec_zenoh_declare_rust() {
    assert_standalone_forge_rust("codec_zenoh_declare", "codec_zenoh_declare.rs");
}

#[test]
fn forge_codec_zenoh_declare_go() {
    assert_standalone_forge_go("codec_zenoh_declare", "codec_zenoh_declare.go");
}

#[test]
fn forge_codec_zenoh_declare_python() {
    assert_standalone_forge_python("codec_zenoh_declare", "codec_zenoh_declare.py");
}

#[test]
fn forge_c11_codec_zenoh_declare() {
    assert_standalone_forge_c("codec_zenoh_declare", "codec_zenoh_declare.c.h");
}

// ── RFC §5.B Wire RFC Phase B Y3 atomic 2b-iv — first realistic
// disjunction primitive consumer + parent-owns-id refactor pin.
// `codec_zenoh_interest` mirrors zenoh-pico
// `_z_n_interest_encode/decode` + `_z_interest_encode/decode`
// (network.c:452-486 + interest.c:41-91). Header carries MID 0x19 +
// CURRENT@5 + FUTURE@6 + Z@7. Wire = header + ALWAYS-present VLE u64
// id + present-if-(`header.CURRENT || header.FUTURE`)-gated
// codec_zenoh_interest_body embed + Z-gated chain. The body embed is
// the first realistic consumer of the Y3 atomic 2b-ii disjunction
// primitive — matches upstream
// `_Z_INTEREST_NOT_FINAL_MASK = (CURRENT | FUTURE)` at interest.h:35.
// Bundled refactor: codec_zenoh_interest_body drops the leading `id`
// VLE field (parent-owns-id textbook composition) so its 6 byte
// goldens regenerate alongside this atomic.

#[test]
fn forge_codec_zenoh_interest_cpp() {
    assert_standalone_forge("codec_zenoh_interest", "codec_zenoh_interest.h");
}

#[test]
fn forge_codec_zenoh_interest_kotlin() {
    assert_standalone_forge_kotlin("codec_zenoh_interest", "CodecZenohInterest.kt");
}

#[test]
fn forge_codec_zenoh_interest_rust() {
    assert_standalone_forge_rust("codec_zenoh_interest", "codec_zenoh_interest.rs");
}

#[test]
fn forge_codec_zenoh_interest_go() {
    assert_standalone_forge_go("codec_zenoh_interest", "codec_zenoh_interest.go");
}

#[test]
fn forge_codec_zenoh_interest_python() {
    assert_standalone_forge_python("codec_zenoh_interest", "codec_zenoh_interest.py");
}

#[test]
fn forge_c11_codec_zenoh_interest() {
    assert_standalone_forge_c("codec_zenoh_interest", "codec_zenoh_interest.c.h");
}

// `forge_codec_tlv_entry_cpp` already exists above (it predates B5-ε;
// the entry codec ships on all 6 backends since it has no MCU-only
// sub-features itself). The new B5-ε closures add the missing kotlin /
// go / python entries below.

#[test]
fn forge_codec_tlv_entry_kotlin() {
    assert_standalone_forge_kotlin("codec_tlv_entry", "CodecTlvEntry.kt");
}

#[test]
fn forge_codec_tlv_entry_go() {
    assert_standalone_forge_go("codec_tlv_entry", "codec_tlv_entry.go");
}

#[test]
fn forge_codec_tlv_entry_python() {
    assert_standalone_forge_python("codec_tlv_entry", "codec_tlv_entry.py");
}

#[test]
fn forge_codec_zenoh_ext_unit_cpp() {
    assert_standalone_forge("codec_zenoh_ext_unit", "codec_zenoh_ext_unit.h");
}

#[test]
fn forge_codec_zenoh_ext_unit_kotlin() {
    assert_standalone_forge_kotlin("codec_zenoh_ext_unit", "CodecZenohExtUnit.kt");
}

#[test]
fn forge_codec_zenoh_ext_unit_go() {
    assert_standalone_forge_go("codec_zenoh_ext_unit", "codec_zenoh_ext_unit.go");
}

#[test]
fn forge_codec_zenoh_ext_unit_python() {
    assert_standalone_forge_python("codec_zenoh_ext_unit", "codec_zenoh_ext_unit.py");
}

#[test]
fn forge_codec_zenoh_ext_zint_cpp() {
    assert_standalone_forge("codec_zenoh_ext_zint", "codec_zenoh_ext_zint.h");
}

#[test]
fn forge_codec_zenoh_ext_zint_kotlin() {
    assert_standalone_forge_kotlin("codec_zenoh_ext_zint", "CodecZenohExtZint.kt");
}

#[test]
fn forge_codec_zenoh_ext_zint_go() {
    assert_standalone_forge_go("codec_zenoh_ext_zint", "codec_zenoh_ext_zint.go");
}

#[test]
fn forge_codec_zenoh_ext_zint_python() {
    assert_standalone_forge_python("codec_zenoh_ext_zint", "codec_zenoh_ext_zint.py");
}

#[test]
fn forge_codec_zenoh_ext_zbuf_cpp() {
    assert_standalone_forge("codec_zenoh_ext_zbuf", "codec_zenoh_ext_zbuf.h");
}

#[test]
fn forge_codec_zenoh_ext_zbuf_kotlin() {
    assert_standalone_forge_kotlin("codec_zenoh_ext_zbuf", "CodecZenohExtZbuf.kt");
}

#[test]
fn forge_codec_zenoh_ext_zbuf_go() {
    assert_standalone_forge_go("codec_zenoh_ext_zbuf", "codec_zenoh_ext_zbuf.go");
}

#[test]
fn forge_codec_zenoh_ext_zbuf_python() {
    assert_standalone_forge_python("codec_zenoh_ext_zbuf", "codec_zenoh_ext_zbuf.py");
}

#[test]
fn forge_codec_zenoh_ext_entry_cpp() {
    assert_standalone_forge("codec_zenoh_ext_entry", "codec_zenoh_ext_entry.h");
}

#[test]
fn forge_codec_zenoh_ext_entry_kotlin() {
    assert_standalone_forge_kotlin("codec_zenoh_ext_entry", "CodecZenohExtEntry.kt");
}

#[test]
fn forge_codec_zenoh_ext_entry_go() {
    assert_standalone_forge_go("codec_zenoh_ext_entry", "codec_zenoh_ext_entry.go");
}

#[test]
fn forge_codec_zenoh_ext_entry_python() {
    assert_standalone_forge_python("codec_zenoh_ext_entry", "codec_zenoh_ext_entry.py");
}

#[test]
fn forge_codec_zenoh_ext_envelope_cpp() {
    assert_standalone_forge("codec_zenoh_ext_envelope", "codec_zenoh_ext_envelope.h");
}

#[test]
fn forge_codec_zenoh_ext_envelope_kotlin() {
    assert_standalone_forge_kotlin("codec_zenoh_ext_envelope", "CodecZenohExtEnvelope.kt");
}

#[test]
fn forge_codec_zenoh_ext_envelope_go() {
    assert_standalone_forge_go("codec_zenoh_ext_envelope", "codec_zenoh_ext_envelope.go");
}

#[test]
fn forge_codec_zenoh_ext_envelope_python() {
    assert_standalone_forge_python("codec_zenoh_ext_envelope", "codec_zenoh_ext_envelope.py");
}

// ── RFC §5.B B5-ζ Surface H — string-vs-bytes typing ─────────
// `codec_zenoh_locator` is the first reachable consumer for
// `sce:type="string"` codec emit. Wire shape mirrors zenoh-pico
// `_z_string_encode`/`_z_string_decode` (codec.c:324-343) — VLE-
// encoded length prefix + length-ref UTF-8 string payload — but the
// host language emits `String` / `std::string` / `kotlin.String` /
// `string` / `str` instead of the byte container (`Vec<u8>` /
// `std::vector<uint8_t>` / `ByteArray` / `[]byte` / `bytes`). Decode
// validates UTF-8 and surfaces typed `CodecError::InvalidUtf8` (Rust /
// Go / Python) or the truncation sentinel (Cpp / Kotlin). Encode
// trusts the host string invariant — codec API stays infallible
// (encode-side validate would force `Result<Vec<u8>, EncodeError>` on
// every String-bearing codec). Parser restricts String fields to
// non-gated `BitSize::LengthRef` in v1; tail / fixed-bit / vle / gated
// shapes defer until a consumer surfaces. C11 closure ships
// separately (sce_forge_string_t storage shape parallel to
// sce_forge_bytes_t) — this commit covers the 5 backends with uniform
// `is_valid_utf8` / UTF-8 codec-helper paths.

#[test]
fn forge_codec_zenoh_locator_cpp() {
    assert_standalone_forge("codec_zenoh_locator", "codec_zenoh_locator.h");
}

#[test]
fn forge_rust_codec_zenoh_locator() {
    assert_standalone_forge_rust("codec_zenoh_locator", "codec_zenoh_locator.rs");
}

#[test]
fn forge_kotlin_codec_zenoh_locator() {
    assert_standalone_forge_kotlin("codec_zenoh_locator", "CodecZenohLocator.kt");
}

#[test]
fn forge_go_codec_zenoh_locator() {
    assert_standalone_forge_go("codec_zenoh_locator", "codec_zenoh_locator.go");
}

#[test]
fn forge_python_codec_zenoh_locator() {
    assert_standalone_forge_python("codec_zenoh_locator", "codec_zenoh_locator.py");
}

/// RFC §5.B B5-ζ Surface H parser validation: `sce:type="string"`
/// must pair with `sce:bit-size="length-ref"`. Tail / fixed-bit / vle
/// shapes reject with `validation/invalid-attribute` reporting the
/// legal combination so authors see the repair hint upfront. Mirror
/// fixture: same SCXML as codec_zenoh_locator but with bit-size
/// swapped to `tail` — the parser must surface InvalidAttribute
/// before any codegen runs.
#[test]
fn forge_codec_string_with_tail_bit_size_rejects() {
    use sce_build::forge::error::{ForgeError, ValidationError};

    let scxml = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       xmlns:sce="http://sce.dev/ext"
       sce:kind="codec" sce:default-endian="big" name="codec_string_tail_reject">
  <datamodel>
    <sce:field id="payload" sce:type="string" sce:byte="0" sce:bit-size="tail"/>
  </datamodel>
</scxml>"#;
    let result = sce_build::compile_forge_with_imports(
        scxml,
        sce_build::DocumentLabel::symmetric("codec_string_tail_reject"),
        sce_build::generator::Language::Rust,
        &resource_dir(),
        &sce_build::ForgeCompileOptions::default(),
    );
    let err = match result {
        Ok(_) => panic!(
            "sce:type=\"string\" + bit-size=\"tail\" must reject \
             with validation/invalid-attribute"
        ),
        Err(e) => e,
    };
    assert!(
        matches!(
            &err.error,
            ForgeError::Validation(ValidationError::InvalidAttribute { ref attr, .. })
                if attr == "sce:bit-size"
        ),
        "must surface ValidationError::InvalidAttribute on sce:bit-size; got: {:?}",
        err.error
    );
}

// RFC §5.B B5-ζ Surface H + Wire RFC Phase B Y0a: the v1 deferral
// (`sce:type="string"` + `sce:present-if` rejected as "no consumer
// yet") was lifted on 2026-05-03 when zenoh-pico's wireexpr surfaced
// as the realistic consumer (gated UTF-8 suffix per
// `_z_wireexpr_encode` message.c:115-125). The corresponding reject
// test (`forge_codec_string_with_present_if_rejects`) was removed at
// the same time — `codec_present_if_string` (local-scope primitive)
// + `codec_zenoh_wireexpr` (parent-scope realistic) byte-golden
// tests positively exercise the lifted surface across all 6
// backends.

// ── RFC §5.B B5-η Surface I — recursive variant body ─────────
// `codec_zenoh_push` is a variant codec whose 0x1d-arm body is
// itself a variant codec (`codec_zenoh_push_body`, dispatching
// PUT 0x01 / DEL 0x02). The per-language emit must therefore
// produce a sum-type whose arm payload is another sum-type
// (Rust `enum Push { Push(PushBodyEnum), ... }`, Cpp
// `std::variant<PushBodyVariant, ...>`, Kotlin `sealed class Push`
// holding `sealed class PushBody`, Go pointer-tagged struct of
// pointer-tagged struct, C11 tagged-union-of-tagged-union, Python
// kind-discriminated dataclass-of-dataclass). Surface I was
// preflight-validated as an IR-level freebie:
// `resolve_variant_arm_body_type` only checks `imp.kind ==
// "codec"`, with no variant-bearing-import gate, and a smoke
// fixture importing codec_variant_dispatch as an arm body produced
// byte-stable C11 compiling under `-std=c11 -Wall -Wextra
// -Wpedantic -Werror`. The atomic-6 commit lands the production
// composition on all 6 backends with no IR change.
//
// Leaves: codec_zenoh_put (1-byte payload) and codec_zenoh_del
// (empty body, mirrors B5-α empty-codec lift) — both stripped down
// from upstream `_z_msg_put_t`/`_z_msg_del_t` to keep the recursion
// surface focus and avoid pulling in B5-γ parent-flags. Inner
// variant: codec_zenoh_push_body (mirrors zenoh-pico
// `_z_push_body_decode` switch on `_Z_MID(header)` PUT/DEL/default).
// Outer variant: codec_zenoh_push (mirrors a stripped `_z_n_msg_t`
// dispatch on the network-MID 0x1d → push_body, default →
// push_body — degenerate two-tag-domain by design; the recursion
// is the test, not the variant cardinality at this layer).

#[test]
fn forge_codec_zenoh_put_cpp() {
    assert_standalone_forge("codec_zenoh_put", "codec_zenoh_put.h");
}

#[test]
fn forge_codec_zenoh_del_cpp() {
    assert_standalone_forge("codec_zenoh_del", "codec_zenoh_del.h");
}

#[test]
fn forge_codec_zenoh_push_body_cpp() {
    assert_standalone_forge("codec_zenoh_push_body", "codec_zenoh_push_body.h");
}

#[test]
fn forge_codec_zenoh_push_cpp() {
    assert_standalone_forge("codec_zenoh_push", "codec_zenoh_push.h");
}

#[test]
fn forge_rust_codec_zenoh_put() {
    assert_standalone_forge_rust("codec_zenoh_put", "codec_zenoh_put.rs");
}

#[test]
fn forge_rust_codec_zenoh_del() {
    assert_standalone_forge_rust("codec_zenoh_del", "codec_zenoh_del.rs");
}

#[test]
fn forge_rust_codec_zenoh_push_body() {
    assert_standalone_forge_rust("codec_zenoh_push_body", "codec_zenoh_push_body.rs");
}

#[test]
fn forge_rust_codec_zenoh_push() {
    assert_standalone_forge_rust("codec_zenoh_push", "codec_zenoh_push.rs");
}

#[test]
fn forge_kotlin_codec_zenoh_put() {
    assert_standalone_forge_kotlin("codec_zenoh_put", "CodecZenohPut.kt");
}

#[test]
fn forge_kotlin_codec_zenoh_del() {
    assert_standalone_forge_kotlin("codec_zenoh_del", "CodecZenohDel.kt");
}

#[test]
fn forge_kotlin_codec_zenoh_push_body() {
    assert_standalone_forge_kotlin("codec_zenoh_push_body", "CodecZenohPushBody.kt");
}

#[test]
fn forge_kotlin_codec_zenoh_push() {
    assert_standalone_forge_kotlin("codec_zenoh_push", "CodecZenohPush.kt");
}

#[test]
fn forge_go_codec_zenoh_put() {
    assert_standalone_forge_go("codec_zenoh_put", "codec_zenoh_put.go");
}

#[test]
fn forge_go_codec_zenoh_del() {
    assert_standalone_forge_go("codec_zenoh_del", "codec_zenoh_del.go");
}

#[test]
fn forge_go_codec_zenoh_push_body() {
    assert_standalone_forge_go("codec_zenoh_push_body", "codec_zenoh_push_body.go");
}

#[test]
fn forge_go_codec_zenoh_push() {
    assert_standalone_forge_go("codec_zenoh_push", "codec_zenoh_push.go");
}

#[test]
fn forge_python_codec_zenoh_put() {
    assert_standalone_forge_python("codec_zenoh_put", "codec_zenoh_put.py");
}

#[test]
fn forge_python_codec_zenoh_del() {
    assert_standalone_forge_python("codec_zenoh_del", "codec_zenoh_del.py");
}

#[test]
fn forge_python_codec_zenoh_push_body() {
    assert_standalone_forge_python("codec_zenoh_push_body", "codec_zenoh_push_body.py");
}

#[test]
fn forge_python_codec_zenoh_push() {
    assert_standalone_forge_python("codec_zenoh_push", "codec_zenoh_push.py");
}

// RFC §5.B B5-ε closures: TLV chain emit landed on cpp/kotlin/go/
// python via the host-language list shape (std::vector / MutableList /
// []T / List); the previous `forge_codec_tlv_chain_rejects_on_cpp`
// gate-rejection test was retired in the same change. Positive byte-
// golden tests for `codec_tlv_chain_basic` on the 4 newly supported
// backends live alongside the existing Rust + C11 tests below. The DMA
// alignment primitive (B3-β) stays MCU-only — its gate-rejection test
// remains at `forge_codec_dma_aligned_basic_rejects_on_*`.

/// RFC §5.B B3: `<sce:tlv-chain>` without `max-depth` rejects with the
/// dedicated `codec/tlv-chain-depth-unspecified` diagnostic so the
/// MCU-class contract is explicit (the runtime decoder needs a build-
/// time bound to size its working set; RFC line 488 + 533).
#[test]
fn forge_codec_tlv_chain_missing_depth_rejects() {
    use sce_build::forge::error::{ForgeError, ValidationError};

    let scxml = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       xmlns:sce="http://sce.dev/ext"
       sce:kind="codec" sce:default-endian="big" name="missing_depth">
  <sce:import src="codec_tlv_entry.scxml" kind="codec" as="codec_tlv_entry"/>
  <datamodel>
    <sce:field id="header" sce:type="uint8" sce:byte="0" sce:bit-size="8"/>
    <sce:tlv-chain id="extensions" type="codec_tlv_entry" sce:byte="1"/>
  </datamodel>
</scxml>"#;
    let result = sce_build::compile_forge_with_imports(
        scxml,
        sce_build::DocumentLabel::symmetric("missing_depth"),
        sce_build::generator::Language::Rust,
        &resource_dir(),
        &sce_build::ForgeCompileOptions::default(),
    );
    let err = match result {
        Ok(_) => panic!(
            "<sce:tlv-chain> without max-depth must reject with \
             codec/tlv-chain-depth-unspecified"
        ),
        Err(e) => e,
    };
    assert!(
        matches!(
            &err.error,
            ForgeError::Validation(ValidationError::CodecTlvChainDepthUnspecified {
                ref codec,
                ref field,
            }) if codec == "missing_depth" && field == "extensions"
        ),
        "must surface CodecTlvChainDepthUnspecified naming the codec + field; got: {:?}",
        err.error
    );
}

/// RFC §5.B B3: v1 rejects `on-overflow="diagnostic-event"` until the
/// §5.A diagnostic-event runtime infrastructure ships a reachable
/// consumer. `reject` and `truncate` are the v1 accept-set; anything
/// else surfaces the generic `validation/invalid-attribute`.
#[test]
fn forge_codec_tlv_chain_diagnostic_event_overflow_rejects() {
    use sce_build::forge::error::{ForgeError, ValidationError};

    let scxml = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       xmlns:sce="http://sce.dev/ext"
       sce:kind="codec" sce:default-endian="big" name="diag_event_overflow">
  <sce:import src="codec_tlv_entry.scxml" kind="codec" as="codec_tlv_entry"/>
  <datamodel>
    <sce:field id="header" sce:type="uint8" sce:byte="0" sce:bit-size="8"/>
    <sce:tlv-chain id="extensions" type="codec_tlv_entry" sce:byte="1"
                   max-depth="4" on-overflow="diagnostic-event"/>
  </datamodel>
</scxml>"#;
    let result = sce_build::compile_forge_with_imports(
        scxml,
        sce_build::DocumentLabel::symmetric("diag_event_overflow"),
        sce_build::generator::Language::Rust,
        &resource_dir(),
        &sce_build::ForgeCompileOptions::default(),
    );
    let err = match result {
        Ok(_) => panic!(
            "on-overflow=\"diagnostic-event\" must reject with \
             validation/invalid-attribute in v1"
        ),
        Err(e) => e,
    };
    assert!(
        matches!(
            &err.error,
            ForgeError::Validation(ValidationError::InvalidAttribute { ref attr, ref value, .. })
                if attr == "on-overflow" && value == "diagnostic-event"
        ),
        "must surface ValidationError::InvalidAttribute naming on-overflow + diagnostic-event; got: {:?}",
        err.error
    );
}

// ── RFC §5.B B3 DMA alignment primitive (Cpp/Rust trunk) ────
// `codec_dma_aligned_basic` declares a uint8 msg_id + uint8 reserved
// at bytes 0-1, then a tail-bytes aligned_payload at byte 32 with
// sce:dma-burst-align="32" — codegen emits 30 bytes of zero padding
// in encode, plus a compile-time assertion that the literal byte
// offset is 32-aligned (drift detection). MCU-class — same gate as
// TLV chain rejects cpp/kotlin/go/python.

#[test]
fn forge_codec_dma_aligned_basic_rust() {
    assert_standalone_forge_rust("codec_dma_aligned_basic", "codec_dma_aligned_basic.rs");
}

/// RFC §5.B B3 MCU gate: a codec with `sce:dma-burst-align` on any
/// field rejects when targeting cpp via the existing codec-content
/// MCU mechanism (mirrors TLV chain). The diagnostic kind name folds
/// the codec identifier + the MCU-only-features marker.
#[test]
fn forge_codec_dma_aligned_rejects_on_cpp() {
    use sce_build::forge::error::{ForgeError, GenerateError};

    let scxml_path = resource_dir().join("codec_dma_aligned_basic.scxml");
    let content =
        std::fs::read_to_string(&scxml_path).expect("Cannot read codec_dma_aligned_basic.scxml");
    let result = sce_build::compile_forge_with_imports(
        &content,
        sce_build::DocumentLabel::symmetric("codec_dma_aligned_basic"),
        sce_build::generator::Language::Cpp,
        &resource_dir(),
        &sce_build::ForgeCompileOptions::default(),
    );
    let err = match result {
        Ok(_) => panic!(
            "codec containing sce:dma-burst-align must reject on Cpp via \
             codegen/mcu-class-kind-on-non-mcu-language"
        ),
        Err(e) => e,
    };
    assert!(
        matches!(
            &err.error,
            ForgeError::Generate(GenerateError::CodegenMcuClassKindOnNonMcuLanguage {
                ref language,
                ..
            }) if language == "cpp"
        ),
        "must surface CodegenMcuClassKindOnNonMcuLanguage targeting cpp; got: {:?}",
        err.error
    );
}

/// RFC §5.B B3: misaligned `sce:byte` rejects with
/// `codec/dma-alignment-unsatisfiable` (e.g. byte=33 against
/// burst-align=32). The reason string names both numbers and the
/// closest aligned offsets so the author sees the repair direction.
#[test]
fn forge_codec_dma_misaligned_byte_rejects() {
    use sce_build::forge::error::{ForgeError, ValidationError};

    let scxml = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       xmlns:sce="http://sce.dev/ext"
       sce:kind="codec" sce:default-endian="big" name="misaligned">
  <datamodel>
    <sce:field id="msg_id"   sce:type="uint8" sce:byte="0" sce:bit-size="8"/>
    <sce:field id="aligned_payload" sce:type="bytes" sce:byte="33"
               sce:bit-size="tail" sce:max-size="32"
               sce:dma-burst-align="32"/>
  </datamodel>
</scxml>"#;
    let result = sce_build::compile_forge_with_imports(
        scxml,
        sce_build::DocumentLabel::symmetric("misaligned"),
        sce_build::generator::Language::Rust,
        &resource_dir(),
        &sce_build::ForgeCompileOptions::default(),
    );
    let err = match result {
        Ok(_) => panic!(
            "byte=33 with burst-align=32 must reject with \
             codec/dma-alignment-unsatisfiable"
        ),
        Err(e) => e,
    };
    assert!(
        matches!(
            &err.error,
            ForgeError::Validation(ValidationError::CodecDmaAlignmentUnsatisfiable {
                ref codec, ref field, burst_align: 32, ..
            }) if codec == "misaligned" && field == "aligned_payload"
        ),
        "must surface CodecDmaAlignmentUnsatisfiable naming codec + field + burst_align; got: {:?}",
        err.error
    );
}

/// RFC §5.B B3 line 558-583 "fixed-offset positions only — no VLE-
/// following alignment": a `sce:dma-burst-align` field after any
/// variable-length predecessor (vle here) rejects with the same
/// diagnostic. The reason names the offending predecessor + its
/// bit-size kind so the repair is unambiguous.
#[test]
fn forge_codec_dma_after_vle_rejects() {
    use sce_build::forge::error::{ForgeError, ValidationError};

    let scxml = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       xmlns:sce="http://sce.dev/ext"
       sce:kind="codec" sce:default-endian="big" name="vle_then_dma">
  <datamodel>
    <sce:field id="value" sce:type="uint64" sce:byte="0" sce:bit-size="vle"/>
    <sce:field id="aligned_payload" sce:type="bytes" sce:byte="32"
               sce:bit-size="tail" sce:max-size="32"
               sce:dma-burst-align="32"/>
  </datamodel>
</scxml>"#;
    let result = sce_build::compile_forge_with_imports(
        scxml,
        sce_build::DocumentLabel::symmetric("vle_then_dma"),
        sce_build::generator::Language::Rust,
        &resource_dir(),
        &sce_build::ForgeCompileOptions::default(),
    );
    let err = match result {
        Ok(_) => panic!(
            "sce:dma-burst-align after a vle field must reject with \
             codec/dma-alignment-unsatisfiable"
        ),
        Err(e) => e,
    };
    assert!(
        matches!(
            &err.error,
            ForgeError::Validation(ValidationError::CodecDmaAlignmentUnsatisfiable {
                ref codec, ref field, burst_align: 32, ref reason,
            }) if codec == "vle_then_dma"
                && field == "aligned_payload"
                && reason.contains("vle")
                && reason.contains("'value'")
        ),
        "reason must name the offending predecessor + its bit-size; got: {:?}",
        err.error
    );
}

/// RFC §5.B B3: non-power-of-2 burst-align value (3, 5, ...) rejects
/// with the generic `validation/invalid-attribute` slot — the repair
/// is text-level (pick a power of 2). v0: 0 also rejects (alignment
/// to a 0-byte boundary is meaningless).
#[test]
fn forge_codec_dma_non_power_of_two_rejects() {
    use sce_build::forge::error::{ForgeError, ValidationError};

    let scxml = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       xmlns:sce="http://sce.dev/ext"
       sce:kind="codec" sce:default-endian="big" name="bad_align">
  <datamodel>
    <sce:field id="msg_id" sce:type="uint8" sce:byte="0" sce:bit-size="8"/>
    <sce:field id="aligned_payload" sce:type="bytes" sce:byte="3"
               sce:bit-size="tail" sce:max-size="32"
               sce:dma-burst-align="3"/>
  </datamodel>
</scxml>"#;
    let result = sce_build::compile_forge_with_imports(
        scxml,
        sce_build::DocumentLabel::symmetric("bad_align"),
        sce_build::generator::Language::Rust,
        &resource_dir(),
        &sce_build::ForgeCompileOptions::default(),
    );
    let err = match result {
        Ok(_) => panic!(
            "sce:dma-burst-align=\"3\" must reject with \
             validation/invalid-attribute"
        ),
        Err(e) => e,
    };
    assert!(
        matches!(
            &err.error,
            ForgeError::Validation(ValidationError::InvalidAttribute { ref attr, ref value, .. })
                if attr == "sce:dma-burst-align" && value == "3"
        ),
        "must surface ValidationError::InvalidAttribute naming dma-burst-align + 3; got: {:?}",
        err.error
    );
}

/// RFC §5.B B2: `codec/repeat-count-refs-later-field` build-time check —
/// a `<sce:repeat sce:count="num_frags">` whose `num_frags` field is
/// declared *after* the repeat must reject so the streaming decoder
/// never reaches the loop without a value to count against. The
/// rejection names the codec, the offending repeat field, and the
/// unresolved count target.
#[test]
fn forge_codec_repeat_forward_count_rejects() {
    use sce_build::forge::error::{ForgeError, ValidationError};

    let scxml = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       xmlns:sce="http://sce.dev/ext"
       sce:kind="codec" sce:default-endian="big" name="forward_count">
  <sce:import src="codec_repeat_elem.scxml" kind="codec" as="codec_repeat_elem"/>
  <datamodel>
    <sce:repeat id="frags" type="codec_repeat_elem" sce:byte="0"
                count="num_frags" max-count="32"/>
    <sce:field id="num_frags" sce:type="uint8" sce:byte="64" sce:bit-size="8"/>
  </datamodel>
</scxml>"#;
    let result = sce_build::compile_forge_with_imports(
        scxml,
        sce_build::DocumentLabel::symmetric("forward_count"),
        sce_build::generator::Language::Rust,
        &resource_dir(),
        &sce_build::ForgeCompileOptions::default(),
    );
    let err = match result {
        Ok(_) => panic!(
            "forward-reference of <sce:repeat sce:count=\"...\"> must reject \
             with codec/repeat-count-refs-later-field"
        ),
        Err(e) => e,
    };
    let inner = err.error;
    assert!(
        matches!(
            inner,
            ForgeError::Validation(ValidationError::CodecRepeatCountRefsLaterField {
                ref field,
                ref refers_to,
                ..
            }) if field == "frags" && refers_to == "num_frags"
        ),
        "must surface as ValidationError::CodecRepeatCountRefsLaterField with the offending repeat and count target; got: {inner:?}"
    );
}

/// RFC §5.B B5-μ co-gating contract — repeat-with-present-if (Wire
/// RFC Phase B X1). When `<sce:repeat sce:count="X"
/// sce:present-if="P"/>` is gated, the count source field `X` MUST
/// also carry the IDENTICAL predicate `P`. Wire semantics: when the
/// gate fires off, the count byte(s) are absent — the streaming
/// decoder cannot read `X` to drive the repeat loop. Validator emits
/// `validation/invalid-attribute` with both predicate strings + the
/// repair hint naming both fields.
///
/// Case 1: count field has NO present-if (predicate mismatch via
/// missing predicate on count side).
#[test]
fn forge_codec_repeat_present_if_count_missing_predicate_rejects() {
    use sce_build::forge::error::{ForgeError, ValidationError};
    let scxml = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       xmlns:sce="http://sce.dev/ext"
       sce:kind="codec" sce:default-endian="big" name="cogating_missing">
  <sce:import src="codec_repeat_elem.scxml" kind="codec" as="codec_repeat_elem"/>
  <datamodel>
    <sce:flags id="carrier" sce:type="uint8" sce:byte="0" sce:bit-size="8">
      <sce:flag name="has_list" bit="0"/>
    </sce:flags>
    <sce:field id="num_elems" sce:type="uint8" sce:byte="1" sce:bit-size="8"/>
    <sce:repeat id="elems" type="codec_repeat_elem" sce:byte="2"
                count="num_elems" max-count="32"
                sce:present-if="carrier.has_list"/>
  </datamodel>
</scxml>"#;
    let err = sce_build::compile_forge_with_imports(
        scxml,
        sce_build::DocumentLabel::symmetric("cogating_missing"),
        sce_build::generator::Language::Rust,
        &resource_dir(),
        &sce_build::ForgeCompileOptions::default(),
    )
    .err()
    .expect("must reject co-gating contract violation");
    assert!(
        matches!(
            err.error,
            ForgeError::Validation(ValidationError::InvalidAttribute { ref attr, .. })
                if attr == "sce:present-if"
        ),
        "must surface as InvalidAttribute on sce:present-if; got: {:?}",
        err.error
    );
}

/// Case 2: count field has a DIFFERENT present-if predicate
/// (predicate identity mismatch). Same gate vs. different gate is the
/// most subtle authoring mistake — the wire would still parse but the
/// presence semantics would diverge between count and repeat.
#[test]
fn forge_codec_repeat_present_if_count_predicate_mismatch_rejects() {
    use sce_build::forge::error::{ForgeError, ValidationError};
    let scxml = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       xmlns:sce="http://sce.dev/ext"
       sce:kind="codec" sce:default-endian="big" name="cogating_mismatch">
  <sce:import src="codec_repeat_elem.scxml" kind="codec" as="codec_repeat_elem"/>
  <datamodel>
    <sce:flags id="carrier" sce:type="uint8" sce:byte="0" sce:bit-size="8">
      <sce:flag name="has_list"  bit="0"/>
      <sce:flag name="has_count" bit="1"/>
    </sce:flags>
    <sce:field id="num_elems" sce:type="uint8" sce:byte="1" sce:bit-size="8"
               sce:present-if="carrier.has_count"/>
    <sce:repeat id="elems" type="codec_repeat_elem" sce:byte="2"
                count="num_elems" max-count="32"
                sce:present-if="carrier.has_list"/>
  </datamodel>
</scxml>"#;
    let err = sce_build::compile_forge_with_imports(
        scxml,
        sce_build::DocumentLabel::symmetric("cogating_mismatch"),
        sce_build::generator::Language::Rust,
        &resource_dir(),
        &sce_build::ForgeCompileOptions::default(),
    )
    .err()
    .expect("must reject co-gating contract violation");
    assert!(
        matches!(
            err.error,
            ForgeError::Validation(ValidationError::InvalidAttribute { ref attr, .. })
                if attr == "sce:present-if"
        ),
        "must surface as InvalidAttribute on sce:present-if; got: {:?}",
        err.error
    );
}

// ── RFC §5.B B5-κ Surface L parser-validation reject tests ────
//
// Mirrors B1-δ present-if's reject coverage (forward-reference,
// non-flags-bearing carrier, missing flag, single-bit flag rejected
// for length-source semantics). Every failure folds into the
// generic `validation/invalid-attribute` (no new diagnostic).

/// Forward-reference: carrier declared AFTER the length-ref payload.
/// Streaming decoder cannot read the carrier byte before reaching the
/// payload, so codegen must reject at parse time.
#[test]
fn forge_codec_length_field_dotted_forward_ref_rejects() {
    use sce_build::forge::error::{ForgeError, ValidationError};
    let scxml = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       xmlns:sce="http://sce.dev/ext"
       sce:kind="codec" sce:default-endian="big" name="forward_ref">
  <datamodel>
    <sce:field id="payload" sce:type="bytes" sce:byte="0" sce:bit-size="length-ref"
               sce:length-field="hdr.payload_len" sce:max-size="15"/>
    <sce:flags id="hdr" sce:type="uint8" sce:byte="64" sce:bit-size="8">
      <sce:flag name="payload_len" bit="4" width="4"/>
    </sce:flags>
  </datamodel>
</scxml>"#;
    let result = sce_build::compile_forge_with_imports(
        scxml,
        sce_build::DocumentLabel::symmetric("forward_ref"),
        sce_build::generator::Language::Rust,
        &resource_dir(),
        &sce_build::ForgeCompileOptions::default(),
    );
    let err = match result {
        Ok(_) => panic!(
            "forward-reference of dotted-path length-field must reject with validation/invalid-attribute"
        ),
        Err(e) => e,
    };
    assert!(
        matches!(
            &err.error,
            ForgeError::Validation(ValidationError::InvalidAttribute { attr, .. })
                if attr == "sce:length-field"
        ),
        "must surface as InvalidAttribute on sce:length-field; got: {:?}",
        err.error
    );
}

/// Single-bit flag rejected: width=1 only ever carries 0 or 1, which is
/// the present-if grammar's domain (bit-test), not the length-field
/// grammar's (value-extract). Forces authors to declare a multi-bit
/// flag when they want a length source.
#[test]
fn forge_codec_length_field_dotted_single_bit_rejects() {
    use sce_build::forge::error::{ForgeError, ValidationError};
    let scxml = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       xmlns:sce="http://sce.dev/ext"
       sce:kind="codec" sce:default-endian="big" name="single_bit_len">
  <datamodel>
    <sce:flags id="hdr" sce:type="uint8" sce:byte="0" sce:bit-size="8">
      <sce:flag name="one_bit" bit="0"/>
    </sce:flags>
    <sce:field id="payload" sce:type="bytes" sce:byte="1" sce:bit-size="length-ref"
               sce:length-field="hdr.one_bit" sce:max-size="15"/>
  </datamodel>
</scxml>"#;
    let result = sce_build::compile_forge_with_imports(
        scxml,
        sce_build::DocumentLabel::symmetric("single_bit_len"),
        sce_build::generator::Language::Rust,
        &resource_dir(),
        &sce_build::ForgeCompileOptions::default(),
    );
    let err = match result {
        Ok(_) => panic!(
            "single-bit flag (width=1) must reject as length source; that's the present-if grammar's domain"
        ),
        Err(e) => e,
    };
    assert!(
        matches!(
            &err.error,
            ForgeError::Validation(ValidationError::InvalidAttribute { attr, expected, .. })
                if attr == "sce:length-field" && expected.contains("multi-bit")
        ),
        "must mention multi-bit requirement; got: {:?}",
        err.error
    );
}

/// Non-flags-bearing carrier: dotted-path LHS must be a `<sce:flags>`
/// container, not a plain `<sce:field>`.
#[test]
fn forge_codec_length_field_dotted_non_flags_carrier_rejects() {
    use sce_build::forge::error::{ForgeError, ValidationError};
    let scxml = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       xmlns:sce="http://sce.dev/ext"
       sce:kind="codec" sce:default-endian="big" name="plain_carrier">
  <datamodel>
    <sce:field id="hdr" sce:type="uint8" sce:byte="0" sce:bit-size="8"/>
    <sce:field id="payload" sce:type="bytes" sce:byte="1" sce:bit-size="length-ref"
               sce:length-field="hdr.bogus" sce:max-size="15"/>
  </datamodel>
</scxml>"#;
    let result = sce_build::compile_forge_with_imports(
        scxml,
        sce_build::DocumentLabel::symmetric("plain_carrier"),
        sce_build::generator::Language::Rust,
        &resource_dir(),
        &sce_build::ForgeCompileOptions::default(),
    );
    let err = match result {
        Ok(_) => panic!("dotted-path against a non-flags-bearing carrier must reject"),
        Err(e) => e,
    };
    assert!(
        matches!(
            &err.error,
            ForgeError::Validation(ValidationError::InvalidAttribute { attr, expected, .. })
                if attr == "sce:length-field" && expected.contains("flags-bearing")
        ),
        "must mention flags-bearing requirement; got: {:?}",
        err.error
    );
}

// ── RFC §5.B B4 applied codec shapes (Cpp) ───────────────────
// Three Zenoh wire-extension shapes built from existing B1/B2
// primitives — no new IR/parser/codegen surface. See
// `tests/forge/resources/codec_ext_*.scxml` for upstream zenoh-pico
// references and the per-fixture deferrals to B5.

#[test]
fn forge_codec_ext_timestamp_cpp() {
    assert_standalone_forge("codec_ext_timestamp", "codec_ext_timestamp.h");
}

#[test]
fn forge_codec_ext_attachment_cpp() {
    assert_standalone_forge("codec_ext_attachment", "codec_ext_attachment.h");
}

#[test]
fn forge_codec_ext_encoding_info_cpp() {
    assert_standalone_forge("codec_ext_encoding_info", "codec_ext_encoding_info.h");
}

// ── RFC §5.B test-vector primitive (B2-test-vector prep) ─────
// `<sce:test-vector hex value/>` parses into AlgorithmModel.test_vectors
// for sce:kind="algorithm" only. Multi-field codec test vectors defer
// to B5 alongside the Zenoh msg-set authoring; v1 rejects test-vector
// declarations under any other kind so the deferral is explicit and
// authors don't silently lose oracle coverage. The four tests below
// pin (1) the positive parse path, (2) the kind-gate rejection, and
// (3+4) the two attribute-text-level malformations that reuse the
// generic validation/invalid-attribute slot.

/// Positive: an algorithm with a single `<sce:test-vector>` parses
/// cleanly into the IR. Verifies the canonical RFC §5.B example
/// (CRC16-CCITT-FALSE: "123456789" → 0x29B1) round-trips through the
/// hex decoder and the integer literal parser, and that the source
/// line of the element is preserved for future per-backend test
/// function naming.
#[test]
fn forge_algorithm_test_vector_parses() {
    use sce_build::forge::model::{ForgeDocument, TestVectorValue};
    use sce_build::forge::parser::parse_forge;
    use sce_build::DocumentLabel;

    let scxml = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       xmlns:sce="http://sce.dev/ext"
       sce:kind="algorithm" name="crc_oracle">
  <sce:signature>
    <sce:param name="data" type="bytes"/>
    <sce:return type="uint16"/>
  </sce:signature>
  <sce:body>
    <sce:return expr="0xFFFF"/>
  </sce:body>
  <sce:test-vector hex="313233343536373839" value="0x29B1"/>
</scxml>"#;

    let doc = parse_forge(scxml, DocumentLabel::symmetric("crc_oracle"))
        .expect("algorithm with <sce:test-vector> must parse cleanly")
        .expect("fixture is sce:kind=\"algorithm\"");
    let alg = match doc {
        ForgeDocument::Algorithm(m) => m,
        other => panic!("expected Algorithm doc, got {:?}", other.kind()),
    };

    assert_eq!(
        alg.test_vectors.len(),
        1,
        "fixture declares one <sce:test-vector>"
    );
    let tv = &alg.test_vectors[0];
    assert_eq!(
        tv.hex,
        vec![0x31, 0x32, 0x33, 0x34, 0x35, 0x36, 0x37, 0x38, 0x39],
        "hex='313233343536373839' decodes to ASCII '123456789' bytes"
    );
    assert_eq!(
        tv.value,
        TestVectorValue::Uint(0x29B1),
        "value='0x29B1' parses as Uint(0x29B1) for uint16 return type"
    );
    assert!(
        tv.source_line >= 1,
        "source_line tracks the SCXML row of the <sce:test-vector> element"
    );
}

/// RFC §5.B B5-θ positive: a codec with multiple `<sce:test-vector>`
/// rows parses cleanly into the IR. Pins the field-name resolution
/// (must match a declared `<sce:field>`), the per-type literal
/// dispatch (uint integer + bytes hex), and the source_line tracking.
#[test]
fn forge_codec_test_vector_parses() {
    use sce_build::forge::model::{DecodedFieldValue, DecodedValue, ForgeDocument};
    use sce_build::forge::parser::parse_forge;
    use sce_build::DocumentLabel;

    let scxml = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       xmlns:sce="http://sce.dev/ext"
       sce:kind="codec" sce:default-endian="big" name="frame_oracle">
  <datamodel>
    <sce:field id="sn" sce:type="uint64" sce:byte="0" sce:bit-size="vle"/>
    <sce:field id="payload" sce:type="bytes" sce:byte="1" sce:bit-size="tail" sce:max-size="32"/>
  </datamodel>
  <sce:test-vector hex="01cafe">
    <sce:decoded field="sn" value="1"/>
    <sce:decoded field="payload" hex="cafe"/>
  </sce:test-vector>
</scxml>"#;

    let doc = parse_forge(scxml, DocumentLabel::symmetric("frame_oracle"))
        .expect("codec with <sce:test-vector> must parse cleanly")
        .expect("fixture is sce:kind=\"codec\"");
    let codec = match doc {
        ForgeDocument::Codec(m) => m,
        other => panic!("expected Codec doc, got {:?}", other.kind()),
    };

    assert_eq!(
        codec.test_vectors.len(),
        1,
        "fixture declares one <sce:test-vector>"
    );
    let tv = &codec.test_vectors[0];
    assert_eq!(tv.hex, vec![0x01, 0xCA, 0xFE]);
    let DecodedValue::Plain { fields } = &tv.decoded;
    assert_eq!(fields.len(), 2);
    assert_eq!(fields[0].name, "sn");
    assert!(matches!(fields[0].value, DecodedFieldValue::Uint(1)));
    assert_eq!(fields[1].name, "payload");
    assert!(matches!(&fields[1].value, DecodedFieldValue::Bytes(bs) if bs == &vec![0xCA, 0xFE]));
}

/// RFC §5.B B5-θ negative: `<sce:decoded field="...">` referencing a
/// field id that does not exist in the codec rejects with the
/// generic `validation/invalid-attribute` slot — the repair stays
/// attribute-text-level so no new diagnostic warranted.
#[test]
fn forge_codec_test_vector_unknown_field_rejects() {
    use sce_build::forge::error::{ForgeError, ValidationError};

    let scxml = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       xmlns:sce="http://sce.dev/ext"
       sce:kind="codec" sce:default-endian="big" name="frame_oracle">
  <datamodel>
    <sce:field id="reason" sce:type="uint8" sce:byte="0" sce:bit-size="8"/>
  </datamodel>
  <sce:test-vector hex="01">
    <sce:decoded field="bogus" value="1"/>
  </sce:test-vector>
</scxml>"#;

    let result = sce_build::compile_forge_with_imports(
        scxml,
        sce_build::DocumentLabel::symmetric("frame_oracle"),
        sce_build::generator::Language::Rust,
        &resource_dir(),
        &sce_build::ForgeCompileOptions::default(),
    );
    let err = match result {
        Ok(_) => panic!(
            "<sce:decoded field=\"bogus\"> must reject — bogus is not a declared codec field"
        ),
        Err(e) => e,
    };
    assert!(
        matches!(
            err.error,
            ForgeError::Validation(ValidationError::InvalidAttribute { ref attr, .. })
                if attr == "field"
        ),
        "must surface as ValidationError::InvalidAttribute targeting `field`; got: {:?}",
        err.error
    );
}

/// Negative: `<sce:test-vector>` declared under `sce:kind="codec"` (or
/// any non-algorithm kind) rejects with the typed
/// `algorithm/test-vector-unsupported-kind` diagnostic so the v1
/// algorithm-only restriction is explicit at parse time. Multi-field
/// non-supported kinds (filter / transform / lookup / etc.) cannot
/// host a hex-bytes round-trip oracle in v1 — the parser-side gate
/// keeps the rejection anchored at the offending element rather
/// than letting it leak into codegen. B5-θ landed `<sce:test-vector>`
/// support for codec; this test rotates from "codec rejects" to
/// "filter still rejects" as the canary for the kind-allowlist gate.
#[test]
fn forge_test_vector_on_filter_rejects() {
    use sce_build::forge::error::{ForgeError, ValidationError};
    use sce_build::forge::model::ForgeKind;

    let scxml = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       xmlns:sce="http://sce.dev/ext"
       version="1.0" sce:kind="filter" name="session_filter">
  <datamodel>
    <data id="rawSignal" sce:type="float64" sce:direction="in"/>
    <data id="smoothed" sce:type="float64" sce:direction="out"
          sce:filter="low-pass" sce:alpha="0.1"/>
  </datamodel>
  <sce:test-vector hex="01" value="0x01"/>
</scxml>"#;
    let result = sce_build::compile_forge_with_imports(
        scxml,
        sce_build::DocumentLabel::symmetric("session_filter"),
        sce_build::generator::Language::Rust,
        &resource_dir(),
        &sce_build::ForgeCompileOptions::default(),
    );
    let err = match result {
        Ok(_) => panic!(
            "<sce:test-vector> under sce:kind=\"filter\" must reject \
             with algorithm/test-vector-unsupported-kind"
        ),
        Err(e) => e,
    };
    assert!(
        matches!(
            err.error,
            ForgeError::Validation(ValidationError::TestVectorUnsupportedKind {
                ref name,
                kind: ForgeKind::Filter,
            }) if name == "session_filter"
        ),
        "must surface as ValidationError::TestVectorUnsupportedKind naming the document and the rejected kind; got: {:?}",
        err.error
    );
}

/// Negative: malformed `hex` (odd-length) reuses the generic
/// `validation/invalid-attribute` slot — the repair stays
/// attribute-text-level (fix the hex string), no new diagnostic
/// variant warranted.
#[test]
fn forge_test_vector_invalid_hex_rejects() {
    use sce_build::forge::error::{ForgeError, ValidationError};
    use sce_build::forge::parser::parse_forge;
    use sce_build::DocumentLabel;

    let scxml = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       xmlns:sce="http://sce.dev/ext"
       sce:kind="algorithm" name="crc_oracle">
  <sce:signature>
    <sce:param name="data" type="bytes"/>
    <sce:return type="uint16"/>
  </sce:signature>
  <sce:body>
    <sce:return expr="0xFFFF"/>
  </sce:body>
  <sce:test-vector hex="ABC" value="0x29B1"/>
</scxml>"#;

    let err = parse_forge(scxml, DocumentLabel::symmetric("crc_oracle"))
        .err()
        .expect("odd-length hex must reject");
    assert!(
        matches!(
            err.error,
            ForgeError::Validation(ValidationError::InvalidAttribute {
                ref attr,
                ..
            }) if attr == "hex"
        ),
        "odd-length hex must surface as InvalidAttribute on attr='hex'; got: {:?}",
        err.error
    );
}

/// Negative: malformed `value` literal (non-numeric on integer return
/// type) reuses `validation/invalid-attribute`. Same rationale as the
/// hex case — repair stays attribute-text-level.
#[test]
fn forge_test_vector_invalid_value_rejects() {
    use sce_build::forge::error::{ForgeError, ValidationError};
    use sce_build::forge::parser::parse_forge;
    use sce_build::DocumentLabel;

    let scxml = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       xmlns:sce="http://sce.dev/ext"
       sce:kind="algorithm" name="crc_oracle">
  <sce:signature>
    <sce:param name="data" type="bytes"/>
    <sce:return type="uint16"/>
  </sce:signature>
  <sce:body>
    <sce:return expr="0xFFFF"/>
  </sce:body>
  <sce:test-vector hex="01" value="not_a_number"/>
</scxml>"#;

    let err = parse_forge(scxml, DocumentLabel::symmetric("crc_oracle"))
        .err()
        .expect("non-numeric value on integer return type must reject");
    assert!(
        matches!(
            err.error,
            ForgeError::Validation(ValidationError::InvalidAttribute {
                ref attr,
                ..
            }) if attr == "value"
        ),
        "non-numeric value must surface as InvalidAttribute on attr='value'; got: {:?}",
        err.error
    );
}

// ── RFC §5.B variant primitive (B1-β trunk) ──────────────────
// `codec_variant_dispatch` imports two arm-body codecs and exposes
// the discriminated-union shape across Rust + Cpp. Sub-codecs ship
// their own goldens so the syn / cpp parser gates compile each
// fixture in isolation; the dispatch fixture references them via
// `super::...` (Rust) / `::SCE::Generated::...` (Cpp) qualified
// paths that resolve at link time when all three goldens are in
// scope (the conformance harness wiring lands in a follow-up).

#[test]
fn forge_codec_variant_session_open_cpp() {
    assert_standalone_forge("codec_variant_session_open", "codec_variant_session_open.h");
}

#[test]
fn forge_codec_variant_session_close_cpp() {
    assert_standalone_forge(
        "codec_variant_session_close",
        "codec_variant_session_close.h",
    );
}

#[test]
fn forge_codec_variant_dispatch_cpp() {
    assert_standalone_forge("codec_variant_dispatch", "codec_variant_dispatch.h");
}

/// RFC §5.B B5-β multi-bit-flag variant dispatch (Cpp): `<sce:variant
/// tag="header.mid">` extracts the 5-bit MID slice from a uint8 flags
/// carrier and dispatches into KeepAlive (empty body) / Close (uint8
/// reason) / Default arms — mirrors zenoh-pico's transport-message
/// envelope shape (`_z_transport_message_decode`,
/// `_Z_MID_MASK = 0x1f`).
#[test]
fn forge_codec_transport_envelope_cpp() {
    assert_standalone_forge("codec_transport_envelope", "codec_transport_envelope.h");
}

/// RFC §5.B B5-γ trunk (Cpp): body codec with `<sce:requires-parent-flags
/// carrier="header"><sce:flag name="S" bit="6"/></sce:requires-parent-flags>`
/// emits decode/encode signatures that take a `std::uint8_t parent_flags`
/// parameter. Body fields gated by `sce:present-if="parent.S"` read the
/// bit from this parameter rather than from a sibling carrier. Mirrors
/// zenoh-pico's `_z_init_decode(.., uint8_t header)` upstream pattern.
#[test]
fn forge_codec_init_syn_body_cpp() {
    assert_standalone_forge("codec_init_syn_body", "codec_init_syn_body.h");
}

/// RFC §5.B B5-γ trunk (Cpp): variant parent codec whose arm body
/// declares `<sce:requires-parent-flags carrier="header">` — the
/// dispatch threads the parent's `header` flags-carrier value into
/// the body's `decode(cursor, header)` / `encode(header)` calls.
/// Cross-codec layout match (parent's `<sce:flags id="header">` has
/// 'S' at bit=6) validated at codegen time per
/// `codec/parent-flag-mismatch`.
#[test]
fn forge_codec_init_syn_envelope_cpp() {
    assert_standalone_forge("codec_init_syn_envelope", "codec_init_syn_envelope.h");
}

/// RFC §5.B B5-δ Surfaces D + E (Cpp): Init body cookie codec exercising
/// VLE-length-ref bit-size on the length sibling AND a gated length
/// sibling (cookie_size + cookie both gated by `parent.A`). The decode
/// helper unwraps the std::optional<uint16_t> sibling inside the gated
/// branch and reads its value as the byte count for the cookie payload.
/// Lifts the B4 deferral that restricted cookie/attachment sizes to
/// ≤ 127 bytes (single-byte VLE = u8) and the B4 deferral that gated
/// only the payload while always-emitting the length byte.
#[test]
fn forge_codec_init_cookie_body_cpp() {
    assert_standalone_forge("codec_init_cookie_body", "codec_init_cookie_body.h");
}

/// RFC §5.B B5-δ Surface F (Cpp): Scout/Hello/Init zid codec exercising
/// arithmetic offset on the length sibling. Author writes
/// `sce:length-arith="+1"` paired with `sce:length-field="zid_len_m1"`;
/// decode reads `_n = sibling_value + 1` bytes. Mirrors zenoh-pico's
/// `zidlen = ((cbyte & 0xF0) >> 4) + (uint8_t)1` (`transport.c:251`).
#[test]
fn forge_codec_scout_zid_body_cpp() {
    assert_standalone_forge("codec_scout_zid_body", "codec_scout_zid_body.h");
}

/// RFC §5.B B1-β: `codec/variant-arm-unreachable` build-time check —
/// a `<sce:variant>` over a uint8 tag with two arms and no
/// `<sce:default>` leaves 254 tag values uncovered, so the parser
/// must reject with the typed diagnostic naming the codec, the tag,
/// the tag's type, the arm count, and the practically-enumerable
/// domain size. The rejection is what makes the runtime decode
/// total — generated codegen omits the unreachable fallback branch.
#[test]
fn forge_codec_variant_missing_default_rejects() {
    use sce_build::forge::error::{ForgeError, ValidationError};

    let scxml = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       xmlns:sce="http://sce.dev/ext"
       sce:kind="codec" sce:default-endian="big" name="missing_default">
  <sce:import src="codec_variant_session_open.scxml" kind="codec" as="codec_variant_session_open"/>
  <sce:import src="codec_variant_session_close.scxml" kind="codec" as="codec_variant_session_close"/>
  <datamodel>
    <data id="msg_id" sce:type="uint8" sce:byte="0" sce:bit-size="8"/>
    <sce:variant tag="msg_id">
      <sce:arm value="0x01" type="codec_variant_session_open"/>
      <sce:arm value="0x02" type="codec_variant_session_close"/>
    </sce:variant>
  </datamodel>
</scxml>"#;
    let result = sce_build::compile_forge_with_imports(
        scxml,
        sce_build::DocumentLabel::symmetric("missing_default"),
        sce_build::generator::Language::Rust,
        &resource_dir(),
        &sce_build::ForgeCompileOptions::default(),
    );
    let err = match result {
        Ok(_) => panic!(
            "missing-default + non-exhaustive arms must reject with codec/variant-arm-unreachable"
        ),
        Err(e) => e,
    };
    let inner = err.error;
    assert!(
        matches!(
            inner,
            ForgeError::Validation(ValidationError::CodecVariantArmUnreachable {
                ref tag_field,
                arm_count: 2,
                domain_size: Some(256),
                ..
            }) if tag_field == "msg_id"
        ),
        "must surface as ValidationError::CodecVariantArmUnreachable with the offending tag and arm count; got: {inner:?}"
    );
}

/// RFC variant-default-uniformity Atomic α: `<sce:arm default="true"/>`
/// is accepted on at most one arm — a second declaration raises
/// `codec/variant-duplicate-default-arm` with both offending arm
/// values preserved on the diagnostic so authors can identify
/// which arm to demote. Independent of the catch-all `<sce:default>`
/// element (per RFC §3 Q-V3 (a) the two are distinct concepts).
#[test]
fn forge_codec_variant_duplicate_default_arm_rejects() {
    use sce_build::forge::error::{ForgeError, ValidationError};

    let scxml = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       xmlns:sce="http://sce.dev/ext"
       sce:kind="codec" sce:default-endian="big" name="dup_default_arm">
  <sce:import src="codec_variant_session_open.scxml" kind="codec" as="codec_variant_session_open"/>
  <sce:import src="codec_variant_session_close.scxml" kind="codec" as="codec_variant_session_close"/>
  <datamodel>
    <data id="msg_id" sce:type="uint8" sce:byte="0" sce:bit-size="8"/>
    <sce:variant tag="msg_id">
      <sce:arm value="0x01" type="codec_variant_session_open" default="true"/>
      <sce:arm value="0x02" type="codec_variant_session_close" default="true"/>
      <sce:default type="codec_variant_session_open"/>
    </sce:variant>
  </datamodel>
</scxml>"#;
    let result = sce_build::compile_forge_with_imports(
        scxml,
        sce_build::DocumentLabel::symmetric("dup_default_arm"),
        sce_build::generator::Language::Rust,
        &resource_dir(),
        &sce_build::ForgeCompileOptions::default(),
    );
    let err = match result {
        Ok(_) => panic!(
            "two <sce:arm default=\"true\"/> must reject with codec/variant-duplicate-default-arm"
        ),
        Err(e) => e,
    };
    let inner = err.error;
    assert!(
        matches!(
            inner,
            ForgeError::Validation(ValidationError::CodecVariantDuplicateDefaultArm {
                ref codec,
                first_arm_value: 0x01,
                second_arm_value: 0x02,
            }) if codec == "dup_default_arm"
        ),
        "must surface as ValidationError::CodecVariantDuplicateDefaultArm with both arm values; got: {inner:?}"
    );
}

/// RFC variant-default-uniformity Atomic γ-1: when the outer
/// `<sce:arm default="true" value="X"/>` selects an inner codec
/// that declares `<sce:flag value="Y"/>` on its matching peek-byte
/// flag with X ≠ Y, codegen rejects with `codec/variant-default-
/// arm-mid-mismatch`. The inner codec's `Default::default()` would
/// emit Y; the outer's dispatch table at decode time would route
/// Y to whichever arm has `value="Y"` — not the marked-default
/// arm — so round-trip is broken.
#[test]
fn forge_codec_variant_default_arm_mid_mismatch_rejects() {
    use sce_build::forge::error::{ForgeError, ValidationError};

    // codec_default_marker_arm_a declares <sce:flag name="kind"
    // value="0x01"/>. Mark a hypothetical outer arm with
    // value="0x03" default="true" → expected slice 0x03 vs inner
    // 0x01 → mismatch.
    let scxml = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       xmlns:sce="http://sce.dev/ext"
       sce:kind="codec" sce:default-endian="big" name="mid_mismatch_outer">
  <sce:import src="codec_default_marker_arm_a.scxml" kind="codec" as="codec_default_marker_arm_a"/>
  <sce:import src="codec_default_marker_arm_b.scxml" kind="codec" as="codec_default_marker_arm_b"/>
  <datamodel>
    <sce:variant tag="peek.kind">
      <sce:peek-byte id="peek" sce:type="uint8">
        <sce:flag name="kind" bit="0" width="2"/>
      </sce:peek-byte>
      <sce:arm value="0x03" type="codec_default_marker_arm_a" default="true"/>
      <sce:arm value="0x02" type="codec_default_marker_arm_b"/>
      <sce:default type="codec_default_marker_arm_b"/>
    </sce:variant>
  </datamodel>
</scxml>"#;
    let result = sce_build::compile_forge_with_imports(
        scxml,
        sce_build::DocumentLabel::symmetric("mid_mismatch_outer"),
        sce_build::generator::Language::Rust,
        &resource_dir(),
        &sce_build::ForgeCompileOptions::default(),
    );
    let err = match result {
        Ok(_) => panic!(
            "outer arm value 0x03 vs inner flag value 0x01 must reject \
             with codec/variant-arm-mid-mismatch"
        ),
        Err(e) => e,
    };
    assert!(
        matches!(
            err.error,
            ForgeError::Validation(ValidationError::CodecVariantArmMidMismatch {
                ref codec,
                arm_value: 0x03,
                ref inner_codec,
                ref inner_flag,
                inner_flag_value: 0x01,
            }) if codec == "mid_mismatch_outer"
                && inner_codec == "codec_default_marker_arm_a"
                && inner_flag == "kind"
        ),
        "must surface as ValidationError::CodecVariantArmMidMismatch \
         with the 4-tuple (codec, arm_value, inner_codec, inner_flag_value); \
         got: {:?}",
        err.error
    );
}

/// RFC variant-default-uniformity Atomic γ-1: when the outer
/// `<sce:arm default="true"/>` selects an inner codec that does
/// NOT declare `<sce:flag value="..."/>` on its dispatch field —
/// either because the inner has no flags carrier at offset 0
/// (B5-η-stripped leaf) or because the matching flag is declared
/// without a value — codegen rejects with `codec/variant-arm-
/// inner-mid-undeclared`. Without the wire-MID baked into the
/// inner's Default, round-trip would zero-fill the dispatch byte
/// and land in the catch-all arm.
#[test]
fn forge_codec_variant_arm_inner_mid_undeclared_rejects() {
    use sce_build::forge::error::{ForgeError, ValidationError};

    // codec_variant_session_open has a uint16 field at offset 0
    // (no <sce:flags> carrier), so the inner has no
    // codec_first_flags → my validator's first early-return path
    // fires with the peek-byte's flag name as the expected
    // location.
    let scxml = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       xmlns:sce="http://sce.dev/ext"
       sce:kind="codec" sce:default-endian="big" name="inner_mid_undeclared_outer">
  <sce:import src="codec_variant_session_open.scxml" kind="codec" as="codec_variant_session_open"/>
  <sce:import src="codec_variant_session_close.scxml" kind="codec" as="codec_variant_session_close"/>
  <datamodel>
    <sce:variant tag="peek.kind">
      <sce:peek-byte id="peek" sce:type="uint8">
        <sce:flag name="kind" bit="0" width="2"/>
      </sce:peek-byte>
      <sce:arm value="0x01" type="codec_variant_session_open" default="true"/>
      <sce:arm value="0x02" type="codec_variant_session_close"/>
      <sce:default type="codec_variant_session_close"/>
    </sce:variant>
  </datamodel>
</scxml>"#;
    let result = sce_build::compile_forge_with_imports(
        scxml,
        sce_build::DocumentLabel::symmetric("inner_mid_undeclared_outer"),
        sce_build::generator::Language::Rust,
        &resource_dir(),
        &sce_build::ForgeCompileOptions::default(),
    );
    let err = match result {
        Ok(_) => panic!(
            "inner codec with no <sce:flag value=> must reject with \
             codec/variant-arm-inner-mid-undeclared when an outer arm \
             selects it as default"
        ),
        Err(e) => e,
    };
    assert!(
        matches!(
            err.error,
            ForgeError::Validation(ValidationError::CodecVariantArmInnerMidUndeclared {
                ref codec,
                arm_value: 0x01,
                ref inner_codec,
                ref expected_flag,
            }) if codec == "inner_mid_undeclared_outer"
                && inner_codec == "codec_variant_session_open"
                && expected_flag == "kind"
        ),
        "must surface as ValidationError::CodecVariantArmInnerMidUndeclared \
         keyed on (codec, arm_value, inner_codec, expected_flag); got: {:?}",
        err.error
    );
}

/// RFC variant-default-uniformity Atomic γ-3 (Q-V4 (a)): every
/// `<sce:variant>` must declare an `<sce:arm default="true"/>` —
/// codegen rejects the legacy "no marker = pick first declared
/// arm" implicit fallback that led to the watching-zenoh R87
/// defect. The catch-all `<sce:default>` is a separate concept
/// and does not satisfy the requirement (Q-V3 (a)).
#[test]
fn forge_codec_variant_no_default_arm_rejects() {
    use sce_build::forge::error::{ForgeError, ValidationError};

    let scxml = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       xmlns:sce="http://sce.dev/ext"
       sce:kind="codec" sce:default-endian="big" name="no_default_arm">
  <sce:import src="codec_variant_session_open.scxml" kind="codec" as="codec_variant_session_open"/>
  <sce:import src="codec_variant_session_close.scxml" kind="codec" as="codec_variant_session_close"/>
  <datamodel>
    <data id="msg_id" sce:type="uint8" sce:byte="0" sce:bit-size="8"/>
    <sce:variant tag="msg_id">
      <sce:arm value="0x01" type="codec_variant_session_open"/>
      <sce:arm value="0x02" type="codec_variant_session_close"/>
      <sce:default type="codec_variant_session_close"/>
    </sce:variant>
  </datamodel>
</scxml>"#;
    let result = sce_build::compile_forge_with_imports(
        scxml,
        sce_build::DocumentLabel::symmetric("no_default_arm"),
        sce_build::generator::Language::Rust,
        &resource_dir(),
        &sce_build::ForgeCompileOptions::default(),
    );
    let err = match result {
        Ok(_) => panic!(
            "variant with no <sce:arm default=\"true\"/> must reject with \
             codec/variant-no-default-arm even when <sce:default> catch-all \
             is declared"
        ),
        Err(e) => e,
    };
    assert!(
        matches!(
            err.error,
            ForgeError::Validation(ValidationError::CodecVariantNoDefaultArm {
                ref codec,
            }) if codec == "no_default_arm"
        ),
        "must surface as ValidationError::CodecVariantNoDefaultArm; got: {:?}",
        err.error
    );
}

/// RFC variant-default-uniformity Atomic α: `default="..."` on
/// `<sce:arm>` is typed as `xs:boolean` in the XSD, so a misspelling
/// like `default="yes"` is caught at the structural XSD layer before
/// the parser runs. The parser's own fallback (rejecting non-"true"/
/// "false" tokens) provides defense-in-depth for paths where the
/// XSD validator is not invoked, but in the production pipeline the
/// schema layer always fires first.
#[test]
fn forge_codec_variant_arm_invalid_default_value_rejects() {
    use sce_build::forge::error::ForgeError;

    let scxml = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       xmlns:sce="http://sce.dev/ext"
       sce:kind="codec" sce:default-endian="big" name="bad_default_value">
  <sce:import src="codec_variant_session_open.scxml" kind="codec" as="codec_variant_session_open"/>
  <datamodel>
    <data id="msg_id" sce:type="uint8" sce:byte="0" sce:bit-size="8"/>
    <sce:variant tag="msg_id">
      <sce:arm value="0x01" type="codec_variant_session_open" default="yes"/>
      <sce:default type="codec_variant_session_open"/>
    </sce:variant>
  </datamodel>
</scxml>"#;
    let result = sce_build::compile_forge_with_imports(
        scxml,
        sce_build::DocumentLabel::symmetric("bad_default_value"),
        sce_build::generator::Language::Rust,
        &resource_dir(),
        &sce_build::ForgeCompileOptions::default(),
    );
    let err = match result {
        Ok(_) => panic!("non-boolean default= attribute must reject"),
        Err(e) => e,
    };
    let inner = err.error;
    assert!(
        matches!(&inner, ForgeError::Xml(_)),
        "must surface at the XSD layer (xs:boolean rejects 'yes'); got: {inner:?}"
    );
}

/// RFC variant-default-uniformity Atomic α: `<sce:flag value="N"/>`
/// must fit the declared bit-range — values exceeding
/// `(1 << width) - 1` reject because the high bits would silently
/// overlap adjacent flags' ranges. The `priority` field in
/// `codec_qos_byte` is width=3, so any value > 7 must reject.
#[test]
fn forge_codec_flag_value_out_of_range_rejects() {
    use sce_build::forge::error::{ForgeError, ValidationError};

    let scxml = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       xmlns:sce="http://sce.dev/ext"
       sce:kind="codec" sce:default-endian="big" name="flag_value_oor">
  <datamodel>
    <sce:flags id="qos" sce:type="uint8" sce:byte="0" sce:bit-size="8">
      <sce:flag name="priority" bit="0" width="3" value="0x10"/>
      <sce:flag name="reliable" bit="3"/>
    </sce:flags>
  </datamodel>
</scxml>"#;
    let result = sce_build::compile_forge_with_imports(
        scxml,
        sce_build::DocumentLabel::symmetric("flag_value_oor"),
        sce_build::generator::Language::Rust,
        &resource_dir(),
        &sce_build::ForgeCompileOptions::default(),
    );
    let err = match result {
        Ok(_) => panic!("<sce:flag value=> exceeding (1<<width)-1 must reject"),
        Err(e) => e,
    };
    let inner = err.error;
    assert!(
        matches!(
            &inner,
            ForgeError::Validation(ValidationError::InvalidAttribute { attr, .. })
                if attr == "value"
        ),
        "must surface as ValidationError::InvalidAttribute on the value attribute; got: {inner:?}"
    );
}

/// RFC variant-default-uniformity Atomic α: `<sce:flag value="N"/>`
/// must parse as an unsigned integer (decimal or 0x-hex). A
/// non-numeric token rejects with the same `validation/numeric-parse`
/// shape as the existing `bit=`/`width=` parser.
#[test]
fn forge_codec_flag_value_malformed_rejects() {
    use sce_build::forge::error::{ForgeError, ValidationError};

    let scxml = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       xmlns:sce="http://sce.dev/ext"
       sce:kind="codec" sce:default-endian="big" name="flag_value_bad">
  <datamodel>
    <sce:flags id="qos" sce:type="uint8" sce:byte="0" sce:bit-size="8">
      <sce:flag name="priority" bit="0" width="3" value="abc"/>
      <sce:flag name="reliable" bit="3"/>
    </sce:flags>
  </datamodel>
</scxml>"#;
    let result = sce_build::compile_forge_with_imports(
        scxml,
        sce_build::DocumentLabel::symmetric("flag_value_bad"),
        sce_build::generator::Language::Rust,
        &resource_dir(),
        &sce_build::ForgeCompileOptions::default(),
    );
    let err = match result {
        Ok(_) => panic!("<sce:flag value=> non-numeric must reject"),
        Err(e) => e,
    };
    let inner = err.error;
    assert!(
        matches!(
            &inner,
            ForgeError::Validation(ValidationError::NumericParse { attr, .. })
                if attr == "value"
        ),
        "must surface as ValidationError::NumericParse on the value attribute; got: {inner:?}"
    );
}

/// RFC variant-default-uniformity Atomic α: happy path — a single
/// arm with `default="true"` AND a catch-all `<sce:default>`
/// coexist without conflict (Q-V3 (a) lock — the two declarations
/// answer independent questions). Verifies that codegen still
/// succeeds end-to-end; the Atomic α surface is parse-only so
/// no emission shape changes here.
#[test]
fn forge_codec_variant_default_arm_and_catch_all_coexist() {
    let scxml = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       xmlns:sce="http://sce.dev/ext"
       sce:kind="codec" sce:default-endian="big" name="default_arm_with_catch_all">
  <sce:import src="codec_variant_session_open.scxml" kind="codec" as="codec_variant_session_open"/>
  <sce:import src="codec_variant_session_close.scxml" kind="codec" as="codec_variant_session_close"/>
  <datamodel>
    <data id="msg_id" sce:type="uint8" sce:byte="0" sce:bit-size="8"/>
    <sce:variant tag="msg_id">
      <sce:arm value="0x01" type="codec_variant_session_open" default="true"/>
      <sce:arm value="0x02" type="codec_variant_session_close"/>
      <sce:default type="codec_variant_session_open"/>
    </sce:variant>
  </datamodel>
</scxml>"#;
    sce_build::compile_forge_with_imports(
        scxml,
        sce_build::DocumentLabel::symmetric("default_arm_with_catch_all"),
        sce_build::generator::Language::Rust,
        &resource_dir(),
        &sce_build::ForgeCompileOptions::default(),
    )
    .expect("default=\"true\" on one arm + <sce:default> catch-all must coexist (RFC Q-V3 (a))");
}

/// RFC variant-default-uniformity Atomic β (Rust): the inner arm body
/// codec with `<sce:flag value="0x02"/>` on its dispatch flag must
/// emit a manual `impl Default` that bakes the wire-MID into the
/// carrier byte. Without this, `CodecDefaultMarkerArmB::default()`
/// would zero-fill the header and break the round-trip invariant.
#[test]
fn forge_codec_default_marker_arm_b_emits_baked_default_rust() {
    let scxml_path = resource_dir().join("codec_default_marker_arm_b.scxml");
    let content =
        std::fs::read_to_string(&scxml_path).expect("codec_default_marker_arm_b.scxml must exist");
    let output = sce_build::compile_forge_with_imports(
        &content,
        sce_build::DocumentLabel::symmetric("codec_default_marker_arm_b"),
        sce_build::generator::Language::Rust,
        &resource_dir(),
        &sce_build::ForgeCompileOptions::default(),
    )
    .expect("forge codegen for arm B fixture must succeed");
    let (_, generated) = &output.files[0];
    assert!(
        !generated.contains("#[derive(Default)]"),
        "arm B has <sce:flag value=>; the codec struct must use a manual \
         impl Default rather than #[derive(Default)] so the wire-MID \
         constant is baked in. Generated source:\n{generated}"
    );
    assert!(
        generated.contains("impl Default for CodecDefaultMarkerArmB"),
        "arm B's generated source must contain a manual `impl Default for \
         CodecDefaultMarkerArmB`. Generated source:\n{generated}"
    );
    assert!(
        generated.contains("header: 0x02u8"),
        "arm B's Default impl must initialize `header` to the wire-MID \
         literal `0x02u8` (value=\"0x02\" shifted by bit=0). Generated \
         source:\n{generated}"
    );
    // Syntactic validation — caught early if any template branch
    // produces invalid Rust.
    syn::parse_file(generated)
        .unwrap_or_else(|e| panic!("generated arm B source must parse as valid Rust: {e}"));
}

/// RFC variant-default-uniformity Atomic β (C++): the inner arm body
/// codec with `<sce:flag value="0x02"/>` on its dispatch flag must
/// emit a default member initializer `std::uint8_t header{0x02u}` so
/// a freshly-constructed instance carries the wire-MID for its
/// dispatch tag. C++'s analog of Rust's manual `impl Default` is a
/// brace-init right on the field declaration.
#[test]
fn forge_codec_default_marker_arm_b_emits_baked_default_cpp() {
    let scxml_path = resource_dir().join("codec_default_marker_arm_b.scxml");
    let content =
        std::fs::read_to_string(&scxml_path).expect("codec_default_marker_arm_b.scxml must exist");
    let output = sce_build::compile_forge_with_imports(
        &content,
        sce_build::DocumentLabel::symmetric("codec_default_marker_arm_b"),
        sce_build::generator::Language::Cpp,
        &resource_dir(),
        &sce_build::ForgeCompileOptions::default(),
    )
    .expect("forge codegen for arm B fixture must succeed (cpp)");
    let (_, generated) = &output.files[0];
    assert!(
        generated.contains("header{0x02u}"),
        "arm B's struct must declare `header` with a default member \
         initializer `{{0x02u}}` (value=\"0x02\" shifted by bit=0). \
         Generated source:\n{generated}"
    );
}

/// RFC variant-default-uniformity Atomic β (C++): the outer codec's
/// `body` `std::variant` member must use `std::in_place_index<N>{}`
/// to select the declared default arm — `N` is the 0-based index of
/// the arm marked `default="true"`. Without this, `std::variant`'s
/// default constructor would pick the first alternative (arm A,
/// index 0), and the round-trip dispatch would land in the wrong
/// arm.
#[test]
fn forge_codec_variant_default_marker_outer_emits_declared_arm_cpp() {
    let scxml_path = resource_dir().join("codec_variant_default_marker.scxml");
    let content = std::fs::read_to_string(&scxml_path)
        .expect("codec_variant_default_marker.scxml must exist");
    let output = sce_build::compile_forge_with_imports(
        &content,
        sce_build::DocumentLabel::symmetric("codec_variant_default_marker"),
        sce_build::generator::Language::Cpp,
        &resource_dir(),
        &sce_build::ForgeCompileOptions::default(),
    )
    .expect("forge codegen for outer marker fixture must succeed (cpp)");
    let (_, generated) = &output.files[0];
    // Arm B is the second declared (index 1) — value=0x02 marked
    // default="true". The outer body member must select index 1, not
    // 0 (the legacy std::variant default). We use `in_place_index_t`
    // explicitly (not `in_place_index<N>{}`, which is a variable
    // template and won't parse in member-init position).
    assert!(
        generated.contains("body{std::in_place_index_t<1>{}}"),
        "outer body must use `std::in_place_index_t<1>{{}}` to select \
         the declared-default arm B. Generated source:\n{generated}"
    );
}

/// RFC variant-default-uniformity Atomic β (Kotlin): the inner arm
/// body codec with `<sce:flag value="0x02"/>` on its dispatch flag
/// must emit a UByte-typed default `header: UByte = 0x02.toUByte()`
/// on the data class primary constructor so a freshly-constructed
/// instance carries the wire-MID for its dispatch tag. Kotlin's
/// `u`/`uL` literal suffixes produce UInt / ULong respectively, so
/// UByte / UShort carriers narrow from Int via `.toU{Byte,Short}()`
/// — mirroring the existing `kotlin_default()` zero pattern
/// (`"0.toUByte()"`).
#[test]
fn forge_codec_default_marker_arm_b_emits_baked_default_kotlin() {
    let scxml_path = resource_dir().join("codec_default_marker_arm_b.scxml");
    let content =
        std::fs::read_to_string(&scxml_path).expect("codec_default_marker_arm_b.scxml must exist");
    let output = sce_build::compile_forge_with_imports(
        &content,
        sce_build::DocumentLabel::symmetric("codec_default_marker_arm_b"),
        sce_build::generator::Language::Kotlin,
        &resource_dir(),
        &sce_build::ForgeCompileOptions::default(),
    )
    .expect("forge codegen for arm B fixture must succeed (kotlin)");
    let (_, generated) = &output.files[0];
    assert!(
        generated.contains("header: UByte = 0x02.toUByte()"),
        "arm B's data class must declare `header: UByte` with a \
         default value of `0x02.toUByte()` (value=\"0x02\" shifted by \
         bit=0). Generated source:\n{generated}"
    );
}

/// RFC variant-default-uniformity Atomic β (Kotlin): the outer
/// codec's primary-constructor `body` default must construct the
/// declared default arm — the one marked `default="true"` — rather
/// than the first declared alternative. Kotlin has no positional
/// `std::variant`-style constructor, so the template selects the arm
/// by sealed-class subtype directly (`Variant.CodecDefaultMarkerArmB(...)`).
#[test]
fn forge_codec_variant_default_marker_outer_emits_declared_arm_kotlin() {
    let scxml_path = resource_dir().join("codec_variant_default_marker.scxml");
    let content = std::fs::read_to_string(&scxml_path)
        .expect("codec_variant_default_marker.scxml must exist");
    let output = sce_build::compile_forge_with_imports(
        &content,
        sce_build::DocumentLabel::symmetric("codec_variant_default_marker"),
        sce_build::generator::Language::Kotlin,
        &resource_dir(),
        &sce_build::ForgeCompileOptions::default(),
    )
    .expect("forge codegen for outer marker fixture must succeed (kotlin)");
    let (_, generated) = &output.files[0];
    // Arm B is marked `default="true"`. The outer body default must
    // wrap a CodecDefaultMarkerArmB instance, not the first declared
    // arm (CodecDefaultMarkerArmA). Arm body types are referenced by
    // FQN in the Kotlin emit to keep imported-codec resolution
    // unambiguous when two imports share a class name.
    assert!(
        generated.contains(
            "CodecVariantDefaultMarkerVariant.CodecDefaultMarkerArmB(\
             com.sce.generated.codec_default_marker_arm_b.CodecDefaultMarkerArmB())"
        ),
        "outer body default must wrap the arm marked \
         default=\"true\" (CodecDefaultMarkerArmB). Generated source:\n{generated}"
    );
    assert!(
        !generated.contains(
            "CodecVariantDefaultMarkerVariant.CodecDefaultMarkerArmA(\
             com.sce.generated.codec_default_marker_arm_a.CodecDefaultMarkerArmA())"
        ),
        "outer body default must NOT wrap the first declared arm \
         (CodecDefaultMarkerArmA) when another arm is marked \
         default=\"true\". Generated source:\n{generated}"
    );
}

/// RFC variant-default-uniformity Atomic β (Python): the inner arm
/// body codec with `<sce:flag value="0x02"/>` on its dispatch flag
/// must emit a Python-int default `header: int = 0x02` on the
/// `@dataclass` so a freshly-constructed instance carries the
/// wire-MID for its dispatch tag. Python's `int` is unbounded so
/// the literal needs no carrier-width suffix.
#[test]
fn forge_codec_default_marker_arm_b_emits_baked_default_python() {
    let scxml_path = resource_dir().join("codec_default_marker_arm_b.scxml");
    let content =
        std::fs::read_to_string(&scxml_path).expect("codec_default_marker_arm_b.scxml must exist");
    let output = sce_build::compile_forge_with_imports(
        &content,
        sce_build::DocumentLabel::symmetric("codec_default_marker_arm_b"),
        sce_build::generator::Language::Python,
        &resource_dir(),
        &sce_build::ForgeCompileOptions::default(),
    )
    .expect("forge codegen for arm B fixture must succeed (python)");
    let (_, generated) = &output.files[0];
    assert!(
        generated.contains("header: int = 0x02"),
        "arm B's @dataclass must declare `header: int = 0x02` \
         (value=\"0x02\" shifted by bit=0). Generated source:\n{generated}"
    );
}

/// RFC variant-default-uniformity Atomic β (Python): the outer
/// codec's `Variant` @dataclass must (a) select the declared default
/// arm via `kind = "<ArmName>"` AND (b) populate that arm's body
/// field via `field(default_factory=ArmType)`. Without (b) the body
/// would remain `None` even though `kind` names the arm — a latent
/// inconsistency the RFC closes alongside the dispatch fix.
#[test]
fn forge_codec_variant_default_marker_outer_emits_declared_arm_python() {
    let scxml_path = resource_dir().join("codec_variant_default_marker.scxml");
    let content = std::fs::read_to_string(&scxml_path)
        .expect("codec_variant_default_marker.scxml must exist");
    let output = sce_build::compile_forge_with_imports(
        &content,
        sce_build::DocumentLabel::symmetric("codec_variant_default_marker"),
        sce_build::generator::Language::Python,
        &resource_dir(),
        &sce_build::ForgeCompileOptions::default(),
    )
    .expect("forge codegen for outer marker fixture must succeed (python)");
    let (_, generated) = &output.files[0];
    assert!(
        generated.contains("kind: str = \"CodecDefaultMarkerArmB\""),
        "outer Variant must select kind=CodecDefaultMarkerArmB (the \
         arm marked default=\"true\"). Generated source:\n{generated}"
    );
    assert!(
        generated.contains(
            "codec_default_marker_arm_b: Optional[CodecDefaultMarkerArmB] = \
             field(default_factory=CodecDefaultMarkerArmB)"
        ),
        "the declared default arm's body field must be populated via \
         field(default_factory=CodecDefaultMarkerArmB), not left None. \
         Generated source:\n{generated}"
    );
    assert!(
        !generated.contains("kind: str = \"CodecDefaultMarkerArmA\""),
        "outer Variant must NOT select kind=CodecDefaultMarkerArmA (the \
         first declared arm) when another arm is marked default. \
         Generated source:\n{generated}"
    );
}

/// RFC variant-default-uniformity Atomic β (Go): the inner arm body
/// codec with `<sce:flag value="0x02"/>` must emit a `NewT()`
/// constructor returning a struct literal with `Header: uint8(0x02)`
/// baked in. Go has no Default trait, so round-trip safety requires
/// callers use `NewT()` instead of `T{}` (the zero-value).
#[test]
fn forge_codec_default_marker_arm_b_emits_baked_default_go() {
    let scxml_path = resource_dir().join("codec_default_marker_arm_b.scxml");
    let content =
        std::fs::read_to_string(&scxml_path).expect("codec_default_marker_arm_b.scxml must exist");
    let mut opts = sce_build::ForgeCompileOptions::default();
    opts.go_module_prefix = Some("github.com/test/codec".to_string());
    let output = sce_build::compile_forge_with_imports(
        &content,
        sce_build::DocumentLabel::symmetric("codec_default_marker_arm_b"),
        sce_build::generator::Language::Go,
        &resource_dir(),
        &opts,
    )
    .expect("forge codegen for arm B fixture must succeed (go)");
    let (_, generated) = &output.files[0];
    assert!(
        generated.contains("func NewCodecDefaultMarkerArmB() *CodecDefaultMarkerArmB"),
        "arm B must emit a `NewCodecDefaultMarkerArmB()` constructor. \
         Generated source:\n{generated}"
    );
    assert!(
        generated.contains("Header: uint8(0x02),"),
        "NewCodecDefaultMarkerArmB must bake `Header: uint8(0x02)` \
         (value=\"0x02\" shifted by bit=0). Generated source:\n{generated}"
    );
}

/// RFC variant-default-uniformity Atomic β (Go): the outer codec
/// must emit a `NewT()` constructor whose returned struct's `Body`
/// is a Variant with the declared default arm's pointer populated
/// via the inner codec's `New<Arm>()`. The bare `T{}` zero-value
/// would leave every Variant pointer nil and break round-trip.
#[test]
fn forge_codec_variant_default_marker_outer_emits_declared_arm_go() {
    let scxml_path = resource_dir().join("codec_variant_default_marker.scxml");
    let content = std::fs::read_to_string(&scxml_path)
        .expect("codec_variant_default_marker.scxml must exist");
    let mut opts = sce_build::ForgeCompileOptions::default();
    opts.go_module_prefix = Some("github.com/test/codec".to_string());
    let output = sce_build::compile_forge_with_imports(
        &content,
        sce_build::DocumentLabel::symmetric("codec_variant_default_marker"),
        sce_build::generator::Language::Go,
        &resource_dir(),
        &opts,
    )
    .expect("forge codegen for outer marker fixture must succeed (go)");
    let (_, generated) = &output.files[0];
    assert!(
        generated.contains("func NewCodecVariantDefaultMarker() *CodecVariantDefaultMarker"),
        "outer must emit a `NewCodecVariantDefaultMarker()` constructor. \
         Generated source:\n{generated}"
    );
    assert!(
        generated.contains(
            "CodecDefaultMarkerArmB: codec_default_marker_arm_b.NewCodecDefaultMarkerArmB(),"
        ),
        "outer ctor must populate the declared default arm \
         (CodecDefaultMarkerArmB) via its own NewT() call. Generated \
         source:\n{generated}"
    );
    assert!(
        !generated.contains("CodecDefaultMarkerArmA: codec_default_marker_arm_a"),
        "outer ctor must NOT populate the first declared arm \
         (CodecDefaultMarkerArmA) when another arm is marked default. \
         Generated source:\n{generated}"
    );
}

/// RFC variant-default-uniformity Atomic β (C11): the inner arm body
/// codec with `<sce:flag value="0x02"/>` must emit a designated-
/// initializer macro `<UPPER>_DEFAULT_INIT { .header = 0x02u, }` so
/// `codec_t x = <UPPER>_DEFAULT_INIT;` constructs an instance whose
/// wire byte is the arm's MID. C has no Default trait; the macro is
/// the textbook header-only equivalent (no linkage / no function-
/// call overhead, composable from outer codec's own macro).
#[test]
fn forge_codec_default_marker_arm_b_emits_baked_default_c11() {
    let scxml_path = resource_dir().join("codec_default_marker_arm_b.scxml");
    let content =
        std::fs::read_to_string(&scxml_path).expect("codec_default_marker_arm_b.scxml must exist");
    let output = sce_build::compile_forge_with_imports(
        &content,
        sce_build::DocumentLabel::symmetric("codec_default_marker_arm_b"),
        sce_build::generator::Language::C11,
        &resource_dir(),
        &sce_build::ForgeCompileOptions::default(),
    )
    .expect("forge codegen for arm B fixture must succeed (c11)");
    let (_, generated) = &output.files[0];
    assert!(
        generated.contains("#define CODEC_DEFAULT_MARKER_ARM_B_DEFAULT_INIT"),
        "arm B must emit the `_DEFAULT_INIT` macro. Generated source:\n{generated}"
    );
    assert!(
        generated.contains(".header = 0x02u,"),
        "arm B's `_DEFAULT_INIT` macro must bake `.header = 0x02u` \
         (value=\"0x02\" shifted by bit=0). Generated source:\n{generated}"
    );
}

/// RFC variant-default-uniformity Atomic β (C11): the outer codec
/// must emit a `_DEFAULT_INIT` macro whose body initializes the
/// Variant `.kind` to the declared default arm's enum constant and
/// composes the inner arm's own `_DEFAULT_INIT` into the matching
/// union slot. Macro composition keeps wire-MID propagation
/// compile-time-constant without a runtime constructor call.
#[test]
fn forge_codec_variant_default_marker_outer_emits_declared_arm_c11() {
    let scxml_path = resource_dir().join("codec_variant_default_marker.scxml");
    let content = std::fs::read_to_string(&scxml_path)
        .expect("codec_variant_default_marker.scxml must exist");
    let output = sce_build::compile_forge_with_imports(
        &content,
        sce_build::DocumentLabel::symmetric("codec_variant_default_marker"),
        sce_build::generator::Language::C11,
        &resource_dir(),
        &sce_build::ForgeCompileOptions::default(),
    )
    .expect("forge codegen for outer marker fixture must succeed (c11)");
    let (_, generated) = &output.files[0];
    assert!(
        generated.contains("#define CODEC_VARIANT_DEFAULT_MARKER_DEFAULT_INIT"),
        "outer must emit the `_DEFAULT_INIT` macro. Generated source:\n{generated}"
    );
    assert!(
        generated
            .contains(".kind = CODEC_VARIANT_DEFAULT_MARKER_BODY_KIND_CODEC_DEFAULT_MARKER_ARM_B,"),
        "outer `_DEFAULT_INIT` must select the declared default arm's \
         kind enum (CODEC_VARIANT_DEFAULT_MARKER_BODY_KIND_CODEC_DEFAULT_MARKER_ARM_B). \
         Generated source:\n{generated}"
    );
    assert!(
        generated.contains(
            ".arm = { .codec_default_marker_arm_b = CODEC_DEFAULT_MARKER_ARM_B_DEFAULT_INIT }"
        ),
        "outer `_DEFAULT_INIT` must compose the inner arm's own \
         `_DEFAULT_INIT` macro into the union slot. Generated \
         source:\n{generated}"
    );
    assert!(
        !generated
            .contains(".kind = CODEC_VARIANT_DEFAULT_MARKER_BODY_KIND_CODEC_DEFAULT_MARKER_ARM_A,"),
        "outer `_DEFAULT_INIT` must NOT select the first declared arm \
         (CODEC_DEFAULT_MARKER_ARM_A) when another arm is marked \
         default. Generated source:\n{generated}"
    );
}

/// RFC variant-default-uniformity Atomic β (Rust): the outer codec's
/// `*Variant::default()` must pick the arm marked `default="true"`
/// (not the first declared arm). Combined with the inner manual
/// Default (previous test), this closes the round-trip dispatch
/// loop — `Outer::default().encode()` produces a byte stream that
/// `Outer::decode()` resolves back to the same arm variant.
#[test]
fn forge_codec_variant_default_marker_outer_emits_declared_arm_rust() {
    let scxml_path = resource_dir().join("codec_variant_default_marker.scxml");
    let content = std::fs::read_to_string(&scxml_path)
        .expect("codec_variant_default_marker.scxml must exist");
    let output = sce_build::compile_forge_with_imports(
        &content,
        sce_build::DocumentLabel::symmetric("codec_variant_default_marker"),
        sce_build::generator::Language::Rust,
        &resource_dir(),
        &sce_build::ForgeCompileOptions::default(),
    )
    .expect("forge codegen for outer marker fixture must succeed");
    let (_, generated) = &output.files[0];
    // The declared-default arm is `CodecDefaultMarkerArmB` (value=0x02
    // is marked `default="true"`). The Variant::default() impl must
    // pick it, NOT the first declared arm (CodecDefaultMarkerArmA).
    assert!(
        generated.contains("Self::CodecDefaultMarkerArmB(CodecDefaultMarkerArmB::default())"),
        "outer Variant::default() must select the arm marked \
         default=\"true\" (CodecDefaultMarkerArmB) — not the first \
         declared arm. Generated source:\n{generated}"
    );
    assert!(
        !generated.contains("Self::CodecDefaultMarkerArmA(CodecDefaultMarkerArmA::default())"),
        "outer Variant::default() must NOT select the first declared arm \
         (CodecDefaultMarkerArmA) when another arm is marked \
         default=\"true\". Generated source:\n{generated}"
    );
    syn::parse_file(generated)
        .unwrap_or_else(|e| panic!("generated outer source must parse as valid Rust: {e}"));
}

/// RFC §5.B B5-β multi-bit-flag dispatch: `<sce:variant
/// tag="<carrier>.<flag>"/>` requires the carrier to be a
/// `<sce:flags>`-bearing field. Pointing at a plain field rejects
/// with `validation/invalid-attribute` so the author sees the
/// correct repair (either author the carrier as `<sce:flags>` or
/// switch to bare `tag="<field>"` whole-field dispatch).
#[test]
fn forge_codec_variant_dotted_tag_carrier_not_flags_rejects() {
    use sce_build::forge::error::{ForgeError, ValidationError};

    let scxml = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       xmlns:sce="http://sce.dev/ext"
       sce:kind="codec" sce:default-endian="big" name="dotted_carrier_plain">
  <sce:import src="codec_variant_session_open.scxml" kind="codec" as="codec_variant_session_open"/>
  <datamodel>
    <data id="header" sce:type="uint8" sce:byte="0" sce:bit-size="8"/>
    <sce:variant tag="header.mid">
      <sce:arm value="0x01" type="codec_variant_session_open"/>
      <sce:default type="codec_variant_session_open"/>
    </sce:variant>
  </datamodel>
</scxml>"#;
    let result = sce_build::compile_forge_with_imports(
        scxml,
        sce_build::DocumentLabel::symmetric("dotted_carrier_plain"),
        sce_build::generator::Language::Rust,
        &resource_dir(),
        &sce_build::ForgeCompileOptions::default(),
    );
    let err = match result {
        Ok(_) => panic!(
            "dotted-form variant tag with non-flags carrier must reject with validation/invalid-attribute"
        ),
        Err(e) => e,
    };
    assert!(
        matches!(
            err.error,
            ForgeError::Validation(ValidationError::InvalidAttribute { ref attr, .. })
                if attr == "tag"
        ),
        "must surface as ValidationError::InvalidAttribute on the variant's tag attribute; got: {:?}",
        err.error
    );
}

/// RFC §5.B B5-β multi-bit-flag dispatch: the named flag must exist
/// on the carrier. Typo'd or undeclared flag names reject with
/// `validation/invalid-attribute`, naming the available flags so
/// the author sees the right repair.
#[test]
fn forge_codec_variant_dotted_tag_unknown_flag_rejects() {
    use sce_build::forge::error::{ForgeError, ValidationError};

    let scxml = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       xmlns:sce="http://sce.dev/ext"
       sce:kind="codec" sce:default-endian="big" name="dotted_unknown_flag">
  <sce:import src="codec_variant_session_open.scxml" kind="codec" as="codec_variant_session_open"/>
  <datamodel>
    <sce:flags id="header" sce:type="uint8" sce:byte="0" sce:bit-size="8">
      <sce:flag name="mid" bit="0" width="5"/>
      <sce:flag name="z"   bit="5"/>
    </sce:flags>
    <sce:variant tag="header.kind">
      <sce:arm value="0x01" type="codec_variant_session_open"/>
      <sce:default type="codec_variant_session_open"/>
    </sce:variant>
  </datamodel>
</scxml>"#;
    let result = sce_build::compile_forge_with_imports(
        scxml,
        sce_build::DocumentLabel::symmetric("dotted_unknown_flag"),
        sce_build::generator::Language::Rust,
        &resource_dir(),
        &sce_build::ForgeCompileOptions::default(),
    );
    let err = match result {
        Ok(_) => panic!(
            "dotted-form variant tag with unknown flag must reject with validation/invalid-attribute"
        ),
        Err(e) => e,
    };
    assert!(
        matches!(
            err.error,
            ForgeError::Validation(ValidationError::InvalidAttribute { ref attr, ref expected, .. })
                if attr == "tag" && expected.contains("mid") && expected.contains("z")
        ),
        "must surface as ValidationError::InvalidAttribute naming the available flags; got: {:?}",
        err.error
    );
}

/// RFC §5.B B5-β multi-bit-flag dispatch: when the named bit-range
/// has small width (e.g. width=1 → domain {0,1}), the arm-domain
/// validator computes domain = `1 << width` and the
/// `codec/variant-arm-unreachable` diagnostic still surfaces if
/// arms don't cover that domain without `<sce:default>`. This pins
/// the FlagBitRange branch of the exhaustiveness check.
#[test]
fn forge_codec_variant_dotted_tag_arm_unreachable_uses_flag_width() {
    use sce_build::forge::error::{ForgeError, ValidationError};

    let scxml = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       xmlns:sce="http://sce.dev/ext"
       sce:kind="codec" sce:default-endian="big" name="dotted_underexhaustive">
  <sce:import src="codec_variant_session_open.scxml" kind="codec" as="codec_variant_session_open"/>
  <datamodel>
    <sce:flags id="header" sce:type="uint8" sce:byte="0" sce:bit-size="8">
      <sce:flag name="kind" bit="0" width="2"/>
    </sce:flags>
    <sce:variant tag="header.kind">
      <sce:arm value="0x00" type="codec_variant_session_open"/>
      <sce:arm value="0x01" type="codec_variant_session_open"/>
    </sce:variant>
  </datamodel>
</scxml>"#;
    let result = sce_build::compile_forge_with_imports(
        scxml,
        sce_build::DocumentLabel::symmetric("dotted_underexhaustive"),
        sce_build::generator::Language::Rust,
        &resource_dir(),
        &sce_build::ForgeCompileOptions::default(),
    );
    let err = match result {
        Ok(_) => panic!("2-arm coverage of width-2 domain (4 values) without default must reject"),
        Err(e) => e,
    };
    assert!(
        matches!(
            err.error,
            ForgeError::Validation(ValidationError::CodecVariantArmUnreachable {
                ref tag_field,
                arm_count: 2,
                domain_size: Some(4),
                ..
            }) if tag_field == "header.kind"
        ),
        "must surface as ValidationError::CodecVariantArmUnreachable with width-derived domain=4 \
         and dotted tag display; got: {:?}",
        err.error
    );
}

// ══════════════════════════════════════════════════════════════
// ── Kotlin conformance tests ─────────────────────────────────
// ══════════════════════════════════════════════════════════════

// ── Transform (Kotlin) ────────────────────────────────────────

#[test]
fn forge_kotlin_transform_temperature() {
    assert_standalone_forge_kotlin("transform_temperature", "TransformTemperature.kt");
}

#[test]
fn forge_kotlin_transform_multi_output() {
    assert_standalone_forge_kotlin("transform_multi_output", "TransformMultiOutput.kt");
}

#[test]
fn forge_kotlin_transform_bitwise() {
    assert_standalone_forge_kotlin("transform_bitwise", "TransformBitwise.kt");
}

// ── Lookup (Kotlin) ──────────────────────────────────────────

#[test]
fn forge_kotlin_lookup_engine_status() {
    assert_standalone_forge_kotlin("lookup_engine_status", "LookupEngineStatus.kt");
}

#[test]
fn forge_kotlin_lookup_gear_position() {
    assert_standalone_forge_kotlin("lookup_gear_position", "LookupGearPosition.kt");
}

#[test]
fn forge_kotlin_lookup_single_default() {
    assert_standalone_forge_kotlin("lookup_single_default", "LookupSingleDefault.kt");
}

#[test]
fn forge_kotlin_lookup_alarm_code() {
    assert_standalone_forge_kotlin("lookup_alarm_code", "LookupAlarmCode.kt");
}

#[test]
fn forge_kotlin_lookup_state_action() {
    assert_standalone_forge_kotlin("lookup_state_action", "LookupStateAction.kt");
}

#[test]
fn forge_kotlin_lookup_unit_scale() {
    assert_standalone_forge_kotlin("lookup_unit_scale", "LookupUnitScale.kt");
}

#[test]
fn forge_kotlin_lookup_severity_default() {
    assert_standalone_forge_kotlin("lookup_severity_default", "LookupSeverityDefault.kt");
}

// ── Condition (Kotlin) ───────────────────────────────────────

#[test]
fn forge_kotlin_condition_programming() {
    assert_standalone_forge_kotlin("condition_programming", "ConditionProgramming.kt");
}

#[test]
fn forge_kotlin_condition_threshold() {
    assert_standalone_forge_kotlin("condition_threshold", "ConditionThreshold.kt");
}

#[test]
fn forge_kotlin_condition_range() {
    assert_standalone_forge_kotlin("condition_range", "ConditionRange.kt");
}

// ── Codec (Kotlin) ───────────────────────────────────────────

#[test]
fn forge_kotlin_codec_simple_frame() {
    assert_standalone_forge_kotlin("codec_simple_frame", "CodecSimpleFrame.kt");
}

#[test]
fn forge_kotlin_codec_little_endian() {
    assert_standalone_forge_kotlin("codec_little_endian", "CodecLittleEndian.kt");
}

#[test]
fn forge_kotlin_codec_subbyte() {
    assert_standalone_forge_kotlin("codec_subbyte", "CodecSubbyte.kt");
}

#[test]
fn forge_kotlin_codec_tail() {
    assert_standalone_forge_kotlin("codec_tail", "CodecTail.kt");
}

#[test]
fn forge_kotlin_codec_length_ref() {
    assert_standalone_forge_kotlin("codec_length_ref", "CodecLengthRef.kt");
}

#[test]
fn forge_kotlin_codec_vle_zint_u64() {
    assert_standalone_forge_kotlin("codec_vle_zint_u64", "CodecVleZintU64.kt");
}

// ── RFC §5.B B5-prep Zenoh transport-message body codecs (Kotlin) ─

#[test]
fn forge_kotlin_codec_zenoh_close() {
    assert_standalone_forge_kotlin("codec_zenoh_close", "CodecZenohClose.kt");
}

#[test]
fn forge_kotlin_codec_zenoh_frame() {
    assert_standalone_forge_kotlin("codec_zenoh_frame", "CodecZenohFrame.kt");
}

// ── RFC §5.B B5-ι cross-codec composition (Kotlin) ───────────

#[test]
fn forge_kotlin_codec_zenoh_open_body() {
    assert_standalone_forge_kotlin("codec_zenoh_open_body", "CodecZenohOpenBody.kt");
}

#[test]
fn forge_kotlin_codec_zenoh_init_body() {
    assert_standalone_forge_kotlin("codec_zenoh_init_body", "CodecZenohInitBody.kt");
}

#[test]
fn forge_kotlin_codec_zenoh_join() {
    assert_standalone_forge_kotlin("codec_zenoh_join", "CodecZenohJoin.kt");
}

#[test]
fn forge_kotlin_codec_zenoh_fragment() {
    assert_standalone_forge_kotlin("codec_zenoh_fragment", "CodecZenohFragment.kt");
}

#[test]
fn forge_kotlin_codec_zenoh_decl_final() {
    assert_standalone_forge_kotlin("codec_zenoh_decl_final", "CodecZenohDeclFinal.kt");
}

// ── RFC §5.B B5-κ Surface L dotted-path length-field (Kotlin) ──

#[test]
fn forge_kotlin_codec_length_ref_dotted_basic() {
    assert_standalone_forge_kotlin(
        "codec_length_ref_dotted_basic",
        "CodecLengthRefDottedBasic.kt",
    );
}

#[test]
fn forge_kotlin_codec_zenoh_scout() {
    assert_standalone_forge_kotlin("codec_zenoh_scout", "CodecZenohScout.kt");
}

// ── RFC §5.B B5-α multi-bit + empty-codec (Kotlin) ───────────

#[test]
fn forge_kotlin_codec_qos_byte() {
    assert_standalone_forge_kotlin("codec_qos_byte", "CodecQosByte.kt");
}

#[test]
fn forge_kotlin_codec_zenoh_keep_alive() {
    assert_standalone_forge_kotlin("codec_zenoh_keep_alive", "CodecZenohKeepAlive.kt");
}

// ── RFC §5.B B1-γ flags primitive (Kotlin) ───────────────────

#[test]
fn forge_kotlin_codec_flags_basic() {
    assert_standalone_forge_kotlin("codec_flags_basic", "CodecFlagsBasic.kt");
}

// ── RFC §5.B variant primitive (Kotlin, B1-β closure) ────────

#[test]
fn forge_kotlin_codec_variant_session_open() {
    assert_standalone_forge_kotlin("codec_variant_session_open", "CodecVariantSessionOpen.kt");
}

#[test]
fn forge_kotlin_codec_variant_session_close() {
    assert_standalone_forge_kotlin("codec_variant_session_close", "CodecVariantSessionClose.kt");
}

#[test]
fn forge_kotlin_codec_variant_dispatch() {
    assert_standalone_forge_kotlin("codec_variant_dispatch", "CodecVariantDispatch.kt");
}

#[test]
fn forge_kotlin_codec_transport_envelope() {
    assert_standalone_forge_kotlin("codec_transport_envelope", "CodecTransportEnvelope.kt");
}

// ── RFC §5.B B1-δ present-if primitive (Kotlin) ─────────────

#[test]
fn forge_kotlin_codec_present_if_basic() {
    assert_standalone_forge_kotlin("codec_present_if_basic", "CodecPresentIfBasic.kt");
}

// ── RFC §5.B B5-λ present-if negation primitive (Kotlin) ────

#[test]
fn forge_kotlin_codec_present_if_negation() {
    assert_standalone_forge_kotlin("codec_present_if_negation", "CodecPresentIfNegation.kt");
}

// ── RFC §5.B Y3 atomic 2b-ii present-if disjunction primitive (Kotlin) ──

#[test]
fn forge_kotlin_codec_present_if_disjunction() {
    assert_standalone_forge_kotlin(
        "codec_present_if_disjunction",
        "CodecPresentIfDisjunction.kt",
    );
}

// ── RFC §5.B Y3 atomic 2b-ii peek-byte peek-byte primitive (Kotlin) ──

#[test]
fn forge_kotlin_codec_peek_arm_a() {
    assert_standalone_forge_kotlin("codec_peek_arm_a", "CodecPeekArmA.kt");
}

#[test]
fn forge_kotlin_codec_peek_arm_b() {
    assert_standalone_forge_kotlin("codec_peek_arm_b", "CodecPeekArmB.kt");
}

#[test]
fn forge_kotlin_codec_variant_peek_basic() {
    assert_standalone_forge_kotlin("codec_variant_peek_basic", "CodecVariantPeekBasic.kt");
}

#[test]
fn forge_kotlin_codec_zenoh_response() {
    assert_standalone_forge_kotlin("codec_zenoh_response", "CodecZenohResponse.kt");
}

// ── RFC §5.B B2-β present-if + variable-length (Kotlin) ─────

#[test]
fn forge_kotlin_codec_present_if_tail() {
    assert_standalone_forge_kotlin("codec_present_if_tail", "CodecPresentIfTail.kt");
}

#[test]
fn forge_kotlin_codec_present_if_length_ref() {
    assert_standalone_forge_kotlin("codec_present_if_length_ref", "CodecPresentIfLengthRef.kt");
}

#[test]
fn forge_kotlin_codec_present_if_vle() {
    assert_standalone_forge_kotlin("codec_present_if_vle", "CodecPresentIfVle.kt");
}

// ── RFC §5.B B2 repeat primitive (Kotlin, closure) ──────────

#[test]
fn forge_kotlin_codec_repeat_elem() {
    assert_standalone_forge_kotlin("codec_repeat_elem", "CodecRepeatElem.kt");
}

#[test]
fn forge_kotlin_codec_repeat_basic() {
    assert_standalone_forge_kotlin("codec_repeat_basic", "CodecRepeatBasic.kt");
}

#[test]
fn forge_kotlin_codec_repeat_present_if_basic() {
    assert_standalone_forge_kotlin(
        "codec_repeat_present_if_basic",
        "CodecRepeatPresentIfBasic.kt",
    );
}

#[test]
fn forge_kotlin_codec_zenoh_hello() {
    assert_standalone_forge_kotlin("codec_zenoh_hello", "CodecZenohHello.kt");
}

// Wire RFC Phase B Y0a — see cpp registrations above for context.
#[test]
fn forge_kotlin_codec_present_if_string() {
    assert_standalone_forge_kotlin("codec_present_if_string", "CodecPresentIfString.kt");
}

#[test]
fn forge_kotlin_codec_zenoh_wireexpr() {
    assert_standalone_forge_kotlin("codec_zenoh_wireexpr", "CodecZenohWireexpr.kt");
}

#[test]
fn forge_kotlin_codec_embed_basic() {
    assert_standalone_forge_kotlin("codec_embed_basic", "CodecEmbedBasic.kt");
}

#[test]
fn forge_kotlin_codec_zenoh_decl_kexpr() {
    assert_standalone_forge_kotlin("codec_zenoh_decl_kexpr", "CodecZenohDeclKexpr.kt");
}

#[test]
fn forge_kotlin_codec_zenoh_decl_subscriber() {
    assert_standalone_forge_kotlin("codec_zenoh_decl_subscriber", "CodecZenohDeclSubscriber.kt");
}

#[test]
fn forge_kotlin_codec_zenoh_decl_queryable() {
    assert_standalone_forge_kotlin("codec_zenoh_decl_queryable", "CodecZenohDeclQueryable.kt");
}

#[test]
fn forge_kotlin_codec_zenoh_decl_token() {
    assert_standalone_forge_kotlin("codec_zenoh_decl_token", "CodecZenohDeclToken.kt");
}

#[test]
fn forge_kotlin_codec_zenoh_undecl_kexpr() {
    assert_standalone_forge_kotlin("codec_zenoh_undecl_kexpr", "CodecZenohUndeclKexpr.kt");
}

// ── RFC §5.B Wire RFC Phase B Y0b — TLV envelope foundation ────
#[test]
fn forge_kotlin_codec_zenoh_decl_ext_keyexpr_inner() {
    assert_standalone_forge_kotlin(
        "codec_zenoh_decl_ext_keyexpr_inner",
        "CodecZenohDeclExtKeyexprInner.kt",
    );
}

#[test]
fn forge_kotlin_codec_zenoh_decl_ext_keyexpr() {
    assert_standalone_forge_kotlin(
        "codec_zenoh_decl_ext_keyexpr",
        "CodecZenohDeclExtKeyexpr.kt",
    );
}

#[test]
fn forge_kotlin_codec_zenoh_undecl_subscriber() {
    assert_standalone_forge_kotlin(
        "codec_zenoh_undecl_subscriber",
        "CodecZenohUndeclSubscriber.kt",
    );
}

#[test]
fn forge_kotlin_codec_zenoh_undecl_queryable() {
    assert_standalone_forge_kotlin(
        "codec_zenoh_undecl_queryable",
        "CodecZenohUndeclQueryable.kt",
    );
}

#[test]
fn forge_kotlin_codec_zenoh_undecl_token() {
    assert_standalone_forge_kotlin("codec_zenoh_undecl_token", "CodecZenohUndeclToken.kt");
}

#[test]
fn forge_kotlin_codec_zenoh_source_info() {
    assert_standalone_forge_kotlin("codec_zenoh_source_info", "CodecZenohSourceInfo.kt");
}

#[test]
fn forge_kotlin_codec_zenoh_source_info_ext() {
    assert_standalone_forge_kotlin("codec_zenoh_source_info_ext", "CodecZenohSourceInfoExt.kt");
}

#[test]
fn forge_kotlin_codec_zenoh_timestamp_ext() {
    assert_standalone_forge_kotlin("codec_zenoh_timestamp_ext", "CodecZenohTimestampExt.kt");
}

#[test]
fn forge_kotlin_codec_until_eof_basic() {
    assert_standalone_forge_kotlin("codec_until_eof_basic", "CodecUntilEofBasic.kt");
}

// ── RFC §5.B B4 applied codec shapes (Kotlin) ───────────────

#[test]
fn forge_kotlin_codec_ext_timestamp() {
    assert_standalone_forge_kotlin("codec_ext_timestamp", "CodecExtTimestamp.kt");
}

#[test]
fn forge_kotlin_codec_ext_attachment() {
    assert_standalone_forge_kotlin("codec_ext_attachment", "CodecExtAttachment.kt");
}

#[test]
fn forge_kotlin_codec_ext_encoding_info() {
    assert_standalone_forge_kotlin("codec_ext_encoding_info", "CodecExtEncodingInfo.kt");
}

// ── Algorithm (Kotlin, RFC §5.A — post-A6 matrix follow-up) ─

/// RFC §5.B B2-test-vector Kotlin closure: the algorithm body
/// itself is byte-stable against its prior golden — the closure
/// only adds a sidecar emission, so the primary algorithm output
/// stays identical to the pre-test-vector form.
#[test]
fn forge_kotlin_algorithm_crc16() {
    assert_standalone_forge_kotlin("algorithm_crc16", "AlgorithmCrc16.kt");
}

/// RFC §5.B B2-test-vector Kotlin closure: pin the per-fixture
/// sidecar (`<Pascal>TestVectors.kt`) emitted next to the
/// algorithm `.kt`. The Kotlin/JVM test runner discovers the
/// `@Test`-annotated class via the `jvmTest` source set wired in
/// `sce-forge-runtime/kotlin/build.gradle.kts`; the sidecar
/// itself is byte-stable as the second entry of the codegen
/// output.
#[test]
fn forge_kotlin_algorithm_crc16_test_vector_sidecar() {
    assert_sidecar_forge_lang(
        "algorithm_crc16",
        "AlgorithmCrc16TestVectors.kt",
        sce_build::generator::Language::Kotlin,
    );
}

#[test]
fn forge_kotlin_algorithm_crc16_table() {
    assert_standalone_forge_kotlin("algorithm_crc16_table", "AlgorithmCrc16Table.kt");
}

#[test]
fn forge_kotlin_algorithm_const_fold_smoke() {
    assert_standalone_forge_kotlin("algorithm_const_fold_smoke", "AlgorithmConstFoldSmoke.kt");
}

// ══════════════════════════════════════════════════════════════
// ── Rust conformance tests ───────────────────────────────────
// ══════════════════════════════════════════════════════════════

// ── Algorithm (Rust, RFC §5.A) ───────────────────────────────

#[test]
fn forge_rust_algorithm_crc16() {
    assert_standalone_forge_rust("algorithm_crc16", "algorithm_crc16.rs");
}

/// RFC §5.B B2-test-vector trunk: pin the Rust sidecar emit so the
/// canonical `<sce:test-vector hex="313233343536373839" value="0x29B1"/>`
/// row round-trips through cargo test. The sidecar is the second
/// entry in the codegen output (the algorithm header stays byte-stable
/// against its existing golden).
#[test]
fn forge_rust_algorithm_crc16_test_vector_sidecar() {
    assert_sidecar_forge_lang(
        "algorithm_crc16",
        "algorithm_crc16_test.rs",
        sce_build::generator::Language::Rust,
    );
}

/// RFC §5.B B2-test-vector trunk: pin the C11 sidecar emit. Mirrors
/// the Rust sidecar test above; the lone test-vector row aggregates
/// into a `static inline int test_vector_algorithm_crc16(void)`
/// helper that the conformance harness folds into `g_failures`.
#[test]
fn forge_c11_algorithm_crc16_test_vector_sidecar() {
    assert_sidecar_forge_lang(
        "algorithm_crc16",
        "algorithm_crc16_test.c.h",
        sce_build::generator::Language::C11,
    );
}

/// RFC §5.B B2-test-vector v1 enforces a single `bytes` parameter on
/// the algorithm signature so the hex bytes lower unambiguously. A
/// non-bytes parameter shape rejects with the typed
/// `generate/unsupported-feature` until B5 widens the binding rules.
#[test]
fn forge_test_vector_non_bytes_signature_rejects() {
    use sce_build::forge::error::{ForgeError, GenerateError};
    let scxml = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       xmlns:sce="http://sce.dev/ext"
       sce:kind="algorithm" version="1.0">
  <sce:signature>
    <sce:param name="value" type="uint32"/>
    <sce:return type="uint16"/>
  </sce:signature>
  <sce:body>
    <sce:return expr="0"/>
  </sce:body>
  <sce:test-vector hex="00" value="0x0000"/>
</scxml>"#;
    let result = sce_build::compile_forge_with_imports(
        scxml,
        sce_build::DocumentLabel::symmetric("non_bytes_tv"),
        sce_build::generator::Language::Rust,
        &resource_dir(),
        &sce_build::ForgeCompileOptions::default(),
    );
    let err = match result {
        Ok(_) => panic!("non-bytes signature with <sce:test-vector> must reject"),
        Err(e) => e,
    };
    let inner = err.error;
    assert!(
        matches!(
            inner,
            ForgeError::Generate(GenerateError::UnsupportedFeature(ref msg))
                if msg.contains("non_bytes_tv") && msg.contains("bytes")
        ),
        "must surface as GenerateError::UnsupportedFeature explaining the signature constraint; got: {inner:?}"
    );
}

// ── Transform (Rust) ─────────────────────────────────────────

#[test]
fn forge_rust_transform_temperature() {
    assert_standalone_forge_rust("transform_temperature", "transform_temperature.rs");
}

#[test]
fn forge_rust_transform_multi_output() {
    assert_standalone_forge_rust("transform_multi_output", "transform_multi_output.rs");
}

#[test]
fn forge_rust_transform_bitwise() {
    assert_standalone_forge_rust("transform_bitwise", "transform_bitwise.rs");
}

// ── Lookup (Rust) ────────────────────────────────────────────

#[test]
fn forge_rust_lookup_engine_status() {
    assert_standalone_forge_rust("lookup_engine_status", "lookup_engine_status.rs");
}

#[test]
fn forge_rust_lookup_gear_position() {
    assert_standalone_forge_rust("lookup_gear_position", "lookup_gear_position.rs");
}

#[test]
fn forge_rust_lookup_single_default() {
    assert_standalone_forge_rust("lookup_single_default", "lookup_single_default.rs");
}

#[test]
fn forge_rust_lookup_alarm_code() {
    assert_standalone_forge_rust("lookup_alarm_code", "lookup_alarm_code.rs");
}

#[test]
fn forge_rust_lookup_state_action() {
    assert_standalone_forge_rust("lookup_state_action", "lookup_state_action.rs");
}

#[test]
fn forge_rust_lookup_unit_scale() {
    assert_standalone_forge_rust("lookup_unit_scale", "lookup_unit_scale.rs");
}

#[test]
fn forge_rust_lookup_severity_default() {
    assert_standalone_forge_rust("lookup_severity_default", "lookup_severity_default.rs");
}

// ── Condition (Rust) ─────────────────────────────────────────

#[test]
fn forge_rust_condition_programming() {
    assert_standalone_forge_rust("condition_programming", "condition_programming.rs");
}

#[test]
fn forge_rust_condition_threshold() {
    assert_standalone_forge_rust("condition_threshold", "condition_threshold.rs");
}

#[test]
fn forge_rust_condition_range() {
    assert_standalone_forge_rust("condition_range", "condition_range.rs");
}

// ── Codec (Rust) ─────────────────────────────────────────────

#[test]
fn forge_rust_codec_simple_frame() {
    assert_standalone_forge_rust("codec_simple_frame", "codec_simple_frame.rs");
}

#[test]
fn forge_rust_codec_little_endian() {
    assert_standalone_forge_rust("codec_little_endian", "codec_little_endian.rs");
}

#[test]
fn forge_rust_codec_subbyte() {
    assert_standalone_forge_rust("codec_subbyte", "codec_subbyte.rs");
}

#[test]
fn forge_rust_codec_tail() {
    assert_standalone_forge_rust("codec_tail", "codec_tail.rs");
}

#[test]
fn forge_rust_codec_length_ref() {
    assert_standalone_forge_rust("codec_length_ref", "codec_length_ref.rs");
}

// ── RFC §5.B B5-prep Zenoh transport-message body codecs (Rust) ──

#[test]
fn forge_rust_codec_zenoh_close() {
    assert_standalone_forge_rust("codec_zenoh_close", "codec_zenoh_close.rs");
}

#[test]
fn forge_rust_codec_zenoh_frame() {
    assert_standalone_forge_rust("codec_zenoh_frame", "codec_zenoh_frame.rs");
}

// ── RFC §5.B B5-ι cross-codec composition (Rust) ─────────────

#[test]
fn forge_rust_codec_zenoh_open_body() {
    assert_standalone_forge_rust("codec_zenoh_open_body", "codec_zenoh_open_body.rs");
}

#[test]
fn forge_rust_codec_zenoh_init_body() {
    assert_standalone_forge_rust("codec_zenoh_init_body", "codec_zenoh_init_body.rs");
}

#[test]
fn forge_rust_codec_zenoh_join() {
    assert_standalone_forge_rust("codec_zenoh_join", "codec_zenoh_join.rs");
}

#[test]
fn forge_rust_codec_zenoh_fragment() {
    assert_standalone_forge_rust("codec_zenoh_fragment", "codec_zenoh_fragment.rs");
}

#[test]
fn forge_rust_codec_zenoh_decl_final() {
    assert_standalone_forge_rust("codec_zenoh_decl_final", "codec_zenoh_decl_final.rs");
}

// ── RFC §5.B B5-κ Surface L dotted-path length-field (Rust) ────

#[test]
fn forge_rust_codec_length_ref_dotted_basic() {
    assert_standalone_forge_rust(
        "codec_length_ref_dotted_basic",
        "codec_length_ref_dotted_basic.rs",
    );
}

#[test]
fn forge_rust_codec_zenoh_scout() {
    assert_standalone_forge_rust("codec_zenoh_scout", "codec_zenoh_scout.rs");
}

// ── RFC §5.B B5-α multi-bit + empty-codec (Rust) ─────────────

#[test]
fn forge_rust_codec_qos_byte() {
    assert_standalone_forge_rust("codec_qos_byte", "codec_qos_byte.rs");
}

#[test]
fn forge_rust_codec_zenoh_keep_alive() {
    assert_standalone_forge_rust("codec_zenoh_keep_alive", "codec_zenoh_keep_alive.rs");
}

// ── RFC §5.B B1-γ flags primitive (Rust) ─────────────────────

#[test]
fn forge_rust_codec_flags_basic() {
    assert_standalone_forge_rust("codec_flags_basic", "codec_flags_basic.rs");
}

// ── RFC §5.B B1-δ present-if primitive (Rust) ───────────────

#[test]
fn forge_rust_codec_present_if_basic() {
    assert_standalone_forge_rust("codec_present_if_basic", "codec_present_if_basic.rs");
}

// ── RFC §5.B B5-λ present-if negation primitive (Rust) ──────

#[test]
fn forge_rust_codec_present_if_negation() {
    assert_standalone_forge_rust("codec_present_if_negation", "codec_present_if_negation.rs");
}

// ── RFC §5.B Y3 atomic 2b-ii present-if disjunction primitive (Rust) ──

#[test]
fn forge_rust_codec_present_if_disjunction() {
    assert_standalone_forge_rust(
        "codec_present_if_disjunction",
        "codec_present_if_disjunction.rs",
    );
}

// ── RFC §5.B Y3 atomic 2b-ii peek-byte peek-byte primitive (Rust) ──

#[test]
fn forge_rust_codec_peek_arm_a() {
    assert_standalone_forge_rust("codec_peek_arm_a", "codec_peek_arm_a.rs");
}

#[test]
fn forge_rust_codec_peek_arm_b() {
    assert_standalone_forge_rust("codec_peek_arm_b", "codec_peek_arm_b.rs");
}

#[test]
fn forge_rust_codec_variant_peek_basic() {
    assert_standalone_forge_rust("codec_variant_peek_basic", "codec_variant_peek_basic.rs");
}

#[test]
fn forge_rust_codec_zenoh_response() {
    assert_standalone_forge_rust("codec_zenoh_response", "codec_zenoh_response.rs");
}

// ── RFC §5.B B2-β present-if + variable-length (Rust) ───────

#[test]
fn forge_rust_codec_present_if_tail() {
    assert_standalone_forge_rust("codec_present_if_tail", "codec_present_if_tail.rs");
}

#[test]
fn forge_rust_codec_present_if_length_ref() {
    assert_standalone_forge_rust(
        "codec_present_if_length_ref",
        "codec_present_if_length_ref.rs",
    );
}

#[test]
fn forge_rust_codec_present_if_vle() {
    assert_standalone_forge_rust("codec_present_if_vle", "codec_present_if_vle.rs");
}

// ── RFC §5.B B2 repeat primitive (Rust, trunk) ──────────────

#[test]
fn forge_rust_codec_repeat_elem() {
    assert_standalone_forge_rust("codec_repeat_elem", "codec_repeat_elem.rs");
}

#[test]
fn forge_rust_codec_repeat_basic() {
    assert_standalone_forge_rust("codec_repeat_basic", "codec_repeat_basic.rs");
}

#[test]
fn forge_rust_codec_until_eof_basic() {
    assert_standalone_forge_rust("codec_until_eof_basic", "codec_until_eof_basic.rs");
}

#[test]
fn forge_rust_codec_repeat_present_if_basic() {
    assert_standalone_forge_rust(
        "codec_repeat_present_if_basic",
        "codec_repeat_present_if_basic.rs",
    );
}

#[test]
fn forge_rust_codec_zenoh_hello() {
    assert_standalone_forge_rust("codec_zenoh_hello", "codec_zenoh_hello.rs");
}

// Wire RFC Phase B Y0a — see cpp registrations above for context.
#[test]
fn forge_rust_codec_present_if_string() {
    assert_standalone_forge_rust("codec_present_if_string", "codec_present_if_string.rs");
}

#[test]
fn forge_rust_codec_zenoh_wireexpr() {
    assert_standalone_forge_rust("codec_zenoh_wireexpr", "codec_zenoh_wireexpr.rs");
}

#[test]
fn forge_rust_codec_embed_basic() {
    assert_standalone_forge_rust("codec_embed_basic", "codec_embed_basic.rs");
}

#[test]
fn forge_rust_codec_zenoh_decl_kexpr() {
    assert_standalone_forge_rust("codec_zenoh_decl_kexpr", "codec_zenoh_decl_kexpr.rs");
}

#[test]
fn forge_rust_codec_zenoh_decl_subscriber() {
    assert_standalone_forge_rust(
        "codec_zenoh_decl_subscriber",
        "codec_zenoh_decl_subscriber.rs",
    );
}

#[test]
fn forge_rust_codec_zenoh_decl_queryable() {
    assert_standalone_forge_rust(
        "codec_zenoh_decl_queryable",
        "codec_zenoh_decl_queryable.rs",
    );
}

#[test]
fn forge_rust_codec_zenoh_decl_token() {
    assert_standalone_forge_rust("codec_zenoh_decl_token", "codec_zenoh_decl_token.rs");
}

#[test]
fn forge_rust_codec_zenoh_undecl_kexpr() {
    assert_standalone_forge_rust(
        "codec_zenoh_undecl_kexpr",
        "codec_zenoh_undecl_kexpr.rs",
    );
}

// ── RFC §5.B Wire RFC Phase B Y0b — TLV envelope foundation ────
#[test]
fn forge_rust_codec_zenoh_decl_ext_keyexpr_inner() {
    assert_standalone_forge_rust(
        "codec_zenoh_decl_ext_keyexpr_inner",
        "codec_zenoh_decl_ext_keyexpr_inner.rs",
    );
}

#[test]
fn forge_rust_codec_zenoh_decl_ext_keyexpr() {
    assert_standalone_forge_rust(
        "codec_zenoh_decl_ext_keyexpr",
        "codec_zenoh_decl_ext_keyexpr.rs",
    );
}

#[test]
fn forge_rust_codec_zenoh_undecl_subscriber() {
    assert_standalone_forge_rust(
        "codec_zenoh_undecl_subscriber",
        "codec_zenoh_undecl_subscriber.rs",
    );
}

#[test]
fn forge_rust_codec_zenoh_undecl_queryable() {
    assert_standalone_forge_rust(
        "codec_zenoh_undecl_queryable",
        "codec_zenoh_undecl_queryable.rs",
    );
}

#[test]
fn forge_rust_codec_zenoh_undecl_token() {
    assert_standalone_forge_rust("codec_zenoh_undecl_token", "codec_zenoh_undecl_token.rs");
}

#[test]
fn forge_rust_codec_zenoh_source_info() {
    assert_standalone_forge_rust("codec_zenoh_source_info", "codec_zenoh_source_info.rs");
}

#[test]
fn forge_rust_codec_zenoh_source_info_ext() {
    assert_standalone_forge_rust(
        "codec_zenoh_source_info_ext",
        "codec_zenoh_source_info_ext.rs",
    );
}

#[test]
fn forge_rust_codec_zenoh_timestamp_ext() {
    assert_standalone_forge_rust("codec_zenoh_timestamp_ext", "codec_zenoh_timestamp_ext.rs");
}

// ── RFC §5.B B4 applied codec shapes (Rust) ─────────────────

#[test]
fn forge_rust_codec_ext_timestamp() {
    assert_standalone_forge_rust("codec_ext_timestamp", "codec_ext_timestamp.rs");
}

#[test]
fn forge_rust_codec_ext_attachment() {
    assert_standalone_forge_rust("codec_ext_attachment", "codec_ext_attachment.rs");
}

#[test]
fn forge_rust_codec_ext_encoding_info() {
    assert_standalone_forge_rust("codec_ext_encoding_info", "codec_ext_encoding_info.rs");
}

// ── RFC §5.B variant primitive (Rust, B1-β trunk) ────────────

#[test]
fn forge_rust_codec_variant_session_open() {
    assert_standalone_forge_rust(
        "codec_variant_session_open",
        "codec_variant_session_open.rs",
    );
}

#[test]
fn forge_rust_codec_variant_session_close() {
    assert_standalone_forge_rust(
        "codec_variant_session_close",
        "codec_variant_session_close.rs",
    );
}

#[test]
fn forge_rust_codec_variant_dispatch() {
    assert_standalone_forge_rust("codec_variant_dispatch", "codec_variant_dispatch.rs");
}

#[test]
fn forge_rust_codec_transport_envelope() {
    assert_standalone_forge_rust("codec_transport_envelope", "codec_transport_envelope.rs");
}

/// RFC §5.B B5-γ trunk (Rust): body codec with parent-flags dependency.
/// Decode/encode signatures gain a `parent_flags: u8` parameter; body
/// fields gated via `parent.<flag>` predicates emit
/// `(parent_flags & 0x40) != 0` style bit-tests. Mirrors
/// `_z_init_decode` upstream signature shape.
#[test]
fn forge_rust_codec_init_syn_body() {
    assert_standalone_forge_rust("codec_init_syn_body", "codec_init_syn_body.rs");
}

/// RFC §5.B B5-γ trunk (Rust): variant parent threading carrier value.
/// Each arm dispatches `Body::decode(cursor, header)` for arm bodies
/// declaring `<sce:requires-parent-flags carrier="header">`; encode
/// passes `body.encode(header)` symmetrically. Cross-codec validator
/// confirms parent's flag layout matches the body's declared
/// `<sce:flag name="S" bit="6"/>`.
#[test]
fn forge_rust_codec_init_syn_envelope() {
    assert_standalone_forge_rust("codec_init_syn_envelope", "codec_init_syn_envelope.rs");
}

/// RFC §5.B B5-δ Surfaces D + E (Rust): Init body cookie codec.
/// `cookie_size` is `Option<u16>` (gated VLE u16); `cookie` is
/// `Option<Vec<u8>>` (gated length-ref bytes). Helper emits
/// `cookie_size.unwrap() as usize` inside the predicate's true-branch.
#[test]
fn forge_rust_codec_init_cookie_body() {
    assert_standalone_forge_rust("codec_init_cookie_body", "codec_init_cookie_body.rs");
}

/// RFC §5.B B5-δ Surface F (Rust): Scout/Hello/Init zid codec.
/// `length-arith="+1"` lifts the byte count by one above the sibling's
/// value: helper emits `(zid_len_m1 as usize).wrapping_add(1)` for
/// decode; encode trusts `zid.len()` as the source of truth (author
/// contract: `zid_len_m1 == zid.len() - 1`).
#[test]
fn forge_rust_codec_scout_zid_body() {
    assert_standalone_forge_rust("codec_scout_zid_body", "codec_scout_zid_body.rs");
}

// ══════════════════════════════════════════════════════════════
// ── Go conformance tests ─────────────────────────────────────
// ══════════════════════════════════════════════════════════════

/// Generate Go from a standalone forge SCXML and compare against expected output.
fn assert_standalone_forge_go(scxml_name: &str, expected_filename: &str) {
    assert_standalone_forge_lang(
        scxml_name,
        expected_filename,
        sce_build::generator::Language::Go,
    );
}

// ── Transform (Go) ──────────────────────────────────────────

#[test]
fn forge_go_transform_temperature() {
    assert_standalone_forge_go("transform_temperature", "transform_temperature.go");
}

#[test]
fn forge_go_transform_multi_output() {
    assert_standalone_forge_go("transform_multi_output", "transform_multi_output.go");
}

#[test]
fn forge_go_transform_bitwise() {
    assert_standalone_forge_go("transform_bitwise", "transform_bitwise.go");
}

// ── Lookup (Go) ─────────────────────────────────────────────

#[test]
fn forge_go_lookup_engine_status() {
    assert_standalone_forge_go("lookup_engine_status", "lookup_engine_status.go");
}

#[test]
fn forge_go_lookup_gear_position() {
    assert_standalone_forge_go("lookup_gear_position", "lookup_gear_position.go");
}

#[test]
fn forge_go_lookup_single_default() {
    assert_standalone_forge_go("lookup_single_default", "lookup_single_default.go");
}

#[test]
fn forge_go_lookup_alarm_code() {
    assert_standalone_forge_go("lookup_alarm_code", "lookup_alarm_code.go");
}

#[test]
fn forge_go_lookup_state_action() {
    assert_standalone_forge_go("lookup_state_action", "lookup_state_action.go");
}

#[test]
fn forge_go_lookup_unit_scale() {
    assert_standalone_forge_go("lookup_unit_scale", "lookup_unit_scale.go");
}

#[test]
fn forge_go_lookup_severity_default() {
    assert_standalone_forge_go("lookup_severity_default", "lookup_severity_default.go");
}

// ── Condition (Go) ──────────────────────────────────────────

#[test]
fn forge_go_condition_programming() {
    assert_standalone_forge_go("condition_programming", "condition_programming.go");
}

#[test]
fn forge_go_condition_threshold() {
    assert_standalone_forge_go("condition_threshold", "condition_threshold.go");
}

#[test]
fn forge_go_condition_range() {
    assert_standalone_forge_go("condition_range", "condition_range.go");
}

// ── Codec (Go) ──────────────────────────────────────────────

#[test]
fn forge_go_codec_simple_frame() {
    assert_standalone_forge_go("codec_simple_frame", "codec_simple_frame.go");
}

#[test]
fn forge_go_codec_little_endian() {
    assert_standalone_forge_go("codec_little_endian", "codec_little_endian.go");
}

#[test]
fn forge_go_codec_subbyte() {
    assert_standalone_forge_go("codec_subbyte", "codec_subbyte.go");
}

#[test]
fn forge_go_codec_tail() {
    assert_standalone_forge_go("codec_tail", "codec_tail.go");
}

#[test]
fn forge_go_codec_length_ref() {
    assert_standalone_forge_go("codec_length_ref", "codec_length_ref.go");
}

#[test]
fn forge_go_codec_vle_zint_u64() {
    assert_standalone_forge_go("codec_vle_zint_u64", "codec_vle_zint_u64.go");
}

// ── RFC §5.B B5-prep Zenoh transport-message body codecs (Go) ────

#[test]
fn forge_go_codec_zenoh_close() {
    assert_standalone_forge_go("codec_zenoh_close", "codec_zenoh_close.go");
}

#[test]
fn forge_go_codec_zenoh_frame() {
    assert_standalone_forge_go("codec_zenoh_frame", "codec_zenoh_frame.go");
}

// ── RFC §5.B B5-ι cross-codec composition (Go) ───────────────

#[test]
fn forge_go_codec_zenoh_open_body() {
    assert_standalone_forge_go("codec_zenoh_open_body", "codec_zenoh_open_body.go");
}

#[test]
fn forge_go_codec_zenoh_init_body() {
    assert_standalone_forge_go("codec_zenoh_init_body", "codec_zenoh_init_body.go");
}

#[test]
fn forge_go_codec_zenoh_join() {
    assert_standalone_forge_go("codec_zenoh_join", "codec_zenoh_join.go");
}

#[test]
fn forge_go_codec_zenoh_fragment() {
    assert_standalone_forge_go("codec_zenoh_fragment", "codec_zenoh_fragment.go");
}

#[test]
fn forge_go_codec_zenoh_decl_final() {
    assert_standalone_forge_go("codec_zenoh_decl_final", "codec_zenoh_decl_final.go");
}

// ── RFC §5.B B5-κ Surface L dotted-path length-field (Go) ──────

#[test]
fn forge_go_codec_length_ref_dotted_basic() {
    assert_standalone_forge_go(
        "codec_length_ref_dotted_basic",
        "codec_length_ref_dotted_basic.go",
    );
}

#[test]
fn forge_go_codec_zenoh_scout() {
    assert_standalone_forge_go("codec_zenoh_scout", "codec_zenoh_scout.go");
}

// ── RFC §5.B B5-α multi-bit + empty-codec (Go) ───────────────

#[test]
fn forge_go_codec_qos_byte() {
    assert_standalone_forge_go("codec_qos_byte", "codec_qos_byte.go");
}

#[test]
fn forge_go_codec_zenoh_keep_alive() {
    assert_standalone_forge_go("codec_zenoh_keep_alive", "codec_zenoh_keep_alive.go");
}

// ── RFC §5.B B1-γ flags primitive (Go) ───────────────────────

#[test]
fn forge_go_codec_flags_basic() {
    assert_standalone_forge_go("codec_flags_basic", "codec_flags_basic.go");
}

// ── RFC §5.B variant primitive (Go, B1-β closure) ────────────

#[test]
fn forge_go_codec_variant_session_open() {
    assert_standalone_forge_go(
        "codec_variant_session_open",
        "codec_variant_session_open.go",
    );
}

#[test]
fn forge_go_codec_variant_session_close() {
    assert_standalone_forge_go(
        "codec_variant_session_close",
        "codec_variant_session_close.go",
    );
}

#[test]
fn forge_go_codec_variant_dispatch() {
    assert_standalone_forge_go("codec_variant_dispatch", "codec_variant_dispatch.go");
}

#[test]
fn forge_go_codec_transport_envelope() {
    assert_standalone_forge_go("codec_transport_envelope", "codec_transport_envelope.go");
}

// ── RFC §5.B B1-δ present-if primitive (Go) ─────────────────

#[test]
fn forge_go_codec_present_if_basic() {
    assert_standalone_forge_go("codec_present_if_basic", "codec_present_if_basic.go");
}

// ── RFC §5.B B5-λ present-if negation primitive (Go) ────────

#[test]
fn forge_go_codec_present_if_negation() {
    assert_standalone_forge_go("codec_present_if_negation", "codec_present_if_negation.go");
}

// ── RFC §5.B Y3 atomic 2b-ii present-if disjunction primitive (Go) ──

#[test]
fn forge_go_codec_present_if_disjunction() {
    assert_standalone_forge_go(
        "codec_present_if_disjunction",
        "codec_present_if_disjunction.go",
    );
}

// ── RFC §5.B Y3 atomic 2b-ii peek-byte peek-byte primitive (Go) ──

#[test]
fn forge_go_codec_peek_arm_a() {
    assert_standalone_forge_go("codec_peek_arm_a", "codec_peek_arm_a.go");
}

#[test]
fn forge_go_codec_peek_arm_b() {
    assert_standalone_forge_go("codec_peek_arm_b", "codec_peek_arm_b.go");
}

#[test]
fn forge_go_codec_variant_peek_basic() {
    assert_standalone_forge_go("codec_variant_peek_basic", "codec_variant_peek_basic.go");
}

#[test]
fn forge_go_codec_zenoh_response() {
    assert_standalone_forge_go("codec_zenoh_response", "codec_zenoh_response.go");
}

// ── RFC §5.B B2-β present-if + variable-length (Go) ─────────

#[test]
fn forge_go_codec_present_if_tail() {
    assert_standalone_forge_go("codec_present_if_tail", "codec_present_if_tail.go");
}

#[test]
fn forge_go_codec_present_if_length_ref() {
    assert_standalone_forge_go(
        "codec_present_if_length_ref",
        "codec_present_if_length_ref.go",
    );
}

#[test]
fn forge_go_codec_present_if_vle() {
    assert_standalone_forge_go("codec_present_if_vle", "codec_present_if_vle.go");
}

// ── RFC §5.B B2 repeat primitive (Go, closure) ──────────────

#[test]
fn forge_go_codec_repeat_elem() {
    assert_standalone_forge_go("codec_repeat_elem", "codec_repeat_elem.go");
}

#[test]
fn forge_go_codec_repeat_basic() {
    assert_standalone_forge_go("codec_repeat_basic", "codec_repeat_basic.go");
}

#[test]
fn forge_go_codec_until_eof_basic() {
    assert_standalone_forge_go("codec_until_eof_basic", "codec_until_eof_basic.go");
}

#[test]
fn forge_go_codec_repeat_present_if_basic() {
    assert_standalone_forge_go(
        "codec_repeat_present_if_basic",
        "codec_repeat_present_if_basic.go",
    );
}

#[test]
fn forge_go_codec_zenoh_hello() {
    assert_standalone_forge_go("codec_zenoh_hello", "codec_zenoh_hello.go");
}

// Wire RFC Phase B Y0a — see cpp registrations above for context.
#[test]
fn forge_go_codec_present_if_string() {
    assert_standalone_forge_go("codec_present_if_string", "codec_present_if_string.go");
}

#[test]
fn forge_go_codec_zenoh_wireexpr() {
    assert_standalone_forge_go("codec_zenoh_wireexpr", "codec_zenoh_wireexpr.go");
}

#[test]
fn forge_go_codec_embed_basic() {
    assert_standalone_forge_go("codec_embed_basic", "codec_embed_basic.go");
}

#[test]
fn forge_go_codec_zenoh_decl_kexpr() {
    assert_standalone_forge_go("codec_zenoh_decl_kexpr", "codec_zenoh_decl_kexpr.go");
}

#[test]
fn forge_go_codec_zenoh_decl_subscriber() {
    assert_standalone_forge_go(
        "codec_zenoh_decl_subscriber",
        "codec_zenoh_decl_subscriber.go",
    );
}

#[test]
fn forge_go_codec_zenoh_decl_queryable() {
    assert_standalone_forge_go(
        "codec_zenoh_decl_queryable",
        "codec_zenoh_decl_queryable.go",
    );
}

#[test]
fn forge_go_codec_zenoh_decl_token() {
    assert_standalone_forge_go("codec_zenoh_decl_token", "codec_zenoh_decl_token.go");
}

#[test]
fn forge_go_codec_zenoh_undecl_kexpr() {
    assert_standalone_forge_go(
        "codec_zenoh_undecl_kexpr",
        "codec_zenoh_undecl_kexpr.go",
    );
}

// ── RFC §5.B Wire RFC Phase B Y0b — TLV envelope foundation ────
#[test]
fn forge_go_codec_zenoh_decl_ext_keyexpr_inner() {
    assert_standalone_forge_go(
        "codec_zenoh_decl_ext_keyexpr_inner",
        "codec_zenoh_decl_ext_keyexpr_inner.go",
    );
}

#[test]
fn forge_go_codec_zenoh_decl_ext_keyexpr() {
    assert_standalone_forge_go(
        "codec_zenoh_decl_ext_keyexpr",
        "codec_zenoh_decl_ext_keyexpr.go",
    );
}

#[test]
fn forge_go_codec_zenoh_undecl_subscriber() {
    assert_standalone_forge_go(
        "codec_zenoh_undecl_subscriber",
        "codec_zenoh_undecl_subscriber.go",
    );
}

#[test]
fn forge_go_codec_zenoh_undecl_queryable() {
    assert_standalone_forge_go(
        "codec_zenoh_undecl_queryable",
        "codec_zenoh_undecl_queryable.go",
    );
}

#[test]
fn forge_go_codec_zenoh_undecl_token() {
    assert_standalone_forge_go("codec_zenoh_undecl_token", "codec_zenoh_undecl_token.go");
}

#[test]
fn forge_go_codec_zenoh_source_info() {
    assert_standalone_forge_go("codec_zenoh_source_info", "codec_zenoh_source_info.go");
}

#[test]
fn forge_go_codec_zenoh_source_info_ext() {
    assert_standalone_forge_go(
        "codec_zenoh_source_info_ext",
        "codec_zenoh_source_info_ext.go",
    );
}

#[test]
fn forge_go_codec_zenoh_timestamp_ext() {
    assert_standalone_forge_go("codec_zenoh_timestamp_ext", "codec_zenoh_timestamp_ext.go");
}

// ── RFC §5.B B4 applied codec shapes (Go) ───────────────────

#[test]
fn forge_go_codec_ext_timestamp() {
    assert_standalone_forge_go("codec_ext_timestamp", "codec_ext_timestamp.go");
}

#[test]
fn forge_go_codec_ext_attachment() {
    assert_standalone_forge_go("codec_ext_attachment", "codec_ext_attachment.go");
}

#[test]
fn forge_go_codec_ext_encoding_info() {
    assert_standalone_forge_go("codec_ext_encoding_info", "codec_ext_encoding_info.go");
}

// ── Algorithm (Go, RFC §5.A — post-A6 matrix follow-up) ────

/// RFC §5.B B2-test-vector Go closure: the algorithm body itself
/// stays byte-stable against its prior golden — the closure only
/// adds a sidecar emission, so the primary algorithm output stays
/// identical to the pre-test-vector form.
#[test]
fn forge_go_algorithm_crc16() {
    assert_standalone_forge_go("algorithm_crc16", "algorithm_crc16.go");
}

/// RFC §5.B B2-test-vector Go closure: pin the per-fixture sidecar
/// (`<snake>_test.go`) emitted next to the algorithm `.go` in the
/// per-fixture package directory. Go's per-directory test
/// discovery picks up `*_test.go` automatically; the existing
/// recursive `go test ./conformance/...` pattern runs the per-
/// fixture package tests without any harness scaffolding edits.
#[test]
fn forge_go_algorithm_crc16_test_vector_sidecar() {
    assert_sidecar_forge_lang(
        "algorithm_crc16",
        "algorithm_crc16_test.go",
        sce_build::generator::Language::Go,
    );
}

#[test]
fn forge_go_algorithm_crc16_table() {
    assert_standalone_forge_go("algorithm_crc16_table", "algorithm_crc16_table.go");
}

#[test]
fn forge_go_algorithm_const_fold_smoke() {
    assert_standalone_forge_go(
        "algorithm_const_fold_smoke",
        "algorithm_const_fold_smoke.go",
    );
}

// ══════════════════════════════════════════════════════════════
// ── Python conformance tests ─────────────────────────────────
// ══════════════════════════════════════════════════════════════

/// Generate Python from a standalone forge SCXML and compare against expected output.
fn assert_standalone_forge_python(scxml_name: &str, expected_filename: &str) {
    assert_standalone_forge_lang(
        scxml_name,
        expected_filename,
        sce_build::generator::Language::Python,
    );
}

// ── Transform (Python) ──────────────────────────────────────

#[test]
fn forge_python_transform_temperature() {
    assert_standalone_forge_python("transform_temperature", "transform_temperature.py");
}

#[test]
fn forge_python_transform_multi_output() {
    assert_standalone_forge_python("transform_multi_output", "transform_multi_output.py");
}

#[test]
fn forge_python_transform_bitwise() {
    assert_standalone_forge_python("transform_bitwise", "transform_bitwise.py");
}

// ── Lookup (Python) ─────────────────────────────────────────

#[test]
fn forge_python_lookup_engine_status() {
    assert_standalone_forge_python("lookup_engine_status", "lookup_engine_status.py");
}

#[test]
fn forge_python_lookup_gear_position() {
    assert_standalone_forge_python("lookup_gear_position", "lookup_gear_position.py");
}

#[test]
fn forge_python_lookup_single_default() {
    assert_standalone_forge_python("lookup_single_default", "lookup_single_default.py");
}

#[test]
fn forge_python_lookup_alarm_code() {
    assert_standalone_forge_python("lookup_alarm_code", "lookup_alarm_code.py");
}

#[test]
fn forge_python_lookup_state_action() {
    assert_standalone_forge_python("lookup_state_action", "lookup_state_action.py");
}

#[test]
fn forge_python_lookup_unit_scale() {
    assert_standalone_forge_python("lookup_unit_scale", "lookup_unit_scale.py");
}

#[test]
fn forge_python_lookup_severity_default() {
    assert_standalone_forge_python("lookup_severity_default", "lookup_severity_default.py");
}

// ── Condition (Python) ──────────────────────────────────────

#[test]
fn forge_python_condition_programming() {
    assert_standalone_forge_python("condition_programming", "condition_programming.py");
}

#[test]
fn forge_python_condition_threshold() {
    assert_standalone_forge_python("condition_threshold", "condition_threshold.py");
}

#[test]
fn forge_python_condition_range() {
    assert_standalone_forge_python("condition_range", "condition_range.py");
}

// ── Codec (Python) ──────────────────────────────────────────

#[test]
fn forge_python_codec_simple_frame() {
    assert_standalone_forge_python("codec_simple_frame", "codec_simple_frame.py");
}

#[test]
fn forge_python_codec_little_endian() {
    assert_standalone_forge_python("codec_little_endian", "codec_little_endian.py");
}

#[test]
fn forge_python_codec_subbyte() {
    assert_standalone_forge_python("codec_subbyte", "codec_subbyte.py");
}

#[test]
fn forge_python_codec_tail() {
    assert_standalone_forge_python("codec_tail", "codec_tail.py");
}

#[test]
fn forge_python_codec_length_ref() {
    assert_standalone_forge_python("codec_length_ref", "codec_length_ref.py");
}

#[test]
fn forge_python_codec_vle_zint_u64() {
    assert_standalone_forge_python("codec_vle_zint_u64", "codec_vle_zint_u64.py");
}

// ── RFC §5.B B5-prep Zenoh transport-message body codecs (Python) ─

#[test]
fn forge_python_codec_zenoh_close() {
    assert_standalone_forge_python("codec_zenoh_close", "codec_zenoh_close.py");
}

#[test]
fn forge_python_codec_zenoh_frame() {
    assert_standalone_forge_python("codec_zenoh_frame", "codec_zenoh_frame.py");
}

// ── RFC §5.B B5-ι cross-codec composition (Python) ───────────

#[test]
fn forge_python_codec_zenoh_open_body() {
    assert_standalone_forge_python("codec_zenoh_open_body", "codec_zenoh_open_body.py");
}

#[test]
fn forge_python_codec_zenoh_init_body() {
    assert_standalone_forge_python("codec_zenoh_init_body", "codec_zenoh_init_body.py");
}

#[test]
fn forge_python_codec_zenoh_join() {
    assert_standalone_forge_python("codec_zenoh_join", "codec_zenoh_join.py");
}

#[test]
fn forge_python_codec_zenoh_fragment() {
    assert_standalone_forge_python("codec_zenoh_fragment", "codec_zenoh_fragment.py");
}

#[test]
fn forge_python_codec_zenoh_decl_final() {
    assert_standalone_forge_python("codec_zenoh_decl_final", "codec_zenoh_decl_final.py");
}

// ── RFC §5.B B5-κ Surface L dotted-path length-field (Python) ──

#[test]
fn forge_python_codec_length_ref_dotted_basic() {
    assert_standalone_forge_python(
        "codec_length_ref_dotted_basic",
        "codec_length_ref_dotted_basic.py",
    );
}

#[test]
fn forge_python_codec_zenoh_scout() {
    assert_standalone_forge_python("codec_zenoh_scout", "codec_zenoh_scout.py");
}

// ── RFC §5.B B5-α multi-bit + empty-codec (Python) ───────────

#[test]
fn forge_python_codec_qos_byte() {
    assert_standalone_forge_python("codec_qos_byte", "codec_qos_byte.py");
}

#[test]
fn forge_python_codec_zenoh_keep_alive() {
    assert_standalone_forge_python("codec_zenoh_keep_alive", "codec_zenoh_keep_alive.py");
}

// ── RFC §5.B B1-γ flags primitive (Python) ───────────────────

#[test]
fn forge_python_codec_flags_basic() {
    assert_standalone_forge_python("codec_flags_basic", "codec_flags_basic.py");
}

// ── RFC §5.B variant primitive (Python, B1-β closure) ────────

#[test]
fn forge_python_codec_variant_session_open() {
    assert_standalone_forge_python(
        "codec_variant_session_open",
        "codec_variant_session_open.py",
    );
}

#[test]
fn forge_python_codec_variant_session_close() {
    assert_standalone_forge_python(
        "codec_variant_session_close",
        "codec_variant_session_close.py",
    );
}

#[test]
fn forge_python_codec_variant_dispatch() {
    assert_standalone_forge_python("codec_variant_dispatch", "codec_variant_dispatch.py");
}

#[test]
fn forge_python_codec_transport_envelope() {
    assert_standalone_forge_python("codec_transport_envelope", "codec_transport_envelope.py");
}

// ── RFC §5.B B1-δ present-if primitive (Python) ─────────────

#[test]
fn forge_python_codec_present_if_basic() {
    assert_standalone_forge_python("codec_present_if_basic", "codec_present_if_basic.py");
}

// ── RFC §5.B B5-λ present-if negation primitive (Python) ────

#[test]
fn forge_python_codec_present_if_negation() {
    assert_standalone_forge_python("codec_present_if_negation", "codec_present_if_negation.py");
}

// ── RFC §5.B Y3 atomic 2b-ii present-if disjunction primitive (Python) ──

#[test]
fn forge_python_codec_present_if_disjunction() {
    assert_standalone_forge_python(
        "codec_present_if_disjunction",
        "codec_present_if_disjunction.py",
    );
}

// ── RFC §5.B Y3 atomic 2b-ii peek-byte peek-byte primitive (Python) ──

#[test]
fn forge_python_codec_peek_arm_a() {
    assert_standalone_forge_python("codec_peek_arm_a", "codec_peek_arm_a.py");
}

#[test]
fn forge_python_codec_peek_arm_b() {
    assert_standalone_forge_python("codec_peek_arm_b", "codec_peek_arm_b.py");
}

#[test]
fn forge_python_codec_variant_peek_basic() {
    assert_standalone_forge_python("codec_variant_peek_basic", "codec_variant_peek_basic.py");
}

#[test]
fn forge_python_codec_zenoh_response() {
    assert_standalone_forge_python("codec_zenoh_response", "codec_zenoh_response.py");
}

// ── RFC §5.B B2-β present-if + variable-length (Python) ─────

#[test]
fn forge_python_codec_present_if_tail() {
    assert_standalone_forge_python("codec_present_if_tail", "codec_present_if_tail.py");
}

#[test]
fn forge_python_codec_present_if_length_ref() {
    assert_standalone_forge_python(
        "codec_present_if_length_ref",
        "codec_present_if_length_ref.py",
    );
}

#[test]
fn forge_python_codec_present_if_vle() {
    assert_standalone_forge_python("codec_present_if_vle", "codec_present_if_vle.py");
}

// ── RFC §5.B B2 repeat primitive (Python, final closure) ────

#[test]
fn forge_python_codec_repeat_elem() {
    assert_standalone_forge_python("codec_repeat_elem", "codec_repeat_elem.py");
}

#[test]
fn forge_python_codec_repeat_basic() {
    assert_standalone_forge_python("codec_repeat_basic", "codec_repeat_basic.py");
}

#[test]
fn forge_python_codec_until_eof_basic() {
    assert_standalone_forge_python("codec_until_eof_basic", "codec_until_eof_basic.py");
}

#[test]
fn forge_python_codec_repeat_present_if_basic() {
    assert_standalone_forge_python(
        "codec_repeat_present_if_basic",
        "codec_repeat_present_if_basic.py",
    );
}

#[test]
fn forge_python_codec_zenoh_hello() {
    assert_standalone_forge_python("codec_zenoh_hello", "codec_zenoh_hello.py");
}

// Wire RFC Phase B Y0a — see cpp registrations above for context.
#[test]
fn forge_python_codec_present_if_string() {
    assert_standalone_forge_python("codec_present_if_string", "codec_present_if_string.py");
}

#[test]
fn forge_python_codec_zenoh_wireexpr() {
    assert_standalone_forge_python("codec_zenoh_wireexpr", "codec_zenoh_wireexpr.py");
}

#[test]
fn forge_python_codec_embed_basic() {
    assert_standalone_forge_python("codec_embed_basic", "codec_embed_basic.py");
}

#[test]
fn forge_python_codec_zenoh_decl_kexpr() {
    assert_standalone_forge_python("codec_zenoh_decl_kexpr", "codec_zenoh_decl_kexpr.py");
}

#[test]
fn forge_python_codec_zenoh_decl_subscriber() {
    assert_standalone_forge_python(
        "codec_zenoh_decl_subscriber",
        "codec_zenoh_decl_subscriber.py",
    );
}

#[test]
fn forge_python_codec_zenoh_decl_queryable() {
    assert_standalone_forge_python(
        "codec_zenoh_decl_queryable",
        "codec_zenoh_decl_queryable.py",
    );
}

#[test]
fn forge_python_codec_zenoh_decl_token() {
    assert_standalone_forge_python("codec_zenoh_decl_token", "codec_zenoh_decl_token.py");
}

#[test]
fn forge_python_codec_zenoh_undecl_kexpr() {
    assert_standalone_forge_python(
        "codec_zenoh_undecl_kexpr",
        "codec_zenoh_undecl_kexpr.py",
    );
}

// ── RFC §5.B Wire RFC Phase B Y0b — TLV envelope foundation ────
#[test]
fn forge_python_codec_zenoh_decl_ext_keyexpr_inner() {
    assert_standalone_forge_python(
        "codec_zenoh_decl_ext_keyexpr_inner",
        "codec_zenoh_decl_ext_keyexpr_inner.py",
    );
}

#[test]
fn forge_python_codec_zenoh_decl_ext_keyexpr() {
    assert_standalone_forge_python(
        "codec_zenoh_decl_ext_keyexpr",
        "codec_zenoh_decl_ext_keyexpr.py",
    );
}

#[test]
fn forge_python_codec_zenoh_undecl_subscriber() {
    assert_standalone_forge_python(
        "codec_zenoh_undecl_subscriber",
        "codec_zenoh_undecl_subscriber.py",
    );
}

#[test]
fn forge_python_codec_zenoh_undecl_queryable() {
    assert_standalone_forge_python(
        "codec_zenoh_undecl_queryable",
        "codec_zenoh_undecl_queryable.py",
    );
}

#[test]
fn forge_python_codec_zenoh_undecl_token() {
    assert_standalone_forge_python("codec_zenoh_undecl_token", "codec_zenoh_undecl_token.py");
}

#[test]
fn forge_python_codec_zenoh_source_info() {
    assert_standalone_forge_python("codec_zenoh_source_info", "codec_zenoh_source_info.py");
}

#[test]
fn forge_python_codec_zenoh_source_info_ext() {
    assert_standalone_forge_python(
        "codec_zenoh_source_info_ext",
        "codec_zenoh_source_info_ext.py",
    );
}

#[test]
fn forge_python_codec_zenoh_timestamp_ext() {
    assert_standalone_forge_python("codec_zenoh_timestamp_ext", "codec_zenoh_timestamp_ext.py");
}

// ── RFC §5.B B4 applied codec shapes (Python) ───────────────

#[test]
fn forge_python_codec_ext_timestamp() {
    assert_standalone_forge_python("codec_ext_timestamp", "codec_ext_timestamp.py");
}

#[test]
fn forge_python_codec_ext_attachment() {
    assert_standalone_forge_python("codec_ext_attachment", "codec_ext_attachment.py");
}

#[test]
fn forge_python_codec_ext_encoding_info() {
    assert_standalone_forge_python("codec_ext_encoding_info", "codec_ext_encoding_info.py");
}

// ── Algorithm (Python, RFC §5.A — post-A6 matrix follow-up) ─

/// RFC §5.B B2-test-vector Python closure (final): the algorithm
/// body itself stays byte-stable against its prior golden — the
/// closure only adds a sidecar emission, so the primary algorithm
/// output stays identical to the pre-test-vector form.
#[test]
fn forge_python_algorithm_crc16() {
    assert_standalone_forge_python("algorithm_crc16", "algorithm_crc16.py");
}

/// RFC §5.B B2-test-vector Python closure (final): pin the
/// per-fixture sidecar (`<snake>_test.py`) emitted next to the
/// algorithm `.py` in the conformance_generated dir. The harness
/// module re-exports the `<Pascal>TestVectors(unittest.TestCase)`
/// class so pytest discovery picks it up via the existing
/// `import *` shim at
/// `sce-forge-runtime/python/tests/test_numerical_conformance.py`.
#[test]
fn forge_python_algorithm_crc16_test_vector_sidecar() {
    assert_sidecar_forge_lang(
        "algorithm_crc16",
        "algorithm_crc16_test.py",
        sce_build::generator::Language::Python,
    );
}

#[test]
fn forge_python_algorithm_crc16_table() {
    assert_standalone_forge_python("algorithm_crc16_table", "algorithm_crc16_table.py");
}

#[test]
fn forge_python_algorithm_const_fold_smoke() {
    assert_standalone_forge_python(
        "algorithm_const_fold_smoke",
        "algorithm_const_fold_smoke.py",
    );
}

// ══════════════════════════════════════════════════════════════
// ── C11 conformance tests ────────────────────────────────────
// ══════════════════════════════════════════════════════════════

/// Generate C11 from a standalone forge SCXML and compare against expected output.
fn assert_standalone_forge_c(scxml_name: &str, expected_filename: &str) {
    assert_standalone_forge_lang(
        scxml_name,
        expected_filename,
        sce_build::generator::Language::C11,
    );
}

// ── Codec (C11) ─────────────────────────────────────────────

#[test]
fn forge_c11_codec_simple_frame() {
    assert_standalone_forge_c("codec_simple_frame", "codec_simple_frame.c.h");
}

#[test]
fn forge_c11_codec_little_endian() {
    assert_standalone_forge_c("codec_little_endian", "codec_little_endian.c.h");
}

#[test]
fn forge_c11_codec_subbyte() {
    assert_standalone_forge_c("codec_subbyte", "codec_subbyte.c.h");
}

#[test]
fn forge_c11_codec_tail() {
    assert_standalone_forge_c("codec_tail", "codec_tail.c.h");
}

#[test]
fn forge_c11_codec_length_ref() {
    assert_standalone_forge_c("codec_length_ref", "codec_length_ref.c.h");
}

#[test]
fn forge_c11_codec_vle_zint_u64() {
    assert_standalone_forge_c("codec_vle_zint_u64", "codec_vle_zint_u64.c.h");
}

// ── RFC §5.B B5-prep Zenoh transport-message body codecs (C11) ───

#[test]
fn forge_c11_codec_zenoh_close() {
    assert_standalone_forge_c("codec_zenoh_close", "codec_zenoh_close.c.h");
}

#[test]
fn forge_c11_codec_zenoh_frame() {
    assert_standalone_forge_c("codec_zenoh_frame", "codec_zenoh_frame.c.h");
}

// ── RFC §5.B B5-ι cross-codec composition (C11) ──────────────

#[test]
fn forge_c11_codec_zenoh_open_body() {
    assert_standalone_forge_c("codec_zenoh_open_body", "codec_zenoh_open_body.c.h");
}

#[test]
fn forge_c11_codec_zenoh_init_body() {
    assert_standalone_forge_c("codec_zenoh_init_body", "codec_zenoh_init_body.c.h");
}

#[test]
fn forge_c11_codec_zenoh_join() {
    assert_standalone_forge_c("codec_zenoh_join", "codec_zenoh_join.c.h");
}

#[test]
fn forge_c11_codec_zenoh_fragment() {
    assert_standalone_forge_c("codec_zenoh_fragment", "codec_zenoh_fragment.c.h");
}

#[test]
fn forge_c11_codec_zenoh_decl_final() {
    assert_standalone_forge_c("codec_zenoh_decl_final", "codec_zenoh_decl_final.c.h");
}

// ── RFC §5.B B5-κ Surface L dotted-path length-field (C11) ─────

#[test]
fn forge_c11_codec_length_ref_dotted_basic() {
    assert_standalone_forge_c(
        "codec_length_ref_dotted_basic",
        "codec_length_ref_dotted_basic.c.h",
    );
}

#[test]
fn forge_c11_codec_zenoh_scout() {
    assert_standalone_forge_c("codec_zenoh_scout", "codec_zenoh_scout.c.h");
}

// ── RFC §5.B B5-ζ Surface H string primitive (C11) ───────────

#[test]
fn forge_c11_codec_zenoh_locator() {
    assert_standalone_forge_c("codec_zenoh_locator", "codec_zenoh_locator.c.h");
}

// ── RFC §5.B B5-η Surface I recursive variant body (C11) ─────

#[test]
fn forge_c11_codec_zenoh_put() {
    assert_standalone_forge_c("codec_zenoh_put", "codec_zenoh_put.c.h");
}

#[test]
fn forge_c11_codec_zenoh_del() {
    assert_standalone_forge_c("codec_zenoh_del", "codec_zenoh_del.c.h");
}

#[test]
fn forge_c11_codec_zenoh_push_body() {
    assert_standalone_forge_c("codec_zenoh_push_body", "codec_zenoh_push_body.c.h");
}

#[test]
fn forge_c11_codec_zenoh_push() {
    assert_standalone_forge_c("codec_zenoh_push", "codec_zenoh_push.c.h");
}

// ── RFC §5.B B5-α multi-bit + empty-codec (C11) ──────────────

#[test]
fn forge_c11_codec_qos_byte() {
    assert_standalone_forge_c("codec_qos_byte", "codec_qos_byte.c.h");
}

#[test]
fn forge_c11_codec_zenoh_keep_alive() {
    assert_standalone_forge_c("codec_zenoh_keep_alive", "codec_zenoh_keep_alive.c.h");
}

// ── RFC §5.B B1-γ flags primitive (C11) ──────────────────────

#[test]
fn forge_c11_codec_flags_basic() {
    assert_standalone_forge_c("codec_flags_basic", "codec_flags_basic.c.h");
}

// ── RFC §5.B variant primitive (C11, B1-β closure) ───────────

#[test]
fn forge_c11_codec_variant_session_open() {
    assert_standalone_forge_c(
        "codec_variant_session_open",
        "codec_variant_session_open.c.h",
    );
}

#[test]
fn forge_c11_codec_variant_session_close() {
    assert_standalone_forge_c(
        "codec_variant_session_close",
        "codec_variant_session_close.c.h",
    );
}

#[test]
fn forge_c11_codec_variant_dispatch() {
    assert_standalone_forge_c("codec_variant_dispatch", "codec_variant_dispatch.c.h");
}

#[test]
fn forge_c11_codec_transport_envelope() {
    assert_standalone_forge_c("codec_transport_envelope", "codec_transport_envelope.c.h");
}

// ── RFC §5.B B1-δ present-if primitive (C11) ────────────────

#[test]
fn forge_c11_codec_present_if_basic() {
    assert_standalone_forge_c("codec_present_if_basic", "codec_present_if_basic.c.h");
}

// ── RFC §5.B B5-λ present-if negation primitive (C11) ───────

#[test]
fn forge_c11_codec_present_if_negation() {
    assert_standalone_forge_c("codec_present_if_negation", "codec_present_if_negation.c.h");
}

// ── RFC §5.B Y3 atomic 2b-ii present-if disjunction primitive (C11) ──

#[test]
fn forge_c11_codec_present_if_disjunction() {
    assert_standalone_forge_c(
        "codec_present_if_disjunction",
        "codec_present_if_disjunction.c.h",
    );
}

// ── RFC §5.B Y3 atomic 2b-ii peek-byte peek-byte primitive (C11) ──

#[test]
fn forge_c11_codec_peek_arm_a() {
    assert_standalone_forge_c("codec_peek_arm_a", "codec_peek_arm_a.c.h");
}

#[test]
fn forge_c11_codec_peek_arm_b() {
    assert_standalone_forge_c("codec_peek_arm_b", "codec_peek_arm_b.c.h");
}

#[test]
fn forge_c11_codec_variant_peek_basic() {
    assert_standalone_forge_c("codec_variant_peek_basic", "codec_variant_peek_basic.c.h");
}

#[test]
fn forge_c11_codec_zenoh_response() {
    assert_standalone_forge_c("codec_zenoh_response", "codec_zenoh_response.c.h");
}

// ── RFC §5.B B2-β present-if + variable-length (C11) ────────

#[test]
fn forge_c11_codec_present_if_tail() {
    assert_standalone_forge_c("codec_present_if_tail", "codec_present_if_tail.c.h");
}

#[test]
fn forge_c11_codec_present_if_length_ref() {
    assert_standalone_forge_c(
        "codec_present_if_length_ref",
        "codec_present_if_length_ref.c.h",
    );
}

#[test]
fn forge_c11_codec_present_if_vle() {
    assert_standalone_forge_c("codec_present_if_vle", "codec_present_if_vle.c.h");
}

// ── RFC §5.B B2 repeat primitive (C11, closure) ─────────────

#[test]
fn forge_c11_codec_repeat_elem() {
    assert_standalone_forge_c("codec_repeat_elem", "codec_repeat_elem.c.h");
}

#[test]
fn forge_c11_codec_repeat_basic() {
    assert_standalone_forge_c("codec_repeat_basic", "codec_repeat_basic.c.h");
}

#[test]
fn forge_c11_codec_until_eof_basic() {
    assert_standalone_forge_c("codec_until_eof_basic", "codec_until_eof_basic.c.h");
}

#[test]
fn forge_c11_codec_repeat_present_if_basic() {
    assert_standalone_forge_c(
        "codec_repeat_present_if_basic",
        "codec_repeat_present_if_basic.c.h",
    );
}

#[test]
fn forge_c11_codec_zenoh_hello() {
    assert_standalone_forge_c("codec_zenoh_hello", "codec_zenoh_hello.c.h");
}

// Wire RFC Phase B Y0a — see cpp registrations above for context.
#[test]
fn forge_c11_codec_present_if_string() {
    assert_standalone_forge_c("codec_present_if_string", "codec_present_if_string.c.h");
}

#[test]
fn forge_c11_codec_zenoh_wireexpr() {
    assert_standalone_forge_c("codec_zenoh_wireexpr", "codec_zenoh_wireexpr.c.h");
}

#[test]
fn forge_c11_codec_embed_basic() {
    assert_standalone_forge_c("codec_embed_basic", "codec_embed_basic.c.h");
}

#[test]
fn forge_c11_codec_zenoh_decl_kexpr() {
    assert_standalone_forge_c("codec_zenoh_decl_kexpr", "codec_zenoh_decl_kexpr.c.h");
}

#[test]
fn forge_c11_codec_zenoh_decl_subscriber() {
    assert_standalone_forge_c(
        "codec_zenoh_decl_subscriber",
        "codec_zenoh_decl_subscriber.c.h",
    );
}

#[test]
fn forge_c11_codec_zenoh_decl_queryable() {
    assert_standalone_forge_c(
        "codec_zenoh_decl_queryable",
        "codec_zenoh_decl_queryable.c.h",
    );
}

#[test]
fn forge_c11_codec_zenoh_decl_token() {
    assert_standalone_forge_c("codec_zenoh_decl_token", "codec_zenoh_decl_token.c.h");
}

#[test]
fn forge_c11_codec_zenoh_undecl_kexpr() {
    assert_standalone_forge_c(
        "codec_zenoh_undecl_kexpr",
        "codec_zenoh_undecl_kexpr.c.h",
    );
}

// ── RFC §5.B Wire RFC Phase B Y0b — TLV envelope foundation ────
#[test]
fn forge_c11_codec_zenoh_decl_ext_keyexpr_inner() {
    assert_standalone_forge_c(
        "codec_zenoh_decl_ext_keyexpr_inner",
        "codec_zenoh_decl_ext_keyexpr_inner.c.h",
    );
}

#[test]
fn forge_c11_codec_zenoh_decl_ext_keyexpr() {
    assert_standalone_forge_c(
        "codec_zenoh_decl_ext_keyexpr",
        "codec_zenoh_decl_ext_keyexpr.c.h",
    );
}

#[test]
fn forge_c11_codec_zenoh_undecl_subscriber() {
    assert_standalone_forge_c(
        "codec_zenoh_undecl_subscriber",
        "codec_zenoh_undecl_subscriber.c.h",
    );
}

#[test]
fn forge_c11_codec_zenoh_undecl_queryable() {
    assert_standalone_forge_c(
        "codec_zenoh_undecl_queryable",
        "codec_zenoh_undecl_queryable.c.h",
    );
}

#[test]
fn forge_c11_codec_zenoh_undecl_token() {
    assert_standalone_forge_c("codec_zenoh_undecl_token", "codec_zenoh_undecl_token.c.h");
}

#[test]
fn forge_c11_codec_zenoh_source_info() {
    assert_standalone_forge_c("codec_zenoh_source_info", "codec_zenoh_source_info.c.h");
}

#[test]
fn forge_c11_codec_zenoh_source_info_ext() {
    assert_standalone_forge_c(
        "codec_zenoh_source_info_ext",
        "codec_zenoh_source_info_ext.c.h",
    );
}

#[test]
fn forge_c11_codec_zenoh_timestamp_ext() {
    assert_standalone_forge_c("codec_zenoh_timestamp_ext", "codec_zenoh_timestamp_ext.c.h");
}

// ── RFC §5.B B3 TLV chain primitive (C11, trunk) ────────────
// `codec_tlv_chain_basic` declares a uint8 header_flags then a TLV
// chain bounded at max-depth=8 with on-overflow="reject". Each entry
// is `codec_tlv_entry` (id+len+value, RFC line 488). MCU-class —
// only Rust + C11 emit; cpp/kotlin/go/python typed-reject in
// render_codec via the codec-content MCU gate. The entry body codec
// `codec_tlv_entry` is plain (no MCU-only sub-features) so it ships
// on all 6 backends — only the *parent* containing the tlv-chain
// field is MCU-only.

#[test]
fn forge_c11_codec_tlv_entry() {
    assert_standalone_forge_c("codec_tlv_entry", "codec_tlv_entry.c.h");
}

#[test]
fn forge_c11_codec_tlv_chain_basic() {
    assert_standalone_forge_c("codec_tlv_chain_basic", "codec_tlv_chain_basic.c.h");
}

// ── RFC §5.B B5-ε surface G — TLV chain entry body keyed by carrier bits ─
// `codec_zenoh_ext_envelope` carries a `<sce:tlv-chain>` of
// variant-bodied `codec_zenoh_ext_entry` entries. C11 trunk pins:
//   - the envelope's MAX_BYTES = `1 + max_depth * entry_max` (345 =
//     1 + 8*43) — proves the transitive recursive max-bytes
//     enrichment kicks in for variant-bearing imports
//   - the entry's tagged-union body slot fits the chain's fixed-array
//     `extensions[max_depth]` slot since each variant arm body is
//     itself bounded (Unit=0, ZInt=10, ZBuf=42)
//   - the chain decode/encode loop calls
//     `codec_zenoh_ext_entry_decode(cursor, &out->extensions[idx])`
//     unchanged from `codec_tlv_chain_basic`'s shape — the
//     variant-aware dispatch is internal to the entry codec.

#[test]
fn forge_c11_codec_zenoh_ext_unit() {
    assert_standalone_forge_c("codec_zenoh_ext_unit", "codec_zenoh_ext_unit.c.h");
}

#[test]
fn forge_c11_codec_zenoh_ext_zint() {
    assert_standalone_forge_c("codec_zenoh_ext_zint", "codec_zenoh_ext_zint.c.h");
}

#[test]
fn forge_c11_codec_zenoh_ext_zbuf() {
    assert_standalone_forge_c("codec_zenoh_ext_zbuf", "codec_zenoh_ext_zbuf.c.h");
}

#[test]
fn forge_c11_codec_zenoh_ext_entry() {
    assert_standalone_forge_c("codec_zenoh_ext_entry", "codec_zenoh_ext_entry.c.h");
}

#[test]
fn forge_c11_codec_zenoh_ext_envelope() {
    assert_standalone_forge_c("codec_zenoh_ext_envelope", "codec_zenoh_ext_envelope.c.h");
}

// ── RFC §5.B B3 DMA alignment primitive (C11, trunk) ────────
// `codec_dma_aligned_basic` declares msg_id + reserved at bytes 0-1
// then a tail-bytes aligned_payload at byte 32 with
// sce:dma-burst-align="32". Codegen emits a `_Static_assert` on the
// literal byte offset (drift detection) + `memset(r.bytes, 0,
// sizeof(r.bytes))` at encode start so the 30 padding bytes between
// the prefix and the aligned field land as deterministic zeros on
// the wire. MCU-class — same gate rejects cpp/kotlin/go/python.

#[test]
fn forge_c11_codec_dma_aligned_basic() {
    assert_standalone_forge_c("codec_dma_aligned_basic", "codec_dma_aligned_basic.c.h");
}

// ── RFC §5.B B4 applied codec shapes (C11) ──────────────────

#[test]
fn forge_c11_codec_ext_timestamp() {
    assert_standalone_forge_c("codec_ext_timestamp", "codec_ext_timestamp.c.h");
}

#[test]
fn forge_c11_codec_ext_attachment() {
    assert_standalone_forge_c("codec_ext_attachment", "codec_ext_attachment.c.h");
}

#[test]
fn forge_c11_codec_ext_encoding_info() {
    assert_standalone_forge_c("codec_ext_encoding_info", "codec_ext_encoding_info.c.h");
}

// ── Crossfile codec (C11) ───────────────────────────────────

#[test]
fn forge_c11_crossfile_procedure_codec() {
    assert_standalone_forge_c("crossfile_procedure_codec", "crossfile_procedure_codec.c.h");
}

#[test]
fn forge_c11_crossfile_procedure_codec_mutate() {
    assert_standalone_forge_c(
        "crossfile_procedure_codec_mutate",
        "crossfile_procedure_codec_mutate.c.h",
    );
}

#[test]
fn forge_c11_crossfile_procedure_filter() {
    assert_standalone_forge_c(
        "crossfile_procedure_filter",
        "crossfile_procedure_filter.c.h",
    );
}

#[test]
fn forge_c11_crossfile_validator_codec() {
    assert_standalone_forge_c("crossfile_validator_codec", "crossfile_validator_codec.c.h");
}

#[test]
fn forge_c11_crossfile_validator_filter() {
    assert_standalone_forge_c(
        "crossfile_validator_filter",
        "crossfile_validator_filter.c.h",
    );
}

#[test]
fn forge_c11_crossfile_validator_transform() {
    assert_standalone_forge_c(
        "crossfile_validator_transform",
        "crossfile_validator_transform.c.h",
    );
}

#[test]
fn forge_c11_crossfile_validator_condition() {
    assert_standalone_forge_c(
        "crossfile_validator_condition",
        "crossfile_validator_condition.c.h",
    );
}

#[test]
fn forge_c11_crossfile_validator_lookup() {
    assert_standalone_forge_c(
        "crossfile_validator_lookup",
        "crossfile_validator_lookup.c.h",
    );
}

#[test]
fn forge_c11_crossfile_validator_interpolation() {
    assert_standalone_forge_c(
        "crossfile_validator_interpolation",
        "crossfile_validator_interpolation.c.h",
    );
}

// ── Transform (C11) ──────────────────────────────────────────

#[test]
fn forge_c11_transform_temperature() {
    assert_standalone_forge_c("transform_temperature", "transform_temperature.c.h");
}

#[test]
fn forge_c11_transform_multi_output() {
    assert_standalone_forge_c("transform_multi_output", "transform_multi_output.c.h");
}

#[test]
fn forge_c11_transform_bitwise() {
    assert_standalone_forge_c("transform_bitwise", "transform_bitwise.c.h");
}

// ── Lookup (C11) ─────────────────────────────────────────────

#[test]
fn forge_c11_lookup_engine_status() {
    assert_standalone_forge_c("lookup_engine_status", "lookup_engine_status.c.h");
}

#[test]
fn forge_c11_lookup_alarm_code() {
    assert_standalone_forge_c("lookup_alarm_code", "lookup_alarm_code.c.h");
}

#[test]
fn forge_c11_lookup_state_action() {
    assert_standalone_forge_c("lookup_state_action", "lookup_state_action.c.h");
}

#[test]
fn forge_c11_lookup_unit_scale() {
    assert_standalone_forge_c("lookup_unit_scale", "lookup_unit_scale.c.h");
}

#[test]
fn forge_c11_lookup_severity_default() {
    assert_standalone_forge_c("lookup_severity_default", "lookup_severity_default.c.h");
}

// ── Condition (C11) ──────────────────────────────────────────

#[test]
fn forge_c11_condition_programming() {
    assert_standalone_forge_c("condition_programming", "condition_programming.c.h");
}

#[test]
fn forge_c11_condition_threshold() {
    assert_standalone_forge_c("condition_threshold", "condition_threshold.c.h");
}

#[test]
fn forge_c11_condition_range() {
    assert_standalone_forge_c("condition_range", "condition_range.c.h");
}

// ── Validator (C11) ──────────────────────────────────────────

#[test]
fn forge_c11_validator_rpm_check() {
    assert_standalone_forge_c("validator_rpm_check", "validator_rpm_check.c.h");
}

#[test]
fn forge_c11_validator_range_only() {
    assert_standalone_forge_c("validator_range_only", "validator_range_only.c.h");
}

#[test]
fn forge_c11_validator_signed_roc() {
    assert_standalone_forge_c("validator_signed_roc", "validator_signed_roc.c.h");
}

#[test]
fn forge_c11_validator_plausibility_only() {
    assert_standalone_forge_c(
        "validator_plausibility_only",
        "validator_plausibility_only.c.h",
    );
}

// ── Procedure (C11) ──────────────────────────────────────────

#[test]
fn forge_c11_procedure_startup_check() {
    assert_standalone_forge_c("procedure_startup_check", "procedure_startup_check.c.h");
}

#[test]
fn forge_c11_procedure_linear() {
    assert_standalone_forge_c("procedure_linear", "procedure_linear.c.h");
}

#[test]
fn forge_c11_procedure_diamond() {
    assert_standalone_forge_c("procedure_diamond", "procedure_diamond.c.h");
}

#[test]
fn forge_c11_procedure_security_access() {
    assert_standalone_forge_c("procedure_security_access", "procedure_security_access.c.h");
}

// ── Interpolation (C11) ──────────────────────────────────────

#[test]
fn forge_c11_interpolation_1d_linear() {
    assert_standalone_forge_c("interpolation_1d_linear", "interpolation_1d_linear.c.h");
}

#[test]
fn forge_c11_interpolation_2d_bilinear() {
    assert_standalone_forge_c("interpolation_2d_bilinear", "interpolation_2d_bilinear.c.h");
}

// ── Filter (C11) ─────────────────────────────────────────────

#[test]
fn forge_c11_filter_moving_average() {
    assert_standalone_forge_c("filter_moving_average", "filter_moving_average.c.h");
}

#[test]
fn forge_c11_filter_low_pass() {
    assert_standalone_forge_c("filter_low_pass", "filter_low_pass.c.h");
}

#[test]
fn forge_c11_filter_debounce() {
    assert_standalone_forge_c("filter_debounce", "filter_debounce.c.h");
}

// ── Observer (C11) ───────────────────────────────────────────

#[test]
fn forge_c11_observer_coolant() {
    assert_standalone_forge_c("observer_coolant", "observer_coolant.c.h");
}

// ── Timer (C11) ──────────────────────────────────────────────

#[test]
fn forge_c11_timer_diag_scheduler() {
    assert_standalone_forge_c("timer_diag_scheduler", "timer_diag_scheduler.c.h");
}

// ── Algorithm (C11, RFC §5.A — Phase A5 closes the §5.J.4 matrix) ──

#[test]
fn forge_c11_algorithm_crc16() {
    assert_standalone_forge_c("algorithm_crc16", "algorithm_crc16.c.h");
}

#[test]
fn forge_c11_algorithm_crc16_table() {
    assert_standalone_forge_c("algorithm_crc16_table", "algorithm_crc16_table.c.h");
}

#[test]
fn forge_c11_algorithm_const_fold_smoke() {
    assert_standalone_forge_c(
        "algorithm_const_fold_smoke",
        "algorithm_const_fold_smoke.c.h",
    );
}

// ── Validator conformance (C++) ──────────────────────────────

#[test]
fn forge_validator_rpm_check() {
    assert_standalone_forge("validator_rpm_check", "validator_rpm_check.h");
}

#[test]
fn forge_validator_range_only() {
    assert_standalone_forge("validator_range_only", "validator_range_only.h");
}

#[test]
fn forge_validator_signed_roc() {
    assert_standalone_forge("validator_signed_roc", "validator_signed_roc.h");
}

#[test]
fn forge_validator_plausibility_only() {
    assert_standalone_forge(
        "validator_plausibility_only",
        "validator_plausibility_only.h",
    );
}

// ── Validator conformance (Kotlin) ──────────────────────────

#[test]
fn forge_kotlin_validator_rpm_check() {
    assert_standalone_forge_kotlin("validator_rpm_check", "ValidatorRpmCheck.kt");
}

#[test]
fn forge_kotlin_validator_range_only() {
    assert_standalone_forge_kotlin("validator_range_only", "ValidatorRangeOnly.kt");
}

#[test]
fn forge_kotlin_validator_signed_roc() {
    assert_standalone_forge_kotlin("validator_signed_roc", "ValidatorSignedRoc.kt");
}

#[test]
fn forge_kotlin_validator_plausibility_only() {
    assert_standalone_forge_kotlin(
        "validator_plausibility_only",
        "ValidatorPlausibilityOnly.kt",
    );
}

// ── Validator conformance (Rust) ────────────────────────────

#[test]
fn forge_rust_validator_rpm_check() {
    assert_standalone_forge_rust("validator_rpm_check", "validator_rpm_check.rs");
}

#[test]
fn forge_rust_validator_range_only() {
    assert_standalone_forge_rust("validator_range_only", "validator_range_only.rs");
}

#[test]
fn forge_rust_validator_signed_roc() {
    assert_standalone_forge_rust("validator_signed_roc", "validator_signed_roc.rs");
}

#[test]
fn forge_rust_validator_plausibility_only() {
    assert_standalone_forge_rust(
        "validator_plausibility_only",
        "validator_plausibility_only.rs",
    );
}

// ── Validator conformance (Go) ──────────────────────────────

#[test]
fn forge_go_validator_rpm_check() {
    assert_standalone_forge_go("validator_rpm_check", "validator_rpm_check.go");
}

#[test]
fn forge_go_validator_range_only() {
    assert_standalone_forge_go("validator_range_only", "validator_range_only.go");
}

#[test]
fn forge_go_validator_signed_roc() {
    assert_standalone_forge_go("validator_signed_roc", "validator_signed_roc.go");
}

#[test]
fn forge_go_validator_plausibility_only() {
    assert_standalone_forge_go(
        "validator_plausibility_only",
        "validator_plausibility_only.go",
    );
}

// ── Validator conformance (Python) ──────────────────────────

#[test]
fn forge_python_validator_rpm_check() {
    assert_standalone_forge_python("validator_rpm_check", "validator_rpm_check.py");
}

#[test]
fn forge_python_validator_range_only() {
    assert_standalone_forge_python("validator_range_only", "validator_range_only.py");
}

#[test]
fn forge_python_validator_signed_roc() {
    assert_standalone_forge_python("validator_signed_roc", "validator_signed_roc.py");
}

#[test]
fn forge_python_validator_plausibility_only() {
    assert_standalone_forge_python(
        "validator_plausibility_only",
        "validator_plausibility_only.py",
    );
}

// ── Procedure conformance (C++) ─────────────────────────────

#[test]
fn forge_procedure_startup_check() {
    assert_standalone_forge("procedure_startup_check", "procedure_startup_check.h");
}

#[test]
fn forge_procedure_linear() {
    assert_standalone_forge("procedure_linear", "procedure_linear.h");
}

#[test]
fn forge_procedure_diamond() {
    assert_standalone_forge("procedure_diamond", "procedure_diamond.h");
}

// ── Procedure Level 2 conformance (C++, event-driven) ───────

#[test]
fn forge_procedure_security_access() {
    assert_standalone_forge("procedure_security_access", "procedure_security_access.h");
}

// ── Procedure Level 2 conformance (Kotlin, event-driven) ────

#[test]
fn forge_kotlin_procedure_security_access() {
    assert_standalone_forge_kotlin("procedure_security_access", "ProcedureSecurityAccess.kt");
}

// ── Procedure Level 2 conformance (Rust, event-driven) ──────

#[test]
fn forge_rust_procedure_security_access() {
    assert_standalone_forge_rust("procedure_security_access", "procedure_security_access.rs");
}

// ── Procedure Level 2 conformance (Go, event-driven) ────────

#[test]
fn forge_go_procedure_security_access() {
    assert_standalone_forge_go("procedure_security_access", "procedure_security_access.go");
}

// ── Procedure Level 2 conformance (Python, event-driven) ────

#[test]
fn forge_python_procedure_security_access() {
    assert_standalone_forge_python("procedure_security_access", "procedure_security_access.py");
}

// ── Procedure conformance (Kotlin) ──────────────────────────

#[test]
fn forge_kotlin_procedure_startup_check() {
    assert_standalone_forge_kotlin("procedure_startup_check", "ProcedureStartupCheck.kt");
}

#[test]
fn forge_kotlin_procedure_linear() {
    assert_standalone_forge_kotlin("procedure_linear", "ProcedureLinear.kt");
}

#[test]
fn forge_kotlin_procedure_diamond() {
    assert_standalone_forge_kotlin("procedure_diamond", "ProcedureDiamond.kt");
}

// ── Procedure conformance (Rust) ────────────────────────────

#[test]
fn forge_rust_procedure_startup_check() {
    assert_standalone_forge_rust("procedure_startup_check", "procedure_startup_check.rs");
}

#[test]
fn forge_rust_procedure_linear() {
    assert_standalone_forge_rust("procedure_linear", "procedure_linear.rs");
}

#[test]
fn forge_rust_procedure_diamond() {
    assert_standalone_forge_rust("procedure_diamond", "procedure_diamond.rs");
}

// ── Procedure conformance (Go) ──────────────────────────────

#[test]
fn forge_go_procedure_startup_check() {
    assert_standalone_forge_go("procedure_startup_check", "procedure_startup_check.go");
}

#[test]
fn forge_go_procedure_linear() {
    assert_standalone_forge_go("procedure_linear", "procedure_linear.go");
}

#[test]
fn forge_go_procedure_diamond() {
    assert_standalone_forge_go("procedure_diamond", "procedure_diamond.go");
}

// ── Procedure conformance (Python) ──────────────────────────

#[test]
fn forge_python_procedure_startup_check() {
    assert_standalone_forge_python("procedure_startup_check", "procedure_startup_check.py");
}

#[test]
fn forge_python_procedure_linear() {
    assert_standalone_forge_python("procedure_linear", "procedure_linear.py");
}

#[test]
fn forge_python_procedure_diamond() {
    assert_standalone_forge_python("procedure_diamond", "procedure_diamond.py");
}

// ── Cross-file kind composition ─────────────────────────────

#[test]
fn forge_crossfile_procedure_codec_cpp() {
    assert_standalone_forge("crossfile_procedure_codec", "crossfile_procedure_codec.h");
}

#[test]
fn forge_crossfile_procedure_codec_kotlin() {
    assert_standalone_forge_kotlin("crossfile_procedure_codec", "CrossfileProcedureCodec.kt");
}

#[test]
fn forge_crossfile_procedure_codec_rust() {
    assert_standalone_forge_rust("crossfile_procedure_codec", "crossfile_procedure_codec.rs");
}

#[test]
fn forge_crossfile_procedure_codec_go() {
    assert_standalone_forge_go("crossfile_procedure_codec", "crossfile_procedure_codec.go");
}

#[test]
fn forge_crossfile_procedure_codec_python() {
    assert_standalone_forge_python("crossfile_procedure_codec", "crossfile_procedure_codec.py");
}

#[test]
fn forge_crossfile_procedure_codec_mutate_cpp() {
    assert_standalone_forge(
        "crossfile_procedure_codec_mutate",
        "crossfile_procedure_codec_mutate.h",
    );
}

#[test]
fn forge_crossfile_procedure_codec_mutate_kotlin() {
    assert_standalone_forge_kotlin(
        "crossfile_procedure_codec_mutate",
        "CrossfileProcedureCodecMutate.kt",
    );
}

#[test]
fn forge_crossfile_procedure_codec_mutate_rust() {
    assert_standalone_forge_rust(
        "crossfile_procedure_codec_mutate",
        "crossfile_procedure_codec_mutate.rs",
    );
}

#[test]
fn forge_crossfile_procedure_codec_mutate_go() {
    assert_standalone_forge_go(
        "crossfile_procedure_codec_mutate",
        "crossfile_procedure_codec_mutate.go",
    );
}

#[test]
fn forge_crossfile_procedure_codec_mutate_python() {
    assert_standalone_forge_python(
        "crossfile_procedure_codec_mutate",
        "crossfile_procedure_codec_mutate.py",
    );
}

#[test]
fn forge_crossfile_procedure_filter_cpp() {
    assert_standalone_forge("crossfile_procedure_filter", "crossfile_procedure_filter.h");
}

#[test]
fn forge_crossfile_procedure_filter_kotlin() {
    assert_standalone_forge_kotlin("crossfile_procedure_filter", "CrossfileProcedureFilter.kt");
}

#[test]
fn forge_crossfile_procedure_filter_rust() {
    assert_standalone_forge_rust(
        "crossfile_procedure_filter",
        "crossfile_procedure_filter.rs",
    );
}

#[test]
fn forge_crossfile_procedure_filter_go() {
    assert_standalone_forge_go(
        "crossfile_procedure_filter",
        "crossfile_procedure_filter.go",
    );
}

#[test]
fn forge_crossfile_procedure_filter_python() {
    assert_standalone_forge_python(
        "crossfile_procedure_filter",
        "crossfile_procedure_filter.py",
    );
}

#[test]
fn forge_crossfile_validator_transform_cpp() {
    assert_standalone_forge(
        "crossfile_validator_transform",
        "crossfile_validator_transform.h",
    );
}

#[test]
fn forge_crossfile_validator_transform_kotlin() {
    assert_standalone_forge_kotlin(
        "crossfile_validator_transform",
        "CrossfileValidatorTransform.kt",
    );
}

#[test]
fn forge_crossfile_validator_transform_rust() {
    assert_standalone_forge_rust(
        "crossfile_validator_transform",
        "crossfile_validator_transform.rs",
    );
}

#[test]
fn forge_crossfile_validator_transform_go() {
    assert_standalone_forge_go(
        "crossfile_validator_transform",
        "crossfile_validator_transform.go",
    );
}

#[test]
fn forge_crossfile_validator_transform_python() {
    assert_standalone_forge_python(
        "crossfile_validator_transform",
        "crossfile_validator_transform.py",
    );
}

#[test]
fn forge_crossfile_validator_codec_cpp() {
    assert_standalone_forge("crossfile_validator_codec", "crossfile_validator_codec.h");
}

#[test]
fn forge_crossfile_validator_codec_kotlin() {
    assert_standalone_forge_kotlin("crossfile_validator_codec", "CrossfileValidatorCodec.kt");
}

#[test]
fn forge_crossfile_validator_codec_rust() {
    assert_standalone_forge_rust("crossfile_validator_codec", "crossfile_validator_codec.rs");
}

#[test]
fn forge_crossfile_validator_codec_go() {
    assert_standalone_forge_go("crossfile_validator_codec", "crossfile_validator_codec.go");
}

#[test]
fn forge_crossfile_validator_codec_python() {
    assert_standalone_forge_python("crossfile_validator_codec", "crossfile_validator_codec.py");
}

#[test]
fn forge_crossfile_validator_filter_cpp() {
    assert_standalone_forge("crossfile_validator_filter", "crossfile_validator_filter.h");
}

#[test]
fn forge_crossfile_validator_filter_kotlin() {
    assert_standalone_forge_kotlin("crossfile_validator_filter", "CrossfileValidatorFilter.kt");
}

#[test]
fn forge_crossfile_validator_filter_rust() {
    assert_standalone_forge_rust(
        "crossfile_validator_filter",
        "crossfile_validator_filter.rs",
    );
}

#[test]
fn forge_crossfile_validator_filter_go() {
    assert_standalone_forge_go(
        "crossfile_validator_filter",
        "crossfile_validator_filter.go",
    );
}

#[test]
fn forge_crossfile_validator_filter_python() {
    assert_standalone_forge_python(
        "crossfile_validator_filter",
        "crossfile_validator_filter.py",
    );
}

#[test]
fn forge_crossfile_validator_condition_cpp() {
    assert_standalone_forge(
        "crossfile_validator_condition",
        "crossfile_validator_condition.h",
    );
}

#[test]
fn forge_crossfile_validator_condition_kotlin() {
    assert_standalone_forge_kotlin(
        "crossfile_validator_condition",
        "CrossfileValidatorCondition.kt",
    );
}

#[test]
fn forge_crossfile_validator_condition_rust() {
    assert_standalone_forge_rust(
        "crossfile_validator_condition",
        "crossfile_validator_condition.rs",
    );
}

#[test]
fn forge_crossfile_validator_condition_go() {
    assert_standalone_forge_go(
        "crossfile_validator_condition",
        "crossfile_validator_condition.go",
    );
}

#[test]
fn forge_crossfile_validator_condition_python() {
    assert_standalone_forge_python(
        "crossfile_validator_condition",
        "crossfile_validator_condition.py",
    );
}

#[test]
fn forge_crossfile_validator_lookup_cpp() {
    assert_standalone_forge("crossfile_validator_lookup", "crossfile_validator_lookup.h");
}

#[test]
fn forge_crossfile_validator_lookup_kotlin() {
    assert_standalone_forge_kotlin("crossfile_validator_lookup", "CrossfileValidatorLookup.kt");
}

#[test]
fn forge_crossfile_validator_lookup_rust() {
    assert_standalone_forge_rust(
        "crossfile_validator_lookup",
        "crossfile_validator_lookup.rs",
    );
}

#[test]
fn forge_crossfile_validator_lookup_go() {
    assert_standalone_forge_go(
        "crossfile_validator_lookup",
        "crossfile_validator_lookup.go",
    );
}

#[test]
fn forge_crossfile_validator_lookup_python() {
    assert_standalone_forge_python(
        "crossfile_validator_lookup",
        "crossfile_validator_lookup.py",
    );
}

#[test]
fn forge_crossfile_validator_interpolation_cpp() {
    assert_standalone_forge(
        "crossfile_validator_interpolation",
        "crossfile_validator_interpolation.h",
    );
}

#[test]
fn forge_crossfile_validator_interpolation_kotlin() {
    assert_standalone_forge_kotlin(
        "crossfile_validator_interpolation",
        "CrossfileValidatorInterpolation.kt",
    );
}

#[test]
fn forge_crossfile_validator_interpolation_rust() {
    assert_standalone_forge_rust(
        "crossfile_validator_interpolation",
        "crossfile_validator_interpolation.rs",
    );
}

#[test]
fn forge_crossfile_validator_interpolation_go() {
    assert_standalone_forge_go(
        "crossfile_validator_interpolation",
        "crossfile_validator_interpolation.go",
    );
}

#[test]
fn forge_crossfile_validator_interpolation_python() {
    assert_standalone_forge_python(
        "crossfile_validator_interpolation",
        "crossfile_validator_interpolation.py",
    );
}

// ── Inline kind conformance ──────────────────────────────────

#[test]
fn forge_inline_mixed() {
    assert_inline_kinds_cpp("inline_mixed");
}

#[test]
fn forge_inline_mixed_kotlin() {
    assert_inline_kinds_lang("inline_mixed", sce_build::generator::Language::Kotlin);
}

#[test]
fn forge_inline_mixed_rust() {
    assert_inline_kinds_lang("inline_mixed", sce_build::generator::Language::Rust);
}

#[test]
fn forge_inline_mixed_go() {
    assert_inline_kinds_lang("inline_mixed", sce_build::generator::Language::Go);
}

#[test]
fn forge_inline_mixed_c11() {
    assert_inline_kinds_lang("inline_mixed", sce_build::generator::Language::C11);
}

// ── Inline codec conformance (Phase F-2) ───────────────────────
//
// Separate fixture from inline_mixed because the codec DSL is the
// only inline kind that emits both a payload struct and its
// (de)serialization pair — co-locating it with lookup/transform/
// condition would force the existing 4-language inline_mixed
// goldens to absorb codec output, conflating two structurally
// distinct emit shapes in one byte-compare.

#[test]
fn forge_inline_codec() {
    assert_inline_kinds_cpp("inline_codec");
}

#[test]
fn forge_inline_codec_kotlin() {
    assert_inline_kinds_lang("inline_codec", sce_build::generator::Language::Kotlin);
}

#[test]
fn forge_inline_codec_rust() {
    assert_inline_kinds_lang("inline_codec", sce_build::generator::Language::Rust);
}

#[test]
fn forge_inline_codec_go() {
    assert_inline_kinds_lang("inline_codec", sce_build::generator::Language::Go);
}

#[test]
fn forge_inline_codec_c11() {
    assert_inline_kinds_lang("inline_codec", sce_build::generator::Language::C11);
}

// ── Named Context typedef emission ─────────────────────────────
//
// The C++ codegen exposes one `using {Id}Type` alias per
// `<sce:context>` declaration, on both the Policy struct (source of
// truth) and the user-facing SM class (re-export). Consumers (traits,
// test harnesses) reference the alias instead of duplicating the
// underlying type across SCXML and C++ — drift between the two is
// then a compile error rather than a silent mismatch. The guard is a
// closed reserved-id list in the parser (`RESERVED_CONTEXT_IDS`).

fn compile_scxml_for_test(scxml: &str) -> String {
    // `compile_scxml_lang` takes a filesystem path; serialise the inline
    // SCXML to a unique temp file so parallel test runs cannot collide.
    // Stdlib only — no `tempfile` dep pulled in for a single test helper.
    static COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let id = COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let pid = std::process::id();
    let path = std::env::temp_dir().join(format!("sce_ctx_test_{pid}_{id}.scxml"));
    std::fs::write(&path, scxml).expect("write fixture");
    let tdir = sce_build::find_template_dir_for(sce_build::generator::Language::Cpp);
    let output = sce_build::compile_scxml_lang(
        path.to_str().unwrap(),
        &tdir,
        sce_build::generator::Language::Cpp,
    )
    .expect("compile_scxml_lang");
    let _ = std::fs::remove_file(&path);
    // files[0] is the header by construction.
    output.files[0].1.clone()
}

#[test]
fn cpp_context_typedef_single_with_cpp_type() {
    // `cpp:type` provided → typedef aliases the concrete type, on
    // both the Policy struct and the user-facing class.
    let scxml = r#"<scxml xmlns="http://www.w3.org/2005/07/scxml"
                    xmlns:sce="http://sce.dev/ext"
                    xmlns:cpp="urn:sce:cpp"
                    version="1.0" name="fixture" initial="s1">
        <sce:context id="hw" cpp:type="Hardware" cpp:include="hardware.h"/>
        <state id="s1"/>
    </scxml>"#;
    let header = compile_scxml_for_test(scxml);
    assert!(
        header.contains("using HwType = Hardware;"),
        "Policy struct should expose `using HwType = Hardware;`\n{header}"
    );
    assert!(
        header.contains("using HwType = typename PolicyType::HwType;"),
        "User-facing class should re-export `using HwType = typename PolicyType::HwType;`\n{header}"
    );
}

#[test]
fn cpp_context_typedef_single_duck_typed() {
    // No `cpp:type` → typedef aliases the template parameter, which
    // preserves duck typing (consumers pick the concrete type at
    // instantiation) while still exposing `FooType` on the class.
    let scxml = r#"<scxml xmlns="http://www.w3.org/2005/07/scxml"
                    xmlns:sce="http://sce.dev/ext"
                    version="1.0" name="fixture" initial="s1">
        <sce:context id="enemy"/>
        <state id="s1"/>
    </scxml>"#;
    let header = compile_scxml_for_test(scxml);
    assert!(
        header.contains("using EnemyType = EnemyT;"),
        "Policy struct should alias the template parameter for duck typing\n{header}"
    );
    assert!(
        header.contains("using EnemyType = typename PolicyType::EnemyType;"),
        "User-facing class should re-export EnemyType\n{header}"
    );
}

#[test]
fn cpp_reserved_ids_cover_all_sm_class_type_aliases() {
    // Extractor regression guard for the template-derived
    // `parser::RESERVED_CONTEXT_IDS`. The LazyLock scans the template
    // source with the same `using {Id}Type =` pattern used here; this
    // test renders a context-less fixture and verifies that every
    // literal alias reaching the rendered output is also in the
    // reserved set.
    //
    // Because the reserved list is *derived from* the template source
    // at first access, source-vs-rendered divergence is the only way
    // this test can fail: either the derive regex missed an alias the
    // Jinja2 compiler still emits (e.g., a different spacing pattern),
    // or a future `using {Id}Type =` was reached through an unexpected
    // expansion path. The failure message names the lowercased id and
    // points at `RESERVED_CONTEXT_IDS` in `sce-build/src/parser.rs` so
    // the extractor can be widened in a single place.
    let scxml = r#"<scxml xmlns="http://www.w3.org/2005/07/scxml"
                    version="1.0" name="fixture" initial="s1">
        <state id="s1"/>
    </scxml>"#;
    let header = compile_scxml_for_test(scxml);

    let re = regex::Regex::new(r"using\s+([A-Z][A-Za-z0-9_]*)Type\s*=").unwrap();
    let mut missing: Vec<String> = Vec::new();
    for cap in re.captures_iter(&header) {
        let prefix = &cap[1];
        let lowered = prefix.to_ascii_lowercase();
        if !sce_build::parser::RESERVED_CONTEXT_IDS.contains(&lowered.as_str()) {
            missing.push(format!(
                "`using {prefix}Type` in rendered output but {lowered:?} missing from template-derived RESERVED_CONTEXT_IDS"
            ));
        }
    }
    assert!(
        missing.is_empty(),
        "Template-derived `RESERVED_CONTEXT_IDS` does not cover every alias \
         reaching the rendered header — the regex extractor in `sce-build/src/parser.rs` \
         needs widening:\n  {}",
        missing.join("\n  "),
    );
}

#[test]
fn cpp_context_typedef_multi_mixed() {
    // Two contexts, one concrete + one duck-typed. Verifies the
    // per-id rule scales uniformly — no special cases for 1 vs N.
    let scxml = r#"<scxml xmlns="http://www.w3.org/2005/07/scxml"
                    xmlns:sce="http://sce.dev/ext"
                    xmlns:cpp="urn:sce:cpp"
                    version="1.0" name="fixture" initial="s1">
        <sce:context id="combo"/>
        <sce:context id="berserk" cpp:type="doom::BerserkState" cpp:include="berserk.h"/>
        <state id="s1"/>
    </scxml>"#;
    let header = compile_scxml_for_test(scxml);
    for expected in [
        "using ComboType = ComboT;",
        "using BerserkType = doom::BerserkState;",
        "using ComboType = typename PolicyType::ComboType;",
        "using BerserkType = typename PolicyType::BerserkType;",
    ] {
        assert!(
            header.contains(expected),
            "Expected typedef `{expected}` not found in header\n{header}"
        );
    }
}

// ══════════════════════════════════════════════════════════════
// ── Phase 3: Interpolation conformance ──────────────────────
// ══════════════════════════════════════════════════════════════

#[test]
fn forge_interpolation_1d_linear_cpp() {
    assert_standalone_forge("interpolation_1d_linear", "interpolation_1d_linear.h");
}

#[test]
fn forge_interpolation_1d_linear_kotlin() {
    assert_standalone_forge_kotlin("interpolation_1d_linear", "Interpolation1dLinear.kt");
}

#[test]
fn forge_interpolation_1d_linear_rust() {
    assert_standalone_forge_rust("interpolation_1d_linear", "interpolation_1d_linear.rs");
}

#[test]
fn forge_interpolation_1d_linear_go() {
    assert_standalone_forge_go("interpolation_1d_linear", "interpolation_1d_linear.go");
}

#[test]
fn forge_interpolation_1d_linear_python() {
    assert_standalone_forge_python("interpolation_1d_linear", "interpolation_1d_linear.py");
}

#[test]
fn forge_interpolation_2d_bilinear_cpp() {
    assert_standalone_forge("interpolation_2d_bilinear", "interpolation_2d_bilinear.h");
}

#[test]
fn forge_interpolation_2d_bilinear_kotlin() {
    assert_standalone_forge_kotlin("interpolation_2d_bilinear", "Interpolation2dBilinear.kt");
}

#[test]
fn forge_interpolation_2d_bilinear_rust() {
    assert_standalone_forge_rust("interpolation_2d_bilinear", "interpolation_2d_bilinear.rs");
}

#[test]
fn forge_interpolation_2d_bilinear_go() {
    assert_standalone_forge_go("interpolation_2d_bilinear", "interpolation_2d_bilinear.go");
}

#[test]
fn forge_interpolation_2d_bilinear_python() {
    assert_standalone_forge_python("interpolation_2d_bilinear", "interpolation_2d_bilinear.py");
}

// ══════════════════════════════════════════════════════════════
// ── Phase 3: Filter conformance ─────────────────────────────
// ══════════════════════════════════════════════════════════════

#[test]
fn forge_filter_moving_average_cpp() {
    assert_standalone_forge("filter_moving_average", "filter_moving_average.h");
}

#[test]
fn forge_filter_moving_average_kotlin() {
    assert_standalone_forge_kotlin("filter_moving_average", "FilterMovingAverage.kt");
}

#[test]
fn forge_filter_moving_average_rust() {
    assert_standalone_forge_rust("filter_moving_average", "filter_moving_average.rs");
}

#[test]
fn forge_filter_moving_average_go() {
    assert_standalone_forge_go("filter_moving_average", "filter_moving_average.go");
}

#[test]
fn forge_filter_moving_average_python() {
    assert_standalone_forge_python("filter_moving_average", "filter_moving_average.py");
}

#[test]
fn forge_filter_low_pass_cpp() {
    assert_standalone_forge("filter_low_pass", "filter_low_pass.h");
}

#[test]
fn forge_filter_low_pass_kotlin() {
    assert_standalone_forge_kotlin("filter_low_pass", "FilterLowPass.kt");
}

#[test]
fn forge_filter_low_pass_rust() {
    assert_standalone_forge_rust("filter_low_pass", "filter_low_pass.rs");
}

#[test]
fn forge_filter_low_pass_go() {
    assert_standalone_forge_go("filter_low_pass", "filter_low_pass.go");
}

#[test]
fn forge_filter_low_pass_python() {
    assert_standalone_forge_python("filter_low_pass", "filter_low_pass.py");
}

#[test]
fn forge_filter_debounce_cpp() {
    assert_standalone_forge("filter_debounce", "filter_debounce.h");
}

#[test]
fn forge_filter_debounce_kotlin() {
    assert_standalone_forge_kotlin("filter_debounce", "FilterDebounce.kt");
}

#[test]
fn forge_filter_debounce_rust() {
    assert_standalone_forge_rust("filter_debounce", "filter_debounce.rs");
}

#[test]
fn forge_filter_debounce_go() {
    assert_standalone_forge_go("filter_debounce", "filter_debounce.go");
}

#[test]
fn forge_filter_debounce_python() {
    assert_standalone_forge_python("filter_debounce", "filter_debounce.py");
}

// ══════════════════════════════════════════════════════════════
// ── Phase 3: Observer conformance ───────────────────────────
// ══════════════════════════════════════════════════════════════

#[test]
fn forge_observer_coolant_cpp() {
    assert_standalone_forge("observer_coolant", "observer_coolant.h");
}

#[test]
fn forge_observer_coolant_kotlin() {
    assert_standalone_forge_kotlin("observer_coolant", "ObserverCoolant.kt");
}

#[test]
fn forge_observer_coolant_rust() {
    assert_standalone_forge_rust("observer_coolant", "observer_coolant.rs");
}

#[test]
fn forge_observer_coolant_go() {
    assert_standalone_forge_go("observer_coolant", "observer_coolant.go");
}

#[test]
fn forge_observer_coolant_python() {
    assert_standalone_forge_python("observer_coolant", "observer_coolant.py");
}

// ══════════════════════════════════════════════════════════════
// ── Phase 3: Timer conformance ──────────────────────────────
// ══════════════════════════════════════════════════════════════

#[test]
fn forge_timer_diag_scheduler_cpp() {
    assert_standalone_forge("timer_diag_scheduler", "timer_diag_scheduler.h");
}

#[test]
fn forge_timer_diag_scheduler_kotlin() {
    assert_standalone_forge_kotlin("timer_diag_scheduler", "TimerDiagScheduler.kt");
}

#[test]
fn forge_timer_diag_scheduler_rust() {
    assert_standalone_forge_rust("timer_diag_scheduler", "timer_diag_scheduler.rs");
}

#[test]
fn forge_timer_diag_scheduler_go() {
    assert_standalone_forge_go("timer_diag_scheduler", "timer_diag_scheduler.go");
}

#[test]
fn forge_timer_diag_scheduler_python() {
    assert_standalone_forge_python("timer_diag_scheduler", "timer_diag_scheduler.py");
}

// ── Golden file generator ───────────────────────────────────

/// Generate golden files for Go and Python. Run with:
/// cargo test -p sce-build --test forge_conformance forge_generate_golden -- --ignored --nocapture
#[test]
#[ignore]
fn forge_generate_golden() {
    let test_cases = [
        "transform_temperature",
        "transform_multi_output",
        "transform_bitwise",
        "lookup_engine_status",
        "lookup_gear_position",
        "lookup_single_default",
        "condition_programming",
        "condition_threshold",
        "condition_range",
        "codec_simple_frame",
        "codec_little_endian",
        "codec_subbyte",
        "validator_rpm_check",
        "validator_range_only",
        "validator_signed_roc",
        "validator_plausibility_only",
        "procedure_startup_check",
        "procedure_linear",
        "procedure_diamond",
    ];

    for name in &test_cases {
        let scxml_path = resource_dir().join(format!("{name}.scxml"));
        let content = std::fs::read_to_string(&scxml_path).unwrap();

        // Go
        let go_out = sce_build::compile_forge_from_string(
            &content,
            sce_build::DocumentLabel::symmetric(name),
            sce_build::generator::Language::Go,
        )
        .unwrap();
        let (go_filename, go_code) = &go_out.files[0];
        let go_path = expected_dir().join(go_filename);
        std::fs::write(&go_path, go_code).unwrap();
        println!("  Go: {}", go_path.display());

        // Python
        let py_out = sce_build::compile_forge_from_string(
            &content,
            sce_build::DocumentLabel::symmetric(name),
            sce_build::generator::Language::Python,
        )
        .unwrap();
        let (py_filename, py_code) = &py_out.files[0];
        let py_path = expected_dir().join(py_filename);
        std::fs::write(&py_path, py_code).unwrap();
        println!("  Py: {}", py_path.display());
    }
}

// ── Negative tests: ForgeCompileOptions validation ──────────────
//
// These guard the fail-fast contract of `resolve_imports`: when Go
// cross-file codegen is asked for but `go_module_prefix` is missing or
// malformed, `compile_forge_with_imports` must return `Err` rather than
// silently emitting broken `import "bare_name"` lines. Every case
// drives the real `crossfile_procedure_codec.scxml` fixture (which has
// an `<sce:import>`) through the public entry point so the tests catch
// regressions in either the validator or its call site.

fn crossfile_scxml() -> (String, std::path::PathBuf) {
    let scxml_path = resource_dir().join("crossfile_procedure_codec.scxml");
    let content = std::fs::read_to_string(&scxml_path)
        .unwrap_or_else(|e| panic!("Cannot read {}: {e}", scxml_path.display()));
    (content, scxml_path)
}

/// Run Go crossfile codegen against a fixture with `<sce:import>` and
/// return the expected `Err` message. Panics if codegen unexpectedly
/// succeeds, since `GeneratedOutput` deliberately does not implement
/// `Debug` (which rules out `Result::expect_err`).
fn expect_go_crossfile_err(options: sce_build::ForgeCompileOptions, test_label: &str) -> String {
    let (content, scxml_path) = crossfile_scxml();
    let base_dir = scxml_path.parent().unwrap();
    match sce_build::compile_forge_with_imports(
        &content,
        sce_build::DocumentLabel::symmetric("crossfile_procedure_codec"),
        sce_build::generator::Language::Go,
        base_dir,
        &options,
    ) {
        Ok(_) => panic!("{test_label}: Go crossfile codegen should have failed but succeeded"),
        Err(e) => e.to_string(),
    }
}

#[test]
fn forge_go_crossfile_rejects_missing_module_prefix() {
    let options = sce_build::ForgeCompileOptions::default();
    let err = expect_go_crossfile_err(options, "missing prefix");
    assert!(
        err.contains("go_module_prefix"),
        "error should mention go_module_prefix, got: {err}"
    );
}

#[test]
fn forge_go_crossfile_rejects_empty_module_prefix() {
    let options = sce_build::ForgeCompileOptions {
        go_module_prefix: Some("///".to_string()),
        ..Default::default()
    };
    let err = expect_go_crossfile_err(options, "empty prefix");
    assert!(
        err.contains("empty"),
        "error should mention empty prefix, got: {err}"
    );
}

#[test]
fn forge_go_crossfile_rejects_whitespace_in_module_prefix() {
    let options = sce_build::ForgeCompileOptions {
        go_module_prefix: Some("github.com/acme/proj generated".to_string()),
        ..Default::default()
    };
    let err = expect_go_crossfile_err(options, "whitespace prefix");
    assert!(
        err.contains("whitespace"),
        "error should mention whitespace, got: {err}"
    );
}

// ═══════════════════════════════════════════════════════════════
// ── Rust golden compile gate ─────────────────────────────────
// ═══════════════════════════════════════════════════════════════

// ═══════════════════════════════════════════════════════════════
// ── Cross-file import combination matrix ─────────────────────
// ═══════════════════════════════════════════════════════════════

/// Dynamically verify that cross-file import combinations produce
/// syntactically valid code across the listed backends. No golden files
/// needed — codegen success + syn parse (for Rust) is the gate.
///
/// Tests import patterns beyond the existing 3 golden-backed combos:
///   - filter → transform (stateful imports stateless)
///   - observer → condition (stateful imports stateless)
///   - validator → lookup (stateful imports stateless)
///
/// `languages` parameter lets per-test callers narrow the matrix when a
/// backend has not landed the importing kind yet (e.g. C11 supports
/// procedure/codec/validator imports but filter/observer arrive in
/// Phase E — adding C11 to those rows would panic on `unimplemented!`).
///
/// Each combination:
///   1. Reads the importing kind's SCXML with an <sce:import> reference
///   2. Runs compile_forge_with_imports for every listed backend
///   3. Asserts codegen succeeds and output is non-empty
///   4. For Rust output, syn::parse_file verifies syntax validity
fn assert_crossfile_codegen_languages(
    scxml_name: &str,
    languages: &[sce_build::generator::Language],
) {
    let scxml_path = resource_dir().join(format!("{scxml_name}.scxml"));
    let content = std::fs::read_to_string(&scxml_path)
        .unwrap_or_else(|e| panic!("Cannot read {}: {e}", scxml_path.display()));
    let base_dir = scxml_path.parent().unwrap();

    for lang in languages {
        let options = golden_options(*lang);
        let result = sce_build::compile_forge_with_imports(
            &content,
            sce_build::DocumentLabel::symmetric(scxml_name),
            *lang,
            base_dir,
            &options,
        );
        match result {
            Ok(output) => {
                assert!(
                    !output.files.is_empty(),
                    "{scxml_name} ({lang:?}): codegen produced no files"
                );
                // Rust-specific: verify syntax with syn
                if matches!(lang, sce_build::generator::Language::Rust) {
                    for (filename, code) in &output.files {
                        if let Err(e) = syn::parse_file(code) {
                            panic!("{scxml_name} ({lang:?}) {filename}: syn parse error: {e}");
                        }
                    }
                }
            }
            Err(e) => {
                panic!("{scxml_name} ({lang:?}): cross-file codegen failed: {e}");
            }
        }
    }
}

/// Cpp/Kotlin/Rust/Go/Python — every backend that supports the importing
/// kind. C11 is added per-test below for kinds whose C11 codegen has
/// landed (procedure/validator); filter/observer C11 paths arrive in
/// Phase E and would panic on `unimplemented!` if listed here.
const FIVE_BACKENDS: &[sce_build::generator::Language] = &[
    sce_build::generator::Language::Cpp,
    sce_build::generator::Language::Kotlin,
    sce_build::generator::Language::Rust,
    sce_build::generator::Language::Go,
    sce_build::generator::Language::Python,
];

const SIX_BACKENDS: &[sce_build::generator::Language] = &[
    sce_build::generator::Language::Cpp,
    sce_build::generator::Language::Kotlin,
    sce_build::generator::Language::Rust,
    sce_build::generator::Language::Go,
    sce_build::generator::Language::Python,
    sce_build::generator::Language::C11,
];

/// Existing golden-backed combos, verified here as codegen-all-languages too.
#[test]
fn crossfile_matrix_procedure_codec() {
    assert_crossfile_codegen_languages("crossfile_procedure_codec", SIX_BACKENDS);
}

#[test]
fn crossfile_matrix_procedure_codec_mutate() {
    assert_crossfile_codegen_languages("crossfile_procedure_codec_mutate", SIX_BACKENDS);
}

#[test]
fn crossfile_matrix_procedure_filter() {
    assert_crossfile_codegen_languages("crossfile_procedure_filter", SIX_BACKENDS);
}

#[test]
fn crossfile_matrix_validator_transform() {
    assert_crossfile_codegen_languages("crossfile_validator_transform", SIX_BACKENDS);
}

#[test]
fn crossfile_matrix_filter_transform() {
    assert_crossfile_codegen_languages("crossfile_filter_transform", FIVE_BACKENDS);
}

#[test]
fn crossfile_matrix_observer_condition() {
    assert_crossfile_codegen_languages("crossfile_observer_condition", FIVE_BACKENDS);
}

#[test]
fn crossfile_matrix_validator_codec() {
    assert_crossfile_codegen_languages("crossfile_validator_codec", SIX_BACKENDS);
}

#[test]
fn crossfile_matrix_validator_filter() {
    assert_crossfile_codegen_languages("crossfile_validator_filter", SIX_BACKENDS);
}

#[test]
fn crossfile_matrix_validator_condition() {
    assert_crossfile_codegen_languages("crossfile_validator_condition", SIX_BACKENDS);
}

#[test]
fn crossfile_matrix_validator_lookup() {
    assert_crossfile_codegen_languages("crossfile_validator_lookup", SIX_BACKENDS);
}

#[test]
fn crossfile_matrix_validator_interpolation() {
    assert_crossfile_codegen_languages("crossfile_validator_interpolation", SIX_BACKENDS);
}

// ═══════════════════════════════════════════════════════════════
// ── B5-γ parent-flags dependency emission tests ──────────────
// ═══════════════════════════════════════════════════════════════
//
// All six language closures landed; the historical gate-rejection
// helper (`assert_b5_gamma_gate_rejects`) and its 4 per-language
// rejection tests deleted at this final closure.

/// RFC §5.B B5-γ Kotlin closure: body codec with parent-flags
/// dependency emits `parentFlags: UByte` parameter on decode/encode;
/// `parent.<flag>` predicates compile to
/// `(parentFlags.toInt() and 0xNN) != 0`. `@Suppress("UNUSED_PARAMETER")`
/// on each fn declaration covers the defensive case where the
/// declaration outlives any predicate consuming it.
#[test]
fn forge_kotlin_codec_init_syn_body() {
    assert_standalone_forge_kotlin("codec_init_syn_body", "CodecInitSynBody.kt");
}

/// RFC §5.B B5-γ Kotlin closure: variant parent threading carrier value.
/// The envelope's `when (val _b = this.body)` arms call
/// `_b.body.encode(this.header)` and the companion `decode(cursor, header)`
/// passes the just-decoded header local. Mirrors the Rust + Cpp goldens.
#[test]
fn forge_kotlin_codec_init_syn_envelope() {
    assert_standalone_forge_kotlin("codec_init_syn_envelope", "CodecInitSynEnvelope.kt");
}

/// RFC §5.B B5-γ Go closure: body codec with parent-flags dependency
/// emits `parentFlags byte` parameter on `Decode<Pascal>` / `Encode`;
/// `parent.<flag>` predicates compile to `(parentFlags & 0xNN) != 0`.
/// Go function parameters tolerate being unused, so no `_ = parentFlags`
/// guard is needed (mirrors Kotlin's `@Suppress("UNUSED_PARAMETER")`
/// but the Go compiler doesn't enforce the use).
#[test]
fn forge_go_codec_init_syn_body() {
    assert_standalone_forge_go("codec_init_syn_body", "codec_init_syn_body.go");
}

/// RFC §5.B B5-γ Go closure: variant parent threading carrier value.
/// The envelope's `switch { case s.Body.X != nil ... }` arms call
/// `s.Body.X.Encode(s.Header)` and the companion
/// `Decode<Body>(cursor, Header)` passes the just-decoded PascalCase
/// local. Mirrors the Rust + Cpp + Kotlin goldens.
#[test]
fn forge_go_codec_init_syn_envelope() {
    assert_standalone_forge_go("codec_init_syn_envelope", "codec_init_syn_envelope.go");
}

/// RFC §5.B B5-γ C11 closure: body codec with parent-flags dependency
/// emits `uint8_t parent_flags` parameter on `decode`/`encode` (after
/// the existing `*cursor`/`*self` arg); `parent.<flag>` predicates
/// compile to `(parent_flags & 0xNN) != 0`. `(void)parent_flags;`
/// defensive guard suppresses `-Wunused-parameter` for codecs that
/// declare `<sce:requires-parent-flags>` without any consuming gated
/// field (mirrors Rust's `let _ = parent_flags;` and Cpp's
/// `(void)parent_flags;`).
#[test]
fn forge_c11_codec_init_syn_body() {
    assert_standalone_forge_c("codec_init_syn_body", "codec_init_syn_body.c.h");
}

/// RFC §5.B B5-γ C11 closure: variant parent threading carrier value.
/// Decode-site dispatcher reads the just-decoded carrier from
/// `out->header` (no separate local — C11 prefix decode writes
/// directly to the parent struct); encode-site dispatcher reads from
/// `self->header`. Mirrors the Rust + Cpp + Kotlin + Go goldens.
#[test]
fn forge_c11_codec_init_syn_envelope() {
    assert_standalone_forge_c("codec_init_syn_envelope", "codec_init_syn_envelope.c.h");
}

/// RFC §5.B B5-γ Python closure (final): body codec with parent-flags
/// dependency emits `parent_flags: int` parameter on `decode`/`encode`
/// (after the `cls, cursor` / `self` preceding args); `parent.<flag>`
/// predicates compile to `(parent_flags & 0xNN) != 0`. `_ = parent_flags`
/// defensive guard suppresses unused-variable warnings (mirrors Rust's
/// `let _ = parent_flags;` and Cpp's `(void)parent_flags;`).
#[test]
fn forge_python_codec_init_syn_body() {
    assert_standalone_forge_python("codec_init_syn_body", "codec_init_syn_body.py");
}

/// RFC §5.B B5-γ Python closure (final): variant parent threading
/// carrier value. Decode-site dispatcher reads the just-decoded
/// snake_case carrier local; encode-site dispatcher reads through
/// `self.<snake>`. Mirrors the Rust + Cpp + Kotlin + Go + C11 goldens.
/// This is the FINAL B5-γ closure — the per-language gate code in
/// `render_codec` and the 4 historical gate-rejection tests are
/// deleted in the same commit.
#[test]
fn forge_python_codec_init_syn_envelope() {
    assert_standalone_forge_python("codec_init_syn_envelope", "codec_init_syn_envelope.py");
}

// ═══════════════════════════════════════════════════════════════
// ── B5-δ Surfaces D + E + F emission tests ───────────────────
// ═══════════════════════════════════════════════════════════════
//
// All six backends share the existing length-ref decode/encode
// helper machinery — the helper now detects sibling `present_if`
// to unwrap the gated Optional (Surface E) and applies
// `field.length_arith` to the byte count (Surface F). Surface D
// is "free" (the streaming helper reads `sibling as usize` which
// works for both Fixed and Vle siblings); fixture pin proves it.

/// RFC §5.B B5-δ Surfaces D + E (Kotlin): Init body cookie codec.
/// `cookieSize: UShort?` (gated VLE u16); `cookie: ByteArray?` (gated
/// length-ref bytes). Helper unwraps the sibling `!!.toInt()` inside
/// the gated branch.
#[test]
fn forge_kotlin_codec_init_cookie_body() {
    assert_standalone_forge_kotlin("codec_init_cookie_body", "CodecInitCookieBody.kt");
}

/// RFC §5.B B5-δ Surface F (Kotlin): Scout/Hello/Init zid codec.
/// `length-arith="+1"` emits `(zidLenM1.toInt() + 1)` for the byte
/// count.
#[test]
fn forge_kotlin_codec_scout_zid_body() {
    assert_standalone_forge_kotlin("codec_scout_zid_body", "CodecScoutZidBody.kt");
}

/// RFC §5.B B5-δ Surfaces D + E (Go): Init body cookie codec.
/// `CookieSize *uint16` (pointer = presence wrapper for VLE u16);
/// `Cookie []byte` (slice nilness encodes presence). Helper deref
/// emits `int(*CookieSize)` inside the gated branch.
#[test]
fn forge_go_codec_init_cookie_body() {
    assert_standalone_forge_go("codec_init_cookie_body", "codec_init_cookie_body.go");
}

/// RFC §5.B B5-δ Surface F (Go): Scout/Hello/Init zid codec.
/// `length-arith="+1"` emits `(int(ZidLenM1) + 1)` for the byte count.
#[test]
fn forge_go_codec_scout_zid_body() {
    assert_standalone_forge_go("codec_scout_zid_body", "codec_scout_zid_body.go");
}

/// RFC §5.B B5-δ Surfaces D + E (C11): Init body cookie codec.
/// C11 has no Option wrapper — sibling `cookie_size` is always-bound
/// on the struct (zero on absent branch). Helper reads through
/// `out->cookie_size` regardless of gating; the carrier bit is the
/// presence source.
#[test]
fn forge_c11_codec_init_cookie_body() {
    assert_standalone_forge_c("codec_init_cookie_body", "codec_init_cookie_body.c.h");
}

/// RFC §5.B B5-δ Surface F (C11): Scout/Hello/Init zid codec.
/// `length-arith="+1"` emits `_n = (size_t)((int64_t)out->zid_len_m1 + 1)`
/// for decode; the encode-loop's upper bound widens symmetrically to
/// `_bi < (size_t)((int64_t)self->zid_len_m1 + 1)` so the wire-correct
/// number of bytes is written.
#[test]
fn forge_c11_codec_scout_zid_body() {
    assert_standalone_forge_c("codec_scout_zid_body", "codec_scout_zid_body.c.h");
}

/// RFC §5.B B5-δ Surfaces D + E (Python): Init body cookie codec.
/// `cookie_size: Optional[int]` and `cookie: Optional[bytes]` — inside
/// the gated branch the int local is guaranteed non-None by the same
/// predicate. Helper reads `cookie_size` directly (no unwrap syntax).
#[test]
fn forge_python_codec_init_cookie_body() {
    assert_standalone_forge_python("codec_init_cookie_body", "codec_init_cookie_body.py");
}

/// RFC §5.B B5-δ Surface F (Python): Scout/Hello/Init zid codec.
/// `length-arith="+1"` emits `_n = (zid_len_m1 + 1)` for the byte
/// count — Python's arbitrary-precision int handles `+1` without
/// overflow.
#[test]
fn forge_python_codec_scout_zid_body() {
    assert_standalone_forge_python("codec_scout_zid_body", "codec_scout_zid_body.py");
}

/// RFC §5.B B5-δ Surface F validation: standalone `sce:length-arith`
/// without `sce:length-field` rejects with `validation/invalid-attribute`
/// — the offset has no source to apply to.
#[test]
fn forge_codec_length_arith_without_length_field_rejects() {
    use sce_build::forge::error::{ForgeError, ValidationError};

    let scxml = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       xmlns:sce="http://sce.dev/ext"
       sce:kind="codec" sce:default-endian="big" name="bad_arith">
  <datamodel>
    <sce:field id="payload" sce:type="bytes" sce:byte="0" sce:bit-size="length-ref"
               sce:length-arith="+1" sce:max-size="16"/>
  </datamodel>
</scxml>"#;
    let result = sce_build::compile_forge_with_imports(
        scxml,
        sce_build::DocumentLabel::symmetric("bad_arith"),
        sce_build::generator::Language::Rust,
        &resource_dir(),
        &sce_build::ForgeCompileOptions::default(),
    );
    let err = match result {
        Ok(_) => panic!("length-arith without length-field must reject"),
        Err(e) => e,
    };
    assert!(
        matches!(
            err.error,
            ForgeError::Validation(ValidationError::InvalidAttribute { ref attr, .. })
                if attr == "sce:length-arith"
        ),
        "must surface as InvalidAttribute on sce:length-arith; got: {:?}",
        err.error
    );
}

/// RFC §5.B B5-δ Surface F validation: arithmetic offset must be ±1.
/// v1 grammar restricts the value (parser rejects 0 and `|x|>1`);
/// widening defers to a reachable consumer.
#[test]
fn forge_codec_length_arith_out_of_range_rejects() {
    use sce_build::forge::error::{ForgeError, ValidationError};

    for bad in ["0", "+2", "-2", "5", "-3"] {
        let scxml = format!(
            r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       xmlns:sce="http://sce.dev/ext"
       sce:kind="codec" sce:default-endian="big" name="bad_arith_range">
  <datamodel>
    <sce:field id="len" sce:type="uint8" sce:byte="0" sce:bit-size="8"/>
    <sce:field id="payload" sce:type="bytes" sce:byte="1" sce:bit-size="length-ref"
               sce:length-field="len" sce:length-arith="{bad}" sce:max-size="16"/>
  </datamodel>
</scxml>"#
        );
        let result = sce_build::compile_forge_with_imports(
            &scxml,
            sce_build::DocumentLabel::symmetric("bad_arith_range"),
            sce_build::generator::Language::Rust,
            &resource_dir(),
            &sce_build::ForgeCompileOptions::default(),
        );
        let err = match result {
            Ok(_) => panic!("length-arith={bad} must reject (v1 limits to ±1)"),
            Err(e) => e,
        };
        assert!(
            matches!(
                err.error,
                ForgeError::Validation(ValidationError::InvalidAttribute { ref attr, .. })
                    if attr == "sce:length-arith"
            ),
            "length-arith={bad} must surface as InvalidAttribute; got: {:?}",
            err.error
        );
    }
}

/// RFC §5.B B5-γ: cross-codec parent-flag layout mismatch — the body
/// codec declares `<sce:flag name="S" bit="6"/>` but the parent codec's
/// `<sce:flags id="header">` places 'S' at a DIFFERENT bit. The
/// codegen-time cross-codec validator surfaces
/// `codec/parent-flag-mismatch` with a precise reason naming the
/// offending flag and both bit positions.
#[test]
fn forge_codec_parent_flag_bit_mismatch_rejects() {
    use sce_build::forge::error::{ForgeError, ValidationError};

    let parent_scxml = r##"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       xmlns:sce="http://sce.dev/ext"
       sce:kind="codec" sce:default-endian="big" name="codec_init_syn_envelope_mismatch">
  <sce:import src="codec_init_syn_body.scxml" kind="codec" as="codec_init_syn_body"/>
  <datamodel>
    <sce:flags id="header" sce:type="uint8" sce:byte="0" sce:bit-size="8">
      <sce:flag name="mid" bit="0" width="5"/>
      <sce:flag name="S"   bit="5"/>
    </sce:flags>
    <sce:variant tag="header.mid">
      <sce:arm value="0x01" type="codec_init_syn_body"/>
      <sce:default type="codec_init_syn_body"/>
    </sce:variant>
  </datamodel>
</scxml>"##;
    let result = sce_build::compile_forge_with_imports(
        parent_scxml,
        sce_build::DocumentLabel::symmetric("codec_init_syn_envelope_mismatch"),
        sce_build::generator::Language::Rust,
        &resource_dir(),
        &sce_build::ForgeCompileOptions::default(),
    );
    let err = match result {
        Ok(_) => panic!(
            "parent flag bit-mismatch must reject with codec/parent-flag-chain-bit-drift"
        ),
        Err(e) => e,
    };
    // RFC B5-ν consumer-parity (claudedocs/rfc-b5-nu-consumer-parity.md)
    // unified Terminal + Forwarding bit-drift onto a single typed
    // variant. The legacy CodecParentFlagMismatch path stays for
    // wrong-shape / wrong-type / flag-name-not-found cases; pure bit-
    // position divergence now routes to the chain-bit-drift variant.
    assert!(
        matches!(
            &err.error,
            ForgeError::Validation(ValidationError::CodecParentFlagChainBitDrift {
                ref flag,
                ref body_bit,
                ref parent_bit,
                ..
            }) if flag == "S" && *body_bit == 6 && *parent_bit == 5
        ),
        "must surface CodecParentFlagChainBitDrift; got: {:?}",
        err.error
    );
}

// ═══════════════════════════════════════════════════════════════
// ── Rust golden compile gate ─────────────────────────────────
// ═══════════════════════════════════════════════════════════════

/// Verify that every Rust golden file in tests/forge/expected/*.rs is
/// syntactically valid Rust via `syn::parse_file`.
///
/// This catches template regressions that produce byte-identical golden
/// diffs but broken syntax (e.g. UPDATE_GOLDEN accepts output that
/// contains a misplaced semicolon). It is faster and cheaper than
/// standing up rustc, and catches the most common template-layer bugs.
#[test]
fn rust_golden_syn_gate() {
    let dir = expected_dir();
    let mut failures: Vec<String> = Vec::new();
    let mut count = 0u32;

    for entry in std::fs::read_dir(&dir).expect("read expected dir") {
        let entry = entry.expect("dir entry");
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("rs") {
            continue;
        }
        count += 1;
        let code = std::fs::read_to_string(&path)
            .unwrap_or_else(|e| panic!("read {}: {e}", path.display()));
        if let Err(e) = syn::parse_file(&code) {
            failures.push(format!(
                "{}: {e}",
                path.file_name().unwrap().to_string_lossy()
            ));
        }
    }

    assert!(count > 0, "no .rs golden files found in {}", dir.display());

    if !failures.is_empty() {
        panic!(
            "Rust golden syn gate failed ({}/{count} files broken):\n  {}",
            failures.len(),
            failures.join("\n  ")
        );
    }
}

// ────────────────────────────────────────────────────────────────────
// RFC variant-default-overlay Atomic A — deploy.yaml-overrides-SCXML
// tests.
//
// The wire-spec invariants (bit positions, MID values) stay in the
// SCXML; the *choice* of which arm a default-constructed instance
// dispatches to moves into the deploy overlay so consumers pick
// their own default without forking the codec. These tests exercise
// the three observable outcomes:
//
//   1. Overlay names a codec + valid arm value → that arm becomes
//      the Default-trait starting arm, overriding any SCXML-side
//      `default="true"` marker.
//   2. Overlay is absent (compile-without-deploy path) → SCXML's
//      own `default="true"` marker carries the choice unchanged.
//      Preserves backward compat for the 107 existing
//      `compile_forge_with_imports` call sites.
//   3. Overlay names a value that no `<sce:arm value=...>` declares
//      → `codec/variant-default-overlay-arm-not-declared` fires
//      with a `Fix::ReplaceOneOf` carrying the declared arm values.
// ────────────────────────────────────────────────────────────────────

/// Build a minimal deploy YAML carrying only `variant_defaults:`.
/// `topology: {}` is the smallest legal value (HashMap<_, _>::new()
/// shape) — overlay-only tests do not need machines/platforms.
fn deploy_with_variant_defaults(entries: &[(&str, u64)]) -> String {
    let mut yaml = String::from("version: \"1.0\"\ntopology: {}\nvariant_defaults:\n");
    for (codec, arm_value) in entries {
        yaml.push_str(&format!("  {codec}: {arm_value:#x}\n"));
    }
    yaml
}

/// Compile `codec_variant_default_marker` with no deploy overlay —
/// the SCXML's `<sce:arm value="0x02" default="true"/>` marker is
/// the only signal. The generated Rust output bakes 0x02 into the
/// `Default::default()` call site for the outer codec's variant.
#[test]
fn forge_variant_default_overlay_absent_falls_back_to_scxml_marker() {
    let scxml_path = resource_dir().join("codec_variant_default_marker.scxml");
    let content = std::fs::read_to_string(&scxml_path).expect("read fixture");

    let output = sce_build::compile_forge_with_imports(
        &content,
        sce_build::DocumentLabel::symmetric("codec_variant_default_marker"),
        sce_build::generator::Language::Rust,
        &resource_dir(),
        &sce_build::ForgeCompileOptions::default(),
    )
    .expect("compile without deploy must succeed — SCXML marker is sole signal");

    let (_, generated) = &output.files[0];
    // The SCXML marker picks arm 0x02 (codec_default_marker_arm_b)
    // as Default. Verify the generated Default impl references arm_b's
    // body type — the C++/Kotlin equivalent literals would also work,
    // but the Rust template emits `CodecDefaultMarkerArmB::default()`
    // verbatim in the variant's Default impl.
    assert!(
        generated.contains("CodecDefaultMarkerArmB"),
        "SCXML default marker selects arm 0x02 (arm_b); generated code must \
         reference CodecDefaultMarkerArmB. Output excerpt:\n{}",
        &generated[..generated.len().min(2000)]
    );
}

/// Parse `codec_variant_default_marker` and apply a deploy overlay
/// picking arm 0x01. Verify the parsed IR's `is_default` flag flips
/// from the SCXML-declared arm (0x02) to the overlay-declared arm
/// (0x01). This bypasses codegen so the test doesn't need the
/// orchestrator-level import resolution — the overlay step is
/// itself an IR mutation and is observable at the parser exit.
#[test]
fn forge_variant_default_overlay_overrides_scxml_marker() {
    use sce_build::forge::model::ForgeDocument;
    use sce_build::forge::parser::parse_forge_with_imports;
    use sce_build::forge::variant_default_overlay::apply_variant_default_overlay;
    use sce_build::mesh::deploy::parse_deploy_str;

    let scxml_path = resource_dir().join("codec_variant_default_marker.scxml");
    let content = std::fs::read_to_string(&scxml_path).expect("read fixture");

    let mut parsed = parse_forge_with_imports(
        &content,
        sce_build::DocumentLabel::symmetric("codec_variant_default_marker"),
    )
    .expect("parse succeeds")
    .expect("forge document parsed");

    // SCXML side baseline: arm 0x02 carries `default="true"`.
    {
        let variant = match &parsed.document {
            ForgeDocument::Codec(c) => c.variant.as_ref().expect("codec has variant"),
            _ => panic!("expected codec document"),
        };
        let arm_a = variant.arms.iter().find(|a| a.value == 0x01).unwrap();
        let arm_b = variant.arms.iter().find(|a| a.value == 0x02).unwrap();
        assert!(!arm_a.is_default, "pre-overlay: arm_a (0x01) not default");
        assert!(arm_b.is_default, "pre-overlay: arm_b (0x02) is default");
    }

    let deploy_yaml = deploy_with_variant_defaults(&[("codec_variant_default_marker", 0x01)]);
    let deploy = parse_deploy_str(&deploy_yaml).expect("deploy parses");

    apply_variant_default_overlay(
        &mut parsed.document,
        &deploy,
        "codec_variant_default_marker",
    )
    .expect("overlay applies cleanly when arm value is declared");

    // Post-overlay: arm_a (0x01) becomes default, arm_b (0x02) loses
    // its SCXML-side default flag.
    let variant = match &parsed.document {
        ForgeDocument::Codec(c) => c.variant.as_ref().expect("codec still has variant"),
        _ => panic!("expected codec document"),
    };
    let arm_a = variant.arms.iter().find(|a| a.value == 0x01).unwrap();
    let arm_b = variant.arms.iter().find(|a| a.value == 0x02).unwrap();
    assert!(
        arm_a.is_default,
        "overlay 0x01 must set is_default on arm_a"
    );
    assert!(
        !arm_b.is_default,
        "overlay 0x01 must clear is_default on arm_b (SCXML marker overridden)"
    );
}

/// Compile `codec_variant_default_marker` with a deploy overlay whose
/// arm value is not declared by any `<sce:arm value=...>` —
/// `codec/variant-default-overlay-arm-not-declared` fires with the
/// declared arm values as a `Fix::ReplaceOneOf` candidate set.
#[test]
fn forge_variant_default_overlay_unknown_arm_rejects() {
    use sce_build::forge::error::{ForgeError, ValidationError};
    use sce_build::mesh::deploy::parse_deploy_str;

    let scxml_path = resource_dir().join("codec_variant_default_marker.scxml");
    let content = std::fs::read_to_string(&scxml_path).expect("read fixture");

    // 0xff is not declared by any arm — the codec's variant only
    // names 0x01 and 0x02.
    let deploy_yaml = deploy_with_variant_defaults(&[("codec_variant_default_marker", 0xff)]);
    let deploy = parse_deploy_str(&deploy_yaml).expect("deploy parses");

    let result = sce_build::compile_forge_with_deploy(
        &content,
        sce_build::DocumentLabel::symmetric("codec_variant_default_marker"),
        sce_build::generator::Language::Rust,
        Some(&deploy),
        None,
    );
    let err = match result {
        Ok(_) => panic!("overlay names arm value 0xff that no <sce:arm> declares — must reject"),
        Err(e) => e,
    };

    assert!(
        matches!(
            err.error,
            ForgeError::Validation(ValidationError::CodecVariantDefaultOverlayArmNotDeclared {
                ref codec,
                overlay_arm_value: 0xff,
                ref declared_arms,
            }) if codec == "codec_variant_default_marker"
                && declared_arms == &[0x01u64, 0x02u64]
        ),
        "must surface as ValidationError::CodecVariantDefaultOverlayArmNotDeclared \
         with declared_arms = [0x01, 0x02]; got: {:?}",
        err.error
    );
}

// ── RFC §5.B B5-ν — variant parent-tag dispatch (4 parser/validator tests) ─
//
// Phase A covers parser + validators + diagnostics. Codegen (Phase B)
// is gated behind a typed `GenerateError::UnsupportedFeature` so
// authors get a clear "infrastructure ready, codegen pending" signal
// rather than broken output. Tests below exercise the Phase A surface;
// the codegen guard's positive case is covered by
// `forge_b5_nu_codegen_phase_b_guard_fires`.

/// B5-ν Q-1 (a): a codec authoring `<sce:variant tag="parent.M">`
/// without a corresponding `<sce:requires-parent-flags>` block must
/// surface `codec/variant-parent-tag-without-requires-parent-flags`.
#[test]
fn forge_b5_nu_parent_tag_without_rpf_rejects() {
    use sce_build::forge::error::{ForgeError, ValidationError};
    use sce_build::forge::parser::parse_forge_with_imports;

    let content = r#"<?xml version="1.0"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       xmlns:sce="http://sce.dev/ext"
       sce:kind="codec" sce:default-endian="big">
  <datamodel>
    <sce:field id="payload" sce:type="uint8" sce:byte="0" sce:bit-size="8"/>
    <sce:variant tag="parent.M">
      <sce:arm value="0x00" type="codec_zenoh_keyexpr_nonlocal" default="true"/>
      <sce:arm value="0x01" type="codec_zenoh_keyexpr_local"/>
    </sce:variant>
  </datamodel>
</scxml>"#;

    let result = parse_forge_with_imports(
        content,
        sce_build::DocumentLabel::symmetric("codec_keyexpr_no_rpf"),
    );
    let err = result.expect_err("parser must reject parent-tag without rpf");
    assert!(
        matches!(
            err.error,
            ForgeError::Validation(ValidationError::CodecVariantParentTagWithoutRequiresParentFlags {
                ref codec,
                ref tag,
            }) if codec == "codec_keyexpr_no_rpf" && tag == "parent.M"
        ),
        "got: {:?}",
        err.error
    );
}

/// B5-ν Q-1 (b): a codec authoring `<sce:variant tag="parent.X">`
/// whose `<sce:requires-parent-flags>` doesn't declare `X` must
/// surface `codec/variant-parent-tag-flag-not-declared` with
/// declared rpf flag names as the candidate set.
#[test]
fn forge_b5_nu_parent_tag_flag_not_in_rpf_rejects() {
    use sce_build::forge::error::{ForgeError, ValidationError};
    use sce_build::forge::parser::parse_forge_with_imports;

    let content = r#"<?xml version="1.0"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       xmlns:sce="http://sce.dev/ext"
       sce:kind="codec" sce:default-endian="big">
  <sce:requires-parent-flags carrier="header">
    <sce:flag name="M" bit="6"/>
    <sce:flag name="N" bit="5"/>
  </sce:requires-parent-flags>
  <datamodel>
    <sce:field id="payload" sce:type="uint8" sce:byte="0" sce:bit-size="8"/>
    <sce:variant tag="parent.X">
      <sce:arm value="0x00" type="codec_zenoh_keyexpr_nonlocal" default="true"/>
      <sce:arm value="0x01" type="codec_zenoh_keyexpr_local"/>
    </sce:variant>
  </datamodel>
</scxml>"#;

    let result = parse_forge_with_imports(
        content,
        sce_build::DocumentLabel::symmetric("codec_keyexpr_wrong_flag"),
    );
    let err = result.expect_err("parser must reject unknown parent-tag flag");
    assert!(
        matches!(
            err.error,
            ForgeError::Validation(ValidationError::CodecVariantParentTagFlagNotDeclared {
                ref codec,
                ref flag,
                ref carrier,
                ref declared_flags,
            }) if codec == "codec_keyexpr_wrong_flag"
                && flag == "X"
                && carrier == "header"
                && declared_flags == &vec!["M".to_string(), "N".to_string()]
        ),
        "got: {:?}",
        err.error
    );
}

// RFC §5.B B5-ν: Phase A's `forge_b5_nu_codegen_phase_b_guard_fires`
// proved the parser+validator entry rejected unimplemented codegen
// cleanly. Phase B drops the guard and replaces this assertion with a
// positive round-trip test (`forge_b5_nu_round_trip_local_nonlocal`
// below) that exercises the full encode → decode → encode chain on a
// 2-arm parent-tag variant. The negative-path coverage now lives in
// the four parse-time / cross-doc diagnostics:
// `forge_b5_nu_parent_tag_without_rpf_rejects`,
// `forge_b5_nu_parent_tag_flag_not_in_rpf_rejects`,
// `forge_b5_nu_parent_flag_derivation_conflict_rejects` (Q-3),
// `forge_b5_nu_parent_tag_variant_before_carrier_rejects` (Q-6).

/// Write a B5-ν cross-codec fixture set to a fresh temp directory and
/// return the directory + the parent codec's SCXML path. Layout:
/// - `codec_b5nu_parent.scxml`     — parent envelope (header flags + embed)
/// - `codec_b5nu_keyexpr.scxml`    — parent-tag-dispatched variant carrier
/// - `codec_b5nu_local.scxml`      — Local arm body (1-byte payload)
/// - `codec_b5nu_nonlocal.scxml`   — Nonlocal arm body (2-byte payload)
/// Each fixture is independently parseable; cross-doc validation runs
/// when the parent resolves its imports through the temp directory.
fn b5_nu_write_round_trip_fixture(
    parent_extra_field: &str,
    parent_header_static: &str,
) -> (std::path::PathBuf, std::path::PathBuf) {
    use std::sync::atomic::{AtomicU64, Ordering};
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let id = COUNTER.fetch_add(1, Ordering::Relaxed);
    let pid = std::process::id();
    let dir = std::env::temp_dir().join(format!("sce_b5nu_fixture_{pid}_{id}"));
    std::fs::create_dir_all(&dir).expect("mkdir fixture");

    let parent = format!(
        r#"<?xml version="1.0"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       xmlns:sce="http://sce.dev/ext"
       sce:kind="codec" sce:codec-id="codec_b5nu_parent" sce:default-endian="big">
  <sce:import src="codec_b5nu_keyexpr.scxml" kind="codec" as="codec_b5nu_keyexpr"/>
  <datamodel>
    <sce:flags id="header" sce:type="uint8" sce:byte="0" sce:bit-size="8">
      <sce:flag name="mid" bit="0" width="5" value="0x1d"/>
      <sce:flag name="M" bit="6"{parent_header_static}/>
    </sce:flags>
    {parent_extra_field}
    <sce:embed id="key" type="codec_b5nu_keyexpr" sce:byte="1"/>
  </datamodel>
</scxml>"#
    );

    let keyexpr = r#"<?xml version="1.0"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       xmlns:sce="http://sce.dev/ext"
       sce:kind="codec" sce:codec-id="codec_b5nu_keyexpr" sce:default-endian="big">
  <sce:import src="codec_b5nu_local.scxml" kind="codec" as="codec_b5nu_local"/>
  <sce:import src="codec_b5nu_nonlocal.scxml" kind="codec" as="codec_b5nu_nonlocal"/>
  <sce:requires-parent-flags carrier="header">
    <sce:flag name="M" bit="6"/>
  </sce:requires-parent-flags>
  <datamodel>
    <sce:variant tag="parent.M">
      <sce:arm value="0x00" type="codec_b5nu_nonlocal" default="true"/>
      <sce:arm value="0x01" type="codec_b5nu_local"/>
    </sce:variant>
  </datamodel>
</scxml>"#;

    let local = r#"<?xml version="1.0"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       xmlns:sce="http://sce.dev/ext"
       sce:kind="codec" sce:codec-id="codec_b5nu_local" sce:default-endian="big">
  <datamodel>
    <sce:field id="payload" sce:type="uint8" sce:byte="0" sce:bit-size="8"/>
  </datamodel>
</scxml>"#;

    let nonlocal = r#"<?xml version="1.0"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       xmlns:sce="http://sce.dev/ext"
       sce:kind="codec" sce:codec-id="codec_b5nu_nonlocal" sce:default-endian="big">
  <datamodel>
    <sce:field id="payload" sce:type="uint16" sce:byte="0" sce:bit-size="16"/>
  </datamodel>
</scxml>"#;

    std::fs::write(dir.join("codec_b5nu_parent.scxml"), &parent).expect("write parent");
    std::fs::write(dir.join("codec_b5nu_keyexpr.scxml"), keyexpr).expect("write keyexpr");
    std::fs::write(dir.join("codec_b5nu_local.scxml"), local).expect("write local");
    std::fs::write(dir.join("codec_b5nu_nonlocal.scxml"), nonlocal).expect("write nonlocal");

    let parent_path = dir.join("codec_b5nu_parent.scxml");
    (dir, parent_path)
}

/// B5-ν Phase B positive round-trip — a parent codec embeds a parent-tag-
/// dispatched variant codec. Codegen succeeds for Rust; emitted code
/// contains the `parent_flags`-driven decode dispatch and the encode-
/// side `_derived_header` local that OR's the active arm's bit into
/// the carrier byte. This exercises the full Phase B surface end-to-
/// end (variant_obj Parent branch + `b5_nu_derivation_block` +
/// `inject_b5_nu_carrier_suffix`).
#[test]
fn forge_b5_nu_round_trip_local_nonlocal_rust() {
    let (dir, parent_path) = b5_nu_write_round_trip_fixture("", "");

    let content = std::fs::read_to_string(&parent_path).expect("read parent");
    let output = sce_build::compile_forge_with_imports(
        &content,
        sce_build::DocumentLabel::symmetric("codec_b5nu_parent"),
        sce_build::generator::Language::Rust,
        &dir,
        &sce_build::ForgeCompileOptions::default(),
    )
    .expect("Phase B codegen must succeed for valid parent-tag variant");

    // Parent codec's encode emits a `_derived_header` local before any
    // byte append, and the carrier emit ORs it into `self.header`.
    let parent_rust = output
        .files
        .iter()
        .find(|(name, _)| name.contains("codec_b5nu_parent"))
        .map(|(_, body)| body.clone())
        .expect("parent codec emit");
    assert!(
        parent_rust.contains("let _derived_header: u8 ="),
        "parent encode must declare `_derived_header: u8`\n{parent_rust}"
    );
    assert!(
        parent_rust.contains("CodecB5nuKeyexprVariant::CodecB5nuLocal(_) => 0x40u8"),
        "match must shift Local arm value 0x01 into bit 6 (0x40)\n{parent_rust}"
    );
    assert!(
        parent_rust.contains("CodecB5nuKeyexprVariant::CodecB5nuNonlocal(_) => 0x00u8"),
        "match must shift Nonlocal arm value 0x00 into bit 6 (0x00)\n{parent_rust}"
    );
    assert!(
        parent_rust.contains("r.push(self.header | _derived_header);"),
        "carrier emit must OR in _derived_header\n{parent_rust}"
    );

    // Child variant codec's decode dispatches on parent_flags.
    let keyexpr_rust = sce_build::compile_forge_with_imports(
        &std::fs::read_to_string(dir.join("codec_b5nu_keyexpr.scxml")).unwrap(),
        sce_build::DocumentLabel::symmetric("codec_b5nu_keyexpr"),
        sce_build::generator::Language::Rust,
        &dir,
        &sce_build::ForgeCompileOptions::default(),
    )
    .expect("keyexpr codegen must succeed")
    .files
    .iter()
    .find(|(name, _)| name.contains("codec_b5nu_keyexpr"))
    .map(|(_, body)| body.clone())
    .expect("keyexpr codec emit");
    assert!(
        keyexpr_rust.contains("match ((parent_flags >> 6) & (0x01 as u8)) as u8 {"),
        "child variant decode must dispatch on `(parent_flags >> 6) & 0x01`\n{keyexpr_rust}"
    );

    let _ = std::fs::remove_dir_all(&dir);
}

/// B5-ν Q-3 cross-doc: parent's `<sce:flag name="M" value="1"/>` static
/// constant conflicts with the embedded codec's parent-tag derivation.
/// Cross-doc validator surfaces `codec/parent-flag-derivation-conflict`.
#[test]
fn forge_b5_nu_parent_flag_derivation_conflict_rejects() {
    use sce_build::forge::error::{ForgeError, ValidationError};

    let (dir, parent_path) = b5_nu_write_round_trip_fixture(
        "", // no extra middle field
        r#" value="0x01""#, // STATIC value= on M flag conflicts with derivation
    );

    let content = std::fs::read_to_string(&parent_path).expect("read parent");
    let err = match sce_build::compile_forge_with_imports(
        &content,
        sce_build::DocumentLabel::symmetric("codec_b5nu_parent"),
        sce_build::generator::Language::Rust,
        &dir,
        &sce_build::ForgeCompileOptions::default(),
    ) {
        Ok(_) => panic!("cross-doc validator must reject Q-3 conflict"),
        Err(e) => e,
    };

    assert!(
        matches!(
            err.error,
            ForgeError::Validation(
                ValidationError::CodecParentFlagDerivationConflict {
                    ref parent_codec,
                    ref embedded_codec,
                    ref flag,
                    ..
                }
            ) if parent_codec == "codec_b5nu_parent"
                && embedded_codec == "codec_b5nu_keyexpr"
                && flag == "M"
        ),
        "got: {:?}",
        err.error
    );
    let _ = std::fs::remove_dir_all(&dir);
}

/// B5-ν Q-6 cross-doc: a parent that declares its B5-ν embed field
/// BEFORE the flags carrier field violates the carrier-before-variant
/// declaration order rule. Cross-doc validator surfaces
/// `codec/parent-tag-variant-before-carrier`.
#[test]
fn forge_b5_nu_parent_tag_variant_before_carrier_rejects() {
    use sce_build::forge::error::{ForgeError, ValidationError};
    use std::sync::atomic::{AtomicU64, Ordering};

    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let id = COUNTER.fetch_add(1, Ordering::Relaxed);
    let pid = std::process::id();
    let dir = std::env::temp_dir().join(format!("sce_b5nu_q6_{pid}_{id}"));
    std::fs::create_dir_all(&dir).expect("mkdir fixture");

    // Parent puts the embed field BEFORE the flags-carrier header field
    // — Q-6 order violation. The embed sits at byte 0, header at byte 2
    // (after the embed's worst-case width). Keyexpr/local/nonlocal
    // fixtures reuse the shape from the round-trip helper, written
    // inline here so the helper's defaults stay consistent.
    let parent = r#"<?xml version="1.0"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       xmlns:sce="http://sce.dev/ext"
       sce:kind="codec" sce:codec-id="codec_b5nu_q6_parent" sce:default-endian="big">
  <sce:import src="codec_b5nu_q6_keyexpr.scxml" kind="codec" as="codec_b5nu_q6_keyexpr"/>
  <datamodel>
    <sce:embed id="key" type="codec_b5nu_q6_keyexpr" sce:byte="0"/>
    <sce:flags id="header" sce:type="uint8" sce:byte="2" sce:bit-size="8">
      <sce:flag name="M" bit="6"/>
    </sce:flags>
  </datamodel>
</scxml>"#;
    let keyexpr = r#"<?xml version="1.0"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       xmlns:sce="http://sce.dev/ext"
       sce:kind="codec" sce:codec-id="codec_b5nu_q6_keyexpr" sce:default-endian="big">
  <sce:import src="codec_b5nu_q6_local.scxml" kind="codec" as="codec_b5nu_q6_local"/>
  <sce:import src="codec_b5nu_q6_nonlocal.scxml" kind="codec" as="codec_b5nu_q6_nonlocal"/>
  <sce:requires-parent-flags carrier="header">
    <sce:flag name="M" bit="6"/>
  </sce:requires-parent-flags>
  <datamodel>
    <sce:variant tag="parent.M">
      <sce:arm value="0x00" type="codec_b5nu_q6_nonlocal" default="true"/>
      <sce:arm value="0x01" type="codec_b5nu_q6_local"/>
    </sce:variant>
  </datamodel>
</scxml>"#;
    let local = r#"<?xml version="1.0"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       xmlns:sce="http://sce.dev/ext"
       sce:kind="codec" sce:codec-id="codec_b5nu_q6_local" sce:default-endian="big">
  <datamodel>
    <sce:field id="payload" sce:type="uint8" sce:byte="0" sce:bit-size="8"/>
  </datamodel>
</scxml>"#;
    let nonlocal = r#"<?xml version="1.0"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       xmlns:sce="http://sce.dev/ext"
       sce:kind="codec" sce:codec-id="codec_b5nu_q6_nonlocal" sce:default-endian="big">
  <datamodel>
    <sce:field id="payload" sce:type="uint16" sce:byte="0" sce:bit-size="16"/>
  </datamodel>
</scxml>"#;
    std::fs::write(dir.join("codec_b5nu_q6_parent.scxml"), parent).expect("write parent");
    std::fs::write(dir.join("codec_b5nu_q6_keyexpr.scxml"), keyexpr).expect("write keyexpr");
    std::fs::write(dir.join("codec_b5nu_q6_local.scxml"), local).expect("write local");
    std::fs::write(dir.join("codec_b5nu_q6_nonlocal.scxml"), nonlocal).expect("write nonlocal");

    let err = match sce_build::compile_forge_with_imports(
        parent,
        sce_build::DocumentLabel::symmetric("codec_b5nu_q6_parent"),
        sce_build::generator::Language::Rust,
        &dir,
        &sce_build::ForgeCompileOptions::default(),
    ) {
        Ok(_) => panic!("cross-doc validator must reject Q-6 order violation"),
        Err(e) => e,
    };
    assert!(
        matches!(
            err.error,
            ForgeError::Validation(
                ValidationError::CodecVariantParentTagBeforeCarrier {
                    ref parent_codec,
                    ref embedded_codec,
                    ref carrier,
                    ..
                }
            ) if parent_codec == "codec_b5nu_q6_parent"
                && embedded_codec == "codec_b5nu_q6_keyexpr"
                && carrier == "header"
        ),
        "got: {:?}",
        err.error
    );
    let _ = std::fs::remove_dir_all(&dir);
}

// RFC §5.B B5-ν consumer-parity (claudedocs/rfc-b5-nu-consumer-parity.md)
// — three independent gaps surfaced by watching-zenoh R125c against
// pin `b719ee3e`. Tests below pin the textbook closures:
//
// * Gap 2 (`forge_b5_nu_consumer_parity_alias_neq_stem_rust`) —
//   parent imports the dispatcher with `as="renamed_alias"` distinct
//   from the dispatcher's stem; the encode-side match must reference
//   the dispatcher's stem-derived variant enum, not the alias's
//   PascalCase. The Phase B regression `forge_b5_nu_round_trip_local_
//   nonlocal_rust` happens to use `as == stem`, which masked this bug.
//
// * Gap 3 (`forge_b5_nu_consumer_parity_present_if_gated_rust`) —
//   parent gates the dispatcher embed with `sce:present-if`; the
//   encode-side derivation must wrap the match in `Option` handling
//   (`if let Some` for Rust; absent → derived carrier bit = 0).
//
// * Gap 1 positive (`forge_b5_nu_consumer_parity_chain_forwarding_
//   resolves`) — dispatcher itself has `<sce:requires-parent-flags>`
//   forwarding the carrier; arm bodies' RPF must resolve against the
//   forwarding source.
//
// * Gap 1 negatives (`..._chain_unresolved_rejects` +
//   `..._chain_bit_drift_rejects`) — neither Terminal nor Forwarding
//   covers the body's carrier (ChainUnresolved); Forwarding has the
//   flag at a different bit than the body (ChainBitDrift).

/// RFC B5-ν consumer-parity Gap 2 — alias ≠ stem at the parent's
/// import. The dispatcher's stem (`b5nu_consumer_disp`) drives the
/// emitted variant enum type (`B5nuConsumerDispVariant`); the parent
/// imports it under `as="renamed_alias"`. Regression: Phase B's
/// `embed_type_pascal: filters::to_pascal_case(alias.clone())` would
/// have emitted `RenamedAliasVariant::...` — failing to compile.
#[test]
fn forge_b5_nu_consumer_parity_alias_neq_stem_rust() {
    use std::sync::atomic::{AtomicU64, Ordering};
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let id = COUNTER.fetch_add(1, Ordering::Relaxed);
    let pid = std::process::id();
    let dir = std::env::temp_dir().join(format!("sce_b5nu_alias_neq_stem_{pid}_{id}"));
    std::fs::create_dir_all(&dir).expect("mkdir fixture");

    let parent = r#"<?xml version="1.0"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       xmlns:sce="http://sce.dev/ext"
       sce:kind="codec" sce:codec-id="b5nu_consumer_parent" sce:default-endian="big">
  <sce:import src="b5nu_consumer_disp.scxml" kind="codec" as="renamed_alias"/>
  <datamodel>
    <sce:flags id="header" sce:type="uint8" sce:byte="0" sce:bit-size="8">
      <sce:flag name="mid" bit="0" width="5" value="0x1d"/>
      <sce:flag name="M" bit="6"/>
    </sce:flags>
    <sce:embed id="key" type="renamed_alias" sce:byte="1"/>
  </datamodel>
</scxml>"#;

    let disp = r#"<?xml version="1.0"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       xmlns:sce="http://sce.dev/ext"
       sce:kind="codec" sce:codec-id="b5nu_consumer_disp" sce:default-endian="big">
  <sce:import src="b5nu_consumer_local.scxml" kind="codec" as="b5nu_consumer_local"/>
  <sce:import src="b5nu_consumer_nonlocal.scxml" kind="codec" as="b5nu_consumer_nonlocal"/>
  <sce:requires-parent-flags carrier="header">
    <sce:flag name="M" bit="6"/>
  </sce:requires-parent-flags>
  <datamodel>
    <sce:variant tag="parent.M">
      <sce:arm value="0x00" type="b5nu_consumer_nonlocal" default="true"/>
      <sce:arm value="0x01" type="b5nu_consumer_local"/>
    </sce:variant>
  </datamodel>
</scxml>"#;

    let local = r#"<?xml version="1.0"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       xmlns:sce="http://sce.dev/ext"
       sce:kind="codec" sce:codec-id="b5nu_consumer_local" sce:default-endian="big">
  <datamodel>
    <sce:field id="payload" sce:type="uint8" sce:byte="0" sce:bit-size="8"/>
  </datamodel>
</scxml>"#;

    let nonlocal = r#"<?xml version="1.0"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       xmlns:sce="http://sce.dev/ext"
       sce:kind="codec" sce:codec-id="b5nu_consumer_nonlocal" sce:default-endian="big">
  <datamodel>
    <sce:field id="payload" sce:type="uint16" sce:byte="0" sce:bit-size="16"/>
  </datamodel>
</scxml>"#;

    std::fs::write(dir.join("b5nu_consumer_parent.scxml"), parent).expect("write parent");
    std::fs::write(dir.join("b5nu_consumer_disp.scxml"), disp).expect("write disp");
    std::fs::write(dir.join("b5nu_consumer_local.scxml"), local).expect("write local");
    std::fs::write(dir.join("b5nu_consumer_nonlocal.scxml"), nonlocal).expect("write nonlocal");

    let output = sce_build::compile_forge_with_imports(
        parent,
        sce_build::DocumentLabel::symmetric("b5nu_consumer_parent"),
        sce_build::generator::Language::Rust,
        &dir,
        &sce_build::ForgeCompileOptions::default(),
    )
    .expect("codegen must succeed when alias differs from dispatcher stem");

    let parent_rust = output
        .files
        .iter()
        .find(|(name, _)| name.contains("b5nu_consumer_parent"))
        .map(|(_, body)| body.clone())
        .expect("parent codec emit");

    // The emit MUST reference the dispatcher's stem-derived variant
    // enum (`B5nuConsumerDispVariant`), NOT the consumer's alias
    // PascalCased (`RenamedAliasVariant`).
    assert!(
        parent_rust.contains("B5nuConsumerDispVariant::B5nuConsumerLocal(_) => 0x40u8"),
        "match must use dispatcher's stem-derived variant enum name; \
         got:\n{parent_rust}"
    );
    assert!(
        parent_rust.contains("B5nuConsumerDispVariant::B5nuConsumerNonlocal(_) => 0x00u8"),
        "match must reference Nonlocal arm via stem-derived enum; \
         got:\n{parent_rust}"
    );
    assert!(
        !parent_rust.contains("RenamedAliasVariant"),
        "emit must NOT contain alias-derived `RenamedAliasVariant` \
         (Gap 2 regression);\n{parent_rust}"
    );

    let _ = std::fs::remove_dir_all(&dir);
}

/// RFC B5-ν consumer-parity Gap 3 — parent gates the dispatcher embed
/// with `sce:present-if`. Encode-side derivation wraps the match in
/// `Option` handling; absent embed → derived carrier bit = 0
/// (deterministic extension of derivation to the don't-care case).
/// Regression: Phase B emitted `match &self.<f>.body { ... }`
/// unconditionally — fails to compile when `<f>` is `Option<T>`.
#[test]
fn forge_b5_nu_consumer_parity_present_if_gated_rust() {
    use std::sync::atomic::{AtomicU64, Ordering};
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let id = COUNTER.fetch_add(1, Ordering::Relaxed);
    let pid = std::process::id();
    let dir = std::env::temp_dir().join(format!("sce_b5nu_present_if_{pid}_{id}"));
    std::fs::create_dir_all(&dir).expect("mkdir fixture");

    // Parent has header.R (presence gate) AND header.M (variant tag).
    // The dispatcher embed `key` is gated by header.R; when R=0 the
    // keyexpr field is absent on the wire, and the derived header
    // contribution must default to 0 in the encode.
    let parent = r#"<?xml version="1.0"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       xmlns:sce="http://sce.dev/ext"
       sce:kind="codec" sce:codec-id="b5nu_consumer_gated_parent" sce:default-endian="big">
  <sce:import src="b5nu_consumer_gated_disp.scxml" kind="codec" as="b5nu_consumer_gated_disp"/>
  <datamodel>
    <sce:flags id="header" sce:type="uint8" sce:byte="0" sce:bit-size="8">
      <sce:flag name="R" bit="4"/>
      <sce:flag name="M" bit="6"/>
    </sce:flags>
    <sce:embed id="key" type="b5nu_consumer_gated_disp" sce:byte="1" sce:present-if="header.R"/>
  </datamodel>
</scxml>"#;

    let disp = r#"<?xml version="1.0"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       xmlns:sce="http://sce.dev/ext"
       sce:kind="codec" sce:codec-id="b5nu_consumer_gated_disp" sce:default-endian="big">
  <sce:import src="b5nu_consumer_gated_local.scxml" kind="codec" as="b5nu_consumer_gated_local"/>
  <sce:import src="b5nu_consumer_gated_nonlocal.scxml" kind="codec" as="b5nu_consumer_gated_nonlocal"/>
  <sce:requires-parent-flags carrier="header">
    <sce:flag name="M" bit="6"/>
  </sce:requires-parent-flags>
  <datamodel>
    <sce:variant tag="parent.M">
      <sce:arm value="0x00" type="b5nu_consumer_gated_nonlocal" default="true"/>
      <sce:arm value="0x01" type="b5nu_consumer_gated_local"/>
    </sce:variant>
  </datamodel>
</scxml>"#;

    let local = r#"<?xml version="1.0"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       xmlns:sce="http://sce.dev/ext"
       sce:kind="codec" sce:codec-id="b5nu_consumer_gated_local" sce:default-endian="big">
  <datamodel>
    <sce:field id="payload" sce:type="uint8" sce:byte="0" sce:bit-size="8"/>
  </datamodel>
</scxml>"#;

    let nonlocal = r#"<?xml version="1.0"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       xmlns:sce="http://sce.dev/ext"
       sce:kind="codec" sce:codec-id="b5nu_consumer_gated_nonlocal" sce:default-endian="big">
  <datamodel>
    <sce:field id="payload" sce:type="uint16" sce:byte="0" sce:bit-size="16"/>
  </datamodel>
</scxml>"#;

    std::fs::write(dir.join("b5nu_consumer_gated_parent.scxml"), parent).expect("write parent");
    std::fs::write(dir.join("b5nu_consumer_gated_disp.scxml"), disp).expect("write disp");
    std::fs::write(dir.join("b5nu_consumer_gated_local.scxml"), local).expect("write local");
    std::fs::write(dir.join("b5nu_consumer_gated_nonlocal.scxml"), nonlocal).expect("write nonlocal");

    let output = sce_build::compile_forge_with_imports(
        parent,
        sce_build::DocumentLabel::symmetric("b5nu_consumer_gated_parent"),
        sce_build::generator::Language::Rust,
        &dir,
        &sce_build::ForgeCompileOptions::default(),
    )
    .expect("codegen must succeed when dispatcher embed is present-if-gated");

    let parent_rust = output
        .files
        .iter()
        .find(|(name, _)| name.contains("b5nu_consumer_gated_parent"))
        .map(|(_, body)| body.clone())
        .expect("parent codec emit");

    // The match must wrap in Option handling — `Some(x) => match
    // &x.body { ... }, None => 0u8`. Without the wrap the emit
    // accesses `.body` on an Option<T> and fails to compile.
    assert!(
        parent_rust.contains("Some(x) => match &x.body"),
        "match must wrap the present-if-gated embed in `Some(x) => match &x.body` \
         pattern;\n{parent_rust}"
    );
    assert!(
        parent_rust.contains("None => 0u8"),
        "absent branch must contribute 0 to derived carrier bits \
         (textbook absent → 0 rule);\n{parent_rust}"
    );
    // Defensive — Phase B's unconditional pattern must not appear.
    assert!(
        !parent_rust.contains("match &self.key.body {"),
        "emit must NOT use the unconditional `match &self.key.body` shape \
         (Gap 3 regression);\n{parent_rust}"
    );

    let _ = std::fs::remove_dir_all(&dir);
}

/// RFC B5-ν consumer-parity Gap 1 positive — dispatcher itself
/// declares `<sce:requires-parent-flags>` forwarding the carrier; an
/// arm body that consults a sibling parent flag (other than the
/// variant tag flag) resolves against the dispatcher's forwarding
/// source. Regression: Phase B's `validate_cross_codec_parent_flags`
/// would have errored with the legacy "no field named X" message.
#[test]
fn forge_b5_nu_consumer_parity_chain_forwarding_resolves() {
    use std::sync::atomic::{AtomicU64, Ordering};
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let id = COUNTER.fetch_add(1, Ordering::Relaxed);
    let pid = std::process::id();
    let dir = std::env::temp_dir().join(format!("sce_b5nu_chain_pos_{pid}_{id}"));
    std::fs::create_dir_all(&dir).expect("mkdir fixture");

    // dispatcher forwards `header.{M, N}` to its arm bodies via its
    // own RPF; the variant tag is M (parent.M), the auxiliary flag N
    // is consulted by arm_a's body via the chain.
    let dispatcher = r#"<?xml version="1.0"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       xmlns:sce="http://sce.dev/ext"
       sce:kind="codec" sce:codec-id="b5nu_chain_disp" sce:default-endian="big">
  <sce:import src="b5nu_chain_arm_a.scxml" kind="codec" as="b5nu_chain_arm_a"/>
  <sce:import src="b5nu_chain_arm_b.scxml" kind="codec" as="b5nu_chain_arm_b"/>
  <sce:requires-parent-flags carrier="header">
    <sce:flag name="N" bit="5"/>
    <sce:flag name="M" bit="6"/>
  </sce:requires-parent-flags>
  <datamodel>
    <sce:variant tag="parent.M">
      <sce:arm value="0x00" type="b5nu_chain_arm_b" default="true"/>
      <sce:arm value="0x01" type="b5nu_chain_arm_a"/>
    </sce:variant>
  </datamodel>
</scxml>"#;

    // arm_a declares RPF for header.N — the parent (dispatcher) must
    // satisfy this via its forwarding RPF.
    let arm_a = r#"<?xml version="1.0"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       xmlns:sce="http://sce.dev/ext"
       sce:kind="codec" sce:codec-id="b5nu_chain_arm_a" sce:default-endian="big">
  <sce:requires-parent-flags carrier="header">
    <sce:flag name="N" bit="5"/>
  </sce:requires-parent-flags>
  <datamodel>
    <sce:field id="payload" sce:type="uint8" sce:byte="0" sce:bit-size="8" sce:present-if="parent.N"/>
  </datamodel>
</scxml>"#;

    let arm_b = r#"<?xml version="1.0"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       xmlns:sce="http://sce.dev/ext"
       sce:kind="codec" sce:codec-id="b5nu_chain_arm_b" sce:default-endian="big">
  <datamodel>
    <sce:field id="payload" sce:type="uint16" sce:byte="0" sce:bit-size="16"/>
  </datamodel>
</scxml>"#;

    std::fs::write(dir.join("b5nu_chain_disp.scxml"), dispatcher).expect("write disp");
    std::fs::write(dir.join("b5nu_chain_arm_a.scxml"), arm_a).expect("write arm_a");
    std::fs::write(dir.join("b5nu_chain_arm_b.scxml"), arm_b).expect("write arm_b");

    // Cross-doc validator at the dispatcher level walks each arm
    // body's RPF and resolves them against the dispatcher (parent).
    // With the chain-aware validator, the Forwarding source covers
    // arm_a's `header.N` need. Codegen for the dispatcher itself
    // succeeds (independent of any grandparent's embedding).
    let _ = sce_build::compile_forge_with_imports(
        dispatcher,
        sce_build::DocumentLabel::symmetric("b5nu_chain_disp"),
        sce_build::generator::Language::Rust,
        &dir,
        &sce_build::ForgeCompileOptions::default(),
    )
    .expect("chain-forwarding source must resolve arm body's RPF");

    let _ = std::fs::remove_dir_all(&dir);
}

/// RFC B5-ν consumer-parity Gap 1 negative — neither parent's own
/// `<sce:flags>` nor its `<sce:requires-parent-flags>` declares the
/// carrier the arm body's RPF requires. ChainUnresolved fires.
#[test]
fn forge_b5_nu_consumer_parity_chain_unresolved_rejects() {
    use sce_build::forge::error::{ForgeError, ValidationError};
    use std::sync::atomic::{AtomicU64, Ordering};
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let id = COUNTER.fetch_add(1, Ordering::Relaxed);
    let pid = std::process::id();
    let dir = std::env::temp_dir().join(format!("sce_b5nu_chain_unres_{pid}_{id}"));
    std::fs::create_dir_all(&dir).expect("mkdir fixture");

    // Dispatcher's own RPF declares ONLY `header.M` (the variant tag);
    // arm_a's body needs `header.N`, which the dispatcher does not
    // forward. The chain cannot resolve here.
    let dispatcher = r#"<?xml version="1.0"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       xmlns:sce="http://sce.dev/ext"
       sce:kind="codec" sce:codec-id="b5nu_chain_unres_disp" sce:default-endian="big">
  <sce:import src="b5nu_chain_unres_arm_a.scxml" kind="codec" as="b5nu_chain_unres_arm_a"/>
  <sce:import src="b5nu_chain_unres_arm_b.scxml" kind="codec" as="b5nu_chain_unres_arm_b"/>
  <sce:requires-parent-flags carrier="header">
    <sce:flag name="M" bit="6"/>
  </sce:requires-parent-flags>
  <datamodel>
    <sce:variant tag="parent.M">
      <sce:arm value="0x00" type="b5nu_chain_unres_arm_b" default="true"/>
      <sce:arm value="0x01" type="b5nu_chain_unres_arm_a"/>
    </sce:variant>
  </datamodel>
</scxml>"#;

    let arm_a = r#"<?xml version="1.0"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       xmlns:sce="http://sce.dev/ext"
       sce:kind="codec" sce:codec-id="b5nu_chain_unres_arm_a" sce:default-endian="big">
  <sce:requires-parent-flags carrier="trailer">
    <sce:flag name="X" bit="3"/>
  </sce:requires-parent-flags>
  <datamodel>
    <sce:field id="payload" sce:type="uint8" sce:byte="0" sce:bit-size="8" sce:present-if="parent.X"/>
  </datamodel>
</scxml>"#;

    let arm_b = r#"<?xml version="1.0"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       xmlns:sce="http://sce.dev/ext"
       sce:kind="codec" sce:codec-id="b5nu_chain_unres_arm_b" sce:default-endian="big">
  <datamodel>
    <sce:field id="payload" sce:type="uint16" sce:byte="0" sce:bit-size="16"/>
  </datamodel>
</scxml>"#;

    std::fs::write(dir.join("b5nu_chain_unres_disp.scxml"), dispatcher).expect("write disp");
    std::fs::write(dir.join("b5nu_chain_unres_arm_a.scxml"), arm_a).expect("write arm_a");
    std::fs::write(dir.join("b5nu_chain_unres_arm_b.scxml"), arm_b).expect("write arm_b");

    let err = match sce_build::compile_forge_with_imports(
        dispatcher,
        sce_build::DocumentLabel::symmetric("b5nu_chain_unres_disp"),
        sce_build::generator::Language::Rust,
        &dir,
        &sce_build::ForgeCompileOptions::default(),
    ) {
        Ok(_) => panic!("validator must reject unresolved carrier chain"),
        Err(e) => e,
    };
    assert!(
        matches!(
            err.error,
            ForgeError::Validation(
                ValidationError::CodecParentFlagChainUnresolved {
                    ref body_codec,
                    ref parent_codec,
                    ref carrier,
                    parent_has_rpf,
                    ..
                }
            ) if body_codec == "b5nu_chain_unres_arm_a"
                && parent_codec == "b5nu_chain_unres_disp"
                && carrier == "trailer"
                && parent_has_rpf
        ),
        "got: {:?}",
        err.error
    );

    let _ = std::fs::remove_dir_all(&dir);
}

/// RFC B5-ν consumer-parity Gap 1 negative — dispatcher forwards the
/// carrier but the flag's bit position diverges between body and
/// forwarding RPF. ChainBitDrift fires (Forwarding-source path).
#[test]
fn forge_b5_nu_consumer_parity_chain_bit_drift_rejects() {
    use sce_build::forge::error::{ForgeError, ValidationError};
    use std::sync::atomic::{AtomicU64, Ordering};
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let id = COUNTER.fetch_add(1, Ordering::Relaxed);
    let pid = std::process::id();
    let dir = std::env::temp_dir().join(format!("sce_b5nu_chain_drift_{pid}_{id}"));
    std::fs::create_dir_all(&dir).expect("mkdir fixture");

    // Dispatcher forwards `header.N` at bit=4; arm_a's RPF declares
    // `header.N` at bit=5 — the body and parent layouts disagree.
    let dispatcher = r#"<?xml version="1.0"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       xmlns:sce="http://sce.dev/ext"
       sce:kind="codec" sce:codec-id="b5nu_chain_drift_disp" sce:default-endian="big">
  <sce:import src="b5nu_chain_drift_arm_a.scxml" kind="codec" as="b5nu_chain_drift_arm_a"/>
  <sce:import src="b5nu_chain_drift_arm_b.scxml" kind="codec" as="b5nu_chain_drift_arm_b"/>
  <sce:requires-parent-flags carrier="header">
    <sce:flag name="N" bit="4"/>
    <sce:flag name="M" bit="6"/>
  </sce:requires-parent-flags>
  <datamodel>
    <sce:variant tag="parent.M">
      <sce:arm value="0x00" type="b5nu_chain_drift_arm_b" default="true"/>
      <sce:arm value="0x01" type="b5nu_chain_drift_arm_a"/>
    </sce:variant>
  </datamodel>
</scxml>"#;

    let arm_a = r#"<?xml version="1.0"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       xmlns:sce="http://sce.dev/ext"
       sce:kind="codec" sce:codec-id="b5nu_chain_drift_arm_a" sce:default-endian="big">
  <sce:requires-parent-flags carrier="header">
    <sce:flag name="N" bit="5"/>
  </sce:requires-parent-flags>
  <datamodel>
    <sce:field id="payload" sce:type="uint8" sce:byte="0" sce:bit-size="8" sce:present-if="parent.N"/>
  </datamodel>
</scxml>"#;

    let arm_b = r#"<?xml version="1.0"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       xmlns:sce="http://sce.dev/ext"
       sce:kind="codec" sce:codec-id="b5nu_chain_drift_arm_b" sce:default-endian="big">
  <datamodel>
    <sce:field id="payload" sce:type="uint16" sce:byte="0" sce:bit-size="16"/>
  </datamodel>
</scxml>"#;

    std::fs::write(dir.join("b5nu_chain_drift_disp.scxml"), dispatcher).expect("write disp");
    std::fs::write(dir.join("b5nu_chain_drift_arm_a.scxml"), arm_a).expect("write arm_a");
    std::fs::write(dir.join("b5nu_chain_drift_arm_b.scxml"), arm_b).expect("write arm_b");

    let err = match sce_build::compile_forge_with_imports(
        dispatcher,
        sce_build::DocumentLabel::symmetric("b5nu_chain_drift_disp"),
        sce_build::generator::Language::Rust,
        &dir,
        &sce_build::ForgeCompileOptions::default(),
    ) {
        Ok(_) => panic!("validator must reject Forwarding-source bit drift"),
        Err(e) => e,
    };
    assert!(
        matches!(
            err.error,
            ForgeError::Validation(
                ValidationError::CodecParentFlagChainBitDrift {
                    ref body_codec,
                    ref parent_codec,
                    ref carrier,
                    ref flag,
                    body_bit,
                    parent_bit,
                }
            ) if body_codec == "b5nu_chain_drift_arm_a"
                && parent_codec == "b5nu_chain_drift_disp"
                && carrier == "header"
                && flag == "N"
                && body_bit == 5
                && parent_bit == 4
        ),
        "got: {:?}",
        err.error
    );

    let _ = std::fs::remove_dir_all(&dir);
}
