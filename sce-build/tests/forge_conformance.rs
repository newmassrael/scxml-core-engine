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

#[test]
fn forge_algorithm_crc16_cpp() {
    assert_standalone_forge("algorithm_crc16", "algorithm_crc16.h");
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

/// RFC §5.B B1-β trunk gate: only Rust + Cpp emit variant codecs in
/// trunk; Kotlin / Go / C11 / Python land in B1-β closures. Until
/// then, `compile_forge_with_imports` must reject with
/// `generate/unsupported-feature` naming the language so authors
/// don't ship silently-broken codegen.
#[test]
fn forge_codec_variant_kotlin_gate_rejects_until_closure() {
    use sce_build::forge::error::{ForgeError, GenerateError};

    let scxml_path = resource_dir().join("codec_variant_dispatch.scxml");
    let content = std::fs::read_to_string(&scxml_path).expect("read variant fixture");
    let result = sce_build::compile_forge_with_imports(
        &content,
        sce_build::DocumentLabel::symmetric("codec_variant_dispatch"),
        sce_build::generator::Language::Kotlin,
        scxml_path.parent().unwrap(),
        &sce_build::ForgeCompileOptions::default(),
    );
    let err = match result {
        Ok(_) => panic!(
            "B1-β trunk must gate <sce:variant> on Kotlin; codegen would otherwise ship broken output"
        ),
        Err(e) => e,
    };
    let inner = err.error;
    assert!(
        matches!(
            inner,
            ForgeError::Generate(GenerateError::UnsupportedFeature(ref msg))
                if msg.contains("codec_variant_dispatch") && msg.contains("Kotlin")
        ),
        "must surface as GenerateError::UnsupportedFeature naming the codec and language; got: {inner:?}"
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

// ── Algorithm (Kotlin, RFC §5.A — post-A6 matrix follow-up) ─

#[test]
fn forge_kotlin_algorithm_crc16() {
    assert_standalone_forge_kotlin("algorithm_crc16", "AlgorithmCrc16.kt");
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

// ── Algorithm (Go, RFC §5.A — post-A6 matrix follow-up) ────

#[test]
fn forge_go_algorithm_crc16() {
    assert_standalone_forge_go("algorithm_crc16", "algorithm_crc16.go");
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

// ── Algorithm (Python, RFC §5.A — post-A6 matrix follow-up) ─

#[test]
fn forge_python_algorithm_crc16() {
    assert_standalone_forge_python("algorithm_crc16", "algorithm_crc16.py");
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
