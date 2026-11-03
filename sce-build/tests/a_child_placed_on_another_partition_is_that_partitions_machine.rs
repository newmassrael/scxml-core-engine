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
//!   machine;
//! * the plan of the parent with the deploy (`--plan --plan-children`), asked
//!   where nothing has been generated, plans that run as well, with the
//!   deploy, and the run adds exactly the files it planned. A build declares
//!   the generation of the child's partition from that answer and names no
//!   file of it (the Mesh rule-3 override in tests/CMakeLists.txt).

use std::collections::BTreeSet;
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

/// The paths of the files directly in `dir`.
fn files_in(dir: &Path) -> BTreeSet<String> {
    std::fs::read_dir(dir)
        .expect("read the directory")
        .map(|entry| entry.expect("a directory entry").path())
        .filter(|path| path.is_file())
        .map(|path| path.display().to_string())
        .collect()
}

#[test]
fn the_parents_plan_plans_the_run_of_the_partition_its_child_is_placed_on() {
    let (dir, parent) = staged();
    let deploy = dir.path().join("deploy.yaml");
    let deploy = deploy.to_str().expect("a UTF-8 path");
    let staged_files = files_in(dir.path());

    // Asked where only the parent and the deploy are: the deploy reads the
    // child's document from where the parent's run writes it, which a plan
    // holds in memory rather than on disk.
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
    assert_eq!(
        files_in(dir.path()),
        staged_files,
        "the plan wrote into the directory"
    );
    let stdout = String::from_utf8_lossy(&plan.stdout).into_owned();
    let lines: Vec<&str> = stdout.lines().collect();
    let in_dir = |name: String| dir.path().join(name).display().to_string();
    let child_document = in_dir(format!("{CHILD}.scxml"));
    let child_machine = in_dir(format!("{CHILD}_sm.h"));
    // A C++ machine is a header and the inline definitions it includes.
    let child_inline = in_dir(format!("{CHILD}_sm.inl"));
    let child_transport = in_dir(format!("{CHILD}_transport.h"));

    let own: Vec<&str> = lines
        .iter()
        .copied()
        .filter(|line| !line.contains('\t'))
        .collect();
    assert!(
        own.contains(&child_document.as_str()),
        "the parent's run does not write the child's document:\n{stdout}"
    );
    assert!(
        !own.contains(&child_machine.as_str()),
        "the parent's run writes the machine of the child on another partition:\n{stdout}"
    );
    let arguments = format!("{child_document}\t\t--deploy\t{deploy}");
    assert_eq!(
        lines.iter().filter(|line| **line == arguments).count(),
        1,
        "the plan does not plan the run of the child's partition once, with the deploy:\n\
         {stdout}"
    );
    let file_prefix = format!("{child_document}\t");
    let planned: BTreeSet<String> = lines
        .iter()
        .filter_map(|line| line.strip_prefix(file_prefix.as_str()))
        .filter(|rest| !rest.starts_with('\t'))
        .map(str::to_string)
        .collect();
    assert_eq!(
        planned,
        BTreeSet::from([child_machine, child_inline, child_transport]),
        "the run of the child's partition is planned to write other files:\n{stdout}"
    );

    // The parent's run, then the run the plan planned, with the arguments it
    // stated: the second adds exactly what was planned for it.
    generate(&parent, &["--deploy", deploy]);
    let before = files_in(dir.path());
    generate(Path::new(&child_document), &["--deploy", deploy]);
    let added: BTreeSet<String> = files_in(dir.path()).difference(&before).cloned().collect();
    assert_eq!(
        added, planned,
        "the run of the child's partition added other files than its plan named"
    );
}
