// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// A C11 machine tells a transition observer each transition it takes only when its
// build asks (docs/adr/0089): the machine's `<PREFIX><NAME>_TRANSITION_OBSERVER`,
// whose default is the build's `SCE_TRANSITION_OBSERVER`, 0 unless set. The claim
// is that a build that does not ask gets the machine it got before the observer
// existed, and it holds when every line of the generated header and source that
// names the observer's hook lies inside a region the preprocessor keeps only when
// the switch is on: removing those regions removes the observer entirely, and the
// rest of the output never depended on it.
//
// This reads the generated text rather than compiling it, so it runs on a host with
// no C toolchain. That the machine builds and runs both ways is the C11 lane's: the
// `c11_integration_a_computed_event_name_is_matched_like_any_other` target builds a
// machine with nothing set and defines no hook, which links only while the build's
// default (`sce/types.h`) is off, and `c11_integration_transition_observer` builds
// the same machine with it on and holds what the hook is told to the machine's
// transition table.

use std::path::{Path, PathBuf};
use std::process::Command;

/// The machine's name: the file stem and the `name` of its root.
const STEM: &str = "observed";

/// A document with a state of two transitions, so the microstep's call has a set of
/// more than one to walk, and a final state.
const DOC: &str = r#"<scxml xmlns="http://www.w3.org/2005/07/scxml"
       version="1.0" name="observed" initial="idle" datamodel="null">
  <state id="idle">
    <transition event="go" target="busy"/>
    <transition event="poke"/>
  </state>
  <state id="busy">
    <transition event="stop" target="done"/>
  </state>
  <final id="done"/>
</scxml>
"#;

/// The hook a machine that observes calls, by the suffix every machine's name takes.
const HOOK: &str = "_observe_transition";

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("sce-build has a parent dir")
        .to_path_buf()
}

/// The generated header and source of the document, as text.
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

/// The switch the header declares for its machine: the name in its `#if` line.
fn switch_of(header: &str) -> String {
    header
        .lines()
        .map(str::trim)
        .find_map(|line| {
            line.strip_prefix("#if ")
                .map(str::trim)
                .filter(|name| name.ends_with("_TRANSITION_OBSERVER"))
        })
        .expect("the header declares the machine's observer switch")
        .to_string()
}

/// The lines of `text` that name the hook, each with whether it lies inside the
/// branch the preprocessor keeps when `switch` is on, and only then.
fn hook_lines(text: &str, switch: &str) -> Vec<(String, bool)> {
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
        } else if line.contains(HOOK) {
            found.push((line.to_string(), open.contains(&true)));
        }
    }
    found
}

#[test]
fn every_line_naming_the_hook_is_kept_only_when_the_build_asks() {
    let dir = tempfile::tempdir().expect("tempdir");
    let (header, source) = generated(dir.path());
    let switch = switch_of(&header);
    assert_eq!(
        switch, "OBSERVED_TRANSITION_OBSERVER",
        "the switch is named by the machine"
    );
    assert!(
        header.contains(&format!("#define {switch} SCE_TRANSITION_OBSERVER")),
        "a build that sets nothing for the machine takes the build's switch"
    );

    let declared = hook_lines(&header, &switch);
    let called = hook_lines(&source, &switch);
    assert!(
        !declared.is_empty() && !called.is_empty(),
        "the header declares the hook and the microstep calls it: {declared:?} {called:?}"
    );
    for (line, kept_only_when_on) in declared.iter().chain(&called) {
        assert!(
            *kept_only_when_on,
            "`{line}` names the hook outside `#if {switch}`, so a build that did not ask for \
             the observer would carry it"
        );
    }
}
