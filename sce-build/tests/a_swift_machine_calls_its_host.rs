// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
//! The Swift backend lowers `<sce:action>` (§scxml-G-7) in a `sce-static` statechart to
//! a call of the host the machine is given: the host's interface is a class-bound
//! protocol with one method per act, a recorder that implements it is written beside
//! it, the machine takes its host at construction, and a machine that states a duty
//! (docs/adr/0038) refuses a host that only simulates it unless it is built for one on
//! purpose.
//!
//! `scenarios/static_host_call.json` and `static_host_duty.json` replay the machines
//! the build generates (`StaticDatamodelTests`), reading what each asked of a recording
//! host; this file holds the text the generator writes, which those read, and the
//! line it refuses by name.
//!
//! # What was wrong
//!
//! Measured in the code on 2026-10-31: the Swift generator refused every `<sce:action>`
//! by name, so four fixtures had no Swift machine, and the host interface a Swift host
//! implements was a hook that panicked (`no_swift_emitter`) for any renderer that
//! reached it. An optional argument of a host method was refused for Swift alone.

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

fn fixtures() -> PathBuf {
    repo_root().join("sce-build/tests/fixtures/static_datamodel")
}

/// What `sce-codegen generate -l swift` made of `machine`, a document of the fixture
/// directory or one of `extra` (name and text, written beside the fixtures): the exit
/// status, the text of every file written by name, and what it said.
fn generate(machine: &str, extra: &[(&str, &str)]) -> (Option<i32>, Vec<(String, String)>, String) {
    let dir = tempdir().expect("tempdir");
    for entry in std::fs::read_dir(fixtures()).expect("the fixture directory") {
        let path = entry.expect("an entry").path();
        if path.is_file() {
            let name = path.file_name().expect("a file name").to_owned();
            std::fs::copy(&path, dir.path().join(name)).expect("copy a fixture");
        }
    }
    for (name, text) in extra {
        std::fs::write(dir.path().join(name), text).expect("write a document");
    }
    let out_dir = tempdir().expect("tempdir");
    let out = Command::new(sce_codegen_bin())
        .arg("--workspace-root")
        .arg(repo_root())
        .arg("--error-format=json")
        .args(["generate", "-l", "swift"])
        .arg(dir.path().join(machine))
        .arg("-o")
        .arg(out_dir.path())
        .output()
        .expect("invoke sce-codegen");
    let mut files = Vec::new();
    for entry in std::fs::read_dir(out_dir.path()).expect("the output directory") {
        let path = entry.expect("an entry").path();
        if path.is_file() {
            let name = path
                .file_name()
                .expect("a file name")
                .to_string_lossy()
                .into_owned();
            files.push((name, std::fs::read_to_string(&path).unwrap_or_default()));
        }
    }
    files.sort();
    let said = format!(
        "{}{}",
        String::from_utf8_lossy(&out.stderr),
        String::from_utf8_lossy(&out.stdout)
    );
    (out.status.code(), files, said)
}

/// The Swift machine file of `machine`, written for a document that is lowered.
fn machine_of(machine: &str, file: &str) -> String {
    let (status, files, said) = generate(machine, &[]);
    assert_eq!(status, Some(0), "{machine} is written for Swift: {said}");
    files
        .into_iter()
        .find(|(name, _)| name == file)
        .unwrap_or_else(|| panic!("{machine} writes {file}"))
        .1
}

/// `written` holds each of `needles`, or the test names the one it lacks and shows
/// what was written.
fn holds_each(written: &str, needles: &[&str]) {
    for needle in needles {
        assert!(
            written.contains(needle),
            "the generated Swift holds `{needle}`:\n{written}"
        );
    }
}

/// The host is a class-bound protocol with one method per act, a parameter for each
/// argument in the order the document gives it, and an argument that may be absent in
/// the optional type over the one a present value has.
#[test]
fn a_host_is_a_protocol_with_a_method_for_each_act() {
    let code = machine_of("static_host_call.scxml", "static_host_call_sm.swift");
    holds_each(
        &code,
        &[
            "public protocol StaticHostCallActions: AnyObject {",
            "    func showAttempts(_ count: UInt32, _ exhausted: Bool)\n",
            "    func showDue(_ day: Int32?, _ urgent: Bool?)\n",
        ],
    );
}

/// A recorder implements the protocol, keeps each call as a case that compares by value,
/// and says nothing about duties where the document states none.
#[test]
fn a_recorder_keeps_each_call_as_a_value() {
    let code = machine_of("static_host_call.scxml", "static_host_call_sm.swift");
    holds_each(
        &code,
        &[
            "public final class RecordingStaticHostCallActions: StaticHostCallActions {",
            "public enum Call: Equatable {",
            "case showAttempts(UInt32, Bool)",
            "case showDue(Int32?, Bool?)",
            "public private(set) var calls: [Call] = []",
            "calls.append(.showDue(day, urgent))",
        ],
    );
    assert!(
        !code.contains("realization"),
        "a document with no duty states no realization:\n{code}"
    );
}

