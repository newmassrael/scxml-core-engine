// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! A Swift algorithm holds a union (docs/adr/0092, Landing order step 5), held
//! to the documents under `sce-build/tests/fixtures/union/` that the Rust,
//! Kotlin, Go and C++ lanes are held to
//! (`a_kotlin_algorithm_narrows_a_union_by_its_tag.rs`):
//!
//! * `algorithm_shape_extent.scxml` takes a `union:Shape`, the enum the union's
//!   own unit declares, and reads each case's fields where a test of the tag
//!   proves the case: the tag by the enum's `kind`, a case's field through the
//!   unit's `proven<Case>`. Its three tests each return, so no case is left
//!   after them (section 6.1 item 4): Swift closes the body with a
//!   `preconditionFailure`, which returns `Never`, at the body's own indentation.
//! * `algorithm_shape_grown.scxml` returns a union: the one it was given, by
//!   name, or a union local it builds as the case its tag names, a `let` of the
//!   enum's type holding the case around the record's payload struct.
//! * Both type-check under `swiftc -warnings-as-errors` beside the union's unit
//!   and the units it names, with a file that calls each.

use std::path::{Path, PathBuf};
use std::process::Command;

use sce_build::forge::error::{ForgeError, Located};
use sce_build::generator::{GeneratedOutput, Language};
use sce_build::ForgeCompileOptions;

fn fixtures() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/union")
}

/// What Swift generates for the document at `path`.
fn swift(path: &Path) -> Result<GeneratedOutput, Located<ForgeError>> {
    sce_build::compile_forge_file(path, Language::Swift, &[], &ForgeCompileOptions::default())
}

/// The one Swift file the document at `path` generates, as `(file, text)`.
fn unit_at(path: &Path) -> (String, String) {
    let output = swift(path)
        .unwrap_or_else(|e| panic!("{} does not generate on Swift: {}", path.display(), e.error));
    let mut files: Vec<(String, String)> = output
        .files
        .into_iter()
        .filter(|(file, _)| file.ends_with(".swift"))
        .collect();
    assert_eq!(
        files.len(),
        1,
        "{}: one Swift unit, got {files:?}",
        path.display()
    );
    files.remove(0)
}

/// The one Swift file the fixture `name` generates, as `(file, text)`.
fn unit(name: &str) -> (String, String) {
    unit_at(&fixtures().join(name))
}

/// Whether `text` holds `needle`, with `text` in the failure.
fn holds(text: &str, needle: &str) {
    assert!(text.contains(needle), "the unit holds `{needle}`:\n{text}");
}

const SHAPE: &str = "UnionShapeMinimal";

#[test]
fn a_swift_algorithm_reads_each_case_of_its_union_where_its_tag_is_tested() {
    let (file, text) = unit("algorithm_shape_extent.scxml");
    assert_eq!(file, "AlgorithmShapeExtent.swift");
    holds(
        &text,
        &format!("public static func algorithmShapeExtent(_ shape: {SHAPE}) -> UInt16"),
    );
    // The tag is read by the enum's property and compared with the algorithm's
    // own enum.
    holds(&text, "shape.kind");
    holds(&text, "ShapeKind.dot");
    holds(&text, "ShapeKind.rect");
    holds(&text, "shape.provenCircle.radius");
    holds(&text, "shape.provenRect.width");
    holds(&text, "shape.provenRect.height");
    // Every case returned before the end of the body, which no path reaches.
    holds(
        &text,
        "\n        preconditionFailure(\"algorithm_shape_extent: every case of its union returned \
         before the end of its body\")",
    );
}

#[test]
fn a_swift_algorithm_returns_a_union_it_was_given_or_built() {
    let (_, text) = unit("algorithm_shape_grown.scxml");
    holds(&text, &format!("_ radius: UInt16) -> {SHAPE}"));
    holds(&text, "return shape\n");
    holds(
        &text,
        &format!(
            "let grown: {SHAPE} = {SHAPE}.circle(ShapeRadiusMinimalPayload(radius: radius))\n"
        ),
    );
    holds(&text, "return grown\n");
    // Its last statement returns, so the body needs nothing after it.
    assert!(!text.contains("preconditionFailure("), "{text}");
}

/// A file that calls both algorithms, the one's union handed to the other.
const PROBE: &str = r#"func grownExtent(_ shape: UnionShapeMinimal) -> UInt16 {
    AlgorithmShapeExtent.algorithmShapeExtent(AlgorithmShapeGrown.algorithmShapeGrown(shape, 3))
}

func dot() -> UnionShapeMinimal {
    UnionShapeMinimal.dot
}
"#;

/// The part of the forge runtime a unit or an algorithm may name: the failure a
/// may-fail algorithm throws (docs/adr/0099). A stand-in module is enough to
/// type-check the units against; the runtime's own tests hold the real one.
const RUNTIME_STAND_IN: &str = "public enum AlgorithmError: Error {\n    case overflow\n}\n";

/// The algorithms beside a file that calls each, type-checked with the union's
/// unit and the units it names: the reads, the build, the returns and the closed
/// end are Swift the compiler accepts with every warning an error.
#[test]
fn the_swift_of_an_algorithm_over_a_union_type_checks_under_warnings_as_errors() {
    let Some(swiftc) = sce_build::toolchain::require_any_or_skip(
        &["swiftc"],
        "type-check a generated algorithm over a union",
    ) else {
        return;
    };
    let dir = tempfile::tempdir().expect("a scratch directory");
    let runtime = dir.path().join("SceForgeRuntime.swift");
    std::fs::write(&runtime, RUNTIME_STAND_IN).expect("write the stand-in runtime");
    let modules = dir.path().join("modules");
    std::fs::create_dir(&modules).expect("a module directory");
    let built = Command::new(&swiftc)
        .args([
            "-emit-module",
            "-module-name",
            "SceForgeRuntime",
            "-parse-as-library",
        ])
        .arg("-emit-module-path")
        .arg(modules.join("SceForgeRuntime.swiftmodule"))
        .arg(&runtime)
        .output()
        .expect("swiftc runs");
    assert!(
        built.status.success(),
        "the stand-in runtime does not build:\n{}",
        String::from_utf8_lossy(&built.stderr)
    );
    let mut sources = Vec::new();
    for name in [
        "shape_kind.scxml",
        "shape_radius_minimal.scxml",
        "shape_rect_minimal.scxml",
        "union_shape_minimal.scxml",
        "algorithm_shape_extent.scxml",
        "algorithm_shape_grown.scxml",
    ] {
        let (file, text) = unit(name);
        let path = dir.path().join(file);
        std::fs::write(&path, text).expect("write the unit");
        sources.push(path);
    }
    let probe = dir.path().join("Probe.swift");
    std::fs::write(&probe, PROBE).expect("write the probe");
    sources.push(probe);
    let out = Command::new(&swiftc)
        .args([
            "-typecheck",
            "-warnings-as-errors",
            "-swift-version",
            "6",
            "-I",
        ])
        .arg(&modules)
        .args(&sources)
        .output()
        .expect("swiftc runs");
    assert!(
        out.status.success(),
        "the generated Swift does not type-check:\n{}",
        String::from_utf8_lossy(&out.stderr)
    );
}
