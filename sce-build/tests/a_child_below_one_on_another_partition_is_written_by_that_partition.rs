// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
//! The document of an in-line child that sits below a child a deploy placed
//! on another partition (SCE_MESH.md §9.6.6 rule 3) is written by the run of
//! that partition, with the machine it writes beside it, and by no other run.
//!
//! # What was wrong
//!
//! The run that generates the parent wrote the document of every child it
//! carries, at every depth, before the deploy was applied, because the deploy
//! reads them. That included the children below the one placed on another
//! partition, whose own run carries them and writes their documents too: two
//! producers of one file, one of them undeclared, since the plan gave each
//! file to the first generation that wrote it.
//!
//! # What is held
//!
//! The parent carries `remote_inv`, which the deploy places on a partition of
//! its own, and `remote_inv` carries `inner_inv`, which stays with it:
//!
//! * without a deploy, the parent's run writes both documents and both
//!   machines (the control: a child in the parent's process is its run's);
//! * with the deploy, the parent's run writes the document of `remote_inv`
//!   and nothing of `inner_inv`, and the run of `remote_inv`'s partition
//!   writes the document and the machine of `inner_inv`;
//! * the plan of the parent (`--plan --plan-children`) says the same: the
//!   parent's lines name `remote_inv`'s document and not `inner_inv`'s, the
//!   generation it plans for `remote_inv` names `inner_inv`'s, and no file is
//!   named twice.

use std::collections::BTreeSet;
use std::path::Path;
use std::process::Command;

const PARENT: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       version="1.0" datamodel="ecmascript" initial="waiting" name="parent_nested">
    <state id="waiting">
        <invoke type="scxml" id="remote_inv">
            <content>
                <scxml xmlns="http://www.w3.org/2005/07/scxml"
                       version="1.0" initial="running">
                    <state id="running">
                        <invoke type="scxml" id="inner_inv">
                            <content>
                                <scxml xmlns="http://www.w3.org/2005/07/scxml"
                                       version="1.0" initial="done">
                                    <final id="done"/>
                                </scxml>
                            </content>
                        </invoke>
                        <transition event="done.invoke.inner_inv" target="finished"/>
                    </state>
                    <final id="finished"/>
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

/// The parent on one partition, its in-line child (and what that child
/// carries) on another.
const DEPLOY: &str = r#"version: "1.0"
topology:
  ecu1:
    machines:
      parent_nested:
        source: parent_nested.scxml
      parent_nested__sce_synth_invoke__remote_inv:
        source: parent_nested__sce_synth_invoke__remote_inv.scxml

partitions:
  p_parent:
    machines: [parent_nested]
    contains:
      invokes:
        - { machine: parent_nested, invoke: remote_inv }
  p_worker:
    machines: [parent_nested__sce_synth_invoke__remote_inv]
    contains:
      invokes:
        - { machine: parent_nested__sce_synth_invoke__remote_inv, invoke: inner_inv }
"#;

/// The child the deploy places on another partition.
const PLACED: &str = "parent_nested__sce_synth_invoke__remote_inv";
/// The child it carries, which stays with it.
const BELOW: &str = "parent_nested__sce_synth_invoke__remote_inv__sce_synth_invoke__inner_inv";

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
    let parent = dir.path().join("parent_nested.scxml");
    std::fs::write(&parent, PARENT).expect("write the parent");
    std::fs::write(dir.path().join("deploy.yaml"), DEPLOY).expect("write the deploy");
    (dir, parent)
}

#[test]
fn without_a_deploy_the_parents_run_writes_every_child_it_carries() {
    let (dir, parent) = staged();
    generate(&parent, &[]);
    for name in [
        format!("{PLACED}.scxml"),
        format!("{PLACED}_sm.h"),
        format!("{BELOW}.scxml"),
        format!("{BELOW}_sm.h"),
    ] {
        assert!(
            dir.path().join(&name).is_file(),
            "the parent's run did not write {name}, of a child in its own process"
        );
    }
}

