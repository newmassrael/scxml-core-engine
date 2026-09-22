//! Every output function of a transform takes the same parameters — the
//! inputs, then one per value read through `previous()` — so that an output
//! reading a sibling can forward its own list unchanged. An output that does
//! not read one of them is therefore normal, not a mistake, and generated
//! code must not be rejected for it by a compiler that warns on an unused
//! parameter: rustc always does, and C, C++ and Kotlin do under the warnings
//! a product commonly makes errors.
//!
//! ⚠ This was latent before `previous()`: every transform fixture's outputs
//! happened to read every input, so nothing generated one that did not, and
//! the first document that did — a holder, whose kept values are parameters
//! of every output and read by one — failed the Rust conformance build with
//! nine `unused variable` errors.
//!
//! Each backend is compiled with the strictest warning set its compiler
//! offers made fatal. The document is built so the question is real: two
//! outputs each read one input, and a third reads a sibling, which forwards
//! every parameter and so reads them all.

use std::path::{Path, PathBuf};
use std::process::Command;

use sce_build::toolchain;

fn sce_codegen_bin() -> String {
    env!("CARGO_BIN_EXE_sce-codegen").to_string()
}

struct Tmp(PathBuf);

impl Tmp {
    fn new(label: &str) -> Self {
        let d = std::env::temp_dir().join(format!("sce_unread_{label}_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).expect("create temp dir");
        Tmp(d)
    }
}

impl Drop for Tmp {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

const DOCUMENT: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       version="1.0" sce:kind="transform" name="split">
  <datamodel>
    <data id="left" sce:type="int32" sce:direction="in"/>
    <data id="right" sce:type="int32" sce:direction="in"/>
    <data id="doubled" sce:type="int32" sce:direction="out" expr="left * 2"/>
    <data id="negated" sce:type="int32" sce:direction="out" expr="0 - right"/>
    <data id="summed" sce:type="int32" sce:direction="out" expr="doubled + right"/>
  </datamodel>
</scxml>
"#;

/// Generate `DOCUMENT` for `lang` and return the one artifact ending in
/// `suffix`, with its text.
fn generated(t: &Tmp, lang: &str, suffix: &str) -> (PathBuf, String) {
    let doc = t.0.join("split.scxml");
    std::fs::write(&doc, DOCUMENT).expect("write document");
    let out_dir = t.0.join(lang);
    std::fs::create_dir_all(&out_dir).expect("create output dir");
    let run = Command::new(sce_codegen_bin())
        .args(["generate", doc.to_str().unwrap(), "-l", lang, "-o"])
        .arg(&out_dir)
        .output()
        .expect("spawn sce-codegen");
    assert!(
        run.status.success(),
        "{lang} refused:\n{}",
        String::from_utf8_lossy(&run.stderr)
    );
    let artifact = std::fs::read_dir(&out_dir)
        .expect("read output dir")
        .flatten()
        .map(|e| e.path())
        .find(|p| p.to_string_lossy().ends_with(suffix))
        .unwrap_or_else(|| panic!("{lang}: no artifact ending in {suffix}"));
    let text = std::fs::read_to_string(&artifact).expect("read artifact");
    (artifact, text)
}

/// ⚠ The floor that keeps this file from passing vacuously: an emitter
/// that stopped generating the unread case at all would compile cleanly.
/// `marker` is how `lang` says "unread", and two outputs have one each.
fn assert_marks_two(lang: &str, text: &str, marker: &str) {
    assert_eq!(
        text.matches(marker).count(),
        2,
        "{lang}: expected `{marker}` on exactly the two unread parameters — `doubled` \
         does not read `right`, `negated` does not read `left`, and `summed` reads a \
         sibling so reads both:\n{text}"
    );
}

fn compile(program: &Path, args: &[&str], cwd: &Path) {
    let out = Command::new(program)
        .args(args)
        .current_dir(cwd)
        .output()
        .unwrap_or_else(|e| panic!("spawn {}: {e}", program.display()));
    assert!(
        out.status.success(),
        "{} {args:?} rejected the generated code:\n{}{}",
        program.display(),
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
}

#[test]
fn rust_compiles_under_deny_warnings() {
    let t = Tmp::new("rust");
    let (file, text) = generated(&t, "rust", ".rs");
    assert_marks_two("rust", &text, "#[allow(unused_variables)]");
    let Some(rustc) = toolchain::require_or_skip("rustc", "compile a generated transform") else {
        return;
    };
    compile(
        &rustc,
        &[
            "--crate-type=lib",
            "--edition=2021",
            "-D",
            "warnings",
            "--out-dir",
            ".",
            file.file_name().unwrap().to_str().unwrap(),
        ],
        file.parent().unwrap(),
    );
}

#[test]
fn cpp_compiles_under_wall_wextra_werror() {
    let t = Tmp::new("cpp");
    let (header, text) = generated(&t, "cpp", ".h");
    assert_marks_two("cpp", &text, "[[maybe_unused]]");
    let Some(cxx) = toolchain::require_or_skip("g++", "compile a generated transform") else {
        return;
    };
    let dir = header.parent().unwrap();
    std::fs::write(
        dir.join("probe.cpp"),
        format!(
            "#include \"{}\"\n",
            header.file_name().unwrap().to_str().unwrap()
        ),
    )
    .expect("write probe.cpp");
    compile(
        &cxx,
        &[
            "-std=c++17",
            "-Wall",
            "-Wextra",
            "-Werror",
            "-fsyntax-only",
            "probe.cpp",
        ],
        dir,
    );
}

#[test]
fn c_compiles_under_wall_wextra_werror() {
    let t = Tmp::new("c");
    let (header, text) = generated(&t, "c", ".h");
    assert_marks_two("c", &text, "(void)");
    let Some(cc) = toolchain::require_any_or_skip(&["gcc", "cc"], "compile a generated transform")
    else {
        return;
    };
    let dir = header.parent().unwrap();
    std::fs::write(
        dir.join("probe.c"),
        format!(
            "#include \"{}\"\n",
            header.file_name().unwrap().to_str().unwrap()
        ),
    )
    .expect("write probe.c");
    compile(
        &cc,
        &[
            "-std=c11",
            "-Wall",
            "-Wextra",
            "-Werror",
            "-fsyntax-only",
            "probe.c",
        ],
        dir,
    );
}

#[test]
fn kotlin_compiles_under_werror() {
    let t = Tmp::new("kotlin");
    let (file, text) = generated(&t, "kotlin", ".kt");
    assert_marks_two("kotlin", &text, "@Suppress(\"UNUSED_PARAMETER\")");
    let Some(kotlinc) = toolchain::require_or_skip("kotlinc", "compile a generated transform")
    else {
        return;
    };
    compile(
        &kotlinc,
        &[
            "-Werror",
            file.file_name().unwrap().to_str().unwrap(),
            "-d",
            "classes",
        ],
        file.parent().unwrap(),
    );
}
