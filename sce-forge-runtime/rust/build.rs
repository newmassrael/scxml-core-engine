// SPDX-License-Identifier: LGPL-2.1-or-later OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// build.rs for the cross-language numerical conformance harness (Rust half).
//
// Invokes sce_build::compile_forge_with_imports_validated() on the fixture
// SCXML files listed in FIXTURES and writes the generated Rust code into
// OUT_DIR, where the integration test under tests/numerical_conformance.rs
// pulls them in via include!. This matches the standard Rust codegen pattern
// used by prost-build, tonic-build, and sqlx-macros: the build.rs of the
// consuming crate calls the generator library in-process.
//
// The single source of truth is the SCXML files; no committed goldens are
// consumed. If an SCXML file, a jinja2 template, or sce-build itself changes,
// cargo reruns this script and the generated fixtures are refreshed before
// the test binary compiles.

use sce_build::{compile_forge_with_imports_validated, generator::Language};
use std::path::Path;

const FIXTURES: &[&str] = &[
    "filter_moving_average",
    "filter_debounce",
    "interpolation_1d_linear",
    "interpolation_2d_bilinear",
    "observer_coolant",
];

fn main() {
    let out_dir = std::env::var("OUT_DIR").expect("OUT_DIR not set by cargo");
    let manifest_dir =
        std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR not set by cargo");
    let resource_dir = Path::new(&manifest_dir)
        .join("../../tests/forge/resources")
        .canonicalize()
        .expect("canonicalize tests/forge/resources");
    let template_dir = Path::new(&manifest_dir)
        .join("../../tools/codegen/templates/rust")
        .canonicalize()
        .ok();

    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed={}", resource_dir.display());
    if let Some(ref dir) = template_dir {
        println!("cargo:rerun-if-changed={}", dir.display());
    }

    for fixture in FIXTURES {
        let scxml_path = resource_dir.join(format!("{fixture}.scxml"));
        let content = std::fs::read_to_string(&scxml_path)
            .unwrap_or_else(|e| panic!("read {}: {e}", scxml_path.display()));

        let output = compile_forge_with_imports_validated(
            &content,
            fixture,
            Language::Rust,
            &resource_dir,
        )
        .unwrap_or_else(|e| panic!("sce-build codegen failed for {fixture}: {e}"));

        for (filename, code) in output.files {
            let target = Path::new(&out_dir).join(&filename);
            std::fs::write(&target, &code)
                .unwrap_or_else(|e| panic!("write {}: {e}", target.display()));
        }
    }
}
