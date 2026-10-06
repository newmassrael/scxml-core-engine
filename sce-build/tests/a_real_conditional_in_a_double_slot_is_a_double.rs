// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// `datamodel="sce-static"`: a conditional that lands in a `float64` slot is made as a double
// (docs/SCE_ACCEPTED_SUBSET.md, "A 32-bit real": an operation is made at the precision of the
// place its value lands).
//
// `v * 0.1` over an `int32` joins to a single of its own, and an arithmetic node took the slot's
// precision from the context the emitter pushes into it. A conditional took its own, so
// `c ? v * 0.1 : 0.0` into a `float64` slot was made as a single on every language that has one
// (`0.1f` in C and C++, `v as f32` in Rust, `toFloat()` in Kotlin, `float32(..)` in Go,
// `to_f32(..)` in Python), and its value reached the slot as -12.300000190734863 where the
// double is -12.3. Measured 2026-10-06: four shipped cases of one component. The plain form
// `a = v * 0.1` was right in every language, which is the control here.

use std::path::{Path, PathBuf};
use std::process::Command;

use tempfile::tempdir;

fn sce_codegen_bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_sce-codegen"))
}

const PROBE: &str = r##"<?xml version="1.0"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       version="1.0" name="probe" initial="s" datamodel="sce-static">
  <datamodel>
    <data id="v" sce:type="int32" expr="0"/>
    <data id="c" sce:type="bool" expr="true"/>
    <data id="plain" sce:type="float64" expr="0.0"/>
    <data id="negated" sce:type="float64" expr="0.0"/>
    <data id="chosen" sce:type="float64" expr="0.0"/>
    <data id="chosenNegated" sce:type="float64" expr="0.0"/>
  </datamodel>
  <state id="s">
    <transition event="go" type="internal">
      <assign location="plain" expr="v * 0.1"/>
      <assign location="negated" expr="0.0 - v * 0.1"/>
      <assign location="chosen" expr="c ? v * 0.1 : 0.0"/>
      <assign location="chosenNegated" expr="c ? 0.0 - v * 0.1 : 0.0"/>
    </transition>
  </state>
</scxml>
"##;

/// The marks of a single in the languages that have one of their own.
const SINGLE: [&str; 7] = [
    "0.1f", "0.0f", "f32", "float32", "to_f32", "toFloat", "(float)",
];

/// Generate the probe for `language` and return the lines that spell the literal `0.1`.
fn lines_with_the_literal(language: &str) -> Vec<String> {
    let dir = tempdir().expect("tempdir");
    let doc = dir.path().join("probe.scxml");
    std::fs::write(&doc, PROBE).expect("write probe");
    let out_dir: &Path = dir.path();
    let out = Command::new(sce_codegen_bin())
        .arg("generate")
        .arg(&doc)
        .arg("-o")
        .arg(out_dir)
        .arg("-l")
        .arg(language)
        .output()
        .expect("invoke sce-codegen");
    assert!(
        out.status.success(),
        "sce-codegen generate -l {language} refused the probe:\n{}{}",
        String::from_utf8_lossy(&out.stderr),
        String::from_utf8_lossy(&out.stdout)
    );
    let mut lines = Vec::new();
    for entry in std::fs::read_dir(out_dir).expect("read output dir") {
        let path = entry.expect("dir entry").path();
        if path == doc || !path.is_file() {
            continue;
        }
        let Ok(text) = std::fs::read_to_string(&path) else {
            continue;
        };
        lines.extend(
            text.lines()
                .filter(|l| l.contains("0.1") && !l.trim_start().starts_with("//"))
                .map(|l| l.trim().to_string()),
        );
    }
    lines
}

#[test]
fn a_conditional_that_lands_in_a_double_is_made_as_a_double_in_every_language() {
    for language in ["cpp", "c11", "rust", "go", "kotlin", "python"] {
        let lines = lines_with_the_literal(language);
        assert!(
            lines.len() >= 4,
            "{language}: the four assignments were not found among {lines:?}"
        );
        for line in &lines {
            assert!(
                !SINGLE.iter().any(|mark| line.contains(mark)),
                "{language}: a value landing in a float64 slot was made as a single: {line}"
            );
        }
    }
}
