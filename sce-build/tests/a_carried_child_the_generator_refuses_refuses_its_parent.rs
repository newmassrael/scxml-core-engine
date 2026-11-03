// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
//! A child a document carries that the generator cannot generate refuses the
//! document, with the child's own record, on the row its author wrote.
//!
//! A child written in `<content>` is a machine of the run that generates its
//! parent, and the parent's machine names it. The preparation every route
//! shares left such a child out when `analyzer::can_generate_static` refused
//! it: `generate` and `orchestrate` wrote the parent, whose code names the
//! child's file, exited 0, and the output did not build. The gate the routes
//! ask of a document now asks it of every child the document carries
//! (`analyzer::can_generate_carried_children`), so each route refuses the
//! document as it refuses one whose own text the gate refuses: `generate`,
//! `orchestrate`, `check` on one document and on a set, the library's file
//! and in-memory entry points, and `generate-w3c`, which skips such a fixture
//! by the rule it skips one whose own document cannot be generated.
//!
//! The record is the child's own, with its code and its fix: placed in the
//! file the author wrote, on the row of what it names, at any depth; and, for
//! a refusal of the whole child, which names no row of it, on the child's
//! root element.

mod common;

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use common::routes::W3cSuite;
use sce_build::analyzer;
use sce_build::forge::codegen_matrix::emitting_in_part;
use sce_build::forge::diagnostic::{DiagnosticCode, ToDiagnostics};
use sce_build::forge::model::ForgeKind;
use sce_build::generator::Language;
use sce_build::model::Invoke;
use sce_build::parser::SCXMLParser;

/// A statechart that starts, in its one state, the child `child` written in
/// `<content>`. Its final state is `pass`, so it is a conformance fixture as
/// well.
fn host(child: &str) -> String {
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0" initial="running" datamodel="ecmascript">
  <state id="running">
    <invoke type="scxml" id="kid">
      <content>
{child}
      </content>
    </invoke>
    <transition event="done.invoke.kid" target="pass"/>
  </state>
  <final id="pass"/>
</scxml>
"#
    )
}

/// A child the generator generates: two states, one taken from the other.
const GENERATED_CHILD: &str = r#"        <scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0" initial="c" datamodel="ecmascript">
          <state id="c"><transition event="go" target="end"/></state>
          <final id="end"/>
        </scxml>"#;

/// The same child, whose transition names a state it does not declare
/// (W3C SCXML 3.5: `target` names states of the document).
const REFUSED_CHILD: &str = r#"        <scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0" initial="c" datamodel="ecmascript">
          <state id="c"><transition event="go" target="nowhere"/></state>
          <final id="end"/>
        </scxml>"#;

/// A child that starts, in turn, a child of its own written in `<content>`,
/// whose transition names a state it does not declare.
const REFUSED_GRANDCHILD: &str = r#"        <scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0" initial="c" datamodel="ecmascript">
          <state id="c">
            <invoke type="scxml" id="grand">
              <content>
                <scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0" initial="g" datamodel="ecmascript">
                  <state id="g"><transition event="go" target="nowhere"/></state>
                  <final id="end"/>
                </scxml>
              </content>
            </invoke>
          </state>
        </scxml>"#;

/// The 1-based row of the first line of `text` holding `needle`.
fn row_of(text: &str, needle: &str) -> u32 {
    let row = text
        .lines()
        .position(|line| line.contains(needle))
        .unwrap_or_else(|| panic!("the document holds `{needle}`"));
    u32::try_from(row + 1).expect("a row fits in u32")
}

/// The 1-based row of the child's root element in a document [`host`] wrote:
/// the second `<scxml` of the text.
fn child_root_row(text: &str) -> u32 {
    let row = text
        .lines()
        .enumerate()
        .filter(|(_, line)| line.contains("<scxml"))
        .nth(1)
        .map(|(index, _)| index)
        .expect("the document holds a child");
    u32::try_from(row + 1).expect("a row fits in u32")
}

/// The refusal the gate gives `model`, analysed under `label` as the routes
/// analyse it, as its file, its row, its code and its `actual`.
fn gate_refusal(model: &mut sce_build::model::SCXMLModel, label: &str) -> GateRefusal {
    analyzer::analyze(model, label);
    let refused = analyzer::can_generate_static(model, label)
        .expect_err("a document carrying a child the generator refuses is refused");
    let record = refused
        .to_diagnostics()
        .into_iter()
        .next()
        .expect("the refusal reports a record");
    GateRefusal {
        file: refused.location.file.clone(),
        line: refused.location.line,
        code: record.code,
        actual: record.actual,
    }
}

