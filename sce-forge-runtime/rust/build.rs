// SPDX-License-Identifier: LGPL-2.1-or-later OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// build.rs for the cross-language numerical conformance harness (Rust half).
//
// Two code-generation steps, both in-process via sce_build:
//
//   1. For every fixture listed in tests/forge/conformance/fixtures.json,
//      invoke compile_forge_with_imports_validated() on the SCXML file and
//      write the generated Rust fixture code into OUT_DIR.
//
//   2. Render the per-fixture test harness from
//      tools/codegen/templates/forge/rust/conformance/harness.rs.jinja2 and
//      write it to OUT_DIR/numerical_conformance_generated.rs.
//
// The integration test under tests/numerical_conformance.rs is a one-line
// shim that include!()'s the generated harness, so no committed Rust test
// code exists — the single source of truth is the fixture catalog and the
// conformance template.

use sce_build::{
    compile_forge_with_imports_validated,
    conformance::{self, Manifest},
    generator::Language,
};
use std::path::Path;

fn main() {
    let out_dir = std::env::var("OUT_DIR").expect("OUT_DIR not set by cargo");
    let manifest_dir =
        std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR not set by cargo");
    let resource_dir = Path::new(&manifest_dir)
        .join("../../tests/forge/resources")
        .canonicalize()
        .expect("canonicalize tests/forge/resources");
    let manifest_path = Path::new(&manifest_dir)
        .join("../../tests/forge/conformance/fixtures.json")
        .canonicalize()
        .expect("canonicalize fixtures.json");
    let template_base = Path::new(&manifest_dir)
        .join("../../tools/codegen/templates")
        .canonicalize()
        .expect("canonicalize tools/codegen/templates");

    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed={}", resource_dir.display());
    println!("cargo:rerun-if-changed={}", manifest_path.display());
    println!(
        "cargo:rerun-if-changed={}",
        template_base.join("forge/rust").display()
    );

    // Step 1: fixture catalog — single source of truth for which SCXML files
    // participate in the conformance harness across all 5 languages.
    let manifest = Manifest::load(&manifest_path)
        .unwrap_or_else(|e| panic!("load conformance manifest: {e}"));

    // Step 2: generate each fixture's Rust code into OUT_DIR.
    for fixture in &manifest.fixtures {
        let scxml_path = resource_dir.join(format!("{}.scxml", fixture.name));
        let content = std::fs::read_to_string(&scxml_path)
            .unwrap_or_else(|e| panic!("read {}: {e}", scxml_path.display()));

        let output = compile_forge_with_imports_validated(
            &content,
            &fixture.name,
            Language::Rust,
            &resource_dir,
        )
        .unwrap_or_else(|e| panic!("sce-build codegen failed for {}: {e}", fixture.name));

        for (filename, code) in output.files {
            let target = Path::new(&out_dir).join(&filename);
            std::fs::write(&target, &code)
                .unwrap_or_else(|e| panic!("write {}: {e}", target.display()));
        }
    }

    // Step 3: render the cross-language test harness from the shared template.
    let harness =
        conformance::render_harness(&manifest, Language::Rust, &template_base, &resource_dir)
            .unwrap_or_else(|e| panic!("render conformance harness: {e}"));
    let harness_path =
        Path::new(&out_dir).join(conformance::harness_filename(Language::Rust));
    std::fs::write(&harness_path, harness)
        .unwrap_or_else(|e| panic!("write {}: {e}", harness_path.display()));
}
