// SPDX-License-Identifier: LGPL-2.1-or-later OR LicenseRef-SCE-Commercial
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

fn template_dir() -> std::path::PathBuf {
    project_root().join("tools/codegen/templates")
}

fn resource_dir() -> std::path::PathBuf {
    project_root().join("tests/forge/resources")
}

fn expected_dir() -> std::path::PathBuf {
    project_root().join("tests/forge/expected")
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
    let output = sce_build::compile_forge_with_imports_validated(&content, stem, language, base_dir)
        .unwrap_or_else(|e| panic!("Forge codegen failed for {scxml_name} ({language:?}): {e}"));

    assert!(!output.files.is_empty(), "No output for {scxml_name}");

    let (_, generated) = &output.files[0];
    let expected_path = expected_dir().join(expected_filename);
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

/// Strip the `// From: ...` comment line (path-dependent) for comparison.
fn normalize_for_comparison(code: &str) -> String {
    code.lines()
        .filter(|line| !line.starts_with("// From:"))
        .collect::<Vec<_>>()
        .join("\n")
}

/// Generate C++ from a statechart with inline kinds and verify inline kind code appears.
fn assert_inline_kinds(scxml_name: &str) {
    let scxml_path = resource_dir().join(format!("{scxml_name}.scxml"));

    let output = sce_build::compile_scxml_lang(
        scxml_path.to_str().unwrap(),
        &template_dir(),
        sce_build::generator::Language::Cpp,
    )
    .unwrap_or_else(|e| panic!("Statechart codegen failed for {scxml_name}: {e}"));

    let header = &output.files[0].1;
    assert!(
        header.contains("SCE Forge: Inline"),
        "Inline kind code missing in {scxml_name} output"
    );

    // Compare against expected output (path-agnostic)
    let expected_path = expected_dir().join(format!("{scxml_name}_sm.h"));
    if expected_path.exists() {
        let expected = std::fs::read_to_string(&expected_path).unwrap();
        assert_eq!(
            normalize_for_comparison(header).trim(),
            normalize_for_comparison(&expected).trim(),
            "Output mismatch for {scxml_name}\n--- expected: {}\n+++ generated",
            expected_path.display()
        );
    }
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

// ══════════════════════════════════════════════════════════════
// ── Rust conformance tests ───────────────────────────────────
// ══════════════════════════════════════════════════════════════

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

// ── Inline kind conformance ──────────────────────────────────

#[test]
fn forge_inline_mixed() {
    assert_inline_kinds("inline_mixed");
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
            name,
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
            name,
            sce_build::generator::Language::Python,
        )
        .unwrap();
        let (py_filename, py_code) = &py_out.files[0];
        let py_path = expected_dir().join(py_filename);
        std::fs::write(&py_path, py_code).unwrap();
        println!("  Py: {}", py_path.display());
    }
}
