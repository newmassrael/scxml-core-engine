// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// A C11 machine tells a trace port each step of Appendix D it takes only when its
// build asks (docs/adr/0115, D4 and D8): the machine's `<PREFIX><NAME>_TRACE_PORT`,
// whose default is the build's `SCE_TRACE_PORT`, 0 unless set. The claim is that a
// build that does not ask gets the machine it got before the port existed, and it
// holds when every line of the generated header and source that names the port
// lies inside a region the preprocessor keeps only when the switch is on: removing
// those regions removes the port entirely, and the rest of the output never
// depended on it. And a machine that asks tells every kind of step: each one the
// port opens, it closes and hands the host.
//
// This reads the generated text rather than compiling it, so it runs on a host with
// no C toolchain. That the machine builds and runs both ways is the C11 lane's: the
// W3C conformance runners build their machines with nothing set and define no hook,
// which links only while the build's default (`sce/types.h`) is off;
// `c11_integration_trace_port` and the `c11_a_run_of_test_*_is_the_recorded_run_of_its_model`
// programs build machines with it on and hold what the hook is told.

use std::path::{Path, PathBuf};
use std::process::Command;

/// The machine's name: the file stem and the `name` of its root.
const STEM: &str = "traced";

/// A document whose run takes every kind of step: a start, an internal event, an
/// eventless microstep, an external event and a stop.
const DOC: &str = r#"<scxml xmlns="http://www.w3.org/2005/07/scxml"
       version="1.0" name="traced" initial="idle" datamodel="null">
  <state id="idle">
    <onentry><raise event="poke"/></onentry>
    <transition event="poke" target="busy"/>
  </state>
  <state id="busy">
    <transition target="ready"/>
  </state>
  <state id="ready">
    <transition event="go" target="done"/>
  </state>
  <final id="done"/>
</scxml>
"#;

/// What a line names when it names the port: the runtime's recorder and its
/// header, the machine's helpers, the hook every machine's name takes, and the
/// member the machine holds the step in.
const MARKS: [&str; 4] = ["sce_trace_", "trace_port", "_trace_step", "->trace"];

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("sce-build has a parent dir")
        .to_path_buf()
}

/// The generated header and source of the document, as the templates write them:
/// unformatted, so a call and its arguments stay on the line they are written on.
fn generated(dir: &Path) -> (String, String) {
    let doc = dir.join(format!("{STEM}.scxml"));
    std::fs::write(&doc, DOC).expect("write the document");
    let out = dir.join("out");
    let output = Command::new(env!("CARGO_BIN_EXE_sce-codegen"))
        .env("SCE_WORKSPACE_ROOT", repo_root())
        .args([
            "generate",
            "-l",
            "c11",
            "-o",
            out.to_str().expect("a UTF-8 path"),
            "--no-format",
            doc.to_str().expect("a UTF-8 path"),
        ])
        .output()
        .expect("spawn sce-codegen");
    assert!(
        output.status.success(),
        "generate -l c11 failed:\nstdout: {}\nstderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr),
    );
    let read = |name: String| {
        std::fs::read_to_string(out.join(&name)).unwrap_or_else(|e| panic!("read {name}: {e}"))
    };
    (read(format!("{STEM}_sm.h")), read(format!("{STEM}_sm.c")))
}

/// The lines of `text` that name the port, each with whether it lies inside the
/// branch the preprocessor keeps when `switch` is on, and only then.
fn port_lines(text: &str, switch: &str) -> Vec<(String, bool)> {
    let wanted = format!("#if {switch}");
    // One entry per open conditional: whether its live branch is the one `switch`
    // turns on. An `#else` or `#elif` of it opens a branch that is not.
    let mut open: Vec<bool> = Vec::new();
    let mut found = Vec::new();
    for raw in text.lines() {
        let line = raw.trim();
        if line.starts_with("#if") {
            open.push(line == wanted);
        } else if line.starts_with("#else") || line.starts_with("#elif") {
            if let Some(top) = open.last_mut() {
                *top = false;
            }
        } else if line.starts_with("#endif") {
            open.pop();
        } else if MARKS.iter().any(|mark| line.contains(mark)) {
            found.push((line.to_string(), open.contains(&true)));
        }
    }
    found
}

/// How many lines of `text` hold `needle`.
fn lines_with(text: &str, needle: &str) -> usize {
    text.lines().filter(|line| line.contains(needle)).count()
}

#[test]
fn every_line_naming_the_port_is_kept_only_when_the_build_asks() {
    let dir = tempfile::tempdir().expect("tempdir");
    let (header, source) = generated(dir.path());
    let switch = "TRACED_TRACE_PORT";
    assert!(
        header.contains(&format!(
            "#ifndef {switch}\n#define {switch} SCE_TRACE_PORT\n"
        )),
        "a build that sets nothing for the machine takes the build's switch:\n{header}"
    );

    let declared = port_lines(&header, switch);
    let called = port_lines(&source, switch);
    assert!(
        declared
            .iter()
            .any(|(line, _)| line.contains("traced_trace_step(")),
        "the header declares the hook: {declared:?}"
    );
    assert!(
        declared
            .iter()
            .any(|(line, _)| line.contains("sce_trace_recorder_t trace;")),
        "the machine holds the step being told: {declared:?}"
    );
    assert!(
        called
            .iter()
            .any(|(line, _)| line.contains("traced_trace_step(sm, ")),
        "the source hands the host each step: {called:?}"
    );
    for (line, kept_only_when_on) in declared.iter().chain(&called) {
        assert!(
            *kept_only_when_on,
            "`{line}` names the trace port outside `#if {switch}`, so a build that did not ask \
             for the port would carry it"
        );
    }
}

#[test]
fn every_kind_of_step_is_opened_and_each_step_opened_is_closed() {
    let dir = tempfile::tempdir().expect("tempdir");
    let (_, source) = generated(dir.path());
    for kind in [
        "SCE_TRACE_START",
        "SCE_TRACE_EVENTLESS",
        "SCE_TRACE_INTERNAL",
        "SCE_TRACE_EXTERNAL",
        "SCE_TRACE_STOP",
    ] {
        assert_eq!(
            source
                .lines()
                .filter(|line| line.contains("sce_trace_begin(") && line.contains(kind))
                .count(),
            1,
            "one place opens a step of {kind}:\n{source}"
        );
    }
    assert_eq!(
        lines_with(&source, "sce_trace_begin("),
        5,
        "a step is opened only where Appendix D begins one"
    );
    assert_eq!(
        lines_with(&source, "trace_port_end(sm);"),
        5,
        "each place that opens a step closes it and hands it to the host"
    );
    assert_eq!(
        lines_with(&source, "sce_trace_exited("),
        2,
        "a state is told exited where a microstep exits it and where the interpreter's exit does"
    );
    assert_eq!(
        lines_with(&source, "sce_trace_entered("),
        1,
        "a state is told entered where every entry enters it"
    );
    assert_eq!(
        lines_with(&source, "sce_trace_fired("),
        1,
        "a transition is told where the microstep takes its set"
    );
}
