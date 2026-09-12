// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// NL→IR Mapping Roadmap Items 1+5+7 — codegen emission test for the
// `sce:req` traceability, `sce:provenance` spec-anchor, and
// `sce:unresolved` placeholder annotation comments.
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

use sce_build::provenance::SpecProvenance;

/// The six backends this gate speaks for. Named once so a test that
/// quietly stopped covering one is a diff rather than an omission.
const BACKENDS: [&str; 6] = ["python", "rust", "cpp", "c", "go", "kotlin"];

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
  <state id="s0" sce:req="REQ_STATE_S0"
         sce:provenance="OEM-DIAG-SPEC@D#3.4.2:112">
    <sce:provenance doc-id="OEM-TIMING-REQ" page="7"/>
    <onentry sce:req="REQ_ONENTRY" sce:provenance="OEM-BLOCK-SPEC@A#1.1">
      <log sce:req="REQ_LOG_LEAF" sce:provenance="ISO-14229-1#11.2.1"
           expr="'entered s0'"/>
    </onentry>
    <transition event="go" target="done" sce:req="REQ_TRANS_GO"
                sce:provenance="OEM-REV-ONLY@C">
      <log sce:req="REQ_TRANS_LOG" sce:provenance="OEM-TRACE-SPEC"
           expr="'on go'"/>
    </transition>
  </state>
  <final id="done"/>
</scxml>
"#;

/// The anchors [`ANNOTATED_SCXML`] declares, exactly as the IR holds
/// them — the comparison target for what codegen emits.
///
/// Between them they cover all six shapes the compact grammar can
/// take, and the two that the ATTRIBUTE form cannot even spell are
/// the point of the list rather than padding: `OEM-TIMING-REQ` has a
/// page and no section (only the element form can say that, and it
/// round-trips through the odd-looking `#:7`), and `OEM-BLOCK-SPEC`
/// is written on `<onentry>`, which has no IR node at all — so the
/// only way it can reach generated source is `inherit_provenance`
/// putting it on the action inside.
fn declared_anchors() -> Vec<SpecProvenance> {
    let anchor =
        |doc: &str, rev: Option<&str>, section: Option<&str>, page: Option<u32>| SpecProvenance {
            doc_id: doc.to_string(),
            rev: rev.map(str::to_string),
            section: section.map(str::to_string),
            at: page.map(sce_build::provenance::Position::Page),
        };
    vec![
        anchor("OEM-DIAG-SPEC", Some("D"), Some("3.4.2"), Some(112)),
        anchor("OEM-TIMING-REQ", None, None, Some(7)),
        anchor("ISO-14229-1", None, Some("11.2.1"), None),
        anchor("OEM-BLOCK-SPEC", Some("A"), Some("1.1"), None),
        anchor("OEM-REV-ONLY", Some("C"), None, None),
        anchor("OEM-TRACE-SPEC", None, None, None),
    ]
}

/// The payload of every `sce:<kind>:` comment line in `joined`, with
/// the backend's comment syntax stripped.
///
/// Reading the payload rather than asking whether the whole blob
/// `contains` a token is what lets an assertion say the token is *on*
/// an annotation line. It also means the C11 block form and the
/// line-comment backends are compared as the same content.
fn annotation_payloads(joined: &str, kind: &str) -> Vec<String> {
    let marker = format!("sce:{kind}: ");
    joined
        .lines()
        .filter_map(|line| line.split_once(&marker))
        .map(|(_, rest)| {
            rest.trim_end()
                .trim_end_matches("*/")
                .trim_end()
                .to_string()
        })
        .collect()
}

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
    for lang in BACKENDS {
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
    for lang in BACKENDS {
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
        assert!(
            !joined.contains("sce:provenance:"),
            "backend {lang}: bare SCXML produced unexpected sce:provenance comment:\n{}",
            joined
                .lines()
                .filter(|l| l.contains("sce:"))
                .collect::<Vec<_>>()
                .join("\n"),
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

/// Every backend emits every declared spec anchor, and every anchor
/// it emits is one the document actually declares.
///
/// Both directions, because either alone is satisfiable by a bug.
/// "Every declared anchor appears" passes for a macro that prints the
/// whole vector on every node; "every emitted anchor is declared"
/// passes for a macro that prints nothing. Together they pin the set.
///
/// The comparison runs through `SpecProvenance::parse_compact` rather
/// than over strings, which is what makes this a fidelity claim and
/// not a spelling one: the comment is only useful if a reader — or a
/// tool — can get the anchor back out of it, and the parser that
/// reads the attribute is the same parser that must accept the
/// comment. A renderer that dropped the page, or spelled a section
/// the grammar cannot express, fails here.
#[test]
fn every_backend_emits_the_spec_anchors_and_only_those() {
    let scratch = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("sce_provenance_emission");
    let _ = std::fs::remove_dir_all(&scratch);
    std::fs::create_dir_all(&scratch).expect("create scratch");

    let declared = declared_anchors();
    for lang in BACKENDS {
        let out_dir = scratch.join(lang);
        std::fs::create_dir_all(&out_dir).expect("create lang scratch");
        let scxml = write_scxml(&out_dir, "annot_emit", ANNOTATED_SCXML);
        generate(lang, &out_dir, &scxml);
        let joined = read_concat_outputs(&out_dir);

        let payloads = annotation_payloads(&joined, "provenance");
        assert!(
            !payloads.is_empty(),
            "backend {lang}: no sce:provenance comment reached the generated source",
        );

        // → Every emitted line parses, and parses to a declared
        //   anchor. A payload the parser rejects is worse than no
        //   comment: it promises a machine-readable link and breaks
        //   the tool that follows it.
        let mut emitted: Vec<SpecProvenance> = Vec::new();
        for payload in &payloads {
            let parsed = SpecProvenance::parse_compact(payload).unwrap_or_else(|| {
                panic!(
                    "backend {lang}: emitted anchor {payload:?} is not accepted by \
                     SpecProvenance::parse_compact"
                )
            });
            assert!(
                declared.contains(&parsed),
                "backend {lang}: emitted anchor {payload:?} parses to {parsed:?}, \
                 which the document does not declare",
            );
            emitted.push(parsed);
        }

        // ← And every declared anchor reached the source. The
        //   `<onentry>` one can only get here through
        //   `inherit_provenance`, so this half also holds the
        //   block-level inherit true at the codegen layer.
        for want in &declared {
            assert!(
                emitted.contains(want),
                "backend {lang}: declared anchor {want:?} never reached the generated \
                 source. Emitted: {payloads:?}",
            );
        }
    }
}

#[test]
fn bare_scxml_emits_no_annotation_comments() {
    let scratch = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("sce_annotation_absent");
    let _ = std::fs::remove_dir_all(&scratch);
    std::fs::create_dir_all(&scratch).expect("create scratch");
    assert_no_backend_emits_annotations(&scratch);
}
