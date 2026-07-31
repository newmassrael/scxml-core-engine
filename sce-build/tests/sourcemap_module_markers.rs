// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// SCE Protocol-Synthesis RFC §synth-5-O — per-backend module-level SCE-MAP marker
// presence fixture.
//
// Each backend's state machine template (Rust / Cpp / C11 / Kotlin /
// Go) imports `_macros/sce_map_marker.jinja2` and emits a single
// module-level marker anchored at the `<scxml>` root element. The
// fixture below confirms the marker reaches the rendered output so a
// future template edit that drops the macro import or the
// `{{ sce_map.source_marker(...) }}` call surfaces immediately
// instead of silently regressing the spec contract
// [[feedback-silently-broken-hooks]].
//
// Per-state / per-transition / per-action / forge-per-kind markers
// are covered by `s5_o_atomic_0c_markers.rs`; this fixture covers
// only the module-level foundation.

use std::path::{Path, PathBuf};
use std::process::Command;

/// Minimal SCXML document exercising the marker emit path. Two states,
/// one transition — enough to exercise State + Transition + SCXMLModel
/// IR creation in the parser and have all three carry
/// `source_location: Some(_)` by the time codegen renders.
const FIXTURE: &str = r#"<?xml version="1.0"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0"
       initial="s1" datamodel="ecmascript">
  <state id="s1">
    <transition event="go" target="s2"/>
  </state>
  <final id="s2"/>
</scxml>
"#;

fn sce_codegen_bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_sce-codegen"))
}

/// Stage the fixture into a unique temp dir and run sce-codegen on it
/// for the given backend. Returns the generated file paths.
fn generate(lang: &str) -> Vec<PathBuf> {
    let tmp = std::env::temp_dir().join(format!(
        "sce_marker_test_{}_{}_pid{}",
        lang,
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(&tmp).expect("create temp dir");
    let scxml = tmp.join("marker_probe.scxml");
    std::fs::write(&scxml, FIXTURE).expect("write fixture");

    let out = Command::new(sce_codegen_bin())
        .arg("generate")
        .arg(&scxml)
        .arg("-l")
        .arg(lang)
        .arg("-o")
        .arg(&tmp)
        .output()
        .expect("sce-codegen invocation");
    assert!(
        out.status.success(),
        "sce-codegen generate -l {lang} failed: stdout={} stderr={}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );

    // Collect the *_sm.* artifacts (excluding the input .scxml).
    let mut artifacts = Vec::new();
    for entry in std::fs::read_dir(&tmp).expect("read temp dir") {
        let entry = entry.expect("dir entry");
        let path = entry.path();
        if path.extension().map(|e| e == "scxml").unwrap_or(false) {
            continue;
        }
        artifacts.push(path);
    }
    assert!(
        !artifacts.is_empty(),
        "no artifacts generated for -l {lang} in {}",
        tmp.display()
    );
    artifacts
}

/// Read `path` and assert a SCE-MAP marker referencing the fixture
/// SCXML's basename appears at least once. Returns the marker line so
/// the per-backend test can additionally assert backend-specific
/// syntax (`#![doc]` for Rust, `//line` deferred to 0c, etc.).
fn assert_marker_present(path: &Path) -> String {
    let body =
        std::fs::read_to_string(path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()));
    let line = body
        .lines()
        .find(|l| l.contains("SCE-MAP") && l.contains("marker_probe.scxml"))
        .unwrap_or_else(|| {
            panic!(
                "{}: no SCE-MAP marker referencing marker_probe.scxml found.\n\
                 ─── file contents ───\n{}",
                path.display(),
                body
            )
        });
    line.trim().to_string()
}

#[test]
fn rust_emits_both_doc_and_comment_marker() {
    let artifacts = generate("rust");
    let rs = artifacts
        .iter()
        .find(|p| p.extension().map(|e| e == "rs").unwrap_or(false))
        .expect("rust artifact");
    let body = std::fs::read_to_string(rs).expect("read rust output");
    // Spec lines 3135-3136 — Rust MUST emit BOTH forms.
    assert!(
        body.contains("#![doc = \"SCE-MAP: marker_probe.scxml"),
        "rust output missing `#![doc = \"SCE-MAP: ...\"]` form"
    );
    assert!(
        body.contains("// SCE-MAP: marker_probe.scxml"),
        "rust output missing `// SCE-MAP: ...` comment form"
    );
}

#[test]
fn cpp_emits_module_level_marker_on_header() {
    let artifacts = generate("cpp");
    let h = artifacts
        .iter()
        .find(|p| p.extension().map(|e| e == "h").unwrap_or(false))
        .expect("cpp header artifact");
    let marker = assert_marker_present(h);
    // Module-level form is a comment; `#line` is reserved for 0c
    // per-function emission.
    assert!(
        marker.starts_with("// SCE-MAP:"),
        "cpp header marker should be `// SCE-MAP:` comment form, got: {marker}"
    );
}

#[test]
fn c11_emits_module_level_marker_on_header_and_impl() {
    let artifacts = generate("c11");
    let h = artifacts
        .iter()
        .find(|p| p.extension().map(|e| e == "h").unwrap_or(false))
        .expect("c11 header artifact");
    let c = artifacts
        .iter()
        .find(|p| p.extension().map(|e| e == "c").unwrap_or(false))
        .expect("c11 implementation artifact");
    let h_marker = assert_marker_present(h);
    let c_marker = assert_marker_present(c);
    assert!(h_marker.starts_with("// SCE-MAP:"));
    assert!(c_marker.starts_with("// SCE-MAP:"));
}

#[test]
fn kotlin_emits_marker() {
    let artifacts = generate("kotlin");
    let kt = artifacts
        .iter()
        .find(|p| p.extension().map(|e| e == "kt").unwrap_or(false))
        .expect("kotlin artifact");
    let marker = assert_marker_present(kt);
    assert!(marker.starts_with("// SCE-MAP:"));
}

#[test]
fn go_emits_module_level_marker() {
    let artifacts = generate("go");
    let go = artifacts
        .iter()
        .find(|p| p.extension().map(|e| e == "go").unwrap_or(false))
        .expect("go artifact");
    let marker = assert_marker_present(go);
    // Module-level form is a comment; `//line` is reserved for 0c
    // per-function emission (it remaps subsequent line positions for
    // the rest of the file, disruptive at module scope).
    assert!(
        marker.starts_with("// SCE-MAP:"),
        "go marker should be comment form at module level, got: {marker}"
    );
}