/// What [`gate_refusal`] reads off a refusal.
struct GateRefusal {
    file: String,
    line: Option<u32>,
    code: DiagnosticCode,
    actual: Option<String>,
}

/// `text` parsed under `label`, which every refusal below names as its file.
fn parsed(text: &str, label: &str) -> sce_build::model::SCXMLModel {
    SCXMLParser::new()
        .parse_string(text, label)
        .expect("the document parses: what refuses it is the gate")
}

/// The gate refuses a document whose child names a state it does not declare,
/// with the child's own record on the row of the transition, in the file the
/// author wrote, and passes the same document whose child declares it.
#[test]
fn the_gate_refuses_the_document_on_the_row_of_the_childs_refusal() {
    let mut generated = parsed(&host(GENERATED_CHILD), "host");
    analyzer::analyze(&mut generated, "host");
    analyzer::can_generate_static(&generated, "host")
        .expect("a document whose child the generator generates passes the gate");

    let text = host(REFUSED_CHILD);
    let refusal = gate_refusal(&mut parsed(&text, "host"), "host");
    assert!(
        matches!(refusal.code, DiagnosticCode::ValidationInvalidReference),
        "the child's own refusal, not a stand-in for it: {:?}",
        refusal.code
    );
    assert_eq!(refusal.actual.as_deref(), Some("nowhere"));
    assert_eq!(refusal.file, "host", "the file its author wrote");
    assert_eq!(
        refusal.line,
        Some(row_of(&text, "target=\"nowhere\"")),
        "the row of the transition, in the document"
    );
}

/// A child's child is asked as the child is, and its record is moved through
/// every document between to the row its author wrote.
#[test]
fn a_grandchilds_refusal_is_placed_in_the_document() {
    let text = host(REFUSED_GRANDCHILD);
    let refusal = gate_refusal(&mut parsed(&text, "host"), "host");
    assert!(
        matches!(refusal.code, DiagnosticCode::ValidationInvalidReference),
        "{:?}",
        refusal.code
    );
    assert_eq!(refusal.file, "host");
    assert_eq!(refusal.line, Some(row_of(&text, "target=\"nowhere\"")));
}

/// A child's child declaring its transition function, in a document handed
/// to the in-memory entry point, which has no source hash to bind it to: the
/// refusal raised against the grandchild is moved through the child that
/// carries it to the row its author wrote in the document, as the gate's is,
/// rather than left in the child's coordinates under a name nobody wrote.
const DECLARING_GRANDCHILD: &str = r#"        <scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0" initial="c" datamodel="ecmascript">
          <state id="c">
            <invoke type="scxml" id="grand">
              <content>
                <scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
                       version="1.0" initial="off" datamodel="ecmascript" sce:transition-function="emit">
                  <state id="off"><transition event="on" target="lit"/></state>
                  <state id="lit"><transition event="off" target="off"/></state>
                </scxml>
              </content>
            </invoke>
          </state>
        </scxml>"#;

#[test]
fn a_grandchilds_refusal_for_want_of_a_hash_is_placed_in_the_document() {
    let text = host(DECLARING_GRANDCHILD);
    let templates = sce_build::template_registry::embedded_templates_for(Language::Rust);
    let refused =
        sce_build::compile_from_string_lang_typed(&text, "host", &templates, Language::Rust)
            .err()
            .expect("the in-memory entry has no hash for the grandchild's function");
    let record = refused
        .to_diagnostics()
        .into_iter()
        .next()
        .expect("the refusal reports a record");
    assert!(
        matches!(record.code, DiagnosticCode::GenerateSourceHashUnavailable),
        "{:?}",
        record.code
    );
    assert_eq!(refused.location.file, "host", "the file its author wrote");
    assert_eq!(
        refused.location.line,
        Some(row_of(&text, "xmlns:sce=")),
        "the row the grandchild's root element opens on, in the document"
    );
}

