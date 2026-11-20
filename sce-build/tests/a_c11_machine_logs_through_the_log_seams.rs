// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! A C11 machine writes its `<log>` through the log seams (`sce/log.h`,
//! docs/adr/0110 D2), and the host's definition of them
//! (`backends/c/runtime/posix/log.c`) writes each line as the machine wrote it
//! to `stderr` before the seams existed.
//!
//! A machine's core does no I/O and formats no real number (docs/adr/0110,
//! D1): it hands the seam a text it built with `sce/text.h`, or a real whole
//! to `_sce_log_real`. What holds that here is the generated text, which names
//! the seams and states the `log` requirement a build links them by, and the
//! lines a machine logging a value of every scalar type writes once it is
//! linked with the host's seams, which are the lines the tests that read a
//! machine's log read.

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

/// A `sce-static` machine that logs, on entering its one state, a value of each
/// scalar type a `<log>` names, an absent optional value, a value with no label
/// and a label with no value.
const LOGGING: &str = r#"<?xml version="1.0"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       version="1.0" initial="logging" datamodel="sce-static" name="seam_log">
  <datamodel>
    <data id="count" sce:type="uint32" expr="7"/>
    <data id="delta" sce:type="int32" expr="-3"/>
    <data id="ready" sce:type="bool" expr="true"/>
    <data id="label" sce:type="string[16]" expr="'tally'"/>
    <data id="level" sce:type="float64" expr="0.5"/>
    <data id="due" sce:type="optional&lt;int64&gt;" expr="null"/>
  </datamodel>
  <state id="logging">
    <onentry>
      <log label="count" expr="count"/>
      <log label="delta" expr="delta"/>
      <log label="ready" expr="ready"/>
      <log label="label" expr="label"/>
      <log label="level" expr="level"/>
      <log label="due" expr="due"/>
      <log expr="count"/>
      <log label="plain"/>
    </onentry>
  </state>
</scxml>
"#;

/// The lines the host's seams write for [`LOGGING`], in document order.
const LINES: &str =
    "count: 7\ndelta: -3\nready: true\nlabel: tally\nlevel: 0.5\ndue: null\n7\nplain\n";

/// Generate [`LOGGING`] for C11 into a directory of its own: the directory, and
/// the manifest line the run printed.
fn generate() -> (tempfile::TempDir, serde_json::Value) {
    let dir = tempdir().expect("tempdir");
    let path = dir.path().join("seam_log.scxml");
    std::fs::write(&path, LOGGING).expect("write the machine");
    let out = Command::new(sce_codegen_bin())
        .arg("--workspace-root")
        .arg(repo_root())
        .arg("generate")
        .arg(&path)
        .args(["-l", "c11", "-o"])
        .arg(dir.path())
        .output()
        .expect("invoke sce-codegen");
    assert!(
        out.status.success(),
        "the machine generates for C11:\n{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let manifest: serde_json::Value =
        serde_json::from_str(String::from_utf8_lossy(&out.stdout).trim())
            .expect("the manifest line is JSON");
    (dir, manifest)
}

#[test]
fn a_c11_machine_hands_each_log_to_a_seam_and_states_that_it_logs() {
    let (dir, manifest) = generate();
    let source = std::fs::read_to_string(dir.path().join("seam_log_sm.c")).expect("the source");
    let holds = |needle: &str| {
        assert!(
            source.contains(needle),
            "the source does not hold `{needle}`:\n{source}"
        );
    };
    holds("#include <sce/log.h>");
    // A real is handed whole: the core formats none.
    holds("_sce_log_real(SCE_LOG_INFO, \"level\", sizeof(\"level\") - 1u, (double)(");
    // An integer is spelled by the runtime's own digits, each by its signedness.
    holds("sce_text_of_u64((uint64_t)(");
    holds("sce_text_of_i64((int64_t)(");
    holds("sce_log_ ? \"true\" : \"false\", sce_log_ ? 4u : 5u");
    holds("\"null\", 4u");
    assert!(
        !source.contains("fprintf(stderr"),
        "a `<log>` is written through the seam, not to a stream the core opens:\n{source}"
    );
    let requires: Vec<&str> = manifest["runtime_requires"]
        .as_array()
        .expect("a C11 run states what its machines require")
        .iter()
        .map(|v| v.as_str().expect("a requirement is a string"))
        .collect();
    assert!(
        requires.contains(&"log"),
        "a machine that logs states `log`, so a build links the seams: {requires:?}"
    );
}

#[test]
fn the_host_seams_write_each_line_as_the_machine_wrote_it() {
    let Some(cc) = sce_build::toolchain::locate_any(&["gcc", "cc"]) else {
        sce_build::toolchain::skipped(
            "the_host_seams_write_each_line_as_the_machine_wrote_it: no C compiler on PATH",
        );
        return;
    };
    let (dir, _) = generate();
    let driver = dir.path().join("driver.c");
    std::fs::write(
        &driver,
        r#"#include "seam_log_sm.h"
int main(void) {
    static seam_log_t sm;
    if (seam_log_init(&sm) != SEAM_LOG_START_OK) {
        return 1;
    }
    seam_log_run(&sm);
    return 0;
}
"#,
    )
    .expect("write the driver");
    let bin = dir.path().join("driver");
    let built = Command::new(&cc)
        .args(["-std=c11", "-O0", "-Wall", "-Wextra", "-Werror"])
        .arg("-I")
        .arg(repo_root().join("backends/c/runtime/include"))
        .arg("-I")
        .arg(repo_root().join("backends/c/forge-runtime/include"))
        .arg("-I")
        .arg(dir.path())
        .arg(dir.path().join("seam_log_sm.c"))
        .arg(repo_root().join("backends/c/runtime/posix/log.c"))
        // docs/adr/0110, D2: the machine states `real`, and the text unit
        // defines the real seams its core hands a real to.
        .arg(repo_root().join("backends/c/runtime/text/real_text.c"))
        .arg(&driver)
        .arg("-o")
        .arg(&bin)
        .output()
        .expect("run the C compiler");
    assert!(
        built.status.success(),
        "the machine, the host's seams and the driver build:\n{}",
        String::from_utf8_lossy(&built.stderr)
    );
    let ran = Command::new(&bin).output().expect("run the driver");
    assert!(
        ran.status.success(),
        "the machine starts and runs:\n{}",
        String::from_utf8_lossy(&ran.stderr)
    );
    assert_eq!(String::from_utf8_lossy(&ran.stderr), LINES);
}
