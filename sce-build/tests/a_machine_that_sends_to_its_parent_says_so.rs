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
