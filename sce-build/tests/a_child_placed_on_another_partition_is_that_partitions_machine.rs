// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
//! A child written in-line that a deploy places on another partition
//! (SCE_MESH.md §9.6.6 rule 3) is that partition's machine: the run that
//! generates its parent writes the child's document and not its machine, and
//! the run that generates the child's partition writes the machine.
//!
//! # What was wrong
//!
//! The parent's run wrote every child it carries beside it, this one too,
//! shaped for a parent in its own process, while the parent reached the child
//! through the transport and named no file of it. The run of the child's
//! partition then wrote the same file again, shaped as a peer, so the file
//! had two producers and held whichever wrote last; the CMake rule that
//! declared it was the second of them.
//!
//! # What is held
//!
//! The document is the shape of `tests/mesh/parent_synth_inline.scxml` and
//! its deploy, written here so the test reads no tracked file:
//!
//! * without a deploy, the parent's run writes the child's machine (the
//!   control: a child in the parent's process is the parent's run's);
//! * with the deploy, the parent's run writes the child's document and its
//!   own machine, and no machine of the child;
//! * the run of the child's document with the deploy writes the child's
//!   machine.

use std::path::Path;
use std::process::Command;

const PARENT: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       version="1.0" datamodel="ecmascript" initial="waiting" name="parent_synth_inline">
    <state id="waiting">
        <invoke type="scxml" id="remote_inv">
            <content>
                <scxml xmlns="http://www.w3.org/2005/07/scxml"
                       version="1.0" initial="done">
                    <final id="done"/>
                </scxml>
            </content>
        </invoke>
        <transition event="done.invoke.remote_inv" target="pass"/>
        <transition event="error.execution" target="fail"/>
    </state>
    <final id="pass"/>
    <final id="fail"/>
</scxml>
"#;

/// The parent on one partition, its in-line child on another.
const DEPLOY: &str = r#"version: "1.0"
topology:
  ecu1:
    machines:
      parent_synth_inline:
        source: parent_synth_inline.scxml
      parent_synth_inline__sce_synth_invoke__remote_inv:
        source: parent_synth_inline__sce_synth_invoke__remote_inv.scxml

partitions:
  p_parent:
    machines: [parent_synth_inline]
    contains:
      invokes:
        - { machine: parent_synth_inline, invoke: remote_inv }
  p_worker:
    machines: [parent_synth_inline__sce_synth_invoke__remote_inv]
    contains:
      invokes:
        - { machine: parent_synth_inline__sce_synth_invoke__remote_inv, invoke: unused }
"#;

const CHILD: &str = "parent_synth_inline__sce_synth_invoke__remote_inv";

/// `generate -l cpp` of `document` into the directory it stands in, with
/// `extra` arguments, which has to succeed.
fn generate(document: &Path, extra: &[&str]) {
    let dir = document
        .parent()
        .expect("the document stands in a directory");
    let run = Command::new(env!("CARGO_BIN_EXE_sce-codegen"))
        .arg("generate")
        .arg(document)
        .args(["-l", "cpp", "--no-format", "-o"])
        .arg(dir)
        .args(extra)
        .output()
        .expect("spawn sce-codegen");
    assert!(
        run.status.success(),
        "generate {} {extra:?} failed:\n{}",
        document.display(),
        String::from_utf8_lossy(&run.stderr)
    );
}

/// The parent and the deploy staged into a directory of their own, as the
/// CMake rule stages them; the directory (kept alive) and the parent's path.
fn staged() -> (tempfile::TempDir, std::path::PathBuf) {
    let dir = tempfile::tempdir().expect("tempdir");
    let parent = dir.path().join("parent_synth_inline.scxml");
    std::fs::write(&parent, PARENT).expect("write the parent");
    std::fs::write(dir.path().join("deploy.yaml"), DEPLOY).expect("write the deploy");
    (dir, parent)
}

#[test]
fn a_child_in_the_parents_process_is_written_by_the_parents_run() {
    let (dir, parent) = staged();
    generate(&parent, &[]);
    assert!(
        dir.path().join(format!("{CHILD}_sm.h")).is_file(),
        "the parent's run did not write the machine of a child in its own process"
    );
}

#[test]
fn a_child_on_another_partition_is_written_by_that_partitions_run() {
    let (dir, parent) = staged();
    let deploy = dir.path().join("deploy.yaml");
    let deploy = deploy.to_str().expect("a UTF-8 path");
    generate(&parent, &["--deploy", deploy]);
    assert!(
        dir.path().join("parent_synth_inline_sm.h").is_file(),
        "the parent's run did not write the parent's machine"
    );
    let child_document = dir.path().join(format!("{CHILD}.scxml"));
    assert!(
        child_document.is_file(),
        "the parent's run did not write the child's document, which the child's partition \
         generates from"
    );
    let child_machine = dir.path().join(format!("{CHILD}_sm.h"));
    assert!(
        !child_machine.exists(),
        "the parent's run wrote the machine of a child the deploy placed on another \
         partition, a file the run of that partition writes as well"
    );

    generate(&child_document, &["--deploy", deploy]);
    assert!(
        child_machine.is_file(),
        "the run of the child's partition did not write the child's machine"
    );
}