#[test]
fn a_child_below_one_on_another_partition_is_written_by_that_partitions_run() {
    let (dir, parent) = staged();
    let deploy = dir.path().join("deploy.yaml");
    let deploy = deploy.to_str().expect("a UTF-8 path");
    generate(&parent, &["--deploy", deploy]);
    let placed_document = dir.path().join(format!("{PLACED}.scxml"));
    assert!(
        placed_document.is_file(),
        "the parent's run did not write the document of the child the deploy placed on \
         another partition, which that partition's run generates from"
    );
    let below_document = dir.path().join(format!("{BELOW}.scxml"));
    let below_machine = dir.path().join(format!("{BELOW}_sm.h"));
    assert!(
        !below_document.exists(),
        "the parent's run wrote the document of a child below the one placed on another \
         partition, a file the run of that partition writes as well"
    );
    assert!(
        !below_machine.exists(),
        "the parent's run wrote the machine of a child below the one placed on another \
         partition"
    );

    generate(&placed_document, &["--deploy", deploy]);
    assert!(
        below_document.is_file(),
        "the run of the partition did not write the document of the child it carries"
    );
    assert!(
        below_machine.is_file(),
        "the run of the partition did not write the machine of the child it carries"
    );
}

#[test]
fn the_parents_plan_gives_the_document_below_to_the_partitions_generation() {
    let (dir, parent) = staged();
    let deploy = dir.path().join("deploy.yaml");
    let deploy = deploy.to_str().expect("a UTF-8 path");

    let plan = Command::new(env!("CARGO_BIN_EXE_sce-codegen"))
        .args(["--plan", "--plan-children", "generate"])
        .arg(&parent)
        .args(["-l", "cpp", "--no-format", "-o"])
        .arg(dir.path())
        .args(["--deploy", deploy])
        .output()
        .expect("spawn sce-codegen");
    assert!(
        plan.status.success(),
        "the plan of the parent with the deploy failed:\n{}",
        String::from_utf8_lossy(&plan.stderr)
    );
    let stdout = String::from_utf8_lossy(&plan.stdout).into_owned();
    let lines: Vec<&str> = stdout.lines().collect();
    let in_dir = |name: String| dir.path().join(name).display().to_string();
    let placed_document = in_dir(format!("{PLACED}.scxml"));
    let below_document = in_dir(format!("{BELOW}.scxml"));

    let own: Vec<&str> = lines
        .iter()
        .copied()
        .filter(|line| !line.contains('\t'))
        .collect();
    assert!(
        own.contains(&placed_document.as_str()),
        "the parent's run does not write the document of the child placed on another \
         partition:\n{stdout}"
    );
    assert!(
        !own.contains(&below_document.as_str()),
        "the parent's run writes the document of a child below the one placed on another \
         partition:\n{stdout}"
    );

    let file_prefix = format!("{placed_document}\t");
    let placed_files: BTreeSet<&str> = lines
        .iter()
        .filter_map(|line| line.strip_prefix(file_prefix.as_str()))
        .filter(|rest| !rest.starts_with('\t'))
        .collect();
    assert!(
        placed_files.contains(below_document.as_str()),
        "the generation planned for the partition does not write the document of the child \
         it carries:\n{stdout}"
    );

    // Every file once: the parent's own lines, and each generation's files.
    let files: Vec<&str> = lines
        .iter()
        .filter_map(|line| match line.split_once('\t') {
            None => Some(*line),
            Some((_, rest)) if !rest.starts_with('\t') => Some(rest),
            Some(_) => None,
        })
        .filter(|path| !path.ends_with("sce_sourcemap.json"))
        .collect();
    let distinct: BTreeSet<&str> = files.iter().copied().collect();
    assert_eq!(
        distinct.len(),
        files.len(),
        "the plan names a file for two generations:\n{stdout}"
    );
}
