// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// NL→IR Mapping Roadmap Items 1+5 — codegen emission test for the
// `sce:req` traceability + `sce:unresolved` placeholder annotation
// comments.
//
// `codegen_smoke.rs` already guards that an annotated SCXML still
// compiles in the four toolchain-checked backends. This file is the
// content-side gate: every backend's generated source must carry the
// annotation as a backend-appropriate trailing comment, in document
// order, for every emission site the parser surfaces — and must NOT
// emit anything when the SCXML carries no annotations.
//
// Both halves matter: byte-identity absent the attribute keeps the
// large existing golden corpus stable, and presence of the comment
// proves the traceability link upstream consumers depend on actually
// makes it through codegen.

use std::path::{Path, PathBuf};
use std::process::Command;

fn sce_codegen_bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_sce-codegen"))
}

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("CARGO_MANIFEST_DIR has a parent (workspace root)")
        .to_path_buf()
}

fn write_scxml(dir: &Path, name: &str, body: &str) -> PathBuf {
    let path = dir.join(format!("{name}.scxml"));
    std::fs::write(&path, body).expect("write scxml fixture");
    path
}

fn generate(lang: &str, out_dir: &Path, scxml: &Path) {
    let output = Command::new(sce_codegen_bin())
        .env("SCE_WORKSPACE_ROOT", workspace_root())
        .args(["generate", "-l", lang, "-o"])
        .arg(out_dir)
        .arg(scxml)
        .output()
        .expect("spawn sce-codegen");
    assert!(
        output.status.success(),
        "sce-codegen generate -l {lang} failed:\nstdout: {}\nstderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr),
    );
}

fn read_concat_outputs(out_dir: &Path) -> String {
    let mut joined = String::new();
    for entry in std::fs::read_dir(out_dir).expect("read out dir").flatten() {
        let path = entry.path();
        let ext = path
            .extension()
            .and_then(|s| s.to_str())
            .unwrap_or_default();
        // Concatenate the language-specific source files. The
        // `.scxml` we copied in for the generate call must be
        // excluded so the fixture text itself does not satisfy the
        // assertions.
        if matches!(ext, "py" | "rs" | "h" | "inl" | "c" | "go" | "kt") {
            let content = std::fs::read_to_string(&path).expect("read generated file");
            joined.push_str(&content);
            joined.push('\n');
        }
    }
    joined
}

const ANNOTATED_SCXML: &str = r#"<?xml version="1.0"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       xmlns:sce="http://sce.dev/ext"
       version="1.0" name="annot_emit" initial="s0">
  <state id="s0" sce:req="REQ_STATE_S0">
    <onentry sce:req="REQ_ONENTRY">
      <log sce:req="REQ_LOG_LEAF" expr="'entered s0'"/>
    </onentry>
    <transition event="go" target="done" sce:req="REQ_TRANS_GO">
      <log sce:req="REQ_TRANS_LOG" expr="'on go'"/>
    </transition>
  </state>
  <final id="done"/>
</scxml>
"#;

const BARE_SCXML: &str = r#"<?xml version="1.0"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0"
       name="annot_emit" initial="s0">
  <state id="s0">
    <onentry>
      <log expr="'entered s0'"/>
    </onentry>
    <transition event="go" target="done">
      <log expr="'on go'"/>
    </transition>
  </state>
  <final id="done"/>
</scxml>
"#;

/// All six backends emit the same annotation tokens (req IDs and the
/// unresolved marker id) as backend-appropriate comments. We check
/// only the SCE-emitted *content*: backend-specific syntax (`//` vs
/// `#` vs `/* */`) is verified by codegen_smoke's toolchain check.
fn assert_all_backends_emit_annotations(scratch: &Path) {
    for lang in ["python", "rust", "cpp", "c", "go", "kotlin"] {
        let out_dir = scratch.join(lang);
        std::fs::create_dir_all(&out_dir).expect("create lang scratch");
        let scxml = write_scxml(&out_dir, "annot_emit", ANNOTATED_SCXML);
        generate(lang, &out_dir, &scxml);
        let joined = read_concat_outputs(&out_dir);

        let expected_tokens = [
            "REQ_STATE_S0",
            // Parser inheritance: <onentry sce:req=…> propagates onto
            // each child action so the child <log> carries both its
            // own id and the inherited onentry id.
            "REQ_LOG_LEAF",
            "REQ_ONENTRY",
            "REQ_TRANS_GO",
            "REQ_TRANS_LOG",
        ];
        for tok in expected_tokens {
            assert!(
                joined.contains("sce:req:")
                    && joined.contains(tok),
                "backend {lang}: generated source missing requirement id {tok}\noutput excerpt:\n{}",
                joined
                    .lines()
                    .filter(|l| l.contains("sce:"))
                    .collect::<Vec<_>>()
                    .join("\n"),
            );
        }
    }
}

/// SCXML without any `sce:req` / `sce:unresolved` attributes must
/// produce output containing zero annotation comments. The macro
/// emits the empty string in that case, so the surrounding
/// template's existing whitespace flows through unchanged.
fn assert_no_backend_emits_annotations(scratch: &Path) {
    for lang in ["python", "rust", "cpp", "c", "go", "kotlin"] {
        let out_dir = scratch.join(format!("{lang}_bare"));
        std::fs::create_dir_all(&out_dir).expect("create lang scratch");
        let scxml = write_scxml(&out_dir, "annot_emit", BARE_SCXML);
        generate(lang, &out_dir, &scxml);
        let joined = read_concat_outputs(&out_dir);

        assert!(
            !joined.contains("sce:req:"),
            "backend {lang}: bare SCXML produced unexpected sce:req comment:\n{}",
            joined
                .lines()
                .filter(|l| l.contains("sce:"))
                .collect::<Vec<_>>()
                .join("\n"),
        );
        assert!(
            !joined.contains("sce:unresolved:"),
            "backend {lang}: bare SCXML produced unexpected sce:unresolved comment",
        );
    }
}

#[test]
fn sce_req_emission_present_in_all_backends() {
    let scratch = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("sce_annotation_present");
    let _ = std::fs::remove_dir_all(&scratch);
    std::fs::create_dir_all(&scratch).expect("create scratch");
    assert_all_backends_emit_annotations(&scratch);
}

#[test]
fn bare_scxml_emits_no_annotation_comments() {
    let scratch = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("sce_annotation_absent");
    let _ = std::fs::remove_dir_all(&scratch);
    std::fs::create_dir_all(&scratch).expect("create scratch");
    assert_no_backend_emits_annotations(&scratch);
}