/// A refusal of the whole child (here a top-level script that cannot be
/// loaded, W3C SCXML 5.8) names no row of it, and stands on the child's root
/// element in the document, not in a file named after the child that nobody
/// wrote.
#[test]
fn a_refusal_of_the_whole_child_stands_on_its_root_element() {
    let text = host(GENERATED_CHILD);
    let mut model = parsed(&text, "host");
    for state in model.states.values_mut() {
        for invoke in &mut state.invokes {
            if let Invoke::Scxml(info) = invoke {
                if let Some(child) = info.inline_child.as_mut() {
                    child.document_rejected = true;
                }
            }
        }
    }
    let refusal = gate_refusal(&mut model, "host");
    assert!(
        matches!(refusal.code, DiagnosticCode::ScxmlTopLevelScriptUnloaded),
        "{:?}",
        refusal.code
    );
    assert_eq!(refusal.file, "host");
    assert_eq!(refusal.line, Some(child_root_row(&text)));
}

/// The library's file and in-memory entry points refuse the document with the
/// record the gate gives, on every backend: the gate runs before any backend
/// is asked.
#[test]
fn the_library_entry_points_refuse_the_document() {
    let work = scratch("library");
    let text = host(REFUSED_CHILD);
    let doc = work.join("host.scxml");
    std::fs::write(&doc, &text).expect("write the document");
    let row = row_of(&text, "target=\"nowhere\"");
    let doc_str = doc.to_str().unwrap();
    for &language in Language::ALL {
        let refused = sce_build::compile_scxml_lang_typed(
            doc_str,
            &sce_build::find_template_dir_for(language),
            language,
        )
        .err()
        .unwrap_or_else(|| panic!("[{language:?}] the file entry wrote the parent alone"));
        assert_eq!(refused.location.file, doc_str, "[{language:?}]");
        assert_eq!(refused.location.line, Some(row), "[{language:?}]");

        let templates = sce_build::template_registry::embedded_templates_for(language);
        let refused =
            sce_build::compile_from_string_lang_typed(&text, "host", &templates, language)
                .err()
                .unwrap_or_else(|| {
                    panic!("[{language:?}] the in-memory entry returned the parent")
                });
        assert_eq!(refused.location.file, "host", "[{language:?}]");
        assert_eq!(refused.location.line, Some(row), "[{language:?}]");
        assert!(
            matches!(
                refused.to_diagnostics()[0].code,
                DiagnosticCode::ValidationInvalidReference
            ),
            "[{language:?}]"
        );
    }
}

fn sce_codegen_bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_sce-codegen"))
}

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("sce-build has a parent dir")
        .to_path_buf()
}

/// A scratch directory of its own for `name`, emptied first.
fn scratch(name: &str) -> PathBuf {
    let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR"))
        .join("a_carried_child_the_generator_refuses")
        .join(name);
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("create scratch");
    dir
}

fn codegen(args: &[&str]) -> Output {
    Command::new(sce_codegen_bin())
        .env("SCE_WORKSPACE_ROOT", repo_root())
        .args(args)
        .output()
        .expect("spawn sce-codegen")
}

/// Every backend that writes a statechart, in whole or in part, by the name
/// `-l` takes. Which of them writes the host is asked of the generator
/// ([`writes_the_host`]), not spelled here.
fn backends() -> Vec<&'static str> {
    emitting_in_part(ForgeKind::Statechart)
        .map(Language::canonical_name)
        .collect()
}

/// Whether `generate -l <lang>` writes the host whose child the generator
/// generates: a backend that refuses that document for a reason of its own
/// has nothing to say about the child, and is not asked.
fn writes_the_host(lang: &str, work: &Path) -> bool {
    let doc = work.join("generated.scxml");
    std::fs::write(&doc, host(GENERATED_CHILD)).expect("write the document");
    let out = work.join(format!("generated_{lang}"));
    codegen(&[
        "generate",
        doc.to_str().unwrap(),
        "-l",
        lang,
        "-o",
        out.to_str().unwrap(),
    ])
    .status
    .success()
}

/// Every file below `dir`, at any depth; none when it does not exist.
fn files_under(dir: &Path) -> Vec<PathBuf> {
    let mut files = Vec::new();
    let mut pending = vec![dir.to_path_buf()];
    while let Some(dir) = pending.pop() {
        let Ok(entries) = std::fs::read_dir(&dir) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                pending.push(path);
            } else {
                files.push(path);
            }
        }
    }
    files
}

