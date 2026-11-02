// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
//! `sce-codegen deployment` on the binary a build runs (docs/adr/0077 §7): one
//! file named by the deployment, holding what its deploy input gives each field;
//! checkable with `--assert-unchanged`, so a changed capacity is drift; and a
//! gate that is not one refused before anything is written.
//!
//! What the file holds and what it refuses is held by `forge_conformance`, on the
//! library. These cases hold the command's side of it.

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

fn scratch(label: &str) -> PathBuf {
    let dir =
        PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join(format!("{label}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("create scratch dir");
    dir
}

fn codec() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../tests/forge/resources/codec_repeat_basic.scxml")
}

fn deploy_input(dir: &Path, frags: u32) -> PathBuf {
    let path = dir.join("deploy.yaml");
    std::fs::write(
        &path,
        format!("codecs:\n  codec_repeat_basic:\n    capacity:\n      frags: {frags}\n"),
    )
    .expect("write deploy input");
    path
}

fn deployment(deploy: &Path, out: &Path, extra: &[&str], gates: &[&str]) -> Output {
    let mut command = Command::new(env!("CARGO_BIN_EXE_sce-codegen"));
    command
        .args(["--error-format", "json"])
        .args(extra)
        .arg("deployment")
        .arg("--deploy")
        .arg(deploy)
        .args(["--name", "Narrow"])
        .arg("-o")
        .arg(out);
    for gate in gates {
        command.args(["--gate", gate]);
    }
    command
        .arg(codec())
        .output()
        .expect("run sce-codegen deployment")
}

/// The file is the deployment's name, holds the capacity its input gives, and a
/// second run over the same inputs is unchanged; a changed capacity is drift.
#[test]
fn the_file_holds_its_deploy_input_and_is_checkable_for_drift() {
    let dir = scratch("deployment-drift");
    let deploy = deploy_input(&dir, 2);
    let out = dir.join("tree");
    let run = deployment(&deploy, &out, &[], &[]);
    assert!(
        run.status.success(),
        "{}",
        String::from_utf8_lossy(&run.stderr)
    );
    let text = std::fs::read_to_string(out.join("narrow.rs")).expect("the deployment's file");
    assert!(text.contains("pub struct Narrow;"), "{text}");
    assert!(text.contains("heapless::Vec<T, 2>"), "{text}");

    let check = deployment(&deploy, &out, &["--assert-unchanged"], &[]);
    assert!(
        check.status.success(),
        "a fresh file is unchanged: {}",
        String::from_utf8_lossy(&check.stderr)
    );
    deploy_input(&dir, 3);
    let check = deployment(&deploy, &out, &["--assert-unchanged"], &[]);
    assert!(!check.status.success(), "a changed capacity is drift");
    let _ = std::fs::remove_dir_all(&dir);
}

/// A gate is `<codec>=<cfg predicate>`; one that is not is refused, and nothing
/// is written.
#[test]
fn a_gate_that_is_not_one_is_refused() {
    let dir = scratch("deployment-gate");
    let deploy = deploy_input(&dir, 2);
    let out = dir.join("tree");
    let run = deployment(&deploy, &out, &[], &["codec_repeat_basic"]);
    assert!(!run.status.success());
    assert!(
        String::from_utf8_lossy(&run.stderr).contains("a gate is `<codec>=<cfg predicate>`"),
        "{}",
        String::from_utf8_lossy(&run.stderr)
    );
    assert!(!out.join("narrow.rs").exists());
    let _ = std::fs::remove_dir_all(&dir);
}
