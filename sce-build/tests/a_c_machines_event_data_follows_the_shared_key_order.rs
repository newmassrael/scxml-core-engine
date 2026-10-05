// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// The members of a JSON object written into `_event.data` come in one order on
// every engine — ascending by the key's UTF-8 bytes (ARCHITECTURE.md, "JSON
// Object Key Order") — and every writer is held to the table
// `tests/json_text/object_key_order.json`.
//
// A `datamodel="sce-static"` machine generated as C writes the pairs of a
// `<send>`, a `<donedata>` and an `<invoke>` through the header-only wire writer
// of the forge runtime, one `sce_forge_wire_pair` per `<param>` in the order the
// generated code lists them. The generator sorts the names once, so a pair a
// failed value leaves out leaves the order of the others as it was; this test
// holds that sort to the table. A document that declares `b` before `a` must
// still list `a` first, and `B` before `a`, `10` before `9`, and a supplementary
// character after the basic plane's.
//
// Only the cases a C machine can state are read: a flat object of whole
// numbers. A name that repeats is one array on every engine, which the wire
// writer collects as it is given the pairs of a name one after another: the
// generated code lists them together, in the order they were declared in, and
// `backends/c/tests/unit/forge_wire_repeat_test.c` holds the writer to the table's
// case. An object or array value is not a value a `sce-static` variable holds.

use std::path::{Path, PathBuf};
use std::process::Command;

use tempfile::tempdir;

fn sce_codegen_bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_sce-codegen"))
}

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("sce-build has a parent")
        .to_path_buf()
}

/// A machine whose entry sends an event carrying `names` as `<param>`s, in that
/// order, each holding the number `n`.
fn sending(names: &[String]) -> String {
    let params: String = names
        .iter()
        .map(|name| format!("        <param name=\"{name}\" expr=\"n\"/>\n"))
        .collect();
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       version="1.0" initial="s" datamodel="sce-static" name="ordered">
  <datamodel>
    <data id="n" sce:type="uint32" expr="1"/>
  </datamodel>
  <state id="s">
    <onentry>
      <send event="go">
{params}      </send>
    </onentry>
    <transition event="go" target="done"/>
  </state>
  <final id="done"/>
</scxml>
"#
    )
}

/// `sce-codegen generate <doc> -l c`, and the generated source, or the refusal.
fn generate_c(doc: &str) -> Result<String, String> {
    let dir = tempdir().expect("tempdir");
    let path = dir.path().join("ordered.scxml");
    std::fs::write(&path, doc).expect("write the probe");
    let out = Command::new(sce_codegen_bin())
        .arg("--workspace-root")
        .arg(repo_root())
        .arg("--error-format=json")
        .args(["generate", "-l", "c", "-o"])
        .arg(dir.path())
        .arg(&path)
        .output()
        .expect("invoke sce-codegen");
    if !out.status.success() {
        return Err(String::from_utf8_lossy(&out.stderr).into_owned()
            + &String::from_utf8_lossy(&out.stdout));
    }
    Ok(std::fs::read_to_string(dir.path().join("ordered_sm.c")).expect("the generated source"))
}

/// The keys of the pairs the machine writes into the event's data, in the order
/// its code lists them.
fn written_keys(source: &str) -> Vec<String> {
    const CALL: &str = "sce_forge_wire_pair(&sce_wire_, \"\\\"";
    source
        .lines()
        .filter_map(|line| {
            let at = line.find(CALL)? + CALL.len();
            let rest = &line[at..];
            Some(rest[..rest.find("\\\"\"")?].to_string())
        })
        .collect()
}

/// The keys of an object the table writes out, in the order it writes them.
fn keys_of(data: &str) -> Vec<String> {
    let object: serde_json::Value = serde_json::from_str(data).expect("the table's own JSON");
    // The table writes the members in the order under test, and `serde_json`
    // returns them either in that text's order or sorted by key — the same order,
    // since the table's text is sorted. The assertion is the match against what the
    // generated code lists, so the table is read back, never recomputed here.
    object
        .as_object()
        .expect("a flat object")
        .keys()
        .cloned()
        .collect()
}

#[test]
fn the_pairs_of_a_send_are_listed_in_the_order_the_shared_table_gives() {
    let table: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string(repo_root().join("tests/json_text/object_key_order.json"))
            .expect("the shared key order table"),
    )
    .expect("the table is JSON");
    let mut held = 0;
    for case in table["cases"].as_array().expect("cases") {
        let params = case["params"].as_array().expect("params");
        let flat = params.iter().all(|pair| pair[1].is_u64());
        let names: Vec<String> = params
            .iter()
            .map(|pair| pair[0].as_str().expect("a name").to_string())
            .collect();
        if !flat {
            continue;
        }
        let name = case["name"].as_str().expect("a case name");
        let source = generate_c(&sending(&names))
            .unwrap_or_else(|why| panic!("{name}: C generation was refused:\n{why}"));
        // The table's keys are in the order under test; a name that repeats is
        // one key there, and its pairs are listed together here, once for each
        // time the document declares it.
        let expected: Vec<String> = keys_of(case["data"].as_str().expect("data"))
            .into_iter()
            .flat_map(|key| {
                let times = names.iter().filter(|name| **name == key).count();
                std::iter::repeat_n(key, times)
            })
            .collect();
        assert_eq!(
            written_keys(&source),
            expected,
            "{name}: the pairs are not listed in the order the table gives"
        );
        held += 1;
    }
    assert!(
        held >= 6,
        "the table's flat cases are the ones this holds C to, and only {held} were read"
    );
}

#[test]
fn the_pairs_of_a_name_that_repeats_are_listed_together_in_document_order() {
    // `k`, `j`, `k` is `j` and then the two `k`s: sorted by name, with a name's
    // pairs in the order they were declared in (the values here are one number, so
    // the order is the one the keys give).
    let doc = sending(&["k".to_string(), "j".to_string(), "k".to_string()]);
    let source = generate_c(&doc).expect("a repeated name is lowered");
    assert_eq!(written_keys(&source), ["j", "k", "k"]);
}