/// The record of the child's refusal a `--error-format=json` run that must
/// fail wrote: one whose code is the child's.
fn the_childs_record(args: &[&str]) -> serde_json::Value {
    let run = codegen(args);
    let stderr = String::from_utf8_lossy(&run.stderr);
    assert!(
        !run.status.success(),
        "{args:?} passed a document whose child the generator refuses:\n{stderr}"
    );
    stderr
        .lines()
        .filter(|line| line.starts_with('{'))
        .filter_map(|line| serde_json::from_str::<serde_json::Value>(line).ok())
        .find(|record| record["code"] == "validation/invalid-reference")
        .unwrap_or_else(|| panic!("{args:?} did not report the child's refusal:\n{stderr}"))
}

/// `generate`, `orchestrate`, and `check` on the document and on a set of it,
/// refuse the document on every backend that writes it, with the child's
/// record on the row its author wrote, and write nothing.
#[test]
fn every_route_refuses_the_document_on_every_backend() {
    let work = scratch("routes");
    let text = host(REFUSED_CHILD);
    let doc = work.join("host.scxml");
    std::fs::write(&doc, &text).expect("write the document");
    let doc_str = doc.to_str().unwrap();
    let row = row_of(&text, "target=\"nowhere\"");
    let mut asked = 0;
    for lang in backends() {
        if !writes_the_host(lang, &work) {
            continue;
        }
        asked += 1;
        let out = work.join(format!("out_{lang}"));
        let out_str = out.to_str().unwrap();
        for args in [
            vec![
                "--error-format=json",
                "generate",
                doc_str,
                "-l",
                lang,
                "-o",
                out_str,
            ],
            vec![
                "--error-format=json",
                "orchestrate",
                "--scxml",
                doc_str,
                "-l",
                lang,
                "-o",
                out_str,
            ],
            vec!["--error-format=json", "check", doc_str, "-l", lang],
            vec![
                "--error-format=json",
                "check",
                "--scxml",
                doc_str,
                "-l",
                lang,
            ],
        ] {
            let record = the_childs_record(&args);
            assert_eq!(record["location"]["file"], doc_str, "{args:?}: {record}");
            assert_eq!(record["location"]["line"], row, "{args:?}: {record}");
            assert_eq!(record["actual"], "nowhere", "{args:?}: {record}");
        }
        assert_eq!(
            files_under(&out),
            Vec::<PathBuf>::new(),
            "[{lang}] a refused document wrote files"
        );
    }
    assert!(
        asked > 0,
        "no backend writes the host, so nothing was asked"
    );
}

/// The count `generate-w3c` prints of the fixtures it generated.
fn generated_total(stdout: &str) -> usize {
    stdout
        .lines()
        .find_map(|line| line.trim().strip_prefix("Generated (total):"))
        .and_then(|count| count.trim().parse().ok())
        .unwrap_or_else(|| panic!("generate-w3c printed no total:\n{stdout}"))
}

/// `generate-w3c` skips a fixture whose carried child the generator cannot
/// generate, as it skips a fixture whose own document it cannot generate:
/// named by the child's refusal, at the row its author wrote, with no file of
/// the fixture written and no stub standing in for the child. Asked of every
/// backend that generates the same fixture with a child it can generate.
#[test]
fn generate_w3c_skips_the_fixture_and_writes_nothing_of_it() {
    let mut asked = 0;
    for lang in backends() {
        let work = scratch(&format!("w3c_{lang}"));
        let generated = W3cSuite::stage(&work.join("generated"), "900", &host(GENERATED_CHILD));
        let control = codegen(&generated.args(lang, &work.join("generated_out")));
        if !control.status.success()
            || generated_total(&String::from_utf8_lossy(&control.stdout)) != 1
        {
            continue;
        }
        asked += 1;

        let text = host(REFUSED_CHILD);
        let refused = W3cSuite::stage(&work.join("refused"), "900", &text);
        let out = work.join("refused_out");
        let run = codegen(&refused.args(lang, &out));
        let stdout = String::from_utf8_lossy(&run.stdout);
        assert!(
            run.status.success(),
            "[{lang}] a skipped fixture failed the run:\n{stdout}\n{}",
            String::from_utf8_lossy(&run.stderr)
        );
        assert_eq!(generated_total(&stdout), 0, "[{lang}]\n{stdout}");
        let skip = stdout
            .lines()
            .find(|line| line.trim_start().starts_with("900: "))
            .unwrap_or_else(|| panic!("[{lang}] the fixture is not named as skipped:\n{stdout}"));
        let row = row_of(&text, "target=\"nowhere\"");
        assert!(
            skip.contains("not generated statically")
                && skip.contains(&format!("test900.scxml:{row}:")),
            "[{lang}] the skip does not name the child's refusal where its author wrote it: {skip}"
        );
        let written: Vec<PathBuf> = files_under(&out)
            .into_iter()
            .filter(|file| file.to_string_lossy().contains("test900"))
            .collect();
        assert_eq!(
            written,
            Vec::<PathBuf>::new(),
            "[{lang}] a skipped fixture left files"
        );
    }
    assert!(
        asked > 0,
        "no backend generates the fixture, so nothing was asked"
    );
}

