// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// A C++ machine tells a trace port each step of Appendix D it takes only when its
// build asks (docs/adr/0115, D4 and D8): the machine's `<NAME>_TRACE_PORT`, whose
// default is the build's `SCE_TRACE_PORT` (core/TracePort.h), 0 unless set, handed
// to the engine as its policy's `TRACE_PORT`. The hooks are the engine's
// (static/StaticExecutionEngine.h), each a branch `if constexpr` discards when the
// policy's switch is off, so what the generated machine holds of the port is the
// switch and nothing else: a build that does not ask gets the machine it got before
// the port existed, and nothing generated names the recorder.
//
// This reads the generated text rather than compiling it, so it runs on a host with
// no C++ toolchain. That the machine builds and runs both ways is the C++ lane's:
// the W3C conformance programs and integration_tests build their machines with
// nothing set; `IntegrationTracePort`, the
// `cpp_a_run_of_test_*_is_the_recorded_run_of_its_model` programs and
// `cpp_a_scenario_of_the_turnstile_is_its_run` build machines with it on and
// hold what the port is told.

use std::path::{Path, PathBuf};
use std::process::Command;

/// The machine's name: the file stem and the `name` of its root.
const STEM: &str = "traced";

/// The machine's switch, as the template spells it from the name.
const SWITCH: &str = "TRACED_TRACE_PORT";

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

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("sce-build has a parent dir")
        .to_path_buf()
}

/// The generated header and inline part of the document, as the templates write
/// them: unformatted, so a declaration stays on the line it is written on.
fn generated(dir: &Path) -> (String, String) {
    let doc = dir.join(format!("{STEM}.scxml"));
    std::fs::write(&doc, DOC).expect("write the document");
    let out = dir.join("out");
    let output = Command::new(env!("CARGO_BIN_EXE_sce-codegen"))
        .env("SCE_WORKSPACE_ROOT", repo_root())
        .args([
            "generate",
            "-l",
            "cpp",
            "-o",
            out.to_str().expect("a UTF-8 path"),
            "--no-format",
            doc.to_str().expect("a UTF-8 path"),
        ])
        .output()
        .expect("spawn sce-codegen");
    assert!(
        output.status.success(),
        "generate -l cpp failed:\nstdout: {}\nstderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr),
    );
    let read = |name: String| {
        std::fs::read_to_string(out.join(&name)).unwrap_or_else(|e| panic!("read {name}: {e}"))
    };
    (read(format!("{STEM}_sm.h")), read(format!("{STEM}_sm.inl")))
}

/// The lines of `text` that are code (not a `//` comment) and hold `needle`,
/// trimmed.
fn code_lines_with<'a>(text: &'a str, needle: &str) -> Vec<&'a str> {
    text.lines()
        .map(str::trim)
        .filter(|line| !line.starts_with("//") && line.contains(needle))
        .collect()
}

#[test]
fn a_build_that_sets_nothing_for_the_machine_takes_the_builds_switch() {
    let dir = tempfile::tempdir().expect("tempdir");
    let (header, _) = generated(dir.path());
    assert!(
        header.contains(&format!(
            "#ifndef {SWITCH}\n#define {SWITCH} SCE_TRACE_PORT\n#endif\n"
        )),
        "a build that sets nothing for the machine takes the build's switch:\n{header}"
    );
    assert_eq!(
        code_lines_with(&header, "static constexpr bool TRACE_PORT"),
        vec![format!(
            "static constexpr bool TRACE_PORT = ({SWITCH}) != 0;"
        )],
        "the policy hands the machine's switch to the engine, once"
    );
}

#[test]
fn the_machine_holds_the_switch_and_nothing_else_of_the_port() {
    let dir = tempfile::tempdir().expect("tempdir");
    let (header, inl) = generated(dir.path());
    let switch_lines = code_lines_with(&header, "TRACE_PORT");
    assert_eq!(
        switch_lines,
        vec![
            format!("#ifndef {SWITCH}"),
            format!("#define {SWITCH} SCE_TRACE_PORT"),
            format!("static constexpr bool TRACE_PORT = ({SWITCH}) != 0;"),
        ],
        "the switch is spelled where it is defaulted and where the policy reads it, and nowhere else"
    );
    assert!(
        code_lines_with(&inl, "TRACE_PORT").is_empty(),
        "the inline part names no switch"
    );
    for needle in [
        "traceRecorder_",
        "TraceRecorder",
        "TraceStep",
        "setTracePort",
    ] {
        assert!(
            code_lines_with(&header, needle).is_empty() && code_lines_with(&inl, needle).is_empty(),
            "the machine names `{needle}`, which is the engine's: the hooks of the port belong to \
             static/StaticExecutionEngine.h, where the switch discards them"
        );
    }
}
