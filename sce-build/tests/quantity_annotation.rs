// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// NL→IR Mapping Roadmap Item 4 — physical-quantity annotation integration
// tests.
//
// These exercise the end-to-end Item 4 surface on three fixtures:
//
//   * `transform_quantity_celsius.scxml`  — Transform with a quantity-
//     annotated input. Compiles cleanly across all six backends; the
//     generated code carries the quantity doc-comment block.
//
//   * `codec_quantity_temperature.scxml`  — Codec field with quantity.
//     Each backend emits a `<field>_phys()` getter and a matching
//     setter (or the per-language equivalent: `<id>Phys()` for Kotlin,
//     `<Id>Phys()` for Go).
//
//   * `transform_quantity_unit_mismatch.scxml` — Negative fixture. The
//     `forge::quantity_check` walker must reject the mixed-unit
//     arithmetic with `validation/cross-kind-type-mismatch`
//     (via the typed `ValidationError::QuantityUnitMismatch` variant).
//
// We intentionally do NOT add these fixtures to the byte-identical
// golden-comparison suite under `tests/forge/conformance/` — the per-
// language emit surface (6 codec + 6 transform templates) would
// triple the golden count for limited extra coverage, and the focused
// `contains(...)` assertions below catch every regression
// `quantity_codegen::build_accessor_payload` can introduce. Atomic B
// (a future task) will fold these into the golden suite if a real
// consumer demands byte-stable physical-accessor output.

use std::path::PathBuf;

fn repo_root() -> PathBuf {
    let cargo_manifest_dir = env!("CARGO_MANIFEST_DIR");
    PathBuf::from(cargo_manifest_dir)
        .parent()
        .unwrap()
        .to_path_buf()
}

fn fixture_path(name: &str) -> PathBuf {
    repo_root().join("tests/forge/resources").join(name)
}