/// A statechart that starts, in its one state, the child document
/// `test900sub1.scxml` beside it, by `src`.
const HOST_OF_A_FILE_CHILD: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0" initial="running" datamodel="ecmascript">
  <state id="running">
    <invoke type="scxml" id="kid" src="file:test900sub1.scxml"/>
    <transition event="done.invoke.kid" target="pass"/>
  </state>
  <final id="pass"/>
</scxml>
"#;

/// Fixture 900 of [`HOST_OF_A_FILE_CHILD`], staged under `work` with `child`
/// as the document of its child beside it.
fn suite_with_a_file_child(work: &Path, child: &str) -> (W3cSuite, String) {
    let suite = W3cSuite::stage(work, "900", HOST_OF_A_FILE_CHILD);
    let text = format!("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n{child}\n");
    let beside = suite
        .document
        .parent()
        .expect("the fixture has a directory")
        .join("test900sub1.scxml");
    std::fs::write(beside, &text).expect("write the child document");
    (suite, text)
}

/// A child the fixture starts by `src` is a document of the suite, judged by
/// the rule the fixture is: one the generator cannot generate skips the
/// fixture, named by the child's refusal in the child's own file, and nothing
/// of the fixture stays, the machine written before the child was judged
/// included, since it names the child. Kotlin used to stand a machine that
/// does nothing in for the child, so the fixture's test ran against a child
/// the document does not describe.
#[test]
fn generate_w3c_skips_a_fixture_whose_file_child_it_cannot_generate() {
    let mut asked = 0;
    for lang in backends() {
        let work = scratch(&format!("w3c_file_child_{lang}"));
        let (generated, _) = suite_with_a_file_child(&work.join("generated"), GENERATED_CHILD);
        let control = codegen(&generated.args(lang, &work.join("generated_out")));
        if !control.status.success()
            || generated_total(&String::from_utf8_lossy(&control.stdout)) != 1
        {
            continue;
        }
        asked += 1;

        let (refused, child) = suite_with_a_file_child(&work.join("refused"), REFUSED_CHILD);
        let out = work.join("refused_out");
        let run = codegen(&refused.args(lang, &out));
        let stdout = String::from_utf8_lossy(&run.stdout);
        assert!(
            run.status.success(),
            "[{lang}] a skipped fixture failed the run:\n{stdout}\n{}",
            String::from_utf8_lossy(&run.stderr)
        );
        assert_eq!(generated_total(&stdout), 0, "[{lang}]\n{stdout}");
        let skip = stdout
            .lines()
            .find(|line| line.trim_start().starts_with("900: "))
            .unwrap_or_else(|| panic!("[{lang}] the fixture is not named as skipped:\n{stdout}"));
        let row = row_of(&child, "target=\"nowhere\"");
        assert!(
            skip.contains("its child test900sub1 is not generated statically")
                && skip.contains(&format!("test900sub1.scxml:{row}:")),
            "[{lang}] the skip does not name the child's refusal: {skip}"
        );
        let written: Vec<PathBuf> = files_under(&out)
            .into_iter()
            .filter(|file| file.to_string_lossy().contains("test900"))
            .collect();
        assert_eq!(
            written,
            Vec::<PathBuf>::new(),
            "[{lang}] a skipped fixture left files"
        );
    }
    assert!(
        asked > 0,
        "no backend generates the fixture, so nothing was asked"
    );
}
