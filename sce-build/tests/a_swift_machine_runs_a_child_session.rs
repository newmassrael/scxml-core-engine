// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
//! The Swift backend runs a child session an `<invoke type="scxml">` starts
//! (§scxml-6.4) in a statechart: the entry of the invoking state defers the start to
//! the end of the macrostep, the parent drives the child it holds (tick, settle,
//! forward, deliver), `done.invoke.<id>` reaches it once, and the exit of the state
//! cancels the child. The child is a machine of its own, written beside the parent.
//!
//! `scenarios/static_invoke.json` replays the machine the build generates
//! (`StaticDatamodelTests`); this file holds the text the generator writes, which
//! that reads, and the line between what is lowered and what is named in a refusal.
//!
//! # What was wrong
//!
//! Measured in the code on 2026-10-30: the Swift generator refused every `<invoke>`
//! by one sentence, so about fifteen fixtures had no Swift machine, while the runtime
//! already held the parent's half (`PendingInvoke`, `ChildEngine`,
//! `drainAndRaiseChildEvents`) and nothing conformed to `ChildEngine`.

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
/// directory: the exit status, and the text of every file written, by file name.
fn generate(machine: &str) -> (Option<i32>, Vec<(String, String)>, String) {
    let dir = tempdir().expect("tempdir");
    for entry in std::fs::read_dir(fixtures()).expect("the fixture directory") {
        let path = entry.expect("an entry").path();
        if path.is_file() {
            let name = path.file_name().expect("a file name").to_owned();
            std::fs::copy(&path, dir.path().join(name)).expect("copy a fixture");
        }
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
    let (status, files, said) = generate(machine);
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

/// The parent holds what a child session needs of it: the deferred starts, the
/// invocations running, and the session each `<invoke>` started, as the one type the
/// runtime conforms to `ChildEngine` for every child.
#[test]
fn a_parent_holds_the_sessions_its_invokes_started() {
    let code = machine_of("static_invoke.scxml", "static_invoke_sm.swift");
    holds_each(
        &code,
        &[
            "public final class StaticInvokePolicy: InvocablePolicy {",
            "private var pendingInvokes: [PendingInvoke<StaticInvokeState>] = []",
            "private var activeInvokes: [String: ChildSession] = [:]",
            "private var sceInvokeChildWorker: InvokedSession<StaticInvokeSceSynthInvokeWorkerPolicy>?",
            "public var hasInvokeSupport: Bool { true }",
            "public var hasChildTick: Bool { true }",
            "public var hasAutoforward: Bool { true }",
        ],
    );
}

/// Entering the invoking state defers the start to the end of the macrostep, and
/// leaving it cancels what was deferred and stops what runs: the exit of the session
/// runs its states' `<onexit>` before it is dropped.
#[test]
fn an_entry_defers_the_start_and_an_exit_stops_the_session() {
    let code = machine_of("static_invoke.scxml", "static_invoke_sm.swift");
    holds_each(
        &code,
        &[
            "deferInvoke(&pendingInvokes, PendingInvoke(",
            "cancelInvokesForState(&pendingInvokes, .working)",
            "sceInvokeChildWorker?.stop()",
            "sceInvokeChildWorker = nil",
            "activeInvokes[\"worker\"] = nil",
        ],
    );
}

/// The parent drives its child through the runtime's helpers and tells its own
/// machine when the child ends, once, with the donedata the child's final stashed.
#[test]
fn a_parent_drives_its_child_and_hears_it_end_once() {
    let code = machine_of("static_invoke.scxml", "static_invoke_sm.swift");
    holds_each(
        &code,
        &[
            "drainAndRaiseChildEvents(",
            "raiseDoneInvoke(invokeID: \"worker\", donedata: child.donedataAtFinal, engine: engine)",
            "session.engine.setClock(engine.clock())",
            "child.raiseExternalByNameWithMeta(eventName, metadata: metadata)",
        ],
    );
}

/// A machine with no `<invoke>` holds none of it: the flags say so and no member
/// is added.
#[test]
fn a_machine_with_no_invoke_holds_no_child_session() {
    let code = machine_of("static_counter.scxml", "static_counter_sm.swift");
    holds_each(
        &code,
        &[
            "public var hasInvokeSupport: Bool { false }",
            "public var hasChildTick: Bool { false }",
            "public var hasAutoforward: Bool { false }",
        ],
    );
    assert!(
        !code.contains("pendingInvokes") && !code.contains("InvokedSession"),
        "a machine with no <invoke> holds no child session:\n{code}"
    );
}

/// The child is a machine of its own, a file beside its parent, and it is the type
/// the parent names.
#[test]
fn the_child_is_written_beside_its_parent_as_a_machine_of_its_own() {
    let (status, files, said) = generate("static_invoke.scxml");
    assert_eq!(
        status,
        Some(0),
        "static_invoke is written for Swift: {said}"
    );
    let child = files
        .iter()
        .find(|(name, _)| name == "static_invoke__sce_synth_invoke__worker_sm.swift")
        .unwrap_or_else(|| {
            panic!(
                "the child is a file of the run: {:?}",
                files.iter().map(|f| &f.0).collect::<Vec<_>>()
            )
        });
    assert!(
        child.1.contains(
            "public final class StaticInvokeSceSynthInvokeWorkerPolicy: InvocablePolicy {"
        ),
        "the child is a policy the parent can run as a session:\n{}",
        child.1
    );
}

/// What an invocation hands its child is a value of the child's variable of the same
/// name: the child takes it in a slot of `InvokeParams` before it starts, and a slot
/// left `nil` keeps the value its `<data>` gave.
#[test]
fn a_child_takes_the_values_its_invoke_names_before_it_starts() {
    let (status, files, said) = generate("static_invoke_params.scxml");
    assert_eq!(status, Some(0), "static_invoke_params is written: {said}");
    let parent = files
        .iter()
        .find(|(name, _)| name == "static_invoke_params_sm.swift")
        .map(|f| f.1.as_str())
        .expect("the parent is a file of the run");
    holds_each(
        parent,
        &[
            "var seed = StaticInvokeParamsSceSynthInvokeWorkerPolicy.InvokeParams()",
            "childPolicy.acceptParams(seed)",
        ],
    );
    let child = files
        .iter()
        .find(|(name, _)| name == "static_invoke_params__sce_synth_invoke__worker_sm.swift")
        .map(|f| f.1.as_str())
        .expect("the child is a file of the run");
    holds_each(
        child,
        &[
            "public struct InvokeParams {",
            "public func acceptParams(_ params: InvokeParams) {",
        ],
    );
}

/// What an invocation hands its child when a value cannot be computed is left out and
/// reported (§scxml-5.7.1): the child starts with the value its `<data>` gave it.
#[test]
fn a_value_that_cannot_be_computed_is_left_out_and_reported() {
    let code = machine_of(
        "static_invoke_string.scxml",
        "static_invoke_string_sm.swift",
    );
    holds_each(&code, &["if sceFailure.failed {", "could not be read"]);
}

/// An `<invoke srcexpr>` starts the candidate its value names: the value is reduced to a
/// document stem, compared by its bytes with each declared candidate, and the session is
/// held by what a parent drives it through, since each candidate is a policy of its own
/// type. A value naming none of them starts nothing and raises `error.execution`.
#[test]
fn a_value_names_the_candidate_an_invoke_starts() {
    let code = machine_of(
        "static_invoke_hybrid.scxml",
        "static_invoke_hybrid_sm.swift",
    );
    holds_each(
        &code,
        &[
            "private var sceInvokeChildFirstRun: (any ChildEngine)?",
            "let sceSelected = documentStem(self.vPick)",
            "if sceSelected.utf8.elementsEqual(\"static_hybrid_first\".utf8) {",
            "} else if sceSelected.utf8.elementsEqual(\"static_hybrid_second\".utf8) {",
            "evaluated to a document it did not declare",
        ],
    );
}

/// An argument the chosen candidate declares no variable for is evaluated and left out
/// (§scxml-6.4.3), and one that cannot be evaluated is still reported (§scxml-5.7.1).
#[test]
fn an_argument_the_candidate_keeps_nowhere_is_evaluated_and_left_out() {
    let code = machine_of(
        "static_invoke_hybrid.scxml",
        "static_invoke_hybrid_sm.swift",
    );
    holds_each(
        &code,
        &[
            "let _: WireValue = .integer(Int64(Checked.mul(sceFailure, self.vBase, 2000000000)))",
            "could not be read",
        ],
    );
}

/// What a child hands back through `<finalize>`, an `idlocation`, and every other way
/// of running one are named in a refusal and not written wrong: exit status 7, the
/// construct and its state in the message.
#[test]
fn what_is_not_lowered_is_refused_by_name() {
    // One construct is left unlowered; the others it once named are lowered now.
    let (machine, construct) = ("static_invoke_tree.scxml", "an <invoke idlocation>");
    let (status, files, said) = generate(machine);
    assert_eq!(status, Some(7), "{machine} is refused, not written: {said}");
    assert!(
        said.contains(construct),
        "the refusal of {machine} names {construct}: {said}"
    );
    assert!(
        files.iter().all(|(name, _)| !name.ends_with("_sm.swift")),
        "a refused document writes no machine: {:?}",
        files.iter().map(|f| &f.0).collect::<Vec<_>>()
    );
}
