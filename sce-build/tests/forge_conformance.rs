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
fn golden_options(
    language: sce_build::generator::Language,
) -> sce_build::ForgeCompileOptions {
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
fn assert_inline_kinds_lang(
    scxml_name: &str,
    lang: sce_build::generator::Language,
) {
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
    let InlineKindCode { type_defs, member_fns } =
        render_inline_kinds(&model.inline_kinds, lang, &machine_name)
            .unwrap_or_else(|e| panic!("render_inline_kinds({lang:?}) failed: {e}"));

    let lang_tag = match lang {
        sce_build::generator::Language::Kotlin => "kt",
        sce_build::generator::Language::Rust => "rs",
        sce_build::generator::Language::Go => "go",
        sce_build::generator::Language::C11 => "c",
        _ => panic!("unsupported language for inline kind lang test"),
    };

    // Member functions golden (always present)
    let fns_golden_path = expected_dir().join(format!(
        "{scxml_name}_inline_fns.{lang_tag}.golden"
    ));
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
        let types_golden_path = expected_dir().join(format!(
            "{scxml_name}_inline_types.{lang_tag}.golden"
        ));
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
    let output = sce_build::compile_scxml_lang(
        scxml_path.to_str().unwrap(),
        &tdir,
        lang,
    )
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
            assert!(member_fns.contains("enum class RpmStatus"),
                "Kotlin: missing nested enum class");
            // when expression
            assert!(member_fns.contains("= when ("),
                "Kotlin: missing when expression in lookup");
            // Kotlin-idiomatic function signatures
            assert!(member_fns.contains("fun isReady(): Boolean ="),
                "Kotlin: missing condition function");
            assert!(member_fns.contains("fun computeToFahrenheit(): Double ="),
                "Kotlin: missing transform function");
        }
        Language::Rust => {
            // Module-level enum in type_defs
            assert!(type_defs.contains("pub enum RpmStatus"),
                "Rust: missing enum in type_defs");
            assert!(type_defs.contains("#[derive(Debug, Clone, Copy, PartialEq)]"),
                "Rust: missing derives on enum");
            // self. prefix for member access
            assert!(member_fns.contains("self."),
                "Rust: missing self. prefix for member access");
            // Idiomatic signatures
            assert!(member_fns.contains("pub fn is_ready(&self) -> bool"),
                "Rust: missing condition function signature");
            assert!(member_fns.contains("pub fn compute_to_fahrenheit(&self) -> f64"),
                "Rust: missing transform function signature");
            // match expression in lookup
            assert!(member_fns.contains("match raw"),
                "Rust: missing match in lookup");
        }
        Language::Go => {
            // Package-level type in type_defs
            assert!(type_defs.contains("type RpmStatus int"),
                "Go: missing type in type_defs");
            assert!(type_defs.contains("RpmStatus = iota"),
                "Go: missing iota const block");
            // p. receiver prefix for member access
            assert!(member_fns.contains("p."),
                "Go: missing p. receiver prefix for member access");
            // Exported method with receiver
            assert!(member_fns.contains("func (p *"),
                "Go: missing receiver method");
            // Package-level lookup (no receiver)
            assert!(member_fns.contains("func LookupRpmStatus("),
                "Go: missing package-level lookup function");
        }
        Language::C11 => {
            // RFC §5.J.2 Phase F. Verifies idiomatic C11 emit shape that
            // byte-comparison alone cannot catch (e.g. a missing `_st->`
            // prefix or wrong typedef shape would still match a stale
            // golden written from a buggy renderer).

            // Top-level enum typedef (no nesting in C)
            assert!(member_fns.contains("typedef enum"),
                "C11: missing typedef enum for lookup");
            assert!(member_fns.contains("} inline_mixed_rpm_status_t;"),
                "C11: missing snake_case typedef name");
            // Prefixed enum constants (no namespacing in C)
            assert!(member_fns.contains("INLINE_MIXED_RPM_STATUS_OFF"),
                "C11: missing prefixed enum constant");
            assert!(member_fns.contains("INLINE_MIXED_RPM_STATUS_RUNNING"),
                "C11: missing prefixed enum constant");
            // _st-> member access (procedure D14a mirror)
            assert!(member_fns.contains("_st->"),
                "C11: missing _st-> prefix for policy member access");
            // Free-standing static inline functions
            assert!(member_fns.contains("static inline bool inline_mixed_is_ready("),
                "C11: missing condition function signature");
            assert!(member_fns.contains("static inline double inline_mixed_compute_to_fahrenheit("),
                "C11: missing transform function signature");
            assert!(member_fns.contains(
                "static inline inline_mixed_rpm_status_t inline_mixed_lookup_rpm_status("
            ), "C11: missing lookup function signature");
            // const policy pointer parameter
            assert!(member_fns.contains("(const inline_mixed_policy_t *_st)"),
                "C11: missing const policy pointer parameter");
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
            assert!(member_fns.contains("data class Frame("),
                "Kotlin: missing data class for inline codec");
            assert!(member_fns.contains("companion object"),
                "Kotlin: missing companion object hosting decode");
            assert!(member_fns.contains("fun decode(cursor: com.sce.forge.runtime.SceCursor): Frame?"),
                "Kotlin: missing cursor-based decode signature");
            assert!(member_fns.contains("fun encode(): ByteArray = byteArrayOf("),
                "Kotlin: missing encode signature");
        }
        Language::Rust => {
            assert!(type_defs.contains("pub struct Frame"),
                "Rust: missing pub struct in type_defs");
            assert!(type_defs.contains("#[derive(Debug, Clone)]"),
                "Rust: missing derives on codec struct");
            assert!(type_defs.contains(
                "pub fn decode(cursor: &mut ::sce_forge_runtime::codec::SceCursor<'_>) -> \
                 Result<Self, ::sce_forge_runtime::codec::CodecError>"
            ), "Rust: missing cursor-based decode signature");
            assert!(type_defs.contains("pub fn encode(&self) -> Vec<u8>"),
                "Rust: missing encode signature");
        }
        Language::Go => {
            assert!(type_defs.contains("type Frame struct"),
                "Go: missing struct in type_defs");
            assert!(type_defs.contains("func DecodeFrame(cursor *codec.SceCursor) (*Frame, error)"),
                "Go: missing cursor-based exported Decode function");
            assert!(type_defs.contains("func (s *Frame) Encode() []byte"),
                "Go: missing receiver Encode method");
        }
        Language::C11 => {
            assert!(member_fns.contains("#define INLINE_CODEC_FRAME_MIN_BYTES 4"),
                "C11: missing min-bytes macro");
            assert!(member_fns.contains("} inline_codec_frame_t;"),
                "C11: missing payload typedef");
            assert!(member_fns.contains("} inline_codec_frame_encoded_t;"),
                "C11: missing encoded envelope typedef");
            assert!(member_fns.contains(
                "static inline sce_forge_codec_status_t inline_codec_frame_decode(\
                 sce_forge_cursor_t *cursor, inline_codec_frame_t *out)"
            ), "C11: missing cursor-based decode signature");
            assert!(member_fns.contains(
                "static inline inline_codec_frame_encoded_t \
                 inline_codec_frame_encode(const inline_codec_frame_t *self)"
            ), "C11: missing encode signature");
            // self->{snake} member access on encode side
            assert!(member_fns.contains("self->msg_id"),
                "C11: missing self-> prefix on encode field access");
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
    use sce_build::forge::model::{
        AlgorithmConstType, ForgeDocument, SceType,
    };
    use sce_build::forge::parser::parse_forge;
    use sce_build::DocumentLabel;

    let scxml_path = resource_dir().join("algorithm_const_fold_smoke.scxml");
    let content = std::fs::read_to_string(&scxml_path).expect("read fixture");

    let doc = parse_forge(&content, DocumentLabel::symmetric("algorithm_const_fold_smoke"))
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
            assert_eq!(*elem, SceType::Uint16, "Rust-style `u16` alias must map to Uint16");
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
    assert_standalone_forge(
        "algorithm_const_fold_smoke",
        "algorithm_const_fold_smoke.h",
    );
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
    assert_standalone_forge_rust(
        "algorithm_crc16_table",
        "algorithm_crc16_table.rs",
    );
}

#[test]
fn forge_algorithm_crc16_table_cpp() {
    assert_standalone_forge(
        "algorithm_crc16_table",
        "algorithm_crc16_table.h",
    );
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
    assert_standalone_forge(
        "codec_zenoh_keep_alive",
        "codec_zenoh_keep_alive.h",
    );
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
    assert_standalone_forge(
        "codec_present_if_basic",
        "codec_present_if_basic.h",
    );
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
    assert_standalone_forge_rust(
        "codec_tlv_chain_basic",
        "codec_tlv_chain_basic.rs",
    );
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
    assert_standalone_forge_rust(
        "codec_zenoh_ext_unit",
        "codec_zenoh_ext_unit.rs",
    );
}

#[test]
fn forge_codec_zenoh_ext_zint_rust() {
    assert_standalone_forge_rust(
        "codec_zenoh_ext_zint",
        "codec_zenoh_ext_zint.rs",
    );
}

#[test]
fn forge_codec_zenoh_ext_zbuf_rust() {
    assert_standalone_forge_rust(
        "codec_zenoh_ext_zbuf",
        "codec_zenoh_ext_zbuf.rs",
    );
}

#[test]
fn forge_codec_zenoh_ext_entry_rust() {
    assert_standalone_forge_rust(
        "codec_zenoh_ext_entry",
        "codec_zenoh_ext_entry.rs",
    );
}

#[test]
fn forge_codec_zenoh_ext_envelope_rust() {
    assert_standalone_forge_rust(
        "codec_zenoh_ext_envelope",
        "codec_zenoh_ext_envelope.rs",
    );
}

/// RFC §5.B B5-ε MCU gate: the envelope's `<sce:tlv-chain>` keeps the
/// MCU-class contract — cpp/kotlin/go/python reject at codegen with
/// `codegen/mcu-class-kind-on-non-mcu-language`, exactly mirroring
/// `codec_tlv_chain_basic`'s rejection shape. Surface G adds variant
/// body dispatch *inside* the chain entry; it does not relax the
/// per-codec MCU gate.
#[test]
fn forge_codec_zenoh_ext_envelope_rejects_on_cpp() {
    use sce_build::forge::error::{ForgeError, GenerateError};

    let scxml_path = resource_dir().join("codec_zenoh_ext_envelope.scxml");
    let content = std::fs::read_to_string(&scxml_path)
        .expect("Cannot read codec_zenoh_ext_envelope.scxml");
    let result = sce_build::compile_forge_with_imports(
        &content,
        sce_build::DocumentLabel::symmetric("codec_zenoh_ext_envelope"),
        sce_build::generator::Language::Cpp,
        &resource_dir(),
        &sce_build::ForgeCompileOptions::default(),
    );
    let err = match result {
        Ok(_) => panic!(
            "MCU-class codec on Cpp must reject with \
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

/// RFC §5.B B3 MCU gate: a codec containing `<sce:tlv-chain>` rejects
/// when targeting `cpp` (and the other 3 non-MCU langs by the same
/// path). The diagnostic is the existing kind-class
/// `codegen/mcu-class-kind-on-non-mcu-language`, repurposed at the
/// codec-content granularity per RFC §5.B "MCU-only codec sub-features"
/// (line 521-525). The `kind` field carries the codec name + the
/// MCU-only-features marker so authors see exactly which codec hit the
/// gate.
#[test]
fn forge_codec_tlv_chain_rejects_on_cpp() {
    use sce_build::forge::error::{ForgeError, GenerateError};

    let scxml_path = resource_dir().join("codec_tlv_chain_basic.scxml");
    let content = std::fs::read_to_string(&scxml_path)
        .expect("Cannot read codec_tlv_chain_basic.scxml");
    let result = sce_build::compile_forge_with_imports(
        &content,
        sce_build::DocumentLabel::symmetric("codec_tlv_chain_basic"),
        sce_build::generator::Language::Cpp,
        &resource_dir(),
        &sce_build::ForgeCompileOptions::default(),
    );
    let err = match result {
        Ok(_) => panic!(
            "MCU-class codec on Cpp must reject with \
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
    assert_standalone_forge_rust(
        "codec_dma_aligned_basic",
        "codec_dma_aligned_basic.rs",
    );
}

/// RFC §5.B B3 MCU gate: a codec with `sce:dma-burst-align` on any
/// field rejects when targeting cpp via the existing codec-content
/// MCU mechanism (mirrors TLV chain). The diagnostic kind name folds
/// the codec identifier + the MCU-only-features marker.
#[test]
fn forge_codec_dma_aligned_rejects_on_cpp() {
    use sce_build::forge::error::{ForgeError, GenerateError};

    let scxml_path = resource_dir().join("codec_dma_aligned_basic.scxml");
    let content = std::fs::read_to_string(&scxml_path)
        .expect("Cannot read codec_dma_aligned_basic.scxml");
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

    assert_eq!(alg.test_vectors.len(), 1, "fixture declares one <sce:test-vector>");
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

/// Negative: `<sce:test-vector>` declared under `sce:kind="codec"` (or
/// any non-algorithm kind) rejects with the typed
/// `algorithm/test-vector-unsupported-kind` diagnostic so the v1
/// algorithm-only restriction is explicit at parse time. Multi-field
/// codec oracle grammar defers to B5 — until then, codec round-trips
/// belong in the existing numerical_reference.json harness.
#[test]
fn forge_test_vector_on_codec_rejects() {
    use sce_build::forge::error::{ForgeError, ValidationError};
    use sce_build::forge::model::ForgeKind;

    let scxml = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       xmlns:sce="http://sce.dev/ext"
       sce:kind="codec" sce:default-endian="big" name="frame_codec">
  <datamodel>
    <sce:field id="msg_id" sce:type="uint8" sce:byte="0" sce:bit-size="8"/>
  </datamodel>
  <sce:test-vector hex="01" value="0x01"/>
</scxml>"#;
    let result = sce_build::compile_forge_with_imports(
        scxml,
        sce_build::DocumentLabel::symmetric("frame_codec"),
        sce_build::generator::Language::Rust,
        &resource_dir(),
        &sce_build::ForgeCompileOptions::default(),
    );
    let err = match result {
        Ok(_) => panic!(
            "<sce:test-vector> under sce:kind=\"codec\" must reject \
             with algorithm/test-vector-unsupported-kind"
        ),
        Err(e) => e,
    };
    assert!(
        matches!(
            err.error,
            ForgeError::Validation(ValidationError::TestVectorUnsupportedKind {
                ref name,
                kind: ForgeKind::Codec,
            }) if name == "frame_codec"
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
    assert_standalone_forge(
        "codec_variant_session_open",
        "codec_variant_session_open.h",
    );
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
    assert_standalone_forge(
        "codec_variant_dispatch",
        "codec_variant_dispatch.h",
    );
}

/// RFC §5.B B5-β multi-bit-flag variant dispatch (Cpp): `<sce:variant
/// tag="header.mid">` extracts the 5-bit MID slice from a uint8 flags
/// carrier and dispatches into KeepAlive (empty body) / Close (uint8
/// reason) / Default arms — mirrors zenoh-pico's transport-message
/// envelope shape (`_z_transport_message_decode`,
/// `_Z_MID_MASK = 0x1f`).
#[test]
fn forge_codec_transport_envelope_cpp() {
    assert_standalone_forge(
        "codec_transport_envelope",
        "codec_transport_envelope.h",
    );
}

/// RFC §5.B B5-γ trunk (Cpp): body codec with `<sce:requires-parent-flags
/// carrier="header"><sce:flag name="S" bit="6"/></sce:requires-parent-flags>`
/// emits decode/encode signatures that take a `std::uint8_t parent_flags`
/// parameter. Body fields gated by `sce:present-if="parent.S"` read the
/// bit from this parameter rather than from a sibling carrier. Mirrors
/// zenoh-pico's `_z_init_decode(.., uint8_t header)` upstream pattern.
#[test]
fn forge_codec_init_syn_body_cpp() {
    assert_standalone_forge(
        "codec_init_syn_body",
        "codec_init_syn_body.h",
    );
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
    assert_standalone_forge(
        "codec_init_syn_envelope",
        "codec_init_syn_envelope.h",
    );
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
    assert_standalone_forge(
        "codec_init_cookie_body",
        "codec_init_cookie_body.h",
    );
}

/// RFC §5.B B5-δ Surface F (Cpp): Scout/Hello/Init zid codec exercising
/// arithmetic offset on the length sibling. Author writes
/// `sce:length-arith="+1"` paired with `sce:length-field="zid_len_m1"`;
/// decode reads `_n = sibling_value + 1` bytes. Mirrors zenoh-pico's
/// `zidlen = ((cbyte & 0xF0) >> 4) + (uint8_t)1` (`transport.c:251`).
#[test]
fn forge_codec_scout_zid_body_cpp() {
    assert_standalone_forge(
        "codec_scout_zid_body",
        "codec_scout_zid_body.h",
    );
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
        Ok(_) => panic!(
            "2-arm coverage of width-2 domain (4 values) without default must reject"
        ),
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

// ── RFC §5.B B5-α multi-bit + empty-codec (Kotlin) ───────────

#[test]
fn forge_kotlin_codec_qos_byte() {
    assert_standalone_forge_kotlin("codec_qos_byte", "CodecQosByte.kt");
}

#[test]
fn forge_kotlin_codec_zenoh_keep_alive() {
    assert_standalone_forge_kotlin(
        "codec_zenoh_keep_alive",
        "CodecZenohKeepAlive.kt",
    );
}

// ── RFC §5.B B1-γ flags primitive (Kotlin) ───────────────────

#[test]
fn forge_kotlin_codec_flags_basic() {
    assert_standalone_forge_kotlin("codec_flags_basic", "CodecFlagsBasic.kt");
}

// ── RFC §5.B variant primitive (Kotlin, B1-β closure) ────────

#[test]
fn forge_kotlin_codec_variant_session_open() {
    assert_standalone_forge_kotlin(
        "codec_variant_session_open",
        "CodecVariantSessionOpen.kt",
    );
}

#[test]
fn forge_kotlin_codec_variant_session_close() {
    assert_standalone_forge_kotlin(
        "codec_variant_session_close",
        "CodecVariantSessionClose.kt",
    );
}

#[test]
fn forge_kotlin_codec_variant_dispatch() {
    assert_standalone_forge_kotlin(
        "codec_variant_dispatch",
        "CodecVariantDispatch.kt",
    );
}

#[test]
fn forge_kotlin_codec_transport_envelope() {
    assert_standalone_forge_kotlin(
        "codec_transport_envelope",
        "CodecTransportEnvelope.kt",
    );
}

// ── RFC §5.B B1-δ present-if primitive (Kotlin) ─────────────

#[test]
fn forge_kotlin_codec_present_if_basic() {
    assert_standalone_forge_kotlin(
        "codec_present_if_basic",
        "CodecPresentIfBasic.kt",
    );
}

// ── RFC §5.B B2-β present-if + variable-length (Kotlin) ─────

#[test]
fn forge_kotlin_codec_present_if_tail() {
    assert_standalone_forge_kotlin(
        "codec_present_if_tail",
        "CodecPresentIfTail.kt",
    );
}

#[test]
fn forge_kotlin_codec_present_if_length_ref() {
    assert_standalone_forge_kotlin(
        "codec_present_if_length_ref",
        "CodecPresentIfLengthRef.kt",
    );
}

#[test]
fn forge_kotlin_codec_present_if_vle() {
    assert_standalone_forge_kotlin(
        "codec_present_if_vle",
        "CodecPresentIfVle.kt",
    );
}

// ── RFC §5.B B2 repeat primitive (Kotlin, closure) ──────────

#[test]
fn forge_kotlin_codec_repeat_elem() {
    assert_standalone_forge_kotlin(
        "codec_repeat_elem",
        "CodecRepeatElem.kt",
    );
}

#[test]
fn forge_kotlin_codec_repeat_basic() {
    assert_standalone_forge_kotlin(
        "codec_repeat_basic",
        "CodecRepeatBasic.kt",
    );
}

#[test]
fn forge_kotlin_codec_until_eof_basic() {
    assert_standalone_forge_kotlin(
        "codec_until_eof_basic",
        "CodecUntilEofBasic.kt",
    );
}

// ── RFC §5.B B4 applied codec shapes (Kotlin) ───────────────

#[test]
fn forge_kotlin_codec_ext_timestamp() {
    assert_standalone_forge_kotlin(
        "codec_ext_timestamp",
        "CodecExtTimestamp.kt",
    );
}

#[test]
fn forge_kotlin_codec_ext_attachment() {
    assert_standalone_forge_kotlin(
        "codec_ext_attachment",
        "CodecExtAttachment.kt",
    );
}

#[test]
fn forge_kotlin_codec_ext_encoding_info() {
    assert_standalone_forge_kotlin(
        "codec_ext_encoding_info",
        "CodecExtEncodingInfo.kt",
    );
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

// ── RFC §5.B B5-α multi-bit + empty-codec (Rust) ─────────────

#[test]
fn forge_rust_codec_qos_byte() {
    assert_standalone_forge_rust("codec_qos_byte", "codec_qos_byte.rs");
}

#[test]
fn forge_rust_codec_zenoh_keep_alive() {
    assert_standalone_forge_rust(
        "codec_zenoh_keep_alive",
        "codec_zenoh_keep_alive.rs",
    );
}

// ── RFC §5.B B1-γ flags primitive (Rust) ─────────────────────

#[test]
fn forge_rust_codec_flags_basic() {
    assert_standalone_forge_rust("codec_flags_basic", "codec_flags_basic.rs");
}

// ── RFC §5.B B1-δ present-if primitive (Rust) ───────────────

#[test]
fn forge_rust_codec_present_if_basic() {
    assert_standalone_forge_rust(
        "codec_present_if_basic",
        "codec_present_if_basic.rs",
    );
}

// ── RFC §5.B B2-β present-if + variable-length (Rust) ───────

#[test]
fn forge_rust_codec_present_if_tail() {
    assert_standalone_forge_rust(
        "codec_present_if_tail",
        "codec_present_if_tail.rs",
    );
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
    assert_standalone_forge_rust(
        "codec_present_if_vle",
        "codec_present_if_vle.rs",
    );
}

// ── RFC §5.B B2 repeat primitive (Rust, trunk) ──────────────

#[test]
fn forge_rust_codec_repeat_elem() {
    assert_standalone_forge_rust(
        "codec_repeat_elem",
        "codec_repeat_elem.rs",
    );
}

#[test]
fn forge_rust_codec_repeat_basic() {
    assert_standalone_forge_rust(
        "codec_repeat_basic",
        "codec_repeat_basic.rs",
    );
}

#[test]
fn forge_rust_codec_until_eof_basic() {
    assert_standalone_forge_rust(
        "codec_until_eof_basic",
        "codec_until_eof_basic.rs",
    );
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
    assert_standalone_forge_rust(
        "codec_ext_encoding_info",
        "codec_ext_encoding_info.rs",
    );
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
    assert_standalone_forge_rust(
        "codec_variant_dispatch",
        "codec_variant_dispatch.rs",
    );
}

#[test]
fn forge_rust_codec_transport_envelope() {
    assert_standalone_forge_rust(
        "codec_transport_envelope",
        "codec_transport_envelope.rs",
    );
}

/// RFC §5.B B5-γ trunk (Rust): body codec with parent-flags dependency.
/// Decode/encode signatures gain a `parent_flags: u8` parameter; body
/// fields gated via `parent.<flag>` predicates emit
/// `(parent_flags & 0x40) != 0` style bit-tests. Mirrors
/// `_z_init_decode` upstream signature shape.
#[test]
fn forge_rust_codec_init_syn_body() {
    assert_standalone_forge_rust(
        "codec_init_syn_body",
        "codec_init_syn_body.rs",
    );
}

/// RFC §5.B B5-γ trunk (Rust): variant parent threading carrier value.
/// Each arm dispatches `Body::decode(cursor, header)` for arm bodies
/// declaring `<sce:requires-parent-flags carrier="header">`; encode
/// passes `body.encode(header)` symmetrically. Cross-codec validator
/// confirms parent's flag layout matches the body's declared
/// `<sce:flag name="S" bit="6"/>`.
#[test]
fn forge_rust_codec_init_syn_envelope() {
    assert_standalone_forge_rust(
        "codec_init_syn_envelope",
        "codec_init_syn_envelope.rs",
    );
}

/// RFC §5.B B5-δ Surfaces D + E (Rust): Init body cookie codec.
/// `cookie_size` is `Option<u16>` (gated VLE u16); `cookie` is
/// `Option<Vec<u8>>` (gated length-ref bytes). Helper emits
/// `cookie_size.unwrap() as usize` inside the predicate's true-branch.
#[test]
fn forge_rust_codec_init_cookie_body() {
    assert_standalone_forge_rust(
        "codec_init_cookie_body",
        "codec_init_cookie_body.rs",
    );
}

/// RFC §5.B B5-δ Surface F (Rust): Scout/Hello/Init zid codec.
/// `length-arith="+1"` lifts the byte count by one above the sibling's
/// value: helper emits `(zid_len_m1 as usize).wrapping_add(1)` for
/// decode; encode trusts `zid.len()` as the source of truth (author
/// contract: `zid_len_m1 == zid.len() - 1`).
#[test]
fn forge_rust_codec_scout_zid_body() {
    assert_standalone_forge_rust(
        "codec_scout_zid_body",
        "codec_scout_zid_body.rs",
    );
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

// ── RFC §5.B B5-α multi-bit + empty-codec (Go) ───────────────

#[test]
fn forge_go_codec_qos_byte() {
    assert_standalone_forge_go("codec_qos_byte", "codec_qos_byte.go");
}

#[test]
fn forge_go_codec_zenoh_keep_alive() {
    assert_standalone_forge_go(
        "codec_zenoh_keep_alive",
        "codec_zenoh_keep_alive.go",
    );
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
    assert_standalone_forge_go(
        "codec_variant_dispatch",
        "codec_variant_dispatch.go",
    );
}

#[test]
fn forge_go_codec_transport_envelope() {
    assert_standalone_forge_go(
        "codec_transport_envelope",
        "codec_transport_envelope.go",
    );
}

// ── RFC §5.B B1-δ present-if primitive (Go) ─────────────────

#[test]
fn forge_go_codec_present_if_basic() {
    assert_standalone_forge_go(
        "codec_present_if_basic",
        "codec_present_if_basic.go",
    );
}

// ── RFC §5.B B2-β present-if + variable-length (Go) ─────────

#[test]
fn forge_go_codec_present_if_tail() {
    assert_standalone_forge_go(
        "codec_present_if_tail",
        "codec_present_if_tail.go",
    );
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
    assert_standalone_forge_go(
        "codec_present_if_vle",
        "codec_present_if_vle.go",
    );
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
    assert_standalone_forge_go("algorithm_const_fold_smoke", "algorithm_const_fold_smoke.go");
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

// ── RFC §5.B B5-α multi-bit + empty-codec (Python) ───────────

#[test]
fn forge_python_codec_qos_byte() {
    assert_standalone_forge_python("codec_qos_byte", "codec_qos_byte.py");
}

#[test]
fn forge_python_codec_zenoh_keep_alive() {
    assert_standalone_forge_python(
        "codec_zenoh_keep_alive",
        "codec_zenoh_keep_alive.py",
    );
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
    assert_standalone_forge_python(
        "codec_variant_dispatch",
        "codec_variant_dispatch.py",
    );
}

#[test]
fn forge_python_codec_transport_envelope() {
    assert_standalone_forge_python(
        "codec_transport_envelope",
        "codec_transport_envelope.py",
    );
}

// ── RFC §5.B B1-δ present-if primitive (Python) ─────────────

#[test]
fn forge_python_codec_present_if_basic() {
    assert_standalone_forge_python(
        "codec_present_if_basic",
        "codec_present_if_basic.py",
    );
}

// ── RFC §5.B B2-β present-if + variable-length (Python) ─────

#[test]
fn forge_python_codec_present_if_tail() {
    assert_standalone_forge_python(
        "codec_present_if_tail",
        "codec_present_if_tail.py",
    );
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
    assert_standalone_forge_python(
        "codec_present_if_vle",
        "codec_present_if_vle.py",
    );
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
    assert_standalone_forge_python(
        "codec_ext_encoding_info",
        "codec_ext_encoding_info.py",
    );
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
    assert_standalone_forge_python("algorithm_const_fold_smoke", "algorithm_const_fold_smoke.py");
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

// ── RFC §5.B B5-α multi-bit + empty-codec (C11) ──────────────

#[test]
fn forge_c11_codec_qos_byte() {
    assert_standalone_forge_c("codec_qos_byte", "codec_qos_byte.c.h");
}

#[test]
fn forge_c11_codec_zenoh_keep_alive() {
    assert_standalone_forge_c(
        "codec_zenoh_keep_alive",
        "codec_zenoh_keep_alive.c.h",
    );
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
    assert_standalone_forge_c(
        "codec_variant_dispatch",
        "codec_variant_dispatch.c.h",
    );
}

#[test]
fn forge_c11_codec_transport_envelope() {
    assert_standalone_forge_c(
        "codec_transport_envelope",
        "codec_transport_envelope.c.h",
    );
}

// ── RFC §5.B B1-δ present-if primitive (C11) ────────────────

#[test]
fn forge_c11_codec_present_if_basic() {
    assert_standalone_forge_c(
        "codec_present_if_basic",
        "codec_present_if_basic.c.h",
    );
}

// ── RFC §5.B B2-β present-if + variable-length (C11) ────────

#[test]
fn forge_c11_codec_present_if_tail() {
    assert_standalone_forge_c(
        "codec_present_if_tail",
        "codec_present_if_tail.c.h",
    );
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
    assert_standalone_forge_c(
        "codec_present_if_vle",
        "codec_present_if_vle.c.h",
    );
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
    assert_standalone_forge_c(
        "codec_tlv_chain_basic",
        "codec_tlv_chain_basic.c.h",
    );
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
    assert_standalone_forge_c(
        "codec_zenoh_ext_unit",
        "codec_zenoh_ext_unit.c.h",
    );
}

#[test]
fn forge_c11_codec_zenoh_ext_zint() {
    assert_standalone_forge_c(
        "codec_zenoh_ext_zint",
        "codec_zenoh_ext_zint.c.h",
    );
}

#[test]
fn forge_c11_codec_zenoh_ext_zbuf() {
    assert_standalone_forge_c(
        "codec_zenoh_ext_zbuf",
        "codec_zenoh_ext_zbuf.c.h",
    );
}

#[test]
fn forge_c11_codec_zenoh_ext_entry() {
    assert_standalone_forge_c(
        "codec_zenoh_ext_entry",
        "codec_zenoh_ext_entry.c.h",
    );
}

#[test]
fn forge_c11_codec_zenoh_ext_envelope() {
    assert_standalone_forge_c(
        "codec_zenoh_ext_envelope",
        "codec_zenoh_ext_envelope.c.h",
    );
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
    assert_standalone_forge_c(
        "codec_dma_aligned_basic",
        "codec_dma_aligned_basic.c.h",
    );
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
    assert_standalone_forge_c(
        "codec_ext_encoding_info",
        "codec_ext_encoding_info.c.h",
    );
}

// ── Crossfile codec (C11) ───────────────────────────────────

#[test]
fn forge_c11_crossfile_procedure_codec() {
    assert_standalone_forge_c("crossfile_procedure_codec", "crossfile_procedure_codec.c.h");
}

#[test]
fn forge_c11_crossfile_procedure_codec_mutate() {
    assert_standalone_forge_c("crossfile_procedure_codec_mutate", "crossfile_procedure_codec_mutate.c.h");
}

#[test]
fn forge_c11_crossfile_procedure_filter() {
    assert_standalone_forge_c("crossfile_procedure_filter", "crossfile_procedure_filter.c.h");
}

#[test]
fn forge_c11_crossfile_validator_codec() {
    assert_standalone_forge_c("crossfile_validator_codec", "crossfile_validator_codec.c.h");
}

#[test]
fn forge_c11_crossfile_validator_filter() {
    assert_standalone_forge_c("crossfile_validator_filter", "crossfile_validator_filter.c.h");
}

#[test]
fn forge_c11_crossfile_validator_transform() {
    assert_standalone_forge_c("crossfile_validator_transform", "crossfile_validator_transform.c.h");
}

#[test]
fn forge_c11_crossfile_validator_condition() {
    assert_standalone_forge_c("crossfile_validator_condition", "crossfile_validator_condition.c.h");
}

#[test]
fn forge_c11_crossfile_validator_lookup() {
    assert_standalone_forge_c("crossfile_validator_lookup", "crossfile_validator_lookup.c.h");
}

#[test]
fn forge_c11_crossfile_validator_interpolation() {
    assert_standalone_forge_c("crossfile_validator_interpolation", "crossfile_validator_interpolation.c.h");
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
    assert_standalone_forge_c("validator_plausibility_only", "validator_plausibility_only.c.h");
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
    assert_standalone_forge_c("algorithm_const_fold_smoke", "algorithm_const_fold_smoke.c.h");
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
    assert_standalone_forge("validator_plausibility_only", "validator_plausibility_only.h");
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
    assert_standalone_forge_kotlin("validator_plausibility_only", "ValidatorPlausibilityOnly.kt");
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
    assert_standalone_forge_rust("validator_plausibility_only", "validator_plausibility_only.rs");
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
    assert_standalone_forge_go("validator_plausibility_only", "validator_plausibility_only.go");
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
    assert_standalone_forge_python("validator_plausibility_only", "validator_plausibility_only.py");
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
    assert_standalone_forge("crossfile_procedure_codec_mutate", "crossfile_procedure_codec_mutate.h");
}

#[test]
fn forge_crossfile_procedure_codec_mutate_kotlin() {
    assert_standalone_forge_kotlin("crossfile_procedure_codec_mutate", "CrossfileProcedureCodecMutate.kt");
}

#[test]
fn forge_crossfile_procedure_codec_mutate_rust() {
    assert_standalone_forge_rust("crossfile_procedure_codec_mutate", "crossfile_procedure_codec_mutate.rs");
}

#[test]
fn forge_crossfile_procedure_codec_mutate_go() {
    assert_standalone_forge_go("crossfile_procedure_codec_mutate", "crossfile_procedure_codec_mutate.go");
}

#[test]
fn forge_crossfile_procedure_codec_mutate_python() {
    assert_standalone_forge_python("crossfile_procedure_codec_mutate", "crossfile_procedure_codec_mutate.py");
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
    assert_standalone_forge_rust("crossfile_procedure_filter", "crossfile_procedure_filter.rs");
}

#[test]
fn forge_crossfile_procedure_filter_go() {
    assert_standalone_forge_go("crossfile_procedure_filter", "crossfile_procedure_filter.go");
}

#[test]
fn forge_crossfile_procedure_filter_python() {
    assert_standalone_forge_python("crossfile_procedure_filter", "crossfile_procedure_filter.py");
}

#[test]
fn forge_crossfile_validator_transform_cpp() {
    assert_standalone_forge("crossfile_validator_transform", "crossfile_validator_transform.h");
}

#[test]
fn forge_crossfile_validator_transform_kotlin() {
    assert_standalone_forge_kotlin("crossfile_validator_transform", "CrossfileValidatorTransform.kt");
}

#[test]
fn forge_crossfile_validator_transform_rust() {
    assert_standalone_forge_rust("crossfile_validator_transform", "crossfile_validator_transform.rs");
}

#[test]
fn forge_crossfile_validator_transform_go() {
    assert_standalone_forge_go("crossfile_validator_transform", "crossfile_validator_transform.go");
}

#[test]
fn forge_crossfile_validator_transform_python() {
    assert_standalone_forge_python("crossfile_validator_transform", "crossfile_validator_transform.py");
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
    assert_standalone_forge_rust("crossfile_validator_filter", "crossfile_validator_filter.rs");
}

#[test]
fn forge_crossfile_validator_filter_go() {
    assert_standalone_forge_go("crossfile_validator_filter", "crossfile_validator_filter.go");
}

#[test]
fn forge_crossfile_validator_filter_python() {
    assert_standalone_forge_python("crossfile_validator_filter", "crossfile_validator_filter.py");
}

#[test]
fn forge_crossfile_validator_condition_cpp() {
    assert_standalone_forge("crossfile_validator_condition", "crossfile_validator_condition.h");
}

#[test]
fn forge_crossfile_validator_condition_kotlin() {
    assert_standalone_forge_kotlin("crossfile_validator_condition", "CrossfileValidatorCondition.kt");
}

#[test]
fn forge_crossfile_validator_condition_rust() {
    assert_standalone_forge_rust("crossfile_validator_condition", "crossfile_validator_condition.rs");
}

#[test]
fn forge_crossfile_validator_condition_go() {
    assert_standalone_forge_go("crossfile_validator_condition", "crossfile_validator_condition.go");
}

#[test]
fn forge_crossfile_validator_condition_python() {
    assert_standalone_forge_python("crossfile_validator_condition", "crossfile_validator_condition.py");
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
    assert_standalone_forge_rust("crossfile_validator_lookup", "crossfile_validator_lookup.rs");
}

#[test]
fn forge_crossfile_validator_lookup_go() {
    assert_standalone_forge_go("crossfile_validator_lookup", "crossfile_validator_lookup.go");
}

#[test]
fn forge_crossfile_validator_lookup_python() {
    assert_standalone_forge_python("crossfile_validator_lookup", "crossfile_validator_lookup.py");
}

#[test]
fn forge_crossfile_validator_interpolation_cpp() {
    assert_standalone_forge("crossfile_validator_interpolation", "crossfile_validator_interpolation.h");
}

#[test]
fn forge_crossfile_validator_interpolation_kotlin() {
    assert_standalone_forge_kotlin("crossfile_validator_interpolation", "CrossfileValidatorInterpolation.kt");
}

#[test]
fn forge_crossfile_validator_interpolation_rust() {
    assert_standalone_forge_rust("crossfile_validator_interpolation", "crossfile_validator_interpolation.rs");
}

#[test]
fn forge_crossfile_validator_interpolation_go() {
    assert_standalone_forge_go("crossfile_validator_interpolation", "crossfile_validator_interpolation.go");
}

#[test]
fn forge_crossfile_validator_interpolation_python() {
    assert_standalone_forge_python("crossfile_validator_interpolation", "crossfile_validator_interpolation.py");
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
                            panic!(
                                "{scxml_name} ({lang:?}) {filename}: syn parse error: {e}"
                            );
                        }
                    }
                }
            }
            Err(e) => {
                panic!(
                    "{scxml_name} ({lang:?}): cross-file codegen failed: {e}"
                );
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
    assert_standalone_forge_kotlin(
        "codec_init_syn_body",
        "CodecInitSynBody.kt",
    );
}

/// RFC §5.B B5-γ Kotlin closure: variant parent threading carrier value.
/// The envelope's `when (val _b = this.body)` arms call
/// `_b.body.encode(this.header)` and the companion `decode(cursor, header)`
/// passes the just-decoded header local. Mirrors the Rust + Cpp goldens.
#[test]
fn forge_kotlin_codec_init_syn_envelope() {
    assert_standalone_forge_kotlin(
        "codec_init_syn_envelope",
        "CodecInitSynEnvelope.kt",
    );
}

/// RFC §5.B B5-γ Go closure: body codec with parent-flags dependency
/// emits `parentFlags byte` parameter on `Decode<Pascal>` / `Encode`;
/// `parent.<flag>` predicates compile to `(parentFlags & 0xNN) != 0`.
/// Go function parameters tolerate being unused, so no `_ = parentFlags`
/// guard is needed (mirrors Kotlin's `@Suppress("UNUSED_PARAMETER")`
/// but the Go compiler doesn't enforce the use).
#[test]
fn forge_go_codec_init_syn_body() {
    assert_standalone_forge_go(
        "codec_init_syn_body",
        "codec_init_syn_body.go",
    );
}

/// RFC §5.B B5-γ Go closure: variant parent threading carrier value.
/// The envelope's `switch { case s.Body.X != nil ... }` arms call
/// `s.Body.X.Encode(s.Header)` and the companion
/// `Decode<Body>(cursor, Header)` passes the just-decoded PascalCase
/// local. Mirrors the Rust + Cpp + Kotlin goldens.
#[test]
fn forge_go_codec_init_syn_envelope() {
    assert_standalone_forge_go(
        "codec_init_syn_envelope",
        "codec_init_syn_envelope.go",
    );
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
    assert_standalone_forge_c(
        "codec_init_syn_body",
        "codec_init_syn_body.c.h",
    );
}

/// RFC §5.B B5-γ C11 closure: variant parent threading carrier value.
/// Decode-site dispatcher reads the just-decoded carrier from
/// `out->header` (no separate local — C11 prefix decode writes
/// directly to the parent struct); encode-site dispatcher reads from
/// `self->header`. Mirrors the Rust + Cpp + Kotlin + Go goldens.
#[test]
fn forge_c11_codec_init_syn_envelope() {
    assert_standalone_forge_c(
        "codec_init_syn_envelope",
        "codec_init_syn_envelope.c.h",
    );
}

/// RFC §5.B B5-γ Python closure (final): body codec with parent-flags
/// dependency emits `parent_flags: int` parameter on `decode`/`encode`
/// (after the `cls, cursor` / `self` preceding args); `parent.<flag>`
/// predicates compile to `(parent_flags & 0xNN) != 0`. `_ = parent_flags`
/// defensive guard suppresses unused-variable warnings (mirrors Rust's
/// `let _ = parent_flags;` and Cpp's `(void)parent_flags;`).
#[test]
fn forge_python_codec_init_syn_body() {
    assert_standalone_forge_python(
        "codec_init_syn_body",
        "codec_init_syn_body.py",
    );
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
    assert_standalone_forge_python(
        "codec_init_syn_envelope",
        "codec_init_syn_envelope.py",
    );
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
    assert_standalone_forge_kotlin(
        "codec_init_cookie_body",
        "CodecInitCookieBody.kt",
    );
}

/// RFC §5.B B5-δ Surface F (Kotlin): Scout/Hello/Init zid codec.
/// `length-arith="+1"` emits `(zidLenM1.toInt() + 1)` for the byte
/// count.
#[test]
fn forge_kotlin_codec_scout_zid_body() {
    assert_standalone_forge_kotlin(
        "codec_scout_zid_body",
        "CodecScoutZidBody.kt",
    );
}

/// RFC §5.B B5-δ Surfaces D + E (Go): Init body cookie codec.
/// `CookieSize *uint16` (pointer = presence wrapper for VLE u16);
/// `Cookie []byte` (slice nilness encodes presence). Helper deref
/// emits `int(*CookieSize)` inside the gated branch.
#[test]
fn forge_go_codec_init_cookie_body() {
    assert_standalone_forge_go(
        "codec_init_cookie_body",
        "codec_init_cookie_body.go",
    );
}

/// RFC §5.B B5-δ Surface F (Go): Scout/Hello/Init zid codec.
/// `length-arith="+1"` emits `(int(ZidLenM1) + 1)` for the byte count.
#[test]
fn forge_go_codec_scout_zid_body() {
    assert_standalone_forge_go(
        "codec_scout_zid_body",
        "codec_scout_zid_body.go",
    );
}

/// RFC §5.B B5-δ Surfaces D + E (C11): Init body cookie codec.
/// C11 has no Option wrapper — sibling `cookie_size` is always-bound
/// on the struct (zero on absent branch). Helper reads through
/// `out->cookie_size` regardless of gating; the carrier bit is the
/// presence source.
#[test]
fn forge_c11_codec_init_cookie_body() {
    assert_standalone_forge_c(
        "codec_init_cookie_body",
        "codec_init_cookie_body.c.h",
    );
}

/// RFC §5.B B5-δ Surface F (C11): Scout/Hello/Init zid codec.
/// `length-arith="+1"` emits `_n = (size_t)((int64_t)out->zid_len_m1 + 1)`
/// for decode; the encode-loop's upper bound widens symmetrically to
/// `_bi < (size_t)((int64_t)self->zid_len_m1 + 1)` so the wire-correct
/// number of bytes is written.
#[test]
fn forge_c11_codec_scout_zid_body() {
    assert_standalone_forge_c(
        "codec_scout_zid_body",
        "codec_scout_zid_body.c.h",
    );
}

/// RFC §5.B B5-δ Surfaces D + E (Python): Init body cookie codec.
/// `cookie_size: Optional[int]` and `cookie: Optional[bytes]` — inside
/// the gated branch the int local is guaranteed non-None by the same
/// predicate. Helper reads `cookie_size` directly (no unwrap syntax).
#[test]
fn forge_python_codec_init_cookie_body() {
    assert_standalone_forge_python(
        "codec_init_cookie_body",
        "codec_init_cookie_body.py",
    );
}

/// RFC §5.B B5-δ Surface F (Python): Scout/Hello/Init zid codec.
/// `length-arith="+1"` emits `_n = (zid_len_m1 + 1)` for the byte
/// count — Python's arbitrary-precision int handles `+1` without
/// overflow.
#[test]
fn forge_python_codec_scout_zid_body() {
    assert_standalone_forge_python(
        "codec_scout_zid_body",
        "codec_scout_zid_body.py",
    );
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
        let scxml = format!(r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       xmlns:sce="http://sce.dev/ext"
       sce:kind="codec" sce:default-endian="big" name="bad_arith_range">
  <datamodel>
    <sce:field id="len" sce:type="uint8" sce:byte="0" sce:bit-size="8"/>
    <sce:field id="payload" sce:type="bytes" sce:byte="1" sce:bit-size="length-ref"
               sce:length-field="len" sce:length-arith="{bad}" sce:max-size="16"/>
  </datamodel>
</scxml>"#);
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
            "parent flag bit-mismatch must reject with codec/parent-flag-mismatch"
        ),
        Err(e) => e,
    };
    assert!(
        matches!(
            &err.error,
            ForgeError::Validation(ValidationError::CodecParentFlagMismatch { .. })
        ),
        "must surface CodecParentFlagMismatch; got: {:?}",
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
