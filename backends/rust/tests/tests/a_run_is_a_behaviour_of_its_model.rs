// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// A run of the Rust lane, recorded as the trace `sce-codegen formal traces`
// judges a behaviour of the document's model (docs/adr/0115, D4 and D8).
//
// Each test runs one W3C document that `docs/formal` has a model of
// (`W3CTest388`, `W3CTest570`, `W3CTest403c` over `AppendixD.tla`) with the
// engine's trace port set, and holds what the port was told to the recorded run
// `docs/formal/traces.json` declares of that model, byte for byte. The judge
// reads the recorded file, so a run that moves is seen here first, and a
// recorded run that no longer is the lane's is never judged in its place.
//
// Recording again, after a change that moves a run on purpose:
//   UPDATE_EXPECT=1 cargo test -p sce-rust-tests --test a_run_is_a_behaviour_of_its_model
// then `sce-codegen formal traces --manifest docs/formal/traces.json`, and read
// the diff of the recorded runs.

use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use sce_rust_runtime::trace_port::{RunTrace, StepKind, TraceStep};
use sce_rust_runtime::{Engine, IScriptEngine, StatePolicy};

/// The recorded run of `model`'s document by this lane, as the traces
/// manifest names it.
fn recorded_path(model: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../docs/formal/traces")
        .join(format!("{model}.rust.json"))
}

fn script_engine() -> Arc<dyn IScriptEngine> {
    Arc::new(sce_rust_lua::LuaEngine::new())
}

/// The trace port's account of one run of `policy`'s machine to the end, and
/// the top-level final state the run ended in.
fn run<P: StatePolicy>(policy: P) -> (RunTrace, Option<P::State>) {
    let trace = Arc::new(Mutex::new(RunTrace::new()));
    let sink = Arc::clone(&trace);
    let mut engine = Engine::new(policy);
    engine.set_trace_port(move |step: &TraceStep| {
        sink.lock().expect("the trace is not poisoned").push(step);
    });
    engine
        .initialize()
        .expect("the machine's script session is established");
    let completed = engine.run_until_completion(Duration::from_secs(3), Duration::from_millis(10));
    assert!(completed, "the run reached no final state");
    let told = trace.lock().expect("the trace is not poisoned").clone();
    (told, engine.terminal_state())
}

/// Hold `trace` to the recorded run of `model`, or record it again when
/// `UPDATE_EXPECT` is set.
fn holds_to_the_recorded_run(model: &str, trace: &RunTrace) {
    let text = trace.to_json();
    let path = recorded_path(model);
    if std::env::var_os("UPDATE_EXPECT").is_some() {
        std::fs::write(&path, &text).expect("the recorded run is writable");
        return;
    }
    let recorded = std::fs::read_to_string(&path).unwrap_or_else(|e| {
        panic!(
            "{} cannot be read ({e}): record it with `UPDATE_EXPECT=1 cargo test -p \
             sce-rust-tests --test a_run_is_a_behaviour_of_its_model`",
            path.display()
        )
    });
    assert_eq!(
        text, recorded,
        "the run of {model} is not the recorded one that `sce-codegen formal traces` judges"
    );
}

/// The run starts, ends in its pass state, and the interpreter's exit leaves
/// that state: the first and the last step a recorded run must hold.
fn starts_and_stops(trace: &RunTrace) {
    let steps = trace.steps();
    assert_eq!(steps.first().map(|step| step.kind), Some(StepKind::Start));
    let last = steps.last().expect("a run has steps");
    assert_eq!(last.kind, StepKind::Stop);
    assert_eq!(last.outcome, Some("pass"));
    assert_eq!(last.exited, vec!["pass"]);
}

#[test]
fn a_run_of_test_388_is_the_recorded_run_of_its_model() {
    use sce_rust_tests::generated::test388::{Test388Policy, Test388State};
    let (trace, ended) = run(Test388Policy::new(script_engine()));
    assert_eq!(ended, Some(Test388State::Pass));
    starts_and_stops(&trace);
    holds_to_the_recorded_run("W3CTest388", &trace);
}

#[test]
fn a_run_of_test_570_is_the_recorded_run_of_its_model() {
    use sce_rust_tests::generated::test570::{Test570Policy, Test570State};
    let (trace, ended) = run(Test570Policy::new(script_engine()));
    assert_eq!(ended, Some(Test570State::Pass));
    starts_and_stops(&trace);
    holds_to_the_recorded_run("W3CTest570", &trace);
}

#[test]
fn a_run_of_test_403c_is_the_recorded_run_of_its_model() {
    use sce_rust_tests::generated::test403c::{Test403cPolicy, Test403cState};
    let (trace, ended) = run(Test403cPolicy::new(script_engine()));
    assert_eq!(ended, Some(Test403cState::Pass));
    starts_and_stops(&trace);
    holds_to_the_recorded_run("W3CTest403c", &trace);
}

/// An event step names its event by the name `_event.name` holds while it is
/// taken, which the run trace format defines the field as: the whole name it
/// was sent under, and not the event of the document a door resolved it to
/// (`request`, or the wildcard). The document sends six events whose names it
/// computes, three to its external queue and three to its internal one, as the
/// C11 lane's `c11_integration_trace_port` holds of the same document.
#[test]
fn an_event_step_names_the_event_by_the_name_it_was_sent_under() {
    use sce_rust_tests::integration::a_computed_event_name_is_matched_like_any_other::{
        AComputedEventNameIsMatchedLikeAnyOtherPolicy, AComputedEventNameIsMatchedLikeAnyOtherState,
    };
    let (trace, ended) = run(AComputedEventNameIsMatchedLikeAnyOtherPolicy::new(
        script_engine(),
    ));
    assert_eq!(
        ended,
        Some(AComputedEventNameIsMatchedLikeAnyOtherState::Pass)
    );
    starts_and_stops(&trace);
    let events: Vec<(StepKind, Option<&str>)> = trace
        .steps()
        .iter()
        .filter(|step| matches!(step.kind, StepKind::Internal | StepKind::External))
        .map(|step| (step.kind, step.event.as_deref()))
        .collect();
    assert_eq!(
        events,
        vec![
            (StepKind::External, Some("request.new")),
            (StepKind::External, Some("other.thing")),
            (StepKind::Internal, Some("request.again")),
            (StepKind::Internal, Some("last.one")),
            (StepKind::Internal, Some("request.third")),
            (StepKind::External, Some("last.two")),
        ]
    );
}

/// A port set and then cleared is told nothing, and the run is the one an
/// engine that never had one makes.
#[test]
fn a_cleared_port_is_told_nothing_and_the_run_is_the_same() {
    use sce_rust_tests::generated::test388::{Test388Policy, Test388State};
    let told: Arc<Mutex<usize>> = Arc::default();
    let sink = Arc::clone(&told);
    let mut engine = Engine::new(Test388Policy::new(script_engine()));
    engine.set_trace_port(move |_: &TraceStep| {
        *sink.lock().expect("the count is not poisoned") += 1;
    });
    engine.clear_trace_port();
    engine
        .initialize()
        .expect("the machine's script session is established");
    let completed = engine.run_until_completion(Duration::from_secs(3), Duration::from_millis(10));
    assert!(completed, "the run reached no final state");
    assert_eq!(engine.terminal_state(), Some(Test388State::Pass));
    assert_eq!(*told.lock().expect("the count is not poisoned"), 0);
}