/// The machine takes its host when it is built, since the initial state's `<onentry>`
/// can already perform an act, and calls it directly: no script engine is asked.
#[test]
fn a_machine_takes_its_host_at_construction_and_calls_it_directly() {
    let code = machine_of("static_host_call.scxml", "static_host_call_sm.swift");
    holds_each(
        &code,
        &[
            "private let actions: any StaticHostCallActions",
            "public init(actions: any StaticHostCallActions) {",
            "self.actions.showAttempts(self.vAttempts, self.vAttempts >= 3)",
            "self.actions.showDue(self.vDue, self.vUrgent)",
        ],
    );
    assert!(
        !code.contains("acceptSimulatedHost"),
        "a machine with no duty takes no statement about a simulated host:\n{code}"
    );
}

/// A machine that states a duty carries the realization its host owes, and traps for a
/// host that says it only simulates unless it was built for one on purpose; the recorder
/// is such a host.
#[test]
fn a_machine_that_states_a_duty_refuses_a_host_that_only_simulates_it() {
    let code = machine_of("static_host_duty.scxml", "static_host_duty_sm.swift");
    holds_each(
        &code,
        &[
            "    var realization: ActionRealization { get }\n",
            "public let realization: ActionRealization = .simulates",
            "public init(actions: any StaticHostDutyActions, acceptSimulatedHost: Bool = false) {",
            "actions.realization == .performs || acceptSimulatedHost,",
        ],
    );
}

/// A child that declares acts is built with a host its parent's host answers, asked once
/// for each start: the parent's interface has an operation for the child, the recorder
/// takes a source for it and records the question, and the child is built with what the
/// host answered. The parent declares no act of its own.
#[test]
fn a_parents_host_answers_the_host_of_the_child_it_starts() {
    let code = machine_of("static_child_host.scxml", "static_child_host_sm.swift");
    holds_each(
        &code,
        &[
            "    func actionsForWorker() -> any StaticChildHostSceSynthInvokeWorkerActions\n",
            "public init(actionsForWorkerSource: @escaping () -> any StaticChildHostSceSynthInvokeWorkerActions) {",
            "calls.append(.actionsForWorker)",
            "StaticChildHostSceSynthInvokeWorkerPolicy(actions: self.actions.actionsForWorker())",
        ],
    );
}

/// Each candidate of a hybrid `<invoke>` is its own document with acts of its own, so the
/// parent's host answers a host for each, and a candidate is built with the host of its own
/// and with no other's.
#[test]
fn a_candidate_is_built_with_the_host_of_its_own() {
    let code = machine_of(
        "static_child_host_hybrid.scxml",
        "static_child_host_hybrid_sm.swift",
    );
    holds_each(
        &code,
        &[
            "    func actionsForWork0jStaticHostedFirst() -> any StaticHostedFirstActions\n",
            "    func actionsForWork0jStaticHostedSecond() -> any StaticHostedSecondActions\n",
            "StaticHostedFirstPolicy(actions: self.actions.actionsForWork0jStaticHostedFirst())",
            "StaticHostedSecondPolicy(actions: self.actions.actionsForWork0jStaticHostedSecond())",
        ],
    );
}

/// A child that states a duty is built under its parent's own statement about a host that
/// only simulates, which the parent's policy does not hold yet: it is named in a refusal and
/// not written wrong.
#[test]
fn a_child_that_states_a_duty_is_refused_by_name() {
    let document = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       version="1.0" initial="working" datamodel="sce-static" name="duty_parent">
  <state id="working">
    <invoke type="scxml" id="worker">
      <content>
        <scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
               version="1.0" initial="busy" datamodel="sce-static">
          <state id="busy">
            <onentry>
              <sce:action name="start_keepalive" effect="duty" released-by="stop_keepalive"/>
            </onentry>
            <onexit>
              <sce:action name="stop_keepalive" effect="duty"/>
            </onexit>
          </state>
        </scxml>
      </content>
    </invoke>
  </state>
</scxml>
"#;
    let (status, files, said) = generate("duty_parent.scxml", &[("duty_parent.scxml", document)]);
    assert_eq!(
        status,
        Some(7),
        "the document is refused, not written: {said}"
    );
    assert!(
        said.contains("a child that states a duty"),
        "the refusal names the child's duty: {said}"
    );
    assert!(
        files.iter().all(|(name, _)| !name.ends_with("_sm.swift")),
        "a refused document writes no machine: {:?}",
        files.iter().map(|f| &f.0).collect::<Vec<_>>()
    );
}

/// An argument that reads the data of the event is named in a refusal, not written
/// wrong: the typed payload a call reads is bound by a guard Swift has not got.
#[test]
fn an_argument_that_reads_the_events_data_is_refused_by_name() {
    let document = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       version="1.0" initial="waiting" datamodel="sce-static" name="host_reads_event">
  <sce:import kind="event-schema" src="schema_day.scxml" as="Day"/>
  <state id="waiting">
    <transition event="day.picked" type="internal">
      <sce:action name="showDay" effect="notice">
        <sce:arg name="day" expr="_event.data.dayOfMonth"/>
      </sce:action>
    </transition>
  </state>
</scxml>
"#;
    let (status, files, said) = generate(
        "host_reads_event.scxml",
        &[("host_reads_event.scxml", document)],
    );
    assert_eq!(
        status,
        Some(7),
        "the document is refused, not written: {said}"
    );
    assert!(
        said.contains("reads the data of the event"),
        "the refusal names the argument: {said}"
    );
    assert!(
        files.iter().all(|(name, _)| !name.ends_with("_sm.swift")),
        "a refused document writes no machine: {:?}",
        files.iter().map(|f| &f.0).collect::<Vec<_>>()
    );
}
