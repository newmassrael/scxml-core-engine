// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
//! A machine that sends to `#_parent` says so on its manifest.
//!
//! # What was wrong
//!
//! Measured 2026-09-28: a machine written through the authoring MCP sent
//! its notices to `#_parent`. It was checked, it passed, it built — and
//! started on its own, every one of those sends raises
//! `error.communication` (§scxml-6.2.4). Nothing between the author and the
//! first send said the machine could only run as a child, because whether
//! a parent exists is a fact about the deployment and no single-document
//! check can judge it.
//!
//! So the document's half is PUBLISHED: `needs_parent` and `parent_sends`,
//! the same kind of answer `needs_host_processor` gives. These cases hold
//! the published half to the document, on the binary a host runs, and to
//! the wire schema a consumer reads it with.

use std::path::{Path, PathBuf};
use std::process::Command;

fn sce_codegen_bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_sce-codegen"))
}

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("sce-build has a parent")
        .to_path_buf()
}

fn scratch(label: &str) -> PathBuf {
    let dir =
        PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join(format!("{label}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("create scratch dir");
    dir
}

/// The manifest `check -l rust` writes for `body`, validated against the
/// checked-in schema before it is returned.
fn manifest(label: &str, body: &str) -> serde_json::Value {
    let dir = scratch(label);
    let path = dir.join(format!("{label}.scxml"));
    std::fs::write(&path, body).expect("write fixture");
    let out = Command::new(sce_codegen_bin())
        .args(["--error-format", "json", "check"])
        .arg(&path)
        .args(["-l", "rust"])
        .output()
        .expect("run sce-codegen check");
    let _ = std::fs::remove_dir_all(&dir);
    assert!(
        out.status.success(),
        "{label} is valid SCXML and must check: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let manifest: serde_json::Value =
        serde_json::from_slice(&out.stdout).expect("the manifest is one JSON line");

    let schema: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string(repo_root().join("schemas/sce-manifest.v1.schema.json"))
            .expect("read manifest schema"),
    )
    .expect("manifest schema is JSON");
    let validator = jsonschema::JSONSchema::options()
        .with_draft(jsonschema::Draft::Draft7)
        .compile(&schema)
        .expect("manifest schema compiles");
    let violations: Vec<String> = match validator.validate(&manifest) {
        Ok(()) => Vec::new(),
        Err(errors) => errors.map(|e| e.to_string()).collect(),
    };
    assert!(violations.is_empty(), "{label}: {violations:?}\n{manifest}");
    manifest
}

/// Each literal `#_parent` site is listed where it is written, a nested
/// one included, and one naming its event by expression is listed without
/// an event rather than left out.
#[test]
fn every_send_to_the_parent_is_published() {
    // `r##` because the fixture itself contains `"#`.
    let m = manifest(
        "notifier",
        r##"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       version="1.0" name="notifier" initial="idle" datamodel="ecmascript">
  <state id="idle">
    <onentry>
      <send target="#_parent" event="indicator.update"/>
    </onentry>
    <transition event="lock" target="locked">
      <if cond="true">
        <send target="#_parent" eventexpr="'computed.' + 'name'"/>
      </if>
    </transition>
  </state>
  <state id="locked">
    <onentry><send target="#_internal" event="not.to.the.parent"/></onentry>
  </state>
</scxml>
"##,
    );
    assert_eq!(m["needs_parent"], true, "{m}");
    let sends = m["parent_sends"]
        .as_array()
        .expect("parent_sends is listed");
    let seen: Vec<(Option<&str>, &str, u64)> = sends
        .iter()
        .map(|s| {
            (
                s["event"].as_str(),
                s["state"].as_str().expect("state"),
                s["location"]["line"].as_u64().expect("line"),
            )
        })
        .collect();
    assert_eq!(
        seen,
        [(Some("indicator.update"), "idle", 6), (None, "idle", 10)],
        "{m}"
    );
}

/// A send to the parent that carries a question the specification leaves open
/// names it. The draft wrote `#_parent` because a send needs a target, and the
/// owner has not said who the caller is: a consumer that plays the design reads
/// the id off the site, and says the example is blocked by that decision and not
/// that the engine had no parent. Only a QUESTION is one: a value chosen without
/// an answer (`sce:assumed`) is applied and is not a route nobody decided.
#[test]
fn a_send_to_the_parent_that_carries_an_open_question_names_it() {
    let m = manifest(
        "asks",
        r##"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       version="1.0" name="asks" initial="idle">
  <state id="idle">
    <onentry>
      <send event="asked" target="#_parent" sce:unresolved="caller-target"
            sce:unresolved-reason="the specification does not say who the caller is"/>
      <send event="assumed" target="#_parent" sce:assumed="parent-by-default"
            sce:assumed-reason="the standing rule says a notice goes to the parent"/>
      <send event="plain" target="#_parent"/>
    </onentry>
  </state>
</scxml>
"##,
    );
    let sends = m["parent_sends"]
        .as_array()
        .expect("parent_sends is listed");
    let named: Vec<(&str, Vec<&str>)> = sends
        .iter()
        .map(|s| {
            (
                s["event"].as_str().expect("event"),
                s["decisions"]
                    .as_array()
                    .map(|d| d.iter().map(|id| id.as_str().expect("an id")).collect())
                    .unwrap_or_default(),
            )
        })
        .collect();
    assert_eq!(
        named,
        [
            ("asked", vec!["caller-target"]),
            ("assumed", vec![]),
            ("plain", vec![])
        ],
        "{m}"
    );
    assert!(
        sends[1].get("decisions").is_none() && sends[2].get("decisions").is_none(),
        "omitted, not []: {m}"
    );
}

/// `false` is an answer, not an absent field — a host reads the need off
/// the manifest, so silence would read as "unknown".
#[test]
fn a_machine_that_never_sends_to_its_parent_says_it_needs_none() {
    let m = manifest(
        "standalone",
        r##"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       version="1.0" name="standalone" initial="idle">
  <state id="idle">
    <onentry><send target="#_internal" event="tick"/></onentry>
    <transition event="tick" target="done"/>
  </state>
  <final id="done"/>
</scxml>
"##,
    );
    assert_eq!(m["needs_parent"], false, "{m}");
    assert!(m.get("parent_sends").is_none(), "omitted, not []: {m}");
}

const NOTIFIER: &str = r##"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       version="1.0" initial="idle">
  <state id="idle">
    <onentry>
      <send target="#_parent" event="indicator.update"/>
    </onentry>
  </state>
</scxml>
"##;

/// `check` over a document set: exit status and every record's code and row.
fn check_set(label: &str, docs: &[(&str, &str)]) -> (i32, Vec<(String, u64)>) {
    let dir = scratch(label);
    let mut cmd = Command::new(sce_codegen_bin());
    cmd.args(["--error-format", "json", "check", "-l", "rust"]);
    for (name, body) in docs {
        let path = dir.join(name);
        std::fs::write(&path, body).expect("write fixture");
        cmd.arg("--scxml").arg(&path);
    }
    let out = cmd.output().expect("run sce-codegen check");
    let _ = std::fs::remove_dir_all(&dir);
    let records = String::from_utf8_lossy(&out.stderr)
        .lines()
        .filter(|l| l.starts_with('{'))
        .map(|l| {
            let v: serde_json::Value = serde_json::from_str(l).expect("NDJSON record");
            (
                v["code"].as_str().expect("code").to_string(),
                v["location"]["line"].as_u64().unwrap_or(0),
            )
        })
        .collect();
    (out.status.code().unwrap_or(-1), records)
}

/// The set is where the need is judged: alone in a set, nothing can be
/// the notifier's parent, and the refusal lands on the send.
#[test]
fn a_set_member_nobody_invokes_is_refused_at_its_send() {
    let (status, records) = check_set("orphan-set", &[("notifier.scxml", NOTIFIER)]);
    assert_ne!(status, 0, "{records:?}");
    assert_eq!(
        records,
        [("scxml/parent-send-without-parent".to_string(), 6)],
        "one record, on the <send>"
    );
}

/// The control: the same notifier, invoked by a member of the set, has
/// its parent — so the refusal above is about the missing invoker and
/// nothing else about the document.
#[test]
fn a_set_member_another_member_invokes_has_its_parent() {
    let parent = r##"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       version="1.0" initial="running">
  <state id="running">
    <invoke type="http://www.w3.org/TR/scxml/" src="notifier.scxml"/>
    <transition event="indicator.update" target="done"/>
  </state>
  <final id="done"/>
</scxml>
"##;
    let (status, records) = check_set(
        "invoked-set",
        &[("host.scxml", parent), ("notifier.scxml", NOTIFIER)],
    );
    assert_eq!(
        status, 0,
        "the invoked notifier has its parent: {records:?}"
    );
    assert!(records.is_empty(), "{records:?}");
}
