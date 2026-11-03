// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
//! A machine whose author wrote it in-line, in an `<invoke>`'s `<content>`,
//! is named by the document that holds its text and the invokes that lead to
//! it, in its generated files and in the sourcemap alike.
//!
//! # What was wrong
//!
//! A child's `// From:` named `<dir>/<parent>__sce_synth_invoke__<id>.scxml`,
//! a file a run writes into its output for a later step to read, and before
//! that `unknown.scxml`. Its sourcemap symbols named that same file and
//! counted rows of the text the parser builds in memory from the `<content>`,
//! so a reader was sent to a copy, or to nothing, at rows the author never
//! wrote; the repository kept committed copies of the W3C children so a gate
//! had something to open.
//!
//! # What is held
//!
//! Read off the fixture, not restated: a child, and a child of that child,
//! each in a document whose `<xi:include>` moves every row after it, so a
//! position that skips any document it was cut from lands on the wrong row.
//!
//! * each machine's `// From:` is the document's provenance, and for a child
//!   `#` and its invoke path (`sc.scxml#outer`, `sc.scxml#outer/_invoke_0`);
//! * every symbol names a document its author wrote (`sc.scxml`), never a
//!   synthesised child's name, at the row its start tag stands on there;
//! * a child's symbols carry the invoke path its `// From:` ends in, and the
//!   document's own carry none;
//! * the markers in a child's machine name the document and its rows.

use std::collections::BTreeSet;
use std::path::Path;
use std::process::Command;

const FRAGMENT: &str = r#"<wrap xmlns="http://www.w3.org/2005/07/scxml">
  <state id="a"/>
  <state id="b"/>
  <state id="c"/>
</wrap>
"#;

const HOST: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       xmlns:xi="http://www.w3.org/2001/XInclude" version="1.0" initial="s" datamodel="null">
  <xi:include href="states.xml"/>
  <state id="s">
    <invoke type="scxml" id="outer">
      <content>
        <scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0" initial="child_wait" datamodel="null">
          <state id="child_wait">
            <invoke type="scxml">
              <content>
                <scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0" initial="grand_done" datamodel="null">
                  <final id="grand_done"/>
                </scxml>
              </content>
            </invoke>
            <transition event="done.invoke" target="child_done"/>
          </state>
          <final id="child_done"/>
        </scxml>
      </content>
    </invoke>
    <transition event="done.invoke.outer" target="pass"/>
  </state>
  <final id="pass"/>
</scxml>
"#;

/// The 1-based row `needle` starts on in `text`.
fn row_of(text: &str, needle: &str) -> u64 {
    let offset = text.find(needle).expect("the fixture spells the needle");
    text[..offset].matches('\n').count() as u64 + 1
}

/// The `// From:` line of a generated Rust file, as written after the colon.
fn from_line(code: &str) -> Option<&str> {
    code.lines()
        .find_map(|line| line.strip_prefix("// From: "))
        .map(str::trim_end)
}

/// Generate the fixture for Rust with its provenance rooted at its directory,
/// and return that directory (kept alive) and the output directory.
fn generated() -> (tempfile::TempDir, std::path::PathBuf) {
    let dir = tempfile::tempdir().expect("tempdir");
    std::fs::write(dir.path().join("states.xml"), FRAGMENT).expect("write fragment");
    let doc = dir.path().join("sc.scxml");
    std::fs::write(&doc, HOST).expect("write host");
    let out = dir.path().join("out");
    let run = Command::new(env!("CARGO_BIN_EXE_sce-codegen"))
        .arg("generate")
        .arg(&doc)
        .args(["-l", "rust", "-o"])
        .arg(&out)
        .arg("--source-root")
        .arg(dir.path())
        .output()
        .expect("spawn sce-codegen");
    assert_eq!(
        run.status.code(),
        Some(0),
        "generate failed:\n{}",
        String::from_utf8_lossy(&run.stderr)
    );
    (dir, out)
}

/// Every generated machine file of `out`, by file name, with its text.
fn machines(out: &Path) -> Vec<(String, String)> {
    let mut found: Vec<(String, String)> = std::fs::read_dir(out)
        .expect("the output directory")
        .map(|entry| entry.expect("an entry").path())
        .filter(|path| {
            path.file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| name.ends_with("_sm.rs"))
        })
        .map(|path| {
            let name = path
                .file_name()
                .and_then(|name| name.to_str())
                .expect("a file name")
                .to_string();
            let code = std::fs::read_to_string(&path).expect("a generated file");
            (name, code)
        })
        .collect();
    found.sort();
    found
}

