// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// Each scenario of the properties judge's turnstile, replayed by the Rust lane
// (docs/adr/0115, D15 and the converse of D8). A scenario is the run TLC gave
// of a violation or a witness, with the events its environment put on the
// external queue (`sce_build::run_trace`); this test gives the machine those
// events, on a manual clock, with its trace port on, and holds the run the port
// is told to be the scenario's, step for step (`sce_build::formal::replay`),
// and to the recorded run `<name>.rust.json` beside the scenario, which the
// same judge reads with the C11 lane's (sce-build/tests/a_scenario_is_the_run_of_every_lane.rs).
//
// The harness gives an input once the machine has taken every step it can,
// which is after the step the input names unless the model's environment put
// it on the queue in the middle of a macrostep. A macrostep that also sends to
// its own external queue after that point would take the two in another order
// here; the runs then part, which is a defect of this harness to settle, and
// not a verdict about the document. A delayed send comes due by the clock
// moving to the next deadline, which is the earliest: a scenario in which the
// model's time makes a later one due first is one no clock replays, and the
// runs part there too.
//
// The scenarios are written by `sce-codegen formal properties --scenarios
// sce-build/tests/fixtures/formal_properties/scenarios`. Recording the lane's
// runs again, after a change that moves a run on purpose:
//   UPDATE_EXPECT=1 cargo test -p sce-rust-tests --test a_scenario_of_the_turnstile_is_its_run

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use sce_build::formal::replay::{parting, SCENARIO_SUFFIX};
use sce_build::run_trace::{Input, InputKind, RunTrace as Scenario};
use sce_rust_runtime::trace_port::{RunTrace, TraceStep};
use sce_rust_runtime::{Engine, IScriptEngine, SceClock};
use sce_rust_tests::integration::formal_turnstile::TurnstilePolicy;

/// The directory of the turnstile's scenarios and of each lane's run of them.
fn scenarios_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../sce-build/tests/fixtures/formal_properties/scenarios")
}

fn script_engine() -> Arc<dyn IScriptEngine> {
    Arc::new(sce_rust_lua::LuaEngine::new())
}

/// Each scenario in the directory, by its name, in the order of the names.
fn scenarios(dir: &Path) -> Vec<(String, PathBuf)> {
    let mut found: Vec<(String, PathBuf)> = std::fs::read_dir(dir)
        .unwrap_or_else(|e| panic!("{} cannot be read: {e}", dir.display()))
        .map(|entry| entry.expect("a directory entry").path())
        .filter_map(|path| {
            let name = path
                .file_name()?
                .to_str()?
                .strip_suffix(SCENARIO_SUFFIX)?
                .to_string();
            Some((name, path))
        })
        .collect();
    found.sort();
    found
}

/// The run the trace port is told when the turnstile is given `inputs`.
fn replay(name: &str, inputs: &[Input]) -> RunTrace {
    let trace = Arc::new(Mutex::new(RunTrace::new()));
    let sink = Arc::clone(&trace);
    let mut engine = Engine::new(TurnstilePolicy::new(script_engine()));
    engine.set_clock(SceClock::Manual(0));
    engine.set_trace_port(move |step: &TraceStep| {
        sink.lock().expect("the trace is not poisoned").push(step);
    });
    engine
        .initialize()
        .expect("the machine's script session is established");
    for input in inputs {
        let told = trace
            .lock()
            .expect("the trace is not poisoned")
            .steps()
            .len();
        assert!(
            told >= input.after,
            "{name}: the machine took {told} steps, and the model {} before `{}` came",
            input.after,
            input.event
        );
        match input.kind {
            InputKind::Arrive => {
                engine.raise_external_by_name(&input.event, "");
                engine.tick();
            }
            InputKind::Due => {
                let wait = engine.time_until_next_scheduled_ms().unwrap_or_else(|| {
                    panic!(
                        "{name}: `{}` comes due, and the machine holds no delayed send",
                        input.event
                    )
                });
                engine.advance_time_ms(wait);
            }
        }
    }
    drop(engine);
    Arc::try_unwrap(trace)
        .unwrap_or_else(|_| panic!("the port is dropped with the engine that held it"))
        .into_inner()
        .expect("the trace is not poisoned")
}

/// Hold `trace` to the recorded run at `path`, or record it again when
/// `UPDATE_EXPECT` is set.
fn holds_to_the_recorded_run(path: &Path, trace: &RunTrace) {
    let text = trace.to_json();
    if std::env::var_os("UPDATE_EXPECT").is_some() {
        std::fs::write(path, &text).expect("the recorded run is writable");
        return;
    }
    let recorded = std::fs::read_to_string(path).unwrap_or_else(|e| {
        panic!(
            "{} cannot be read ({e}): record it with `UPDATE_EXPECT=1 cargo test -p \
             sce-rust-tests --test a_scenario_of_the_turnstile_is_its_run`",
            path.display()
        )
    });
    assert_eq!(
        text,
        recorded,
        "the run of {} is not the recorded one the replay judge reads",
        path.display()
    );
}

#[test]
fn each_scenario_of_the_turnstile_is_its_run_on_the_rust_lane() {
    let dir = scenarios_dir();
    let found = scenarios(&dir);
    assert!(
        !found.is_empty(),
        "{} holds no scenario: write them with `sce-codegen formal properties --scenarios`",
        dir.display()
    );
    for (name, path) in found {
        let text = std::fs::read_to_string(&path).expect("the scenario is readable");
        let scenario = Scenario::from_json(&text)
            .unwrap_or_else(|e| panic!("{} is no scenario: {e}", path.display()));
        let trace = replay(&name, &scenario.inputs);
        let run =
            Scenario::from_json(&trace.to_json()).expect("the lane writes the run trace form");
        if let Some(parted) = parting(&scenario, &run) {
            panic!("{name}: the Rust lane's run is not the model's: {parted}");
        }
        holds_to_the_recorded_run(&dir.join(format!("{name}.rust.json")), &trace);
    }
}
