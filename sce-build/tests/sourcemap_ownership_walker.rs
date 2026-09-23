// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// SCE Protocol-Synthesis RFC §synth-5-O — ownership-boundary
// walker integration fixture.
//
// `forge::sourcemap::validate_emitted_files_have_markers` runs at the
// end of every successful `cmd_generate` / `cmd_generate_w3c` and
// enforces ARCHITECTURE.md "Traceability Ownership Boundary": every
// file SCE emitted (one carrying a §synth-6.2.6 drift header) must carry
// at least one `SCE-MAP:` marker that `forge::sourcemap::read_marker`
// reads back. External meta-generator output (no drift header) is
// silently out-of-scope.
//
// Five contracts pinned here (the fourth is
// `walker_fires_when_no_marker_reads_back`: a file whose markers carry the
// substring but no payload the comment encoder wrote is refused; the fifth is
// `a_marker_names_the_document_it_was_generated_from`: every marker and Go
// `//line` directive of all six backends names a document whose name the
// encoder changes, through the template writer and the Rust-spelled
// rejection stub alike):
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
    run_generate_in("rust", out_dir, scxml_path)
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

/// A marker the comment encoder did not write is not a marker.
///
/// The walker used to accept the substring `SCE-MAP:`, so a payload no
/// consumer can decode passed. Here every comment-form marker of a clean
/// generate is rewritten to name its document through a backslash. The
/// encoder's own spelling of one reads back as the name it encodes and
/// passes; an escape the encoder never writes leaves the file with no
/// marker that reads back and fails — while its `#[doc = "SCE-MAP: …"]`
/// lines, which still carry the substring, are what the old check would
/// have passed on.
#[test]
fn walker_fires_when_no_marker_reads_back() {
    use sce_build::forge::sourcemap::{read_marker, validate_emitted_files_have_markers};

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

    let sm = out_dir.join("boundary_sm.rs");
    let original = fs::read_to_string(&sm).unwrap();
    let naming = |name: &str| -> String {
        original
            .lines()
            .map(|line| match read_marker(line) {
                Some(_) => line.replacen("boundary.scxml", name, 1),
                None => line.to_string(),
            })
            .collect::<Vec<_>>()
            .join("\n")
    };

    fs::write(&sm, naming("back\\x5Cslash.scxml")).unwrap();
    validate_emitted_files_have_markers(&out_dir)
        .unwrap_or_else(|e| panic!("markers in the encoder's own spelling must read back: {e}"));
    let read_back: Vec<String> = fs::read_to_string(&sm)
        .unwrap()
        .lines()
        .filter_map(read_marker)
        .map(|m| m.expect("an encoder-spelled marker reads back").scxml_file)
        .collect();
    assert!(
        !read_back.is_empty() && read_back.iter().all(|f| f == "back\\slash.scxml"),
        "every marker must read back as the decoded name: {read_back:?}",
    );

    let undecodable = naming("back\\qslash.scxml");
    assert!(
        undecodable.contains("SCE-MAP:"),
        "the file must still carry the substring, or this does not separate reading from finding",
    );
    fs::write(&sm, undecodable).unwrap();
    let err = validate_emitted_files_have_markers(&out_dir)
        .expect_err("a file with no marker that reads back must be refused");
    assert!(
        format!("{err}").contains("boundary_sm.rs"),
        "the diagnostic must name the file: {err}",
    );
}

