// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// docs/adr/0124: every generated Rust machine hands the data of the event
// being processed to its hooks the same way, with one signature for every
// hook, whether its events carry no data, carry data a guard reads, or carry
// data an act reads. Nothing about it is a build option: the same command
// writes every machine, and no machine names a second policy trait or a
// second set of hooks.

use std::path::{Path, PathBuf};
use std::process::Command;

fn sce_codegen_bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_sce-codegen"))
}

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("sce-build manifest dir has parent (workspace root)")
        .to_path_buf()
}

/// The Rust machine `sce-codegen generate -l rust` writes for `fixture`, its
/// whitespace collapsed so a formatter's line breaks do not decide a match.
fn generated(subdir: &str, fixture: &Path) -> String {
    let out = PathBuf::from(env!("CARGO_TARGET_TMPDIR"))
        .join("one_hook_signature")
        .join(subdir);
    let _ = std::fs::remove_dir_all(&out);
    std::fs::create_dir_all(&out).expect("create scratch");
    let output = Command::new(sce_codegen_bin())
        .args(["generate", "-l", "rust", "-o"])
        .arg(&out)
        .arg(fixture)
        .output()
        .expect("spawn sce-codegen");
    assert!(
        output.status.success(),
        "sce-codegen generate -l rust {} failed (exit {:?})\nstderr: {}",
        fixture.display(),
        output.status.code(),
        String::from_utf8_lossy(&output.stderr),
    );
    let mut text = String::new();
    for entry in std::fs::read_dir(&out).expect("read scratch").flatten() {
        let path = entry.path();
        if path.extension().and_then(|s| s.to_str()) == Some("rs") {
            text.push_str(&std::fs::read_to_string(&path).expect("read .rs"));
            text.push('\n');
        }
    }
    assert!(!text.is_empty(), "no .rs emitted for {}", fixture.display());
    collapsed(&text)
}

fn collapsed(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// The hooks every machine writes that run executable content, each handed the
/// event's data in the one parameter every machine names. A machine with a
/// guard writes `guard_holds` with the same parameter ([`GUARD_HOOK`]).
const HOOKS: &[&str] = &[
    "fn execute_transition_content( &mut self, transition: ::sce_rust_runtime::TransitionId, \
     sce_event_data: Self::EventData<'_>,",
    "fn execute_entry_actions(&mut self, state: ::sce_rust_runtime::StateId, \
     sce_event_data: Self::EventData<'_>,",
    "fn execute_exit_actions( &mut self, state: ::sce_rust_runtime::StateId, \
     sce_event_data: Self::EventData<'_>,",
];

/// The hook that runs a guard (docs/adr/0127), written by a machine that has
/// one.
const GUARD_HOOK: &str =
    "fn guard_holds( &mut self, transition: ::sce_rust_runtime::TransitionId, \
     sce_event_data: Self::EventData<'_>,";

fn assert_one_signature(name: &str, code: &str) {
    for hook in HOOKS {
        let hook = collapsed(hook);
        assert!(code.contains(&hook), "{name}: expected `{hook}`:\n{code}");
    }
    assert!(
        code.contains(&collapsed(
            "fn event_data<'a>(held: &'a ::sce_rust_runtime::CurrentEventData<'_, Self>) \
             -> Self::EventData<'a>"
        )) || code.contains(&collapsed(
            "fn event_data<'a>(_held: &'a ::sce_rust_runtime::CurrentEventData<'_, Self>) \
             -> Self::EventData<'a>"
        )),
        "{name}: expected the policy's reading of the data it is handed:\n{code}"
    );
    for absent in [
        "EventContextPolicy",
        "pending_payload",
        "populate_event_payload",
    ] {
        assert!(
            !code.contains(absent),
            "{name}: no second trait, second hook or stored payload, found `{absent}`:\n{code}"
        );
    }
}

#[test]
fn a_machine_whose_events_carry_no_data_is_handed_none() {
    let code = generated(
        "traffic_light",
        &repo_root().join("examples/traffic_light/traffic_light.scxml"),
    );
    assert_one_signature("traffic_light", &code);
    assert!(
        code.contains("type EventData<'e> = ();"),
        "a machine with no typed data reads it as `()`:\n{code}"
    );
}

#[test]
fn a_machine_whose_guard_reads_its_data_is_handed_it_the_same_way() {
    let code = generated(
        "statechart_minimal",
        &repo_root().join("sce-build/tests/fixtures/event_schema/statechart_minimal.scxml"),
    );
    assert_one_signature("statechart_minimal", &code);
    // A schema of numbers alone borrows nothing, so its parts take no lifetime.
    for expected in [
        "type EventData<'e> = StatechartMinimalEventData;",
        "pub struct StatechartMinimalJobCompletedRef {",
        GUARD_HOOK,
        "matches!(sce_event_data, StatechartMinimalEventData::JobCompleted(ev) if ev.elapsed_ms == 0)",
        "fn process_job_completed_with(&mut self, event_data: StatechartMinimalJobCompletedRef);",
    ] {
        let expected = collapsed(expected);
        assert!(code.contains(&expected), "expected `{expected}`:\n{code}");
    }
}

#[test]
fn a_machine_whose_act_reads_its_data_is_handed_it_the_same_way() {
    let code = generated(
        "statechart_native_action",
        &repo_root().join("sce-build/tests/fixtures/event_schema/statechart_native_action.scxml"),
    );
    assert_one_signature("statechart_native_action", &code);
    // A byte string is borrowed, so the parts and the data take `'e`.
    for expected in [
        "type EventData<'e> = StatechartNativeActionEventData<'e>;",
        "pub struct StatechartNativeActionFragmentReceivedRef<'e> {",
        "pub payload: &'e [u8],",
        "match sce_event_data {",
        "StatechartNativeActionEventData::FragmentReceived(ev) =>",
        "fn process_fragment_received_with(&mut self, \
         event_data: StatechartNativeActionFragmentReceivedRef<'_>);",
    ] {
        let expected = collapsed(expected);
        assert!(code.contains(&expected), "expected `{expected}`:\n{code}");
    }
}
