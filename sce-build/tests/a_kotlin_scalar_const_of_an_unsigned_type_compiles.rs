// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
//! A scalar `<sce:const type="uint16" init="0x1021"/>` of an algorithm is a
//! constant in every backend, and a value of its declared type where an
//! expression reads it.
//!
//! Kotlin wrote it as `const val POLY: UShort = (4129).toUShort()`, which is a
//! call and so not a constant (`const 'val' initializer must be a constant
//! value`), and read it in `(word.toInt() and POLY)`, an `Int` and a `UShort`,
//! because a scalar constant was typed `Unknown` and so never converted. No
//! committed output held the fixture, so nothing compiled it; the Kotlin name
//! oracle found it as a control that did not build.

use std::path::Path;
use std::process::Command;

use sce_build::generator::Language;
use sce_build::{compile_forge_with_imports, DocumentLabel, ForgeCompileOptions};

fn kotlin_for_the_fixture() -> String {
    let resources = Path::new(env!("CARGO_MANIFEST_DIR")).join("../tests/forge/resources");
    let text = std::fs::read_to_string(resources.join("algorithm_const_scalar_init.scxml"))
        .expect("the fixture");
    let output = compile_forge_with_imports(
        &text,
        DocumentLabel::symmetric("algorithm_const_scalar_init"),
        Language::Kotlin,
        &resources,
        &ForgeCompileOptions::default(),
    )
    .unwrap_or_else(|e| panic!("the fixture does not generate: {}", e.error));
    output
        .files
        .iter()
        .find(|(file, _)| file.ends_with(".kt"))
        .map(|(_, source)| source.clone())
        .expect("a Kotlin unit")
}

#[test]
fn a_kotlin_scalar_of_an_unsigned_type_is_an_unsigned_literal_and_is_converted() {
    let source = kotlin_for_the_fixture();
    assert!(
        source.contains("const val POLY: UShort = 4129u\n"),
        "an unsigned scalar constant is not a literal:\n{source}"
    );
    assert!(
        source.contains("(word.toInt() and POLY.toInt()).toUShort()"),
        "the constant is not converted beside an Int:\n{source}"
    );
}

#[test]
fn the_kotlin_of_a_scalar_const_compiles_under_werror() {
    let Some(kotlinc) =
        sce_build::toolchain::require_any_or_skip(&["kotlinc"], "compile a generated scalar const")
    else {
        return;
    };
    let dir = tempfile::tempdir().expect("a scratch directory");
    let file = dir.path().join("AlgorithmConstScalarInit.kt");
    std::fs::write(&file, kotlin_for_the_fixture()).expect("write the unit");
    let out = Command::new(&kotlinc)
        .arg("-Werror")
        .arg("-d")
        .arg(dir.path().join("classes"))
        .arg(&file)
        .output()
        .expect("kotlinc runs");
    assert!(
        out.status.success(),
        "the generated Kotlin does not compile:\n{}",
        String::from_utf8_lossy(&out.stderr)
    );
}