fn compile_first_file(name: &str, lang: sce_build::generator::Language) -> Result<String, String> {
    let path = fixture_path(name);
    let content =
        std::fs::read_to_string(&path).map_err(|e| format!("read {}: {e}", path.display()))?;
    let base_dir = path.parent().unwrap();
    let stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or(name);

    let mut options = sce_build::ForgeCompileOptions::default();
    if matches!(lang, sce_build::generator::Language::Go) {
        // The forge_conformance suite pins this prefix; mirror it so
        // generated Go imports resolve identically.
        options.go_module_prefix = Some("sce/generated".to_string());
    }

    sce_build::compile_forge_with_imports(
        &content,
        sce_build::DocumentLabel::symmetric(stem),
        lang,
        base_dir,
        &options,
    )
    .map(|out| out.files[0].1.clone())
    .map_err(|e| e.to_string())
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// Transform — positive case (Item 4 doc-comment block)
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

const TRANSFORM_FIXTURE: &str = "transform_quantity_celsius.scxml";

fn assert_doc_block_contains(code: &str, lang: sce_build::generator::Language) {
    // The Item 4 marker phrase is the same across all six backends —
    // the only variation is the comment-syntax prefix that wraps it.
    assert!(
        code.contains("NL→IR Mapping Roadmap Item 4"),
        "{lang:?}: generated code missing Item-4 doc-comment marker\n--- code ---\n{code}",
    );
    assert!(
        code.contains("celsius"),
        "{lang:?}: generated code missing celsius unit reference",
    );
    assert!(
        code.contains("raw_temp") || code.contains("rawTemp") || code.contains("RawTemp"),
        "{lang:?}: generated code missing raw_temp identifier reference",
    );
}

#[test]
fn transform_quantity_rust() {
    let code = compile_first_file(TRANSFORM_FIXTURE, sce_build::generator::Language::Rust)
        .expect("Rust transform compile");
    assert_doc_block_contains(&code, sce_build::generator::Language::Rust);
    // Body expression should retain the integer raw_temp identifier
    // and the literal `0.5` (untyped float adopts the f64 base, the
    // emitter then keeps the literal as-is since the surrounding
    // arithmetic is float-typed).
    assert!(
        code.contains("raw_temp") && code.contains("0.5"),
        "Rust body should reference raw_temp and the scale literal — got:\n{code}",
    );
}

#[test]
fn transform_quantity_cpp() {
    let code = compile_first_file(TRANSFORM_FIXTURE, sce_build::generator::Language::Cpp)
        .expect("Cpp transform compile");
    assert_doc_block_contains(&code, sce_build::generator::Language::Cpp);
}

#[test]
fn transform_quantity_c11() {
    let code = compile_first_file(TRANSFORM_FIXTURE, sce_build::generator::Language::C11)
        .expect("C11 transform compile");
    assert_doc_block_contains(&code, sce_build::generator::Language::C11);
}

#[test]
fn transform_quantity_kotlin() {
    let code = compile_first_file(TRANSFORM_FIXTURE, sce_build::generator::Language::Kotlin)
        .expect("Kotlin transform compile");
    assert_doc_block_contains(&code, sce_build::generator::Language::Kotlin);
}

#[test]
fn transform_quantity_go() {
    let code = compile_first_file(TRANSFORM_FIXTURE, sce_build::generator::Language::Go)
        .expect("Go transform compile");
    assert_doc_block_contains(&code, sce_build::generator::Language::Go);
}

#[test]
fn transform_quantity_python() {
    let code = compile_first_file(TRANSFORM_FIXTURE, sce_build::generator::Language::Python)
        .expect("Python transform compile");
    assert_doc_block_contains(&code, sce_build::generator::Language::Python);
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// Codec — positive case (raw↔physical accessor pair)
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

const CODEC_FIXTURE: &str = "codec_quantity_temperature.scxml";

fn assert_codec_accessors(
    code: &str,
    lang: sce_build::generator::Language,
    expected_getter: &str,
    expected_setter: &str,
) {
    assert!(
        code.contains(expected_getter),
        "{lang:?}: codec missing expected getter `{expected_getter}`\n--- code ---\n{code}",
    );
    assert!(
        code.contains(expected_setter),
        "{lang:?}: codec missing expected setter `{expected_setter}`\n--- code ---\n{code}",
    );
    assert!(
        code.contains("celsius"),
        "{lang:?}: codec missing celsius unit reference",
    );
}

#[test]
fn codec_quantity_rust() {
    let code = compile_first_file(CODEC_FIXTURE, sce_build::generator::Language::Rust)
        .expect("Rust codec compile");
    assert_codec_accessors(
        &code,
        sce_build::generator::Language::Rust,
        "raw_temp_phys",
        "set_raw_temp_phys",
    );
}

#[test]
fn codec_quantity_cpp() {
    let code = compile_first_file(CODEC_FIXTURE, sce_build::generator::Language::Cpp)
        .expect("Cpp codec compile");
    assert_codec_accessors(
        &code,
        sce_build::generator::Language::Cpp,
        "raw_temp_phys",
        "set_raw_temp_phys",
    );
}

#[test]
fn codec_quantity_c11() {
    let code = compile_first_file(CODEC_FIXTURE, sce_build::generator::Language::C11)
        .expect("C11 codec compile");
    // C11 prefixes accessors with the snake-cased struct name so two
    // codecs with same-named fields don't clash in a single TU.
    assert!(
        code.contains("_raw_temp_phys"),
        "C11: codec missing struct-prefixed raw_temp_phys accessor\n--- code ---\n{code}",
    );
    assert!(
        code.contains("_set_raw_temp_phys"),
        "C11: codec missing struct-prefixed set_raw_temp_phys accessor",
    );
}

#[test]
fn codec_quantity_kotlin() {
    let code = compile_first_file(CODEC_FIXTURE, sce_build::generator::Language::Kotlin)
        .expect("Kotlin codec compile");
    assert_codec_accessors(
        &code,
        sce_build::generator::Language::Kotlin,
        "rawTempPhys",
        "setRawTempPhys",
    );
}

#[test]
fn codec_quantity_go() {
    let code = compile_first_file(CODEC_FIXTURE, sce_build::generator::Language::Go)
        .expect("Go codec compile");
    assert_codec_accessors(
        &code,
        sce_build::generator::Language::Go,
        "RawTempPhys",
        "SetRawTempPhys",
    );
}

#[test]
fn codec_quantity_python() {
    let code = compile_first_file(CODEC_FIXTURE, sce_build::generator::Language::Python)
        .expect("Python codec compile");
    assert_codec_accessors(
        &code,
        sce_build::generator::Language::Python,
        "raw_temp_phys",
        "set_raw_temp_phys",
    );
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// Negative — unit-mismatch arithmetic rejected
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

#[test]
fn transform_unit_mismatch_is_rejected() {
    let err = compile_first_file(
        "transform_quantity_unit_mismatch.scxml",
        sce_build::generator::Language::Rust,
    )
    .expect_err("mixed-unit arithmetic must be rejected");
    // The error chain renders through `ValidationError::QuantityUnitMismatch`,
    // whose Display contains both unit strings and the operator. Any of
    // those substrings prove the right typed diagnostic surfaced.
    assert!(
        err.contains("celsius") && err.contains("kelvin"),
        "diagnostic should name both colliding units; got: {err}",
    );
    assert!(
        err.contains("'+'"),
        "diagnostic should name the offending operator; got: {err}",
    );
}
