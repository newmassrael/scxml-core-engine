// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// SCE Protocol-Synthesis RFC §synth-5-O — ownership-boundary
// walker integration fixture.
//
// `forge::sourcemap::validate_emitted_files_have_markers` runs at the
// end of every successful `cmd_generate` / `cmd_generate_w3c` and
// enforces ARCHITECTURE.md "Traceability Ownership Boundary": every
// file SCE emitted (one carrying a §synth-6.2.6 drift header) must contain
// at least one `SCE-MAP:` marker line. External meta-generator output
// (no drift header) is silently out-of-scope.
//
// Three contracts pinned here:
//   1. Normal generate emits drift-headered files that already carry
//      the marker family — walker passes.
//   2. Stripping the marker line from a tracked emitted file (simulates
//      a template regression dropping its `sce_map_marker` macro call)
//      fires `traceability/meta-generated-source-line-marker-missing`
//      on the next walker run.
//   3. A drift-headerless file (simulates protoc / bindgen output) is
//      silently skipped — proves the boundary, not a recursive
//      ownership chain.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

const FIXTURE: &str = r#"<?xml version="1.0"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0"
       initial="s1" datamodel="ecmascript" name="boundary">
  <state id="s1">
    <transition event="go" target="s2"/>
  </state>
  <final id="s2"/>
</scxml>
"#;

fn sce_codegen_bin() -> &'static str {
    env!("CARGO_BIN_EXE_sce-codegen")
}

fn run_generate(out_dir: &Path, scxml_path: &Path) -> (i32, String) {
    let result = Command::new(sce_codegen_bin())
        .arg("generate")
        .arg(scxml_path.to_str().unwrap())
        .arg("-l")
        .arg("rust")
        .arg("-o")
        .arg(out_dir.to_str().unwrap())
        .output()
        .expect("spawn sce-codegen generate");
    let code = result.status.code().unwrap_or(-1);
    let stderr = String::from_utf8_lossy(&result.stderr).into_owned();
    (code, stderr)
}

#[test]
fn walker_passes_on_clean_generate() {
    let tmp = tempfile::TempDir::new().unwrap();
    let scxml = tmp.path().join("boundary.scxml");
    fs::write(&scxml, FIXTURE).unwrap();

    let out_dir = tmp.path().join("out");
    fs::create_dir_all(&out_dir).unwrap();

    let (code, stderr) = run_generate(&out_dir, &scxml);
    assert_eq!(
        code, 0,
        "generate must succeed on a clean fixture. stderr: {stderr}",
    );

    // The emitted *_sm.rs must contain both the drift header and an
    // SCE-MAP marker; otherwise the walker would have fired and the
    // command would have failed above.
    let sm = out_dir.join("boundary_sm.rs");
    assert!(sm.exists(), "generate must emit boundary_sm.rs");
    let content = fs::read_to_string(&sm).unwrap();
    assert!(
        content.contains("SCE-GENERATED"),
        "emitted file must carry the §6.2.6 drift header",
    );
    assert!(
        content.contains("SCE-MAP:"),
        "emitted file must carry at least one SCE-MAP marker line",
    );
}

#[test]
fn walker_fires_when_marker_stripped_from_emitted_file() {
    // Re-run generate so we have a known-good output set.
    let tmp = tempfile::TempDir::new().unwrap();
    let scxml = tmp.path().join("boundary.scxml");
    fs::write(&scxml, FIXTURE).unwrap();

    let out_dir = tmp.path().join("out");
    fs::create_dir_all(&out_dir).unwrap();

    let (code, stderr) = run_generate(&out_dir, &scxml);
    assert_eq!(
        code, 0,
        "fixture setup: clean generate must pass. stderr: {stderr}"
    );

    // Now simulate a template regression that dropped the marker
    // macro call: strip every `SCE-MAP:` line from the emitted file
    // and re-run the walker on the now-broken output via the
    // library API. (Re-running `generate` would re-emit the marker
    // — the walker is the production consumer at codegen finalize,
    // and the library API is what we test here.)
    let sm = out_dir.join("boundary_sm.rs");
    let original = fs::read_to_string(&sm).unwrap();
    let stripped: String = original
        .lines()
        .filter(|line| !line.contains("SCE-MAP:"))
        .collect::<Vec<_>>()
        .join("\n");
    fs::write(&sm, &stripped).unwrap();

    let err = sce_build::forge::sourcemap::validate_emitted_files_have_markers(&out_dir)
        .expect_err("walker must fire when an emitted file's SCE-MAP markers are stripped");
    // Surface check on the message; the diagnostic code wire is
    // verified separately in the diagnostic_goldens_are_byte_stable
    // suite.
    let display = format!("{err}");
    assert!(
        display.contains("SCE-MAP:"),
        "walker diagnostic must name the missing marker. got: {display}",
    );
    assert!(
        display.contains("Traceability Ownership Boundary")
            || display.contains("traceability/meta-generated-source-line-marker-missing"),
        "walker diagnostic must cite ARCHITECTURE.md §13 or the code. got: {display}",
    );
}

