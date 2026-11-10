// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
//! One SCXML document carrying author-controlled text into every field a
//! backend echoes, generated for every backend, twice.
//!
//! # Why a DIFFERENTIAL rather than a search
//!
//! Asking whether a hostile value leaked into code means knowing what the code
//! would have been without it. So each check generates the same document
//! twice: once with the hazard characters, once with inert characters of the
//! same shape in the same places. The two renderings must have the same
//! STRUCTURE — the text with comments and string literals blanked — and any
//! difference is the value having escaped the place it was written into.
//!
//! A search for markers in code cannot replace it. It reports nothing when the
//! value swallowed the code that followed it instead of injecting any, which is
//! what a line break in a `//` comment and an unterminated string literal both
//! do.
//!
//! # Why it is here rather than in one test
//!
//! Two suites need it — `a_value_written_into_a_comment_is_encoded` and
//! `a_value_written_into_a_string_literal_is_escaped` — because the generator
//! has two encoding doors and each one's proof is the same experiment with
//! different hazards. A second copy of the document, the six backends and the
//! structural comparison would be a second answer to what "the value stayed
//! where it was written" means.

#![allow(dead_code)]

use std::path::{Path, PathBuf};
use std::process::Command;

use sce_build::forge::codegen_matrix::emitting;
use sce_build::forge::model::ForgeKind;

use super::source_lexing::{structure_mask, Lang};

/// Every backend `sce-codegen` generates for, by its `--lang` value.
///
/// All of them, because an encoding rule is per-language and a sweep over
/// five proves nothing about the sixth. "Generates for" is read off the codegen
/// matrix, so a backend that is named in `Language` and emits nothing yet is
/// not in a sweep that needs its output, and joins it in the commit that gives
/// it an emitter.
pub fn backends() -> Vec<&'static str> {
    emitting(ForgeKind::Statechart)
        .map(|language| language.canonical_name())
        .collect()
}

/// Where a hostile document carries its markers, and nowhere else.
pub const MARKER: &str = "INJ_";

/// The workspace root, from the crate this test belongs to.
pub fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("sce-build has a parent")
        .to_path_buf()
}

/// A document echoing one value per field a backend writes out.
///
/// `expression` supplies the value of each ECMAScript-expression field, named
/// by the field it stands in, and `label` the one free-text field — `<log
/// label>`, which no grammar constrains and which reaches a string literal in
/// every backend.
pub fn document(label: &str, expression: impl Fn(&str) -> String) -> String {
    let e = expression;
    format!(
        r#"<?xml version="1.0"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0" name="probe" initial="s0" datamodel="ecmascript">
  <datamodel>
    <data id="d" expr="{data}"/>
    <data id="arr" expr="[1, 2]"/>
  </datamodel>
  <state id="s0">
    <onentry>
      <log label="{label}" expr="{log}"/>
      <assign location="d" expr="{assign}"/>
      <if cond="d == {if_}">
        <log expr="1"/>
      <elseif cond="d == {elseif}"/>
        <log expr="2"/>
      <else/>
        <log expr="3"/>
      </if>
      <foreach array="arr" item="it" index="ix">
        <log expr="{foreach}"/>
      </foreach>
      <script>var sv = {script};</script>
      <send event="tick"><param name="p" expr="{param}"/></send>
      <raise event="go"/>
    </onentry>
    <transition event="go" cond="d != {cond}" target="done"/>
  </state>
  <final id="done">
    <donedata><content expr="{done}"/></donedata>
  </final>
</scxml>
"#,
        data = e("DATA"),
        log = e("LOG"),
        assign = e("ASSIGN"),
        if_ = e("IF"),
        elseif = e("ELSEIF"),
        foreach = e("FOREACH"),
        script = e("SCRIPT"),
        param = e("PARAM"),
        cond = e("COND"),
        done = e("DONE"),
    )
}

/// Generate `doc` for `lang` into `dir`, returning the output directory.
///
/// Panics with the generator's own words when it refuses, so a refusal is
/// read as the failure it is rather than as an empty directory that compares
/// equal to another empty directory.
///
/// ⚠ `codegen` is a PARAMETER rather than `env!("CARGO_BIN_EXE_sce-codegen")`
/// read here, and both halves of the reason matter. `env!` expands while the
/// module is COMPILED, and this file compiles into every target that declares
/// `mod common;` — so reading it here would make the whole suite fail to build
/// whenever the `cli` feature is off, whether or not a target ever generates
/// anything. And `cli_feature_gating` derives which targets reach the binary
/// by looking for that expansion in the TARGET's own source, so a target that
/// reached it only through this file would go unseen. Keeping the macro where
/// the target writes it answers both.
pub fn generate(codegen: &str, lang: &str, dir: &Path, doc: &str) -> PathBuf {
    std::fs::create_dir_all(dir).expect("create scratch directory");
    // One basename for both renderings: generated file names follow the
    // document's, and a control under another name would compare against
    // files that do not exist.
    let scxml = dir.join("probe.scxml");
    std::fs::write(&scxml, doc).expect("write probe document");
    let out = dir.join("gen");
    std::fs::create_dir_all(&out).expect("create output directory");
    let output = Command::new(codegen)
        .env("SCE_WORKSPACE_ROOT", repo_root())
        .args(["generate", "-l", lang, "-o"])
        .arg(&out)
        .arg(&scxml)
        .output()
        .expect("spawn sce-codegen");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        output.status.success() && !stdout.contains("\"rejected\""),
        "sce-codegen generate -l {lang} did not generate the probe:\nstdout: {stdout}\nstderr: {}",
        String::from_utf8_lossy(&output.stderr),
    );
    out
}

/// What a generated file DOES: its text with comments and literals blanked,
/// every numeric literal reduced to `0`, and whitespace removed. Two renderings
/// that differ only inside comments and literals, or in the value of a number,
/// have the same structure.
///
/// ⚠ A number is compared as a number and not by its digits, because the build
/// sizes storage by the bytes of the values a document writes (docs/adr/0086,
/// section 8: a C11 machine's slot is as large as the data its sends carry). A
/// hostile text and its control differ in length once encoded (`"` is two bytes
/// of JSON, `$x` is two characters where the control has one), so a size the
/// build derived from them differs too, and rightly. Measured on main 2d7f7d916a:
/// `PROBE_DATA_LEN_TO_PARENT` was `16u` for the hostile rendering and `15u` for
/// the control, and nothing else in the code differed. A value that left its
/// literal still shows: it arrives as punctuation and identifiers, which stay
/// compared, and no hazard text holds a digit.
pub fn code_structure(source: &str, lang: Lang) -> String {
    let masked = structure_mask(source, lang);
    let mut out = String::with_capacity(masked.len());
    let mut chars = masked.chars().peekable();
    let mut previous: Option<char> = None;
    while let Some(c) = chars.next() {
        let starts_number =
            c.is_ascii_digit() && !previous.is_some_and(|p| p.is_alphanumeric() || p == '_');
        if starts_number {
            // The whole literal: digits, a radix prefix, a fraction, an
            // exponent's letter and a type suffix (`16u`, `0x1Fu`, `1.5e3f`).
            let mut last = c;
            while let Some(&n) = chars.peek() {
                if !(n.is_ascii_alphanumeric() || n == '_' || n == '.') {
                    break;
                }
                last = n;
                chars.next();
            }
            out.push('0');
            previous = Some(last);
            continue;
        }
        if !c.is_whitespace() {
            out.push(c);
        }
        previous = Some(c);
    }
    out
}