/// A marker names the document it was generated from, in every backend, when
/// the comment encoder changes that name.
///
/// `back\slash.scxml` is the name: a backslash is the character a file name
/// can carry that the encoder rewrites. Every comment-form marker must read
/// back through `read_marker` as that name, and every Go `//line` directive —
/// which the Go toolchain reads byte for byte and decodes nothing in — must
/// carry it as written rather than as `back\x5Cslash.scxml`. The rejected
/// document reaches the other writer: its stub is spelled in Rust by
/// `cmd_generate`, not rendered from a template.
///
/// The C-family `#line N "…"` and Rust `#[doc = "…"]` forms are string
/// literals, escaped at the literal door, and are not read here.
///
/// ⚠ Each backend must yield a marker, and Go's generated tree a directive, or
/// a sweep that found nothing to read would pass.
#[test]
fn a_marker_names_the_document_it_was_generated_from() {
    use sce_build::forge::sourcemap::read_marker;

    const NAME: &str = "back\\slash.scxml";
    // W3C SCXML 5.8: a `<script>` with neither `src` nor content rejects the
    // document, which sends `cmd_generate` down its stub-writing path.
    const REJECTED: &str = r#"<?xml version="1.0"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0"
       initial="s1" datamodel="ecmascript">
  <script/>
  <state id="s1"/>
</scxml>
"#;

    let tmp = tempfile::TempDir::new().unwrap();
    let mut failures = Vec::new();
    for (path_kind, document) in [("generated", FIXTURE), ("rejected", REJECTED)] {
        for lang in ["cpp", "c11", "rust", "kotlin", "go", "python"] {
            let dir = tmp.path().join(path_kind).join(lang);
            let out_dir = dir.join("out");
            fs::create_dir_all(&out_dir).unwrap();
            let scxml = dir.join(NAME);
            fs::write(&scxml, document).unwrap();
            let (code, stderr) = run_generate_in(lang, &out_dir, &scxml);
            assert_eq!(
                code, 0,
                "{path_kind} / {lang}: generate failed. stderr: {stderr}"
            );

            let (mut markers, mut directives) = (0usize, 0usize);
            for file in emitted_files(&out_dir) {
                let Ok(text) = fs::read_to_string(&file) else {
                    continue;
                };
                let origin = format!("{path_kind} / {lang} / {}", file.display());
                for line in text.lines() {
                    if let Some(marker) = read_marker(line) {
                        markers += 1;
                        match marker {
                            Ok(marker) if marker.scxml_file == NAME => {}
                            other => failures
                                .push(format!("  {origin}: `{line}` reads back as {other:?}")),
                        }
                    }
                    if let Some(directive) = line.trim_start().strip_prefix("//line ") {
                        directives += 1;
                        if directive.rsplit_once(':').map(|(file, _)| file) != Some(NAME) {
                            failures.push(format!("  {origin}: `{line}` does not name {NAME}"));
                        }
                    }
                }
            }
            if markers == 0 {
                failures.push(format!(
                    "  {path_kind} / {lang}: no `SCE-MAP:` marker to read"
                ));
            }
            if lang == "go" && path_kind == "generated" && directives == 0 {
                failures.push("  generated / go: no `//line` directive to read".to_string());
            }
        }
    }
    assert!(
        failures.is_empty(),
        "a marker does not name the document it was generated from:\n{}",
        failures.join("\n")
    );
}

fn run_generate_in(lang: &str, out_dir: &Path, scxml_path: &Path) -> (i32, String) {
    let result = Command::new(sce_codegen_bin())
        .arg("generate")
        .arg(scxml_path.to_str().unwrap())
        .arg("-l")
        .arg(lang)
        .arg("-o")
        .arg(out_dir.to_str().unwrap())
        .output()
        .expect("spawn sce-codegen generate");
    let code = result.status.code().unwrap_or(-1);
    let stderr = String::from_utf8_lossy(&result.stderr).into_owned();
    (code, stderr)
}

fn emitted_files(dir: &Path) -> Vec<PathBuf> {
    let mut pending = vec![dir.to_path_buf()];
    let mut files = Vec::new();
    while let Some(dir) = pending.pop() {
        for entry in fs::read_dir(&dir).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                pending.push(path);
            } else {
                files.push(path);
            }
        }
    }
    files
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
        r#"{"v":1,"source_hash":"abc","symbols":{}}"#,
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