#[test]
fn each_machine_names_the_document_and_the_invokes_that_lead_to_it() {
    let (_dir, out) = generated();
    let named: BTreeSet<String> = machines(&out)
        .iter()
        .map(|(name, code)| {
            from_line(code)
                .unwrap_or_else(|| panic!("{name} has no `// From:` line"))
                .to_string()
        })
        .collect();
    let want: BTreeSet<String> = ["sc.scxml", "sc.scxml#outer", "sc.scxml#outer/_invoke_0"]
        .into_iter()
        .map(str::to_string)
        .collect();
    assert_eq!(
        named, want,
        "a machine's provenance is its document's, and a child's adds its invoke path"
    );
}

#[test]
fn every_symbol_names_the_document_and_the_row_its_author_wrote() {
    let (_dir, out) = generated();
    let sourcemap: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string(out.join("sce_sourcemap.json")).expect("the sourcemap"),
    )
    .expect("the sourcemap is JSON");
    let symbols = sourcemap["symbols"].as_object().expect("a symbol table");

    let named_by_copy: Vec<String> = symbols
        .iter()
        .filter(|(_, s)| s["scxml_file"] != "sc.scxml" && s["scxml_file"] != "states.xml")
        .map(|(name, s)| format!("{name}: {s}"))
        .collect();
    assert!(
        named_by_copy.is_empty(),
        "a symbol names a file nobody wrote:\n{}",
        named_by_copy.join("\n")
    );

    // (kind, state path, invoke path, needle whose row the symbol is on)
    let expected: [(&str, &str, Option<&str>, &str); 6] = [
        ("state", "s", None, "<state id=\"s\""),
        ("state", "child_wait", Some("outer"), "<state id=\"child_wait\""),
        ("state", "child_done", Some("outer"), "<final id=\"child_done\""),
        (
            "machine",
            "",
            Some("outer"),
            "<scxml xmlns=\"http://www.w3.org/2005/07/scxml\" version=\"1.0\" initial=\"child_wait\"",
        ),
        (
            "state",
            "grand_done",
            Some("outer/_invoke_0"),
            "<final id=\"grand_done\"",
        ),
        (
            "machine",
            "",
            Some("outer/_invoke_0"),
            "<scxml xmlns=\"http://www.w3.org/2005/07/scxml\" version=\"1.0\" initial=\"grand_done\"",
        ),
    ];
    let mut wrong = Vec::new();
    for (kind, state, invoke_path, needle) in expected {
        let row = row_of(HOST, needle);
        let symbol = symbols.values().find(|s| {
            s["kind"] == kind
                && s["scxml_state_path"] == state
                && s.get("scxml_invoke_path").and_then(|p| p.as_str()) == invoke_path
        });
        match symbol {
            None => wrong.push(format!(
                "no {kind} symbol for `{state}` under invoke path {invoke_path:?}"
            )),
            Some(symbol) => {
                if symbol["scxml_file"] != "sc.scxml"
                    || symbol["line_range"] != serde_json::json!([row, row])
                {
                    wrong.push(format!(
                        "{kind} `{state}` ({invoke_path:?}): want sc.scxml:{row}, got {symbol}"
                    ));
                }
            }
        }
    }
    assert!(wrong.is_empty(), "{}\n{sourcemap}", wrong.join("\n"));
}

#[test]
fn a_childs_markers_name_the_document_that_holds_it() {
    let (_dir, out) = generated();
    let row = row_of(
        HOST,
        "<scxml xmlns=\"http://www.w3.org/2005/07/scxml\" version=\"1.0\" initial=\"child_wait\"",
    );
    let child = machines(&out)
        .into_iter()
        .find(|(_, code)| from_line(code) == Some("sc.scxml#outer"))
        .map(|(_, code)| code)
        .expect("the child's machine");
    assert!(
        child.contains(&format!("SCE-MAP: sc.scxml:{row} :: _machine")),
        "the child's machine marker does not name sc.scxml:{row}"
    );
    assert!(
        !child.contains("__sce_synth_invoke__outer.scxml"),
        "a marker of the child names a synthesised file"
    );
}
