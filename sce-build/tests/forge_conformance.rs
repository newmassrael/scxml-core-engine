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

/// Generate C++ from a standalone forge SCXML and compare against expected output.
fn assert_standalone_forge(scxml_name: &str, expected_filename: &str) {
    let scxml_path = resource_dir().join(format!("{scxml_name}.scxml"));
    let content = std::fs::read_to_string(&scxml_path)
        .unwrap_or_else(|e| panic!("Cannot read {}: {e}", scxml_path.display()));

    let stem = scxml_name;
    let output = sce_build::compile_forge_from_string(
        &content,
        stem,
        sce_build::generator::Language::Cpp,
    )
    .unwrap_or_else(|e| panic!("Forge codegen failed for {scxml_name}: {e}"));

    assert!(!output.files.is_empty(), "No output for {scxml_name}");

    let (_, generated) = &output.files[0];
    let expected_path = expected_dir().join(expected_filename);
    let expected = std::fs::read_to_string(&expected_path)
        .unwrap_or_else(|e| panic!("Cannot read expected {}: {e}", expected_path.display()));

    assert_eq!(
        generated.trim(),
        expected.trim(),
        "Output mismatch for {scxml_name}\n--- expected: {}\n+++ generated",
        expected_path.display()
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

// ── Inline kind conformance ──────────────────────────────────

#[test]
fn forge_inline_mixed() {
    assert_inline_kinds("inline_mixed");
}