#[test]
fn walker_silently_skips_files_without_drift_header() {
    // Simulate an external meta-generator (protoc, bindgen) writing
    // a `.rs` file into the same out_dir without the §synth-6.2.6 drift
    // header. The walker must skip it silently — that file is
    // out-of-scope per the ownership boundary contract.
    let tmp = tempfile::TempDir::new().unwrap();
    let out_dir = tmp.path().join("out");
    fs::create_dir_all(&out_dir).unwrap();

    // 1. Hand-written file with no drift header — out of scope.
    fs::write(
        out_dir.join("external_meta_gen.rs"),
        "// auto-generated by protoc-rs; do not edit\npub struct External;\n",
    )
    .unwrap();

    // 2. Hand-written file with a `SCE-MAP:` marker but no drift
    //    header — still out of scope. (A marker without a drift
    //    header is not an SCE-emitted artefact; the drift header is
    //    the ownership signal, not the marker.)
    fs::write(
        out_dir.join("hand_authored_with_marker.rs"),
        "// not from sce-codegen\n// SCE-MAP: looks-like-one-but-isn't\npub fn x() {}\n",
    )
    .unwrap();

    // 3. Hand-written file with neither — trivially out of scope.
    fs::write(out_dir.join("regular.rs"), "pub fn y() -> u32 { 0 }\n").unwrap();

    let result = sce_build::forge::sourcemap::validate_emitted_files_have_markers(&out_dir);
    assert!(
        result.is_ok(),
        "walker must skip files without a §6.2.6 drift header (out-of-scope per ARCHITECTURE.md). got: {result:?}",
    );
}

#[test]
fn walker_passes_on_empty_directory() {
    let tmp = tempfile::TempDir::new().unwrap();
    let result = sce_build::forge::sourcemap::validate_emitted_files_have_markers(tmp.path());
    assert!(
        result.is_ok(),
        "walker must accept an empty directory (no emitted files = no invariant to check). got: {result:?}",
    );
}

#[test]
fn walker_diagnostic_code_is_meta_generated_source_line_marker_missing() {
    use sce_build::forge::diagnostic::ToDiagnostics;
    use sce_build::forge::error::{ForgeError, ValidationError};

    // Construct the diagnostic directly and route through
    // to_diagnostics — verifies the wire code name matches the
    // spec-anchored slash-path and the spec anchor is §synth-5-O.
    let err: ForgeError = ValidationError::TraceabilityMetaGeneratedSourceLineMarkerMissing {
        file: "out/test144/test144_sm.rs".into(),
    }
    .into();
    let d = err.to_diagnostics().pop().expect("one diagnostic");
    let code_str = serde_json::to_string(&d.code).unwrap();
    assert_eq!(
        code_str,
        "\"traceability/meta-generated-source-line-marker-missing\""
    );
    assert_eq!(d.spec, Some("SCE Protocol-Synthesis RFC §5.O"));
    assert_eq!(
        d.actual.as_deref(),
        Some("out/test144/test144_sm.rs"),
        "wire `actual` must carry the offending file path",
    );
    assert!(
        d.fix.is_none(),
        "no author repair — codegen-internal invariant"
    );
}

#[test]
fn walker_does_not_descend_into_files_with_non_source_extensions() {
    // Files with non-source extensions (`.json`, `.txt`, `.d`,
    // `.scxml`) are never §synth-6.2.6 drift-eligible, so the walker
    // skips them by extension before reading. Plant one such file
    // missing its marker; walker must still pass.
    let tmp = tempfile::TempDir::new().unwrap();
    let out_dir = tmp.path().join("out");
    fs::create_dir_all(&out_dir).unwrap();
    // sce_sourcemap.json is a sidecar emitted per §synth-5-O; it
    // never carries a `SCE-MAP:` marker (the markers live in the
    // accompanying *_sm.rs file), so the walker must NOT inspect it.
    fs::write(
        out_dir.join("sce_sourcemap.json"),
        r#"{"v":1,"source_hash":"abc","template_hash":"def","symbols":{}}"#,
    )
    .unwrap();
    let result = sce_build::forge::sourcemap::validate_emitted_files_have_markers(&out_dir);
    assert!(
        result.is_ok(),
        "walker must skip non-source files (e.g. sce_sourcemap.json). got: {result:?}",
    );
}

#[allow(dead_code)]
fn _tempfile_usage_check() {
    // Compile-only guard ensuring the `tempfile` dev-dep stays in
    // sce-build's Cargo.toml — removing it elsewhere would silently
    // break this test crate.
    let _: PathBuf = tempfile::TempDir::new().unwrap().path().to_path_buf();
}
